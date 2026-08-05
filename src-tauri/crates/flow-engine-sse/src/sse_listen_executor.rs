//! SSE 监听执行器
//!
//! 从已建立的 SSE 连接监听事件，支持事件过滤、最大事件数和超时控制。
//! Success收到事件路由到 true 端口，超时或无事件路由到 false 端口。

use flow_engine_core::error::ErrorDetail;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{NodeData, SseListenConfig};
use flow_engine_core::traits::SseProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// SSE 监听节点执行器
pub struct SseListenExecutor {
    sse_manager: Arc<dyn SseProvider>,
}

impl SseListenExecutor {
    /// 创建新的 SSE 监听执行器
    pub fn new(sse_manager: Arc<dyn SseProvider>) -> Self {
        Self { sse_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for SseListenExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        // 1. 提取 SseListenConfig
        let config = match &node.data {
            NodeData::SseListen(cfg) => cfg.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not SseListenConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换：connection_id 和 event_filter
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            ErrorDetail::execution_error(format!(
                "connection_id variable substitution failed: {}",
                e
            ))
        })?;

        let event_filter = match &config.event_filter {
            Some(filter) => Some(replace_variables(filter, context).map_err(|e| {
                ErrorDetail::execution_error(format!(
                    "event_filter variable substitution failed: {}",
                    e
                ))
            })?),
            None => None,
        };

        // 3. 日志：开始监听
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: sse_listen, connection_id: {}, event_filter: {:?}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            event_filter
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "SSE 监听节点 {} start execution: connection_id={}",
                    node.name, connection_id
                ),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建监听配置（使用替换后的值）
        let listen_config = SseListenConfig {
            connection_id: connection_id.clone(),
            event_filter: event_filter.clone(),
            max_events: config.max_events,
            timeout_ms: config.timeout_ms,
            include_data: config.include_data,
            decode_json: config.decode_json,
        };

        // 5. 调用 provider.listen()，传入 connection_id（即 connect 节点的 UUID）
        let listen_result = self
            .sse_manager
            .listen(&connection_id, &listen_config)
            .await;

