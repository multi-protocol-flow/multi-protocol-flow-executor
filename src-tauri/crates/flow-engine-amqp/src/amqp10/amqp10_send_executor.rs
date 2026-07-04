//! AMQP 1.0 发送消息节点执行器
//!
//! 实现 AMQP 1.0 通过 Sender Link 发送消息功能。

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
use flow_engine_core::node_config::{Amqp10SendConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 1.0 发送消息节点执行器
pub struct Amqp10SendExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl Amqp10SendExecutor {
    /// 创建新的 AMQP 1.0 发送消息执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for Amqp10SendExecutor {
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
            NodeData::Amqp10Send(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not Amqp10SendConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("connection_id variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid, node.uuid, node.name, error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let address = replace_variables(&config.address, context).map_err(|e| {
            let error_msg = format!("address variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid, node.uuid, node.name, error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let message_body = match &config.message_body {
            Some(body) => Some(replace_variables(body, context).map_err(|e| {
                let error_msg = format!("message_body variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid, node.uuid, node.name, error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        // 3. 日志：开始发送
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp10_send, Connection ID: {}, Address: {}",
            context.flow_uuid, node.uuid, node.name, connection_id, address
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 1.0 send node {} start execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建发送配置（使用替换后的值）
        let send_config = Amqp10SendConfig {
            connection_id: connection_id.clone(),
            address: address.clone(),
            message_body,
            ..config
        };

        // 5. 发送消息
        let send_result = self.amqp_manager.amqp10_send(&connection_id, &send_config).await;

        let duration = start_time.elapsed();

        match send_result {
            Ok(output) => {
                if output.success {
                    log::info!(
                        "[AMQP 1.0] Send succeeded: connection_id={}, address={}",
                        connection_id, address
                    );
                } else {
                    log::warn!(
                        "[AMQP 1.0] Send failed: connection_id={}, address={}, error={:?}",
                        connection_id, address, output.error
                    );
                }

                let output_data = json!({
                    "success": output.success,
                    "request": {
                        "connection_id": connection_id,
                        "address": address,
                    },
                    "timing": {
                        "send_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                let log_level = if output.success { LogLevel::Info } else { LogLevel::Error };
                let log_message = if output.success {
                    format!("AMQP 1.0 发送Success ({}ms)", duration.as_millis())
                } else {
                    format!("AMQP 1.0 发送Failed: {} ({}ms)", output.error.as_deref().unwrap_or("未知Error"), duration.as_millis())
                };

                context.emit_log(LogEvent {
                    level: log_level,
                    message: log_message,
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP 1.0] 发送异常: connection_id={}, address={}, error={}",
                    connection_id, address, e
                );

                context.emit_log(LogEvent {
                    level: LogLevel::Error,
                    message: format!("AMQP 1.0 发送异常: {}", e),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                let error_output = json!({
                    "success": false,
                    "request": { "connection_id": connection_id, "address": address },
                    "timing": { "send_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("AMQP 1.0 send Failed: {}", e)));
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
        Amqp10AcceptOutput, Amqp10ConnectOutput, Amqp10ReceiveOutput,
        Amqp10RejectOutput, Amqp10SendOutput, AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider,
    };

    /// Mock AmqpProvider（发送Success）
    struct MockAmqpProvider {
        output: Amqp10SendOutput,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: Amqp10SendOutput { success: true, error: None },
            }
        }
        fn new_failure(error: &str) -> Self {
            Self {
                output: Amqp10SendOutput { success: false, error: Some(error.to_string()) },
            }
        }
    }

    // 宏避免重复实现所有 trait 方法 — 手动实现完整 AmqpProvider
    #[async_trait::async_trait]
    impl AmqpProvider for MockAmqpProvider {
        async fn connect(&self, _uuid: &str, _config: &AmqpConnectConfig) -> Result<AmqpConnectOutput, String> { Err("not implemented".to_string()) }
        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> { Ok(()) }
        async fn exchange_declare(&self, _uuid: &str, _config: &AmqpExchangeDeclareConfig) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn exchange_delete(&self, _uuid: &str, _config: &AmqpExchangeDeleteConfig) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn queue_declare(&self, _uuid: &str, _config: &AmqpQueueDeclareConfig) -> Result<flow_engine_core::traits::AmqpQueueDeclareOutput, String> { Err("not implemented".to_string()) }
        async fn queue_delete(&self, _uuid: &str, _config: &AmqpQueueDeleteConfig) -> Result<flow_engine_core::traits::AmqpQueueDeleteOutput, String> { Err("not implemented".to_string()) }
        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> { Ok(()) }
        async fn publish(&self, _uuid: &str, _config: &AmqpPublishConfig) -> Result<flow_engine_core::traits::AmqpPublishOutput, String> { Err("not implemented".to_string()) }
        async fn consume(&self, _uuid: &str, _config: &AmqpConsumeConfig) -> Result<flow_engine_core::traits::AmqpConsumeOutput, String> { Err("not implemented".to_string()) }
        async fn get(&self, _uuid: &str, _config: &AmqpGetConfig) -> Result<flow_engine_core::traits::AmqpGetOutput, String> { Err("not implemented".to_string()) }
        async fn ack(&self, _uuid: &str, _config: &AmqpAckConfig) -> Result<flow_engine_core::traits::AmqpAckOutput, String> { Err("not implemented".to_string()) }
        async fn management(&self, _uuid: &str, _config: &AmqpManagementConfig) -> Result<flow_engine_core::traits::AmqpManagementOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_connect(&self, _uuid: &str, _config: &Amqp10ConnectConfig) -> Result<Amqp10ConnectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_send(&self, _uuid: &str, _config: &Amqp10SendConfig) -> Result<Amqp10SendOutput, String> { Ok(self.output.clone()) }
        async fn amqp10_receive(&self, _uuid: &str, _config: &Amqp10ReceiveConfig) -> Result<Amqp10ReceiveOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_accept(&self, _uuid: &str, _config: &Amqp10AcceptConfig) -> Result<Amqp10AcceptOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_reject(&self, _uuid: &str, _config: &Amqp10RejectConfig) -> Result<Amqp10RejectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_disconnect(&self, _uuid: &str, _config: &Amqp10DisconnectConfig) -> Result<(), String> { Err("not implemented".to_string()) }
        fn get_status(&self, _uuid: &str) -> Option<String> { Some("connected".to_string()) }
        fn list_connections(&self) -> Vec<AmqpConnectionInfo> { vec![] }
    }

    /// Mock AmqpProvider（amqp10_send 返回 Err）
    struct MockAmqpProviderError;

    #[async_trait::async_trait]
    impl AmqpProvider for MockAmqpProviderError {
        async fn connect(&self, _uuid: &str, _config: &AmqpConnectConfig) -> Result<AmqpConnectOutput, String> { Err("not implemented".to_string()) }
        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> { Ok(()) }
        async fn exchange_declare(&self, _uuid: &str, _config: &AmqpExchangeDeclareConfig) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn exchange_delete(&self, _uuid: &str, _config: &AmqpExchangeDeleteConfig) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn queue_declare(&self, _uuid: &str, _config: &AmqpQueueDeclareConfig) -> Result<flow_engine_core::traits::AmqpQueueDeclareOutput, String> { Err("not implemented".to_string()) }
        async fn queue_delete(&self, _uuid: &str, _config: &AmqpQueueDeleteConfig) -> Result<flow_engine_core::traits::AmqpQueueDeleteOutput, String> { Err("not implemented".to_string()) }
        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> { Ok(()) }
        async fn publish(&self, _uuid: &str, _config: &AmqpPublishConfig) -> Result<flow_engine_core::traits::AmqpPublishOutput, String> { Err("not implemented".to_string()) }
        async fn consume(&self, _uuid: &str, _config: &AmqpConsumeConfig) -> Result<flow_engine_core::traits::AmqpConsumeOutput, String> { Err("not implemented".to_string()) }
        async fn get(&self, _uuid: &str, _config: &AmqpGetConfig) -> Result<flow_engine_core::traits::AmqpGetOutput, String> { Err("not implemented".to_string()) }
        async fn ack(&self, _uuid: &str, _config: &AmqpAckConfig) -> Result<flow_engine_core::traits::AmqpAckOutput, String> { Err("not implemented".to_string()) }
        async fn management(&self, _uuid: &str, _config: &AmqpManagementConfig) -> Result<flow_engine_core::traits::AmqpManagementOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_connect(&self, _uuid: &str, _config: &Amqp10ConnectConfig) -> Result<Amqp10ConnectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_send(&self, _uuid: &str, _config: &Amqp10SendConfig) -> Result<Amqp10SendOutput, String> { Err("发送Failed".to_string()) }
        async fn amqp10_receive(&self, _uuid: &str, _config: &Amqp10ReceiveConfig) -> Result<Amqp10ReceiveOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_accept(&self, _uuid: &str, _config: &Amqp10AcceptConfig) -> Result<Amqp10AcceptOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_reject(&self, _uuid: &str, _config: &Amqp10RejectConfig) -> Result<Amqp10RejectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_disconnect(&self, _uuid: &str, _config: &Amqp10DisconnectConfig) -> Result<(), String> { Err("not implemented".to_string()) }
        fn get_status(&self, _uuid: &str) -> Option<String> { None }
        fn list_connections(&self) -> Vec<AmqpConnectionInfo> { vec![] }
    }

    fn create_test_node(uuid: &str, name: &str, connection_id: &str, address: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp10_send".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Send(Amqp10SendConfig {
                connection_id: connection_id.to_string(),
                address: address.to_string(),
                message_body: Some("test message".to_string()),
                ..Amqp10SendConfig::default()
            }),
            ports: None,
            has_breakpoint: None, on_error: None,
        }
    }

    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
            log_sender: None, flow_file_path: None, ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：发送Success时路由到 true 端口
    #[tokio::test]
    async fn test_amqp10_send_success_routes_to_true() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10SendExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试发送", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["request"]["connection_id"], "conn-1");
        assert_eq!(data["request"]["address"], "test-queue");
    }

    /// 测试：发送Failed时路由到 false 端口
    #[tokio::test]
    async fn test_amqp10_send_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("Link detached"));
        let executor = Amqp10SendExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "Failed测试", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Link detached");
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_amqp10_send_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10SendExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-wrong".to_string(), node_type: "http".to_string(), name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 }, data: NodeData::Entry(EntryConfig::default()),
            ports: None, has_breakpoint: None, on_error: None,
        };
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Amqp10SendConfig"));
    }

    /// 测试：变量替换在 connection_id 和 address 中生效
    #[tokio::test]
    async fn test_amqp10_send_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10SendExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(), node_type: "amqp10_send".to_string(), name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Send(Amqp10SendConfig {
                connection_id: "{{conn}}".to_string(),
                address: "queue://{{queue_name}}".to_string(),
                ..Amqp10SendConfig::default()
            }),
            ports: None, has_breakpoint: None, on_error: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable("conn".to_string(), json!("my-conn"), String::new());
        context.variables.set_variable("queue_name".to_string(), json!("orders"), String::new());

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let data = result.unwrap().1.unwrap();
        assert_eq!(data["request"]["connection_id"], "my-conn");
        assert_eq!(data["request"]["address"], "queue://orders");
    }

    /// 测试：provider amqp10_send 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp10_send_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = Amqp10SendExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(output_data.unwrap()["error"], "发送Failed");
    }

    /// 测试：连接端口路由
    #[tokio::test]
    async fn test_amqp10_send_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10SendExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", "test-queue");
        let mut context = create_context("test-flow");

        let connections = vec![Connection {
            id: "conn-1".to_string(), source_node_uuid: "test-route".to_string(),
            target_node_uuid: "next-node".to_string(), source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor.execute(&node, &mut context, &connections, &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-node"]);
    }
}
