//! GraphQL Schema 探索节点执行器
//!
//! 发送完整的 __schema 内省查询到 GraphQL 端点，
//! 解析响应为 Schema 数据，并可选生成基本的查询/变更模板。

use std::collections::HashMap;
use std::sync::Arc;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{GraphqlIntrospectConfig, NodeData};
use flow_engine_core::traits::GraphqlProvider;
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
impl NodeExecutor for GraphqlIntrospectExecutor {
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
            NodeData::GraphqlIntrospect(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 GraphqlIntrospectConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换（connection_id）
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
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

        // 3. 日志：开始探索
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: graphql_introspect, connection_id: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("GraphQL Schema 探索节点 {} 开始执行", node.name),
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
                        "[GraphQL] Schema 探索成功: uuid={}, connection_id={}, 耗时={}ms",
                        node.uuid,
                        connection_id,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[GraphQL] Schema 探索失败: uuid={}, connection_id={}, error={:?}",
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
                        "GraphQL Schema 探索成功 ({}ms), 生成 {} 个查询模板",
                        duration.as_millis(),
                        template_count
                    )
                } else {
                    format!(
                        "GraphQL Schema 探索失败: {} ({}ms)",
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
                let next_node_ids = Self::resolve_port_targets(&node.uuid, port, connections);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                // Provider 返回 Err（如连接不存在）→ 异常
                let duration = start_time.elapsed();
                log::error!(
                    "[GraphQL] Schema 探索异常: uuid={}, connection_id={}, error={}",
                    node.uuid,
                    connection_id,
                    e
                );

                // 日志：探索异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("GraphQL Schema 探索异常: {}", e),
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
        GraphqlConnectConfig, GraphqlDisconnectConfig, GraphqlQueryConfig,
        GraphqlSubscribeConfig,
    };
    use flow_engine_core::traits::{
        GraphqlConnectOutput, GraphqlConnectionInfo, GraphqlIntrospectOutput,
        GraphqlQueryOutput, GraphqlQueryTemplate, GraphqlSubscribeOutput,
    };

    // ---------------------------------------------------------------------------
    // Mock GraphqlProvider
    // ---------------------------------------------------------------------------

    /// 模拟 GraphQL Provider（探索成功，含 Schema 和模板）
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

    /// 创建测试节点
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
    async fn test_graphql_introspect_config_extraction_success() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-1", "Schema 探索", "conn-1");
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
    async fn test_graphql_introspect_config_extraction_wrong_variant() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);

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
        assert!(result.unwrap_err().contains("GraphqlIntrospectConfig"));
    }

    /// 测试：成功时路由到 true 端口
    #[tokio::test]
    async fn test_graphql_introspect_success_routes_to_true_port() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-true", "成功测试", "conn-1");
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
        // error 应为 null
        assert!(data["error"].is_null());
    }

    /// 测试：失败时路由到 false 端口
    #[tokio::test]
    async fn test_graphql_introspect_failure_routes_to_false_port() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_failure("Endpoint unreachable"));
        let executor = GraphqlIntrospectExecutor::new(graphql_manager);
        let node = create_test_node("test-false", "失败测试", "conn-1");
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
        assert_eq!(data["error"], "Endpoint unreachable");
    }

    /// 测试：输出结构包含 schema 和 query_templates
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
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        // 验证顶层字段
        assert!(data.get("success").is_some());
        assert!(data.get("schema").is_some());
        assert!(data.get("query_templates").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        // 验证 schema 包含类型信息
        let schema = data["schema"].as_object().expect("schema 应为对象");
        assert!(schema.contains_key("queryType"));
        assert!(schema.contains_key("types"));

        // 验证 query_templates 包含模板列表
        let templates = data["query_templates"]
            .as_array()
            .expect("query_templates 应为数组");
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
        let timing = data["timing"].as_object().expect("timing 应为对象");
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
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().unwrap().contains("不存在"));
        assert!(data["schema"].is_null());
        assert!(data["query_templates"].is_null());
    }

    /// 测试：无模板时 query_templates 为 null
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
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

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
        let (next_nodes, _) = result.unwrap();
        // 应路由到连接中指定的目标节点
        assert_eq!(next_nodes, vec!["next-node"]);
    }
}
