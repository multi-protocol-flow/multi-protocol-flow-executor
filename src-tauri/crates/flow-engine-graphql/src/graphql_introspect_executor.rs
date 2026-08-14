//! GraphQL Schema 探索节点执行器
//!
//! 发送完整的 __schema 内省查询到 GraphQL 端点，
//! 解析响应为 Schema 数据，并可选生成基本的查询/变更模板。

use flow_engine_core::error::ErrorDetail;
use std::collections::HashMap;
use std::sync::Arc;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{GraphqlIntrospectConfig, NodeData};
use flow_engine_core::traits::GraphqlProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// GraphQL Schema 探索节点执行器
///
/// 包装 `GraphqlProvider::introspect()` 方法：
/// - 发送完整 __schema 内省查询到 GraphQL 端点
/// - 解析响应为 Schema 数据
/// - 可选生成基本的查询/变更模板
pub struct GraphqlIntrospectExecutor {
    graphql_manager: Arc<dyn GraphqlProvider>,
}

impl GraphqlIntrospectExecutor {
    /// 创建新的 GraphQL Schema 探索执行器
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
            .filter(|conn| conn.source_node_uuid == node_uuid && conn.source_port_id == port)
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
impl NodeExecutor for GraphqlIntrospectExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = std::time::Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::GraphqlIntrospect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not GraphqlIntrospectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（connection_id）
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

