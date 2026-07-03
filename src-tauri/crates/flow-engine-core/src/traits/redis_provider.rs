//! Redis 提供者抽象 trait

use async_trait::async_trait;

/// Redis 连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedisConnectOutput {
    /// 是否Success
    pub success: bool,
    /// 连接 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// 服务器信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_info: Option<String>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Redis 命令输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedisCommandOutput {
    /// 是否Success
    pub success: bool,
    /// 执行的命令
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// 结果行数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_count: Option<u64>,
    /// 结果数据（JSON 行数组）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// 列名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
    /// 受影响行数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_rows: Option<u64>,
    /// 是否被截断（超出限制）
    #[serde(default)]
    pub truncated: bool,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Redis 关闭连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedisCloseOutput {
    /// 是否Success
    pub success: bool,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Redis 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedisConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// 服务器地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 数据库索引
    pub db_index: i64,
}

/// Redis 提供者 trait
///
/// 抽象 Redis 连接管理功能，使得执行器可以与具体的 Redis 实现解耦。
#[async_trait]
pub trait RedisProvider: Send + Sync {
    /// 建立 Redis 连接
    async fn connect(
        &self,
        connection_id: &str,
        host: &str,
        port: u16,
        password: Option<&str>,
        db_index: i64,
        connect_timeout_ms: u64,
    ) -> Result<RedisConnectOutput, String>;

    /// 执行 Redis 命令
    async fn command(
        &self,
        connection_id: &str,
        command: &str,
        args: &[String],
    ) -> Result<RedisCommandOutput, String>;

    /// 关闭 Redis 连接
    async fn close(&self, connection_id: &str) -> Result<RedisCloseOutput, String>;

    /// 获取连接状态
    fn get_status(&self, connection_id: &str) -> Option<String>;

    /// 列出所有连接
    fn list_connections(&self) -> Vec<RedisConnectionInfo>;
}