        match listen_result {
            Ok(output) => {
                let duration = start_time.elapsed();
                let event_count = output.event_count;

                // 6. 构建输出 JSON
                let output_data = json!({
                    "success": output.success,
                    "events": output.events.iter().map(|evt| {
                        let mut obj = json!({
                            "data": evt.data,
                            "received_at": evt.received_at,
                        });
                        if let Some(ref event_name) = evt.event {
                            obj["event"] = json!(event_name);
                        }
                        if let Some(ref id) = evt.id {
                            obj["id"] = json!(id);
                        }
                        if let Some(ref decoded) = evt.data_decoded {
                            obj["data_decoded"] = json!(decoded);
                        }
                        obj
                    }).collect::<Vec<_>>(),
                    "event_count": event_count,
                    "duration_ms": duration.as_millis() as u64,
                    "error": output.error,
                    "request": {
                        "connection_id": connection_id,
                        "event_filter": event_filter,
                        "max_events": config.max_events,
                        "timeout_ms": config.timeout_ms,
                    },
                });

                // 7. 路由：Success且有事件 → true，否则 → false
                if !output.success || event_count == 0 {
                    // 检查 on_error 策略
                    if !node
                        .on_error
                        .as_ref()
                        .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                    {
                        let error_msg = output_data
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("SSE 监听无事件或Failed")
                            .to_string();
                        return Err(ErrorDetail::execution_error(error_msg));
                    }
                }
                let (port, is_success) = if output.success && event_count > 0 {
                    ("true", true)
                } else {
                    ("false", false)
                };

                if is_success {
                    log::info!(
                        "[SSE] 监听Success: connection_id={}, 事件数={}, 耗时={}ms",
                        connection_id,
                        event_count,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[SSE] 监听超时或无事件: connection_id={}, 事件数={}, 耗时={}ms",
                        connection_id,
                        event_count,
                        duration.as_millis()
                    );
                }

                // 8. 日志：监听完成
                let log_level = if is_success {
                    LogLevel::Info
                } else {
                    LogLevel::Warn
                };
                let log_msg = if is_success {
                    format!(
                        "SSE 监听Success: 收到 {} 个事件 ({}ms)",
                        event_count,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "SSE 监听超时或无事件: 事件数={} ({}ms)",
                        event_count,
                        duration.as_millis()
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_msg,
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9. 端口路由
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
                let duration = start_time.elapsed();

                log::error!(
                    "[SSE] 监听异常: uuid={}, connection_id={}, error={}",
                    node.uuid,
                    connection_id,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("SSE 监听异常: {}", e),
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
                    "error": e,
                    "request": {
                        "connection_id": connection_id,
                        "event_filter": event_filter,
                        "max_events": config.max_events,
                        "timeout_ms": config.timeout_ms,
                    },
                });

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(e));
                }

                // Failed时走 false 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
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
    use flow_engine_core::node_config::SseConnectConfig;
    use flow_engine_core::traits::{
        SseConnectOutput, SseConnectionInfo, SseEventItem, SseListenOutput,
    };

    // ---------------------------------------------------------------------------
    // 辅助工具
    // ---------------------------------------------------------------------------

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "sse_listen".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseListen(SseListenConfig {
                connection_id: connection_id.to_string(),
                event_filter: None,
                max_events: 10,
                timeout_ms: 5000,
                include_data: true,
                decode_json: false,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建带事件过滤的Test Node
    fn create_test_node_with_filter(
        uuid: &str,
        name: &str,
        connection_id: &str,
        event_filter: &str,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "sse_listen".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseListen(SseListenConfig {
                connection_id: connection_id.to_string(),
                event_filter: Some(event_filter.to_string()),
                max_events: 10,
                timeout_ms: 5000,
                include_data: true,
                decode_json: false,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建执行上下文（无 `sse_manager`）
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
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
    // Mock SseProvider
    // ---------------------------------------------------------------------------

    /// 模拟 SseProvider（控制 listen 返回值）
    struct MockSseProvider {
        listen_output: SseListenOutput,
        listen_error: Option<String>,
    }

    impl MockSseProvider {
        fn new_success(events: Vec<SseEventItem>) -> Self {
            let event_count = events.len() as u32;
            Self {
                listen_output: SseListenOutput {
                    success: true,
                    events,
                    event_count,
                    duration_ms: 100,
                    error: None,
                },
                listen_error: None,
            }
        }

        fn new_timeout() -> Self {
            Self {
                listen_output: SseListenOutput {
                    success: true,
                    events: vec![],
                    event_count: 0,
                    duration_ms: 5000,
                    error: Some("监听超时".to_string()),
                },
                listen_error: None,
            }
        }

        fn new_error(error: &str) -> Self {
            Self {
                listen_output: SseListenOutput {
                    success: false,
                    events: vec![],
                    event_count: 0,
                    duration_ms: 0,
                    error: Some(error.to_string()),
                },
                listen_error: Some(error.to_string()),
            }
        }
    }

    #[async_trait::async_trait]
    impl SseProvider for MockSseProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &SseConnectConfig,
        ) -> Result<SseConnectOutput, String> {
            Ok(SseConnectOutput {
                success: true,
                connection_id: "mock".to_string(),
                server_headers: None,
                last_event_id: None,
                error: None,
            })
        }

        async fn listen(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::SseListenConfig,
        ) -> Result<SseListenOutput, String> {
            match &self.listen_error {
                Some(e) => Err(e.clone()),
                None => Ok(self.listen_output.clone()),
            }
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::SseDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<SseConnectionInfo> {
            vec![]
        }
    }

    // ---------------------------------------------------------------------------
    // 测试用例
    // ---------------------------------------------------------------------------

    /// 测试：Success监听并收到事件，路由到 true 端口
    #[tokio::test]
    async fn test_sse_listen_success_with_events() {
        let events = vec![
            SseEventItem {
                event: Some("message".to_string()),
                id: Some("1".to_string()),
                data: "hello world".to_string(),
                data_decoded: None,
                received_at: 1700000000000,
            },
            SseEventItem {
                event: Some("update".to_string()),
                id: Some("2".to_string()),
                data: r#"{"key":"value"}"#.to_string(),
                data_decoded: Some(json!({"key": "value"})),
                received_at: 1700000001000,
            },
        ];

        let provider = Arc::new(MockSseProvider::new_success(events));
        let executor = SseListenExecutor::new(provider);
        let node = create_test_node("listen-1", "SSE监听", "conn-1");
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
        assert_eq!(data["event_count"], 2);

        // 验证事件内容
        let events = data["events"]
            .as_array()
            .unwrap_or_else(|| panic!("events 应为数组"));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["event"], "message");
        assert_eq!(events[0]["id"], "1");
        assert_eq!(events[0]["data"], "hello world");
        assert_eq!(events[0]["received_at"], 1700000000000_u64);
        assert_eq!(events[1]["event"], "update");
        assert_eq!(events[1]["data_decoded"]["key"], "value");

        // 验证 request 信息
        assert_eq!(data["request"]["connection_id"], "conn-1");
    }

    /// 测试：监听超时/无事件，路由到 false 端口
    #[tokio::test]
    async fn test_sse_listen_timeout_routes_to_false() {
        let provider = Arc::new(MockSseProvider::new_timeout());
        let executor = SseListenExecutor::new(provider);
        let node = create_test_node("listen-2", "超时测试", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        // 超时时 success=true（provider 本身没出错），但 event_count=0
        assert_eq!(data["success"], true);
        assert_eq!(data["event_count"], 0);
        assert_eq!(data["events"].as_array().map(|a| a.len()), Some(0));
    }

    /// 测试：provider 返回Error，路由到 false 端口
    #[tokio::test]
    async fn test_sse_listen_error_routes_to_false() {
        let provider = Arc::new(MockSseProvider::new_error("连接 'conn-missing' 不存在"));
        let executor = SseListenExecutor::new(provider);
        let node = create_test_node("listen-3", "Error测试", "conn-missing");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "连接 'conn-missing' 不存在");
        assert_eq!(data["event_count"], 0);
    }

    /// 测试：节点数据类型不匹配
    #[tokio::test]
    async fn test_sse_listen_wrong_node_data_type() {
        let provider = Arc::new(MockSseProvider::new_success(vec![]));
        let executor = SseListenExecutor::new(provider);

        let node = Node {
            uuid: "wrong-type".to_string(),
            node_type: "http".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
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
            .contains("SseListenConfig"));
    }

    /// 测试：Config 提取自 `NodeData::SseListen`
    #[tokio::test]
    async fn test_sse_listen_config_extraction() {
        let provider = Arc::new(MockSseProvider::new_success(vec![]));
        let executor = SseListenExecutor::new(provider);

        let node = Node {
            uuid: "extract-1".to_string(),
            node_type: "sse_listen".to_string(),
            name: "配置提取".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseListen(SseListenConfig {
                connection_id: "my-conn".to_string(),
                event_filter: Some("ping".to_string()),
                max_events: 5,
                timeout_ms: 3000,
                include_data: true,
                decode_json: true,
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
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));

        // 验证 request 字段包含正确配置
        assert_eq!(data["request"]["connection_id"], "my-conn");
        assert_eq!(data["request"]["event_filter"], "ping");
        assert_eq!(data["request"]["max_events"], 5);
        assert_eq!(data["request"]["timeout_ms"], 3000);
    }

    /// 测试：事件过滤
    #[tokio::test]
    async fn test_sse_listen_with_event_filter() {
        let events = vec![SseEventItem {
            event: Some("ping".to_string()),
            id: None,
            data: "pong".to_string(),
            data_decoded: None,
            received_at: 1700000000000,
        }];

        let provider = Arc::new(MockSseProvider::new_success(events));
        let executor = SseListenExecutor::new(provider);
        let node = create_test_node_with_filter("filter-1", "过滤测试", "conn-1", "ping");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["event_count"], 1);
        assert_eq!(data["request"]["event_filter"], "ping");
    }

    /// 测试：变量替换在 `connection_id` 和 `event_filter` 中生效
    #[tokio::test]
    async fn test_sse_listen_variable_replacement() {
        let events = vec![SseEventItem {
            event: Some("message".to_string()),
            id: None,
            data: "test data".to_string(),
            data_decoded: None,
            received_at: 1700000000000,
        }];

        let provider = Arc::new(MockSseProvider::new_success(events));
        let executor = SseListenExecutor::new(provider);

        let node = Node {
            uuid: "var-1".to_string(),
            node_type: "sse_listen".to_string(),
            name: "变量替换".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseListen(SseListenConfig {
                connection_id: "{{conn_id}}".to_string(),
                event_filter: Some("{{event_name}}".to_string()),
                max_events: 0,
                timeout_ms: 10000,
                include_data: false,
                decode_json: false,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("my-connection"),
            String::new(),
        );
        context.variables.set_variable(
            "event_name".to_string(),
            json!("notification"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["request"]["connection_id"], "my-connection");
        assert_eq!(data["request"]["event_filter"], "notification");
    }

    /// 测试：连接端口路由（true 端口指向具体节点）
    #[tokio::test]
    async fn test_sse_listen_connection_routing() {
        let events = vec![SseEventItem {
            event: Some("data".to_string()),
            id: None,
            data: "payload".to_string(),
            data_decoded: None,
            received_at: 1700000000000,
        }];

        let provider = Arc::new(MockSseProvider::new_success(events));
        let executor = SseListenExecutor::new(provider);
        let node = create_test_node("route-1", "路由测试", "conn-1");
        let mut context = create_context("test-flow");

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "route-1".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["next-node"]);
    }

    /// 测试：Error时连接端口路由（false 端口指向具体节点）
    #[tokio::test]
    async fn test_sse_listen_error_connection_routing() {
        let provider = Arc::new(MockSseProvider::new_error("连接不存在"));
        let executor = SseListenExecutor::new(provider);
        let node = create_test_node("route-err", "路由Error", "conn-1");
        let mut context = create_context("test-flow");

        let connections = vec![Connection {
            id: "conn-err".to_string(),
            source_node_uuid: "route-err".to_string(),
            target_node_uuid: "error-handler".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["error-handler"]);
    }

    /// `测试：max_events=0` 且无事件时走 false 端口
    #[tokio::test]
    async fn test_sse_listen_max_events_zero_no_events() {
        let provider = Arc::new(MockSseProvider::new_timeout());
        let executor = SseListenExecutor::new(provider);

        let node = Node {
            uuid: "max-zero".to_string(),
            node_type: "sse_listen".to_string(),
            name: "max_events=0".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseListen(SseListenConfig {
                connection_id: "conn-1".to_string(),
                event_filter: None,
                max_events: 0,
                timeout_ms: 1000,
                include_data: false,
                decode_json: false,
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
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false"]);

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["event_count"], 0);
        assert_eq!(data["request"]["max_events"], 0);
    }

    /// 测试：输出 JSON 结构完整性
    #[tokio::test]
    async fn test_sse_listen_output_structure() {
        let events = vec![SseEventItem {
            event: None,
            id: None,
            data: "plain data".to_string(),
            data_decoded: None,
            received_at: 1700000000000,
        }];

        let provider = Arc::new(MockSseProvider::new_success(events));
        let executor = SseListenExecutor::new(provider);
        let node = create_test_node("struct-1", "结构测试", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));

        // 验证顶层字段
        assert!(data.get("success").is_some());
        assert!(data.get("events").is_some());
        assert!(data.get("event_count").is_some());
        assert!(data.get("duration_ms").is_some());
        assert!(data.get("error").is_some());
        assert!(data.get("request").is_some());

        // 验证 request 子字段
        let request = &data["request"];
        assert!(request.get("connection_id").is_some());
        assert!(request.get("event_filter").is_some());
        assert!(request.get("max_events").is_some());
        assert!(request.get("timeout_ms").is_some());

        // 验证事件字段（event 和 id 为 None 时不出现）
        let events = data["events"]
            .as_array()
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["data"], "plain data");
        assert_eq!(events[0]["received_at"], 1700000000000_u64);
        // event 和 id 为 None，不应在输出中出现
        assert!(events[0].get("event").is_none());
        assert!(events[0].get("id").is_none());
    }
}
