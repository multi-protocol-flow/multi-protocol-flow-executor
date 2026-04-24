// MCP 调用工具节点执行器
//
// 实现 MCP 工具调用功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::McpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// MCP 调用工具节点执行器
pub struct McpCallToolExecutor {
    mcp_manager: Arc<dyn McpProvider>,
}

impl McpCallToolExecutor {
    /// 创建新的 MCP 调用工具执行器
    pub fn new(mcp_manager: Arc<dyn McpProvider>) -> Self {
        Self { mcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for McpCallToolExecutor {
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
            NodeData::McpCallTool(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 McpCallToolConfig: {}", node.uuid)),
        };

        // 2. 变量替换 connection_id 和 tool_name
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

        let tool_name = replace_variables(&config.tool_name, context).map_err(|e| {
            let error_msg = format!("工具名称变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 日志：开始调用
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mcp_call_tool, 连接ID: {}, 工具: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            tool_name
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("MCP 调用工具节点 {} 开始执行 (工具: {})", node.name, tool_name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 调用工具
        let call_result = self
            .mcp_manager
            .call_tool(&connection_id, &tool_name, config.arguments.clone())
            .await;

        match call_result {
            Ok(tool_result) => {
                let duration = start_time.elapsed();
                let is_error = tool_result.is_error;

                if is_error {
                    log::warn!(
                        "[MCP] 工具调用返回错误: connection_id={}, tool={}, error_content={:?}",
                        connection_id,
                        tool_name,
                        tool_result.content
                    );
                } else {
                    log::info!(
                        "[MCP] 工具调用成功: connection_id={}, tool={} ({}ms)",
                        connection_id,
                        tool_name,
                        duration.as_millis()
                    );
                }

                // 5. 构建输出数据
                let output_data = json!({
                    "success": !is_error,
                    "tool_name": tool_name,
                    "result": {
                        "content": tool_result.content,
                        "is_error": is_error,
                    },
                    "is_error": is_error,
                    "request": {
                        "connection_id": connection_id,
                        "tool_name": tool_name,
                        "arguments": config.arguments,
                    },
                    "timing": {
                        "call_ms": tool_result.timing_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 6. 日志：调用完成
                let log_level = if is_error {
                    LogLevel::Error
                } else {
                    LogLevel::Info
                };
                let log_msg = if is_error {
                    format!("MCP 工具调用返回错误: {}", tool_name)
                } else {
                    format!(
                        "MCP 工具调用成功: {} ({}ms)",
                        tool_name,
                        duration.as_millis()
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_msg,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 7. 端口路由：is_error → false 端口
                let port_id = if is_error { "false" } else { "true" };
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid
                                && conn.source_port_id == port_id
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec![port_id.to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[MCP] 工具调用失败: connection_id={}, tool={}, error={}",
                    connection_id,
                    tool_name,
                    e
                );

                // 日志：调用失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MCP 工具调用失败: {}", e),
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
                    "tool_name": tool_name,
                    "result": null,
                    "is_error": true,
                    "request": {
                        "connection_id": connection_id,
                        "tool_name": tool_name,
                        "arguments": config.arguments,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

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
    use flow_engine_core::node_config::McpCallToolConfig;
    use crate::mcp_connection_manager::McpConnectionManager;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, tool_name: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mcp_call_tool".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::McpCallTool(McpCallToolConfig {
                connection_id: connection_id.to_string(),
                tool_name: tool_name.to_string(),
                arguments: None,
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
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_call_tool_nonexistent_connection() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpCallToolExecutor::new(mcp_manager);
        let node = create_test_node("test-1", "测试调用", "nonexistent-conn", "get_weather");
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
        assert_eq!(data["is_error"], true);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_call_tool_with_variable_replacement() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpCallToolExecutor::new(mcp_manager);

        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}", "{{tool}}");

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("mcp-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "tool".to_string(),
            json!("get_weather"),
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
        assert_eq!(data["tool_name"], "get_weather");
    }

    #[tokio::test]
    async fn test_call_tool_wrong_node_data_type() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpCallToolExecutor::new(mcp_manager);

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
        assert!(result.unwrap_err().contains("McpCallToolConfig"));
    }
}
