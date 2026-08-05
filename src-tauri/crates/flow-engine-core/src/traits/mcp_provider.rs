//! MCP 提供者抽象 trait

use crate::node_config::{McpConnectConfig, McpTransportMode};
use async_trait::async_trait;

/// MCP 传输模式默认值辅助函数
fn default_transport_mode() -> McpTransportMode {
    McpTransportMode::default()
}

/// MCP 工具信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpToolInfo {
    /// 工具名称
    pub name: String,
    /// 工具描述
    #[serde(default)]
    pub description: Option<String>,
    /// 工具输入 schema (JSON Schema)
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
}

/// MCP 资源信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpResourceInfo {
    /// 资源 URI
    pub uri: String,
    /// 资源名称
    pub name: String,
    /// 资源描述
    #[serde(default)]
    pub description: Option<String>,
    /// MIME 类型
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// MCP 提示词参数信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpPromptArgumentInfo {
    /// 参数名称
    pub name: String,
    /// 参数描述
    #[serde(default)]
    pub description: Option<String>,
    /// 是否必填
    #[serde(default)]
    pub required: Option<bool>,
}

/// MCP 提示词信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpPromptInfo {
    /// 提示词名称
    pub name: String,
    /// 提示词描述
    #[serde(default)]
    pub description: Option<String>,
    /// 提示词参数
    #[serde(default)]
    pub arguments: Vec<McpPromptArgumentInfo>,
}

/// MCP 提示词结果
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpPromptResult {
    /// 提示词描述
    #[serde(default)]
    pub description: Option<String>,
    /// 提示词消息列表
    pub messages: Vec<serde_json::Value>,
    /// 耗时（毫秒）
    pub timing_ms: u64,
}

/// MCP 服务器信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerInfo {
    /// 服务器名称
    pub name: String,
    /// 服务器版本
    #[serde(default)]
    pub version: Option<String>,
}

/// MCP 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// 连接 URL（Stdio 模式下为 command 描述，如 "npx @server/memory"）
    pub url: String,
    /// 服务器信息
    pub server_info: McpServerInfo,
    /// 可用工具列表
    pub tools: Vec<McpToolInfo>,
    /// 可用资源列表
    pub resources: Vec<McpResourceInfo>,
    /// 可用提示词列表
    pub prompts: Vec<McpPromptInfo>,
    /// 传输模式
    #[serde(default = "default_transport_mode")]
    pub transport: McpTransportMode,
}

/// MCP 工具调用结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolCallResult {
    /// 内容列表
    pub content: Vec<serde_json::Value>,
    /// 是否为Error
    pub is_error: bool,
    /// 耗时（毫秒）
    pub timing_ms: u64,
}

/// MCP 资源读取结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpResourceReadResult {
    /// 内容列表
    pub contents: Vec<serde_json::Value>,
    /// 耗时（毫秒）
    pub timing_ms: u64,
}

/// MCP 提供者 trait
///
/// 抽象 MCP 连接管理功能，使得执行器可以与具体的 MCP 实现解耦。
#[async_trait]
pub trait McpProvider: Send + Sync {
    /// 建立 MCP 连接
    async fn connect(
        &self,
        uuid: String,
        config: McpConnectConfig,
    ) -> Result<McpConnectionInfo, String>;

    /// 调用 MCP 工具
    async fn call_tool(
        &self,
        connection_id: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<McpToolCallResult, String>;

    /// 读取 MCP 资源
    async fn read_resource(
        &self,
        connection_id: &str,
        resource_uri: &str,
    ) -> Result<McpResourceReadResult, String>;

    /// 关闭 MCP 连接
    async fn close(&self, connection_id: &str) -> Result<(), String>;

    /// 获取连接信息
    fn get_connection_info(&self, connection_id: &str) -> Option<McpConnectionInfo>;

    /// 获取 MCP 提示词列表
    async fn list_prompts(&self, connection_id: &str) -> Result<Vec<McpPromptInfo>, String>;

    /// 获取 MCP 提示词
    async fn get_prompt(
        &self,
        connection_id: &str,
        prompt_name: &str,
        arguments: std::collections::HashMap<String, String>,
    ) -> Result<McpPromptResult, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_info_serialization() {
        let info = McpToolInfo {
            name: "get_weather".to_string(),
            description: Some("获取天气信息".to_string()),
            input_schema: Some(serde_json::json!({"type": "object"})),
        };

        let json = serde_json::to_string(&info).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("get_weather"));
        assert!(json.contains("获取天气信息"));

        let deserialized: McpToolInfo =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(deserialized.name, "get_weather");
        assert_eq!(deserialized.description, Some("获取天气信息".to_string()));
    }

    #[test]
    fn test_mcp_resource_info_serialization() {
        let info = McpResourceInfo {
            uri: "file:///data/config.json".to_string(),
            name: "config".to_string(),
            description: None,
            mime_type: Some("application/json".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        let deserialized: McpResourceInfo =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(deserialized.uri, "file:///data/config.json");
        assert_eq!(deserialized.mime_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_mcp_server_info_serialization() {
        let info = McpServerInfo {
            name: "test-server".to_string(),
            version: Some("1.0.0".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        let deserialized: McpServerInfo =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(deserialized.name, "test-server");
        assert_eq!(deserialized.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_mcp_connection_info_serialization() {
        let info = McpConnectionInfo {
            connection_id: "conn-1".to_string(),
            url: "http://localhost:8080/mcp".to_string(),
            server_info: McpServerInfo {
                name: "test-server".to_string(),
                version: None,
            },
            tools: vec![McpToolInfo {
                name: "tool1".to_string(),
                description: None,
                input_schema: None,
            }],
            resources: vec![],
            prompts: vec![],
            transport: McpTransportMode::StreamableHttp,
        };

        let json = serde_json::to_string(&info).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("conn-1"));

        let deserialized: McpConnectionInfo =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(deserialized.connection_id, "conn-1");
        assert_eq!(deserialized.tools.len(), 1);
        assert!(deserialized.resources.is_empty());
        assert!(deserialized.prompts.is_empty());
        assert_eq!(deserialized.transport, McpTransportMode::StreamableHttp);
    }

    #[test]
    fn test_mcp_tool_call_result_serialization() {
        let result = McpToolCallResult {
            content: vec![serde_json::json!({"text": "hello"})],
            is_error: false,
            timing_ms: 150,
        };

        let json =
            serde_json::to_string(&result).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        let deserialized: McpToolCallResult =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert!(!deserialized.is_error);
        assert_eq!(deserialized.timing_ms, 150);
        assert_eq!(deserialized.content.len(), 1);
    }

    #[test]
    fn test_mcp_resource_read_result_serialization() {
        let result = McpResourceReadResult {
            contents: vec![serde_json::json!({"data": "value"})],
            timing_ms: 80,
        };

        let json =
            serde_json::to_string(&result).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        let deserialized: McpResourceReadResult =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(deserialized.timing_ms, 80);
        assert_eq!(deserialized.contents.len(), 1);
    }
}
