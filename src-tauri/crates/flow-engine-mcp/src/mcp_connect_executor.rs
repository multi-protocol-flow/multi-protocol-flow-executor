// MCP 连接节点执行器
//
// 实现 MCP 连接建立、变量替换和服务发现功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::McpProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// MCP 连接节点执行器
pub struct McpConnectExecutor {
    mcp_manager: Arc<dyn McpProvider>,
}

impl McpConnectExecutor {
    /// 创建新的 MCP 连接执行器
    pub fn new(mcp_manager: Arc<dyn McpProvider>) -> Self {
        Self { mcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for McpConnectExecutor {
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
            NodeData::McpConnect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not McpConnectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（URL 和 Stdio 的 command/args 都需要替换）
        let url = replace_variables(&config.url, context).map_err(|e| {
            let error_msg = format!("URL variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // Stdio 模式下替换 command 和 args 中的变量
        let command = match &config.command {
            Some(cmd) => Some(replace_variables(cmd, context).map_err(|e| {
                ErrorDetail::execution_error(format!("command variable substitution failed: {}", e))
            })?),
            None => None,
        };
        let args = match &config.args {
            Some(arg_list) => {
                let mut replaced = Vec::new();
                for arg in arg_list {
                    replaced.push(replace_variables(arg, context).map_err(|e| {
                        ErrorDetail::execution_error(format!(
                            "args variable substitution failed: {}",
                            e
                        ))
                    })?);
                }
                Some(replaced)
            }
            None => None,
        };

        // 3. 日志：开始连接
        let display_url = match &command {
            Some(cmd) => {
                let args_display = args.as_ref().map(|a| a.join(" ")).unwrap_or_default();
                if args_display.is_empty() {
                    cmd.clone()
                } else {
                    format!("{} {}", cmd, args_display)
                }
            }
            None => url.clone(),
        };

        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: mcp_connect, URL: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            display_url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("MCP connect node {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 变量替换（headers / custom_headers / env 的键和值都需替换）
        let headers = match &config.headers {
            Some(h) => {
                let mut replaced = HashMap::new();
                for (key, value) in h {
                    let replaced_key = replace_variables(key, context).map_err(|e| {
                        ErrorDetail::execution_error(format!(
                            "headers key variable substitution failed: {}",
                            e
                        ))
                    })?;
                    let replaced_value = replace_variables(value, context).map_err(|e| {
                        ErrorDetail::execution_error(format!(
                            "headers value variable substitution failed: {}",
                            e
                        ))
                    })?;
                    replaced.insert(replaced_key, replaced_value);
                }
                Some(replaced)
            }
            None => None,
        };

        let custom_headers = match &config.custom_headers {
            Some(h) => {
                let mut replaced = HashMap::new();
                for (key, value) in h {
                    let replaced_key = replace_variables(key, context).map_err(|e| {
                        ErrorDetail::execution_error(format!(
                            "custom_headers key variable substitution failed: {}",
                            e
                        ))
                    })?;
                    let replaced_value = replace_variables(value, context).map_err(|e| {
                        ErrorDetail::execution_error(format!(
                            "custom_headers value variable substitution failed: {}",
                            e
                        ))
                    })?;
                    replaced.insert(replaced_key, replaced_value);
                }
                Some(replaced)
            }
            None => None,
        };

        let env = match &config.env {
            Some(e) => {
                let mut replaced = HashMap::new();
                for (key, value) in e {
                    let replaced_key = replace_variables(key, context).map_err(|e| {
                        ErrorDetail::execution_error(format!(
                            "env key variable substitution failed: {}",
                            e
                        ))
                    })?;
                    let replaced_value = replace_variables(value, context).map_err(|e| {
                        ErrorDetail::execution_error(format!(
                            "env value variable substitution failed: {}",
                            e
                        ))
                    })?;
                    replaced.insert(replaced_key, replaced_value);
                }
                Some(replaced)
            }
            None => None,
        };

        // 5. 构建连接配置（使用替换后的值）
        let connect_config = flow_engine_core::node_config::McpConnectConfig {
            url,
            headers,
            protocol_version: config.protocol_version.clone(),
            custom_headers,
            discovered_tools: Vec::new(),
            discovered_resources: Vec::new(),
            discovered_prompts: Vec::new(),
            transport: config.transport.clone(),
            command,
            args,
            env,
        };

        // 6. 建立连接
        let connect_result = self
            .mcp_manager
            .connect(node.uuid.clone(), connect_config)
            .await;

        match connect_result {
            Ok(conn_info) => {
                log::info!(
                    "[MCP] Connection succeeded: uuid={}, url={}, tool_count={}, resource_count={}",
                    node.uuid,
                    conn_info.url,
                    conn_info.tools.len(),
                    conn_info.resources.len()
                );

                // 发送调试事件：McpConnectionCreated
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::McpConnectionCreated {
                        node_uuid: node.uuid.clone(),
                        connection_id: conn_info.connection_id.clone(),
                        url: conn_info.url.clone(),
                        server_name: conn_info.server_info.name.clone(),
                    });
                }

                // 6. 构建输出数据
                let duration = start_time.elapsed();
                let output_data = json!({
                    "success": true,
                    "connection_id": conn_info.connection_id,
                    "url": conn_info.url,
                    "server_info": {
                        "name": conn_info.server_info.name,
                        "version": conn_info.server_info.version,
                    },
                    "tools": conn_info.tools.iter().map(|t| json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                    })).collect::<Vec<_>>(),
                    "resources": conn_info.resources.iter().map(|r| json!({
                        "uri": r.uri,
                        "name": r.name,
                        "description": r.description,
                        "mime_type": r.mime_type,
                    })).collect::<Vec<_>>(),
                    "prompts": conn_info.prompts.iter().map(|p| json!({
                        "name": p.name,
                        "description": p.description,
                        "arguments": p.arguments.iter().map(|a| json!({
                            "name": a.name,
                            "description": a.description,
                            "required": a.required,
                        })).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                    "request": {
                        "url": conn_info.url,
                        "protocol_version": config.protocol_version,
                        "transport": match config.transport {
                            flow_engine_core::node_config::McpTransportMode::StreamableHttp => "streamable_http",
                            flow_engine_core::node_config::McpTransportMode::Stdio => "stdio",
                        },
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 7. 日志：连接完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "MCP connect succeeded, discovered {} tools, {} resources, {} prompts ({}ms)",
                            conn_info.tools.len(),
                            conn_info.resources.len(),
                            conn_info.prompts.len(),
                            duration.as_millis()
                        ),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. Success时走 true 端口
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
                    "[MCP] connection failed: uuid={}, url={}, error={}",
                    node.uuid,
                    display_url,
                    e
                );

                // 日志：connection failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MCP connection failed: {}", e),
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
                    "connection_id": serde_json::Value::Null,
                    "url": display_url,
                    "request": {
                        "url": display_url,
                        "protocol_version": config.protocol_version,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
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
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                // 返回 false 端口
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
    use crate::mcp_connection_manager::McpConnectionManager;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::McpConnectConfig;
    use flow_engine_core::node_config::McpTransportMode;
    use flow_engine_core::traits::{
        McpConnectionInfo, McpPromptInfo, McpResourceInfo, McpServerInfo, McpToolInfo,
    };

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mcp_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::McpConnect(McpConnectConfig {
                url: url.to_string(),
                headers: None,
                protocol_version: "2025-11-25".to_string(),
                custom_headers: None,
                discovered_tools: Vec::new(),
                discovered_resources: Vec::new(),
                discovered_prompts: Vec::new(),
                transport: McpTransportMode::StreamableHttp,
                command: None,
                args: None,
                env: None,
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
            current_node_plugin_data: None,
        }
    }

    fn init_rustls() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    #[tokio::test]
    async fn test_connect_unreachable_url() {
        init_rustls();
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpConnectExecutor::new(mcp_manager);
        let node = create_test_node("test-1", "测试连接", "http://192.0.2.1:9999/mcp");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接不可达）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_connect_with_variable_replacement() {
        init_rustls();
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpConnectExecutor::new(mcp_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "http://{{host}}:9999/mcp");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("host".to_string(), json!("localhost"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(data["url"], "http://localhost:9999/mcp");
    }

    #[tokio::test]
    async fn test_connect_wrong_node_data_type() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpConnectExecutor::new(mcp_manager);

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
        assert!(result.unwrap_err().message.contains("McpConnectConfig"));
    }

    // ===== 调试事件测试 =====

    /// mpsc adapter for `DebugEventSender` (测试用)
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

    /// 模拟 MCP Provider（连接Success）
    struct MockMcpProvider {
        connection_info: McpConnectionInfo,
    }

    impl MockMcpProvider {
        fn new() -> Self {
            Self {
                connection_info: McpConnectionInfo {
                    connection_id: "mock-conn-1".to_string(),
                    url: "http://mock-server/mcp".to_string(),
                    server_info: McpServerInfo {
                        name: "mock-server".to_string(),
                        version: Some("1.0".to_string()),
                    },
                    tools: vec![McpToolInfo {
                        name: "test_tool".to_string(),
                        description: Some("测试工具".to_string()),
                        input_schema: None,
                    }],
                    resources: vec![McpResourceInfo {
                        uri: "resource://test".to_string(),
                        name: "test_resource".to_string(),
                        description: None,
                        mime_type: None,
                    }],
                    prompts: vec![McpPromptInfo {
                        name: "test_prompt".to_string(),
                        description: None,
                        arguments: vec![],
                    }],
                    transport: McpTransportMode::StreamableHttp,
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl flow_engine_core::traits::McpProvider for MockMcpProvider {
        async fn connect(
            &self,
            _uuid: String,
            _config: flow_engine_core::node_config::McpConnectConfig,
        ) -> Result<McpConnectionInfo, String> {
            Ok(self.connection_info.clone())
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
        fn get_connection_info(&self, _connection_id: &str) -> Option<McpConnectionInfo> {
            Some(self.connection_info.clone())
        }
        async fn list_prompts(&self, _connection_id: &str) -> Result<Vec<McpPromptInfo>, String> {
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
            current_node_plugin_data: None,
        };
        (context, event_receiver)
    }

    /// 测试：connection failed时不发射 `McpConnectionCreated` 调试事件
    #[tokio::test]
    async fn test_debug_event_not_emitted_on_connect_failure() {
        init_rustls();
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpConnectExecutor::new(mcp_manager);
        let node = create_test_node("test-fail", "Failed测试", "http://192.0.2.1:9999/mcp");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        // connection failed，不应发射调试事件
        let events: Vec<_> = event_receiver.try_recv().into_iter().collect();
        assert!(
            events.is_empty(),
            "connection failed时不应发射调试事件，但收到: {:?}",
            events
        );
    }

    /// 测试：连接Success时发射 `McpConnectionCreated` 调试事件
    #[tokio::test]
    async fn test_debug_event_mcp_connection_created() {
        let mcp_manager = Arc::new(MockMcpProvider::new());
        let executor = McpConnectExecutor::new(mcp_manager);
        let node = create_test_node("test-ok", "Success测试", "http://mock-server/mcp");
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        // 验证收到 McpConnectionCreated 调试事件
        let event = event_receiver.try_recv().expect("应收到调试事件");
        match &event {
            DebugEvent::McpConnectionCreated {
                node_uuid,
                connection_id,
                url,
                server_name,
            } => {
                assert_eq!(node_uuid, "test-ok");
                assert_eq!(connection_id, "mock-conn-1");
                assert_eq!(url, "http://mock-server/mcp");
                assert_eq!(server_name, "mock-server");
            }
            _ => panic!("期望 McpConnectionCreated 事件，收到: {:?}", event),
        }

        // 不应有更多事件
        assert!(event_receiver.try_recv().is_err(), "不应有额外的调试事件");
    }
}
