//! AMQP 确认消息节点执行器
//!
//! 实现 AMQP 0-9-1 消息确认/拒绝功能。
//! 支持 `basic_ack(multiple)、basic_nack(multiple`, `requeue)、basic_reject(requeue`)。
//! 成功走 true 端口，失败走 false 端口。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{AmqpAckConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 确认消息节点执行器
pub struct AmqpAckExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpAckExecutor {
    /// 创建新的 AMQP 确认消息执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpAckExecutor {
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
            NodeData::AmqpAck(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 AmqpAckConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换 connection_id 和 action
        let connection_id =
            replace_variables(&config.connection_id, context).map_err(|e| {
                let error_msg = format!("connection_id 变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?;

        let action = replace_variables(&config.action, context).map_err(|e| {
            let error_msg = format!("action 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 验证 action 值
        if !matches!(action.as_str(), "ack" | "nack" | "reject") {
            let error_msg = format!(
                "无效的 ack action: '{}', 只支持 ack/nack/reject",
                action
            );
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            return Err(error_msg);
        }

        // 4. 日志：开始执行
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp_ack, 连接ID: {}, action: {}, delivery_tag: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            action,
            config.delivery_tag
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "AMQP Ack 节点 {} 开始执行 (action={}, delivery_tag={})",
                    node.name, action, config.delivery_tag
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

        // 5. 构建确认配置（使用替换后的值）
        let ack_config = AmqpAckConfig {
            connection_id: connection_id.clone(),
            action: action.clone(),
            delivery_tag: config.delivery_tag,
            multiple: config.multiple,
            requeue: config.requeue,
        };

        // 6. 执行确认操作
        let ack_result = self.amqp_manager.ack(&connection_id, &ack_config).await;

        match ack_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[AMQP] 确认成功: connection_id={}, action={}, delivery_tag={}",
                        connection_id,
                        action,
                        config.delivery_tag
                    );
                } else {
                    log::warn!(
                        "[AMQP] 确认失败: connection_id={}, action={}, delivery_tag={}, error={:?}",
                        connection_id,
                        action,
                        config.delivery_tag,
                        output.error
                    );
                }

                // 7. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "action": output.action.unwrap_or_else(|| action.clone()),
                    "delivery_tag": output.delivery_tag.unwrap_or(config.delivery_tag),
                    "request": {
                        "connection_id": connection_id,
                        "action": action,
                        "delivery_tag": config.delivery_tag,
                        "multiple": config.multiple,
                        "requeue": config.requeue,
                    },
                    "timing": {
                        "ack_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 8. 日志：执行完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "AMQP {} 成功 (delivery_tag={}, {}ms)",
                        action,
                        config.delivery_tag,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP {} 失败: {} ({}ms)",
                        action,
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
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9. 路由：成功走 true 端口，失败走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[AMQP] 确认异常: connection_id={}, action={}, error={}",
                    connection_id,
                    action,
                    e
                );

                // 日志：执行异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP {} 异常: {}", action, e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据
                let error_output = json!({
                    "success": false,
                    "action": action,
                    "delivery_tag": config.delivery_tag,
                    "request": {
                        "connection_id": connection_id,
                        "action": ack_config.action,
                        "delivery_tag": config.delivery_tag,
                        "multiple": config.multiple,
                        "requeue": config.requeue,
                    },
                    "timing": {
                        "ack_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 返回 false 端口
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
        Amqp10RejectConfig, Amqp10SendConfig, AmqpCloseConfig, AmqpConnectConfig,
        AmqpConsumeConfig, AmqpExchangeDeclareConfig, AmqpExchangeDeleteConfig, AmqpGetConfig,
        AmqpManagementConfig, AmqpPublishConfig, AmqpQueueBindConfig, AmqpQueueDeclareConfig,
        AmqpQueueDeleteConfig,
    };
    use flow_engine_core::traits::{AmqpAckOutput, AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider};

    /// Mock AmqpProvider（ack 成功）
    struct MockAmqpProvider {
        output: AmqpAckOutput,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: AmqpAckOutput {
                    success: true,
                    action: None,
                    delivery_tag: None,
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpAckOutput {
                    success: false,
                    action: None,
                    delivery_tag: None,
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
                connection_id: String::new(),
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
        ) -> Result<AmqpAckOutput, String> {
            Ok(self.output.clone())
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

    /// Mock AmqpProvider（ack 返回 Err）
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
        ) -> Result<AmqpAckOutput, String> {
            Err("连接不存在".to_string())
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
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, action: &str, delivery_tag: u64) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_ack".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpAck(AmqpAckConfig {
                connection_id: connection_id.to_string(),
                action: action.to_string(),
                delivery_tag,
                multiple: false,
                requeue: false,
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
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None, redis_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：ack 成功时输出 JSON 结构正确并路由到 true 端口
    #[tokio::test]
    async fn test_amqp_ack_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpAckExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "确认测试", "conn-1", "ack", 42);
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
        assert_eq!(data["action"], "ack");
        assert_eq!(data["delivery_tag"], 42);
        assert!(data["timing"]["ack_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert!(data["error"].is_null());
        assert_eq!(data["request"]["connection_id"], "conn-1");
        assert_eq!(data["request"]["action"], "ack");
        assert_eq!(data["request"]["delivery_tag"], 42);
        assert_eq!(data["request"]["multiple"], false);
        assert_eq!(data["request"]["requeue"], false);
    }

    /// 测试：ack 失败时路由到 false 端口
    #[tokio::test]
    async fn test_amqp_ack_failure_routes_to_false_port() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("未知投递标签"));
        let executor = AmqpAckExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "失败测试", "conn-1", "ack", 99);
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
        assert_eq!(data["error"], "未知投递标签");
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_amqp_ack_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpAckExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().contains("AmqpAckConfig"));
    }

    /// 测试：无效 action 值时返回错误
    #[tokio::test]
    async fn test_amqp_ack_invalid_action() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpAckExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-invalid".to_string(),
            node_type: "amqp_ack".to_string(),
            name: "无效action".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpAck(AmqpAckConfig {
                connection_id: "conn-1".to_string(),
                action: "invalid".to_string(),
                delivery_tag: 1,
                multiple: false,
                requeue: false,
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("无效的 ack action"));
        assert!(err.contains("invalid"));
    }

    /// 测试：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp_ack_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpAckExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "conn-1", "ack", 1);
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

    /// 测试：变量替换在 connection_id 和 action 中生效
    #[tokio::test]
    async fn test_amqp_ack_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpAckExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp_ack".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpAck(AmqpAckConfig {
                connection_id: "{{conn_ref}}".to_string(),
                action: "{{ack_type}}".to_string(),
                delivery_tag: 10,
                multiple: true,
                requeue: false,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_ref".to_string(),
            json!("my-connection"),
            String::new(),
        );
        context.variables.set_variable(
            "ack_type".to_string(),
            json!("nack"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);

        let data = output_data.unwrap();
        assert_eq!(data["action"], "nack");
        assert_eq!(data["request"]["connection_id"], "my-connection");
        assert_eq!(data["request"]["action"], "nack");
    }

    /// 测试：连接端口路由到下游节点
    #[tokio::test]
    async fn test_amqp_ack_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpAckExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", "ack", 1);
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

    /// 测试：reject 操作带 requeue=true 的配置正确传递
    #[tokio::test]
    async fn test_amqp_ack_reject_with_requeue() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpAckExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-reject".to_string(),
            node_type: "amqp_ack".to_string(),
            name: "reject 测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpAck(AmqpAckConfig {
                connection_id: "conn-1".to_string(),
                action: "reject".to_string(),
                delivery_tag: 5,
                multiple: false,
                requeue: true,
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        assert_eq!(data["action"], "reject");
        assert_eq!(data["delivery_tag"], 5);
        assert_eq!(data["request"]["requeue"], true);
        assert_eq!(data["request"]["multiple"], false);
    }
}
