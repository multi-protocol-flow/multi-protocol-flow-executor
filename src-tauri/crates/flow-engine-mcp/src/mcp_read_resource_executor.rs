// MCP 读取资源节点执行器
//
// 实现 MCP 资源读取功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::traits::McpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// MCP 读取资源节点执行器
pub struct McpReadResourceExecutor {
    mcp_manager: Arc<dyn McpProvider>,
}

impl McpReadResourceExecutor {
    /// 创建新的 MCP 读取资源执行器
    pub fn new(mcp_manager: Arc<dyn McpProvider>) -> Self {
        Self { mcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for McpReadResourceExecutor {
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
            NodeData::McpReadResource(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not McpReadResourceConfig: {}", node.uuid))),
        };

        // 2. 变量替换 connection_id 和 resource_uri
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("Connection ID variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let resource_uri = replace_variables(&config.resource_uri, context).map_err(|e| {
            let error_msg = format!("Resource URI variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始读取
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: mcp_read_resource, Connection ID: {}, resource URI: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            resource_uri
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "MCP read resource node {} start execution (resource: {})",
                    node.name, resource_uri
                ),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 读取资源
        let read_result = self
            .mcp_manager
            .read_resource(&connection_id, &resource_uri)
            .await;

        match read_result {
            Ok(resource_result) => {
                log::info!(
                    "[MCP] Resource read succeeded: connection_id={}, uri={}, content_count={}",
                    connection_id,
                    resource_uri,
                    resource_result.contents.len()
                );

                // 5. 构建输出数据
                let duration = start_time.elapsed();

                // 发送调试事件：McpResourceRead
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::McpResourceRead {
                        node_uuid: node.uuid.clone(),
                        connection_id: connection_id.clone(),
                        uri: resource_uri.clone(),
                        duration_ms: duration.as_millis() as u64,
                    });
                }

                let output_data = json!({
                    "success": true,
                    "resource_uri": resource_uri,
                    "contents": resource_result.contents,
                    "request": {
                        "connection_id": connection_id,
                        "resource_uri": resource_uri,
                    },
                    "timing": {
                        "read_ms": resource_result.timing_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 6. 日志：读取完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "MCP resource read succeeded: {} ({}ms)",
                            resource_uri,
                            duration.as_millis()
                        ),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 7. Success走 true 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec!["true".to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[MCP] Resource read failed: connection_id={}, uri={}, error={}",
                    connection_id,
                    resource_uri,
                    e
                );

                // 日志：读取Failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MCP resource read failed: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "resource_uri": resource_uri,
                    "contents": [],
                    "request": {
                        "connection_id": connection_id,
                        "resource_uri": resource_uri,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                // Failed走 false 端口
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
    use flow_engine_core::node_config::McpReadResourceConfig;
    use crate::mcp_connection_manager::McpConnectionManager;

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, resource_uri: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mcp_read_resource".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::McpReadResource(McpReadResourceConfig {
                connection_id: connection_id.to_string(),
                resource_uri: resource_uri.to_string(),
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
        }
    }

    #[tokio::test]
    async fn test_read_resource_nonexistent_connection() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpReadResourceExecutor::new(mcp_manager);
        let node = create_test_node(
            "test-1",
            "测试读取",
            "nonexistent-conn",
            "resource://test",
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_read_resource_with_variable_replacement() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpReadResourceExecutor::new(mcp_manager);

        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}", "{{uri}}");

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("mcp-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "uri".to_string(),
            json!("resource://config"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(data["resource_uri"], "resource://config");
    }

    #[tokio::test]
    async fn test_read_resource_wrong_node_data_type() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpReadResourceExecutor::new(mcp_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
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
        assert!(result.unwrap_err().message.contains("McpReadResourceConfig"));
    }

    // ===== 调试事件测试 =====

    /// mpsc adapter for DebugEventSender (测试用)
    struct MpscDebugEventSender {
        sender: tokio::sync::mpsc::UnboundedSender<DebugEvent>,
    }

    impl MpscDebugEventSender {
        fn new(sender: tokio::sync::mpsc::UnboundedSender<DebugEvent>) -> Self {
            Self { sender }
        }
    }

    impl flow_engine_core::traits::DebugEventSender for MpscDebugEventSender {
        fn send_event(&self, event: DebugEvent) -> Result<(), String> {
            self.sender
                .send(event)
                .map_err(|e| format!("Failed to send debug event: {}", e))
        }
    }

    /// 模拟 MCP Provider（read_resource Success）
    struct MockMcpProvider;

    #[async_trait::async_trait]
    impl flow_engine_core::traits::McpProvider for MockMcpProvider {
        async fn connect(
            &self,
            _uuid: String,
            _config: flow_engine_core::node_config::McpConnectConfig,
        ) -> Result<flow_engine_core::traits::McpConnectionInfo, String> {
            Err("not implemented".to_string())
        }
        async fn call_tool(
            &self,
            _connection_id: &str,
            _tool_name: &str,
            _arguments: Option<serde_json::Value>,
        ) -> Result<flow_engine_core::traits::McpToolCallResult, String> {
            Err("not implemented".to_string())
        }
        async fn read_resource(
            &self,
            _connection_id: &str,
            _resource_uri: &str,
        ) -> Result<flow_engine_core::traits::McpResourceReadResult, String> {
            Ok(flow_engine_core::traits::McpResourceReadResult {
                contents: vec![serde_json::json!({"data": "hello"})],
                timing_ms: 30,
            })
        }
        async fn close(&self, _connection_id: &str) -> Result<(), String> {
            Ok(())
        }
        fn get_connection_info(
            &self,
            _connection_id: &str,
        ) -> Option<flow_engine_core::traits::McpConnectionInfo> {
            None
        }
        async fn list_prompts(
            &self,
            _connection_id: &str,
        ) -> Result<Vec<flow_engine_core::traits::McpPromptInfo>, String> {
            Ok(vec![])
        }
        async fn get_prompt(
            &self,
            _connection_id: &str,
            _prompt_name: &str,
            _arguments: std::collections::HashMap<String, String>,
        ) -> Result<flow_engine_core::traits::McpPromptResult, String> {
            Err("not implemented".to_string())
        }
    }

    /// 创建带调试事件发送器的上下文
    fn create_context_with_debug_sender(
        flow_id: &str,
    ) -> (
        ExecutionContext,
        tokio::sync::mpsc::UnboundedReceiver<DebugEvent>,
    ) {
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let context = ExecutionContext {
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
            debug_event_sender: Some(Arc::new(MpscDebugEventSender::new(event_sender))),
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        };
        (context, event_receiver)
    }

    /// 测试：读取Failed时不发射 McpResourceRead 调试事件
    #[tokio::test]
    async fn test_debug_event_not_emitted_on_read_failure() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpReadResourceExecutor::new(mcp_manager);
        let node = create_test_node("test-fail", "Failed测试", "nonexistent", "resource://test");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let events: Vec<_> = (&mut event_receiver).try_recv().into_iter().collect();
        assert!(
            events.is_empty(),
            "读取Failed时不应发射调试事件，但收到: {:?}",
            events
        );
    }

    /// 测试：读取Success时发射 McpResourceRead 调试事件
    #[tokio::test]
    async fn test_debug_event_mcp_resource_read() {
        let mcp_manager = Arc::new(MockMcpProvider);
        let executor = McpReadResourceExecutor::new(mcp_manager);
        let node = create_test_node("test-ok", "Success测试", "mock-conn-1", "resource://config");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        let event = event_receiver.try_recv().expect("应收到调试事件");
        match &event {
            DebugEvent::McpResourceRead {
                node_uuid,
                connection_id,
                uri,
                duration_ms,
            } => {
                assert_eq!(node_uuid, "test-ok");
                assert_eq!(connection_id, "mock-conn-1");
                assert_eq!(uri, "resource://config");
                // duration_ms 为实际执行时间
                assert!(*duration_ms < 10000, "duration_ms 应小于 10 秒");
            }
            _ => panic!("期望 McpResourceRead 事件，收到: {:?}", event),
        }

        assert!(
            event_receiver.try_recv().is_err(),
            "不应有额外的调试事件"
        );
    }
}
