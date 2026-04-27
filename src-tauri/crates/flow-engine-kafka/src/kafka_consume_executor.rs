//! Kafka 消费节点执行器
//!
//! 实现 Kafka 消息消费功能，支持变量替换、取消信号检查和输出构建。
//! 参照 `mqtt_subscribe_executor.rs` 模式实现。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{KafkaConsumeConfig, NodeData};
use flow_engine_core::traits::KafkaProvider;
use serde_json::json;

/// Kafka 消费节点执行器
pub struct KafkaConsumeExecutor {
    kafka_manager: Arc<dyn KafkaProvider>,
}

impl KafkaConsumeExecutor {
    /// 创建新的 Kafka 消费执行器
    pub fn new(kafka_manager: Arc<dyn KafkaProvider>) -> Self {
        Self { kafka_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for KafkaConsumeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 检查取消信号
        if context.is_cancelled() {
            log::info!(
                "[Kafka] 消费节点被取消: uuid={}",
                node.uuid
            );
            return Ok((vec![], None));
        }

        // 2. 提取配置
        let config = match &node.data {
            NodeData::KafkaConsume(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 KafkaConsumeConfig: {}",
                    node.uuid
                ))
            }
        };

        // 3. 变量替换: connection_id, topic, group_id
        let connection_id =
            replace_variables(&config.connection_id, context).map_err(|e| {
                format!("connection_id 变量替换失败: {}", e)
            })?;

        let topic = replace_variables(&config.topic, context).map_err(|e| {
            format!("主题变量替换失败: {}", e)
        })?;

        let group_id = match &config.group_id {
            Some(gid) => Some(replace_variables(gid, context).map_err(|e| {
                format!("group_id 变量替换失败: {}", e)
            })?),
            None => None,
        };

        // 4. 日志：开始消费
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: kafka_consume, 主题: {}, 连接ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            topic,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "Kafka 消费节点 {} 开始执行: topic={}",
                    node.name, topic
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 5. 构建消费配置（使用变量替换后的值）
        let consume_config = KafkaConsumeConfig {
            r#type: config.r#type.clone(),
            connection_id: connection_id.clone(),
            topic: topic.clone(),
            group_id,
            ..config.clone()
        };

        // 6. 再次检查取消信号（在调用 provider 之前）
        if context.is_cancelled() {
            log::info!(
                "[Kafka] 消费节点在调用 provider 前被取消: uuid={}",
                node.uuid
            );
            return Ok((vec![], None));
        }

        // 7. 调用 provider.consume
        let consume_result = self
            .kafka_manager
            .consume(&connection_id, &consume_config)
            .await;

        match consume_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                // 8. 构建输出 JSON
                let messages_json: Vec<serde_json::Value> = output
                    .messages
                    .iter()
                    .map(|msg| {
                        json!({
                            "topic": msg.topic,
                            "partition": msg.partition,
                            "offset": msg.offset,
                            "key": msg.key,
                            "value": msg.value,
                            "headers": msg.headers,
                            "timestamp": msg.timestamp,
                        })
                    })
                    .collect();

                let timing_json = match &output.timing {
                    Some(timing) => json!({
                        "total_ms": timing.total_ms,
                        "first_message_ms": timing.first_message_ms,
                    }),
                    None => json!({
                        "total_ms": duration.as_millis() as u64,
                    }),
                };

                let output_data = json!({
                    "success": output.success,
                    "messages": messages_json,
                    "message_count": output.messages.len(),
                    "timing": timing_json,
                    "error": output.error,
                    "request": {
                        "topic": topic,
                        "connection_id": connection_id,
                        "group_id": consume_config.group_id,
                        "offset_reset": consume_config.offset_reset,
                        "max_messages": consume_config.max_messages,
                        "poll_timeout_ms": consume_config.poll_timeout_ms,
                    },
                });

                log::info!(
                    "[Kafka] 消费完成: topic={}, 消息数={}, 耗时={}ms",
                    topic,
                    output.messages.len(),
                    duration.as_millis()
                );

