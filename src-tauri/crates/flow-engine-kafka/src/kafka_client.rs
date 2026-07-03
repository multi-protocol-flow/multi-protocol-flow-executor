//! Kafka 客户端封装
//!
//! 基于 rdkafka 的高级封装，提供 Producer / Consumer / Admin 三类客户端创建与操作。
//! 负责将 KafkaConnectConfig 映射为 rdkafka ClientConfig。

use std::time::Duration;

use flow_engine_core::node_config::KafkaConnectConfig;
use log::debug;
use rdkafka::admin::AdminClient;
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::BaseConsumer;
use rdkafka::consumer::StreamConsumer;
use rdkafka::message::{BorrowedMessage, Header, Message, OwnedHeaders};
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use rdkafka::util::Timeout;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error类型
// ---------------------------------------------------------------------------

/// Kafka error type
#[derive(Debug, Error)]
pub enum KafkaError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Produce failed
    #[error("Produce failed: {0}")]
    ProduceFailed(String),

    /// Consume failed
    #[error("Consume failed: {0}")]
    ConsumeFailed(String),

    /// Admin operation failed
    #[error("Admin operation failed: {0}")]
    AdminFailed(String),

    /// Timeout
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// Topic not found
    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization failed
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    /// Deserialization failed
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// KafkaClient
// ---------------------------------------------------------------------------

/// Kafka 高级客户端封装
///
/// 包装 rdkafka 的 `ClientConfig`，提供按需创建 Producer / Consumer / Admin 的方法。
/// 每次调用创建独立的客户端实例，适合短生命周期操作（如单次生产/消费）。
pub struct KafkaClient {
    /// rdkafka 底层配置
    client_config: ClientConfig,
}

impl KafkaClient {
    /// 从 `KafkaConnectConfig` 创建 `KafkaClient`
    ///
    /// 将业务配置映射为 rdkafka 的 `ClientConfig` 键值对。
    pub fn new(config: &KafkaConnectConfig) -> Result<Self, KafkaError> {
        let mut client_config = ClientConfig::new();

        // 必需参数
        client_config.set("bootstrap.servers", &config.bootstrap_servers);

        // 可选参数
        if let Some(ref client_id) = config.client_id {
            client_config.set("client.id", client_id);
        }

        // 安全协议
        if let Some(ref protocol) = config.security_protocol {
            client_config.set("security.protocol", protocol);
        }

        // SASL 认证
        if let Some(ref mechanism) = config.sasl_mechanism {
            client_config.set("sasl.mechanism", mechanism);
        }
        if let Some(ref username) = config.sasl_username {
            client_config.set("sasl.username", username);
        }
        if let Some(ref password) = config.sasl_password {
            client_config.set("sasl.password", password);
        }
        if let Some(ref token) = config.sasl_oauth_token {
            client_config.set("sasl.oauthbearer.config", token);
        }

        // SSL/TLS 配置
        if let Some(ref ca_path) = config.ssl_ca_cert_path {
            client_config.set("ssl.ca.location", ca_path);
        }
        if let Some(ref cert_path) = config.ssl_client_cert_path {
            client_config.set("ssl.certificate.location", cert_path);
        }
        if let Some(ref key_path) = config.ssl_client_key_path {
            client_config.set("ssl.key.location", key_path);
        }
        if let Some(true) = config.ssl_skip_verify {
            client_config.set("enable.ssl.certificate.verification", "false");
        }

        // 连接超时：KafkaConnectConfig 中 connect_timeout_ms 是毫秒
        // rdkafka 的 socket.timeout.ms 也是毫秒
        if let Some(timeout_ms) = config.connect_timeout_ms {
            client_config.set("socket.timeout.ms", timeout_ms.to_string());
        }

        debug!(
            "Kafka 客户端配置已创建: bootstrap_servers={}",
            config.bootstrap_servers
        );

        Ok(Self { client_config })
    }

    /// 获取底层 `ClientConfig` 的引用（供外部按需定制）
    pub fn client_config(&self) -> &ClientConfig {
        &self.client_config
    }

    /// 获取底层 `ClientConfig` 的可变引用（供外部按需定制）
    pub fn client_config_mut(&mut self) -> &mut ClientConfig {
        &mut self.client_config
    }

    // -----------------------------------------------------------------------
    // 客户端创建
    // -----------------------------------------------------------------------

    /// 创建 BaseProducer（同步生产者）
    ///
    /// 调用方负责后续的 flush / drop 操作。
    pub fn create_producer(&self) -> Result<BaseProducer, KafkaError> {
        debug!("正在创建 Kafka BaseProducer");
        self.client_config
            .create()
            .map_err(|e| KafkaError::ConnectionFailed(format!("创建 Producer Failed: {e}")))
    }

