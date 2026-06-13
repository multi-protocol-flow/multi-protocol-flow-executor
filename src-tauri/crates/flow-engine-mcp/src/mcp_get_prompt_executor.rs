// MCP 获取提示词节点执行器
//
// 实现 MCP 提示词获取功能。

use flow_engine_core::connection::Connection;
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

/// MCP 获取提示词节点执行器
pub struct McpGetPromptExecutor {
    mcp_manager: Arc<dyn McpProvider>,
}

impl McpGetPromptExecutor {
    /// 创建新的 MCP 获取提示词执行器
    pub fn new(mcp_manager: Arc<dyn McpProvider>) -> Self {
        Self { mcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for McpGetPromptExecutor {
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
            NodeData::McpGetPrompt(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 McpGetPromptConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换 connection_id 和 prompt_name
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        let prompt_name = replace_variables(&config.prompt_name, context).map_err(|e| {
            let error_msg = format!("提示词名称变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 2.5 变量替换 arguments 值
        let mut resolved_arguments = HashMap::new();
        for (key, value) in &config.arguments {
            let resolved_value = replace_variables(value, context).map_err(|e| {
                let error_msg = format!("参数 '{}' 变量替换失败: {}", key, e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?;
            resolved_arguments.insert(key.clone(), resolved_value);
        }

        // 3. 日志：开始获取
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mcp_get_prompt, 连接ID: {}, 提示词: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            prompt_name
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "MCP 获取提示词节点 {} 开始执行 (提示词: {})",
                    node.name, prompt_name
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

        // 4. 获取提示词
        let prompt_result = self
            .mcp_manager
            .get_prompt(&connection_id, &prompt_name, resolved_arguments.clone())
            .await;

        match prompt_result {
            Ok(result) => {
                log::info!(
                    "[MCP] 提示词获取成功: connection_id={}, prompt={}, 消息数={}",
                    connection_id,
                    prompt_name,
                    result.messages.len()
                );

                // 5. 构建输出数据
                let duration = start_time.elapsed();

                // 发送调试事件：McpPromptGet
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::McpPromptGet {
                        node_uuid: node.uuid.clone(),
                        connection_id: connection_id.clone(),
                        prompt_name: prompt_name.clone(),
                        duration_ms: duration.as_millis() as u64,
                    });
                }

                let output_data = json!({
                    "success": true,
                    "prompt_name": prompt_name,
                    "result": {
                        "description": result.description,
                        "messages": result.messages,
                    },
                    "request": {
                        "connection_id": connection_id,
                        "prompt_name": prompt_name,
                        "arguments": resolved_arguments,
                    },
                    "timing": {
                        "call_ms": result.timing_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 6. 日志：获取完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "MCP 提示词获取成功: {} ({}ms)",
                            prompt_name,
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

                // 7. 成功走 true 端口
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
                    "[MCP] 提示词获取失败: connection_id={}, prompt={}, error={}",
                    connection_id,
                    prompt_name,
                    e
                );

                // 日志：获取失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MCP 提示词获取失败: {}", e),
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
                    "prompt_name": prompt_name,
                    "result": null,
                    "request": {
                        "connection_id": connection_id,
                        "prompt_name": prompt_name,
                        "arguments": resolved_arguments,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(format!("Flow aborted: {}", e));
                }

                // 失败走 false 端口
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
    use flow_engine_core::node_config::McpGetPromptConfig;
    use crate::mcp_connection_manager::McpConnectionManager;

    /// 创建测试节点
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        prompt_name: &str,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mcp_get_prompt".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::McpGetPrompt(McpGetPromptConfig {
                connection_id: connection_id.to_string(),
                prompt_name: prompt_name.to_string(),
                arguments: HashMap::new(),
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
            imap_manager: None, redis_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_get_prompt_nonexistent_connection() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpGetPromptExecutor::new(mcp_manager);
        let node = create_test_node("test-1", "测试获取", "nonexistent-conn", "greet");
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
    async fn test_get_prompt_with_variable_replacement() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpGetPromptExecutor::new(mcp_manager);

        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}", "{{prompt}}");

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("mcp-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "prompt".to_string(),
            json!("greet"),
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
        assert_eq!(data["prompt_name"], "greet");
    }

    #[tokio::test]
    async fn test_get_prompt_wrong_node_data_type() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpGetPromptExecutor::new(mcp_manager);

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
        assert!(result.unwrap_err().contains("McpGetPromptConfig"));
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

    /// 模拟 MCP Provider（get_prompt 成功）
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
            Err("not implemented".to_string())
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
            Ok(flow_engine_core::traits::McpPromptResult {
                description: Some("测试提示词".to_string()),
                messages: vec![serde_json::json!({"role": "user", "content": "hello"})],
                timing_ms: 20,
            })
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
            imap_manager: None, redis_manager: None,
            last_node_output: None,
            debug_event_sender: Some(Arc::new(MpscDebugEventSender::new(event_sender))),
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        };
        (context, event_receiver)
    }

    /// 测试：获取失败时不发射 McpPromptGet 调试事件
    #[tokio::test]
    async fn test_debug_event_not_emitted_on_get_prompt_failure() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpGetPromptExecutor::new(mcp_manager);
        let node = create_test_node("test-fail", "失败测试", "nonexistent", "greet");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let events: Vec<_> = (&mut event_receiver).try_recv().into_iter().collect();
        assert!(
            events.is_empty(),
            "获取失败时不应发射调试事件，但收到: {:?}",
            events
        );
    }

    /// 测试：获取成功时发射 McpPromptGet 调试事件
    #[tokio::test]
    async fn test_debug_event_mcp_prompt_get() {
        let mcp_manager = Arc::new(MockMcpProvider);
        let executor = McpGetPromptExecutor::new(mcp_manager);
        let node = create_test_node("test-ok", "成功测试", "mock-conn-1", "greet");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        let event = event_receiver.try_recv().expect("应收到调试事件");
        match &event {
            DebugEvent::McpPromptGet {
                node_uuid,
                connection_id,
                prompt_name,
                duration_ms,
            } => {
                assert_eq!(node_uuid, "test-ok");
                assert_eq!(connection_id, "mock-conn-1");
                assert_eq!(prompt_name, "greet");
                assert!(*duration_ms < 10000, "duration_ms 应小于 10 秒");
            }
            _ => panic!("期望 McpPromptGet 事件，收到: {:?}", event),
        }

        assert!(
            event_receiver.try_recv().is_err(),
            "不应有额外的调试事件"
        );
    }
}
