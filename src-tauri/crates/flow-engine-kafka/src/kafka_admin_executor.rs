// Kafka 管理操作执行器
//
// 实现 Kafka 管理操作（主题管理、消费者组管理、Broker 查询等），
// 变量替换和统一输出映射功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::KafkaProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Kafka 管理操作执行器
pub struct KafkaAdminExecutor {
    kafka_manager: Arc<dyn KafkaProvider>,
}

impl KafkaAdminExecutor {
    /// 创建新的 Kafka 管理操作执行器
    pub fn new(kafka_manager: Arc<dyn KafkaProvider>) -> Self {
        Self { kafka_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for KafkaAdminExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        // 1. 提取配置
        let mut config = match &node.data {
            NodeData::KafkaAdmin(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not KafkaAdminConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换 connection_id
        config.connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
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

        // 3. 变量替换 topic_name
        if let Some(ref topic) = config.topic_name {
            config.topic_name = Some(replace_variables(topic, context).map_err(|e| {
                let error_msg = format!("主题名称variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?);
        }

        // 4. 变量替换 consumer_group_id
        if let Some(ref group_id) = config.consumer_group_id {
            config.consumer_group_id = Some(replace_variables(group_id, context).map_err(|e| {
                let error_msg = format!("消费者组 ID variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?);
        }

        // 5. 日志：start execution
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: kafka_admin, 操作: {}, Connection ID: {}",
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
                    "Kafka 管理节点 {} start execution: {}",
                    node.name, config.operation
                ),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 6. 调用管理操作
        let admin_result = self
            .kafka_manager
            .admin(&config.connection_id, &config)
            .await;

        match admin_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                log::info!(
                    "[Kafka] 管理操作完成: connection_id={}, operation={}, success={}, 耗时={}ms",
                    config.connection_id,
                    config.operation,
                    output.success,
                    duration.as_millis()
                );

                // 7. 构建输出数据
                let output_data = serde_json::to_value(&output).map_err(|e| {
                    ErrorDetail::execution_error(format!(
                        "Failed to serialize admin operation output: {}",
                        e
                    ))
                })?;

                // 8. 日志：execution complete
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "Kafka 管理操作Success: {} ({}ms)",
                        config.operation,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "Kafka 管理操作Failed: {} - {}",
                        config.operation,
                        output.error.as_deref().unwrap_or("Unknown error")
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_message,
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9. 根据Success/Failed选择端口
                if !output.success
                    && !node
                        .on_error
                        .as_ref()
                        .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    let error_msg = output
                        .error
                        .as_deref()
                        .unwrap_or("Kafka 管理操作Failed")
                        .to_string();
                    return Err(ErrorDetail::execution_error(error_msg));
                }
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
                    "[Kafka] 管理操作异常: connection_id={}, operation={}, error={}",
                    config.connection_id,
                    config.operation,
                    e
                );

                // 日志：execution error
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Kafka 管理操作异常: {} - {}", config.operation, e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据
                let error_output = json!({
                    "success": false,
                    "operation": config.operation,
                    "connection_id": config.connection_id,
                    "error": e,
                });

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(e));
                }

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
    use crate::kafka_connection_manager::KafkaConnectionManager;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::KafkaAdminConfig;

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, operation: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "kafka_admin".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaAdmin(KafkaAdminConfig {
                connection_id: connection_id.to_string(),
                operation: operation.to_string(),
                topic_name: None,
                partitions: None,
                replication_factor: None,
                topic_config: None,
                consumer_group_id: None,
                acl_principal: None,
                acl_host: None,
                acl_operation: None,
                acl_permission_type: None,
                acl_resource_type: None,
                acl_resource_name: None,
                dry_run: Some(false),
                ..KafkaAdminConfig::default()
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
            current_node_plugin_data: None,
        }
    }

    #[tokio::test]
    async fn test_admin_connection_not_found() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaAdminExecutor::new(kafka_manager);
        let node = create_test_node("test-1", "测试管理", "nonexistent-conn", "topic_list");
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
    async fn test_admin_with_variable_replacement() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaAdminExecutor::new(kafka_manager);

        // 创建带变量的节点
        let mut node = create_test_node("test-var", "变量替换测试", "{{conn_id}}", "topic_list");
        // 设置 topic_name 带变量
        if let NodeData::KafkaAdmin(ref mut config) = node.data {
            config.topic_name = Some("{{topic}}".to_string());
        }

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("kafka-connection-123"),
            String::new(),
        );
        context
            .variables
            .set_variable("topic".to_string(), json!("my-topic"), String::new());

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
    async fn test_admin_wrong_node_data_type() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaAdminExecutor::new(kafka_manager);

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
        assert!(result.unwrap_err().message.contains("KafkaAdminConfig"));
    }

    #[tokio::test]
    async fn test_admin_config_parsing() {
        // 验证配置可以正确从 JSON 反序列化
        let json = serde_json::json!({
            "type": "kafka_admin",
            "connection_id": "conn-1",
            "operation": "topic_create",
            "topic_name": "test-topic",
            "partitions": 3,
            "replication_factor": 2,
            "dry_run": false,
        });

        let config: KafkaAdminConfig = serde_json::from_value(json).expect("反序列化应Success");
        assert_eq!(config.connection_id, "conn-1");
        assert_eq!(config.operation, "topic_create");
        assert_eq!(config.topic_name, Some("test-topic".to_string()));
        assert_eq!(config.partitions, Some(3));
        assert_eq!(config.replication_factor, Some(2));
        assert_eq!(config.dry_run, Some(false));
    }
}