        // 3. 日志：开始探索
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: graphql_introspect, connection_id: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "GraphQL schema introspection node {} start execution",
                    node.name
                ),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建替换后的配置
        let resolved_config = GraphqlIntrospectConfig {
            connection_id: connection_id.clone(),
            timeout_ms: config.timeout_ms,
        };

        // 5. 执行 Schema 探索
        let introspect_result = self
            .graphql_manager
            .introspect(&resolved_config.connection_id, &resolved_config)
            .await;

        match introspect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[GraphQL] Introspection succeeded: uuid={}, connection_id={}, duration={}ms",
                        node.uuid,
                        connection_id,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[GraphQL] Introspection failed: uuid={}, connection_id={}, error={:?}",
                        node.uuid,
                        connection_id,
                        output.error
                    );
                }

                // 6. 构建输出数据
                let query_templates_json = match &output.query_templates {
                    Some(templates) => {
                        let arr: Vec<serde_json::Value> = templates
                            .iter()
                            .map(|t| {
                                json!({
                                    "type_name": t.type_name,
                                    "operation": t.operation,
                                    "template": t.template,
                                })
                            })
                            .collect();
                        serde_json::Value::Array(arr)
                    }
                    None => serde_json::Value::Null,
                };

                let output_data = json!({
                    "success": output.success,
                    "schema": output.schema,
                    "query_templates": query_templates_json,
                    "request": {
                        "connection_id": connection_id,
                        "timeout_ms": config.timeout_ms,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 7. 日志：探索完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    let template_count = output
                        .query_templates
                        .as_ref()
                        .map(|t| t.len())
                        .unwrap_or(0);
                    format!(
                        "GraphQL schema introspection succeeded ({}ms), generated {} query templates",
                        duration.as_millis(),
                        template_count
                    )
                } else {
                    format!(
                        "GraphQL schema introspection failed: {} ({}ms)",
                        output.error.as_deref().unwrap_or("unknown error"),
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
                if !output.success
                    && !node
                        .on_error
                        .as_ref()
                        .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    let error_msg = output
                        .error
                        .as_deref()
                        .unwrap_or("GraphQL introspection query failed")
                        .to_string();
                    return Err(ErrorDetail::execution_error(error_msg));
                }
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = Self::resolve_port_targets(&node.uuid, port, connections);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                // Provider 返回 Err（如连接不存在）→ 异常
                let duration = start_time.elapsed();
                log::error!(
                    "[GraphQL] Introspection error: uuid={}, connection_id={}, error={}",
                    node.uuid,
                    connection_id,
                    e
                );

                // 日志：探索异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("GraphQL schema introspection error: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出
                let error_output = json!({
                    "success": false,
                    "schema": serde_json::Value::Null,
                    "query_templates": serde_json::Value::Null,
                    "request": {
                        "connection_id": connection_id,
                        "timeout_ms": config.timeout_ms,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(e));
                }

                // 走 false 端口
                let next_node_ids = Self::resolve_port_targets(&node.uuid, "false", connections);

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
        GraphqlConnectConfig, GraphqlDisconnectConfig, GraphqlQueryConfig, GraphqlSubscribeConfig,
    };
    use flow_engine_core::traits::{
        GraphqlConnectOutput, GraphqlConnectionInfo, GraphqlIntrospectOutput, GraphqlQueryOutput,
        GraphqlQueryTemplate, GraphqlSubscribeOutput,
    };

    // ---------------------------------------------------------------------------
    // Mock GraphqlProvider
    // ---------------------------------------------------------------------------

    /// 模拟 GraphQL Provider（探索Success，含 Schema 和模板）
    struct MockGraphqlProvider {
        output: GraphqlIntrospectOutput,
    }

    impl MockGraphqlProvider {
        fn new_success() -> Self {
            Self {
                output: GraphqlIntrospectOutput {
                    success: true,
                    schema: Some(json!({
                        "queryType": { "name": "Query" },
                        "mutationType": { "name": "Mutation" },
                        "types": [
                            { "name": "Query", "fields": [
                                { "name": "users", "type": { "name": "User" } }
                            ]},
                            { "name": "User", "fields": [
                                { "name": "id", "type": { "name": "ID" } },
                                { "name": "name", "type": { "name": "String" } }
                            ]}
                        ]
                    })),
                    query_templates: Some(vec![
                        GraphqlQueryTemplate {
                            type_name: "User".to_string(),
                            operation: "query".to_string(),
                            template: "query { users { id name } }".to_string(),
                        },
                        GraphqlQueryTemplate {
                            type_name: "User".to_string(),
                            operation: "mutation".to_string(),
                            template: "mutation { createUser(name: \"\") { id name } }".to_string(),
                        },
                    ]),
                    error: None,
                },
            }
        }

        fn new_success_no_templates() -> Self {
            Self {
                output: GraphqlIntrospectOutput {
                    success: true,
                    schema: Some(json!({ "queryType": { "name": "Query" } })),
                    query_templates: None,
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: GraphqlIntrospectOutput {
                    success: false,
                    schema: None,
                    query_templates: None,
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
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &GraphqlSubscribeConfig,
        ) -> Result<GraphqlSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn introspect(
            &self,
            _uuid: &str,
            _config: &GraphqlIntrospectConfig,
        ) -> Result<GraphqlIntrospectOutput, String> {
            Ok(self.output.clone())
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

    /// 模拟 GraphQL Provider（introspect 返回 Err）
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
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &GraphqlSubscribeConfig,
        ) -> Result<GraphqlSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn introspect(
            &self,
            _uuid: &str,
            _config: &GraphqlIntrospectConfig,
        ) -> Result<GraphqlIntrospectOutput, String> {
            Err("连接 'missing-conn' 不存在".to_string())
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

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "graphql_introspect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlIntrospect(GraphqlIntrospectConfig {
                connection_id: connection_id.to_string(),
                timeout_ms: 30000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            execution_id: String::new(),
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

    // ---------------------------------------------------------------------------
    // 测试
    // ---------------------------------------------------------------------------

    /// 测试：配置提取Success（正确的 `NodeData` 变体）
    #[tokio::test]
    async fn test_graphql_introspect_config_extraction_success() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-1", "Schema 探索", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        // Success应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());
    }

    /// 测试：配置提取拒绝Error的 `NodeData` 变体
    #[tokio::test]
    async fn test_graphql_introspect_config_extraction_wrong_variant() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "graphql_connect".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlConnect(GraphqlConnectConfig::default()),
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
            .contains("GraphqlIntrospectConfig"));
    }

    /// 测试：Success时路由到 true 端口
    #[tokio::test]
    async fn test_graphql_introspect_success_routes_to_true_port() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-true", "Success测试", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], true);
        // error 应为 null
        assert!(data["error"].is_null());
    }

    /// 测试：Failed时路由到 false 端口
    #[tokio::test]
    async fn test_graphql_introspect_failure_routes_to_false_port() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_failure("Endpoint unreachable"));
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-false", "Failed测试", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Endpoint unreachable");
    }

    /// 测试：输出结构包含 schema 和 `query_templates`
    #[tokio::test]
    async fn test_graphql_introspect_report_input_structure() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-report", "报告结构测试", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        let data = output_data.unwrap_or_else(|| panic!("expected output data"));

        // 验证顶层字段
        assert!(data.get("success").is_some());
        assert!(data.get("schema").is_some());
        assert!(data.get("query_templates").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        // 验证 schema 包含类型信息
        let schema = data["schema"]
            .as_object()
            .unwrap_or_else(|| panic!("expected schema object"));
        assert!(schema.contains_key("queryType"));
        assert!(schema.contains_key("types"));

        // 验证 query_templates 包含模板列表
        let templates = data["query_templates"]
            .as_array()
            .unwrap_or_else(|| panic!("expected query_templates array"));
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0]["type_name"], "User");
        assert_eq!(templates[0]["operation"], "query");
        assert!(templates[0]["template"].is_string());
        assert_eq!(templates[1]["operation"], "mutation");

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["connection_id"], "conn-1");
        assert_eq!(request["timeout_ms"], 30000);

        // 验证 timing 子结构
        let timing = data["timing"]
            .as_object()
            .unwrap_or_else(|| panic!("expected timing object"));
        assert!(timing.contains_key("total_ms"));
    }

    /// 测试：Provider 返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_graphql_introspect_provider_error_routes_to_false() {
        let graphql_manager = Arc::new(MockGraphqlProviderError);
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-err", "异常测试", "missing-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], false);
        assert!(data["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("不存在"));
        assert!(data["schema"].is_null());
        assert!(data["query_templates"].is_null());
    }

    /// 测试：无模板时 `query_templates` 为 null
    #[tokio::test]
    async fn test_graphql_introspect_no_templates_output() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success_no_templates());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-no-tmpl", "无模板测试", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        let data = output_data.unwrap_or_else(|| panic!("expected output data"));

        assert_eq!(data["success"], true);
        // query_templates 为 null
        assert!(data["query_templates"].is_null());
        // schema 仍存在
        assert!(data["schema"].is_object());
    }

    /// 测试：连接端口路由（通过 connections 查找目标节点）
    #[tokio::test]
    async fn test_graphql_introspect_with_connections_routing() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1");
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
        let (next_nodes, _) = result.unwrap_or_else(|e| panic!("{e}"));
        // 应路由到连接中指定的目标节点
        assert_eq!(next_nodes, vec!["next-node"]);
    }
}
