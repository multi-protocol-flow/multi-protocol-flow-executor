//! GraphQL 订阅节点执行器
//!
//! 实现 GraphQL Subscription 操作，通过 WebSocket 接收事件流。
//! 支持变量替换、操作名称指定、最大事件数和超时配置。

use flow_engine_core::error::ErrorDetail;
use std::collections::HashMap;
use std::sync::Arc;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{GraphqlSubscribeConfig, NodeData};
use flow_engine_core::traits::GraphqlProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// GraphQL 订阅节点执行器
///
/// 处理 GraphQL Subscription 操作。
/// 从 `context.get_provider::<GraphqlConnectionManager>("graphql")` 获取连接管理器来执行订阅。
pub struct GraphqlSubscribeExecutor {
    graphql_manager: Arc<dyn GraphqlProvider>,
}

impl GraphqlSubscribeExecutor {
    /// 创建新的 GraphQL 订阅执行器
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
impl NodeExecutor for GraphqlSubscribeExecutor {
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
            NodeData::GraphqlSubscribe(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not GraphqlSubscribeConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（subscription, variables, operation_name）
        let subscription = replace_variables(&config.subscription, context).map_err(|e| {
            let error_msg = format!("subscription variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let operation_name = match &config.operation_name {
            Some(op_name) => {
                let replaced = replace_variables(op_name, context).map_err(|e| {
                    ErrorDetail::execution_error(format!(
                        "operation_name variable substitution failed: {}",
                        e
                    ))
                })?;
                Some(replaced)
            }
            None => None,
        };

        // variables 变量替换：序列化为字符串后替换，再解析回 JSON
        let variables = match &config.variables {
            Some(vars) => {
                let vars_str = serde_json::to_string(vars).unwrap_or_default();
                let replaced_str = replace_variables(&vars_str, context).map_err(|e| {
                    ErrorDetail::execution_error(format!(
                        "variables variable substitution failed: {}",
                        e
                    ))
                })?;
                let replaced_vars: serde_json::Value =
                    serde_json::from_str(&replaced_str).unwrap_or_else(|_| vars.clone());
                Some(replaced_vars)
            }
            None => None,
        };

        // 3. 日志：开始订阅
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: graphql_subscribe, connection_id: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("GraphQL subscribe node {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建替换后的配置
        let resolved_config = GraphqlSubscribeConfig {
            connection_id: config.connection_id.clone(),
            subscription,
            variables,
            operation_name,
            max_events: config.max_events,
            timeout_ms: config.timeout_ms,
        };

        // 5. 执行订阅
        let subscribe_result = self
            .graphql_manager
            .subscribe(&resolved_config.connection_id, &resolved_config)
            .await;

        match subscribe_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[GraphQL] Subscribe succeeded: uuid={}, connection_id={}, event_count={}, duration={}ms",
                        node.uuid,
                        config.connection_id,
                        output.event_count,
                        output.duration_ms
                    );
                } else {
                    log::warn!(
                        "[GraphQL] Subscribe failed: uuid={}, connection_id={}, error={:?}",
                        node.uuid,
                        config.connection_id,
                        output.error
                    );
                }

                // 6. 构建输出数据
                let events_json: Vec<serde_json::Value> = output
                    .events
                    .iter()
                    .map(|evt| {
                        json!({
                            "data": evt.data,
                            "errors": evt.errors,
                            "extensions": evt.extensions,
                            "received_at": evt.received_at,
                        })
                    })
                    .collect();

                let output_data = json!({
                    "success": output.success,
                    "events": events_json,
                    "event_count": output.event_count,
                    "duration_ms": output.duration_ms,
                    "request": {
                        "connection_id": config.connection_id,
                        "subscription": resolved_config.subscription,
                        "operation_name": resolved_config.operation_name,
                        "max_events": config.max_events,
                        "timeout_ms": config.timeout_ms,
                    },
                    "timing": {
                        "subscribe_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 7. 日志：订阅完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "GraphQL subscribe succeeded: received {} events ({}ms)",
                        output.event_count,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "GraphQL subscribe failed: {} ({}ms)",
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
                        .unwrap_or("GraphQL subscribe failed")
                        .to_string();
                    return Err(ErrorDetail::execution_error(error_msg));
                }
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = Self::resolve_port_targets(&node.uuid, port, connections);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[GraphQL] Subscribe error: uuid={}, connection_id={}, error={}",
                    node.uuid,
                    config.connection_id,
                    e
                );

                // 日志：订阅异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("GraphQL subscribe error: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出
                let error_output = json!({
                    "success": false,
                    "events": [],
                    "event_count": 0,
                    "duration_ms": duration.as_millis() as u64,
                    "request": {
                        "connection_id": config.connection_id,
                        "subscription": resolved_config.subscription,
                    },
                    "timing": {
                        "subscribe_ms": duration.as_millis() as u64,
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
        GraphqlConnectConfig, GraphqlDisconnectConfig, GraphqlIntrospectConfig, GraphqlQueryConfig,
        GraphqlSubscribeConfig,
    };
    use flow_engine_core::traits::{
        GraphqlConnectOutput, GraphqlConnectionInfo, GraphqlIntrospectOutput, GraphqlQueryOutput,
        GraphqlSubscribeOutput, GraphqlSubscriptionEvent,
    };

    // ---------------------------------------------------------------------------
    // Mock GraphqlProvider
    // ---------------------------------------------------------------------------

    /// 模拟 GraphQL Provider（订阅Success）
    struct MockGraphqlProvider {
        output: GraphqlSubscribeOutput,
    }

    impl MockGraphqlProvider {
        fn new_success() -> Self {
            Self {
                output: GraphqlSubscribeOutput {
                    success: true,
                    events: vec![GraphqlSubscriptionEvent {
                        data: Some(json!({ "messageAdded": { "text": "hello" } })),
                        errors: vec![],
                        extensions: None,
                        received_at: 1714089600000,
                    }],
                    event_count: 1,
                    duration_ms: 150,
                    error: None,
                },
            }
        }

        fn new_success_multiple_events() -> Self {
            Self {
                output: GraphqlSubscribeOutput {
                    success: true,
                    events: vec![
                        GraphqlSubscriptionEvent {
                            data: Some(json!({ "messageAdded": { "text": "hello" } })),
                            errors: vec![],
                            extensions: None,
                            received_at: 1714089600000,
                        },
                        GraphqlSubscriptionEvent {
                            data: Some(json!({ "messageAdded": { "text": "world" } })),
                            errors: vec![],
                            extensions: None,
                            received_at: 1714089601000,
                        },
                    ],
                    event_count: 2,
                    duration_ms: 300,
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: GraphqlSubscribeOutput {
                    success: false,
                    events: vec![],
                    event_count: 0,
                    duration_ms: 0,
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
            Ok(self.output.clone())
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

    /// 模拟 GraphQL Provider（subscribe 返回 Err）
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
            Err("连接 'missing-conn' 不存在".to_string())
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

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, subscription: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "graphql_subscribe".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlSubscribe(GraphqlSubscribeConfig {
                connection_id: connection_id.to_string(),
                subscription: subscription.to_string(),
                variables: None,
                operation_name: None,
                max_events: 10,
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
            call_stack: Vec::new(),
            current_sub_flow_report: None,
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
    async fn test_graphql_subscribe_config_extraction_success() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlSubscribeExecutor::new(graphql_manager);
        let node = create_test_node(
            "test-1",
            "订阅消息",
            "conn-1",
            "subscription { messageAdded { text } }",
        );
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
    async fn test_graphql_subscribe_config_extraction_wrong_variant() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlSubscribeExecutor::new(graphql_manager);

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
            .contains("GraphqlSubscribeConfig"));
    }

    /// 测试：变量替换在 subscription、variables 和 `operation_name` 中生效
    #[tokio::test]
    async fn test_graphql_subscribe_with_variable_replacement() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlSubscribeExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "graphql_subscribe".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlSubscribe(GraphqlSubscribeConfig {
                connection_id: "conn-1".to_string(),
                subscription: "subscription { messageAdded(roomId: {{room_id}}) { text } }"
                    .to_string(),
                variables: Some(json!({
                    "roomId": "{{room_id}}"
                })),
                operation_name: Some("OnMessage_{{suffix}}".to_string()),
                max_events: 10,
                timeout_ms: 30000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("room_id".to_string(), json!("42"), String::new());
        context
            .variables
            .set_variable("suffix".to_string(), json!("v1"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        // 验证变量替换在 subscription 中生效
        assert_eq!(
            data["request"]["subscription"],
            "subscription { messageAdded(roomId: 42) { text } }"
        );
        // 验证 operation_name 替换
        assert_eq!(data["request"]["operation_name"], "OnMessage_v1");
    }

    /// 测试：Success时路由到 true 端口，Failed时路由到 false 端口
    #[tokio::test]
    async fn test_graphql_subscribe_true_false_port_routing() {
        // 测试Success → true
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlSubscribeExecutor::new(graphql_manager);
        let node = create_test_node(
            "test-true",
            "Success测试",
            "conn-1",
            "subscription { messageAdded { text } }",
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);

        // 测试Failed → false
        let graphql_manager_fail = Arc::new(MockGraphqlProvider::new_failure(
            "WebSocket connection failed",
        ));
        let executor_fail = GraphqlSubscribeExecutor::new(graphql_manager_fail);
        let node_fail = create_test_node(
            "test-false",
            "Failed测试",
            "conn-1",
            "subscription { messageAdded { text } }",
        );
        let mut context_fail = create_context("test-flow");

        let result_fail = executor_fail
            .execute(&node_fail, &mut context_fail, &[], &HashMap::new())
            .await;
        assert!(result_fail.is_ok());
        let (next_nodes_fail, _) = result_fail.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes_fail, vec!["false"]);
    }

    /// 测试：输出结构包含所有必要字段
    #[tokio::test]
    async fn test_graphql_subscribe_report_input_structure() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success_multiple_events());
        let executor = GraphqlSubscribeExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "graphql_subscribe".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlSubscribe(GraphqlSubscribeConfig {
                connection_id: "conn-1".to_string(),
                subscription: "subscription { messageAdded { text } }".to_string(),
                variables: Some(json!({"roomId": "42"})),
                operation_name: Some("OnMessage".to_string()),
                max_events: 5,
                timeout_ms: 10000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        let data = output_data.unwrap_or_else(|| panic!("expected output data"));

        // 验证顶层字段
        assert!(data.get("success").is_some());
        assert!(data.get("events").is_some());
        assert!(data.get("event_count").is_some());
        assert!(data.get("duration_ms").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());
        assert!(data.get("error").is_some());

        // 验证事件列表
        let events = data["events"]
            .as_array()
            .unwrap_or_else(|| panic!("expected events array"));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["data"]["messageAdded"]["text"], "hello");
        assert_eq!(events[1]["data"]["messageAdded"]["text"], "world");

        // 验证 event_count
        assert_eq!(data["event_count"], 2);

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["connection_id"], "conn-1");
        assert_eq!(
            request["subscription"],
            "subscription { messageAdded { text } }"
        );
        assert_eq!(request["operation_name"], "OnMessage");
        assert_eq!(request["max_events"], 5);
        assert_eq!(request["timeout_ms"], 10000);

        // 验证 timing 子结构
        let timing = data["timing"]
            .as_object()
            .unwrap_or_else(|| panic!("expected timing object"));
        assert!(timing.contains_key("subscribe_ms"));
        assert!(timing.contains_key("total_ms"));
    }

    /// 测试：Provider 返回 Err 时（如连接不存在）走 false 端口
    #[tokio::test]
    async fn test_graphql_subscribe_provider_error_routes_to_false() {
        let graphql_manager = Arc::new(MockGraphqlProviderError);
        let executor = GraphqlSubscribeExecutor::new(graphql_manager);
        let node = create_test_node(
            "test-err",
            "异常测试",
            "missing-conn",
            "subscription { messageAdded { text } }",
        );
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
    }

    /// 测试：连接端口路由（通过 connections 查找目标节点）
    #[tokio::test]
    async fn test_graphql_subscribe_with_connections_routing() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlSubscribeExecutor::new(graphql_manager);
        let node = create_test_node(
            "test-route",
            "路由测试",
            "conn-1",
            "subscription { messageAdded { text } }",
        );
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
