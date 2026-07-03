// MCP 关闭节点执行器
//
// 实现 MCP 连接关闭功能。
// 关闭始终走 "out" 端口（Failed不阻塞流程）。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::McpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// MCP 关闭节点执行器
pub struct McpCloseExecutor {
    mcp_manager: Arc<dyn McpProvider>,
}

impl McpCloseExecutor {
    /// 创建新的 MCP 关闭执行器
    pub fn new(mcp_manager: Arc<dyn McpProvider>) -> Self {
        Self { mcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for McpCloseExecutor {
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
            NodeData::McpClose(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not McpCloseConfig: {}", node.uuid))),
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始关闭
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: mcp_close, Connection ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("MCP Close 节点 {} start execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间Error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 获取连接信息（关闭前获取 URL，关闭后不可用）
        let conn_url = self
            .mcp_manager
            .get_connection_info(&connection_id)
            .map(|info| info.url)
            .unwrap_or_default();

        // 5. 关闭连接
        let close_result = self.mcp_manager.close(&connection_id).await;

        // MCP Close 始终走 "out" 端口，不阻塞流程
        let next_node_ids: Vec<String> = {
            let filtered: Vec<String> = connections
                .iter()
                .filter(|conn| {
                    conn.source_node_uuid == node.uuid && conn.source_port_id == "out"
                })
                .map(|conn| conn.target_node_uuid.clone())
                .collect();

            if filtered.is_empty() {
                vec!["out".to_string()]
            } else {
                filtered
            }
        };

        let duration = start_time.elapsed();

        match close_result {
            Ok(()) => {
                log::info!(
                    "[MCP] 连接关闭Success: connection_id={}",
                    connection_id
                );

                // 发送调试事件：McpConnectionClosed
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::McpConnectionClosed {
                        node_uuid: node.uuid.clone(),
                        connection_id: connection_id.clone(),
                        url: conn_url.clone(),
                    });
                }

                // 6. 构建输出数据
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                    },
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 6. 日志：关闭完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "MCP 连接关闭Success ({}ms)",
                            duration.as_millis()
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间Error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[MCP] 连接关闭Failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：关闭Failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MCP 连接关闭Failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间Error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据（仍然走 out 端口）
                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                    },
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

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
    use flow_engine_core::node_config::McpCloseConfig;
    use crate::mcp_connection_manager::McpConnectionManager;

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mcp_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::McpClose(McpCloseConfig {
                connection_id: connection_id.to_string(),
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
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间Error")
                .as_millis() as u64,
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
    async fn test_close_connection_not_found() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpCloseExecutor::new(mcp_manager);
        let node = create_test_node("test-1", "测试关闭", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口（关闭Failed不阻塞）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "nonexistent-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_close_with_variable_replacement() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpCloseExecutor::new(mcp_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("mcp-connection-123"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "mcp-connection-123");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_close_wrong_node_data_type() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpCloseExecutor::new(mcp_manager);

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
        assert!(result.unwrap_err().message.contains("McpCloseConfig"));
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

    /// 模拟 MCP Provider（close + get_connection_info Success）
    struct MockMcpProvider {
        url: String,
    }

    impl MockMcpProvider {
        fn new() -> Self {
            Self {
                url: "http://mock-server/mcp".to_string(),
            }
        }
    }

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
            Err("not implemented".to_string())
        }
        async fn close(&self, _connection_id: &str) -> Result<(), String> {
            Ok(())
        }
        fn get_connection_info(
            &self,
            _connection_id: &str,
        ) -> Option<flow_engine_core::traits::McpConnectionInfo> {
            use flow_engine_core::traits::*;
            Some(McpConnectionInfo {
                connection_id: "mock-conn-1".to_string(),
                url: self.url.clone(),
                server_info: McpServerInfo {
                    name: "mock-server".to_string(),
                    version: Some("1.0".to_string()),
                },
                tools: vec![],
                resources: vec![],
                prompts: vec![],
                transport: flow_engine_core::node_config::McpTransportMode::StreamableHttp,
            })
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
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间Error")
                .as_millis() as u64,
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

    /// 测试：关闭Failed时不发射 McpConnectionClosed 调试事件
    #[tokio::test]
    async fn test_debug_event_not_emitted_on_close_failure() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpCloseExecutor::new(mcp_manager);
        let node = create_test_node("test-fail", "Failed测试", "nonexistent-conn");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let events: Vec<_> = (&mut event_receiver).try_recv().into_iter().collect();
        assert!(
            events.is_empty(),
            "关闭Failed时不应发射调试事件，但收到: {:?}",
            events
        );
    }

    /// 测试：关闭Success时发射 McpConnectionClosed 调试事件
    #[tokio::test]
    async fn test_debug_event_mcp_connection_closed() {
        let mcp_manager = Arc::new(MockMcpProvider::new());
        let executor = McpCloseExecutor::new(mcp_manager);
        let node = create_test_node("test-ok", "Success测试", "mock-conn-1");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        let event = event_receiver.try_recv().expect("应收到调试事件");
        match &event {
            DebugEvent::McpConnectionClosed {
                node_uuid,
                connection_id,
                url,
            } => {
                assert_eq!(node_uuid, "test-ok");
                assert_eq!(connection_id, "mock-conn-1");
                assert_eq!(url, "http://mock-server/mcp");
            }
            _ => panic!("期望 McpConnectionClosed 事件，收到: {:?}", event),
        }

        assert!(
            event_receiver.try_recv().is_err(),
            "不应有额外的调试事件"
        );
    }
}
