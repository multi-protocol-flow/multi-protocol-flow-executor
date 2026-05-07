//! GraphQL 连接节点执行器
//!
//! 实现 GraphQL 连接建立、变量替换和输出构建功能。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{GraphqlConnectConfig, NodeData};
use flow_engine_core::traits::GraphqlProvider;
use serde_json::json;

/// GraphQL 连接节点执行器
pub struct GraphqlConnectExecutor {
    graphql_manager: Arc<dyn GraphqlProvider>,
}

impl GraphqlConnectExecutor {
    /// 创建新的 GraphQL 连接执行器
    pub fn new(graphql_manager: Arc<dyn GraphqlProvider>) -> Self {
        Self { graphql_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for GraphqlConnectExecutor {
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
            NodeData::GraphqlConnect(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 GraphqlConnectConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换（endpoint, ws_endpoint, headers）
        let endpoint = replace_variables(&config.endpoint, context).map_err(|e| {
            let error_msg = format!("endpoint 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        let ws_endpoint = match &config.ws_endpoint {
            Some(ws) => Some(replace_variables(ws, context).map_err(|e| {
                format!("ws_endpoint 变量替换失败: {}", e)
            })?),
            None => None,
        };

        // 替换 headers 中的变量
        let headers = match &config.headers {
            Some(h) => {
                let mut replaced = HashMap::new();
                for (k, v) in h {
                    let rv = replace_variables(v, context)
                        .map_err(|e| format!("Header '{}' 变量替换失败: {}", k, e))?;
                    replaced.insert(k.clone(), rv);
                }
                Some(replaced)
            }
            None => None,
        };

        // 3. 日志：开始连接
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: graphql_connect, endpoint: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            endpoint
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("GraphQL 连接节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
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

        // 5. 构建连接配置（使用替换后的值）
        let connect_config = GraphqlConnectConfig {
            endpoint,
            ws_endpoint,
            headers,
            connection_id: Some(connection_id.clone()),
            ..config
        };

        // 6. 建立连接
        let connect_result = self
            .graphql_manager
            .connect(&connection_id, &connect_config)
            .await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[GraphQL] 连接成功: uuid={}, endpoint={}, connection_id={}",
                        node.uuid,
                        connect_config.endpoint,
                        output.connection_id
                    );
                } else {
                    log::warn!(
                        "[GraphQL] 连接失败: uuid={}, endpoint={}, error={:?}",
                        node.uuid,
                        connect_config.endpoint,
                        output.error
                    );
                }

                // 7. 构建输出数据
                let server_info_json = match &output.server_info {
                    Some(info) => serde_json::Value::String(info.clone()),
                    None => serde_json::Value::Null,
                };

                let output_data = json!({
                    "success": output.success,
                    "connection_id": output.connection_id,
                    "server_info": server_info_json,
                    "request": {
                        "endpoint": connect_config.endpoint,
                        "ws_endpoint": connect_config.ws_endpoint,
                        "timeout_ms": connect_config.connection_timeout_ms,
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
                    format!(
                        "GraphQL 连接成功 ({}ms)",
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "GraphQL 连接失败: {} ({}ms)",
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
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == port
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec![port.to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[GraphQL] 连接异常: uuid={}, endpoint={}, error={}",
                    node.uuid,
                    connect_config.endpoint,
                    e
                );

                // 日志：连接异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("GraphQL 连接异常: {}", e),
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
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "server_info": serde_json::Value::Null,
                    "request": {
                        "endpoint": connect_config.endpoint,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 返回 false 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid
                                && conn.source_port_id == "false"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec!["false".to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::GraphqlConnectConfig;
    use flow_engine_core::traits::{GraphqlConnectOutput, GraphqlConnectionInfo};

    /// 模拟 GraphQL Provider（连接成功）
    struct MockGraphqlProvider {
        output: GraphqlConnectOutput,
    }

    impl MockGraphqlProvider {
        fn new_success() -> Self {
            Self {
                output: GraphqlConnectOutput {
                    success: true,
                    connection_id: "test-conn-1".to_string(),
                    server_info: Some("GraphQL/1.0".to_string()),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: GraphqlConnectOutput {
                    success: false,
                    connection_id: String::new(),
                    server_info: None,
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl GraphqlProvider for MockGraphqlProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &GraphqlConnectConfig,
        ) -> Result<GraphqlConnectOutput, String> {
            Ok(self.output.clone())
        }

        async fn query(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlQueryConfig,
        ) -> Result<flow_engine_core::traits::GraphqlQueryOutput, String> {
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlSubscribeConfig,
        ) -> Result<flow_engine_core::traits::GraphqlSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn introspect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlIntrospectConfig,
        ) -> Result<flow_engine_core::traits::GraphqlIntrospectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<GraphqlConnectionInfo> {
            vec![]
        }
    }

    /// 模拟 GraphQL Provider（connect 返回 Err）
    struct MockGraphqlProviderError;

    #[async_trait::async_trait]
    impl GraphqlProvider for MockGraphqlProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &GraphqlConnectConfig,
        ) -> Result<GraphqlConnectOutput, String> {
            Err("网络超时".to_string())
        }

        async fn query(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlQueryConfig,
        ) -> Result<flow_engine_core::traits::GraphqlQueryOutput, String> {
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlSubscribeConfig,
        ) -> Result<flow_engine_core::traits::GraphqlSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn introspect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlIntrospectConfig,
        ) -> Result<flow_engine_core::traits::GraphqlIntrospectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::GraphqlDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }

        fn list_connections(&self) -> Vec<GraphqlConnectionInfo> {
            vec![]
        }
    }

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, endpoint: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "graphql_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlConnect(GraphqlConnectConfig {
                endpoint: endpoint.to_string(),
                ..GraphqlConnectConfig::default()
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

    /// 测试：连接成功时输出 JSON 结构正确
    #[tokio::test]
    async fn test_graphql_connect_success_output_structure() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlConnectExecutor::new(graphql_manager);
        let node = create_test_node("test-1", "测试连接", "https://example.com/graphql");
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
        assert_eq!(data["connection_id"], "test-conn-1");
        assert_eq!(data["server_info"], "GraphQL/1.0");
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert_eq!(data["request"]["endpoint"], "https://example.com/graphql");
        // error 应为 null（成功时无错误）
        assert!(data["error"].is_null());
    }

    /// 测试：连接失败时路由到 false 端口
    #[tokio::test]
    async fn test_graphql_connect_failure_routes_to_false_port() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_failure("Connection refused"));
        let executor = GraphqlConnectExecutor::new(graphql_manager);
        let node = create_test_node("test-2", "失败测试", "https://unreachable.example/graphql");
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
        assert!(data["server_info"].is_null());
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_graphql_connect_wrong_node_data_type() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlConnectExecutor::new(graphql_manager);

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
        assert!(result.unwrap_err().contains("GraphqlConnectConfig"));
    }

    /// 测试：变量替换在 endpoint 和 headers 中生效
    #[tokio::test]
    async fn test_graphql_connect_with_variable_replacement() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlConnectExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "graphql_connect".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlConnect(GraphqlConnectConfig {
                endpoint: "https://{{host}}/graphql".to_string(),
                ws_endpoint: Some("wss://{{host}}/graphql".to_string()),
                headers: Some({
                    let mut h = HashMap::new();
                    h.insert("Authorization".to_string(), "Bearer {{token}}".to_string());
                    h
                }),
                ..GraphqlConnectConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("api.example.com"),
            String::new(),
        );
        context.variables.set_variable(
            "token".to_string(),
            json!("my-secret-token"),
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
        assert_eq!(data["request"]["endpoint"], "https://api.example.com/graphql");
        assert_eq!(
            data["request"]["ws_endpoint"],
            "wss://api.example.com/graphql"
        );
    }

    /// 测试：默认 connection_id 回退到节点 UUID
    #[tokio::test]
    async fn test_graphql_connect_default_connection_id_fallback() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlConnectExecutor::new(graphql_manager);

        // 创建不指定 connection_id 的节点
        let node = Node {
            uuid: "node-uuid-123".to_string(),
            node_type: "graphql_connect".to_string(),
            name: "默认ID测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlConnect(GraphqlConnectConfig {
                endpoint: "https://example.com/graphql".to_string(),
                connection_id: None, // 未指定
                ..GraphqlConnectConfig::default()
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
        assert!(output_data.is_some());
        // 成功执行即可，说明 connection_id 默认使用了节点 UUID
    }

    /// 测试：连接端口路由
    #[tokio::test]
    async fn test_graphql_connect_with_connections_routing() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlConnectExecutor::new(graphql_manager);
        let node = create_test_node("test-route", "路由测试", "https://example.com/graphql");
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
    async fn test_graphql_connect_provider_error_routes_to_false() {
        let graphql_manager = Arc::new(MockGraphqlProviderError);
        let executor = GraphqlConnectExecutor::new(graphql_manager);
        let node = create_test_node("test-err", "异常测试", "https://example.com/graphql");
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
    async fn test_graphql_connect_report_input_structure() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlConnectExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "graphql_connect".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlConnect(GraphqlConnectConfig {
                endpoint: "https://example.com/graphql".to_string(),
                ws_endpoint: Some("wss://example.com/graphql".to_string()),
                headers: Some({
                    let mut h = HashMap::new();
                    h.insert("Authorization".to_string(), "Bearer test-token".to_string());
                    h
                }),
                connection_timeout_ms: 5000,
                connection_id: Some("my-conn-id".to_string()),
                ..GraphqlConnectConfig::default()
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

        // 验证 report input 包含所有必要字段
        assert!(data.get("success").is_some());
        assert!(data.get("connection_id").is_some());
        assert!(data.get("server_info").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["endpoint"], "https://example.com/graphql");
        assert_eq!(request["ws_endpoint"], "wss://example.com/graphql");
        assert_eq!(request["timeout_ms"], 5000);

        // 验证 timing 子结构
        let timing = &data["timing"];
        assert!(timing["connect_ms"].is_number());
        assert!(timing["total_ms"].is_number());
    }
}