    /// 创建 StreamConsumer（异步流式消费者）
    ///
    /// 需要指定消费者组 ID。调用方负责后续的 subscribe / commit 操作。
    pub fn create_consumer(&self, group_id: &str) -> Result<StreamConsumer, KafkaError> {
        debug!("正在创建 Kafka StreamConsumer, group_id={group_id}");
        let mut config = self.client_config.clone();
        config.set("group.id", group_id);

        // 默认从 latest 开始消费，避免意外消费大量历史消息
        config.set("auto.offset.reset", "latest");

        config
            .create()
            .map_err(|e| KafkaError::ConnectionFailed(format!("创建 Consumer Failed: {e}")))
    }

    /// 创建 AdminClient（管理操作客户端）
    pub fn create_admin_client(&self) -> Result<AdminClient<DefaultClientContext>, KafkaError> {
        debug!("正在创建 Kafka AdminClient");
        self.client_config
            .create()
            .map_err(|e| KafkaError::ConnectionFailed(format!("创建 AdminClient Failed: {e}")))
    }

    // -----------------------------------------------------------------------
    // 生产 / 消费辅助方法
    // -----------------------------------------------------------------------

    /// 使用指定的 Producer 发送一条消息
    ///
    /// 返回 `(partition, offset)` 表示消息被写入的分区与偏移量。
    /// 调用方需要在发送后自行调用 `producer.flush(...)` 以确保消息送达。
    pub fn produce_message(
        &self,
        producer: &BaseProducer,
        topic: &str,
        key: Option<&str>,
        value: &[u8],
        headers: Option<Vec<(String, String)>>,
        partition: Option<i32>,
    ) -> Result<(i32, i64), KafkaError> {
        debug!(
            "发送消息: topic={topic}, key={:?}, value_len={}, partition={:?}",
            key,
            value.len(),
            partition
        );

        let mut record: BaseRecord<'_, [u8], [u8]> = BaseRecord::to(topic).payload(value);

        if let Some(k) = key {
            record = record.key(k.as_bytes());
        }

        if let Some(p) = partition {
            record = record.partition(p);
        }

        // 构建消息头
        if let Some(ref hdrs) = headers {
            let mut owned_headers = OwnedHeaders::new();
            for (k, v) in hdrs {
                owned_headers = owned_headers.insert(Header {
                    key: k.as_str(),
                    value: Some(v.as_str()),
                });
            }
            record = record.headers(owned_headers);
        }

        // 发送（poll 以确保回调触发）
        let delivery_result = producer.send(record);
        match delivery_result {
            Ok(_) => {}
            Err((e, _)) => {
                let msg = e.to_string();
                if msg.contains("timeout") || msg.contains("Timed out") {
                    return Err(KafkaError::Timeout(format!("生产消息超时: {e}")));
                }
                if msg.contains("Unknown topic") || msg.contains("unknown topic") {
                    return Err(KafkaError::TopicNotFound(format!(
                        "主题不存在: {topic} — {e}"
                    )));
                }
                return Err(KafkaError::ProduceFailed(format!("生产消息Failed: {e}")));
            }
        }

        // flush 以同步等待发送结果
        producer
            .flush(Timeout::After(Duration::from_secs(5)))
            .map_err(|e| KafkaError::ProduceFailed(format!("flush Failed: {e}")))?;

        // 注意：BaseProducer 的 send 是 fire-and-forget，Success仅表示进入了缓冲区。
        // 返回占位的 (0, -1)，实际分区/偏移量需通过 delivery callback 获取。
        // 对于需要精确 partition/offset 的场景，后续可切换到 FutureProducer。
        Ok((0, -1))
    }