                // 9. 日志：消费完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_msg = if output.success {
                    format!(
                        "Kafka 消费成功: topic={}, 收到 {} 条消息 ({}ms)",
                        topic,
                        output.messages.len(),
                        duration.as_millis()
                    )
                } else {
                    let err = output.error.as_deref().unwrap_or("未知错误");
                    format!("Kafka 消费失败: {} - {}", topic, err)
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_msg,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 10. 端口路由
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = route_to_port(node, connections, port);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let duration = start_time.elapsed();

                log::error!(
                    "[Kafka] 消费失败: uuid={}, topic={}, error={}",
                    node.uuid,
                    topic,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Kafka 消费失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出
                let error_output = json!({
                    "success": false,
                    "messages": [],
                    "message_count": 0,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "request": {
                        "topic": topic,
                        "connection_id": connection_id,
                    },
                    "error": e,
                });

                let next_node_ids = route_to_port(node, connections, "false");
                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 端口路由辅助函数
// ---------------------------------------------------------------------------

/// 根据 port_id 查找连接中的目标节点 UUID
fn route_to_port(node: &Node, connections: &[Connection], port: &str) -> Vec<String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == port)
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    if filtered.is_empty() {
        vec![port.to_string()]
    } else {
        filtered
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::KafkaConsumeConfig;
    use flow_engine_core::traits::{
        KafkaCloseOutput, KafkaConsumeOutput, KafkaConnectionInfo, KafkaMessageOutput,
        KafkaTimingInfo,
    };
    use crate::kafka_connection_manager::KafkaConnectionManager;

    // ---------------------------------------------------------------------------
    // 辅助工具
    // ---------------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, topic: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "kafka_consume".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaConsume(KafkaConsumeConfig {
                r#type: "kafka_consume".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
                partition: None,
                offset: None,
                offset_reset: Some("latest".to_string()),
                group_id: None,
                enable_auto_commit: Some(false),
                max_messages: Some(1),
                poll_timeout_ms: Some(5000),
                deserialization_format: Some("json".to_string()),
                schema_subject: None,
                fetch_min_bytes: None,
                fetch_max_wait_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            grpc_manager: None,
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    // ---------------------------------------------------------------------------
    // Mock KafkaProvider
    // ---------------------------------------------------------------------------

    /// 模拟成功消费的 KafkaProvider
    struct MockKafkaProvider {
        consume_output: KafkaConsumeOutput,
    }

    impl MockKafkaProvider {
        fn new_success(messages: Vec<KafkaMessageOutput>) -> Self {
            Self {
                consume_output: KafkaConsumeOutput {
                    success: true,
                    messages,
                    timing: Some(KafkaTimingInfo {
                        total_ms: 100,
                        first_message_ms: Some(20),
                    }),
                    error: None,
                },
            }
        }

        fn new_failure(error: String) -> Self {
            Self {
                consume_output: KafkaConsumeOutput {
                    success: false,
                    messages: vec![],
                    timing: None,
                    error: Some(error),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl KafkaProvider for MockKafkaProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaConnectConfig,
        ) -> Result<flow_engine_core::traits::KafkaConnectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn produce(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaProduceConfig,
        ) -> Result<flow_engine_core::traits::KafkaProduceOutput, String> {
            Err("not implemented".to_string())
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &KafkaConsumeConfig,
        ) -> Result<KafkaConsumeOutput, String> {
            if self.consume_output.success {
                Ok(self.consume_output.clone())
            } else {
                Err(self
                    .consume_output
                    .error
                    .clone()
                    .unwrap_or_else(|| "未知错误".to_string()))
            }
        }

        async fn consume_batch(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaConsumeBatchConfig,
        ) -> Result<flow_engine_core::traits::KafkaConsumeBatchOutput, String> {
            Err("not implemented".to_string())
        }

        async fn admin(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaAdminConfig,
        ) -> Result<flow_engine_core::traits::KafkaAdminOutput, String> {
            Err("not implemented".to_string())
        }

        async fn schema_registry(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaSchemaRegistryConfig,
        ) -> Result<flow_engine_core::traits::KafkaSchemaRegistryOutput, String> {
            Err("not implemented".to_string())
        }

        async fn close(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaCloseConfig,
        ) -> Result<KafkaCloseOutput, String> {
            Ok(KafkaCloseOutput {
                success: true,
                error: None,
            })
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<KafkaConnectionInfo> {
            vec![]
        }
    }

    // ---------------------------------------------------------------------------
    // 测试用例
    // ---------------------------------------------------------------------------

    /// 测试：消费不存在的连接 → 走 false 端口（使用真实 KafkaConnectionManager）
    #[tokio::test]
    async fn test_consume_nonexistent_connection() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaConsumeExecutor::new(kafka_manager);
        let node = create_test_node("test-1", "测试消费", "test-topic", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    /// 测试：NodeData::KafkaConsume 提取成功（Mock）
    #[tokio::test]
    async fn test_consume_node_data_extraction() {
        let provider = Arc::new(MockKafkaProvider::new_success(vec![]));
        let executor = KafkaConsumeExecutor::new(provider);
        let node = create_test_node("sub-1", "测试消费", "test-topic", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["request"]["topic"], "test-topic");
        assert_eq!(data["request"]["connection_id"], "conn-1");
        assert_eq!(data["message_count"], 0);
    }

    /// 测试：消费消息内容正确传递
    #[tokio::test]
    async fn test_consume_with_messages() {
        let messages = vec![
            KafkaMessageOutput {
                topic: "test-topic".to_string(),
                partition: 0,
                offset: 100,
                key: Some("key-1".to_string()),
                value: Some(serde_json::json!({"message": "hello"})),
                headers: vec![("trace-id".to_string(), "abc123".to_string())],
                timestamp: Some(1700000000000),
            },
        ];

        let provider = Arc::new(MockKafkaProvider::new_success(messages));
        let executor = KafkaConsumeExecutor::new(provider);
        let node = create_test_node("sub-2", "消息消费", "test-topic", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["message_count"], 1);
        let msgs = data["messages"].as_array().expect("messages 应为数组");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["topic"], "test-topic");
        assert_eq!(msgs[0]["partition"], 0);
        assert_eq!(msgs[0]["offset"], 100);
        assert_eq!(msgs[0]["key"], "key-1");
    }

    /// 测试：消费失败时走 false 端口
    #[tokio::test]
    async fn test_consume_failure_routes_to_false_port() {
        let provider = Arc::new(MockKafkaProvider::new_failure(
            "连接 'conn-missing' 不存在".to_string(),
        ));
        let executor = KafkaConsumeExecutor::new(provider);
        let node = create_test_node("sub-fail", "失败消费", "test-topic", "conn-missing");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    /// 测试：错误的节点数据类型 → Err
    #[tokio::test]
    async fn test_wrong_node_data_type() {
        let provider = Arc::new(MockKafkaProvider::new_success(vec![]));
        let executor = KafkaConsumeExecutor::new(provider);

        let node = Node {
            uuid: "wrong-type".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("KafkaConsumeConfig"));
    }

    /// 测试：变量替换
    #[tokio::test]
    async fn test_variable_replacement() {
        let provider = Arc::new(MockKafkaProvider::new_success(vec![]));
        let executor = KafkaConsumeExecutor::new(provider);

        let node = Node {
            uuid: "sub-var-1".to_string(),
            node_type: "kafka_consume".to_string(),
            name: "变量替换".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaConsume(KafkaConsumeConfig {
                r#type: "kafka_consume".to_string(),
                connection_id: "{{conn_id}}".to_string(),
                topic: "sensors/{{sensor_type}}".to_string(),
                partition: None,
                offset: None,
                offset_reset: Some("latest".to_string()),
                group_id: Some("group-{{env}}".to_string()),
                enable_auto_commit: Some(false),
                max_messages: Some(1),
                poll_timeout_ms: Some(5000),
                deserialization_format: Some("json".to_string()),
                schema_subject: None,
                fetch_min_bytes: None,
                fetch_max_wait_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("my-connection"),
            String::new(),
        );
        context.variables.set_variable(
            "sensor_type".to_string(),
            json!("temperature"),
            String::new(),
        );
        context
            .variables
            .set_variable("env".to_string(), json!("prod"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["request"]["topic"], "sensors/temperature");
        assert_eq!(data["request"]["connection_id"], "my-connection");
        assert_eq!(data["request"]["group_id"], "group-prod");
    }

    /// 测试：取消信号导致提前返回空结果
    #[tokio::test]
    async fn test_consume_cancelled() {
        let provider = Arc::new(MockKafkaProvider::new_success(vec![]));
        let executor = KafkaConsumeExecutor::new(provider);
        let node = create_test_node("sub-cancel", "取消消费", "test-topic", "conn-1");

        let mut context = create_context("test-flow");
        // 设置取消信号
        let (_tx, rx) = tokio::sync::watch::channel(true);
        context.cancel_receiver = Some(rx);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 取消时应返回空结果
        assert!(next_nodes.is_empty());
        assert!(output_data.is_none());
    }
}
