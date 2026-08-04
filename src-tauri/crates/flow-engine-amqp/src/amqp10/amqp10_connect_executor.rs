//! AMQP 1.0 连接节点执行器
//!
//! 实现 AMQP 1.0 连接建立、变量替换和输出构建功能。

use flow_engine_core::error::ErrorDetail;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{Amqp10ConnectConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// AMQP 1.0 连接节点执行器
pub struct Amqp10ConnectExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl Amqp10ConnectExecutor {
    /// 创建新的 AMQP 1.0 连接执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for Amqp10ConnectExecutor {
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
            NodeData::Amqp10Connect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not Amqp10ConnectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（url）
        let url = replace_variables(&config.url, context).map_err(|e| {
            let error_msg = format!("url variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始连接
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp10_connect, url: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 1.0 连接节点 {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 确定连接 ID（优先使用配置中的 connection_id，否则使用节点 UUID）
        let connection_id = config
            .connection_id
            .clone()
            .unwrap_or_else(|| node.uuid.clone());

        // 5. 构建连接配置（使用替换后的 url）
        let connect_config = Amqp10ConnectConfig {
            url,
            connection_id: Some(connection_id.clone()),
            ..config
        };

        // 6. 建立连接
        let connect_result = self
            .amqp_manager
            .amqp10_connect(&connection_id, &connect_config)
            .await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[AMQP 1.0] 连接Success: uuid={}, url={}, connection_id={}",
                        node.uuid,
                        connect_config.url,
                        output.connection_id
                    );
                } else {
                    log::warn!(
                        "[AMQP 1.0] connection failed: uuid={}, url={}, error={:?}",
                        node.uuid,
                        connect_config.url,
                        output.error
                    );
                }

                // 7. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "connection_id": output.connection_id,
                    "request": {
                        "url": connect_config.url,
                        "tls_mode": connect_config.tls_mode,
                        "auth_mechanism": connect_config.auth_mechanism,
                        "container_id": connect_config.container_id,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 8. 日志：连接完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!("AMQP 1.0 连接Success ({}ms)", duration.as_millis())
                } else {
                    format!(
                        "AMQP 1.0 connection failed: {} ({}ms)",
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

                // 9. 路由：Success走 true 端口，Failed走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP 1.0] connection error: uuid={}, url={}, error={}",
                    node.uuid,
                    connect_config.url,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 1.0 connection error: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "request": {
                        "url": connect_config.url,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "AMQP 1.0 connect Failed: {}",
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
        Amqp10AcceptOutput, Amqp10ConnectOutput, Amqp10ReceiveOutput, Amqp10RejectOutput,
        Amqp10SendOutput, AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider,
    };

    /// Mock AmqpProvider（连接Success）
    struct MockAmqpProvider {
        output: Amqp10ConnectOutput,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: Amqp10ConnectOutput {
                    success: true,
                    connection_id: "test-amqp10-conn-1".to_string(),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: Amqp10ConnectOutput {
                    success: false,
                    connection_id: String::new(),
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
            Ok(self.output.clone())
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
            Err("not implemented".to_string())
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

    /// Mock `AmqpProvider（amqp10_connect` 返回 Err）
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
            Err("网络超时".to_string())
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
            Err("not implemented".to_string())
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

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp10_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Connect(Amqp10ConnectConfig {
                url: url.to_string(),
                ..Amqp10ConnectConfig::default()
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

    /// 测试：连接Success时输出 JSON 结构正确
    #[tokio::test]
    async fn test_amqp10_connect_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试连接", "amqp://localhost:5672");
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
        assert_eq!(data["connection_id"], "test-amqp10-conn-1");
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["error"].is_null());
    }

    /// 测试：connection failed时路由到 false 端口
    #[tokio::test]
    async fn test_amqp10_connect_failure_routes_to_false_port() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("Connection refused"));
        let executor = Amqp10ConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "Failed测试", "amqp://unreachable:5672");
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
        assert_eq!(data["error"], "Connection refused");
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_amqp10_connect_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ConnectExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().message.contains("Amqp10ConnectConfig"));
    }

    /// 测试：变量替换在 url 中生效
    #[tokio::test]
    async fn test_amqp10_connect_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ConnectExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp10_connect".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Connect(Amqp10ConnectConfig {
                url: "amqp://{{host}}:5672".to_string(),
                ..Amqp10ConnectConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("amqp-broker.example.com"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(
            data["request"]["url"],
            "amqp://amqp-broker.example.com:5672"
        );
    }

    /// 测试：provider `amqp10_connect` 方法返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp10_connect_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = Amqp10ConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "amqp://localhost:5672");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "网络超时");
    }

    /// 测试：连接端口路由
    #[tokio::test]
    async fn test_amqp10_connect_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "amqp://localhost:5672");
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

    /// 测试：默认 `connection_id` 回退到节点 UUID
    #[tokio::test]
    async fn test_amqp10_connect_default_connection_id_fallback() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = Amqp10ConnectExecutor::new(amqp_manager);

        let node = Node {
            uuid: "node-uuid-123".to_string(),
            node_type: "amqp10_connect".to_string(),
            name: "默认ID测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Connect(Amqp10ConnectConfig {
                url: "amqp://localhost:5672".to_string(),
                connection_id: None,
                ..Amqp10ConnectConfig::default()
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
    }
}
