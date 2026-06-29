//! MCP 客户端封装模块
//!
//! 封装 rmcp SDK，提供 MCP 客户端连接功能。
//! 支持 Streamable HTTP 和 Stdio 两种传输模式。
//! 所有 rmcp 类型保持内部可见，不通过公共 API 泄露。

use std::collections::HashMap;
use std::time::Instant;

use flow_engine_core::node_config::{McpConnectConfig, McpTransportMode};
use flow_engine_core::traits::{McpPromptArgumentInfo, McpPromptInfo, McpResourceInfo, McpToolInfo};
use log::info;

use rmcp::service::ServiceExt;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::TokioChildProcess;
use rmcp::transport::ConfigureCommandExt;

/// 创建 MCP 客户端并连接到服务器
///
/// 根据 `config.transport` 选择传输模式：
/// - `StreamableHttp`: 使用 HTTP 传输
/// - `Stdio`: 启动子进程，通过 stdin/stdout 通信
///
/// 返回 [`RunningService`](rmcp::service::RunningService) + 服务器元数据（工具列表、资源列表）。
/// 所有 rmcp 错误在边界转为 String。
pub async fn create_mcp_client(
    config: &McpConnectConfig,
) -> Result<McpClientResult, String> {
    let start = Instant::now();

    // 合并 headers 和 custom_headers
    let mut merged_headers = config.headers.clone().unwrap_or_default();
    if let Some(custom) = &config.custom_headers {
        merged_headers.extend(custom.clone());
    }

    // 根据传输模式创建 transport
    let service = match config.transport {
        McpTransportMode::StreamableHttp => {
            create_streamable_http_transport(&config.url, merged_headers).await?
        }
        McpTransportMode::Stdio => {
            create_stdio_transport(config).await?
        }
    };

    let connect_time = start.elapsed();
    let display_url = get_display_url(config);
    info!("MCP 连接成功: {} ({:?})", display_url, connect_time);

    // 获取服务器信息
    let server_name = service
        .peer_info()
        .map(|info| info.server_info.name.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let server_version = service
        .peer_info()
        .map(|info| info.server_info.version.clone());

    // 获取可用工具列表
    let tools_result = service
        .list_all_tools()
        .await
        .map_err(|e| format!("获取工具列表失败: {}", e))?;
    let tools: Vec<McpToolInfo> = tools_result
        .iter()
        .map(|tool| McpToolInfo {
            name: tool.name.to_string(),
            description: tool.description.as_ref().map(|d| d.to_string()),
            input_schema: Some(serde_json::Value::Object(
                (*tool.input_schema).clone(),
            )),
        })
        .collect();

    // 获取可用资源列表（优雅降级：服务器不支持时返回空列表）
    let resources: Vec<McpResourceInfo> = match service.list_all_resources().await {
        Ok(result) => result
            .iter()
            .map(|res| McpResourceInfo {
                uri: res.uri.to_string(),
                name: res.name.to_string(),
                description: res.description.clone(),
                mime_type: res.mime_type.clone(),
            })
            .collect(),
        Err(e) => {
            let err_str = e.to_string();
            log::warn!("[MCP] 获取资源列表失败（已降级为空列表）: {}", err_str);
            Vec::new()
        }
    };

    // 获取可用提示词列表（优雅降级：服务器不支持时返回空列表）
    let prompts: Vec<McpPromptInfo> = match service.list_all_prompts().await {
        Ok(result) => result
            .iter()
            .map(|prompt| {
                let arguments = match &prompt.arguments {
                    Some(args) => args
                        .iter()
                        .map(|arg| McpPromptArgumentInfo {
                            name: arg.name.clone(),
                            description: arg.description.clone(),
                            required: arg.required,
                        })
                        .collect(),
                    None => Vec::new(),
                };
                McpPromptInfo {
                    name: prompt.name.clone(),
                    description: prompt.description.clone(),
                    arguments,
                }
            })
            .collect(),
        Err(e) => {
            let err_str = e.to_string();
            log::warn!("[MCP] 获取提示词列表失败（已降级为空列表）: {}", err_str);
            Vec::new()
        }
    };

    Ok(McpClientResult {
        service,
        server_name,
        server_version,
        tools,
        resources,
        prompts,
        connect_time_ms: connect_time.as_millis() as u64,
    })
}

/// 创建 Streamable HTTP 传输
async fn create_streamable_http_transport(
    url: &str,
    headers: HashMap<String, String>,
) -> Result<rmcp::service::RunningService<rmcp::service::RoleClient, ()>, String> {
    if headers.is_empty() {
        let transport = StreamableHttpClientTransport::from_uri(url.to_string());
        ().serve(transport)
            .await
            .map_err(|e| format!("MCP 连接失败: {}", e))
    } else {
        // 创建带自定义头的 reqwest 客户端
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in &headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的请求头名称 '{}': {}", key, e))?;
            let header_value = reqwest::header::HeaderValue::from_str(value)
                .map_err(|e| format!("无效的请求头值 '{}': {}", value, e))?;
            header_map.insert(header_name, header_value);
        }

        let client = reqwest::Client::builder()
            .default_headers(header_map)
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let config = StreamableHttpClientTransportConfig::with_uri(url.to_string());
        let transport = StreamableHttpClientTransport::with_client(client, config);
        ().serve(transport)
            .await
            .map_err(|e| format!("MCP 连接失败: {}", e))
    }
}

/// 创建 Stdio 子进程传输
async fn create_stdio_transport(
    config: &McpConnectConfig,
) -> Result<rmcp::service::RunningService<rmcp::service::RoleClient, ()>, String> {
    let command = config.command.as_ref()
        .ok_or_else(|| "stdio 模式需要指定 command".to_string())?;
    let args = config.args.as_ref()
        .cloned()
        .unwrap_or_default();
    let env = config.env.as_ref()
        .cloned()
        .unwrap_or_default();

    let transport = TokioChildProcess::new(
        tokio::process::Command::new(command).configure(|cmd| {
            cmd.args(&args);
            for (k, v) in &env {
                cmd.env(k, v);
            }
        })
    ).map_err(|e| format!("启动 MCP 进程失败 '{}': {}", command, e))?;

    ().serve(transport).await
        .map_err(|e| format!("MCP stdio 连接失败: {}", e))
}

/// 获取用于日志显示的连接描述
pub fn get_display_url(config: &McpConnectConfig) -> String {
    match config.transport {
        McpTransportMode::StreamableHttp => config.url.clone(),
        McpTransportMode::Stdio => {
            let command = config.command.as_deref().unwrap_or("");
            let args = config.args.as_ref()
                .map(|a| a.join(" "))
                .unwrap_or_default();
            if args.is_empty() {
                command.to_string()
            } else {
                format!("{} {}", command, args)
            }
        }
    }
}

/// MCP 客户端连接结果
///
/// 包含 [`RunningService`](rmcp::service::RunningService)（保持连接活跃）和服务器元数据。
pub struct McpClientResult {
    /// rmcp 运行中的服务（保持连接活跃，提供工具调用/资源读取方法）
    pub service: rmcp::service::RunningService<rmcp::service::RoleClient, ()>,
    /// 服务器名称
    pub server_name: String,
    /// 服务器版本
    pub server_version: Option<String>,
    /// 可用工具列表
    pub tools: Vec<McpToolInfo>,
    /// 可用资源列表
    pub resources: Vec<McpResourceInfo>,
    /// 可用提示词列表
    pub prompts: Vec<McpPromptInfo>,
    /// 连接耗时（毫秒）
    pub connect_time_ms: u64,
}
