//! AMQP 1.0 接收消息节点执行器
//!
//! 实现 AMQP 1.0 通过 Receiver Link 接收消息功能。

use flow_engine_core::error::ErrorDetail;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{Amqp10ReceiveConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// AMQP 1.0 接收消息节点执行器
pub struct Amqp10ReceiveExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl Amqp10ReceiveExecutor {
    /// 创建新的 AMQP 1.0 接收消息执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for Amqp10ReceiveExecutor {
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
            NodeData::Amqp10Receive(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not Amqp10ReceiveConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("connection_id variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let address = replace_variables(&config.address, context).map_err(|e| {
            let error_msg = format!("address variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始接收
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp10_receive, Connection ID: {}, 地址: {}",
            context.flow_uuid, node.uuid, node.name, connection_id, address
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 1.0 接收节点 {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建接收配置
        let receive_config = Amqp10ReceiveConfig {
            connection_id: connection_id.clone(),
            address: address.clone(),
            ..config
        };

        // 5. 接收消息
        let receive_result = self
            .amqp_manager
            .amqp10_receive(&connection_id, &receive_config)
            .await;

        let duration = start_time.elapsed();

        match receive_result {
            Ok(output) => {
                if output.success {
                    log::info!(
                        "[AMQP 1.0] 接收Success: connection_id={}, address={}, 消息数={}",
                        connection_id,
                        address,
                        output.message_count
                    );
                } else {
                    log::warn!(
                        "[AMQP 1.0] 接收Failed: connection_id={}, address={}, error={:?}",
                        connection_id,
                        address,
                        output.error
                    );
                }

                let output_data = json!({
                    "success": output.success,
                    "messages": output.messages,
                    "message_count": output.message_count,
                    "request": {
                        "connection_id": connection_id,
                        "address": address,
                        "max_messages": receive_config.max_messages,
                        "timeout_ms": receive_config.timeout_ms,
                    },
                    "timing": {
                        "receive_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "AMQP 1.0 接收Success，消息数: {} ({}ms)",
                        output.message_count,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP 1.0 接收Failed: {} ({}ms)",
                        output.error.as_deref().unwrap_or("未知Error"),
                        duration.as_millis()
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

                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP 1.0] 接收异常: connection_id={}, address={}, error={}",
                    connection_id,
                    address,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 1.0 接收异常: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "messages": [],
                    "message_count": 0,
                    "request": { "connection_id": connection_id, "address": address },
                    "timing": { "receive_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "AMQP 1.0 receive Failed: {}",
                        e
                    )));
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
    use flow_engine_core::node_config::*;
    use flow_engine_core::traits::{
        Amqp10AcceptOutput, Amqp10ConnectOutput, Amqp10Message, Amqp10ReceiveOutput,
        Amqp10RejectOutput, Amqp10SendOutput, AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider,
    };

    /// Mock AmqpProvider（接收Success）
    struct MockAmqpProvider {
        output: Amqp10ReceiveOutput,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: Amqp10ReceiveOutput {
                    success: true,
                    messages: vec![Amqp10Message {
                        body: "hello".to_string(),
                        body_decoded: None,
                        properties: None,
                        message_annotations: None,
                        delivery_id: None,
                    }],
                    message_count: 1,
                    error: None,
                },
            }
        }
        fn new_failure(error: &str) -> Self {
            Self {
                output: Amqp10ReceiveOutput {
                    success: false,
                    messages: vec![],
                    message_count: 0,
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
        ) -> Result<flow_engine_core::traits::AmqpQueueDeleteOutput, String> {
            Err("not implemented".to_string())
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
        ) -> Result<Amqp10ConnectOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_send(
            &self,
            _uuid: &str,
            _config: &Amqp10SendConfig,
        ) -> Result<Amqp10SendOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_receive(
            &self,
            _uuid: &str,
            _config: &Amqp10ReceiveConfig,
        ) -> Result<Amqp10ReceiveOutput, String> {
            Ok(self.output.clone())
        }
        async fn amqp10_accept(
            &self,
            _uuid: &str,
            _config: &Amqp10AcceptConfig,
        ) -> Result<Amqp10AcceptOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_reject(
            &self,
            _uuid: &str,
            _config: &Amqp10RejectConfig,
        ) -> Result<Amqp10RejectOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_disconnect(
            &self,
            _uuid: &str,
            _config: &Amqp10DisconnectConfig,
        ) -> Result<(), String> {
            Err("not implemented".to_string())
        }
        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }
        fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
            vec![]
        }
    }

    /// Mock `AmqpProvider（amqp10_receive` 返回 Err）
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
        ) -> Result<flow_engine_core::traits::AmqpQueueDeleteOutput, String> {
            Err("not implemented".to_string())
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
        ) -> Result<Amqp10ConnectOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_send(
            &self,
            _uuid: &str,
            _config: &Amqp10SendConfig,
        ) -> Result<Amqp10SendOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_receive(
            &self,
            _uuid: &str,
            _config: &Amqp10ReceiveConfig,
        ) -> Result<Amqp10ReceiveOutput, String> {
            Err("接收超时".to_string())
        }
        async fn amqp10_accept(
            &self,
            _uuid: &str,
            _config: &Amqp10AcceptConfig,
        ) -> Result<Amqp10AcceptOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_reject(
            &self,
            _uuid: &str,
            _config: &Amqp10RejectConfig,
        ) -> Result<Amqp10RejectOutput, String> {
            Err("not implemented".to_string())
        }
        async fn amqp10_disconnect(
            &self,
            _uuid: &str,
            _config: &Amqp10DisconnectConfig,
        ) -> Result<(), String> {
            Err("not implemented".to_string())
        }
        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }
        fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
            vec![]
        }
    }

    fn create_test_node(uuid: &str, name: &str, connection_id: &str, address: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp10_receive".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Receive(Amqp10ReceiveConfig {
                connection_id: connection_id.to_string(),
                address: address.to_string(),
                ..Amqp10ReceiveConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            execution_id: String::new(),
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

    /// 测试：接收Success时路由到 true 端口
    #[tokio::test]
    async fn test_amqp10_receive_success_routes_to_true() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ReceiveExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试接收", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], true);
        assert_eq!(data["message_count"], 1);
    }

    /// 测试：接收Failed时路由到 false 端口
    #[tokio::test]
    async fn test_amqp10_receive_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("No messages"));
        let executor = Amqp10ReceiveExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "Failed测试", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(
            data.unwrap_or_else(|| panic!("expected Some"))["error"],
            "No messages"
        );
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_amqp10_receive_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ReceiveExecutor::new(amqp_manager);
        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .message
            .contains("Amqp10ReceiveConfig"));
    }

    /// 测试：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp10_receive_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = Amqp10ReceiveExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(
            data.unwrap_or_else(|| panic!("expected Some"))["error"],
            "接收超时"
        );
    }

    /// 测试：输出数据包含 messages 数组
    #[tokio::test]
    async fn test_amqp10_receive_output_contains_messages() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ReceiveExecutor::new(amqp_manager);
        let node = create_test_node("test-msg", "消息测试", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let data = result
            .unwrap_or_else(|e| panic!("{e}"))
            .1
            .unwrap_or_else(|| panic!("expected Some"));
        assert!(data["messages"].is_array());
        assert_eq!(
            data["messages"]
                .as_array()
                .unwrap_or_else(|| panic!("expected Some"))
                .len(),
            1
        );
        assert_eq!(data["messages"][0]["body"], "hello");
    }
}
