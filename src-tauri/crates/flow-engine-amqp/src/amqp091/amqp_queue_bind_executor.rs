//! AMQP 绑定队列节点执行器
//!
//! 实现 AMQP 0-9-1 队列绑定到交换器、变量替换和输出构建功能。
//! 成功路由到 true 端口，失败路由到 false 端口。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{AmqpQueueBindConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 绑定队列节点执行器
pub struct AmqpQueueBindExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpQueueBindExecutor {
    /// 创建新的 AMQP 绑定队列执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpQueueBindExecutor {
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
            NodeData::AmqpQueueBind(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 AmqpQueueBindConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换（connection_id、queue、exchange）
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        let queue = replace_variables(&config.queue, context).map_err(|e| {
            let error_msg = format!("队列名称变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
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
            error_msg
        })?;

        // 3. 变量替换（routing_key）
        let routing_key = match &config.routing_key {
            Some(rk) => {
                let replaced = replace_variables(rk, context).map_err(|e| {
                    let error_msg = format!("路由键变量替换失败: {}", e);
                    log::error!(
                        "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                        context.flow_uuid,
                        node.uuid,
                        node.name,
                        error_msg
                    );
                    error_msg
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
                            .map_err(|e| format!("arguments[{}] 变量替换失败: {}", k, e))?;
                        resolved.insert(k.clone(), json!(replaced));
                    } else {
                        resolved.insert(k.clone(), v.clone());
                    }
                }
                Some(resolved)
            }
            None => None,
        };

        // 5. 日志：开始绑定
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp_queue_bind, queue: {}, exchange: {}, routing_key: {:?}",
            context.flow_uuid,
            node.uuid,
            node.name,
            queue,
            exchange,
            routing_key,
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "AMQP QueueBind 节点 {} 开始执行 (queue={}, exchange={})",
                    node.name, queue, exchange
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

        // 6. 构建绑定配置（使用替换后的值）
        let bind_config = AmqpQueueBindConfig {
            connection_id: connection_id.clone(),
            queue: queue.clone(),
            exchange: exchange.clone(),
            routing_key: routing_key.clone(),
            arguments: resolved_arguments,
        };

        // 7. 绑定队列
        let bind_result = self
            .amqp_manager
            .queue_bind(&connection_id, &bind_config)
            .await;

        let duration = start_time.elapsed();

        match bind_result {
            Ok(()) => {
                log::info!(
                    "[AMQP] 队列绑定成功: uuid={}, queue={}, exchange={}, routing_key={:?}",
                    node.uuid,
                    queue,
                    exchange,
                    routing_key,
                );

                // 8. 构建输出数据
                let output_data = json!({
                    "success": true,
                    "request": {
                        "connection_id": connection_id,
                        "queue": queue,
                        "exchange": exchange,
                        "routing_key": routing_key,
                        "arguments": bind_config.arguments,
                    },
                    "timing": {
                        "bind_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": serde_json::Value::Null,
                });

                // 9. 日志：绑定完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "AMQP 队列绑定成功: {} -> {} ({}ms)",
                            queue,
                            exchange,
                            duration.as_millis()
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

                // 10. 路由：成功走 true 端口
                let next_node_ids = resolve_port_targets(connections, &node.uuid, "true");

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 队列绑定失败: uuid={}, queue={}, exchange={}, error={}",
                    node.uuid,
                    queue,
                    exchange,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 队列绑定失败: {}", e),
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
                    "request": {
                        "connection_id": connection_id,
                        "queue": queue,
                        "exchange": exchange,
                        "routing_key": routing_key,
                    },
                    "timing": {
                        "bind_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 绑定失败走 false 端口
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
        AmqpQueueDeclareConfig, AmqpQueueDeleteConfig,
    };
    use flow_engine_core::traits::{AmqpConnectOutput, AmqpConnectionInfo, AmqpProvider};

    /// Mock AmqpProvider（queue_bind 成功）
    struct MockAmqpProvider {
        should_fail: bool,
    }

    impl MockAmqpProvider {
        fn new_success() -> Self {
            Self { should_fail: false }
        }

        fn new_failure() -> Self {
            Self { should_fail: true }
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
            if self.should_fail {
                Err("交换器不存在".to_string())
            } else {
                Ok(())
            }
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

    /// 创建测试节点
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        queue: &str,
        exchange: &str,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_queue_bind".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueBind(AmqpQueueBindConfig {
                connection_id: connection_id.to_string(),
                queue: queue.to_string(),
                exchange: exchange.to_string(),
                routing_key: Some("my.routing.key".to_string()),
                arguments: None,
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
            imap_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：绑定成功时输出 JSON 结构正确，路由到 true 端口
    #[tokio::test]
    async fn test_queue_bind_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueBindExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "绑定队列", "conn-1", "my-queue", "my-exchange");
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
        assert!(data["timing"]["bind_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert!(data["error"].is_null());
    }

    /// 测试：绑定失败时路由到 false 端口
    #[tokio::test]
    async fn test_queue_bind_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure());
        let executor = AmqpQueueBindExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "失败测试", "conn-1", "my-queue", "my-exchange");
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
        assert_eq!(data["error"], "交换器不存在");
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_queue_bind_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueBindExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().contains("AmqpQueueBindConfig"));
    }

    /// 测试：变量替换在 queue、exchange、connection_id 和 routing_key 中生效
    #[tokio::test]
    async fn test_queue_bind_with_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueBindExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "amqp_queue_bind".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueBind(AmqpQueueBindConfig {
                connection_id: "{{conn_id}}".to_string(),
                queue: "{{queue_name}}".to_string(),
                exchange: "{{exchange_name}}".to_string(),
                routing_key: Some("order.{{event_type}}".to_string()),
                arguments: None,
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
            json!("order.queue"),
            String::new(),
        );
        context.variables.set_variable(
            "exchange_name".to_string(),
            json!("order.exchange"),
            String::new(),
        );
        context.variables.set_variable(
            "event_type".to_string(),
            json!("created"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["request"]["connection_id"], "amqp-conn-789");
        assert_eq!(data["request"]["queue"], "order.queue");
        assert_eq!(data["request"]["exchange"], "order.exchange");
        assert_eq!(data["request"]["routing_key"], "order.created");
    }

    /// 测试：连接端口路由到下游节点
    #[tokio::test]
    async fn test_queue_bind_with_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueBindExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", "my-queue", "my-exchange");
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
    async fn test_queue_bind_report_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueBindExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "amqp_queue_bind".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueBind(AmqpQueueBindConfig {
                connection_id: "my-conn".to_string(),
                queue: "test.queue".to_string(),
                exchange: "test.exchange".to_string(),
                routing_key: Some("order.*".to_string()),
                arguments: Some(HashMap::from([(
                    "x-match".to_string(),
                    json!("all"),
                )])),
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
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        let request = &data["request"];
        assert_eq!(request["connection_id"], "my-conn");
        assert_eq!(request["queue"], "test.queue");
        assert_eq!(request["exchange"], "test.exchange");
        assert_eq!(request["routing_key"], "order.*");
        assert_eq!(request["arguments"]["x-match"], "all");
    }

    /// 测试：routing_key 为 None 时绑定成功
    #[tokio::test]
    async fn test_queue_bind_without_routing_key() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success());
        let executor = AmqpQueueBindExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-no-rk".to_string(),
            node_type: "amqp_queue_bind".to_string(),
            name: "无路由键测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpQueueBind(AmqpQueueBindConfig {
                connection_id: "conn-1".to_string(),
                queue: "my-queue".to_string(),
                exchange: "fanout-exchange".to_string(),
                routing_key: None,
                arguments: None,
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert!(data["request"]["routing_key"].is_null());
    }
}
