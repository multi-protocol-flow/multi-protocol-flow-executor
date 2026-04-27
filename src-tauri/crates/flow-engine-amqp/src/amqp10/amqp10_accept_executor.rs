//! AMQP 1.0 确认消息节点执行器
//!
//! 实现 AMQP 1.0 确认（ack）已接收消息的功能。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{Amqp10AcceptConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 1.0 确认消息节点执行器
pub struct Amqp10AcceptExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl Amqp10AcceptExecutor {
    /// 创建新的 AMQP 1.0 确认消息执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for Amqp10AcceptExecutor {
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
            NodeData::Amqp10Accept(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 Amqp10AcceptConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("connection_id 变量替换失败: {}", e);
            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
            error_msg
        })?;

        // 3. 日志：开始确认
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp10_accept, 连接ID: {}, delivery_id: {}",
            context.flow_uuid, node.uuid, node.name, connection_id, config.delivery_id
        );

        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("AMQP 1.0 确认节点 {} 开始执行", node.name),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("系统时间错误").as_millis() as u64,
            node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        // 4. 构建确认配置
        let accept_config = Amqp10AcceptConfig {
            connection_id: connection_id.clone(),
            ..config
        };

        // 5. 确认消息
        let accept_result = self.amqp_manager.amqp10_accept(&connection_id, &accept_config).await;
        let duration = start_time.elapsed();

        match accept_result {
            Ok(output) => {
                let output_data = json!({
                    "success": output.success,
                    "request": { "connection_id": connection_id, "delivery_id": accept_config.delivery_id },
                    "timing": { "accept_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": output.error,
                });

                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let error_output = json!({
                    "success": false,
                    "request": { "connection_id": connection_id, "delivery_id": accept_config.delivery_id },
                    "timing": { "accept_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": e,
                });

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
    if filtered.is_empty() { vec![port.to_string()] } else { filtered }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::*;
    use flow_engine_core::traits::*;

    struct MockAmqpProvider { output: Amqp10AcceptOutput }
    impl MockAmqpProvider {
        fn new_success() -> Self { Self { output: Amqp10AcceptOutput { success: true, error: None } } }
        fn new_failure(error: &str) -> Self { Self { output: Amqp10AcceptOutput { success: false, error: Some(error.to_string()) } } }
    }

    #[async_trait::async_trait]
    impl AmqpProvider for MockAmqpProvider {
        async fn connect(&self, _uuid: &str, _config: &AmqpConnectConfig) -> Result<AmqpConnectOutput, String> { Err("not implemented".to_string()) }
        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> { Ok(()) }
        async fn exchange_declare(&self, _uuid: &str, _config: &AmqpExchangeDeclareConfig) -> Result<AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn exchange_delete(&self, _uuid: &str, _config: &AmqpExchangeDeleteConfig) -> Result<AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn queue_declare(&self, _uuid: &str, _config: &AmqpQueueDeclareConfig) -> Result<AmqpQueueDeclareOutput, String> { Err("not implemented".to_string()) }
        async fn queue_delete(&self, _uuid: &str, _config: &AmqpQueueDeleteConfig) -> Result<AmqpQueueDeleteOutput, String> { Err("not implemented".to_string()) }
        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> { Ok(()) }
        async fn publish(&self, _uuid: &str, _config: &AmqpPublishConfig) -> Result<AmqpPublishOutput, String> { Err("not implemented".to_string()) }
        async fn consume(&self, _uuid: &str, _config: &AmqpConsumeConfig) -> Result<AmqpConsumeOutput, String> { Err("not implemented".to_string()) }
        async fn get(&self, _uuid: &str, _config: &AmqpGetConfig) -> Result<AmqpGetOutput, String> { Err("not implemented".to_string()) }
        async fn ack(&self, _uuid: &str, _config: &AmqpAckConfig) -> Result<AmqpAckOutput, String> { Err("not implemented".to_string()) }
        async fn management(&self, _uuid: &str, _config: &AmqpManagementConfig) -> Result<AmqpManagementOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_connect(&self, _uuid: &str, _config: &Amqp10ConnectConfig) -> Result<Amqp10ConnectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_send(&self, _uuid: &str, _config: &Amqp10SendConfig) -> Result<Amqp10SendOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_receive(&self, _uuid: &str, _config: &Amqp10ReceiveConfig) -> Result<Amqp10ReceiveOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_accept(&self, _uuid: &str, _config: &Amqp10AcceptConfig) -> Result<Amqp10AcceptOutput, String> { Ok(self.output.clone()) }
        async fn amqp10_reject(&self, _uuid: &str, _config: &Amqp10RejectConfig) -> Result<Amqp10RejectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_disconnect(&self, _uuid: &str, _config: &Amqp10DisconnectConfig) -> Result<(), String> { Err("not implemented".to_string()) }
        fn get_status(&self, _uuid: &str) -> Option<String> { Some("connected".to_string()) }
        fn list_connections(&self) -> Vec<AmqpConnectionInfo> { vec![] }
    }

    struct MockAmqpProviderError;
    #[async_trait::async_trait]
    impl AmqpProvider for MockAmqpProviderError {
        async fn connect(&self, _uuid: &str, _config: &AmqpConnectConfig) -> Result<AmqpConnectOutput, String> { Err("not implemented".to_string()) }
        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> { Ok(()) }
        async fn exchange_declare(&self, _uuid: &str, _config: &AmqpExchangeDeclareConfig) -> Result<AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn exchange_delete(&self, _uuid: &str, _config: &AmqpExchangeDeleteConfig) -> Result<AmqpExchangeOutput, String> { Err("not implemented".to_string()) }
        async fn queue_declare(&self, _uuid: &str, _config: &AmqpQueueDeclareConfig) -> Result<AmqpQueueDeclareOutput, String> { Err("not implemented".to_string()) }
        async fn queue_delete(&self, _uuid: &str, _config: &AmqpQueueDeleteConfig) -> Result<AmqpQueueDeleteOutput, String> { Err("not implemented".to_string()) }
        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> { Ok(()) }
        async fn publish(&self, _uuid: &str, _config: &AmqpPublishConfig) -> Result<AmqpPublishOutput, String> { Err("not implemented".to_string()) }
        async fn consume(&self, _uuid: &str, _config: &AmqpConsumeConfig) -> Result<AmqpConsumeOutput, String> { Err("not implemented".to_string()) }
        async fn get(&self, _uuid: &str, _config: &AmqpGetConfig) -> Result<AmqpGetOutput, String> { Err("not implemented".to_string()) }
        async fn ack(&self, _uuid: &str, _config: &AmqpAckConfig) -> Result<AmqpAckOutput, String> { Err("not implemented".to_string()) }
        async fn management(&self, _uuid: &str, _config: &AmqpManagementConfig) -> Result<AmqpManagementOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_connect(&self, _uuid: &str, _config: &Amqp10ConnectConfig) -> Result<Amqp10ConnectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_send(&self, _uuid: &str, _config: &Amqp10SendConfig) -> Result<Amqp10SendOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_receive(&self, _uuid: &str, _config: &Amqp10ReceiveConfig) -> Result<Amqp10ReceiveOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_accept(&self, _uuid: &str, _config: &Amqp10AcceptConfig) -> Result<Amqp10AcceptOutput, String> { Err("确认失败".to_string()) }
        async fn amqp10_reject(&self, _uuid: &str, _config: &Amqp10RejectConfig) -> Result<Amqp10RejectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_disconnect(&self, _uuid: &str, _config: &Amqp10DisconnectConfig) -> Result<(), String> { Err("not implemented".to_string()) }
        fn get_status(&self, _uuid: &str) -> Option<String> { None }
        fn list_connections(&self) -> Vec<AmqpConnectionInfo> { vec![] }
    }

    fn create_test_node(uuid: &str, name: &str, connection_id: &str, delivery_id: u64) -> Node {
        Node {
            uuid: uuid.to_string(), node_type: "amqp10_accept".to_string(), name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Accept(Amqp10AcceptConfig {
                connection_id: connection_id.to_string(), delivery_id,
            }),
            ports: None, has_breakpoint: None,
        }
    }

    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(), current_node_uuid: None, variables: VariableStorage::new(),
            node_reports: vec![], flow_trigger_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("系统时间错误").as_millis() as u64,
            log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None,
            mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None,
            last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：确认成功时路由到 true 端口
    #[tokio::test]
    async fn test_amqp10_accept_success_routes_to_true() {
        let executor = Amqp10AcceptExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = create_test_node("test-1", "确认测试", "conn-1", 42);
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert_eq!(data.unwrap()["success"], true);
    }

    /// 测试：确认失败时路由到 false 端口
    #[tokio::test]
    async fn test_amqp10_accept_failure_routes_to_false() {
        let executor = Amqp10AcceptExecutor::new(Arc::new(MockAmqpProvider::new_failure("Session closed")));
        let node = create_test_node("test-2", "失败测试", "conn-1", 42);
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(data.unwrap()["error"], "Session closed");
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_amqp10_accept_wrong_node_data_type() {
        let executor = Amqp10AcceptExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = Node {
            uuid: "test-wrong".to_string(), node_type: "http".to_string(), name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 }, data: NodeData::Entry(EntryConfig::default()),
            ports: None, has_breakpoint: None,
        };
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Amqp10AcceptConfig"));
    }

    /// 测试：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp10_accept_provider_error_routes_to_false() {
        let executor = Amqp10AcceptExecutor::new(Arc::new(MockAmqpProviderError));
        let node = create_test_node("test-err", "异常测试", "conn-1", 42);
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(data.unwrap()["error"], "确认失败");
    }

    /// 测试：输出数据包含 delivery_id
    #[tokio::test]
    async fn test_amqp10_accept_output_contains_delivery_id() {
        let executor = Amqp10AcceptExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = create_test_node("test-did", "delivery_id测试", "conn-1", 99);
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let data = result.unwrap().1.unwrap();
        assert_eq!(data["request"]["delivery_id"], 99);
    }
}
