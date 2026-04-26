// SSE Provider trait - SSE 连接管理抽象接口

use crate::node_config::{SseConnectConfig, SseListenConfig, SseDisconnectConfig};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// SSE 连接输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SseConnectOutput {
    /// 是否成功
    pub success: bool,
    /// 连接 ID
    pub connection_id: String,
    /// 服务器响应头
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_headers: Option<std::collections::HashMap<String, String>>,
    /// Last-Event-ID（断线续传）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_id: Option<String>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// SSE 事件条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SseEventItem {
    /// 事件类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// 事件 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 事件数据
    pub data: String,
    /// JSON 解码后的数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_decoded: Option<serde_json::Value>,
    /// 接收时间（毫秒时间戳）
    pub received_at: u64,
}

/// SSE 监听输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SseListenOutput {
    /// 是否成功
    pub success: bool,
    /// 接收到的事件列表
    pub events: Vec<SseEventItem>,
    /// 事件数量
    pub event_count: u32,
    /// 监听持续时间（毫秒）
    pub duration_ms: u64,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// SSE 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// SSE 端点 URL
    pub url: String,
    /// 连接状态
    pub status: String,
}

/// SSE 连接管理器 trait
///
/// 抽象 SSE 连接的生命周期管理（连接、监听、断开），
/// 使得业务逻辑可以在无具体实现依赖的环境下运行。
#[async_trait]
pub trait SseProvider: Send + Sync {
    /// 建立 SSE 连接
    async fn connect(
        &self,
        uuid: &str,
        config: &SseConnectConfig,
    ) -> Result<SseConnectOutput, String>;

    /// 监听 SSE 事件
    async fn listen(
        &self,
        uuid: &str,
        config: &SseListenConfig,
    ) -> Result<SseListenOutput, String>;

    /// 断开 SSE 连接
    async fn disconnect(
        &self,
        uuid: &str,
        config: &SseDisconnectConfig,
    ) -> Result<(), String>;

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String>;

    /// 列出所有连接
    fn list_connections(&self) -> Vec<SseConnectionInfo>;
}
