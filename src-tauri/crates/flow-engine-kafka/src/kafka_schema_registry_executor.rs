// Kafka Schema Registry 执行器
//
// 实现 Kafka Schema Registry 操作（列表、查询、注册、兼容性检查等），
// 变量替换和统一输出映射功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::KafkaProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Kafka Schema Registry 执行器
pub struct KafkaSchemaRegistryExecutor {
    kafka_manager: Arc<dyn KafkaProvider>,
}

impl KafkaSchemaRegistryExecutor {
    /// 创建新的 Kafka Schema Registry 执行器
    pub fn new(kafka_manager: Arc<dyn KafkaProvider>) -> Self {
        Self { kafka_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for KafkaSchemaRegistryExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取配置
        let mut config = match &node.data {
            NodeData::KafkaSchemaRegistry(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 KafkaSchemaRegistryConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换 connection_id
        config.connection_id =
            replace_variables(&config.connection_id, context).map_err(|e| {
                let error_msg = format!("连接 ID 变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?;

        // 3. 变量替换 subject
        if let Some(ref subject) = config.subject {
            config.subject = Some(replace_variables(subject, context).map_err(|e| {
                let error_msg = format!("Subject 变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?);
        }

        // 4. 日志：开始执行
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: kafka_schema_registry, 操作: {}, 连接ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.operation,
            config.connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "Kafka Schema Registry 节点 {} 开始执行: {}",
                    node.name, config.operation
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 5. 调用 Schema Registry 操作
        let sr_result = self
            .kafka_manager
            .schema_registry(&config.connection_id, &config)
            .await;

        match sr_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                log::info!(
                    "[Kafka] Schema Registry 操作完成: connection_id={}, operation={}, success={}, 耗时={}ms",
                    config.connection_id,
                    config.operation,
                    output.success,
                    duration.as_millis()
                );

                // 6. 构建输出数据
                let output_data = serde_json::to_value(&output)
                    .map_err(|e| format!("序列化 Schema Registry 输出失败: {}", e))?;

                // 7. 日志：执行完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "Kafka Schema Registry 操作成功: {} ({}ms)",
                        config.operation,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "Kafka Schema Registry 操作失败: {} - {}",
                        config.operation,
                        output.error.as_deref().unwrap_or("未知错误")
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_message,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. 根据成功/失败选择端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| {
                        conn.source_node_uuid == node.uuid && conn.source_port_id == port
                    })
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let _duration = start_time.elapsed();

                log::error!(
                    "[Kafka] Schema Registry 操作异常: connection_id={}, operation={}, error={}",
                    config.connection_id,
                    config.operation,
                    e
                );

                // 日志：执行异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!(
                            "Kafka Schema Registry 操作异常: {} - {}",
                            config.operation, e
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据
                let error_output = json!({
                    "success": false,
                    "operation": config.operation,
                    "connection_id": config.connection_id,
                    "error": e,
                });

                // 异常走 false 端口
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| {
                        conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                    })
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::KafkaSchemaRegistryConfig;
    use crate::kafka_connection_manager::KafkaConnectionManager;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, operation: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "kafka_schema_registry".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaSchemaRegistry(KafkaSchemaRegistryConfig {
                connection_id: connection_id.to_string(),
                operation: operation.to_string(),
                subject: None,
                version: None,
                schema_definition: None,
                schema_type: None,
                compatibility_level: None,
                ..KafkaSchemaRegistryConfig::default()
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
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            grpc_manager: None,
            providers: HashMap::new(),
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
        }
    }

    #[tokio::test]
    async fn test_schema_registry_connection_not_found() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaSchemaRegistryExecutor::new(kafka_manager);
        let node = create_test_node("test-1", "测试SR", "nonexistent-conn", "list_subjects");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_schema_registry_with_variable_replacement() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaSchemaRegistryExecutor::new(kafka_manager);

        // 创建带变量的节点
        let mut node = create_test_node("test-var", "变量替换测试", "{{conn_id}}", "get_subject_versions");
        // 设置 subject 带变量
        if let NodeData::KafkaSchemaRegistry(ref mut config) = node.data {
            config.subject = Some("{{subject_name}}".to_string());
        }

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("kafka-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "subject_name".to_string(),
            json!("orders-value"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
    }

    #[tokio::test]
    async fn test_schema_registry_wrong_node_data_type() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaSchemaRegistryExecutor::new(kafka_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
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
        assert!(result.unwrap_err().contains("KafkaSchemaRegistryConfig"));
    }

    #[tokio::test]
    async fn test_schema_registry_config_parsing() {
        // 验证配置可以正确从 JSON 反序列化
        let json = serde_json::json!({
            "type": "kafka_schema_registry",
            "connection_id": "conn-1",
            "operation": "register_schema",
            "subject": "orders-value",
            "schema_definition": "{\"type\":\"record\",\"name\":\"Order\"}",
            "schema_type": "AVRO",
        });

        let config: KafkaSchemaRegistryConfig =
            serde_json::from_value(json).expect("反序列化应成功");
        assert_eq!(config.connection_id, "conn-1");
        assert_eq!(config.operation, "register_schema");
        assert_eq!(config.subject, Some("orders-value".to_string()));
        assert_eq!(
            config.schema_definition,
            Some("{\"type\":\"record\",\"name\":\"Order\"}".to_string())
        );
        assert_eq!(config.schema_type, Some("AVRO".to_string()));
    }
}