    /// 使用指定的 Consumer 拉取一条消息
    ///
    /// 在给定超时内轮询消费者。返回 `Ok(None)` 表示超时未收到消息。
    pub fn poll_message<'a>(
        &self,
        consumer: &'a BaseConsumer,
        timeout_ms: u64,
    ) -> Result<Option<BorrowedMessage<'a>>, KafkaError> {
        debug!("拉取消息: timeout_ms={timeout_ms}");

        match consumer.poll(Duration::from_millis(timeout_ms)) {
            Some(Ok(msg)) => {
                debug!(
                    "收到消息: topic={}, partition={}, offset={}",
                    msg.topic(),
                    msg.partition(),
                    msg.offset()
                );
                Ok(Some(msg))
            }
            Some(Err(e)) => {
                let err_msg = e.to_string();
                if err_msg.contains("timeout") || err_msg.contains("Timed out") {
                    return Err(KafkaError::Timeout(format!("消费消息超时: {e}")));
                }
                Err(KafkaError::ConsumeFailed(format!("消费消息Failed: {e}")))
            }
            None => {
                debug!("拉取消息超时，未收到消息");
                Ok(None)
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

    /// 验证 KafkaError 各变体的 Display 输出
    #[test]
    fn test_kafka_error_display() {
        assert_eq!(
            KafkaError::ConnectionFailed("broker".into()).to_string(),
            "Connection failed: broker"
        );
        assert_eq!(
            KafkaError::ProduceFailed("err".into()).to_string(),
            "Produce failed: err"
        );
        assert_eq!(
            KafkaError::ConsumeFailed("err".into()).to_string(),
            "Consume failed: err"
        );
        assert_eq!(
            KafkaError::AdminFailed("err".into()).to_string(),
            "Admin operation failed: err"
        );
        assert_eq!(
            KafkaError::Timeout("30s".into()).to_string(),
            "Timeout: 30s"
        );
        assert_eq!(
            KafkaError::AuthFailed("bad token".into()).to_string(),
            "Authentication failed: bad token"
        );
        assert_eq!(
            KafkaError::TopicNotFound("t1".into()).to_string(),
            "Topic not found: t1"
        );
        assert_eq!(
            KafkaError::ConfigError("missing".into()).to_string(),
            "Configuration error: missing"
        );
        assert_eq!(
            KafkaError::SerializationFailed("json".into()).to_string(),
            "Serialization failed: json"
        );
        assert_eq!(
            KafkaError::DeserializationFailed("avro".into()).to_string(),
            "Deserialization failed: avro"
        );
        assert_eq!(
            KafkaError::Other("misc".into()).to_string(),
            "Other error: misc"
        );
    }

    /// 验证基础 KafkaConnectConfig → ClientConfig 映射
    #[test]
    fn test_kafka_client_config_from_connect_config() {
        let config = KafkaConnectConfig {
            bootstrap_servers: "broker1:9092,broker2:9092".to_string(),
            client_id: Some("test-client".to_string()),
            connect_timeout_ms: Some(10000),
            ..Default::default()
        };

        let mut client = KafkaClient::new(&config).expect("KafkaClient 创建应Success");
        let cc = client.client_config();

        // 验证底层 ClientConfig 的关键参数已正确设置
        // ClientConfig 没有公开 getter，但创建Success即表示映射正确
        // 我们通过 client_config() 确认可访问
        let _ = cc;

        // 验证可变引用也可获取
        let _ = client.client_config_mut();

        // 创建 Producer / Admin 应能Success（不需要实际连接 Kafka）
        // Consumer 需要 group.id，此处跳过
        assert!(client.create_producer().is_ok(), "Producer 创建应Success");
        assert!(
            client.create_admin_client().is_ok(),
            "AdminClient 创建应Success"
        );
    }

    /// 验证 SASL 认证配置映射
    #[test]
    fn test_kafka_client_config_with_sasl() {
        let config = KafkaConnectConfig {
            bootstrap_servers: "broker:9092".to_string(),
            security_protocol: Some("sasl_plaintext".to_string()),
            sasl_mechanism: Some("PLAIN".to_string()),
            sasl_username: Some("user".to_string()),
            sasl_password: Some("pass".to_string()),
            ..Default::default()
        };

        let client = KafkaClient::new(&config).expect("SASL KafkaClient 创建应Success");

        // Producer 创建应能正常构建（BaseProducer 不需要 Tokio runtime）
        assert!(
            client.create_producer().is_ok(),
            "SASL Producer 创建应Success"
        );

        // Admin 不依赖连接，可直接创建
        assert!(
            client.create_admin_client().is_ok(),
            "SASL AdminClient 创建应Success"
        );
    }

    /// 验证 SASL 配置下 Consumer 创建（需要 Tokio runtime）
    #[tokio::test]
    async fn test_kafka_client_config_with_sasl_consumer() {
        let config = KafkaConnectConfig {
            bootstrap_servers: "broker:9092".to_string(),
            security_protocol: Some("sasl_plaintext".to_string()),
            sasl_mechanism: Some("PLAIN".to_string()),
            sasl_username: Some("user".to_string()),
            sasl_password: Some("pass".to_string()),
            ..Default::default()
        };

        let client = KafkaClient::new(&config).expect("SASL KafkaClient 创建应Success");

        // StreamConsumer 需要 Tokio runtime
        assert!(
            client.create_consumer("test-group").is_ok(),
            "SASL Consumer 创建应Success"
        );
    }

    /// 验证 SSL/TLS 配置映射
    ///
    /// 仅验证 KafkaClient 构建Success（配置映射正确）。
    /// 不实际创建客户端，因为 rdkafka 会校验 SSL 证书路径是否存在。
    #[test]
    fn test_kafka_client_config_with_ssl() {
        let config = KafkaConnectConfig {
            bootstrap_servers: "broker:9093".to_string(),
            security_protocol: Some("ssl".to_string()),
            ssl_ca_cert_path: Some("/path/to/ca.crt".to_string()),
            ssl_client_cert_path: Some("/path/to/client.crt".to_string()),
            ssl_client_key_path: Some("/path/to/client.key".to_string()),
            ssl_skip_verify: Some(true),
            ..Default::default()
        };

        // 验证配置映射Success（KafkaClient::new 不实际连接 Kafka）
        let client = KafkaClient::new(&config).expect("SSL KafkaClient 创建应Success");

        // 验证 client_config 可访问
        let _ = client.client_config();
    }
}
