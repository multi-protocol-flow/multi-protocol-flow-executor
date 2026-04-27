//! GraphQL 查询/变更节点执行器
//!
//! 实现 GraphQL Query 和 Mutation 操作，通过 HTTP POST 发送请求。
//! 支持 GraphQL 变量替换、操作名称指定、文件上传（multipart）等特性。

use std::collections::HashMap;
use std::sync::Arc;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{GraphqlQueryConfig, NodeData};
use flow_engine_core::traits::GraphqlProvider;
use serde_json::json;

/// GraphQL 查询/变更节点执行器
///
/// 处理 GraphQL Query 和 Mutation 操作。
/// 从 `context.graphql_manager` 获取 `GraphqlProvider` 来执行请求。
pub struct GraphqlQueryExecutor {
    graphql_manager: Arc<dyn GraphqlProvider>,
}

impl GraphqlQueryExecutor {
    /// 创建新的 GraphQL 查询执行器
    pub fn new(graphql_manager: Arc<dyn GraphqlProvider>) -> Self {
        Self { graphql_manager }
    }

    /// 构建路由端口列表
    ///
    /// 根据 true/false 端口从连接中查找目标节点，若无匹配则返回端口名本身。
    fn resolve_port_targets(
        node_uuid: &str,
        port: &str,
        connections: &[Connection],
    ) -> Vec<String> {
        let targets: Vec<String> = connections
            .iter()
            .filter(|conn| {
                conn.source_node_uuid == node_uuid && conn.source_port_id == port
            })
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        if targets.is_empty() {
            vec![port.to_string()]
        } else {
            targets
        }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for GraphqlQueryExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = std::time::Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::GraphqlQuery(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 GraphqlQueryConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换（query, variables, operation_name）
        let query = replace_variables(&config.query, context).map_err(|e| {
            let error_msg = format!("query 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        let operation_name = match &config.operation_name {
            Some(op_name) => {
                let replaced = replace_variables(op_name, context).map_err(|e| {
                    format!("operation_name 变量替换失败: {}", e)
                })?;
                Some(replaced)
            }
            None => None,
        };

        // variables 变量替换：序列化为字符串后替换，再解析回 JSON
        let variables = match &config.variables {
            Some(vars) => {
                let vars_str = serde_json::to_string(vars).unwrap_or_default();
                let replaced_str = replace_variables(&vars_str, context)
                    .map_err(|e| format!("variables 变量替换失败: {}", e))?;
                let replaced_vars: serde_json::Value =
                    serde_json::from_str(&replaced_str).unwrap_or_else(|_| vars.clone());
                Some(replaced_vars)
            }
            None => None,
        };

        // 3. 日志：开始查询
        let has_files = config
            .files
            .as_ref()
            .is_some_and(|f| !f.is_empty());

        let query_type = if has_files { "mutation (文件上传)" } else { "query" };

        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: graphql_query ({}), connection_id: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            query_type,
            config.connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("GraphQL {} 节点 {} 开始执行", query_type, node.name),
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
        let resolved_config = GraphqlQueryConfig {
            connection_id: config.connection_id.clone(),
            query,
            variables,
            operation_name,
            timeout_ms: config.timeout_ms,
            files: config.files.clone(),
        };

        // 5. 执行查询
        let query_result = self
            .graphql_manager
            .query(&resolved_config.connection_id, &resolved_config)
            .await;

        match query_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                // 6. 区分网络错误和 GraphQL 错误
                // - 网络错误（success=false, 有 error 字段）→ 走 false 端口
                // - GraphQL 错误（HTTP 200 但有 errors 字段）→ 走 true 端口（用户决定是否中断）
                let is_network_error = !output.success && output.error.is_some();
                let has_graphql_errors = !output.errors.is_empty();

                if output.success {
                    log::info!(
                        "[GraphQL] 查询成功: uuid={}, connection_id={}, 耗时={}ms",
                        node.uuid,
                        config.connection_id,
                        duration.as_millis()
                    );
                } else if is_network_error {
                    log::warn!(
                        "[GraphQL] 查询网络错误: uuid={}, connection_id={}, error={:?}",
                        node.uuid,
                        config.connection_id,
                        output.error
                    );
                } else {
                    log::warn!(
                        "[GraphQL] 查询包含错误: uuid={}, connection_id={}, errors={:?}",
                        node.uuid,
                        config.connection_id,
                        output.errors
                    );
                }

                // 7. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "data": output.data,
                    "errors": output.errors,
                    "extensions": output.extensions,
                    "status_code": output.status_code,
                    "timing": output.timing.as_ref().map(|t| json!({
                        "total_ms": t.total_ms,
                        "dns_ms": t.dns_ms,
                        "connect_ms": t.connect_ms,
                        "tls_ms": t.tls_ms,
                        "server_ms": t.server_ms,
                    })),
                    "request": {
                        "connection_id": config.connection_id,
                        "query": resolved_config.query,
                        "operation_name": resolved_config.operation_name,
                        "has_files": has_files,
                        "timeout_ms": config.timeout_ms,
                    },
                    "error": output.error,
                });

                // 8. 日志：查询完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "GraphQL 查询成功 ({}ms){}",
                        duration.as_millis(),
                        if has_graphql_errors {
                            format!(" (含 {} 个 GraphQL 错误)", output.errors.len())
                        } else {
                            String::new()
                        }
                    )
                } else {
                    format!(
                        "GraphQL 查询失败: {} ({}ms)",
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

                // 9. 路由：网络错误走 false，其他走 true
                let port = if is_network_error { "false" } else { "true" };
                let next_node_ids = Self::resolve_port_targets(&node.uuid, port, connections);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                // Provider 返回 Err（如连接不存在）→ 网络/系统错误
                let duration = start_time.elapsed();
                log::error!(
                    "[GraphQL] 查询异常: uuid={}, connection_id={}, error={}",
                    node.uuid,
                    config.connection_id,
                    e
                );

                // 日志：查询异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("GraphQL 查询异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出
                let error_output = json!({
                    "success": false,
                    "data": serde_json::Value::Null,
                    "errors": [],
                    "extensions": serde_json::Value::Null,
                    "status_code": serde_json::Value::Null,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "request": {
                        "connection_id": config.connection_id,
                        "query": resolved_config.query,
                    },
                    "error": e,
                });

                // 走 false 端口
                let next_node_ids =
                    Self::resolve_port_targets(&node.uuid, "false", connections);

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
    use flow_engine_core::node_config::{
        GraphqlConnectConfig, GraphqlDisconnectConfig, GraphqlIntrospectConfig,
        GraphqlQueryConfig, GraphqlSubscribeConfig, GraphqlUnsubscribeConfig,
    };
    use flow_engine_core::traits::{
        GraphqlConnectOutput, GraphqlConnectionInfo, GraphqlIntrospectOutput,
        GraphqlQueryOutput, GraphqlSubscribeOutput, GraphqlTimingInfo,
    };

    // ---------------------------------------------------------------------------
    // Mock GraphqlProvider
    // ---------------------------------------------------------------------------

    /// 模拟 GraphQL Provider（查询成功）
    struct MockGraphqlProvider {
        output: GraphqlQueryOutput,
    }

    impl MockGraphqlProvider {
        fn new_success() -> Self {
            Self {
                output: GraphqlQueryOutput {
                    success: true,
                    data: Some(json!({ "users": [{"id": 1, "name": "Alice"}] })),
                    errors: vec![],
                    extensions: None,
                    status_code: Some(200),
                    timing: Some(GraphqlTimingInfo {
                        total_ms: 50,
                        dns_ms: Some(5),
                        connect_ms: Some(10),
                        tls_ms: Some(8),
                        server_ms: Some(25),
                    }),
                    error: None,
                },
            }
        }

        fn new_graphql_error() -> Self {
            Self {
                output: GraphqlQueryOutput {
                    success: false,
                    data: Some(json!({ "users": null })),
                    errors: vec![json!({
                        "message": "Field 'foo' not found",
                        "locations": [{"line": 1, "column": 5}]
                    })],
                    extensions: None,
                    status_code: Some(200),
                    timing: Some(GraphqlTimingInfo {
                        total_ms: 30,
                        dns_ms: None,
                        connect_ms: None,
                        tls_ms: None,
                        server_ms: None,
                    }),
                    error: None,
                },
            }
        }

        fn new_network_error(error: &str) -> Self {
            Self {
                output: GraphqlQueryOutput {
                    success: false,
                    data: None,
                    errors: vec![],
                    extensions: None,
                    status_code: None,
                    timing: Some(GraphqlTimingInfo {
                        total_ms: 100,
                        dns_ms: None,
                        connect_ms: None,
                        tls_ms: None,
                        server_ms: None,
                    }),
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
            Err("not implemented".to_string())
        }

        async fn query(
            &self,
            _uuid: &str,
            _config: &GraphqlQueryConfig,
        ) -> Result<GraphqlQueryOutput, String> {
            Ok(self.output.clone())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &GraphqlSubscribeConfig,
        ) -> Result<GraphqlSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn unsubscribe(
            &self,
            _uuid: &str,
            _config: &GraphqlUnsubscribeConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn introspect(
            &self,
            _uuid: &str,
            _config: &GraphqlIntrospectConfig,
        ) -> Result<GraphqlIntrospectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &GraphqlDisconnectConfig,
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

    /// 模拟 GraphQL Provider（query 返回 Err）
    struct MockGraphqlProviderError;

    #[async_trait::async_trait]
    impl GraphqlProvider for MockGraphqlProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &GraphqlConnectConfig,
        ) -> Result<GraphqlConnectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn query(
            &self,
            _uuid: &str,
            _config: &GraphqlQueryConfig,
        ) -> Result<GraphqlQueryOutput, String> {
            Err("连接 'missing-conn' 不存在".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &GraphqlSubscribeConfig,
        ) -> Result<GraphqlSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn unsubscribe(
            &self,
            _uuid: &str,
            _config: &GraphqlUnsubscribeConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn introspect(
            &self,
            _uuid: &str,
            _config: &GraphqlIntrospectConfig,
        ) -> Result<GraphqlIntrospectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &GraphqlDisconnectConfig,
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

    // ---------------------------------------------------------------------------
    // 测试辅助函数
    // ---------------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, query: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "graphql_query".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlQuery(GraphqlQueryConfig {
                connection_id: connection_id.to_string(),
                query: query.to_string(),
                variables: None,
                operation_name: None,
                timeout_ms: 30000,
                files: None,
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

    // ---------------------------------------------------------------------------
    // 测试
    // ---------------------------------------------------------------------------

    /// 测试：配置提取成功（正确的 NodeData 变体）
    #[tokio::test]
    async fn test_graphql_query_config_extraction_success() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);
        let node = create_test_node("test-1", "查询用户", "conn-1", "{ users { id name } }");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 成功应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());
    }

    /// 测试：配置提取拒绝错误的 NodeData 变体
    #[tokio::test]
    async fn test_graphql_query_config_extraction_wrong_variant() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "graphql_connect".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlConnect(GraphqlConnectConfig::default()),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("GraphqlQueryConfig"));
    }

    /// 测试：变量替换在 query 和 variables 中生效
    #[tokio::test]
    async fn test_graphql_query_with_variable_replacement() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "graphql_query".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlQuery(GraphqlQueryConfig {
                connection_id: "conn-1".to_string(),
                query: "query { user(id: {{user_id}}) { name } }".to_string(),
                variables: Some(json!({
                    "id": "{{user_id}}"
                })),
                operation_name: Some("GetUser_{{suffix}}".to_string()),
                timeout_ms: 30000,
                files: None,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "user_id".to_string(),
            json!("42"),
            String::new(),
        );
        context.variables.set_variable(
            "suffix".to_string(),
            json!("v1"),
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
        // 验证变量替换在 query 中生效
        assert_eq!(
            data["request"]["query"],
            "query { user(id: 42) { name } }"
        );
        // 验证 operation_name 替换
        assert_eq!(data["request"]["operation_name"], "GetUser_v1");
    }

    /// 测试：成功时路由到 true 端口，失败时路由到 false 端口
    #[tokio::test]
    async fn test_graphql_query_true_false_port_routing() {
        // 测试成功 → true
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);
        let node = create_test_node("test-true", "成功测试", "conn-1", "{ users { id } }");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);

        // 测试网络错误 → false
        let graphql_manager_err = Arc::new(MockGraphqlProvider::new_network_error("Connection refused"));
        let executor_err = GraphqlQueryExecutor::new(graphql_manager_err);
        let node_err = create_test_node("test-false", "失败测试", "conn-1", "{ users { id } }");
        let mut context_err = create_context("test-flow");

        let result_err = executor_err
            .execute(&node_err, &mut context_err, &[], &HashMap::new())
            .await;
        assert!(result_err.is_ok());
        let (next_nodes_err, _) = result_err.unwrap();
        assert_eq!(next_nodes_err, vec!["false"]);
    }

    /// 测试：输出结构包含所有必要字段
    #[tokio::test]
    async fn test_graphql_query_report_input_structure() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "graphql_query".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlQuery(GraphqlQueryConfig {
                connection_id: "conn-1".to_string(),
                query: "{ users { id name } }".to_string(),
                variables: Some(json!({"limit": 10})),
                operation_name: Some("GetUsers".to_string()),
                timeout_ms: 5000,
                files: None,
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

        // 验证顶层字段
        assert!(data.get("success").is_some());
        assert!(data.get("data").is_some());
        assert!(data.get("errors").is_some());
        assert!(data.get("extensions").is_some());
        assert!(data.get("status_code").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("error").is_some());

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["connection_id"], "conn-1");
        assert_eq!(request["query"], "{ users { id name } }");
        assert_eq!(request["operation_name"], "GetUsers");
        assert_eq!(request["has_files"], false);
        assert_eq!(request["timeout_ms"], 5000);

        // 验证 timing 子结构
        let timing = data["timing"].as_object().expect("timing 应为对象");
        assert!(timing.contains_key("total_ms"));
    }

    /// 测试：文件上传检测
    #[tokio::test]
    async fn test_graphql_query_file_upload_detection() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-files".to_string(),
            node_type: "graphql_query".to_string(),
            name: "文件上传测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlQuery(GraphqlQueryConfig {
                connection_id: "conn-1".to_string(),
                query: "mutation ($file: Upload!) { upload(file: $file) { url } }".to_string(),
                variables: None,
                operation_name: None,
                timeout_ms: 30000,
                files: Some(vec![flow_engine_core::node_config::GraphqlFileUpload {
                    name: "file".to_string(),
                    path: Some("/path/to/file.txt".to_string()),
                    content_type: Some("text/plain".to_string()),
                }]),
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

        // 验证 has_files 为 true
        assert_eq!(data["request"]["has_files"], true);
    }

