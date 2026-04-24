//! MCP 连接管理器
//!
//! 实现 [`McpProvider`] trait，管理 MCP 连接的生命周期。
//! 使用 rmcp [`RunningService`](rmcp::service::RunningService) 进行 MCP 通信，
//! 使用 [`tokio::sync::RwLock`] 保护连接池。

use std::collections::HashMap;
use std::time::Instant;

use async_trait::async_trait;
use log::{info, warn};
use tokio::sync::RwLock;

use flow_engine_core::node_config::{McpConnectConfig, McpTransportMode};
use flow_engine_core::traits::{
    McpConnectionInfo, McpProvider, McpPromptInfo, McpPromptResult, McpResourceInfo,
    McpResourceReadResult, McpServerInfo, McpToolCallResult, McpToolInfo,
};

use crate::mcp_client;

/// MCP 连接（存储 `RunningService` + 元数据）
///
/// `RunningService` 保持 MCP 连接活跃，通过它调用工具和读取资源。
pub struct McpConnection {
    /// rmcp 运行中的服务（保持连接活跃）
    pub(crate) service: rmcp::service::RunningService<rmcp::service::RoleClient, ()>,
    /// 连接 ID
    pub connection_id: String,
    /// 连接 URL 或 command 描述
    pub url: String,
    /// 服务器信息
    pub server_info: McpServerInfo,
    /// 可用工具列表（连接时获取）
    pub tools: Vec<McpToolInfo>,
    /// 可用资源列表（连接时获取）
    pub resources: Vec<McpResourceInfo>,
    /// 可用提示词列表（连接时获取）
    pub prompts: Vec<McpPromptInfo>,
    /// 连接创建时间（毫秒时间戳）
    pub created_at: u64,
    /// 传输模式
    pub transport: McpTransportMode,
}

/// MCP 连接管理器（per-flow-execution 作用域）
///
/// 管理 MCP 连接池，实现 `McpProvider` trait。
/// 线程安全，使用 `tokio::sync::RwLock` 保护连接池。
pub struct McpConnectionManager {
    /// 连接池: `connection_id` → [`McpConnection`]
    connections: RwLock<HashMap<String, McpConnection>>,
    /// 压测共享模式下 close 为 no-op
    shared_mode: bool,
}

impl McpConnectionManager {
    /// 创建新的连接管理器（普通模式）
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            shared_mode: false,
        }
    }

    /// 创建共享模式的连接管理器（压测场景）
    ///
    /// 共享模式下 close 为 no-op，避免并发虚拟用户互相关闭连接。
    pub fn new_shared() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            shared_mode: true,
        }
    }
}

impl Default for McpConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for McpConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接（非阻塞）
        if let Ok(mut connections) = self.connections.try_write() {
            for (id, _conn) in connections.drain() {
                log::info!("MCP 连接 {} 已随 Manager 释放而清理", id);
            }
        }
    }
}

