//! Kafka 生产节点执行器
//!
//! 实现 Kafka 消息生产功能，支持变量替换和输出构建。
//! 参照 `mqtt_publish_executor.rs` 模式实现。

use std::collections::HashMap;
use flow_engine_core::error::ErrorDetail;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{KafkaProduceConfig, NodeData};
use flow_engine_core::traits::KafkaProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// Kafka 生产节点执行器
pub struct KafkaProduceExecutor {
    kafka_manager: Arc<dyn KafkaProvider>,
}

impl KafkaProduceExecutor {
    /// 创建新的 Kafka 生产执行器
    pub fn new(kafka_manager: Arc<dyn KafkaProvider>) -> Self {
        Self { kafka_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for KafkaProduceExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::KafkaProduce(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not KafkaProduceConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换: connection_id, topic, key, value
        let connection_id =
            replace_variables(&config.connection_id, context).map_err(|e| {
                let error_msg = format!("Connection ID variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?;

        let topic = replace_variables(&config.topic, context).map_err(|e| {
            let error_msg = format!("主题variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let key = match &config.key {
            Some(k) => Some(replace_variables(k, context).map_err(|e| {
                ErrorDetail::execution_error(format!("消息键variable substitution failed: {}", e))
            })?),
            None => None,
        };

        let value = match &config.value {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("消息值variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        // 3. 日志：开始生产
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: kafka_produce, Connection ID: {}, Topic: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            topic
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "Kafka 生产节点 {} start execution (Topic: {})",
                    node.name, topic
                ),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建生产配置（使用变量替换后的值）
        let produce_config = KafkaProduceConfig {
            topic: topic.clone(),
            key,
            value,
            connection_id: connection_id.clone(),
            ..config.clone()
        };

        // 5. 调用 provider.produce
        let produce_result = self
            .kafka_manager
            .produce(&connection_id, &produce_config)
            .await;

        match produce_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[Kafka] 生产Success: connection_id={}, topic={} ({}ms)",
                        connection_id,
                        topic,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[Kafka] 生产返回Error: connection_id={}, topic={}, error={:?}",
                        connection_id,
                        topic,
                        output.error
                    );
                }

                // 6. 构建输出数据
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
                    "partition": output.partition,
                    "offset": output.offset,
                    "timing": timing_json,
                    "error": output.error,
                    "request": {
                        "connection_id": connection_id,
                        "topic": topic,
                        "serialization_format": config.serialization_format,
                        "compression": config.compression,
                        "ack_timeout_ms": config.ack_timeout_ms,
                    },
                });

                // 7. 日志：生产完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_msg = if output.success {
                    format!(
                        "Kafka 生产Success: {} ({}ms)",
                        topic,
                        duration.as_millis()
                    )
                } else {
                    let err = output.error.as_deref().unwrap_or("Unknown error");
                    format!("Kafka 生产Failed: {} - {}", topic, err)
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_msg,
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. 端口路由
                if !output.success
                    && !node.on_error.as_ref().map_or(true, |s| {
                        *s == ExecutionErrorStrategy::RouteToFalse
                    })
                {
                    let error_msg = output
                        .error
                        .as_deref()
                        .unwrap_or("Kafka 生产Failed")
                        .to_string();
                    return Err(ErrorDetail::execution_error(error_msg));
                }
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = route_to_port(node, connections, port);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[Kafka] 生产Failed: connection_id={}, topic={}, error={}",
                    connection_id,
                    topic,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Kafka 生产Failed: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "partition": null,
                    "offset": null,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                    "request": {
                        "connection_id": connection_id,
                        "topic": topic,
                    },
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| {
                    *s == ExecutionErrorStrategy::RouteToFalse
                }) {
                    return Err(ErrorDetail::execution_error(e));
                }

                let next_node_ids = route_to_port(node, connections, "false");
                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 端口路由辅助函数
// ---------------------------------------------------------------------------

/// 根据 `port_id` 查找连接中的目标节点 UUID
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
    use flow_engine_core::node_config::KafkaProduceConfig;
    use flow_engine_core::traits::{
        KafkaCloseOutput, KafkaConnectionInfo, KafkaProduceOutput, KafkaTimingInfo,
    };
    use crate::kafka_connection_manager::KafkaConnectionManager;

    // ---------------------------------------------------------------------------
    // 辅助工具
    // ---------------------------------------------------------------------------

    /// 创建Test Node
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        topic: &str,
        key: Option<&str>,
        value: Option<&str>,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "kafka_produce".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaProduce(KafkaProduceConfig {
                r#type: "kafka_produce".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
                key: key.map(|s| s.to_string()),
                value: value.map(|s| s.to_string()),
                headers: None,
                partition: None,
                timestamp: None,
                serialization_format: Some("json".to_string()),
                schema_subject: None,
                ack_timeout_ms: Some(5000),
                compression: Some("none".to_string()),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: flow_engine_core::utils::unix_timestamp_ms(),
            log_sender: None,
            flow_file_path: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    // ---------------------------------------------------------------------------
    // Mock KafkaProvider（生产Success）
    // ---------------------------------------------------------------------------

    struct MockKafkaProvider;

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
            _config: &KafkaProduceConfig,
        ) -> Result<KafkaProduceOutput, String> {
            Ok(KafkaProduceOutput {
                success: true,
                partition: Some(0),
                offset: Some(42),
                timing: Some(KafkaTimingInfo {
                    total_ms: 10,
                    first_message_ms: None,
                }),
                error: None,
            })
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaConsumeConfig,
        ) -> Result<flow_engine_core::traits::KafkaConsumeOutput, String> {
            Err("not implemented".to_string())
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
            None
        }

        fn list_connections(&self) -> Vec<KafkaConnectionInfo> {
            vec![]
        }
    }

    // ---------------------------------------------------------------------------
    // 测试用例
    // ---------------------------------------------------------------------------

    /// 测试：生产到不存在的连接 → 走 false 端口（使用真实 KafkaConnectionManager）
    #[tokio::test]
    async fn test_produce_nonexistent_connection() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaProduceExecutor::new(kafka_manager);
        let node = create_test_node(
            "test-1",
            "测试生产",
            "nonexistent-conn",
            "test-topic",
            Some("key1"),
            Some("hello kafka"),
        );
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

    /// 测试：Mock Success生产 → 走 true 端口
    #[tokio::test]
    async fn test_produce_success_with_mock() {
        let kafka_manager = Arc::new(MockKafkaProvider);
        let executor = KafkaProduceExecutor::new(kafka_manager);
        let node = create_test_node(
            "test-ok",
            "Success生产",
            "mock-conn",
            "test-topic",
            Some("key1"),
            Some("hello kafka"),
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["partition"], 0);
        assert_eq!(data["offset"], 42);
        assert_eq!(data["request"]["topic"], "test-topic");
        assert_eq!(data["request"]["connection_id"], "mock-conn");
    }

    /// 测试：变量替换
    #[tokio::test]
    async fn test_produce_with_variable_replacement() {
        let kafka_manager = Arc::new(MockKafkaProvider);
        let executor = KafkaProduceExecutor::new(kafka_manager);
        let node = create_test_node(
            "test-var",
            "变量替换测试",
            "{{conn_id}}",
            "sensor/{{device_id}}/data",
            Some("key-{{env}}"),
            Some("{{msg}}"),
        );

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("kafka-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "device_id".to_string(),
            json!("sensor-001"),
            String::new(),
        );
        context
            .variables
            .set_variable("env".to_string(), json!("prod"), String::new());
        context
            .variables
            .set_variable("msg".to_string(), json!("hello"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(data["request"]["topic"], "sensor/sensor-001/data");
        assert_eq!(data["request"]["connection_id"], "kafka-connection-123");
    }

    /// 测试：Error的节点数据类型 → Err
    #[tokio::test]
    async fn test_produce_wrong_node_data_type() {
        let kafka_manager = Arc::new(MockKafkaProvider);
        let executor = KafkaProduceExecutor::new(kafka_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("KafkaProduceConfig"));
    }

    /// 测试：输出 JSON 结构完整性
    #[tokio::test]
    async fn test_produce_output_json_structure() {
        let kafka_manager = Arc::new(MockKafkaProvider);
        let executor = KafkaProduceExecutor::new(kafka_manager);
        let node = create_test_node(
            "test-structure",
            "结构测试",
            "mock-conn",
            "sensor/data",
            Some("my-key"),
            Some(r#"{"temp": 42}"#),
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        // 验证所有预期字段存在
        assert!(data.get("success").is_some(), "应有 success 字段");
        assert!(data.get("partition").is_some(), "应有 partition 字段");
        assert!(data.get("offset").is_some(), "应有 offset 字段");
        assert!(data.get("timing").is_some(), "应有 timing 字段");
        assert!(data.get("request").is_some(), "应有 request 字段");
        assert!(
            data["request"]["connection_id"].is_string(),
            "request 中应有 connection_id"
        );
        assert!(
            data["request"]["topic"].is_string(),
            "request 中应有 topic"
        );
        assert!(
            data["request"]["serialization_format"].is_string(),
            "request 中应有 serialization_format"
        );
    }
}
