//! AMQP 1.0 断开连接节点执行器
//!
//! 实现 AMQP 1.0 断开连接功能。
//! 断开始终走 "out" 端口（失败不阻塞流程）。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{Amqp10DisconnectConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 1.0 断开连接节点执行器
pub struct Amqp10DisconnectExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl Amqp10DisconnectExecutor {
    /// 创建新的 AMQP 1.0 断开连接执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for Amqp10DisconnectExecutor {
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
            NodeData::Amqp10Disconnect(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 Amqp10DisconnectConfig: {}",
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

        // 3. 日志：开始断开
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp10_disconnect, 连接ID: {}",
            context.flow_uuid, node.uuid, node.name, connection_id
        );

        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("AMQP 1.0 断开节点 {} 开始执行", node.name),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("系统时间错误").as_millis() as u64,
            node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        // 4. 构建断开配置
        let disconnect_config = Amqp10DisconnectConfig {
            connection_id: connection_id.clone(),
        };

        // 5. 断开连接
        let disconnect_result = self.amqp_manager.amqp10_disconnect(&connection_id, &disconnect_config).await;

        // AMQP 1.0 断开始终走 "out" 端口，不阻塞流程
        let next_node_ids: Vec<String> = {
            let filtered: Vec<String> = connections
                .iter()
                .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "out")
                .map(|conn| conn.target_node_uuid.clone())
                .collect();
            if filtered.is_empty() { vec!["out".to_string()] } else { filtered }
        };

        let duration = start_time.elapsed();

        match disconnect_result {
            Ok(()) => {
                log::info!("[AMQP 1.0] 断开成功: connection_id={}", connection_id);

                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "timing": { "disconnect_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                });

                context.emit_log(LogEvent {
                    level: LogLevel::Info,
                    message: format!("AMQP 1.0 断开成功 ({}ms)", duration.as_millis()),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("系统时间错误").as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!("[AMQP 1.0] 断开失败: connection_id={}, error={}", connection_id, e);

                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "timing": { "disconnect_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": e,
                });

                context.emit_log(LogEvent {
                    level: LogLevel::Error,
                    message: format!("AMQP 1.0 断开失败: {}", e),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("系统时间错误").as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

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
    use flow_engine_core::node_config::*;
    use flow_engine_core::traits::*;

    /// Mock AmqpProvider（断开成功）
    struct MockAmqpProvider { should_error: bool }
    impl MockAmqpProvider {
        fn new_success() -> Self { Self { should_error: false } }
        fn new_error() -> Self { Self { should_error: true } }
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
        async fn amqp10_accept(&self, _uuid: &str, _config: &Amqp10AcceptConfig) -> Result<Amqp10AcceptOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_reject(&self, _uuid: &str, _config: &Amqp10RejectConfig) -> Result<Amqp10RejectOutput, String> { Err("not implemented".to_string()) }
        async fn amqp10_disconnect(&self, _uuid: &str, _config: &Amqp10DisconnectConfig) -> Result<(), String> {
            if self.should_error { Err("断开失败".to_string()) } else { Ok(()) }
        }
        fn get_status(&self, _uuid: &str) -> Option<String> { Some("connected".to_string()) }
        fn list_connections(&self) -> Vec<AmqpConnectionInfo> { vec![] }
    }

    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(), node_type: "amqp10_disconnect".to_string(), name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Disconnect(Amqp10DisconnectConfig {
                connection_id: connection_id.to_string(),
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
            mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, redis_manager: None,
            last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：断开成功时路由到 out 端口
    #[tokio::test]
    async fn test_amqp10_disconnect_success_routes_to_out() {
        let executor = Amqp10DisconnectExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = create_test_node("test-1", "断开测试", "conn-1");
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert_eq!(data.unwrap()["success"], true);
    }

    /// 测试：断开失败时仍然路由到 out 端口（失败不阻塞）
    #[tokio::test]
    async fn test_amqp10_disconnect_failure_routes_to_out() {
        let executor = Amqp10DisconnectExecutor::new(Arc::new(MockAmqpProvider::new_error()));
        let node = create_test_node("test-2", "失败测试", "conn-1");
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        let data = data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "断开失败");
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_amqp10_disconnect_wrong_node_data_type() {
        let executor = Amqp10DisconnectExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = Node {
            uuid: "test-wrong".to_string(), node_type: "http".to_string(), name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 }, data: NodeData::Entry(EntryConfig::default()),
            ports: None, has_breakpoint: None,
        };
        let mut context = create_context("test-flow");
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Amqp10DisconnectConfig"));
    }

    /// 测试：变量替换在 connection_id 中生效
    #[tokio::test]
    async fn test_amqp10_disconnect_with_variable_replacement() {
        let executor = Amqp10DisconnectExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        let mut context = create_context("test-flow");
        context.variables.set_variable("conn_id".to_string(), json!("my-amqp10-conn"), String::new());

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let data = result.unwrap().1.unwrap();
        assert_eq!(data["connection_id"], "my-amqp10-conn");
    }

    /// 测试：连接端口路由到下游节点
    #[tokio::test]
    async fn test_amqp10_disconnect_with_connections_routing() {
        let executor = Amqp10DisconnectExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = create_test_node("test-route", "路由测试", "conn-1");
        let mut context = create_context("test-flow");

        let connections = vec![Connection {
            id: "conn-1".to_string(), source_node_uuid: "test-route".to_string(),
            target_node_uuid: "next-node".to_string(), source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor.execute(&node, &mut context, &connections, &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-node"]);
    }
}