#[async_trait]
impl McpProvider for McpConnectionManager {
    /// 建立 MCP 连接
    ///
    /// 调用 [`mcp_client::create_mcp_client`] 创建连接，
    /// 将连接信息存入连接池。
    async fn connect(
        &self,
        uuid: String,
        config: McpConnectConfig,
    ) -> Result<McpConnectionInfo, String> {
        let display_url = mcp_client::get_display_url(&config);
        info!("MCP 开始连接: url={}, id={}", display_url, uuid);

        // UUID 去重：如果已存在直接返回缓存的连接信息
        {
            let conns = self.connections.read().await;
            if let Some(existing) = conns.get(&uuid) {
                info!("MCP 复用已有连接: id={}", uuid);
                return Ok(McpConnectionInfo {
                    connection_id: existing.connection_id.clone(),
                    url: existing.url.clone(),
                    server_info: existing.server_info.clone(),
                    tools: existing.tools.clone(),
                    resources: existing.resources.clone(),
                    prompts: existing.prompts.clone(),
                    transport: existing.transport.clone(),
                });
            }
        }

        // 创建 MCP 客户端
        let result = mcp_client::create_mcp_client(&config).await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let server_info = McpServerInfo {
            name: result.server_name.clone(),
            version: result.server_version.clone(),
        };

        let transport = config.transport.clone();

        let connection_info = McpConnectionInfo {
            connection_id: uuid.clone(),
            url: display_url.clone(),
            server_info: server_info.clone(),
            tools: result.tools.clone(),
            resources: result.resources.clone(),
            prompts: result.prompts.clone(),
            transport: transport.clone(),
        };

        // 存入连接池
        let connection = McpConnection {
            service: result.service,
            connection_id: uuid.clone(),
            url: display_url,
            server_info,
            tools: result.tools,
            resources: result.resources,
            prompts: result.prompts,
            created_at: now,
            transport,
        };

        {
            let mut conns = self.connections.write().await;
            info!("MCP 添加连接: id={}", uuid);
            conns.insert(uuid, connection);
        }

        Ok(connection_info)
    }

    /// 调用 MCP 工具
    ///
    /// 从连接池获取连接，调用工具并记录耗时。
    async fn call_tool(
        &self,
        connection_id: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<McpToolCallResult, String> {
        let start = Instant::now();

        // 从连接池获取连接（克隆 service 的 peer 引用后释放锁）
        let peer = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("MCP 连接 '{}' 不存在", connection_id))?;
            conn.service.peer().clone()
        }; // 读锁在此释放

        // 构建调用参数（non-exhaustive struct，使用 builder 模式）
        let args_map = arguments
            .and_then(|v| v.as_object().cloned());

        let mut call_params = rmcp::model::CallToolRequestParams::new(tool_name.to_string());
        if let Some(args) = args_map {
            call_params = call_params.with_arguments(args);
        }

        // 调用工具
        let result = peer
            .call_tool(call_params)
            .await
            .map_err(|e| format!("MCP 工具调用失败 '{}': {}", tool_name, e))?;

        let timing_ms = start.elapsed().as_millis() as u64;