    /// 测试：Provider 返回 Err 时（如连接不存在）走 false 端口
    #[tokio::test]
    async fn test_graphql_query_provider_error_routes_to_false() {
        let graphql_manager = Arc::new(MockGraphqlProviderError);
        let executor = GraphqlQueryExecutor::new(graphql_manager);
        let node =
            create_test_node("test-err", "异常测试", "missing-conn", "{ users { id } }");
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
        assert!(data["error"].as_str().unwrap().contains("不存在"));
    }

    /// 测试：GraphQL 错误（200 + errors）走 true 端口
    #[tokio::test]
    async fn test_graphql_query_graphql_errors_route_to_true() {
        // GraphQL 错误：HTTP 200 但有 errors 字段
        let graphql_manager = Arc::new(MockGraphqlProvider::new_graphql_error());
        let executor = GraphqlQueryExecutor::new(graphql_manager);
        let node = create_test_node("test-gql-err", "GraphQL 错误测试", "conn-1", "{ foo }");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // GraphQL 错误（非网络错误）应走 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        // success 应为 false（有 GraphQL 错误）
        assert_eq!(data["success"], false);
        // 但路由到了 true 端口
        // errors 应包含错误详情
        let errors = data["errors"].as_array().expect("errors 应为数组");
        assert!(!errors.is_empty());
        assert!(errors[0]["message"]
            .as_str()
            .unwrap()
            .contains("Field 'foo' not found"));
    }

    /// 测试：输出包含 timing 信息
    #[tokio::test]
    async fn test_graphql_query_timing_in_output() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);
        let node = create_test_node("test-timing", "耗时测试", "conn-1", "{ users { id } }");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        // 验证 timing 字段存在且有 total_ms
        let timing = data["timing"].as_object().expect("timing 应为对象");
        assert!(timing.contains_key("total_ms"));
        let total_ms = timing["total_ms"].as_u64().expect("total_ms 应为数字");
        // timing 中有详细的子字段（来自 provider 的输出）
        assert!(timing.contains_key("dns_ms"));
        assert!(timing.contains_key("connect_ms"));
        assert!(timing.contains_key("tls_ms"));
        assert!(timing.contains_key("server_ms"));
        // total_ms 应大于 0
        assert!(total_ms > 0);
    }

    /// 测试：连接端口路由（通过 connections 查找目标节点）
    #[tokio::test]
    async fn test_graphql_query_with_connections_routing() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlQueryExecutor::new(graphql_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", "{ users { id } }");
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
