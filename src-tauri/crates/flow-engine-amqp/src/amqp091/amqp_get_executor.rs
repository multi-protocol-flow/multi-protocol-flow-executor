//! AMQP 获取消息节点执行器
//!
//! 实现 AMQP 0-9-1 拉模式获取（`basic_get`），获取单条消息或返回队列为空。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{AmqpGetConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 获取消息节点执行器
pub struct AmqpGetExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpGetExecutor {
    /// 创建新的 AMQP 获取执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpGetExecutor {
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
            NodeData::AmqpGet(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 AmqpGetConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换（queue）
        let queue = replace_variables(&config.queue, context).map_err(|e| {
            let error_msg = format!("queue 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 日志：开始获取
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp_get, queue: {}, no_ack: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            queue,
            config.no_ack
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 获取节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建替换后的配置
        let resolved_config = AmqpGetConfig {
            connection_id: config.connection_id.clone(),
            queue,
            no_ack: config.no_ack,
        };

        // 5. 获取消息
        let get_result = self
            .amqp_manager
            .get(&config.connection_id, &resolved_config)
            .await;

        match get_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[AMQP] 获取成功: uuid={}, queue={}, 队列剩余={}, 耗时={}ms",
                        node.uuid,
                        resolved_config.queue,
                        output.message_count_remaining.unwrap_or(0),
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[AMQP] 获取失败: uuid={}, queue={}, error={:?} ({}ms)",
                        node.uuid,
                        resolved_config.queue,
                        output.error,
                        duration.as_millis()
                    );
                }

                // 6. 构建输出数据
                let message_json = output.message.as_ref().map(|msg| {
                    json!({
                        "delivery_tag": msg.delivery_tag,
                        "exchange": msg.exchange,
                        "routing_key": msg.routing_key,
                        "redelivered": msg.redelivered,
                        "properties": msg.properties,
                        "body": msg.body,
                        "body_decoded": msg.body_decoded,
                    })
                });

                let output_data = json!({
                    "success": output.success,
                    "message": message_json,
                    "message_count_remaining": output.message_count_remaining,
                    "error": output.error,
                    "request": {
                        "connection_id": resolved_config.connection_id,
                        "queue": resolved_config.queue,
                        "no_ack": resolved_config.no_ack,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 7. 日志：获取完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    let has_msg = output.message.is_some();
                    format!(
                        "AMQP 获取成功, {} ({}ms)",
                        if has_msg { "获取到消息" } else { "队列为空" },
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP 获取失败: {} ({}ms)",
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

                // 8. 路由：成功走 true 端口，失败走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 获取异常: uuid={}, queue={}, error={}",
                    node.uuid,
                    resolved_config.queue,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 获取异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "message": serde_json::Value::Null,
                    "message_count_remaining": serde_json::Value::Null,
                    "error": e,
                    "request": {
                        "connection_id": resolved_config.connection_id,
                        "queue": resolved_config.queue,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
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
        Amqp10RejectConfig, Amqp10SendConfig, AmqpAckConfig, AmqpCloseConfig,
        AmqpConnectConfig, AmqpConsumeConfig, AmqpExchangeDeclareConfig, AmqpExchangeDeleteConfig,
        AmqpGetConfig, AmqpManagementConfig, AmqpPublishConfig, AmqpQueueBindConfig,
        AmqpQueueDeclareConfig, AmqpQueueDeleteConfig,
    };
    use flow_engine_core::traits::{
        AmqpConnectOutput, AmqpConnectionInfo, AmqpGetOutput, AmqpMessage,
    };

    /// Mock AmqpProvider（获取成功，返回消息）
    struct MockAmqpProvider {
        output: AmqpGetOutput,
    }

    impl MockAmqpProvider {
        /// 创建成功获取 Mock，返回指定消息
        fn new_success_with_message(message: AmqpMessage, remaining: u32) -> Self {
            Self {
                output: AmqpGetOutput {
                    success: true,
                    message: Some(message),
                    message_count_remaining: Some(remaining),
                    error: None,
                },
            }
        }

        /// 创建成功获取 Mock，队列为空
        fn new_success_empty_queue() -> Self {
            Self {
                output: AmqpGetOutput {
                    success: true,
                    message: None,
                    message_count_remaining: Some(0),
                    error: None,
                },
            }
        }

        /// 创建失败获取 Mock
        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpGetOutput {
                    success: false,
                    message: None,
                    message_count_remaining: None,
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl flow_engine_core::traits::AmqpProvider for MockAmqpProvider {
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
        ) -> Result<AmqpGetOutput, String> {
            Ok(self.output.clone())
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

    /// Mock AmqpProvider（get 返回 Err）
    struct MockAmqpProviderError;

    #[async_trait::async_trait]
    impl flow_engine_core::traits::AmqpProvider for MockAmqpProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<AmqpConnectOutput, String> {
            Ok(AmqpConnectOutput {
                success: true,
                connection_id: "test".to_string(),
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
        ) -> Result<AmqpGetOutput, String> {
            Err("通道未打开".to_string())
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
    fn create_test_node(uuid: &str, name: &str, config: AmqpGetConfig) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_get".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpGet(config),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建默认获取配置
    fn default_get_config() -> AmqpGetConfig {
        AmqpGetConfig {
            connection_id: "test-conn".to_string(),
            queue: "test-queue".to_string(),
            no_ack: false,
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
            imap_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：获取成功时输出 JSON 结构正确
    #[tokio::test]
    async fn test_amqp_get_success_with_message() {
        let message = AmqpMessage {
            delivery_tag: Some(42),
            exchange: Some("my-exchange".to_string()),
            routing_key: Some("my.routing.key".to_string()),
            redelivered: Some(false),
            properties: Some(json!({"content_type": "application/json"})),
            body: "{\"order_id\":123}".to_string(),
            body_decoded: Some(json!({"order_id": 123})),
        };
        let amqp_manager = Arc::new(MockAmqpProvider::new_success_with_message(message, 5));
        let executor = AmqpGetExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试获取", default_get_config());
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 成功应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["message_count_remaining"], 5);
        assert!(data["error"].is_null());
        assert!(data["timing"]["total_ms"].is_number());

        // 验证消息内容
        let msg = &data["message"];
        assert!(msg.is_object());
        assert_eq!(msg["delivery_tag"], 42);
        assert_eq!(msg["exchange"], "my-exchange");
        assert_eq!(msg["routing_key"], "my.routing.key");
        assert_eq!(msg["redelivered"], false);
        assert_eq!(msg["body"], "{\"order_id\":123}");
        assert_eq!(msg["body_decoded"]["order_id"], 123);
    }

    /// 测试：队列为空时仍走 true 端口
    #[tokio::test]
    async fn test_amqp_get_success_empty_queue() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success_empty_queue());
        let executor = AmqpGetExecutor::new(amqp_manager);
        let node = create_test_node("test-empty", "空队列", default_get_config());
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 成功（即使无消息）仍走 true 端口
        assert_eq!(next_nodes, vec!["true"]);

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["message_count_remaining"], 0);
        // 消息应为 null
        assert!(data["message"].is_null());
    }

    /// 测试：获取失败时路由到 false 端口
    #[tokio::test]
    async fn test_amqp_get_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("队列不存在"));
        let executor = AmqpGetExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "失败测试", default_get_config());
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 失败应路由到 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "队列不存在");
        assert!(data["message"].is_null());
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_amqp_get_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success_empty_queue());
        let executor = AmqpGetExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().contains("AmqpGetConfig"));
    }

    /// 测试：变量替换在 queue 中生效
    #[tokio::test]
    async fn test_amqp_get_variable_replacement() {
        let message = AmqpMessage {
            delivery_tag: Some(1),
            exchange: None,
            routing_key: Some("key".to_string()),
            redelivered: None,
            properties: None,
            body: "replaced".to_string(),
            body_decoded: None,
        };
        let amqp_manager = Arc::new(MockAmqpProvider::new_success_with_message(message, 0));
        let executor = AmqpGetExecutor::new(amqp_manager);

        let config = AmqpGetConfig {
            connection_id: "conn-1".to_string(),
            queue: "{{env}}-orders".to_string(),
            no_ack: true,
        };
        let node = create_test_node("test-var", "变量替换", config);

        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("env".to_string(), json!("staging"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // 验证变量替换生效
        assert_eq!(data["request"]["queue"], "staging-orders");
        assert_eq!(data["request"]["no_ack"], true);
    }

    /// 测试：provider get 方法返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp_get_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpGetExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", default_get_config());
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
        assert_eq!(data["error"], "通道未打开");
    }

    /// 测试：端口路由（true 端口连接到下一个节点）
    #[tokio::test]
    async fn test_amqp_get_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success_empty_queue());
        let executor = AmqpGetExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", default_get_config());
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
        assert_eq!(next_nodes, vec!["next-node"]);
    }
}
