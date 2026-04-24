//! MCP 客户端封装模块
//!
//! 封装 rmcp SDK，提供 MCP 客户端连接功能。
//! 所有 rmcp 类型保持内部可见，不通过公共 API 泄露。

use std::collections::HashMap;
use std::time::Instant;

use flow_engine_core::traits::{McpResourceInfo, McpToolInfo};
use log::info;

use rmcp::service::ServiceExt;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;

/// 创建 MCP 客户端并连接到服务器
///
/// 使用 rmcp 的 `StreamableHttpClientTransport` 和 `ServiceExt::serve()` 完成连接+初始化。
/// 返回 [`RunningService`](rmcp::service::RunningService) + 服务器元数据（工具列表、资源列表）。
/// 所有 rmcp 错误在边界转为 String。
pub async fn create_mcp_client(
    url: &str,
    headers: HashMap<String, String>,
) -> Result<McpClientResult, String> {
    let start = Instant::now();

    // 创建 transport（带自定义头或默认）
    let service = if headers.is_empty() {
        let transport = StreamableHttpClientTransport::from_uri(url.to_string());
        ().serve(transport)
            .await
            .map_err(|e| format!("MCP 连接失败: {}", e))?
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
            .map_err(|e| format!("MCP 连接失败: {}", e))?
    };

    let connect_time = start.elapsed();
    info!("MCP 连接成功: {} ({:?})", url, connect_time);

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

    // 获取可用资源列表
    let resources_result = service
        .list_all_resources()
        .await
        .map_err(|e| format!("获取资源列表失败: {}", e))?;
    let resources: Vec<McpResourceInfo> = resources_result
        .iter()
        .map(|res| McpResourceInfo {
            uri: res.uri.to_string(),
            name: res.name.to_string(),
            description: res.description.clone(),
            mime_type: res.mime_type.clone(),
        })
        .collect();

    Ok(McpClientResult {
        service,
        server_name,
        server_version,
        tools,
        resources,
        connect_time_ms: connect_time.as_millis() as u64,
    })
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
    /// 连接耗时（毫秒）
    pub connect_time_ms: u64,
}