        // 转换结果内容
        let content: Vec<serde_json::Value> = result
            .content
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or(serde_json::Value::Null))
            .collect();

        let is_error = result.is_error.unwrap_or(false);

        Ok(McpToolCallResult {
            content,
            is_error,
            timing_ms,
        })
    }

    /// 读取 MCP 资源
    ///
    /// 从连接池获取连接，读取资源并记录耗时。
    async fn read_resource(
        &self,
        connection_id: &str,
        resource_uri: &str,
    ) -> Result<McpResourceReadResult, String> {
        let start = Instant::now();

        // 从连接池获取连接
        let peer = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("MCP 连接 '{}' 不存在", connection_id))?;
            conn.service.peer().clone()
        };

        // 读取资源
        let read_params = rmcp::model::ReadResourceRequestParams::new(resource_uri.to_string());

        let result = peer
            .read_resource(read_params)
            .await
            .map_err(|e| format!("MCP 读取资源失败 '{}': {}", resource_uri, e))?;

        let timing_ms = start.elapsed().as_millis() as u64;

        // 转换资源内容
        let contents: Vec<serde_json::Value> = result
            .contents
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or(serde_json::Value::Null))
            .collect();

        Ok(McpResourceReadResult {
            contents,
            timing_ms,
        })
    }

    /// 获取 MCP 提示词列表
    async fn list_prompts(&self, connection_id: &str) -> Result<Vec<McpPromptInfo>, String> {
        let conns = self.connections.read().await;
        let conn = conns
            .get(connection_id)
            .ok_or_else(|| format!("MCP 连接 '{}' 不存在", connection_id))?;
        Ok(conn.prompts.clone())
    }

    /// 获取 MCP 提示词
    async fn get_prompt(
        &self,
        connection_id: &str,
        prompt_name: &str,
        arguments: std::collections::HashMap<String, String>,
    ) -> Result<McpPromptResult, String> {
        let start = Instant::now();

        // 从连接池获取连接
        let peer = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("MCP 连接 '{}' 不存在", connection_id))?;
            conn.service.peer().clone()
        };

        // 构建获取提示词参数
        let mut prompt_params = rmcp::model::GetPromptRequestParams::new(prompt_name.to_string());
        if !arguments.is_empty() {
            let args_map: serde_json::Map<String, serde_json::Value> = arguments
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            prompt_params = prompt_params.with_arguments(args_map);
        }

        // 获取提示词
        let result = peer
            .get_prompt(prompt_params)
            .await
            .map_err(|e| format!("MCP 获取提示词失败 '{}': {}", prompt_name, e))?;

        let timing_ms = start.elapsed().as_millis() as u64;

        // 转换消息内容
        let messages: Vec<serde_json::Value> = result
            .messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
            .collect();

        Ok(McpPromptResult {
            description: result.description,
            messages,
            timing_ms,
        })
    }

    /// 关闭 MCP 连接
    ///
    /// 从连接池中移除连接，RunningService drop 自动关闭连接。
    /// 共享模式下为 no-op，避免并发虚拟用户互相关闭连接。
    async fn close(&self, connection_id: &str) -> Result<(), String> {
        if self.shared_mode {
            log::info!("共享模式下忽略 close 请求: {}", connection_id);
            return Ok(());
        }

        let mut conns = self.connections.write().await;
        let removed = conns.remove(connection_id);
        if removed.is_some() {
            info!("MCP 关闭连接: id={}", connection_id);
            Ok(())
        } else {
            warn!("MCP 关闭连接失败: 连接 '{}' 不存在", connection_id);
            Err(format!("MCP 连接 '{}' 不存在", connection_id))
        }
    }

    /// 获取连接信息
    ///
    /// 非阻塞读取连接池，构造 [`McpConnectionInfo`]。
    fn get_connection_info(&self, connection_id: &str) -> Option<McpConnectionInfo> {
        let conns = self.connections.try_read().ok()?;
        let conn = conns.get(connection_id)?;
        Some(McpConnectionInfo {
            connection_id: conn.connection_id.clone(),
            url: conn.url.clone(),
            server_info: conn.server_info.clone(),
            tools: conn.tools.clone(),
            resources: conn.resources.clone(),
            prompts: conn.prompts.clone(),
            transport: conn.transport.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = McpConnectionManager::new();
        // 新创建的 manager 应该没有连接
        assert!(manager.get_connection_info("non-existent").is_none());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = McpConnectionManager::default();
        assert!(manager.get_connection_info("non-existent").is_none());
    }

    #[test]
    fn test_connection_manager_new_shared() {
        let manager = McpConnectionManager::new_shared();
        assert!(manager.get_connection_info("non-existent").is_none());
    }

    #[tokio::test]
    async fn test_close_nonexistent_connection() {
        let manager = McpConnectionManager::new();
        let result = manager.close("non-existent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_shared_mode_close_is_noop() {
        let manager = McpConnectionManager::new_shared();
        // 共享模式下 close 不存在的连接也返回 Ok
        let result = manager.close("non-existent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_call_tool_nonexistent_connection() {
        let manager = McpConnectionManager::new();
        let result = manager.call_tool("non-existent", "tool", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_read_resource_nonexistent_connection() {
        let manager = McpConnectionManager::new();
        let result = manager.read_resource("non-existent", "resource://test").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_list_prompts_nonexistent_connection() {
        let manager = McpConnectionManager::new();
        let result = manager.list_prompts("non-existent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_get_prompt_nonexistent_connection() {
        let manager = McpConnectionManager::new();
        let result = manager
            .get_prompt("non-existent", "test_prompt", std::collections::HashMap::new())
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }
}
