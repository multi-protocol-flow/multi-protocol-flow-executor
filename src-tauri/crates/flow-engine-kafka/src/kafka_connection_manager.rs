//! Kafka 连接管理器
//!
//! 实现 [`KafkaProvider`] trait，管理 Kafka 连接的生命周期。
//! 使用 `Arc<RwLock<HashMap>>` 保护连接池，支持并发访问。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use log::{debug, info, warn};
use rdkafka::consumer::{BaseConsumer, Consumer, StreamConsumer};
use rdkafka::message::{Headers, Message};
use rdkafka::metadata::Metadata;
use rdkafka::topic_partition_list::TopicPartitionList;
use tokio::sync::RwLock;

use flow_engine_core::node_config::{
    KafkaAdminConfig, KafkaCloseConfig, KafkaConnectConfig, KafkaConsumeBatchConfig,
    KafkaConsumeConfig, KafkaProduceConfig, KafkaSchemaRegistryConfig,
};
use flow_engine_core::traits::{
    KafkaAdminOutput, KafkaBrokerInfo, KafkaCloseOutput, KafkaConnectOutput, KafkaConnectionInfo,
    KafkaConsumeBatchOutput, KafkaConsumeOutput, KafkaMessageOutput, KafkaPartitionInfo,
    KafkaProduceOutput, KafkaSchemaInfo, KafkaSchemaRegistryOutput, KafkaTimingInfo,
    KafkaTopicInfo, KafkaProvider,
};

use crate::kafka_client::KafkaClient;
use crate::kafka_client::KafkaError;
use crate::schema_registry_client::{SchemaRegistryAuth, SchemaRegistryClient};

// ---------------------------------------------------------------------------
// KafkaConnection — 内部连接状态
// ---------------------------------------------------------------------------

/// Kafka 连接
///
/// 持有 KafkaClient、原始配置和元数据。
/// KafkaClient 按需创建 Producer / Consumer / Admin 客户端。
struct KafkaConnection {
    /// Kafka 高级客户端封装
    client: KafkaClient,
    /// 原始连接配置
    config: KafkaConnectConfig,
    /// 连接状态
    status: String,
    /// 创建时间（毫秒时间戳）
    created_at: i64,
    /// Schema Registry 客户端（惰性初始化）
    schema_registry: Option<SchemaRegistryClient>,
}

// ---------------------------------------------------------------------------
// KafkaConnectionManager
// ---------------------------------------------------------------------------

/// Kafka 连接管理器（per-flow-execution 作用域）
///
/// 管理 Kafka 连接池，实现 [`KafkaProvider`] trait。
/// 线程安全，使用 `Arc<RwLock<HashMap>>` 保护连接池。
pub struct KafkaConnectionManager {
    /// 活跃连接池: `connection_id` → [`KafkaConnection`]
    connections: Arc<RwLock<HashMap<String, KafkaConnection>>>,
}

// ---------------------------------------------------------------------------
// 构造器
// ---------------------------------------------------------------------------

