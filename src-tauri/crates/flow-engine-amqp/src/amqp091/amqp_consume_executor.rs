//! AMQP 消费消息节点执行器
//!
//! 实现 AMQP 0-9-1 推模式消费（`basic_consume`），支持多种确认模式和消息批量采集。

use flow_engine_core::error::ErrorDetail;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{AmqpConsumeConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// AMQP 消费消息节点执行器
pub struct AmqpConsumeExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpConsumeExecutor {
    /// 创建新的 AMQP 消费执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpConsumeExecutor {
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
            NodeData::AmqpConsume(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not AmqpConsumeConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（queue, consumer_tag）
        let queue = replace_variables(&config.queue, context).map_err(|e| {
            let error_msg = format!("queue variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let consumer_tag = match &config.consumer_tag {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("consumer_tag variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        // 3. 日志：开始消费
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp_consume, queue: {}, consumer_tag: {:?}",
            context.flow_uuid,
            node.uuid,
            node.name,
            queue,
            consumer_tag
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 消费节点 {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建替换后的配置
        let resolved_config = AmqpConsumeConfig {
            connection_id: config.connection_id.clone(),
            queue,
            consumer_tag,
            ack_mode: config.ack_mode.clone(),
            exclusive: config.exclusive,
            no_local: config.no_local,
            prefetch_count: config.prefetch_count,
            max_messages: config.max_messages,
            timeout_ms: config.timeout_ms,
            auto_ack_on_success: config.auto_ack_on_success,
            auto_reject_on_failure: config.auto_reject_on_failure,
            reject_requeue: config.reject_requeue,
        };

        // 5. 消费消息
        let consume_result = self
            .amqp_manager
            .consume(&config.connection_id, &resolved_config)
            .await;

        match consume_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[AMQP] 消费Success: uuid={}, queue={}, 消息数={}, 耗时={}ms",
                        node.uuid,
                        resolved_config.queue,
                        output.message_count,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[AMQP] 消费Failed: uuid={}, queue={}, error={:?} ({}ms)",
                        node.uuid,
                        resolved_config.queue,
                        output.error,
                        duration.as_millis()
                    );
                }

                // 6. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "messages": output.messages.iter().map(|msg| json!({
                        "delivery_tag": msg.delivery_tag,
                        "exchange": msg.exchange,
                        "routing_key": msg.routing_key,
                        "redelivered": msg.redelivered,
                        "properties": msg.properties,
                        "body": msg.body,
                        "body_decoded": msg.body_decoded,
                    })).collect::<Vec<_>>(),
                    "message_count": output.message_count,
                    "error": output.error,
                    "request": {
                        "connection_id": resolved_config.connection_id,
                        "queue": resolved_config.queue,
                        "consumer_tag": resolved_config.consumer_tag,
                        "ack_mode": resolved_config.ack_mode,
                        "exclusive": resolved_config.exclusive,
                        "no_local": resolved_config.no_local,
                        "prefetch_count": resolved_config.prefetch_count,
                        "max_messages": resolved_config.max_messages,
                        "timeout_ms": resolved_config.timeout_ms,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 7. 日志：消费完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "AMQP 消费Success, 消息数: {} ({}ms)",
                        output.message_count,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP 消费Failed: {} ({}ms)",
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

                // 8. 路由：Success走 true 端口，Failed走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 消费异常: uuid={}, queue={}, error={}",
                    node.uuid,
                    resolved_config.queue,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 消费异常: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "messages": [],
                    "message_count": 0,
                    "error": e,
                    "request": {
                        "connection_id": resolved_config.connection_id,
                        "queue": resolved_config.queue,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "AMQP consume Failed: {}",
                        e
                    )));
                }

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
        Amqp10RejectConfig, Amqp10SendConfig, AmqpAckConfig, AmqpCloseConfig, AmqpConnectConfig,
        AmqpConsumeConfig, AmqpExchangeDeclareConfig, AmqpExchangeDeleteConfig, AmqpGetConfig,
        AmqpManagementConfig, AmqpPublishConfig, AmqpQueueBindConfig, AmqpQueueDeclareConfig,
        AmqpQueueDeleteConfig,
    };
    use flow_engine_core::traits::{
        AmqpConnectOutput, AmqpConnectionInfo, AmqpConsumeOutput, AmqpMessage,
    };

    /// Mock AmqpProvider（消费Success，返回多条消息）
    struct MockAmqpProvider {
        output: AmqpConsumeOutput,
    }

    impl MockAmqpProvider {
        /// 创建Success消费 Mock，返回指定消息列表
        fn new_success(messages: Vec<AmqpMessage>) -> Self {
            let count = messages.len() as u32;
            Self {
                output: AmqpConsumeOutput {
                    success: true,
                    messages,
                    message_count: count,
                    error: None,
                },
            }
        }

        /// 创建Failed消费 Mock
        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpConsumeOutput {
                    success: false,
                    messages: vec![],
                    message_count: 0,
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
        ) -> Result<AmqpConsumeOutput, String> {
            Ok(self.output.clone())
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

    /// Mock AmqpProvider（consume 返回 Err）
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
        ) -> Result<AmqpConsumeOutput, String> {
            Err("连接已断开".to_string())
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
    fn create_test_node(uuid: &str, name: &str, config: AmqpConsumeConfig) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_consume".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpConsume(config),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建默认消费配置
    fn default_consume_config() -> AmqpConsumeConfig {
        AmqpConsumeConfig {
            connection_id: "test-conn".to_string(),
            queue: "test-queue".to_string(),
            consumer_tag: None,
            ack_mode: "auto".to_string(),
            exclusive: false,
            no_local: false,
            prefetch_count: 10,
            max_messages: 1,
            timeout_ms: 5000,
            ..AmqpConsumeConfig::default()
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

    /// 测试：消费Success时输出 JSON 结构正确
    #[tokio::test]
    async fn test_amqp_consume_success_output_structure() {
        let messages = vec![AmqpMessage {
            delivery_tag: Some(1),
            exchange: Some("test-exchange".to_string()),
            routing_key: Some("test.key".to_string()),
            redelivered: Some(false),
            properties: None,
            body: "hello world".to_string(),
            body_decoded: None,
        }];
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(messages));
        let executor = AmqpConsumeExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试消费", default_consume_config());
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
        assert_eq!(data["message_count"], 1);
        assert!(data["error"].is_null());
        assert!(data["timing"]["total_ms"].is_number());

        // 验证消息内容
        let messages = data["messages"].as_array().expect("messages 应为数组");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["delivery_tag"], 1);
        assert_eq!(messages[0]["exchange"], "test-exchange");
        assert_eq!(messages[0]["routing_key"], "test.key");
        assert_eq!(messages[0]["redelivered"], false);
        assert_eq!(messages[0]["body"], "hello world");
    }

    /// 测试：消费多条消息
    #[tokio::test]
    async fn test_amqp_consume_multiple_messages() {
        let messages = vec![
            AmqpMessage {
                delivery_tag: Some(1),
                exchange: Some("ex".to_string()),
                routing_key: Some("rk1".to_string()),
                redelivered: Some(false),
                properties: None,
                body: "msg1".to_string(),
                body_decoded: None,
            },
            AmqpMessage {
                delivery_tag: Some(2),
                exchange: Some("ex".to_string()),
                routing_key: Some("rk2".to_string()),
                redelivered: Some(true),
                properties: None,
                body: "msg2".to_string(),
                body_decoded: None,
            },
            AmqpMessage {
                delivery_tag: Some(3),
                exchange: Some("ex".to_string()),
                routing_key: Some("rk3".to_string()),
                redelivered: Some(false),
                properties: None,
                body: "msg3".to_string(),
                body_decoded: None,
            },
        ];
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(messages));
        let executor = AmqpConsumeExecutor::new(amqp_manager);
        let node = create_test_node("test-multi", "多条消息", default_consume_config());
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);

        let data = output_data.unwrap();
        assert_eq!(data["message_count"], 3);
        let messages = data["messages"].as_array().expect("messages 应为数组");
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0]["body"], "msg1");
        assert_eq!(messages[1]["body"], "msg2");
        assert_eq!(messages[2]["body"], "msg3");
    }

    /// 测试：消费Failed时路由到 false 端口
    #[tokio::test]
    async fn test_amqp_consume_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("队列不存在"));
        let executor = AmqpConsumeExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "Failed测试", default_consume_config());
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
        assert_eq!(data["error"], "队列不存在");
        assert_eq!(data["message_count"], 0);
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_amqp_consume_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(vec![]));
        let executor = AmqpConsumeExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().message.contains("AmqpConsumeConfig"));
    }

    /// 测试：变量替换在 queue 和 `consumer_tag` 中生效
    #[tokio::test]
    async fn test_amqp_consume_variable_replacement() {
        let messages = vec![AmqpMessage {
            delivery_tag: Some(1),
            exchange: None,
            routing_key: Some("key".to_string()),
            redelivered: None,
            properties: None,
            body: "replaced".to_string(),
            body_decoded: None,
        }];
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(messages));
        let executor = AmqpConsumeExecutor::new(amqp_manager);

        let config = AmqpConsumeConfig {
            connection_id: "conn-1".to_string(),
            queue: "{{env}}-orders".to_string(),
            consumer_tag: Some("consumer-{{region}}".to_string()),
            ..AmqpConsumeConfig::default()
        };
        let node = create_test_node("test-var", "变量替换", config);

        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("env".to_string(), json!("prod"), String::new());
        context
            .variables
            .set_variable("region".to_string(), json!("cn"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // 验证变量替换生效
        assert_eq!(data["request"]["queue"], "prod-orders");
        assert_eq!(data["request"]["consumer_tag"], "consumer-cn");
    }

    /// 测试：provider consume 方法返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp_consume_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpConsumeExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", default_consume_config());
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
        assert_eq!(data["error"], "连接已断开");
        assert_eq!(data["message_count"], 0);
    }

    /// 测试：输出数据结构包含所有必要字段
    #[tokio::test]
    async fn test_amqp_consume_report_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(vec![]));
        let executor = AmqpConsumeExecutor::new(amqp_manager);

        let config = AmqpConsumeConfig {
            connection_id: "conn-report".to_string(),
            queue: "report-queue".to_string(),
            consumer_tag: Some("report-consumer".to_string()),
            ack_mode: "manual".to_string(),
            exclusive: true,
            no_local: true,
            prefetch_count: 50,
            max_messages: 10,
            timeout_ms: 30000,
            ..AmqpConsumeConfig::default()
        };
        let node = create_test_node("test-report", "报告结构", config);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        // 验证顶层字段
        assert!(data.get("success").is_some());
        assert!(data.get("messages").is_some());
        assert!(data.get("message_count").is_some());
        assert!(data.get("error").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["connection_id"], "conn-report");
        assert_eq!(request["queue"], "report-queue");
        assert_eq!(request["consumer_tag"], "report-consumer");
        assert_eq!(request["ack_mode"], "manual");
        assert_eq!(request["exclusive"], true);
        assert_eq!(request["no_local"], true);
        assert_eq!(request["prefetch_count"], 50);
        assert_eq!(request["max_messages"], 10);
        assert_eq!(request["timeout_ms"], 30000);

        // 验证 timing 子结构
        let timing = &data["timing"];
        assert!(timing["total_ms"].is_number());
    }

    /// 测试：端口路由（true 端口连接到下一个节点）
    #[tokio::test]
    async fn test_amqp_consume_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(vec![]));
        let executor = AmqpConsumeExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", default_consume_config());
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
