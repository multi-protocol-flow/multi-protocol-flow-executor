//! AMQP 删除队列节点执行器
//!
//! 实现 AMQP 0-9-1 队列删除/清空、变量替换和输出构建功能。
//! Success路由到 true 端口，Failed路由到 false 端口。

use std::collections::HashMap;
use flow_engine_core::error::ErrorDetail;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::{AmqpQueueDeleteConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 删除队列节点执行器
pub struct AmqpQueueDeleteExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpQueueDeleteExecutor {
    /// 创建新的 AMQP 删除队列执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpQueueDeleteExecutor {
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
            NodeData::AmqpQueueDelete(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not AmqpQueueDeleteConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（connection_id、queue）
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let queue = replace_variables(&config.queue, context).map_err(|e| {
            let error_msg = format!("队列名称variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始删除
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp_queue_delete, queue: {}, action: {:?}, if_unused: {}, if_empty: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            queue,
            config.action,
            config.if_unused,
            config.if_empty,
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "AMQP QueueDelete 节点 {} start execution (queue={}, action={:?})",
                    node.name, queue, config.action
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 保存原始配置值用于报告
        let original_action = config.action.clone();
        let original_if_unused = config.if_unused;
        let original_if_empty = config.if_empty;

        // 5. 构建删除配置（使用替换后的值）
        let delete_config = AmqpQueueDeleteConfig {
            connection_id: connection_id.clone(),
            queue: queue.clone(),
            ..config
        };

        // 5. 删除队列
        let delete_result = self
            .amqp_manager
            .queue_delete(&connection_id, &delete_config)
            .await;

        let duration = start_time.elapsed();

        match delete_result {
            Ok(output) => {
                if output.success {
                    log::info!(
                        "[AMQP] 队列删除Success: uuid={}, queue={}, messages_deleted={:?}",
                        node.uuid,
                        queue,
                        output.messages_deleted,
                    );
                } else {
                    log::warn!(
                        "[AMQP] 队列删除Failed: uuid={}, queue={}, error={:?}",
                        node.uuid,
                        queue,
                        output.error,
                    );
                }

                // 6. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "messages_deleted": output.messages_deleted,
                    "request": {
                        "connection_id": connection_id,
                        "queue": queue,
                        "action": original_action,
                        "if_unused": original_if_unused,
                        "if_empty": original_if_empty,
                    },
                    "timing": {
                        "delete_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 7. 日志：删除完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "AMQP 队列删除Success: {} 条消息已删除 ({}ms)",
                        output.messages_deleted.unwrap_or(0),
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP 队列删除Failed: {} ({}ms)",
                        output.error.as_deref().unwrap_or("未知Error"),
                        duration.as_millis()
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_message,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. 路由：Success走 true 端口，Failed走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 队列删除异常: uuid={}, queue={}, error={}",
                    node.uuid,
                    queue,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 队列删除异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "messages_deleted": serde_json::Value::Null,
                    "request": {
                        "connection_id": connection_id,
                        "queue": queue,
                        "action": original_action,
                        "if_unused": original_if_unused,
                        "if_empty": original_if_empty,
                    },
                    "timing": {
                        "delete_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("AMQP queue_delete Failed: {}", e)));
                }

                let next_node_ids = resolve_port_targets(connections, &node.uuid, "false");

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

/// 解析端口路由目标
fn resolve_port_targets(connections: &[Connection], node_uuid: &str, port: &str) -> Vec<String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node_uuid && conn.source_port_id == port)
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    if filtered.is_empty() {
        vec![port.to_string()]
    } else {
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::{
        Amqp10AcceptConfig, Amqp10ConnectConfig, Amqp10DisconnectConfig, Amqp10ReceiveConfig,
        Amqp10RejectConfig, Amqp10SendConfig, AmqpAckConfig, AmqpCloseConfig,
        AmqpConsumeConfig, AmqpConnectConfig, AmqpExchangeDeclareConfig,
        AmqpExchangeDeleteConfig, AmqpGetConfig, AmqpManagementConfig, AmqpPublishConfig,
        AmqpQueueBindConfig, AmqpQueueDeclareConfig,
    };
    use flow_engine_core::traits::{
        AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider, AmqpQueueDeleteOutput,
    };

    /// Mock AmqpProvider（queue_delete Success）
    struct MockAmqpProvider {
        output: AmqpQueueDeleteOutput,
    }

    impl MockAmqpProvider {
        fn new_success(messages_deleted: u32) -> Self {
            Self {
                output: AmqpQueueDeleteOutput {
                    success: true,
                    messages_deleted: Some(messages_deleted),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpQueueDeleteOutput {
                    success: false,
                    messages_deleted: None,
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl AmqpProvider for MockAmqpProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<AmqpConnectOutput, String> {
            Ok(AmqpConnectOutput {
                success: true,
                connection_id: "test-conn".to_string(),
                server_properties: None,
                error: None,
            })
        }

        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> {
            Ok(())
        }

        async fn exchange_declare(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn exchange_delete(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeleteConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_declare(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpQueueDeclareOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_delete(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeleteConfig,
        ) -> Result<AmqpQueueDeleteOutput, String> {
            Ok(self.output.clone())
        }

        async fn queue_bind(
            &self,
            _uuid: &str,
            _config: &AmqpQueueBindConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &AmqpPublishConfig,
        ) -> Result<flow_engine_core::traits::AmqpPublishOutput, String> {
            Err("not implemented".to_string())
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &AmqpConsumeConfig,
        ) -> Result<flow_engine_core::traits::AmqpConsumeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn get(
            &self,
            _uuid: &str,
            _config: &AmqpGetConfig,
        ) -> Result<flow_engine_core::traits::AmqpGetOutput, String> {
            Err("not implemented".to_string())
        }

        async fn ack(
            &self,
            _uuid: &str,
            _config: &AmqpAckConfig,
        ) -> Result<flow_engine_core::traits::AmqpAckOutput, String> {
            Err("not implemented".to_string())
        }

        async fn management(
            &self,
            _uuid: &str,
            _config: &AmqpManagementConfig,
        ) -> Result<flow_engine_core::traits::AmqpManagementOutput, String> {
            Err("not implemented".to_string())
        }

        async fn amqp10_connect(
            &self,
            _uuid: &str,
            _config: &Amqp10ConnectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ConnectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_send(
            &self,
            _uuid: &str,
            _config: &Amqp10SendConfig,
        ) -> Result<flow_engine_core::traits::Amqp10SendOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_receive(
            &self,
            _uuid: &str,
            _config: &Amqp10ReceiveConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ReceiveOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_accept(
            &self,
            _uuid: &str,
            _config: &Amqp10AcceptConfig,
        ) -> Result<flow_engine_core::traits::Amqp10AcceptOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_reject(
            &self,
            _uuid: &str,
            _config: &Amqp10RejectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10RejectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_disconnect(
            &self,
            _uuid: &str,
            _config: &Amqp10DisconnectConfig,
        ) -> Result<(), String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
            vec![]
        }
    }

    /// Mock AmqpProvider（queue_delete 返回 Err）
    struct MockAmqpProviderError;

    #[async_trait::async_trait]
    impl AmqpProvider for MockAmqpProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<AmqpConnectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> {
            Ok(())
        }

        async fn exchange_declare(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn exchange_delete(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeleteConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_declare(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpQueueDeclareOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_delete(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeleteConfig,
        ) -> Result<AmqpQueueDeleteOutput, String> {
            Err("连接不存在".to_string())
        }

        async fn queue_bind(
            &self,
            _uuid: &str,
            _config: &AmqpQueueBindConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &AmqpPublishConfig,
        ) -> Result<flow_engine_core::traits::AmqpPublishOutput, String> {
            Err("not implemented".to_string())
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &AmqpConsumeConfig,
        ) -> Result<flow_engine_core::traits::AmqpConsumeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn get(
            &self,
            _uuid: &str,
            _config: &AmqpGetConfig,
        ) -> Result<flow_engine_core::traits::AmqpGetOutput, String> {
            Err("not implemented".to_string())
        }

        async fn ack(
            &self,
            _uuid: &str,
            _config: &AmqpAckConfig,
        ) -> Result<flow_engine_core::traits::AmqpAckOutput, String> {
            Err("not implemented".to_string())
        }

        async fn management(
            &self,
            _uuid: &str,
            _config: &AmqpManagementConfig,
        ) -> Result<flow_engine_core::traits::AmqpManagementOutput, String> {
            Err("not implemented".to_string())
        }

        async fn amqp10_connect(
            &self,
            _uuid: &str,
            _config: &Amqp10ConnectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ConnectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_send(
            &self,
            _uuid: &str,
            _config: &Amqp10SendConfig,
        ) -> Result<flow_engine_core::traits::Amqp10SendOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_receive(
            &self,
            _uuid: &str,
            _config: &Amqp10ReceiveConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ReceiveOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_accept(
            &self,
            _uuid: &str,
            _config: &Amqp10AcceptConfig,
        ) -> Result<flow_engine_core::traits::Amqp10AcceptOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_reject(
            &self,
            _uuid: &str,
            _config: &Amqp10RejectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10RejectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_disconnect(
            &self,
            _uuid: &str,
            _config: &Amqp10DisconnectConfig,
        ) -> Result<(), String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }

        fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
            vec![]
        }
    }

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, queue: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_queue_delete".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueDelete(AmqpQueueDeleteConfig {
                connection_id: connection_id.to_string(),
                queue: queue.to_string(),
                action: Some("delete".to_string()),
                if_unused: false,
                if_empty: false,
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
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
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

    /// 测试：删除Success时输出 JSON 结构正确，路由到 true 端口
    #[tokio::test]
    async fn test_queue_delete_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(42));
        let executor = AmqpQueueDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "删除队列", "conn-1", "my-queue");
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
        assert_eq!(data["messages_deleted"], 42);
        assert!(data["timing"]["delete_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert!(data["error"].is_null());
    }

    /// 测试：删除Failed时路由到 false 端口
    #[tokio::test]
    async fn test_queue_delete_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("Queue in use"));
        let executor = AmqpQueueDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "Failed测试", "conn-1", "my-queue");
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
        assert_eq!(data["error"], "Queue in use");
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_queue_delete_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(0));
        let executor = AmqpQueueDeleteExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().message.contains("AmqpQueueDeleteConfig"));
    }

    /// 测试：变量替换在 queue 和 connection_id 中生效
    #[tokio::test]
    async fn test_queue_delete_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(10));
        let executor = AmqpQueueDeleteExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp_queue_delete".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueDelete(AmqpQueueDeleteConfig {
                connection_id: "{{conn_id}}".to_string(),
                queue: "{{queue_name}}".to_string(),
                action: Some("purge".to_string()),
                if_unused: true,
                if_empty: true,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("amqp-conn-999"),
            String::new(),
        );
        context.variables.set_variable(
            "queue_name".to_string(),
            json!("old.queue"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["request"]["connection_id"], "amqp-conn-999");
        assert_eq!(data["request"]["queue"], "old.queue");
        assert_eq!(data["request"]["action"], "purge");
        assert_eq!(data["request"]["if_unused"], true);
        assert_eq!(data["request"]["if_empty"], true);
    }

    /// 测试：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_queue_delete_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpQueueDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "conn-1", "my-queue");
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
        assert_eq!(data["error"], "连接不存在");
    }

    /// 测试：连接端口路由到下游节点
    #[tokio::test]
    async fn test_queue_delete_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(0));
        let executor = AmqpQueueDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", "my-queue");
        let mut context = create_context("test-flow");

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-route".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-node"]);
    }

    /// 测试：输出数据包含完整 request 字段
    #[tokio::test]
    async fn test_queue_delete_report_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(7));
        let executor = AmqpQueueDeleteExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "amqp_queue_delete".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueDelete(AmqpQueueDeleteConfig {
                connection_id: "my-conn".to_string(),
                queue: "test.queue".to_string(),
                action: Some("delete".to_string()),
                if_unused: true,
                if_empty: false,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        assert!(data.get("success").is_some());
        assert!(data.get("messages_deleted").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        let request = &data["request"];
        assert_eq!(request["connection_id"], "my-conn");
        assert_eq!(request["queue"], "test.queue");
        assert_eq!(request["action"], "delete");
        assert_eq!(request["if_unused"], true);
        assert_eq!(request["if_empty"], false);
    }
}
