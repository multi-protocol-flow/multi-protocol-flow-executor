//! Kafka 连接节点执行器
//!
//! 实现 Kafka 连接建立、变量替换和输出构建功能。
//! 参照 `mqtt_connect_executor.rs` 模式实现。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{KafkaConnectConfig, NodeData};
use flow_engine_core::traits::KafkaProvider;
use serde_json::json;

/// Kafka 连接节点执行器
pub struct KafkaConnectExecutor {
    kafka_manager: Arc<dyn KafkaProvider>,
}

impl KafkaConnectExecutor {
    /// 创建新的 Kafka 连接执行器
    pub fn new(kafka_manager: Arc<dyn KafkaProvider>) -> Self {
        Self { kafka_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for KafkaConnectExecutor {
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
            NodeData::KafkaConnect(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 KafkaConnectConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换（bootstrap_servers, client_id, sasl_username, sasl_password 等）
        let bootstrap_servers =
            replace_variables(&config.bootstrap_servers, context).map_err(|e| {
                let error_msg = format!("bootstrap_servers 变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?;

        let client_id = match &config.client_id {
            Some(id) => Some(replace_variables(id, context).map_err(|e| {
                format!("client_id 变量替换失败: {}", e)
            })?),
            None => None,
        };

        let sasl_username = match &config.sasl_username {
            Some(u) => Some(replace_variables(u, context).map_err(|e| {
                format!("sasl_username 变量替换失败: {}", e)
            })?),
            None => None,
        };

        let sasl_password = match &config.sasl_password {
            Some(p) => Some(replace_variables(p, context).map_err(|e| {
                format!("sasl_password 变量替换失败: {}", e)
            })?),
            None => None,
        };

        // 3. 日志：开始连接
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: kafka_connect, Bootstrap: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            bootstrap_servers
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Kafka 连接节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建连接配置（使用替换后的值）
        let connect_config = KafkaConnectConfig {
            bootstrap_servers,
            client_id,
            sasl_username,
            sasl_password,
            ..config
        };

        // 5. 建立连接
        let connect_result = self
            .kafka_manager
            .connect(&node.uuid, &connect_config)
            .await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[Kafka] 连接成功: uuid={}, bootstrap_servers={}",
                        node.uuid,
                        connect_config.bootstrap_servers
                    );
                } else {
                    log::warn!(
                        "[Kafka] 连接失败: uuid={}, bootstrap_servers={}, error={:?}",
                        node.uuid,
                        connect_config.bootstrap_servers,
                        output.error
                    );
                }

                // 6. 构建输出数据
                let connection_info_json = match &output.connection_info {
                    Some(info) => json!({
                        "uuid": info.uuid,
                        "bootstrap_servers": info.bootstrap_servers,
                        "client_id": info.client_id,
                        "security_protocol": info.security_protocol,
                        "status": info.status,
                        "connected_at": info.connected_at,
                    }),
                    None => serde_json::Value::Null,
                };

                let output_data = json!({
                    "success": output.success,
                    "connection_info": connection_info_json,
                    "request": {
                        "bootstrap_servers": connect_config.bootstrap_servers,
                        "security_protocol": connect_config.security_protocol,
                        "sasl_mechanism": connect_config.sasl_mechanism,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 7. 日志：连接完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!("Kafka 连接成功 ({}ms)", duration.as_millis())
                } else {
                    format!(
                        "Kafka 连接失败: {} ({}ms)",
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
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. 端口路由
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = route_to_port(node, connections, port);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[Kafka] 连接异常: uuid={}, bootstrap_servers={}, error={}",
                    node.uuid,
                    connect_config.bootstrap_servers,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Kafka 连接异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "connection_info": serde_json::Value::Null,
                    "request": {
                        "bootstrap_servers": connect_config.bootstrap_servers,
                        "security_protocol": connect_config.security_protocol,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                let next_node_ids = route_to_port(node, connections, "false");
                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 端口路由辅助函数
// ---------------------------------------------------------------------------

/// 根据 `port_id` 查找连接中的目标节点 UUID
fn route_to_port(node: &Node, connections: &[Connection], port: &str) -> Vec<String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == port)
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    if filtered.is_empty() {
        vec![port.to_string()]
    } else {
        filtered
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::KafkaConnectConfig;
    use flow_engine_core::traits::{
        KafkaCloseOutput, KafkaConnectOutput, KafkaConnectionInfo,
    };

    // ---------------------------------------------------------------------------
    // Mock KafkaProvider
    // ---------------------------------------------------------------------------

    /// 模拟成功连接的 KafkaProvider
    struct MockKafkaProvider {
        output: KafkaConnectOutput,
    }

    impl MockKafkaProvider {
        fn new_success() -> Self {
            Self {
                output: KafkaConnectOutput {
                    success: true,
                    connection_info: Some(KafkaConnectionInfo {
                        uuid: "conn-001".to_string(),
                        bootstrap_servers: "localhost:9092".to_string(),
                        client_id: Some("test-client".to_string()),
                        security_protocol: Some("plaintext".to_string()),
                        status: "connected".to_string(),
                        connected_at: 1700000000000,
                    }),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: KafkaConnectOutput {
                    success: false,
                    connection_info: None,
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl KafkaProvider for MockKafkaProvider {
        async fn connect(
            &self,
            uuid: &str,
            _config: &KafkaConnectConfig,
        ) -> Result<KafkaConnectOutput, String> {
            // 返回的 connection_info 中使用实际的 uuid
            let mut output = self.output.clone();
            if let Some(ref mut info) = output.connection_info {
                info.uuid = uuid.to_string();
            }
            Ok(output)
        }

        async fn produce(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaProduceConfig,
        ) -> Result<flow_engine_core::traits::KafkaProduceOutput, String> {
            Err("not implemented".to_string())
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaConsumeConfig,
        ) -> Result<flow_engine_core::traits::KafkaConsumeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn consume_batch(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaConsumeBatchConfig,
        ) -> Result<flow_engine_core::traits::KafkaConsumeBatchOutput, String> {
            Err("not implemented".to_string())
        }

        async fn admin(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaAdminConfig,
        ) -> Result<flow_engine_core::traits::KafkaAdminOutput, String> {
            Err("not implemented".to_string())
        }

        async fn schema_registry(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaSchemaRegistryConfig,
        ) -> Result<flow_engine_core::traits::KafkaSchemaRegistryOutput, String> {
            Err("not implemented".to_string())
        }

        async fn close(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::KafkaCloseConfig,
        ) -> Result<KafkaCloseOutput, String> {
            Ok(KafkaCloseOutput {
                success: true,
                error: None,
            })
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<KafkaConnectionInfo> {
            vec![]
        }
    }

    // ---------------------------------------------------------------------------
    // 辅助工具
    // ---------------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, bootstrap_servers: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "kafka_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaConnect(KafkaConnectConfig {
                bootstrap_servers: bootstrap_servers.to_string(),
                client_id: Some("test-client".to_string()),
                ..KafkaConnectConfig::default()
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
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
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

    // ---------------------------------------------------------------------------
    // 测试用例
    // ---------------------------------------------------------------------------

    /// 测试：连接成功时输出 JSON 结构正确
    #[tokio::test]
    async fn test_connect_success_output_structure() {
        let kafka_manager = Arc::new(MockKafkaProvider::new_success());
        let executor = KafkaConnectExecutor::new(kafka_manager);
        let node = create_test_node("test-1", "测试连接", "localhost:9092");
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
        assert!(data["connection_info"].is_object());
        assert_eq!(data["connection_info"]["bootstrap_servers"], "localhost:9092");
        assert_eq!(data["connection_info"]["client_id"], "test-client");
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert_eq!(data["request"]["bootstrap_servers"], "localhost:9092");
    }

    /// 测试：连接失败时路由到 false 端口
    #[tokio::test]
    async fn test_connect_failure_routes_to_false_port() {
        let kafka_manager = Arc::new(MockKafkaProvider::new_failure("Connection refused"));
        let executor = KafkaConnectExecutor::new(kafka_manager);
        let node = create_test_node("test-2", "失败测试", "unreachable:9092");
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
        assert_eq!(data["error"], "Connection refused");
        assert!(data["connection_info"].is_null());
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_connect_wrong_node_data_type() {
        let kafka_manager = Arc::new(MockKafkaProvider::new_success());
        let executor = KafkaConnectExecutor::new(kafka_manager);

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
        assert!(result.unwrap_err().contains("KafkaConnectConfig"));
    }

    /// 测试：变量替换在 bootstrap_servers 和 client_id 中生效
    #[tokio::test]
    async fn test_connect_with_variable_replacement() {
        let kafka_manager = Arc::new(MockKafkaProvider::new_success());
        let executor = KafkaConnectExecutor::new(kafka_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "kafka_connect".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaConnect(KafkaConnectConfig {
                bootstrap_servers: "{{host}}:9092".to_string(),
                client_id: Some("client-{{env}}".to_string()),
                sasl_username: Some("user-{{env}}".to_string()),
                sasl_password: None,
                ..KafkaConnectConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("broker.example.com"),
            String::new(),
        );
        context.variables.set_variable(
            "env".to_string(),
            json!("prod"),
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
        assert_eq!(data["request"]["bootstrap_servers"], "broker.example.com:9092");
    }

    /// 测试：连接配置中的连接端口路由
    #[tokio::test]
    async fn test_connect_with_connections_routing() {
        let kafka_manager = Arc::new(MockKafkaProvider::new_success());
        let executor = KafkaConnectExecutor::new(kafka_manager);
        let node = create_test_node("test-route", "路由测试", "localhost:9092");
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
}
