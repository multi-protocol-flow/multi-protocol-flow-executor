//! AMQP 删除交换器节点执行器
//!
//! 实现 AMQP 0-9-1 交换器删除、变量替换和输出构建功能。
//! 成功路由到 true 端口，失败路由到 false 端口。

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
use flow_engine_core::node_config::{AmqpExchangeDeleteConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 删除交换器节点执行器
pub struct AmqpExchangeDeleteExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpExchangeDeleteExecutor {
    /// 创建新的 AMQP 删除交换器执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpExchangeDeleteExecutor {
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
            NodeData::AmqpExchangeDelete(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "节点数据类型不是 AmqpExchangeDeleteConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（connection_id、exchange）
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let exchange = replace_variables(&config.exchange, context).map_err(|e| {
            let error_msg = format!("交换器名称变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始删除
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp_exchange_delete, exchange: {}, if_unused: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            exchange,
            config.if_unused,
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "AMQP ExchangeDelete 节点 {} 开始执行 (exchange={})",
                    node.name, exchange
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

        // 4. 构建删除配置（使用替换后的值）
        let delete_config = AmqpExchangeDeleteConfig {
            connection_id: connection_id.clone(),
            exchange: exchange.clone(),
            ..config
        };

        // 5. 删除交换器
        let delete_result = self
            .amqp_manager
            .exchange_delete(&connection_id, &delete_config)
            .await;

        let duration = start_time.elapsed();

        match delete_result {
            Ok(output) => {
                if output.success {
                    log::info!(
                        "[AMQP] 交换器删除成功: uuid={}, exchange={}",
                        node.uuid,
                        exchange,
                    );
                } else {
                    log::warn!(
                        "[AMQP] 交换器删除失败: uuid={}, exchange={}, error={:?}",
                        node.uuid,
                        exchange,
                        output.error,
                    );
                }

                // 6. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "exchange": output.exchange,
                    "request": {
                        "connection_id": connection_id,
                        "exchange": exchange,
                        "if_unused": config.if_unused,
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
                        "AMQP 交换器删除成功: {} ({}ms)",
                        exchange,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP 交换器删除失败: {} ({}ms)",
                        output.error.as_deref().unwrap_or("未知错误"),
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

                // 8. 路由：成功走 true 端口，失败走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 交换器删除异常: uuid={}, exchange={}, error={}",
                    node.uuid,
                    exchange,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 交换器删除异常: {}", e),
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
                    "exchange": exchange,
                    "request": {
                        "connection_id": connection_id,
                        "exchange": exchange,
                        "if_unused": config.if_unused,
                    },
                    "timing": {
                        "delete_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回错误而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("AMQP exchange_delete 失败: {}", e)));
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
        AmqpConsumeConfig, AmqpExchangeDeclareConfig, AmqpGetConfig, AmqpManagementConfig,
        AmqpPublishConfig, AmqpQueueBindConfig, AmqpQueueDeclareConfig, AmqpQueueDeleteConfig,
        AmqpConnectConfig,
    };
    use flow_engine_core::traits::{
        AmqpConnectOutput, AmqpConnectionInfo, AmqpExchangeOutput, AmqpProvider,
    };

    /// Mock AmqpProvider（exchange_delete 成功）
    struct MockAmqpProvider {
        output: AmqpExchangeOutput,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: AmqpExchangeOutput {
                    success: true,
                    exchange: Some("test-exchange".to_string()),
                    exchange_type: None,
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpExchangeOutput {
                    success: false,
                    exchange: None,
                    exchange_type: None,
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
        ) -> Result<AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn exchange_delete(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeleteConfig,
        ) -> Result<AmqpExchangeOutput, String> {
            Ok(self.output.clone())
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

    /// Mock AmqpProvider（exchange_delete 返回 Err）
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
        ) -> Result<AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn exchange_delete(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeleteConfig,
        ) -> Result<AmqpExchangeOutput, String> {
            Err("连接不存在".to_string())
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

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, exchange: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_exchange_delete".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpExchangeDelete(AmqpExchangeDeleteConfig {
                connection_id: connection_id.to_string(),
                exchange: exchange.to_string(),
                if_unused: false,
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

    /// 测试：删除成功时输出 JSON 结构正确，路由到 true 端口
    #[tokio::test]
    async fn test_exchange_delete_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpExchangeDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "删除交换器", "conn-1", "my-exchange");
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
        assert_eq!(data["exchange"], "test-exchange");
        assert!(data["timing"]["delete_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert!(data["error"].is_null());
    }

    /// 测试：删除失败时路由到 false 端口
    #[tokio::test]
    async fn test_exchange_delete_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("Exchange in use"));
        let executor = AmqpExchangeDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "失败测试", "conn-1", "my-exchange");
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
        assert_eq!(data["error"], "Exchange in use");
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_exchange_delete_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpExchangeDeleteExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
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
        assert!(result.unwrap_err().message.contains("AmqpExchangeDeleteConfig"));
    }

    /// 测试：变量替换在 exchange 和 connection_id 中生效
    #[tokio::test]
    async fn test_exchange_delete_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpExchangeDeleteExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp_exchange_delete".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpExchangeDelete(AmqpExchangeDeleteConfig {
                connection_id: "{{conn_id}}".to_string(),
                exchange: "{{exchange_name}}".to_string(),
                if_unused: true,
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
            "exchange_name".to_string(),
            json!("old.exchange"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["request"]["connection_id"], "amqp-conn-999");
        assert_eq!(data["request"]["exchange"], "old.exchange");
        assert_eq!(data["request"]["if_unused"], true);
    }

    /// 测试：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_exchange_delete_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpExchangeDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "conn-1", "my-exchange");
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
    async fn test_exchange_delete_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpExchangeDeleteExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", "my-exchange");
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
    async fn test_exchange_delete_report_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpExchangeDeleteExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "amqp_exchange_delete".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpExchangeDelete(AmqpExchangeDeleteConfig {
                connection_id: "my-conn".to_string(),
                exchange: "test.exchange".to_string(),
                if_unused: true,
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
        assert!(data.get("exchange").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        let request = &data["request"];
        assert_eq!(request["connection_id"], "my-conn");
        assert_eq!(request["exchange"], "test.exchange");
        assert_eq!(request["if_unused"], true);
    }
}
