//! WebSocket 提供者抽象 trait

use async_trait::async_trait;
use crate::node_config::{FormField, WsCollectCondition};

/// WebSocket 提供者 trait
///
/// 抽象 WebSocket 连接管理功能，使得执行器可以与具体的 WebSocket 实现解耦。
#[async_trait]
pub trait WsProvider: Send + Sync {
    /// 建立 WebSocket 连接
    ///
    /// # 参数
    ///
    /// * `use_tls` - 是否使用 TLS 加密
    /// * `tls_skip_verify` - 是否跳过 TLS 证书验证
    async fn connect(
        &self,
        uuid: String,
        name: String,
        url: String,
        headers: Vec<FormField>,
        subprotocols: Vec<String>,
        connect_timeout_ms: u64,
        use_tls: bool,
        tls_skip_verify: bool,
        ping_interval_ms: Option<u64>,
        ping_timeout_ms: Option<u64>,
    ) -> Result<(), String>;

    /// 发送消息
    async fn send(
        &self,
        uuid: &str,
        message: String,
        is_binary: bool,
        send_timeout_ms: u64,
    ) -> Result<(), String>;

    /// 接收消息
    async fn receive(
        &self,
        uuid: &str,
        receive_timeout_ms: u64,
    ) -> Result<Option<String>, String>;

    /// 关闭连接
    async fn close(
        &self,
        uuid: &str,
        close_code: u16,
        close_reason: Option<String>,
        close_timeout_ms: u64,
    ) -> Result<(), String>;

    /// 清空连接中待处理的消息
    async fn drain_pending_messages(&self, uuid: &str) -> Result<usize, String>;

    /// 匹配 WebSocket 消息
    fn match_message(&self, message: &str, condition: &WsCollectCondition) -> Result<bool, String>;

    /// 获取连接状态
    ///
    /// 返回连接状态的字符串表示（如 "Connected"、"Closed" 等），如果连接不存在则返回 None
    fn get_status(&self, uuid: &str) -> Option<String>;
}
