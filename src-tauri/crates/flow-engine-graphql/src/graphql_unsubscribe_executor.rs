//! GraphQL 取消订阅节点执行器
//!
//! 实现 GraphQL Subscription 取消操作，停止接收事件流。

use std::collections::HashMap;
use std::sync::Arc;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::GraphqlProvider;
use serde_json::json;

/// GraphQL 取消订阅节点执行器
///
/// 处理 GraphQL Subscription 取消操作。
/// 从 `context.graphql_manager` 获取 `GraphqlProvider` 来取消订阅。
pub struct GraphqlUnsubscribeExecutor {
    graphql_manager: Arc<dyn GraphqlProvider>,
}

impl GraphqlUnsubscribeExecutor {
    /// 创建新的 GraphQL 取消订阅执行器
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
impl NodeExecutor for GraphqlUnsubscribeExecutor {
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
            NodeData::GraphqlUnsubscribe(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 GraphqlUnsubscribeConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 日志：开始取消订阅
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: graphql_unsubscribe, connection_id: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("GraphQL 取消订阅节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 3. 执行取消订阅
        let unsubscribe_result = self
            .graphql_manager
            .unsubscribe(&config.connection_id, &config)
            .await;

        match unsubscribe_result {
            Ok(()) => {
                let duration = start_time.elapsed();

                log::info!(
                    "[GraphQL] 取消订阅成功: uuid={}, connection_id={}, 耗时={}ms",
                    node.uuid,
                    config.connection_id,
                    duration.as_millis()
                );

                // 4. 构建输出数据
                let output_data = json!({
                    "success": true,
                    "error": serde_json::Value::Null,
                    "request": {
                        "connection_id": config.connection_id,
                        "subscription_id": config.subscription_id,
                    },
                    "timing": {
                        "unsubscribe_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 5. 日志：取消订阅完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "GraphQL 取消订阅成功 ({}ms)",
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

                // 6. 路由到 true 端口
                let next_node_ids =
                    Self::resolve_port_targets(&node.uuid, "true", connections);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[GraphQL] 取消订阅失败: uuid={}, connection_id={}, error={}",
                    node.uuid,
                    config.connection_id,
                    e
                );

                // 日志：取消订阅失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("GraphQL 取消订阅失败: {}", e),
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
                    "error": e,
                    "request": {
                        "connection_id": config.connection_id,
                        "subscription_id": config.subscription_id,
                    },
                    "timing": {
                        "unsubscribe_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
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
        GraphqlQueryOutput, GraphqlSubscribeOutput,
    };

    // ---------------------------------------------------------------------------
    // Mock GraphqlProvider
    // ---------------------------------------------------------------------------

    /// 模拟 GraphQL Provider（取消订阅成功）
    struct MockGraphqlProvider {
        should_fail: bool,
        error_message: String,
    }

    impl MockGraphqlProvider {
        fn new_success() -> Self {
            Self {
                should_fail: false,
                error_message: String::new(),
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                should_fail: true,
                error_message: error.to_string(),
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

        async fn unsubscribe(
            &self,
            _uuid: &str,
            _config: &GraphqlUnsubscribeConfig,
        ) -> Result<(), String> {
            if self.should_fail {
                Err(self.error_message.clone())
            } else {
                Ok(())
            }
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

    // ---------------------------------------------------------------------------
    // 测试辅助函数
    // ---------------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        subscription_id: Option<&str>,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "graphql_unsubscribe".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlUnsubscribe(GraphqlUnsubscribeConfig {
                connection_id: connection_id.to_string(),
                subscription_id: subscription_id.map(|s| s.to_string()),
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
    async fn test_graphql_unsubscribe_config_extraction_success() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlUnsubscribeExecutor::new(graphql_manager);
        let node = create_test_node("test-1", "取消订阅", "conn-1", Some("sub-1"));
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
    async fn test_graphql_unsubscribe_config_extraction_wrong_variant() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlUnsubscribeExecutor::new(graphql_manager);

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
        assert!(result.unwrap_err().contains("GraphqlUnsubscribeConfig"));
    }

    /// 测试：成功时路由到 true 端口，失败时路由到 false 端口
    #[tokio::test]
    async fn test_graphql_unsubscribe_true_false_port_routing() {
        // 测试成功 → true
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlUnsubscribeExecutor::new(graphql_manager);
        let node = create_test_node("test-true", "成功测试", "conn-1", Some("sub-1"));
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);

        // 测试失败 → false
        let graphql_manager_fail = Arc::new(MockGraphqlProvider::new_failure("订阅不存在"));
        let executor_fail = GraphqlUnsubscribeExecutor::new(graphql_manager_fail);
        let node_fail = create_test_node("test-false", "失败测试", "conn-1", Some("sub-1"));
        let mut context_fail = create_context("test-flow");

        let result_fail = executor_fail
            .execute(&node_fail, &mut context_fail, &[], &HashMap::new())
            .await;
        assert!(result_fail.is_ok());
        let (next_nodes_fail, _) = result_fail.unwrap();
        assert_eq!(next_nodes_fail, vec!["false"]);
    }

    /// 测试：输出结构包含所有必要字段
    #[tokio::test]
    async fn test_graphql_unsubscribe_report_input_structure() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlUnsubscribeExecutor::new(graphql_manager);

        let node = Node {
            uuid: "test-report".to_string(),
            node_type: "graphql_unsubscribe".to_string(),
            name: "报告结构测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GraphqlUnsubscribe(GraphqlUnsubscribeConfig {
                connection_id: "conn-1".to_string(),
                subscription_id: Some("sub-abc-123".to_string()),
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
        assert!(data.get("error").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());

        // 验证 success
        assert_eq!(data["success"], true);

        // 验证 request 子结构
        let request = &data["request"];
        assert_eq!(request["connection_id"], "conn-1");
        assert_eq!(request["subscription_id"], "sub-abc-123");

        // 验证 timing 子结构
        let timing = data["timing"].as_object().expect("timing 应为对象");
        assert!(timing.contains_key("unsubscribe_ms"));
        assert!(timing.contains_key("total_ms"));
    }

    /// 测试：Provider 返回 Err 时走 false 端口且包含错误信息
    #[tokio::test]
    async fn test_graphql_unsubscribe_provider_error_includes_error() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_failure("连接不存在"));
        let executor = GraphqlUnsubscribeExecutor::new(graphql_manager);
        let node = create_test_node("test-err", "异常测试", "bad-conn", Some("sub-x"));
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

    /// 测试：连接端口路由（通过 connections 查找目标节点）
    #[tokio::test]
    async fn test_graphql_unsubscribe_with_connections_routing() {
        let graphql_manager = Arc::new(MockGraphqlProvider::new_success());
        let executor = GraphqlUnsubscribeExecutor::new(graphql_manager);
        let node = create_test_node("test-route", "路由测试", "conn-1", Some("sub-1"));
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
