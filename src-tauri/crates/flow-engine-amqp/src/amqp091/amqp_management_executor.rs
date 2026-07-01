//! AMQP Management HTTP API 执行器
//!
//! 通过 RabbitMQ Management HTTP API (端口 15672) 执行管理操作。
//! 支持 overview、queue_info、list_queues、list_connections、list_channels、
//! list_exchanges、list_consumers 等 action。
//!
//! 仅在 `management` feature 启用时编译。

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
use flow_engine_core::node_config::{AmqpManagementConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP Management 执行器
pub struct AmqpManagementExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpManagementExecutor {
    /// 创建新的 Management 执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpManagementExecutor {
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
            NodeData::AmqpManagement(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 AmqpManagementConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换
        let management_url = replace_variables(&config.management_url, context).map_err(|e| {
            let error_msg = format!("management_url 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let action = replace_variables(&config.action, context).map_err(|e| {
            let error_msg = format!("action 变量替换失败: {}", e);
            ErrorDetail::execution_error(error_msg)
        })?;

        let vhost = config
            .vhost
            .as_deref()
            .map(|v| replace_variables(v, context))
            .transpose()
            .map_err(|e| ErrorDetail::execution_error(format!("vhost 变量替换失败: {}", e)))?;

        let queue_name = config
            .queue_name
            .as_deref()
            .map(|v| replace_variables(v, context))
            .transpose()
            .map_err(|e| ErrorDetail::execution_error(format!("queue_name 变量替换失败: {}", e)))?;

        let replaced_config = AmqpManagementConfig {
            management_url,
            username: config.username.clone(),
            password: config.password.clone(),
            action,
            vhost,
            queue_name,
            connection_id: config.connection_id.clone(),
        };

        // 3. 日志：开始执行
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp_management, action: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            replaced_config.action
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "AMQP Management 节点 {} 开始执行, action: {}",
                    node.name, replaced_config.action
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

        // 4. 调用 management 方法
        let management_result = self
            .amqp_manager
            .management(&replaced_config.connection_id, &replaced_config)
            .await;

        match management_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[AMQP Management] 操作成功: uuid={}, action={}, 耗时={}ms",
                        node.uuid,
                        replaced_config.action,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[AMQP Management] 操作失败: uuid={}, action={}, error={:?}",
                        node.uuid,
                        replaced_config.action,
                        output.error
                    );
                }

                // 5. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "data": output.data,
                    "request": {
                        "management_url": replaced_config.management_url,
                        "action": replaced_config.action,
                        "vhost": replaced_config.vhost,
                        "queue_name": replaced_config.queue_name,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 6. 日志：完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "AMQP Management 操作成功 ({}ms)",
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP Management 操作失败: {} ({}ms)",
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

                // 7. 路由
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP Management] 操作异常: uuid={}, action={}, error={}",
                    node.uuid,
                    replaced_config.action,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP Management 操作异常: {}", e),
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
                    "data": serde_json::Value::Null,
                    "request": {
                        "management_url": replaced_config.management_url,
                        "action": replaced_config.action,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回错误而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(format!("AMQP management 失败: {}", e));
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
        AmqpManagementOutput, AmqpProvider,
    };

    /// Mock AmqpProvider — 管理操作成功
    struct MockManagementProvider {
        output: AmqpManagementOutput,
    }

    impl MockManagementProvider {
        fn new_success(data: serde_json::Value) -> Self {
            Self {
                output: AmqpManagementOutput {
                    success: true,
                    data: Some(data),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpManagementOutput {
                    success: false,
                    data: None,
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl AmqpProvider for MockManagementProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<flow_engine_core::traits::AmqpConnectOutput, String> {
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

        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> {
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
        ) -> Result<AmqpManagementOutput, String> {
            Ok(self.output.clone())
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

        fn list_connections(&self) -> Vec<flow_engine_core::traits::AmqpConnectionInfo> {
            vec![]
        }
    }

    /// Mock AmqpProvider — management 方法返回 Err
    struct MockManagementProviderError;

    #[async_trait::async_trait]
    impl AmqpProvider for MockManagementProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<flow_engine_core::traits::AmqpConnectOutput, String> {
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

        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> {
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
        ) -> Result<AmqpManagementOutput, String> {
            Err("Management API 连接失败: Connection refused".to_string())
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

        fn list_connections(&self) -> Vec<flow_engine_core::traits::AmqpConnectionInfo> {
            vec![]
        }
    }

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, action: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_management".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpManagement(AmqpManagementConfig {
                management_url: "http://localhost:15672".to_string(),
                username: Some("guest".to_string()),
                password: Some("guest".to_string()),
                action: action.to_string(),
                vhost: Some("/".to_string()),
                queue_name: None,
                connection_id: String::new(),
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

    /// 测试 1：overview 操作成功时输出 JSON 结构正确
    #[tokio::test]
    async fn test_amqp_management_overview_success() {
        let overview_data = json!({
            "rabbitmq_version": "3.13.0",
            "management_version": "3.13.0",
            "cluster_name": "rabbit@localhost",
        });
        let provider = Arc::new(MockManagementProvider::new_success(overview_data));
        let executor = AmqpManagementExecutor::new(provider);
        let node = create_test_node("test-1", "Overview 测试", "overview");
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
        assert!(data["data"]["rabbitmq_version"].is_string());
        assert!(data["timing"]["total_ms"].is_number());
        assert!(data["error"].is_null());
    }

    /// 测试 2：操作失败时路由到 false 端口
    #[tokio::test]
    async fn test_amqp_management_failure_routes_to_false() {
        let provider = Arc::new(MockManagementProvider::new_failure("队列不存在"));
        let executor = AmqpManagementExecutor::new(provider);
        let node = create_test_node("test-2", "失败测试", "queue_info");
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
    }

    /// 测试 3：provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp_management_provider_error_routes_to_false() {
        let provider = Arc::new(MockManagementProviderError);
        let executor = AmqpManagementExecutor::new(provider);
        let node = create_test_node("test-3", "异常测试", "list_queues");
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
        assert_eq!(data["error"], "Management API 连接失败: Connection refused");
    }

    /// 测试 4：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_amqp_management_wrong_node_data_type() {
        let provider = Arc::new(MockManagementProvider::new_success(json!(null)));
        let executor = AmqpManagementExecutor::new(provider);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("AmqpManagementConfig"));
    }

    /// 测试 5：连接端口路由
    #[tokio::test]
    async fn test_amqp_management_with_connections_routing() {
        let provider = Arc::new(MockManagementProvider::new_success(json!({"items": []})));
        let executor = AmqpManagementExecutor::new(provider);
        let node = create_test_node("test-route", "路由测试", "list_exchanges");
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

    /// 测试 6：变量替换在 management_url 和 action 中生效
    #[tokio::test]
    async fn test_amqp_management_with_variable_replacement() {
        let provider = Arc::new(MockManagementProvider::new_success(json!({"queues": 5})));
        let executor = AmqpManagementExecutor::new(provider);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp_management".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpManagement(AmqpManagementConfig {
                management_url: "http://{{host}}:{{port}}".to_string(),
                username: Some("guest".to_string()),
                password: Some("guest".to_string()),
                action: "list_{{resource}}".to_string(),
                vhost: Some("/".to_string()),
                queue_name: None,
                connection_id: String::new(),
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("host".to_string(), json!("rabbitmq.example.com"), String::new());
        context
            .variables
            .set_variable("port".to_string(), json!("15672"), String::new());
        context
            .variables
            .set_variable("resource".to_string(), json!("queues"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");
        // 验证变量替换生效
        assert_eq!(
            data["request"]["management_url"],
            "http://rabbitmq.example.com:15672"
        );
        assert_eq!(data["request"]["action"], "list_queues");
    }

    /// 测试 7：输出数据结构包含所有必要字段
    #[tokio::test]
    async fn test_amqp_management_output_structure_complete() {
        let list_data = json!([
            {"name": "queue1", "messages": 10},
            {"name": "queue2", "messages": 20},
        ]);
        let provider = Arc::new(MockManagementProvider::new_success(list_data));
        let executor = AmqpManagementExecutor::new(provider);

        let node = Node {
            uuid: "test-struct".to_string(),
            node_type: "amqp_management".to_string(),
            name: "结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpManagement(AmqpManagementConfig {
                management_url: "http://localhost:15672".to_string(),
                username: Some("guest".to_string()),
                password: Some("guest".to_string()),
                action: "list_queues".to_string(),
                vhost: Some("/myhost".to_string()),
                queue_name: Some("test-queue".to_string()),
                connection_id: String::new(),
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

        // 验证所有必要字段
        assert!(data.get("success").is_some());
        assert!(data.get("data").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["management_url"], "http://localhost:15672");
        assert_eq!(request["action"], "list_queues");
        assert_eq!(request["vhost"], "/myhost");
        assert_eq!(request["queue_name"], "test-queue");

        // 验证 data 包含队列列表
        assert!(data["data"].is_array());
        assert_eq!(data["data"].as_array().expect("应为数组").len(), 2);
    }
}
