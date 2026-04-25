// MCP 连接节点执行器
//
// 实现 MCP 连接建立、变量替换和服务发现功能。

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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::McpConnect(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 McpConnectConfig: {}", node.uuid)),
        };

        // 2. 变量替换（URL 和 Stdio 的 command/args 都需要替换）
        let url = replace_variables(&config.url, context).map_err(|e| {
            let error_msg = format!("URL 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // Stdio 模式下替换 command 和 args 中的变量
        let command = match &config.command {
            Some(cmd) => Some(replace_variables(cmd, context).map_err(|e| {
                format!("command 变量替换失败: {}", e)
            })?),
            None => None,
        };
        let args = match &config.args {
            Some(arg_list) => {
                let mut replaced = Vec::new();
                for arg in arg_list {
                    replaced.push(replace_variables(arg, context).map_err(|e| {
                        format!("args 变量替换失败: {}", e)
                    })?);
                }
                Some(replaced)
            }
            None => None,
        };

        // 3. 日志：开始连接
        let display_url = match &command {
            Some(cmd) => {
                let args_display = args.as_ref()
                    .map(|a| a.join(" "))
                    .unwrap_or_default();
                if args_display.is_empty() {
                    cmd.clone()
                } else {
                    format!("{} {}", cmd, args_display)
                }
            }
            None => url.clone(),
        };

        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mcp_connect, URL: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            display_url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("MCP 连接节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建连接配置（使用替换后的值）
        let connect_config = flow_engine_core::node_config::McpConnectConfig {
            url,
            headers: config.headers.clone(),
            protocol_version: config.protocol_version.clone(),
            custom_headers: config.custom_headers.clone(),
            discovered_tools: Vec::new(),
            discovered_resources: Vec::new(),
            discovered_prompts: Vec::new(),
            transport: config.transport.clone(),
            command,
            args,
            env: config.env.clone(),
        };

        // 5. 建立连接
        let connect_result = self
            .mcp_manager
            .connect(node.uuid.clone(), connect_config)
            .await;

        match connect_result {
            Ok(conn_info) => {
                log::info!(
                    "[MCP] 连接成功: uuid={}, url={}, 工具数={}, 资源数={}",
                    node.uuid,
                    conn_info.url,
                    conn_info.tools.len(),
                    conn_info.resources.len()
                );

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
                            "MCP 连接成功，发现 {} 个工具，{} 个资源，{} 个提示词 ({}ms)",
                            conn_info.tools.len(),
                            conn_info.resources.len(),
                            conn_info.prompts.len(),
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

                // 8. 成功时走 true 端口
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
                    "[MCP] 连接失败: uuid={}, url={}, error={}",
                    node.uuid,
                    display_url,
                    e
                );

                // 日志：连接失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MCP 连接失败: {}", e),
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
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::McpConnectConfig;
    use flow_engine_core::node_config::McpTransportMode;
    use crate::mcp_connection_manager::McpConnectionManager;

    /// 创建测试节点
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
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_connect_unreachable_url() {
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
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpConnectExecutor::new(mcp_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "http://{{host}}:9999/mcp");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("localhost"),
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
        assert_eq!(data["url"], "http://localhost:9999/mcp");
    }

    #[tokio::test]
    async fn test_connect_wrong_node_data_type() {
        let mcp_manager = Arc::new(McpConnectionManager::new());
        let executor = McpConnectExecutor::new(mcp_manager);

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
        assert!(result.unwrap_err().contains("McpConnectConfig"));
    }
}
