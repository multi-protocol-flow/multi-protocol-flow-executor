//! AMQP 声明队列节点执行器
//!
//! 实现 AMQP 0-9-1 队列声明、变量替换和输出构建功能。
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
use flow_engine_core::node_config::{AmqpQueueDeclareConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 声明队列节点执行器
pub struct AmqpQueueDeclareExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpQueueDeclareExecutor {
    /// 创建新的 AMQP 声明队列执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpQueueDeclareExecutor {
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
            NodeData::AmqpQueueDeclare(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "节点数据类型不是 AmqpQueueDeclareConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（connection_id）
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

        // 3. 变量替换（queue）
        let queue = match &config.queue {
            Some(q) => {
                let replaced = replace_variables(q, context).map_err(|e| {
                    let error_msg = format!("队列名称变量替换失败: {}", e);
                    log::error!(
                        "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                        context.flow_uuid,
                        node.uuid,
                        node.name,
                        error_msg
                    );
                    ErrorDetail::execution_error(error_msg)
                })?;
                Some(replaced)
            }
            None => None,
        };

        // 4. 变量替换（arguments 中的值）
        let resolved_arguments = match &config.arguments {
            Some(args) => {
                let mut resolved = HashMap::new();
                for (k, v) in args {
                    if let Some(s) = v.as_str() {
                        let replaced = replace_variables(s, context)
                            .map_err(|e| ErrorDetail::execution_error(format!("arguments[{}] 变量替换失败: {}", k, e)))?;
                        resolved.insert(k.clone(), json!(replaced));
                    } else {
                        resolved.insert(k.clone(), v.clone());
                    }
                }
                Some(resolved)
            }
            None => None,
        };

        // 5. 日志：开始声明
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp_queue_declare, queue: {:?}, durable: {}, exclusive: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            queue,
            config.durable,
            config.exclusive,
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "AMQP QueueDeclare 节点 {} 开始执行 (queue={:?})",
                    node.name, queue
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

        // 6. 保存原始配置值用于报告
        let original_passive = config.passive;
        let original_durable = config.durable;
        let original_exclusive = config.exclusive;
        let original_auto_delete = config.auto_delete;

        // 7. 构建声明配置（使用替换后的值）
        let declare_config = AmqpQueueDeclareConfig {
            connection_id: connection_id.clone(),
            queue: queue.clone(),
            arguments: resolved_arguments,
            ..config
        };

        // 8. 声明队列
        let declare_result = self
            .amqp_manager
            .queue_declare(&connection_id, &declare_config)
            .await;

        let duration = start_time.elapsed();

        match declare_result {
            Ok(output) => {
                if output.success {
                    log::info!(
                        "[AMQP] 队列声明成功: uuid={}, queue={:?}, message_count={:?}, consumer_count={:?}",
                        node.uuid,
                        output.queue_name,
                        output.message_count,
                        output.consumer_count,
                    );
                } else {
                    log::warn!(
                        "[AMQP] 队列声明失败: uuid={}, queue={:?}, error={:?}",
                        node.uuid,
                        queue,
                        output.error,
                    );
                }

                // 9. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "queue_name": output.queue_name,
                    "message_count": output.message_count,
                    "consumer_count": output.consumer_count,
                    "request": {
                        "connection_id": connection_id,
                        "queue": queue,
                        "passive": original_passive,
                        "durable": original_durable,
                        "exclusive": original_exclusive,
                        "auto_delete": original_auto_delete,
                        "arguments": declare_config.arguments,
                    },
                    "timing": {
                        "declare_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 10. 日志：声明完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "AMQP 队列声明成功: {} ({}ms)",
                        output
                            .queue_name
                            .as_deref()
                            .unwrap_or("(auto-generated)"),
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP 队列声明失败: {} ({}ms)",
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

                // 11. 路由：成功走 true 端口，失败走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 队列声明异常: uuid={}, queue={:?}, error={}",
                    node.uuid,
                    queue,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 队列声明异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "queue_name": serde_json::Value::Null,
                    "message_count": serde_json::Value::Null,
                    "consumer_count": serde_json::Value::Null,
                    "request": {
                        "connection_id": connection_id,
                        "queue": queue,
                    },
                    "timing": {
                        "declare_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回错误而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("AMQP queue_declare 失败: {}", e)));
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
        AmqpConsumeConfig, AmqpConnectConfig, AmqpExchangeDeclareConfig,
        AmqpExchangeDeleteConfig, AmqpGetConfig, AmqpManagementConfig, AmqpPublishConfig,
        AmqpQueueBindConfig, AmqpQueueDeleteConfig,
    };
    use flow_engine_core::traits::{
        AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider, AmqpQueueDeclareOutput,
    };

    /// Mock AmqpProvider（queue_declare 成功）
    struct MockAmqpProvider {
        output: AmqpQueueDeclareOutput,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self {
                output: AmqpQueueDeclareOutput {
                    success: true,
                    queue_name: Some("test-queue".to_string()),
                    message_count: Some(5),
                    consumer_count: Some(2),
                    error: None,
                },
            }
        }

        fn new_auto_generated() -> Self {
            Self {
                output: AmqpQueueDeclareOutput {
                    success: true,
                    queue_name: Some("amq.gen-xyz123".to_string()),
                    message_count: Some(0),
                    consumer_count: Some(0),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpQueueDeclareOutput {
                    success: false,
                    queue_name: None,
                    message_count: None,
                    consumer_count: None,
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
        ) -> Result<AmqpQueueDeclareOutput, String> {
            Ok(self.output.clone())
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

    /// Mock AmqpProvider（queue_declare 返回 Err）
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
        ) -> Result<AmqpQueueDeclareOutput, String> {
            Err("连接不存在".to_string())
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
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, queue: Option<&str>) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_queue_declare".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueDeclare(AmqpQueueDeclareConfig {
                connection_id: connection_id.to_string(),
                queue: queue.map(|s| s.to_string()),
                durable: true,
                ..AmqpQueueDeclareConfig::default()
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

    /// 测试：声明成功时输出 JSON 结构正确，路由到 true 端口
    #[tokio::test]
    async fn test_queue_declare_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "声明队列", "conn-1", Some("my-queue"));
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
        assert_eq!(data["queue_name"], "test-queue");
        assert_eq!(data["message_count"], 5);
        assert_eq!(data["consumer_count"], 2);
        assert!(data["timing"]["declare_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert!(data["error"].is_null());
    }

    /// 测试：声明失败时路由到 false 端口
    #[tokio::test]
    async fn test_queue_declare_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("Queue already exists"));
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "失败测试", "conn-1", Some("my-queue"));
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
        assert_eq!(data["error"], "Queue already exists");
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_queue_declare_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().contains("AmqpQueueDeclareConfig"));
    }

    /// 测试：变量替换在 queue 和 connection_id 中生效
    #[tokio::test]
    async fn test_queue_declare_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp_queue_declare".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueDeclare(AmqpQueueDeclareConfig {
                connection_id: "{{conn_id}}".to_string(),
                queue: Some("{{queue_name}}".to_string()),
                durable: true,
                ..AmqpQueueDeclareConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("amqp-conn-789"),
            String::new(),
        );
        context.variables.set_variable(
            "queue_name".to_string(),
            json!("my.dynamic.queue"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["request"]["connection_id"], "amqp-conn-789");
        assert_eq!(data["request"]["queue"], "my.dynamic.queue");
    }

    /// 测试：自动生成队列名称（queue 为 None）
    #[tokio::test]
    async fn test_queue_declare_auto_generated_name() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_auto_generated());
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);
        let node = create_test_node("test-auto", "自动生成", "conn-1", None);
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
        assert_eq!(data["queue_name"], "amq.gen-xyz123");
        assert_eq!(data["message_count"], 0);
        assert_eq!(data["consumer_count"], 0);
    }

    /// 测试：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_queue_declare_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", "conn-1", Some("my-queue"));
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
    async fn test_queue_declare_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", Some("my-queue"));
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
    async fn test_queue_declare_report_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueDeclareExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "amqp_queue_declare".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueDeclare(AmqpQueueDeclareConfig {
                connection_id: "my-conn".to_string(),
                queue: Some("test.queue".to_string()),
                passive: false,
                durable: true,
                exclusive: false,
                auto_delete: false,
                arguments: Some(HashMap::from([
                    ("x-message-ttl".to_string(), json!(60000)),
                    ("x-max-length".to_string(), json!(1000)),
                ])),
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
        let data = output_data.expect("应有输出数据");

        assert!(data.get("success").is_some());
        assert!(data.get("queue_name").is_some());
        assert!(data.get("message_count").is_some());
        assert!(data.get("consumer_count").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        let request = &data["request"];
        assert_eq!(request["connection_id"], "my-conn");
        assert_eq!(request["queue"], "test.queue");
        assert_eq!(request["durable"], true);
        assert_eq!(request["passive"], false);
        assert_eq!(request["exclusive"], false);
        assert_eq!(request["auto_delete"], false);
        assert_eq!(request["arguments"]["x-message-ttl"], 60000);
        assert_eq!(request["arguments"]["x-max-length"], 1000);
    }
}