impl KafkaConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取当前时间戳（毫秒）
    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    /// 将 KafkaError 映射为 String
    fn map_error(e: KafkaError) -> String {
        e.to_string()
    }

    /// 将 Schema Registry 错误映射为 String
    fn map_sr_error(e: crate::schema_registry_client::SchemaRegistryError) -> String {
        e.to_string()
    }

    /// 惰性初始化连接的 Schema Registry 客户端
    async fn ensure_schema_registry(conn: &mut KafkaConnection) -> Result<(), String> {
        if conn.schema_registry.is_some() {
            return Ok(());
        }

        let url = match conn.config.schema_registry_url {
            Some(ref u) if !u.is_empty() => u.clone(),
            _ => {
                return Err("Schema Registry URL 未配置".to_string());
            }
        };

        let auth = if conn.config.schema_registry_username.is_some()
            || conn.config.schema_registry_password.is_some()
        {
            Some(SchemaRegistryAuth {
                username: conn.config.schema_registry_username.clone(),
                password: conn.config.schema_registry_password.clone(),
                bearer_token: None,
            })
        } else {
            None
        };

        let client =
            SchemaRegistryClient::new(&url, auth).map_err(Self::map_sr_error)?;

        conn.schema_registry = Some(client);
        Ok(())
    }

    /// 尝试获取 metadata 验证连接是否可用
    async fn verify_connection(client: &KafkaClient) -> Result<(), KafkaError> {
        let consumer: BaseConsumer = client
            .client_config()
            .create()
            .map_err(|e| KafkaError::ConnectionFailed(format!("创建验证 Consumer 失败: {e}")))?;

        let timeout = Duration::from_secs(5);
        consumer
            .fetch_metadata(None, timeout)
            .map_err(|e| {
                KafkaError::ConnectionFailed(format!("获取 Metadata 失败（连接不可达）: {e}"))
            })?;

        Ok(())
    }

    /// 从 Metadata 中提取主题列表
    fn extract_topics(metadata: &Metadata) -> Vec<KafkaTopicInfo> {
        metadata
            .topics()
            .iter()
            .map(|t| {
                let replication = t.partitions().first().map(|p| p.replicas().len() as i32);
                KafkaTopicInfo {
                    name: t.name().to_string(),
                    is_internal: false, // rdkafka 0.39 MetadataTopic 无 is_internal
                    partition_count: Some(t.partitions().len() as i32),
                    replication_factor: replication,
                    config: None,
                }
            })
            .collect()
    }

    /// 从 Metadata 中提取 Broker 列表
    fn extract_brokers(metadata: &Metadata) -> Vec<KafkaBrokerInfo> {
        metadata
            .brokers()
            .iter()
            .map(|b| KafkaBrokerInfo {
                broker_id: b.id(),
                host: b.host().to_string(),
                port: b.port(),
                rack: None,
            })
            .collect()
    }

    /// 从 Metadata 中提取指定主题的分区信息
    fn extract_partitions(metadata: &Metadata) -> Vec<KafkaPartitionInfo> {
        metadata
            .topics()
            .first()
            .map(|t| {
                t.partitions()
                    .iter()
                    .map(|p| KafkaPartitionInfo {
                        partition_id: p.id(),
                        leader: Some(p.leader()),
                        replicas: p.replicas().to_vec(),
                        isr: p.isr().to_vec(),
                        beginning_offset: None,
                        end_offset: None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 将 rdkafka 的 BorrowedMessage 转换为 KafkaMessageOutput
    fn msg_to_output(msg: &rdkafka::message::BorrowedMessage<'_>) -> KafkaMessageOutput {
        let key = msg.key().map(|k| String::from_utf8_lossy(k).to_string());
        let value = msg.payload().map(|p| {
            match serde_json::from_slice::<serde_json::Value>(p) {
                Ok(v) => v,
                Err(_) => serde_json::Value::String(String::from_utf8_lossy(p).to_string()),
            }
        });

        let headers: Vec<(String, String)> = msg
            .headers()
            .map(|h| {
                let mut hdrs = Vec::new();
                for i in 0..h.count() {
                    let header = h.get(i);
                    let val = header
                        .value
                        .map(|v| String::from_utf8_lossy(v).to_string())
                        .unwrap_or_default();
                    hdrs.push((header.key.to_string(), val));
                }
                hdrs
            })
            .unwrap_or_default();

        KafkaMessageOutput {
            topic: msg.topic().to_string(),
            partition: msg.partition(),
            offset: msg.offset(),
            key,
            value,
            headers,
            timestamp: msg.timestamp().to_millis(),
        }
    }

    /// 使用 StreamConsumer 消费指定数量的消息
    async fn consume_messages(
        consumer: &StreamConsumer,
        max_messages: usize,
        poll_timeout_ms: u64,
        deadline: Option<Instant>,
    ) -> (Vec<KafkaMessageOutput>, Option<u64>) {
        let mut messages = Vec::new();
        let mut first_message_elapsed: Option<u64> = None;
        let start = Instant::now();

        for _ in 0..max_messages {
            // 检查是否超过截止时间
            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    break;
                }
            }

            let timeout_dur = if let Some(dl) = deadline {
                dl.saturating_duration_since(Instant::now())
            } else {
                Duration::from_millis(poll_timeout_ms)
            };

            match tokio::time::timeout(timeout_dur, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    if first_message_elapsed.is_none() {
                        first_message_elapsed = Some(start.elapsed().as_millis() as u64);
                    }
                    messages.push(Self::msg_to_output(&msg));
                }
                Ok(Err(e)) => {
                    warn!("Kafka 消费消息错误: {}", e);
                    break;
                }
                Err(_) => {
                    debug!("Kafka 消费超时，已收集 {} 条消息", messages.len());
                    break;
                }
            }
        }

        (messages, first_message_elapsed)
    }

    /// 获取指定连接的 Metadata（同步方法，供 admin 操作使用）
    fn fetch_metadata_sync(
        client: &KafkaClient,
        topic: Option<&str>,
        timeout: Duration,
    ) -> Result<Metadata, String> {
        let consumer: BaseConsumer = client
            .client_config()
            .create()
            .map_err(|e| format!("创建 Consumer 失败: {e}"))?;

        consumer
            .fetch_metadata(topic, timeout)
            .map_err(|e| format!("获取 Metadata 失败: {e}"))
    }
}

impl Default for KafkaConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KafkaConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接（非阻塞）
        if let Ok(mut connections) = self.connections.try_write() {
            let count = connections.len();
            connections.clear();
            if count > 0 {
                info!("Kafka 连接管理器释放，已清理 {} 个连接", count);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// KafkaProvider 实现
// ---------------------------------------------------------------------------

#[async_trait]
impl KafkaProvider for KafkaConnectionManager {
    /// 建立 Kafka 连接
    async fn connect(
        &self,
        uuid: &str,
        config: &KafkaConnectConfig,
    ) -> Result<KafkaConnectOutput, String> {
        info!(
            "Kafka 连接 {} 正在建立: bootstrap_servers={}",
            uuid, config.bootstrap_servers
        );

        // 创建 KafkaClient
        let client = KafkaClient::new(config).map_err(Self::map_error)?;

        // 尝试验证连接（非阻塞，失败不影响存储）
        let status = match Self::verify_connection(&client).await {
            Ok(()) => {
                info!("Kafka 连接 {} 验证成功", uuid);
                "connected".to_string()
            }
            Err(e) => {
                warn!(
                    "Kafka 连接 {} 验证失败: {}，仍存储连接（首次使用时重试）",
                    uuid, e
                );
                "connected".to_string()
            }
        };

        let created_at = Self::now_ms();

        // 存入连接池
        {
            let mut connections = self.connections.write().await;
            connections.insert(
                uuid.to_string(),
                KafkaConnection {
                    client,
                    config: config.clone(),
                    status: status.clone(),
                    created_at,
                    schema_registry: None,
                },
            );
        }

        Ok(KafkaConnectOutput {
            success: true,
            connection_info: Some(KafkaConnectionInfo {
                uuid: uuid.to_string(),
                bootstrap_servers: config.bootstrap_servers.clone(),
                client_id: config.client_id.clone(),
                security_protocol: config.security_protocol.clone(),
                status,
                connected_at: created_at,
            }),
            error: None,
        })
    }

    /// 生产消息
    async fn produce(
        &self,
        uuid: &str,
        config: &KafkaProduceConfig,
    ) -> Result<KafkaProduceOutput, String> {
        let start = Instant::now();
        debug!("Kafka 生产: connection_id={}, topic={}", uuid, config.topic);

        let connections = self.connections.read().await;
        let conn = connections
            .get(uuid)
            .ok_or_else(|| format!("Kafka 连接不存在: {}", uuid))?;

        // 创建 Producer
        let producer = conn.client.create_producer().map_err(Self::map_error)?;

        // 准备消息值
        let value_bytes = config.value.as_deref().map(|v| v.as_bytes()).unwrap_or(b"");

        // 发送消息
        let result = conn.client.produce_message(
            &producer,
            &config.topic,
            config.key.as_deref(),
            value_bytes,
            config.headers.clone(),
            config.partition,
        );

        let elapsed = start.elapsed();

        match result {
            Ok((partition, offset)) => {
                debug!(
                    "Kafka 生产成功: topic={}, partition={}, offset={}, 耗时={}ms",
                    config.topic,
                    partition,
                    offset,
                    elapsed.as_millis()
                );
                Ok(KafkaProduceOutput {
                    success: true,
                    partition: Some(partition),
                    offset: Some(offset),
                    timing: Some(KafkaTimingInfo {
                        total_ms: elapsed.as_millis() as u64,
                        first_message_ms: None,
                    }),
                    error: None,
                })
            }
            Err(e) => {
                let err_msg = e.to_string();
                warn!("Kafka 生产失败: {}", err_msg);
                Ok(KafkaProduceOutput {
                    success: false,
                    partition: None,
                    offset: None,
                    timing: Some(KafkaTimingInfo {
                        total_ms: elapsed.as_millis() as u64,
                        first_message_ms: None,
                    }),
                    error: Some(err_msg),
                })
            }
        }
    }

    /// 消费消息
    async fn consume(
        &self,
        uuid: &str,
        config: &KafkaConsumeConfig,
    ) -> Result<KafkaConsumeOutput, String> {
        let start = Instant::now();
        debug!("Kafka 消费: connection_id={}, topic={}", uuid, config.topic);

        let connections = self.connections.read().await;
        let conn = connections
            .get(uuid)
            .ok_or_else(|| format!("Kafka 连接不存在: {}", uuid))?;

        // 消费者组 ID，默认使用自动生成的
        let default_group_id = format!("mpe-consumer-{}", uuid::Uuid::new_v4());
        let group_id = config
            .group_id
            .as_deref()
            .unwrap_or(&default_group_id);

        // 创建 Consumer
        let consumer: StreamConsumer = conn
            .client
            .create_consumer(group_id)
            .map_err(Self::map_error)?;

        // 订阅主题
        consumer
            .subscribe(&[config.topic.as_str()])
            .map_err(|e| format!("订阅主题失败: {e}"))?;

        // 如果指定了分区和偏移量，则手动分配
        if let (Some(partition), Some(offset)) = (config.partition, config.offset) {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(&config.topic, partition, rdkafka::Offset::from_raw(offset))
                .map_err(|e| format!("添加分区偏移失败: {e}"))?;
            consumer
                .assign(&tpl)
                .map_err(|e| format!("分配分区失败: {e}"))?;
        }

        let max_messages = config.max_messages.unwrap_or(1) as usize;
        let poll_timeout = config.poll_timeout_ms.unwrap_or(5000);

        let (messages, first_message_elapsed) =
            Self::consume_messages(&consumer, max_messages, poll_timeout, None).await;

        let elapsed = start.elapsed();

        Ok(KafkaConsumeOutput {
            success: true,
            messages,
            timing: Some(KafkaTimingInfo {
                total_ms: elapsed.as_millis() as u64,
                first_message_ms: first_message_elapsed,
            }),
            error: None,
        })
    }

    /// 批量消费消息
    async fn consume_batch(
        &self,
        uuid: &str,
        config: &KafkaConsumeBatchConfig,
    ) -> Result<KafkaConsumeBatchOutput, String> {
        let start = Instant::now();
        debug!(
            "Kafka 批量消费: connection_id={}, topic={}, strategy={:?}",
            uuid, config.topic, config.batch_strategy
        );

        let connections = self.connections.read().await;
        let conn = connections
            .get(uuid)
            .ok_or_else(|| format!("Kafka 连接不存在: {}", uuid))?;

        let default_group_id = format!("mpe-batch-{}", uuid::Uuid::new_v4());
        let group_id = config
            .group_id
            .as_deref()
            .unwrap_or(&default_group_id);

        let consumer: StreamConsumer = conn
            .client
            .create_consumer(group_id)
            .map_err(Self::map_error)?;

        consumer
            .subscribe(&[config.topic.as_str()])
            .map_err(|e| format!("订阅主题失败: {e}"))?;

        if let (Some(partition), Some(offset)) = (config.partition, config.offset) {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(&config.topic, partition, rdkafka::Offset::from_raw(offset))
                .map_err(|e| format!("添加分区偏移失败: {e}"))?;
            consumer
                .assign(&tpl)
                .map_err(|e| format!("分配分区失败: {e}"))?;
        }

        let max_messages = config.max_messages.unwrap_or(10) as usize;
        let poll_timeout = config.poll_timeout_ms.unwrap_or(10000);
        let strategy = config.batch_strategy.as_deref().unwrap_or("count_timeout");

        let (messages, first_message_elapsed) = match strategy {
            "count_only" => {
                Self::consume_messages(&consumer, max_messages, poll_timeout, None).await
            }
            "timeout_only" => {
                let deadline = Instant::now() + Duration::from_millis(poll_timeout);
                Self::consume_messages(&consumer, usize::MAX, poll_timeout, Some(deadline)).await
            }
            _ => {
                let deadline = Instant::now() + Duration::from_millis(poll_timeout);
                Self::consume_messages(&consumer, max_messages, poll_timeout, Some(deadline)).await
            }
        };

        let elapsed = start.elapsed();
        let batch_size = messages.len() as u32;

        Ok(KafkaConsumeBatchOutput {
            success: true,
            messages,
            batch_size: Some(batch_size),
            timing: Some(KafkaTimingInfo {
                total_ms: elapsed.as_millis() as u64,
                first_message_ms: first_message_elapsed,
            }),
            error: None,
        })
    }

    /// 管理操作
    async fn admin(
        &self,
        uuid: &str,
        config: &KafkaAdminConfig,
    ) -> Result<KafkaAdminOutput, String> {
        debug!(
            "Kafka 管理操作: connection_id={}, operation={}",
            uuid, config.operation
        );

        let connections = self.connections.read().await;
        let conn = connections
            .get(uuid)
            .ok_or_else(|| format!("Kafka 连接不存在: {}", uuid))?;

        let timeout = Duration::from_secs(30);

        match config.operation.as_str() {
            "topic_list" => {
                let metadata = Self::fetch_metadata_sync(&conn.client, None, timeout)?;
                let topics = Self::extract_topics(&metadata);

                Ok(KafkaAdminOutput {
                    success: true,
                    operation: Some("topic_list".to_string()),
                    topics,
                    partitions: vec![],
                    consumer_groups: vec![],
                    brokers: vec![],
                    config: None,
                    error: None,
                })
            }
            "broker_list" => {
                let metadata = Self::fetch_metadata_sync(&conn.client, None, timeout)?;
                let brokers = Self::extract_brokers(&metadata);

                Ok(KafkaAdminOutput {
                    success: true,
                    operation: Some("broker_list".to_string()),
                    topics: vec![],
                    partitions: vec![],
                    consumer_groups: vec![],
                    brokers,
                    config: None,
                    error: None,
                })
            }
            "topic_create" => {
                let topic_name = config
                    .topic_name
                    .as_deref()
                    .ok_or("创建主题需要指定 topic_name")?;

                let partitions = config.partitions.unwrap_or(1);
                let replication = config.replication_factor.unwrap_or(1);

                let admin_client = conn.client.create_admin_client().map_err(Self::map_error)?;

                let mut new_topic = rdkafka::admin::NewTopic::new(
                    topic_name,
                    partitions,
                    rdkafka::admin::TopicReplication::Fixed(replication),
                );

                if let Some(ref topic_config) = config.topic_config {
                    for (k, v) in topic_config {
                        new_topic = new_topic.set(k, v);
                    }
                }

                let options = rdkafka::admin::AdminOptions::new()
                    .operation_timeout(Some(timeout))
                    .validate_only(config.dry_run.unwrap_or(false));

                let results = admin_client
                    .create_topics([&new_topic], &options)
                    .await
                    .map_err(|e| format!("创建主题请求失败: {e}"))?;

                let mut error_msg = None;
                for result in results {
                    match result {
                        Ok(_) => {
                            info!("Kafka 主题 {} 创建成功", topic_name);
                        }
                        Err((_, err_code)) => {
                            let msg = format!("创建主题 {} 失败: code={:?}", topic_name, err_code);
                            warn!("{}", msg);
                            error_msg = Some(msg);
                        }
                    }
                }

                Ok(KafkaAdminOutput {
                    success: error_msg.is_none(),
                    operation: Some("topic_create".to_string()),
                    topics: vec![],
                    partitions: vec![],
                    consumer_groups: vec![],
                    brokers: vec![],
                    config: None,
                    error: error_msg,
                })
            }
            "topic_delete" => {
                let topic_name = config
                    .topic_name
                    .as_deref()
                    .ok_or("删除主题需要指定 topic_name")?;

                let admin_client = conn.client.create_admin_client().map_err(Self::map_error)?;

                let options = rdkafka::admin::AdminOptions::new()
                    .operation_timeout(Some(timeout))
                    .validate_only(config.dry_run.unwrap_or(false));

                let results = admin_client
                    .delete_topics(&[topic_name], &options)
                    .await
                    .map_err(|e| format!("删除主题请求失败: {e}"))?;

                let mut error_msg = None;
                for result in results {
                    match result {
                        Ok(_) => {
                            info!("Kafka 主题 {} 删除成功", topic_name);
                        }
                        Err((_, err_code)) => {
                            let msg = format!("删除主题 {} 失败: code={:?}", topic_name, err_code);
                            warn!("{}", msg);
                            error_msg = Some(msg);
                        }
                    }
                }

                Ok(KafkaAdminOutput {
                    success: error_msg.is_none(),
                    operation: Some("topic_delete".to_string()),
                    topics: vec![],
                    partitions: vec![],
                    consumer_groups: vec![],
                    brokers: vec![],
                    config: None,
                    error: error_msg,
                })
            }
            "topic_describe" => {
                let topic_name = config
                    .topic_name
                    .as_deref()
                    .ok_or("描述主题需要指定 topic_name")?;

                let metadata =
                    Self::fetch_metadata_sync(&conn.client, Some(topic_name), timeout)?;

                if metadata.topics().is_empty() {
                    return Ok(KafkaAdminOutput {
                        success: false,
                        operation: Some("topic_describe".to_string()),
                        topics: vec![],
                        partitions: vec![],
                        consumer_groups: vec![],
                        brokers: vec![],
                        config: None,
                        error: Some(format!("未找到主题: {}", topic_name)),
                    });
                }

                let topics = Self::extract_topics(&metadata);
                let partitions = Self::extract_partitions(&metadata);

                Ok(KafkaAdminOutput {
                    success: true,
                    operation: Some("topic_describe".to_string()),
                    topics,
                    partitions,
                    consumer_groups: vec![],
                    brokers: vec![],
                    config: None,
                    error: None,
                })
            }
            _ => {
                let msg = format!("暂不支持的管理操作: {}", config.operation);
                warn!("{}", msg);
                Ok(KafkaAdminOutput {
                    success: false,
                    operation: Some(config.operation.clone()),
                    topics: vec![],
                    partitions: vec![],
                    consumer_groups: vec![],
                    brokers: vec![],
                    config: None,
                    error: Some(msg),
                })
            }
        }
    }

    /// Schema Registry 操作
    async fn schema_registry(
        &self,
        uuid: &str,
        config: &KafkaSchemaRegistryConfig,
    ) -> Result<KafkaSchemaRegistryOutput, String> {
        debug!(
            "Kafka Schema Registry: connection_id={}, operation={}",
            uuid, config.operation
        );

        // 需要可变引用来惰性初始化 schema_registry
        let mut connections = self.connections.write().await;
        let conn = connections
            .get_mut(uuid)
            .ok_or_else(|| format!("Kafka 连接不存在: {}", uuid))?;

        Self::ensure_schema_registry(conn).await?;

        let sr_client = conn
            .schema_registry
            .as_ref()
            .expect("ensure_schema_registry 成功后 schema_registry 必定有值");

        match config.operation.as_str() {
            "list_subjects" => {
                let subjects = sr_client.list_subjects().await.map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("list_subjects".to_string()),
                    subjects,
                    versions: vec![],
                    schema: None,
                    is_compatible: None,
                    registered_schema_id: None,
                    error: None,
                })
            }
            "get_subject_versions" => {
                let subject = config
                    .subject
                    .as_deref()
                    .ok_or("获取版本列表需要指定 subject")?;
                let versions = sr_client
                    .get_subject_versions(subject)
                    .await
                    .map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("get_subject_versions".to_string()),
                    subjects: vec![],
                    versions,
                    schema: None,
                    is_compatible: None,
                    registered_schema_id: None,
                    error: None,
                })
            }
            "get_schema_by_version" => {
                let subject = config
                    .subject
                    .as_deref()
                    .ok_or("获取 Schema 需要指定 subject")?;
                let version = config.version.ok_or("获取 Schema 需要指定 version")?;
                let si = sr_client
                    .get_schema_by_version(subject, version)
                    .await
                    .map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("get_schema_by_version".to_string()),
                    subjects: vec![],
                    versions: vec![],
                    schema: Some(KafkaSchemaInfo {
                        subject: si.subject,
                        version: si.version,
                        schema_id: Some(si.id),
                        schema_type: si.schema_type,
                        schema_definition: Some(si.schema),
                        compatibility: None,
                    }),
                    is_compatible: None,
                    registered_schema_id: None,
                    error: None,
                })
            }
            "get_latest_schema" => {
                let subject = config
                    .subject
                    .as_deref()
                    .ok_or("获取最新 Schema 需要指定 subject")?;
                let si = sr_client
                    .get_latest_schema(subject)
                    .await
                    .map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("get_latest_schema".to_string()),
                    subjects: vec![],
                    versions: vec![],
                    schema: Some(KafkaSchemaInfo {
                        subject: si.subject,
                        version: si.version,
                        schema_id: Some(si.id),
                        schema_type: si.schema_type,
                        schema_definition: Some(si.schema),
                        compatibility: None,
                    }),
                    is_compatible: None,
                    registered_schema_id: None,
                    error: None,
                })
            }
            "register_schema" => {
                let subject = config
                    .subject
                    .as_deref()
                    .ok_or("注册 Schema 需要指定 subject")?;
                let schema_def = config
                    .schema_definition
                    .as_deref()
                    .ok_or("注册 Schema 需要指定 schema_definition")?;
                let schema_type = config.schema_type.as_deref().unwrap_or("AVRO");
                let id = sr_client
                    .register_schema(subject, schema_def, schema_type)
                    .await
                    .map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("register_schema".to_string()),
                    subjects: vec![],
                    versions: vec![],
                    schema: None,
                    is_compatible: None,
                    registered_schema_id: Some(id),
                    error: None,
                })
            }
            "check_compatibility" => {
                let subject = config
                    .subject
                    .as_deref()
                    .ok_or("检查兼容性需要指定 subject")?;
                let schema_def = config
                    .schema_definition
                    .as_deref()
                    .ok_or("检查兼容性需要指定 schema_definition")?;
                let schema_type = config.schema_type.as_deref().unwrap_or("AVRO");
                let is_compatible = sr_client
                    .check_compatibility(subject, schema_def, schema_type, config.version)
                    .await
                    .map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("check_compatibility".to_string()),
                    subjects: vec![],
                    versions: vec![],
                    schema: None,
                    is_compatible: Some(is_compatible),
                    registered_schema_id: None,
                    error: None,
                })
            }
            "get_compatibility" => {
                let subject = config
                    .subject
                    .as_deref()
                    .ok_or("获取兼容性配置需要指定 subject")?;
                let level = sr_client
                    .get_compatibility(subject)
                    .await
                    .map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("get_compatibility".to_string()),
                    subjects: vec![],
                    versions: vec![],
                    schema: Some(KafkaSchemaInfo {
                        subject: subject.to_string(),
                        version: 0,
                        schema_id: None,
                        schema_type: None,
                        schema_definition: None,
                        compatibility: Some(level),
                    }),
                    is_compatible: None,
                    registered_schema_id: None,
                    error: None,
                })
            }
            "set_compatibility" => {
                let subject = config
                    .subject
                    .as_deref()
                    .ok_or("设置兼容性需要指定 subject")?;
                let level = config
                    .compatibility_level
                    .as_deref()
                    .ok_or("设置兼容性需要指定 compatibility_level")?;
                sr_client
                    .set_compatibility(subject, level)
                    .await
                    .map_err(Self::map_sr_error)?;
                Ok(KafkaSchemaRegistryOutput {
                    success: true,
                    operation: Some("set_compatibility".to_string()),
                    subjects: vec![],
                    versions: vec![],
                    schema: Some(KafkaSchemaInfo {
                        subject: subject.to_string(),
                        version: 0,
                        schema_id: None,
                        schema_type: None,
                        schema_definition: None,
                        compatibility: Some(level.to_string()),
                    }),
                    is_compatible: None,
                    registered_schema_id: None,
                    error: None,
                })
            }
            _ => {
                let msg = format!("暂不支持的 Schema Registry 操作: {}", config.operation);
                warn!("{}", msg);
                Ok(KafkaSchemaRegistryOutput {
                    success: false,
                    operation: Some(config.operation.clone()),
                    subjects: vec![],
                    versions: vec![],
                    schema: None,
                    is_compatible: None,
                    registered_schema_id: None,
                    error: Some(msg),
                })
            }
        }
    }

    /// 关闭连接
    async fn close(
        &self,
        uuid: &str,
        _config: &KafkaCloseConfig,
    ) -> Result<KafkaCloseOutput, String> {
        info!("Kafka 关闭连接: {}", uuid);

        let mut connections = self.connections.write().await;
        match connections.remove(uuid) {
            Some(_) => {
                info!("Kafka 连接 {} 已关闭并移除", uuid);
                Ok(KafkaCloseOutput {
                    success: true,
                    error: None,
                })
            }
            None => {
                let msg = format!("Kafka 连接不存在: {}", uuid);
                warn!("{}", msg);
                Ok(KafkaCloseOutput {
                    success: false,
                    error: Some(msg),
                })
            }
        }
    }

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String> {
        match self.connections.try_read() {
            Ok(connections) => connections.get(uuid).map(|c| c.status.clone()),
            Err(_) => {
                warn!("获取 Kafka 连接状态失败: 锁竞争");
                None
            }
        }
    }

    /// 列出所有连接
    fn list_connections(&self) -> Vec<KafkaConnectionInfo> {
        match self.connections.try_read() {
            Ok(connections) => connections
                .iter()
                .map(|(uuid, conn)| KafkaConnectionInfo {
                    uuid: uuid.clone(),
                    bootstrap_servers: conn.config.bootstrap_servers.clone(),
                    client_id: conn.config.client_id.clone(),
                    security_protocol: conn.config.security_protocol.clone(),
                    status: conn.status.clone(),
                    connected_at: conn.created_at,
                })
                .collect(),
            Err(_) => {
                warn!("列出 Kafka 连接失败: 锁竞争");
                vec![]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证新建管理器为空
    #[test]
    fn test_manager_new() {
        let manager = KafkaConnectionManager::new();
        let connections = manager
            .connections
            .try_read()
            .expect("应能获取读锁");
        assert!(connections.is_empty(), "新建管理器应无连接");
    }

    /// 验证空管理器 get_status 返回 None
    #[test]
    fn test_get_status_empty() {
        let manager = KafkaConnectionManager::new();
        let status = manager.get_status("non-existent-id");
        assert!(status.is_none(), "空管理器 get_status 应返回 None");
    }

    /// 验证空管理器 list_connections 返回空列表
    #[test]
    fn test_list_connections_empty() {
        let manager = KafkaConnectionManager::new();
        let list = manager.list_connections();
        assert!(list.is_empty(), "空管理器 list_connections 应返回空列表");
    }

    /// 验证关闭不存在的连接返回错误
    #[tokio::test]
    async fn test_close_not_found() {
        let manager = KafkaConnectionManager::new();
        let config = KafkaCloseConfig::default();
        let result = manager.close("non-existent-id", &config).await;
        let output = result.expect("close 不应返回 Err");
        assert!(!output.success, "关闭不存在的连接 success 应为 false");
        assert!(output.error.is_some(), "关闭不存在的连接应有错误信息");
        assert!(
            output.error.as_ref().unwrap().contains("不存在"),
            "错误信息应包含'不存在'"
        );
    }
}
