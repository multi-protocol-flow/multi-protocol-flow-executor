//! AMQP 1.0 拒绝消息节点执行器
//!
//! 实现 AMQP 1.0 拒绝（reject/nack）已接收消息的功能。

use flow_engine_core::error::ErrorDetail;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{Amqp10RejectConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// AMQP 1.0 拒绝消息节点执行器
pub struct Amqp10RejectExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl Amqp10RejectExecutor {
    /// 创建新的 AMQP 1.0 拒绝消息执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for Amqp10RejectExecutor {
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
            NodeData::Amqp10Reject(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not Amqp10RejectConfig: {}",
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

        // 3. 日志：开始拒绝
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp10_reject, Connection ID: {}, delivery_id: {}",
            context.flow_uuid, node.uuid, node.name, connection_id, config.delivery_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 1.0 拒绝节点 {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建拒绝配置
        let reject_config = Amqp10RejectConfig {
            connection_id: connection_id.clone(),
            ..config
        };

        // 5. 拒绝消息
        let reject_result = self
            .amqp_manager
            .amqp10_reject(&connection_id, &reject_config)
            .await;
        let duration = start_time.elapsed();

        match reject_result {
            Ok(output) => {
                let output_data = json!({
                    "success": output.success,
                    "request": {
                        "connection_id": connection_id,
                        "delivery_id": reject_config.delivery_id,
                        "error_condition": reject_config.error_condition,
                        "error_description": reject_config.error_description,
                    },
                    "timing": { "reject_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": output.error,
                });

                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let error_output = json!({
                    "success": false,
                    "request": {
                        "connection_id": connection_id,
                        "delivery_id": reject_config.delivery_id,
                    },
                    "timing": { "reject_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "AMQP 1.0 reject Failed: {}",
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
    use flow_engine_core::traits::*;

    struct MockAmqpProvider {
        output: Amqp10RejectOutput,
    }
    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: Amqp10RejectOutput {
                    success: true,
                    error: None,
                },
            }
        }
        fn new_failure(error: &str) -> Self {
            Self {
                output: Amqp10RejectOutput {
                    success: false,
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
        ) -> Result<AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }
        async fn exchange_delete(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeleteConfig,
        ) -> Result<AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }
        async fn queue_declare(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeclareConfig,
        ) -> Result<AmqpQueueDeclareOutput, String> {
            Err("not implemented".to_string())
        }
        async fn queue_delete(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeleteConfig,
        ) -> Result<AmqpQueueDeleteOutput, String> {
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
        ) -> Result<AmqpPublishOutput, String> {
            Err("not implemented".to_string())
        }
        async fn consume(
            &self,
            _uuid: &str,
            _config: &AmqpConsumeConfig,
        ) -> Result<AmqpConsumeOutput, String> {
            Err("not implemented".to_string())
        }
        async fn get(&self, _uuid: &str, _config: &AmqpGetConfig) -> Result<AmqpGetOutput, String> {
            Err("not implemented".to_string())
        }
        async fn ack(&self, _uuid: &str, _config: &AmqpAckConfig) -> Result<AmqpAckOutput, String> {
            Err("not implemented".to_string())
        }
        async fn management(
            &self,
            _uuid: &str,
            _config: &AmqpManagementConfig,
        ) -> Result<AmqpManagementOutput, String> {
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
            Ok(self.output.clone())
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
            Err("not implemented".to_string())
        }
        async fn queue_declare(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeclareConfig,
        ) -> Result<AmqpQueueDeclareOutput, String> {
            Err("not implemented".to_string())
        }
        async fn queue_delete(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeleteConfig,
        ) -> Result<AmqpQueueDeleteOutput, String> {
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
        ) -> Result<AmqpPublishOutput, String> {
            Err("not implemented".to_string())
        }
        async fn consume(
            &self,
            _uuid: &str,
            _config: &AmqpConsumeConfig,
        ) -> Result<AmqpConsumeOutput, String> {
            Err("not implemented".to_string())
        }
        async fn get(&self, _uuid: &str, _config: &AmqpGetConfig) -> Result<AmqpGetOutput, String> {
            Err("not implemented".to_string())
        }
        async fn ack(&self, _uuid: &str, _config: &AmqpAckConfig) -> Result<AmqpAckOutput, String> {
            Err("not implemented".to_string())
        }
        async fn management(
            &self,
            _uuid: &str,
            _config: &AmqpManagementConfig,
        ) -> Result<AmqpManagementOutput, String> {
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
            Err("拒绝Failed".to_string())
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

    fn create_test_node(uuid: &str, name: &str, connection_id: &str, delivery_id: u64) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp10_reject".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Amqp10Reject(Amqp10RejectConfig {
                connection_id: connection_id.to_string(),
                delivery_id,
                error_condition: Some("amqp:internal-error".to_string()),
                error_description: Some("消息处理Failed".to_string()),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

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

    /// 测试：拒绝Success时路由到 true 端口
    #[tokio::test]
    async fn test_amqp10_reject_success_routes_to_true() {
        let executor = Amqp10RejectExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = create_test_node("test-1", "拒绝测试", "conn-1", 42);
        let mut context = create_context("test-flow");
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);
        assert_eq!(
            data.unwrap_or_else(|| panic!("expected Some"))["success"],
            true
        );
    }

    /// 测试：拒绝Failed时路由到 false 端口
    #[tokio::test]
    async fn test_amqp10_reject_failure_routes_to_false() {
        let executor =
            Amqp10RejectExecutor::new(Arc::new(MockAmqpProvider::new_failure("Link error")));
        let node = create_test_node("test-2", "Failed测试", "conn-1", 42);
        let mut context = create_context("test-flow");
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(
            data.unwrap_or_else(|| panic!("expected Some"))["error"],
            "Link error"
        );
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_amqp10_reject_wrong_node_data_type() {
        let executor = Amqp10RejectExecutor::new(Arc::new(MockAmqpProvider::new_success()));
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
            .contains("Amqp10RejectConfig"));
    }

    /// 测试：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp10_reject_provider_error_routes_to_false() {
        let executor = Amqp10RejectExecutor::new(Arc::new(MockAmqpProviderError));
        let node = create_test_node("test-err", "异常测试", "conn-1", 42);
        let mut context = create_context("test-flow");
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(
            data.unwrap_or_else(|| panic!("expected Some"))["error"],
            "拒绝Failed"
        );
    }

    /// 测试：输出数据包含 `error_condition` 和 `error_description`
    #[tokio::test]
    async fn test_amqp10_reject_output_contains_error_info() {
        let executor = Amqp10RejectExecutor::new(Arc::new(MockAmqpProvider::new_success()));
        let node = create_test_node("test-info", "Error信息测试", "conn-1", 55);
        let mut context = create_context("test-flow");
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let data = result
            .unwrap_or_else(|e| panic!("{e}"))
            .1
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["request"]["delivery_id"], 55);
        assert_eq!(data["request"]["error_condition"], "amqp:internal-error");
        assert_eq!(data["request"]["error_description"], "消息处理Failed");
    }
}
