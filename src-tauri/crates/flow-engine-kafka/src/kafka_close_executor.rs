// Kafka 关闭连接节点执行器
//
// 关闭指定的 Kafka 连接，释放连接池中的资源。
// 关闭始终走 "out" 端口（失败不阻塞流程）。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{KafkaCloseConfig, NodeData};
use flow_engine_core::traits::KafkaProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Kafka 关闭连接节点执行器
pub struct KafkaCloseExecutor {
    kafka_manager: Arc<dyn KafkaProvider>,
}

impl KafkaCloseExecutor {
    /// 创建新的 Kafka 关闭连接执行器
    pub fn new(kafka_manager: Arc<dyn KafkaProvider>) -> Self {
        Self { kafka_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for KafkaCloseExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::KafkaClose(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 KafkaCloseConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
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

        // 3. 日志：开始关闭
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: kafka_close, 连接ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Kafka Close 节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建关闭配置（使用替换后的 connection_id）
        let close_config = KafkaCloseConfig {
            r#type: config.r#type,
            connection_id: connection_id.clone(),
        };

        // 5. 关闭连接
        let close_result = self
            .kafka_manager
            .close(&connection_id, &close_config)
            .await;

        // KafkaClose 始终走 "out" 端口，不阻塞流程
        let next_node_ids: Vec<String> = {
            let filtered: Vec<String> = connections
                .iter()
                .filter(|conn| {
                    conn.source_node_uuid == node.uuid && conn.source_port_id == "out"
                })
                .map(|conn| conn.target_node_uuid.clone())
                .collect();

            if filtered.is_empty() {
                vec!["out".to_string()]
            } else {
                filtered
            }
        };

        let duration = start_time.elapsed();

        match close_result {
            Ok(output) => {
                if output.success {
                    log::info!(
                        "[Kafka] 连接关闭成功: connection_id={}",
                        connection_id
                    );
                } else {
                    log::warn!(
                        "[Kafka] 连接关闭失败: connection_id={}, error={:?}",
                        connection_id,
                        output.error
                    );
                }

                // 6. 构建输出数据
                let output_data = serde_json::to_value(&output).unwrap_or_else(|_| {
                    json!({
                        "success": false,
                        "error": "序列化关闭输出失败",
                    })
                });

                // 日志：关闭完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Warn
                };
                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: format!(
                            "Kafka 连接关闭{} ({}ms)",
                            if output.success { "成功" } else { "失败" },
                            duration.as_millis()
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

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[Kafka] 连接关闭异常: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：关闭异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Kafka 连接关闭异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据（仍然走 out 端口）
                let error_output = json!({
                    "success": false,
                    "error": e,
                });

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

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "kafka_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaClose(KafkaCloseConfig {
                r#type: "kafka_close".to_string(),
                connection_id: connection_id.to_string(),
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
            grpc_manager: None,
            mcp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            tcp_manager: None,
            udp_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：关闭不存在的连接，仍然路由到 "out" 端口（失败不阻塞流程）
    #[tokio::test]
    async fn test_close_nonexistent_connection_routes_to_out() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaCloseExecutor::new(kafka_manager);
        let node = create_test_node("test-1", "测试关闭", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口（关闭失败不阻塞）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    /// 测试：NodeData 类型不匹配时返回错误
    #[tokio::test]
    async fn test_close_wrong_node_data_type() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaCloseExecutor::new(kafka_manager);

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
        assert!(result.unwrap_err().contains("KafkaCloseConfig"));
    }

    /// 测试：变量替换 — connection_id 中的变量应被正确替换
    #[tokio::test]
    async fn test_close_with_variable_replacement() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaCloseExecutor::new(kafka_manager);

        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("kafka-connection-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
    }
}
