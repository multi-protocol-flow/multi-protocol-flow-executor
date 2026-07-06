// GraphQL Provider trait - GraphQL 连接管理抽象接口

use crate::node_config::{
    GraphqlConnectConfig, GraphqlDisconnectConfig, GraphqlIntrospectConfig, GraphqlQueryConfig,
    GraphqlSubscribeConfig,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// GraphQL 连接输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphqlConnectOutput {
    /// 是否Success
    pub success: bool,
    /// 连接 ID
    pub connection_id: String,
    /// 服务器信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<String>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GraphQL 请求耗时信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphqlTimingInfo {
    /// 总耗时（毫秒）
    pub total_ms: u64,
    /// DNS 解析耗时
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_ms: Option<u64>,
    /// 连接耗时
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_ms: Option<u64>,
    /// TLS 握手耗时
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_ms: Option<u64>,
    /// 服务器处理耗时
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_ms: Option<u64>,
}

/// GraphQL 查询输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphqlQueryOutput {
    /// 是否Success
    pub success: bool,
    /// 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// GraphQL Error列表
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
    /// 响应扩展信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
    /// HTTP 状态码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// 请求耗时信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<GraphqlTimingInfo>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GraphQL 订阅事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphqlSubscriptionEvent {
    /// 事件数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// GraphQL Error列表
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
    /// 响应扩展信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
    /// 接收时间（毫秒时间戳）
    pub received_at: u64,
}

/// GraphQL 订阅输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphqlSubscribeOutput {
    /// 是否Success
    pub success: bool,
    /// 接收到的事件列表
    pub events: Vec<GraphqlSubscriptionEvent>,
    /// 事件数量
    pub event_count: u32,
    /// 订阅持续时间（毫秒）
    pub duration_ms: u64,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GraphQL 查询模板
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphqlQueryTemplate {
    /// 类型名称
    pub type_name: String,
    /// 操作类型（query/mutation/subscription）
    pub operation: String,
    /// 查询模板
    pub template: String,
}

/// GraphQL Schema 探索输出
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphqlIntrospectOutput {
    /// 是否Success
    pub success: bool,
    /// 完整 Schema（JSON）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    /// 生成的查询模板
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_templates: Option<Vec<GraphqlQueryTemplate>>,
    /// Error信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GraphQL 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// GraphQL 端点 URL
    pub endpoint: String,
    /// 连接状态
    pub status: String,
}

/// GraphQL 连接管理器 trait
///
/// 抽象 GraphQL 连接的生命周期管理（连接、查询、订阅、探索、断开），
/// 使得业务逻辑可以在无具体实现依赖的环境下运行。
#[async_trait]
pub trait GraphqlProvider: Send + Sync {
    /// 建立 GraphQL 连接
    async fn connect(
        &self,
        uuid: &str,
        config: &GraphqlConnectConfig,
    ) -> Result<GraphqlConnectOutput, String>;

    /// 执行 GraphQL 查询/变更
    async fn query(
        &self,
        uuid: &str,
        config: &GraphqlQueryConfig,
    ) -> Result<GraphqlQueryOutput, String>;

    /// 建立 GraphQL 订阅
    async fn subscribe(
        &self,
        uuid: &str,
        config: &GraphqlSubscribeConfig,
    ) -> Result<GraphqlSubscribeOutput, String>;

    /// 执行 GraphQL Schema 探索
    async fn introspect(
        &self,
        uuid: &str,
        config: &GraphqlIntrospectConfig,
    ) -> Result<GraphqlIntrospectOutput, String>;

    /// 断开 GraphQL 连接
    async fn disconnect(&self, uuid: &str, config: &GraphqlDisconnectConfig) -> Result<(), String>;

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String>;

    /// 列出所有连接
    fn list_connections(&self) -> Vec<GraphqlConnectionInfo>;
}
