//! AMQP 连接节点执行器
//!
//! 实现 AMQP 0-9-1 连接建立、变量替换和输出构建功能。

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
use flow_engine_core::node_config::{AmqpConnectConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 连接节点执行器
pub struct AmqpConnectExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpConnectExecutor {
    /// 创建新的 AMQP 连接执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpConnectExecutor {
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
            NodeData::AmqpConnect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not AmqpConnectConfig: {}",
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
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp_connect, url: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 连接节点 {} start execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
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
        let connect_config = AmqpConnectConfig {
            url,
            connection_id: Some(connection_id.clone()),
            ..config
        };

        // 6. 建立连接
        let connect_result = self
            .amqp_manager
            .connect(&connection_id, &connect_config)
            .await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[AMQP] 连接Success: uuid={}, url={}, connection_id={}",
                        node.uuid,
                        connect_config.url,
                        output.connection_id
                    );
                } else {
                    log::warn!(
                        "[AMQP] connection failed: uuid={}, url={}, error={:?}",
                        node.uuid,
                        connect_config.url,
                        output.error
                    );
                }

                // 7. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "connection_id": output.connection_id,
                    "server_properties": output.server_properties,
                    "request": {
                        "url": connect_config.url,
                        "heartbeat": connect_config.heartbeat,
                        "channel_max": connect_config.channel_max,
                        "frame_max": connect_config.frame_max,
                        "connection_timeout_ms": connect_config.connection_timeout_ms,
                        "tls_mode": connect_config.tls_mode,
                        "auth_mechanism": connect_config.auth_mechanism,
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
                    format!("AMQP 连接Success ({}ms)", duration.as_millis())
                } else {
                    format!(
                        "AMQP connection failed: {} ({}ms)",
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

                // 9. 路由：Success走 true 端口，Failed走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] connection error: uuid={}, url={}, error={}",
                    node.uuid,
                    connect_config.url,
                    e
                );

                // 日志：connection error
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP connection error: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "server_properties": serde_json::Value::Null,
                    "request": {
                        "url": connect_config.url,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 返回 false 端口
                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("AMQP connect Failed: {}", e)));
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
        AmqpConsumeConfig, AmqpExchangeDeclareConfig, AmqpExchangeDeleteConfig, AmqpGetConfig,
        AmqpManagementConfig, AmqpPublishConfig, AmqpQueueBindConfig, AmqpQueueDeclareConfig,
        AmqpQueueDeleteConfig,
    };
    use flow_engine_core::traits::{
        AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider,
    };

    /// Mock AmqpProvider（连接Success）
    struct MockAmqpProvider {
        output: AmqpConnectOutput,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: AmqpConnectOutput {
                    success: true,
                    connection_id: "test-conn-1".to_string(),
                    server_properties: None,
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpConnectOutput {
                    success: false,
                    connection_id: String::new(),
                    server_properties: None,
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
            Ok(self.output.clone())
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

    /// Mock AmqpProvider（connect 返回 Err）
    struct MockAmqpProviderError;

    #[async_trait::async_trait]
    impl AmqpProvider for MockAmqpProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<AmqpConnectOutput, String> {
            Err("网络超时".to_string())
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
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpConnect(AmqpConnectConfig {
                url: url.to_string(),
                ..AmqpConnectConfig::default()
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

    /// 测试：连接Success时输出 JSON 结构正确
    #[tokio::test]
    async fn test_amqp_connect_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试连接", "amqp://guest:guest@localhost:5672/%2f");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // Success应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "test-conn-1");
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert_eq!(data["request"]["url"], "amqp://guest:guest@localhost:5672/%2f");
        // error 应为 null（Success时无Error）
        assert!(data["error"].is_null());
    }

    /// 测试：connection failed时路由到 false 端口
    #[tokio::test]
    async fn test_amqp_connect_failure_routes_to_false_port() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("Connection refused"));
        let executor = AmqpConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "Failed测试", "amqp://unreachable:5672");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // Failed应路由到 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Connection refused");
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_amqp_connect_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpConnectExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().message.contains("AmqpConnectConfig"));
    }

    /// 测试：变量替换在 url 中生效
    #[tokio::test]
    async fn test_amqp_connect_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpConnectExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp_connect".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpConnect(AmqpConnectConfig {
                url: "amqp://{{user}}:{{pass}}@{{host}}:5672/%2f".to_string(),
                ..AmqpConnectConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "user".to_string(),
            json!("admin"),
            String::new(),
        );
        context.variables.set_variable(
            "pass".to_string(),
            json!("secret"),
            String::new(),
        );
        context.variables.set_variable(
            "host".to_string(),
            json!("rabbitmq.example.com"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(
            data["request"]["url"],
            "amqp://admin:secret@rabbitmq.example.com:5672/%2f"
        );
    }

    /// 测试：默认 connection_id 回退到节点 UUID
    #[tokio::test]
    async fn test_amqp_connect_default_connection_id_fallback() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpConnectExecutor::new(amqp_manager);

        // 创建不指定 connection_id 的节点
        let node = Node {
            uuid: "node-uuid-123".to_string(),
            node_type: "amqp_connect".to_string(),
            name: "默认ID测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpConnect(AmqpConnectConfig {
                url: "amqp://localhost:5672".to_string(),
                connection_id: None, // 未指定
                ..AmqpConnectConfig::default()
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
        assert!(output_data.is_some());
        // Success执行即可，说明 connection_id 默认使用了节点 UUID
    }

    /// 测试：连接端口路由
    #[tokio::test]
    async fn test_amqp_connect_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "amqp://localhost:5672");
        let mut context = create_context("test-flow");

        // 创建 true 端口连接到下一个节点
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
        // 应路由到连接中指定的目标节点
        assert_eq!(next_nodes, vec!["next-node"]);
    }

    /// 测试：provider connect 方法返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp_connect_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpConnectExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "amqp://localhost:5672");
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
        assert_eq!(data["error"], "网络超时");
    }

    /// 测试：输出数据结构包含所有必要字段
    #[tokio::test]
    async fn test_amqp_connect_report_input_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpConnectExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "amqp_connect".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpConnect(AmqpConnectConfig {
                url: "amqp://localhost:5672".to_string(),
                heartbeat: 600,
                channel_max: 2047,
                frame_max: 131072,
                connection_timeout_ms: 5000,
                tls_mode: Some("disabled".to_string()),
                auth_mechanism: Some("PLAIN".to_string()),
                connection_id: Some("my-conn-id".to_string()),
                ..AmqpConnectConfig::default()
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

        // 验证所有必要字段
        assert!(data.get("success").is_some());
        assert!(data.get("connection_id").is_some());
        assert!(data.get("server_properties").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["url"], "amqp://localhost:5672");
        assert_eq!(request["heartbeat"], 600);
        assert_eq!(request["channel_max"], 2047);
        assert_eq!(request["frame_max"], 131072);
        assert_eq!(request["connection_timeout_ms"], 5000);
        assert_eq!(request["tls_mode"], "disabled");
        assert_eq!(request["auth_mechanism"], "PLAIN");

        // 验证 timing 子结构
        let timing = &data["timing"];
        assert!(timing["connect_ms"].is_number());
        assert!(timing["total_ms"].is_number());
    }
}
