// 节点配置模块 - 强类型节点配置定义

use crate::traits::{FramingConfig, GrpcServiceInfo, GraphqlQueryTemplate, TcpFramingMode};
use crate::variable::InitialVariable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// ============================================================================
// Entry 节点配置
// ============================================================================

/// 入口节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntryConfig {
    /// 初始变量列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_variables: Vec<InitialVariable>,
    /// 关联的 CSV 文件路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_path: Option<String>,
    /// CSV 编码（None=自动检测）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_encoding: Option<String>,
    /// 流程钩子列表（入口节点配置中可编辑）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Vec<crate::hook::FlowHook>>,
}

// ============================================================================
// Http 节点配置
// ============================================================================

/// HTTP 方法枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    /// GET 请求
    #[default]
    Get,
    /// POST 请求
    Post,
    /// PUT 请求
    Put,
    /// DELETE 请求
    Delete,
    /// PATCH 请求
    Patch,
    /// HEAD 请求
    Head,
    /// OPTIONS 请求
    Options,
}

/// HTTP 请求体类型
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum HttpBodyType {
    /// 无请求体
    #[default]
    None,
    /// 原始请求体
    Body,
    /// multipart/form-data 表单
    FormData,
    /// application/x-www-form-urlencoded 表单
    XWwwFormUrlencoded,
    /// 二进制文件
    Binary,
}

/// Content-Type 枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    /// application/json
    #[default]
    Json,
    /// text/plain
    Text,
    /// text/html
    Html,
    /// application/xml
    Xml,
    /// application/javascript
    Javascript,
}

/// 表单字段类型
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FormFieldType {
    /// 文本字段
    #[default]
    Text,
    /// 文件字段
    File,
}

/// 表单字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormField {
    /// 字段键名
    pub key: String,
    /// 字段值
    pub value: String,
    /// 字段类型
    #[serde(default, alias = "type")]
    pub field_type: FormFieldType,
    /// 文件路径（仅文件类型字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Http 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpConfig {
    /// 请求 URL
    pub url: String,
    /// HTTP 方法
    #[serde(default)]
    pub method: HttpMethod,
    /// 请求头
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// 请求体类型
    #[serde(default)]
    pub body_type: HttpBodyType,
    /// Content-Type
    #[serde(default)]
    pub content_type: ContentType,
    /// 表单字段
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_fields: Option<Vec<FormField>>,
    /// 二进制文件路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// 请求体
    pub body: Option<String>,
    /// 超时时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    // === 新增字段 ===
    // 认证配置
    /// 认证配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<HttpAuthConfig>,
    // 重定向配置
    /// 是否跟随重定向
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,
    /// 最大重定向次数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_redirects: Option<u32>,
    // TLS 配置
    /// 是否跳过 TLS 证书验证
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_skip_verify: Option<bool>,
    /// TLS CA 证书路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_ca_cert_path: Option<String>,
    /// TLS 客户端证书路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_client_cert_path: Option<String>,
    /// TLS 客户端私钥路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_client_key_path: Option<String>,
    // 压缩配置
    /// 是否接受压缩编码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_encoding: Option<bool>,
    // Cookie 配置
    /// 是否启用 Cookie
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_cookies: Option<bool>,
    /// Cookie 列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookie_jar: Option<Vec<CookieEntry>>,
    // 重试配置
    /// 重试次数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
    /// 重试延迟（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_delay_ms: Option<u64>,
    /// 触发重试的 HTTP 状态码列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_on_status: Option<Vec<u16>>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: HttpMethod::Get,
            headers: None,
            body_type: HttpBodyType::None,
            content_type: ContentType::Json,
            form_fields: None,
            file_path: None,
            body: None,
            timeout_ms: None,
            auth: None,
            follow_redirects: None,
            max_redirects: None,
            tls_skip_verify: None,
            tls_ca_cert_path: None,
            tls_client_cert_path: None,
            tls_client_key_path: None,
            accept_encoding: None,
            enable_cookies: None,
            cookie_jar: None,
            retry_count: None,
            retry_delay_ms: None,
            retry_on_status: None,
        }
    }
}

/// HTTP 认证类型
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HttpAuthType {
    /// 无认证
    #[default]
    None,
    /// HTTP Basic 认证
    Basic,
    /// Bearer Token 认证
    Bearer,
    /// API Key 放在请求头
    ApiKeyHeader,
    /// API Key 放在查询参数
    ApiKeyQuery,
    /// HTTP Digest 认证
    Digest,
}

/// HTTP 认证配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HttpAuthConfig {
    /// 认证类型
    #[serde(default)]
    pub auth_type: HttpAuthType,
    /// 用户名（Basic/Digest 认证）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 密码（Basic/Digest 认证）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Token（Bearer 认证）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// API Key 名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_name: Option<String>,
    /// API Key 值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_value: Option<String>,
}

/// Cookie 条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CookieEntry {
    /// Cookie 名称
    pub name: String,
    /// Cookie 值
    pub value: String,
    /// Cookie 域名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Cookie 路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

// ============================================================================
// Conditional 节点配置
// ============================================================================

/// 条件类型枚举
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionType {
    /// 简单条件（字段比较）
    #[default]
    Simple,
    /// 高级条件（表达式）
    Advanced,
}

/// 条件操作符枚举
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// 等于
    #[default]
    Equal,
    /// 不等于
    NotEqual,
    /// 大于
    Greater,
    /// 大于等于
    GreaterOrEqual,
    /// 小于
    Less,
    /// 小于等于
    LessOrEqual,
    /// 包含
    Contains,
    /// 以...开头
    StartsWith,
    /// 以...结尾
    EndsWith,
    /// 存在
    Exists,
}

/// 期望值类型枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExpectedValueType {
    /// 数字类型
    #[default]
    Number,
    /// 字符串类型
    String,
    /// 布尔类型
    Boolean,
    /// 空值类型
    Null,
}

/// Conditional 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalConfig {
    /// 条件类型
    #[serde(default)]
    pub condition_type: ConditionType,
    /// 字段路径（用于简单条件）
    #[serde(default)]
    pub field_path: Option<String>,
    /// 操作符
    #[serde(default)]
    pub operator: Option<ConditionOperator>,
    /// 期望值
    #[serde(default)]
    pub expected_value: Option<serde_json::Value>,
    /// 高级表达式（用于复杂条件）
    #[serde(default)]
    pub advanced_expression: Option<String>,
    /// 期望值类型选择器
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_value_type: Option<serde_json::Value>,
    /// 可选，指定读取特定上游节点的输出（UUID）。
    /// 为 None 时读取 `last_node_output`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_node_id: Option<String>,
}

// ============================================================================
// WebSocket 节点配置
// ============================================================================

/// WebSocket 采集模式
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsCollectMode {
    /// 采集第一条匹配消息
    #[default]
    FirstMatch,
    /// 采集满足条件消息
    ConditionMatch,
    /// 持续采集指定时长
    DurationCollect,
}

/// WebSocket 匹配类型
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsMatchType {
    /// `JSONPath` 匹配
    #[default]
    JsonPath,
    /// 正则表达式匹配
    Regex,
    /// 表达式匹配
    Expression,
}

/// WebSocket 消息类型
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WsMessageType {
    /// 文本消息
    #[default]
    Text,
    /// 二进制消息
    Binary,
}

/// WebSocket 采集条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WsCollectCondition {
    /// 匹配类型
    pub match_type: WsMatchType,
    /// 匹配表达式
    pub expression: String,
}

impl PartialEq for WsCollectCondition {
    fn eq(&self, other: &Self) -> bool {
        self.match_type == other.match_type && self.expression == other.expression
    }
}

/// WebSocket Connect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsConnectConfig {
    /// WebSocket URL
    pub url: String,
    /// 请求头
    #[serde(default)]
    pub headers: Vec<FormField>,
    /// 子协议
    #[serde(default)]
    pub subprotocols: Vec<String>,
    /// 连接超时（毫秒）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// 是否采集消息
    #[serde(default)]
    pub collect_message: bool,
    /// 采集模式
    #[serde(default)]
    pub collect_mode: WsCollectMode,
    /// 采集条件
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collect_condition: Option<WsCollectCondition>,
    /// 采集超时（毫秒）
    #[serde(default = "default_message_timeout")]
    pub collect_timeout_ms: u64,
    /// 是否使用 TLS
    #[serde(default)]
    pub use_tls: bool,
    /// 是否跳过 TLS 证书验证
    #[serde(default)]
    pub tls_skip_verify: bool,
    /// Ping 间隔（毫秒），None 表示不发 Ping
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ping_interval_ms: Option<u64>,
    /// Ping 超时（毫秒），None 表示不检查超时
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ping_timeout_ms: Option<u64>,
}

impl Default for WsConnectConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            headers: Vec::new(),
            subprotocols: Vec::new(),
            connect_timeout_ms: default_connect_timeout(),
            collect_message: false,
            collect_mode: WsCollectMode::default(),
            collect_condition: None,
            collect_timeout_ms: default_message_timeout(),
            use_tls: false,
            tls_skip_verify: false,
            ping_interval_ms: None,
            ping_timeout_ms: None,
        }
    }
}

/// WebSocket Send+Collect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsSendCollectConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 消息内容
    pub message: String,
    /// 消息类型
    #[serde(default)]
    pub message_type: WsMessageType,
    /// 发送超时（毫秒）
    #[serde(default = "default_send_timeout")]
    pub send_timeout_ms: u64,
    /// 是否采集响应
    #[serde(default)]
    pub collect_response: bool,
    /// 采集模式
    #[serde(default)]
    pub collect_mode: WsCollectMode,
    /// 采集条件
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collect_condition: Option<WsCollectCondition>,
    /// 采集超时（毫秒）
    #[serde(default = "default_message_timeout")]
    pub collect_timeout_ms: u64,
}

impl Default for WsSendCollectConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            message: String::new(),
            message_type: WsMessageType::default(),
            send_timeout_ms: default_send_timeout(),
            collect_response: true,
            collect_mode: WsCollectMode::default(),
            collect_condition: None,
            collect_timeout_ms: default_message_timeout(),
        }
    }
}

/// WebSocket Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsCloseConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 关闭码
    #[serde(default = "default_close_code")]
    pub close_code: u16,
    /// 关闭原因
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<String>,
    /// 关闭超时（毫秒）
    #[serde(default = "default_close_timeout")]
    pub close_timeout_ms: u64,
}

impl Default for WsCloseConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            close_code: default_close_code(),
            close_reason: None,
            close_timeout_ms: default_close_timeout(),
        }
    }
}

// Default value helpers
fn default_connect_timeout() -> u64 {
    30000
}
fn default_message_timeout() -> u64 {
    10000
}
fn default_send_timeout() -> u64 {
    10000
}
fn default_close_code() -> u16 {
    1000
}
fn default_close_timeout() -> u64 {
    5000
}

// ============================================================================
// gRPC 节点配置
// ============================================================================

/// Proto 文件描述
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProtoFile {
    /// 文件路径/名称
    pub path: String,
    /// 文件内容
    pub content: String,
}

/// gRPC Metadata 条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GrpcMetadataEntry {
    /// 键
    pub key: String,
    /// 值
    pub value: String,
}

/// gRPC 流式消息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamMessage {
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 消息内容（JSON 字符串）
    #[serde(default)]
    pub content: String,
}

/// gRPC Connect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrpcConnectConfig {
    /// gRPC 服务器地址
    pub url: String,
    /// 是否使用 TLS
    #[serde(default)]
    pub use_tls: bool,
    /// 是否跳过 TLS 证书验证
    #[serde(default)]
    pub tls_skip_verify: bool,
    /// 是否启用 Server Reflection
    #[serde(default)]
    pub enable_reflection: bool,
    /// Proto 文件列表（当不使用 Reflection 时）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proto_files: Vec<ProtoFile>,
    /// 默认 metadata
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_metadata: Vec<GrpcMetadataEntry>,
    /// 连接超时（毫秒）
    #[serde(default = "default_grpc_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// 通过 Server Reflection 或 Proto 解析发现的服务列表（设计时缓存，仅前端使用）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovered_services: Vec<GrpcServiceInfo>,
    // === TLS 增强字段 ===
    /// TLS CA 证书（PEM 格式内容）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_ca_cert: Option<String>,
    /// TLS 客户端证书（PEM 格式内容）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_client_cert: Option<String>,
    /// TLS 客户端私钥（PEM 格式内容）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_client_key: Option<String>,
    /// TLS 服务器名称覆盖
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_server_name_override: Option<String>,
    // === 压缩配置 ===
    /// 压缩编码："gzip" 或 "none"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_encoding: Option<String>,
    // === Keepalive 配置 ===
    /// Keepalive 间隔（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keepalive_time_ms: Option<u64>,
    /// Keepalive 超时（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keepalive_timeout_ms: Option<u64>,
    /// 是否允许无流时发送 Keepalive
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keepalive_permit_without_streams: Option<bool>,
    // === Reflection 增强配置 ===
    /// Reflection 请求附加 metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflection_metadata: Option<Vec<GrpcMetadataEntry>>,
    // === 健康检查配置 ===
    /// 健康检查服务名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_check_service: Option<String>,
    // === 负载均衡配置 ===
    /// 多端点列表（用于负载均衡）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<String>>,
    // === 重试配置 ===
    /// 最大重试次数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    /// 初始退避时间（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_backoff_ms: Option<u64>,
    /// 最大退避时间（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_backoff_ms: Option<u64>,
    /// 可重试的 gRPC 状态码列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retryable_status_codes: Option<Vec<String>>,
}

impl Default for GrpcConnectConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            use_tls: false,
            tls_skip_verify: false,
            enable_reflection: false,
            proto_files: Vec::new(),
            default_metadata: Vec::new(),
            connect_timeout_ms: default_grpc_connect_timeout(),
            discovered_services: Vec::new(),
            tls_ca_cert: None,
            tls_client_cert: None,
            tls_client_key: None,
            tls_server_name_override: None,
            compression_encoding: None,
            keepalive_time_ms: None,
            keepalive_timeout_ms: None,
            keepalive_permit_without_streams: None,
            reflection_metadata: None,
            health_check_service: None,
            endpoints: None,
            max_retries: None,
            initial_backoff_ms: None,
            max_backoff_ms: None,
            retryable_status_codes: None,
        }
    }
}

/// gRPC Call 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrpcCallConfig {
    /// 连接 ID（引用 `grpc_connect` 节点的 UUID）
    pub connection_id: String,
    /// 服务名称
    pub service_name: String,
    /// 方法名称
    pub method_name: String,
    /// 请求 JSON
    pub request_json: String,
    /// 调用超时（毫秒）
    #[serde(default = "default_grpc_call_timeout")]
    pub timeout_ms: u64,
    /// 调用级 metadata
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<GrpcMetadataEntry>,
    /// 流式消息列表（Client/Bidi Streaming 模式）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub request_messages: Vec<StreamMessage>,
    // === 压缩覆盖配置 ===
    /// 调用级压缩编码（覆盖连接级配置）："gzip" 或 "none"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_encoding: Option<String>,
    // === 消息大小限制 ===
    /// 最大接收消息大小（字节）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_receive_message_size: Option<usize>,
    /// 最大发送消息大小（字节）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_send_message_size: Option<usize>,
    // === 重试配置（调用级覆盖，None 时使用连接级默认值）===
    /// 最大重试次数（0 = 不重试，覆盖连接级配置）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    /// 初始退避时间（毫秒，覆盖连接级配置）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_backoff_ms: Option<u64>,
}

impl Default for GrpcCallConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            service_name: String::new(),
            method_name: String::new(),
            request_json: String::new(),
            timeout_ms: default_grpc_call_timeout(),
            metadata: Vec::new(),
            request_messages: Vec::new(),
            compression_encoding: None,
            max_receive_message_size: None,
            max_send_message_size: None,
            max_retries: None,
            initial_backoff_ms: None,
        }
    }
}

/// gRPC Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GrpcCloseConfig {
    /// 连接 ID（引用 `grpc_connect` 节点的 UUID）
    pub connection_id: String,
}

// gRPC Default value helpers
fn default_grpc_connect_timeout() -> u64 {
    30000
}
fn default_grpc_call_timeout() -> u64 {
    30000
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Assertion 节点配置
// ============================================================================

/// 断言操作符
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionOperator {
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 大于
    Gt,
    /// 大于等于
    Gte,
    /// 小于
    Lt,
    /// 小于等于
    Lte,
    /// 包含
    Contains,
    /// 不包含
    NotContains,
    /// 正则匹配
    Matches,
    /// 存在
    Exists,
    /// 不存在
    NotExists,
    /// 在列表中
    In,
    /// 不在列表中
    NotIn,
    /// 类型是
    TypeIs,
    /// 在范围内
    Between,
}

/// 单条断言配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assertion {
    /// 断言 ID
    pub id: String,
    /// 断言描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 目标路径
    pub target: String,
    /// 操作符
    pub operator: AssertionOperator,
    /// 期望值
    pub expected: serde_json::Value,
    /// 是否启用
    #[serde(default = "default_assertion_enabled")]
    pub enabled: bool,
}

fn default_assertion_enabled() -> bool {
    true
}

/// 断言组合模式
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionMode {
    /// 全部通过才算通过
    #[default]
    All,
    /// 任一通过即算通过
    Any,
}

/// 失败行为
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnFailureMode {
    /// 路由到 false 端口
    #[default]
    RouteFalse,
    /// 抛出错误
    Throw,
}

/// 断言节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssertionConfig {
    /// 预设名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// 断言组合模式
    #[serde(default)]
    pub mode: AssertionMode,
    /// 失败行为
    #[serde(default)]
    pub on_failure: OnFailureMode,
    /// 失败后是否继续执行后续断言
    #[serde(default)]
    pub continue_on_failure: bool,
    /// 断言列表
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    /// 可选，指定读取特定上游节点的输出进行断言判断
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_node_id: Option<String>,
}

/// 单条断言执行结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionResult {
    /// 断言 ID
    pub id: String,
    /// 断言描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否通过
    pub passed: bool,
    /// 目标路径
    pub target: String,
    /// 操作符
    pub operator: String,
    /// 期望值
    pub expected: serde_json::Value,
    /// 实际值
    pub actual: serde_json::Value,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 断言节点输出
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionOutput {
    /// 整体是否通过
    pub passed: bool,
    /// 总断言数
    pub total: usize,
    /// 通过数
    pub passed_count: usize,
    /// 失败数
    pub failed_count: usize,
    /// 所有断言结果
    pub results: Vec<AssertionResult>,
    /// 失败的断言
    pub failed_assertions: Vec<AssertionResult>,
    /// 输入数据
    pub input: serde_json::Value,
}

// ============================================================================
// 数据库节点配置
// ============================================================================

/// 数据库连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbConnectConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
    /// 数据库驱动类型: "postgres" | "mysql" | "sqlite" | "redis"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    /// 连接字符串（SQLite 使用文件路径或 :memory:）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    /// 主机地址（PostgreSQL/MySQL）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// 端口号（PostgreSQL/MySQL）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// 数据库名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    /// 用户名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 密码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 最小连接数
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    /// 最大连接数
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// 测试查询语句
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_query: Option<String>,
    /// 连接超时（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_timeout_ms: Option<u64>,
}

fn default_min_connections() -> u32 {
    1
}

fn default_max_connections() -> u32 {
    10
}

/// 数据库查询节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbQueryConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
    /// SQL 查询语句
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    /// SQL 参数列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<String>>,
    /// Redis 命令
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis_command: Option<String>,
    /// Redis 键
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis_key: Option<String>,
    /// Redis 值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis_value: Option<String>,
    /// Redis TTL（秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis_ttl: Option<u64>,
    /// 结果获取模式: "all" | "one" | "scalar"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_mode: Option<String>,
    /// 结果存储变量名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_to: Option<String>,
    /// 查询超时（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// 数据库事务开始节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbBeginConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
    /// 隔离级别
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolation_level: Option<String>,
}

/// 数据库事务提交节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbCommitConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
}

/// 数据库事务回滚节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbRollbackConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
}

/// 数据库关闭节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbCloseConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
}

// ============================================================================
// Script 节点配置
// ============================================================================

/// 脚本执行时机
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptExecutionPhase {
    /// 后置脚本（请求后执行）- 默认
    #[default]
    PostRequest,
    /// 前置脚本（请求前执行）
    PreRequest,
    /// 独立脚本（作为单独节点）
    Standalone,
}

/// 脚本错误处理策略
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptErrorStrategy {
    /// 可恢复错误 → 路由到 false 端口（默认）
    #[default]
    Recoverable,
    /// 致命错误 → 终止整个流程
    Fatal,
}

/// 断言失败处理策略
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionFailStrategy {
    /// 路由到 false 端口（默认）
    #[default]
    RouteFalse,
    /// 忽略断言，继续 true 端口
    Ignore,
    /// 终止整个流程
    Abort,
}

/// 脚本节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptNodeConfig {
    /// 脚本内容（JavaScript 代码）
    pub script: String,
    /// 超时时间（毫秒），默认 5000
    #[serde(default = "default_script_timeout")]
    pub timeout_ms: u64,
    /// 内存限制（MB），默认 10
    #[serde(default = "default_memory_limit")]
    pub memory_limit_mb: u64,
    /// 脚本执行时机
    #[serde(default)]
    pub execution_phase: ScriptExecutionPhase,
    /// 错误处理策略
    #[serde(default)]
    pub on_error: ScriptErrorStrategy,
}

fn default_script_timeout() -> u64 {
    5000
}
fn default_memory_limit() -> u64 {
    10
}

impl Default for ScriptNodeConfig {
    fn default() -> Self {
        Self {
            script: String::new(),
            timeout_ms: default_script_timeout(),
            memory_limit_mb: default_memory_limit(),
            execution_phase: ScriptExecutionPhase::default(),
            on_error: ScriptErrorStrategy::default(),
        }
    }
}

// ============================================================================
// MCP 节点配置
// ============================================================================

/// MCP 传输模式
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum McpTransportMode {
    /// Streamable HTTP 传输（默认）
    #[serde(rename = "streamable_http")]
    #[default]
    StreamableHttp,
    /// Stdio 子进程传输
    #[serde(rename = "stdio")]
    Stdio,
}

/// MCP Connect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpConnectConfig {
    /// MCP 服务器 URL（Streamable HTTP 模式使用；Stdio 模式下为空字符串）
    pub url: String,
    /// 请求头
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// 协议版本（默认 "2025-11-25"）
    #[serde(default = "default_mcp_protocol_version")]
    pub protocol_version: String,
    /// 自定义请求头
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_headers: Option<std::collections::HashMap<String, String>>,
    /// 发现的工具列表（缓存）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovered_tools: Vec<crate::traits::McpToolInfo>,
    /// 发现的资源列表（缓存）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovered_resources: Vec<crate::traits::McpResourceInfo>,
    /// 发现的提示词列表（缓存）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovered_prompts: Vec<crate::traits::McpPromptInfo>,
    /// 传输模式（默认 `streamable_http`）
    #[serde(default)]
    pub transport: McpTransportMode,
    /// Stdio 模式：可执行文件路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Stdio 模式：命令参数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Stdio 模式：环境变量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<std::collections::HashMap<String, String>>,
}

impl Default for McpConnectConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            headers: None,
            protocol_version: default_mcp_protocol_version(),
            custom_headers: None,
            discovered_tools: Vec::new(),
            discovered_resources: Vec::new(),
            discovered_prompts: Vec::new(),
            transport: McpTransportMode::default(),
            command: None,
            args: None,
            env: None,
        }
    }
}

fn default_mcp_protocol_version() -> String {
    "2025-11-25".to_string()
}

/// MCP Call Tool 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub struct McpCallToolConfig {
    /// 连接 ID（引用 `mcp_connect` 节点的 UUID）
    pub connection_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 工具参数（JSON）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}


/// MCP Read Resource 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub struct McpReadResourceConfig {
    /// 连接 ID（引用 `mcp_connect` 节点的 UUID）
    pub connection_id: String,
    /// 资源 URI
    pub resource_uri: String,
}


/// MCP Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct McpCloseConfig {
    /// 连接 ID（引用 `mcp_connect` 节点的 UUID）
    pub connection_id: String,
}

/// MCP Get Prompt 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub struct McpGetPromptConfig {
    /// 连接 ID（引用 `mcp_connect` 节点的 UUID）
    pub connection_id: String,
    /// 提示词名称
    pub prompt_name: String,
    /// 提示词参数
    #[serde(default)]
    pub arguments: std::collections::HashMap<String, String>,
}


// ============================================================================
// TCP 节点配置
// ============================================================================

/// TCP Keepalive 配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpKeepaliveConfig {
    /// 空闲时间（秒）
    pub idle_secs: u64,
    /// 探测间隔（秒）
    pub interval_secs: u64,
    /// 探测次数
    pub probe_count: u32,
}

/// TCP Connect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpConnectConfig {
    /// 目标主机地址
    pub host: String,
    /// 目标端口
    pub port: u16,
    /// 是否使用 TLS
    #[serde(default)]
    pub use_tls: bool,
    /// 是否跳过 TLS 证书验证
    #[serde(default)]
    pub tls_skip_verify: bool,
    /// 连接超时（毫秒）
    #[serde(default = "default_tcp_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// 是否禁用 Nagle 算法
    #[serde(default)]
    pub no_delay: bool,
    /// Keepalive 配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keepalive: Option<TcpKeepaliveConfig>,
    /// 自定义 CA 证书（PEM 格式内容）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_ca_cert: Option<String>,
    /// 帧定界模式
    #[serde(default)]
    pub framing_mode: TcpFramingMode,
    /// 帧配置
    #[serde(default)]
    pub framing_config: FramingConfig,
}

fn default_tcp_connect_timeout() -> u64 {
    30000
}

impl Default for TcpConnectConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 0,
            use_tls: false,
            tls_skip_verify: false,
            connect_timeout_ms: default_tcp_connect_timeout(),
            no_delay: false,
            keepalive: None,
            tls_ca_cert: None,
            framing_mode: TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
            framing_config: FramingConfig::default(),
        }
    }
}

/// TCP Send 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpSendConfig {
    /// 连接 ID（引用 `tcp_connect` 节点的 UUID）
    pub connection_id: String,
    /// 要发送的数据
    pub data: String,
    /// 数据编码方式："text" | "hex" | "base64"
    #[serde(default = "default_data_encoding")]
    pub data_encoding: String,
    /// 帧定界模式
    #[serde(default)]
    pub framing_mode: TcpFramingMode,
    /// 帧配置
    #[serde(default)]
    pub framing_config: FramingConfig,
    /// 发送超时（毫秒）
    #[serde(default = "default_tcp_send_timeout")]
    pub send_timeout_ms: u64,
    /// 协议解析器配置（JSON 序列化的 `ParserConfig`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parser_config: Option<serde_json::Value>,
}

fn default_data_encoding() -> String {
    "text".to_string()
}

fn default_tcp_send_timeout() -> u64 {
    30000
}

impl Default for TcpSendConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            data: String::new(),
            data_encoding: default_data_encoding(),
            framing_mode: TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
            framing_config: FramingConfig::default(),
            send_timeout_ms: default_tcp_send_timeout(),
            parser_config: None,
        }
    }
}

/// TCP Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpCloseConfig {
    /// 连接 ID（引用 `tcp_connect` 节点的 UUID）
    pub connection_id: String,
    /// 关闭超时（毫秒）
    #[serde(default = "default_tcp_close_timeout")]
    pub close_timeout_ms: u64,
}

fn default_tcp_close_timeout() -> u64 {
    5000
}

impl Default for TcpCloseConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            close_timeout_ms: default_tcp_close_timeout(),
        }
    }
}

/// TCP Receive 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpReceiveConfig {
    /// 连接 ID（引用 `tcp_connect` 节点的 UUID）
    pub connection_id: String,
    /// 接收超时（毫秒）
    #[serde(default = "default_tcp_receive_timeout")]
    pub receive_timeout_ms: u64,
    /// 帧定界模式
    #[serde(default)]
    pub framing_mode: TcpFramingMode,
    /// 帧配置
    #[serde(default)]
    pub framing_config: FramingConfig,
}

fn default_tcp_receive_timeout() -> u64 {
    5000
}

impl Default for TcpReceiveConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            receive_timeout_ms: default_tcp_receive_timeout(),
            framing_mode: TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
            framing_config: FramingConfig::default(),
        }
    }
}

// ============================================================================
// UDP 节点配置
// ============================================================================

/// UDP Bind 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UdpBindConfig {
    /// 本地绑定地址
    #[serde(default = "default_udp_local_addr")]
    pub local_addr: String,
    /// 本地绑定端口
    pub local_port: u16,
    /// 是否允许地址复用
    #[serde(default = "default_true")]
    pub reuse_addr: bool,
    /// 是否启用广播
    #[serde(default)]
    pub broadcast: bool,
    /// 多播组地址
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multicast_group: Option<String>,
    /// 多播 TTL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multicast_ttl: Option<u32>,
    /// 多播环回
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multicast_loop: Option<bool>,
    /// 多播绑定接口
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multicast_interface: Option<String>,
}

fn default_udp_local_addr() -> String {
    "0.0.0.0".to_string()
}

impl Default for UdpBindConfig {
    fn default() -> Self {
        Self {
            local_addr: default_udp_local_addr(),
            local_port: 0,
            reuse_addr: true,
            broadcast: false,
            multicast_group: None,
            multicast_ttl: None,
            multicast_loop: None,
            multicast_interface: None,
        }
    }
}

/// UDP `SendTo` 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UdpSendToConfig {
    /// 连接 ID（引用 `udp_bind` 节点的 UUID）
    pub connection_id: String,
    /// 要发送的数据
    pub data: String,
    /// 数据编码方式："text" | "hex" | "base64"
    #[serde(default = "default_data_encoding")]
    pub data_encoding: String,
    /// 目标地址
    pub target_addr: String,
    /// 目标端口
    pub target_port: u16,
    /// 发送超时（毫秒）
    #[serde(default = "default_udp_send_timeout")]
    pub send_timeout_ms: u64,
}

fn default_udp_send_timeout() -> u64 {
    30000
}

impl Default for UdpSendToConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            data: String::new(),
            data_encoding: default_data_encoding(),
            target_addr: String::new(),
            target_port: 0,
            send_timeout_ms: default_udp_send_timeout(),
        }
    }
}

/// UDP `RecvFrom` 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UdpRecvFromConfig {
    /// 连接 ID（引用 `udp_bind` 节点的 UUID）
    pub connection_id: String,
    /// 接收超时（毫秒）
    #[serde(default = "default_udp_recv_timeout")]
    pub receive_timeout_ms: u64,
    /// 最大接收大小（字节）
    #[serde(default = "default_udp_max_size")]
    pub max_size: usize,
}

fn default_udp_recv_timeout() -> u64 {
    5000
}

fn default_udp_max_size() -> usize {
    65536
}

impl Default for UdpRecvFromConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            receive_timeout_ms: default_udp_recv_timeout(),
            max_size: default_udp_max_size(),
        }
    }
}

/// UDP Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UdpCloseConfig {
    /// 连接 ID（引用 `udp_bind` 节点的 UUID）
    pub connection_id: String,
}

// ============================================================================
// MQTT 节点配置
// ============================================================================

/// MQTT Connect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttConnectConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_connect_type")]
    pub r#type: String,
    /// MQTT Broker 地址
    #[serde(default = "default_mqtt_url")]
    pub url: String,
    /// 客户端 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// 用户名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 密码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// MQTT 协议版本："3.1.1" 或 "5.0"
    #[serde(default = "default_mqtt_version")]
    pub mqtt_version: Option<String>,
    /// 清除会话
    #[serde(default = "default_mqtt_clean_session")]
    pub clean_session: Option<bool>,
    /// 心跳保活间隔（秒）
    #[serde(default = "default_keep_alive_secs")]
    pub keep_alive_secs: Option<u16>,
    /// 连接超时（毫秒）
    #[serde(default = "default_mqtt_connect_timeout")]
    pub connect_timeout_ms: Option<u64>,
    /// 遗嘱主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub will_topic: Option<String>,
    /// 遗嘱载荷
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub will_payload: Option<String>,
    /// 遗嘱 `QoS`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub will_qos: Option<u8>,
    /// 遗嘱保留标志
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub will_retain: Option<bool>,
    /// 是否使用 TLS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_tls: Option<bool>,
    /// 是否跳过 TLS 证书验证
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_skip_verify: Option<bool>,
    /// 会话过期间隔（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_expiry_interval: Option<u32>,
    /// 遗嘱延迟间隔（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub will_delay_interval: Option<u32>,
    /// 接收最大值（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receive_maximum: Option<u16>,
    /// 主题别名最大值（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_alias_maximum: Option<u16>,
    /// 最大报文大小（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_packet_size: Option<u32>,
    /// 用户属性（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_properties: Option<Vec<(String, String)>>,
}

fn default_mqtt_connect_type() -> String {
    "mqtt_connect".to_string()
}
fn default_mqtt_url() -> String {
    "mqtt://localhost:1883".to_string()
}
fn default_mqtt_version() -> Option<String> {
    Some("5.0".to_string())
}
fn default_keep_alive_secs() -> Option<u16> {
    Some(60)
}
fn default_mqtt_connect_timeout() -> Option<u64> {
    Some(5000)
}
fn default_mqtt_clean_session() -> Option<bool> {
    Some(true)
}

impl Default for MqttConnectConfig {
    fn default() -> Self {
        Self {
            r#type: default_mqtt_connect_type(),
            url: default_mqtt_url(),
            client_id: None,
            username: None,
            password: None,
            mqtt_version: default_mqtt_version(),
            clean_session: Some(true),
            keep_alive_secs: default_keep_alive_secs(),
            connect_timeout_ms: default_mqtt_connect_timeout(),
            will_topic: None,
            will_payload: None,
            will_qos: None,
            will_retain: None,
            use_tls: None,
            tls_skip_verify: None,
            session_expiry_interval: None,
            will_delay_interval: None,
            receive_maximum: None,
            topic_alias_maximum: None,
            max_packet_size: None,
            user_properties: None,
        }
    }
}

/// MQTT Publish 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttPublishConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_publish_type")]
    pub r#type: String,
    /// 连接 ID（引用 `mqtt_connect` 节点的 UUID）
    pub connection_id: String,
    /// 发布主题
    pub topic: String,
    /// 发布载荷
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// `QoS` 等级
    #[serde(default = "default_mqtt_qos")]
    pub qos: Option<u8>,
    /// 保留标志
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retain: Option<bool>,
    /// 载荷格式："text" | "json" | "hex" | "base64"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_format: Option<String>,
    /// 内容类型（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// 消息过期间隔（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_expiry_interval: Option<u32>,
    /// 响应主题（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_topic: Option<String>,
    /// 关联数据（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_data: Option<String>,
    /// 主题别名（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_alias: Option<u16>,
    /// 用户属性（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_properties: Option<Vec<(String, String)>>,
}

fn default_mqtt_publish_type() -> String {
    "mqtt_publish".to_string()
}
fn default_mqtt_qos() -> Option<u8> {
    Some(0)
}

impl Default for MqttPublishConfig {
    fn default() -> Self {
        Self {
            r#type: default_mqtt_publish_type(),
            connection_id: String::new(),
            topic: String::new(),
            payload: None,
            qos: default_mqtt_qos(),
            retain: None,
            payload_format: None,
            content_type: None,
            message_expiry_interval: None,
            response_topic: None,
            correlation_data: None,
            topic_alias: None,
            user_properties: None,
        }
    }
}

/// MQTT Subscribe 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttSubscribeConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_subscribe_type")]
    pub r#type: String,
    /// 连接 ID（引用 `mqtt_connect` 节点的 UUID）
    pub connection_id: String,
    /// 订阅主题
    pub topic: String,
    /// `QoS` 等级
    #[serde(default = "default_mqtt_qos")]
    pub qos: Option<u8>,
    /// 最大接收消息数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_messages: Option<usize>,
    /// 采集超时（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collect_timeout_ms: Option<u64>,
    /// 输出模式："first" | "all"
    #[serde(default = "default_mqtt_output_mode")]
    pub output_mode: Option<String>,
    /// 不转发自身发布的消息（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_local: Option<bool>,
    /// 保留消息按原样发布（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retain_as_published: Option<bool>,
    /// 保留消息处理策略（MQTT 5.0: 0, 1, 2）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retain_handling: Option<u8>,
    /// 订阅标识符（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_identifier: Option<u32>,
    /// 共享订阅组名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_group: Option<String>,
}

fn default_mqtt_subscribe_type() -> String {
    "mqtt_subscribe".to_string()
}
fn default_mqtt_output_mode() -> Option<String> {
    Some("first".to_string())
}

impl Default for MqttSubscribeConfig {
    fn default() -> Self {
        Self {
            r#type: default_mqtt_subscribe_type(),
            connection_id: String::new(),
            topic: String::new(),
            qos: default_mqtt_qos(),
            max_messages: None,
            collect_timeout_ms: None,
            output_mode: default_mqtt_output_mode(),
            no_local: None,
            retain_as_published: None,
            retain_handling: None,
            subscription_identifier: None,
            share_group: None,
        }
    }
}

/// MQTT Unsubscribe 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttUnsubscribeConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_unsubscribe_type")]
    pub r#type: String,
    /// 连接 ID（引用 `mqtt_connect` 节点的 UUID）
    pub connection_id: String,
    /// 取消订阅主题
    pub topic: String,
}

fn default_mqtt_unsubscribe_type() -> String {
    "mqtt_unsubscribe".to_string()
}

impl Default for MqttUnsubscribeConfig {
    fn default() -> Self {
        Self {
            r#type: default_mqtt_unsubscribe_type(),
            connection_id: String::new(),
            topic: String::new(),
        }
    }
}

/// MQTT Disconnect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttDisconnectConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_disconnect_type")]
    pub r#type: String,
    /// 连接 ID（引用 `mqtt_connect` 节点的 UUID）
    pub connection_id: String,
    /// 断开原因码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<u8>,
    /// 会话过期间隔（MQTT 5.0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_expiry_interval: Option<u32>,
}

fn default_mqtt_disconnect_type() -> String {
    "mqtt_disconnect".to_string()
}

impl Default for MqttDisconnectConfig {
    fn default() -> Self {
        Self {
            r#type: default_mqtt_disconnect_type(),
            connection_id: String::new(),
            reason_code: None,
            session_expiry_interval: None,
        }
    }
}

/// MQTT `WaitMessage` 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttWaitMessageConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_wait_message_type")]
    pub r#type: String,
    /// 连接 ID（引用 `mqtt_connect` 节点的 UUID）
    pub connection_id: String,
    /// 等待主题
    pub topic: String,
    /// 匹配条件（包含 `match_type` + expression）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_condition: Option<serde_json::Value>,
    /// 等待超时（毫秒）
    #[serde(default = "default_mqtt_wait_timeout")]
    pub timeout_ms: Option<u64>,
    /// 最大等待消息数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_messages: Option<usize>,
}

fn default_mqtt_wait_message_type() -> String {
    "mqtt_wait_message".to_string()
}
fn default_mqtt_wait_timeout() -> Option<u64> {
    Some(30000)
}

impl Default for MqttWaitMessageConfig {
    fn default() -> Self {
        Self {
            r#type: default_mqtt_wait_message_type(),
            connection_id: String::new(),
            topic: String::new(),
            match_condition: None,
            timeout_ms: default_mqtt_wait_timeout(),
            max_messages: None,
        }
    }
}

/// MQTT Request 节点配置（请求-响应模式）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttRequestConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_request_type")]
    pub r#type: String,
    /// 连接 ID（引用 `mqtt_connect` 节点的 UUID）
    pub connection_id: String,
    /// 请求主题
    pub request_topic: String,
    /// 响应主题
    pub response_topic: String,
    /// 请求载荷
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// `QoS` 等级
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qos: Option<u8>,
    /// 关联数据
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_data: Option<String>,
    /// 响应等待超时（毫秒）
    #[serde(default = "default_mqtt_request_timeout")]
    pub timeout_ms: Option<u64>,
}

fn default_mqtt_request_type() -> String {
    "mqtt_request".to_string()
}
fn default_mqtt_request_timeout() -> Option<u64> {
    Some(30000)
}

impl Default for MqttRequestConfig {
    fn default() -> Self {
        Self {
            r#type: default_mqtt_request_type(),
            connection_id: String::new(),
            request_topic: String::new(),
            response_topic: String::new(),
            payload: None,
            qos: None,
            correlation_data: None,
            timeout_ms: default_mqtt_request_timeout(),
        }
    }
}

// ============================================================================
// Kafka 节点配置
// ============================================================================

/// Kafka Connect 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaConnectConfig {
    /// 节点类型标识
    #[serde(default = "default_kafka_connect_type")]
    pub r#type: String,
    /// Kafka Broker 地址列表（逗号分隔）
    #[serde(default = "default_kafka_bootstrap_servers")]
    pub bootstrap_servers: String,
    /// 客户端 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// `安全协议："plaintext"/"ssl"/"sasl_plaintext"/"sasl_ssl`"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_protocol: Option<String>,
    /// SASL 认证机制："PLAIN"/"SCRAM-SHA-256"/"SCRAM-SHA-512"/"OAUTHBEARER"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sasl_mechanism: Option<String>,
    /// SASL 用户名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sasl_username: Option<String>,
    /// SASL 密码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sasl_password: Option<String>,
    /// SASL OAuth Token
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sasl_oauth_token: Option<String>,
    /// SSL CA 证书路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl_ca_cert_path: Option<String>,
    /// SSL 客户端证书路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_cert_path: Option<String>,
    /// SSL 客户端密钥路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_key_path: Option<String>,
    /// 是否跳过 SSL 证书验证
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl_skip_verify: Option<bool>,
    /// Schema Registry URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_registry_url: Option<String>,
    /// Schema Registry 用户名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_registry_username: Option<String>,
    /// Schema Registry 密码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_registry_password: Option<String>,
    /// 连接超时（毫秒）
    #[serde(default = "default_kafka_connect_timeout")]
    pub connect_timeout_ms: Option<u64>,
}

fn default_kafka_connect_type() -> String {
    "kafka_connect".to_string()
}
fn default_kafka_bootstrap_servers() -> String {
    "localhost:9092".to_string()
}
fn default_kafka_connect_timeout() -> Option<u64> {
    Some(5000)
}

impl Default for KafkaConnectConfig {
    fn default() -> Self {
        Self {
            r#type: default_kafka_connect_type(),
            bootstrap_servers: default_kafka_bootstrap_servers(),
            client_id: None,
            security_protocol: None,
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            sasl_oauth_token: None,
            ssl_ca_cert_path: None,
            ssl_client_cert_path: None,
            ssl_client_key_path: None,
            ssl_skip_verify: None,
            schema_registry_url: None,
            schema_registry_username: None,
            schema_registry_password: None,
            connect_timeout_ms: default_kafka_connect_timeout(),
        }
    }
}

/// Kafka Produce 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaProduceConfig {
    /// 节点类型标识
    #[serde(default = "default_kafka_produce_type")]
    pub r#type: String,
    /// 连接 ID（引用 `kafka_connect` 节点的 UUID）
    pub connection_id: String,
    /// 目标主题
    pub topic: String,
    /// 消息键
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// 消息值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// 消息头
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,
    /// 目标分区
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<i32>,
    /// 时间戳
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    /// 序列化格式："json"/"avro"/"protobuf"/"raw"
    #[serde(default = "default_kafka_serialization_format")]
    pub serialization_format: Option<String>,
    /// Schema 主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_subject: Option<String>,
    /// 确认超时（毫秒）
    #[serde(default = "default_kafka_ack_timeout")]
    pub ack_timeout_ms: Option<u64>,
    /// 压缩算法："none"/"gzip"/"snappy"/"lz4"/"zstd"
    #[serde(default = "default_kafka_compression")]
    pub compression: Option<String>,
}

fn default_kafka_produce_type() -> String {
    "kafka_produce".to_string()
}
fn default_kafka_serialization_format() -> Option<String> {
    Some("json".to_string())
}
fn default_kafka_ack_timeout() -> Option<u64> {
    Some(5000)
}
fn default_kafka_compression() -> Option<String> {
    Some("none".to_string())
}

impl Default for KafkaProduceConfig {
    fn default() -> Self {
        Self {
            r#type: default_kafka_produce_type(),
            connection_id: String::new(),
            topic: String::new(),
            key: None,
            value: None,
            headers: None,
            partition: None,
            timestamp: None,
            serialization_format: default_kafka_serialization_format(),
            schema_subject: None,
            ack_timeout_ms: default_kafka_ack_timeout(),
            compression: default_kafka_compression(),
        }
    }
}

/// Kafka Consume 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaConsumeConfig {
    /// 节点类型标识
    #[serde(default = "default_kafka_consume_type")]
    pub r#type: String,
    /// 连接 ID（引用 `kafka_connect` 节点的 UUID）
    pub connection_id: String,
    /// 订阅主题
    pub topic: String,
    /// 指定分区
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<i32>,
    /// 起始偏移量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    /// 偏移重置策略："earliest"/"latest"
    #[serde(default = "default_kafka_offset_reset")]
    pub offset_reset: Option<String>,
    /// 消费者组 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// 是否启用自动提交
    #[serde(default = "default_kafka_auto_commit")]
    pub enable_auto_commit: Option<bool>,
    /// 最大消费消息数
    #[serde(default = "default_kafka_max_messages")]
    pub max_messages: Option<u32>,
    /// 拉取超时（毫秒）
    #[serde(default = "default_kafka_poll_timeout")]
    pub poll_timeout_ms: Option<u64>,
    /// 反序列化格式："json"/"avro"/"protobuf"/"raw"
    #[serde(default = "default_kafka_deserialization_format")]
    pub deserialization_format: Option<String>,
    /// Schema 主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_subject: Option<String>,
    /// 最小拉取字节数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetch_min_bytes: Option<u32>,
    /// 最大拉取等待时间（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetch_max_wait_ms: Option<u32>,
}

fn default_kafka_consume_type() -> String {
    "kafka_consume".to_string()
}
fn default_kafka_offset_reset() -> Option<String> {
    Some("latest".to_string())
}
fn default_kafka_auto_commit() -> Option<bool> {
    Some(false)
}
fn default_kafka_max_messages() -> Option<u32> {
    Some(1)
}
fn default_kafka_poll_timeout() -> Option<u64> {
    Some(5000)
}
fn default_kafka_deserialization_format() -> Option<String> {
    Some("raw".to_string())
}

impl Default for KafkaConsumeConfig {
    fn default() -> Self {
        Self {
            r#type: default_kafka_consume_type(),
            connection_id: String::new(),
            topic: String::new(),
            partition: None,
            offset: None,
            offset_reset: default_kafka_offset_reset(),
            group_id: None,
            enable_auto_commit: default_kafka_auto_commit(),
            max_messages: default_kafka_max_messages(),
            poll_timeout_ms: default_kafka_poll_timeout(),
            deserialization_format: default_kafka_deserialization_format(),
            schema_subject: None,
            fetch_min_bytes: None,
            fetch_max_wait_ms: None,
        }
    }
}

/// Kafka `ConsumeBatch` 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaConsumeBatchConfig {
    /// 节点类型标识
    #[serde(default = "default_kafka_consume_batch_type")]
    pub r#type: String,
    /// 连接 ID（引用 `kafka_connect` 节点的 UUID）
    pub connection_id: String,
    /// 订阅主题
    pub topic: String,
    /// 指定分区
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<i32>,
    /// 起始偏移量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    /// 偏移重置策略："earliest"/"latest"
    #[serde(default = "default_kafka_batch_offset_reset")]
    pub offset_reset: Option<String>,
    /// 消费者组 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// 是否启用自动提交
    #[serde(default = "default_kafka_batch_auto_commit")]
    pub enable_auto_commit: Option<bool>,
    /// 最大消费消息数
    #[serde(default = "default_kafka_batch_max_messages")]
    pub max_messages: Option<u32>,
    /// 拉取超时（毫秒）
    #[serde(default = "default_kafka_batch_poll_timeout")]
    pub poll_timeout_ms: Option<u64>,
    /// 反序列化格式
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deserialization_format: Option<String>,
    /// Schema 主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_subject: Option<String>,
    /// `批量策略："count_timeout"/"count_only"/"timeout_only`"
    #[serde(default = "default_kafka_batch_strategy")]
    pub batch_strategy: Option<String>,
}

fn default_kafka_consume_batch_type() -> String {
    "kafka_consume_batch".to_string()
}
fn default_kafka_batch_offset_reset() -> Option<String> {
    Some("latest".to_string())
}
fn default_kafka_batch_auto_commit() -> Option<bool> {
    Some(false)
}
fn default_kafka_batch_max_messages() -> Option<u32> {
    Some(10)
}
fn default_kafka_batch_poll_timeout() -> Option<u64> {
    Some(10000)
}
fn default_kafka_batch_strategy() -> Option<String> {
    Some("count_timeout".to_string())
}

impl Default for KafkaConsumeBatchConfig {
    fn default() -> Self {
        Self {
            r#type: default_kafka_consume_batch_type(),
            connection_id: String::new(),
            topic: String::new(),
            partition: None,
            offset: None,
            offset_reset: default_kafka_batch_offset_reset(),
            group_id: None,
            enable_auto_commit: default_kafka_batch_auto_commit(),
            max_messages: default_kafka_batch_max_messages(),
            poll_timeout_ms: default_kafka_batch_poll_timeout(),
            deserialization_format: None,
            schema_subject: None,
            batch_strategy: default_kafka_batch_strategy(),
        }
    }
}

/// Kafka Admin 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaAdminConfig {
    /// 节点类型标识
    #[serde(default = "default_kafka_admin_type")]
    pub r#type: String,
    /// 连接 ID（引用 `kafka_connect` 节点的 UUID）
    pub connection_id: String,
    /// `管理操作："topic_create"/"topic_delete"/"topic_describe"/"topic_list"/"topic_alter_config`"/
    /// "`consumer_group_list"/"consumer_group_describe"/"consumer_group_reset_offset`"/
    /// "`broker_list"/"broker_describe_config"/"acl_create"/"acl_delete"/"acl_list`"
    pub operation: String,
    /// 主题名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_name: Option<String>,
    /// 分区数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partitions: Option<i32>,
    /// 副本因子
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replication_factor: Option<i32>,
    /// 主题配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_config: Option<HashMap<String, String>>,
    /// 消费者组 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_group_id: Option<String>,
    /// ACL 主体
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acl_principal: Option<String>,
    /// ACL 主机
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acl_host: Option<String>,
    /// ACL 操作
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acl_operation: Option<String>,
    /// ACL 权限类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acl_permission_type: Option<String>,
    /// ACL 资源类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acl_resource_type: Option<String>,
    /// ACL 资源名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acl_resource_name: Option<String>,
    /// 是否试运行
    #[serde(default = "default_kafka_dry_run")]
    pub dry_run: Option<bool>,
}

fn default_kafka_admin_type() -> String {
    "kafka_admin".to_string()
}
fn default_kafka_dry_run() -> Option<bool> {
    Some(false)
}

impl Default for KafkaAdminConfig {
    fn default() -> Self {
        Self {
            r#type: default_kafka_admin_type(),
            connection_id: String::new(),
            operation: String::new(),
            topic_name: None,
            partitions: None,
            replication_factor: None,
            topic_config: None,
            consumer_group_id: None,
            acl_principal: None,
            acl_host: None,
            acl_operation: None,
            acl_permission_type: None,
            acl_resource_type: None,
            acl_resource_name: None,
            dry_run: default_kafka_dry_run(),
        }
    }
}

/// Kafka Schema Registry 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaSchemaRegistryConfig {
    /// 节点类型标识
    #[serde(default = "default_kafka_schema_registry_type")]
    pub r#type: String,
    /// 连接 ID（引用 `kafka_connect` 节点的 UUID）
    pub connection_id: String,
    /// `操作类型："list_subjects"/"get_subject_versions"/"get_schema_by_version`"/
    /// "`check_compatibility"/"register_schema`"
    pub operation: String,
    /// Schema 主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// 版本号
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    /// Schema 定义
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_definition: Option<String>,
    /// Schema `类型："avro"/"protobuf"/"json_schema`"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
    /// 兼容性级别
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_level: Option<String>,
}

fn default_kafka_schema_registry_type() -> String {
    "kafka_schema_registry".to_string()
}

impl Default for KafkaSchemaRegistryConfig {
    fn default() -> Self {
        Self {
            r#type: default_kafka_schema_registry_type(),
            connection_id: String::new(),
            operation: String::new(),
            subject: None,
            version: None,
            schema_definition: None,
            schema_type: None,
            compatibility_level: None,
        }
    }
}

/// Kafka Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KafkaCloseConfig {
    /// 节点类型标识
    #[serde(default = "default_kafka_close_type")]
    pub r#type: String,
    /// 连接 ID（引用 `kafka_connect` 节点的 UUID）
    pub connection_id: String,
}

fn default_kafka_close_type() -> String {
    "kafka_close".to_string()
}

impl Default for KafkaCloseConfig {
    fn default() -> Self {
        Self {
            r#type: default_kafka_close_type(),
            connection_id: String::new(),
        }
    }
}

// ============================================================================
// SSE 节点配置
// ============================================================================

/// SSE 连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SseConnectConfig {
    /// SSE 端点 URL
    pub url: String,
    /// 请求头
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Last-Event-ID（断线续传）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_id: Option<String>,
    /// 连接超时（毫秒）
    #[serde(default)]
    pub timeout_ms: u64,
    /// 连接 ID（用于后续节点引用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// 是否自动重连
    #[serde(default)]
    pub auto_reconnect: bool,
    /// 重连间隔（毫秒）
    #[serde(default)]
    pub reconnect_interval_ms: u64,
    /// 最大重连次数
    #[serde(default)]
    pub max_reconnect_attempts: u32,
    /// TLS 模式
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_mode: Option<String>,
}

/// SSE 监听事件节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SseListenConfig {
    /// 连接 ID（引用 `sse_connect` 节点的 UUID）
    pub connection_id: String,
    /// 事件过滤（仅接收匹配的事件名）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_filter: Option<String>,
    /// 最大接收事件数
    #[serde(default)]
    pub max_events: u32,
    /// 监听超时（毫秒）
    #[serde(default)]
    pub timeout_ms: u64,
    /// 是否包含原始数据
    #[serde(default)]
    pub include_data: bool,
    /// 是否尝试 JSON 解码 data 字段
    #[serde(default)]
    pub decode_json: bool,
}

/// SSE 断开连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SseDisconnectConfig {
    /// 连接 ID（引用 `sse_connect` 节点的 UUID）
    pub connection_id: String,
}

// ============================================================================
// GraphQL 节点配置
// ============================================================================

/// GraphQL 连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphqlConnectConfig {
    /// GraphQL HTTP 端点 URL
    pub endpoint: String,
    /// GraphQL WebSocket 端点 URL（用于订阅）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ws_endpoint: Option<String>,
    /// 请求头
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// 连接超时（毫秒）
    #[serde(default)]
    pub connection_timeout_ms: u64,
    /// 连接 ID（用于后续节点引用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// 设计时内省发现的 Schema（JSON）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovered_schema: Option<serde_json::Value>,
    /// 设计时内省发现的查询模板
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovered_queries: Option<Vec<GraphqlQueryTemplate>>,
}

/// GraphQL 文件上传描述
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphqlFileUpload {
    /// 文件名
    pub name: String,
    /// 文件路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 文件 MIME 类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// GraphQL 查询节点配置（Query + Mutation）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphqlQueryConfig {
    /// 连接 ID（引用 `graphql_connect` 节点的 UUID）
    pub connection_id: String,
    /// GraphQL 查询/变更语句
    pub query: String,
    /// 查询变量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
    /// 操作名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    /// 查询超时（毫秒）
    #[serde(default)]
    pub timeout_ms: u64,
    /// 文件上传列表（用于 multipart 请求）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<GraphqlFileUpload>>,
}

/// GraphQL 订阅节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphqlSubscribeConfig {
    /// 连接 ID（引用 `graphql_connect` 节点的 UUID）
    pub connection_id: String,
    /// GraphQL 订阅语句
    pub subscription: String,
    /// 订阅变量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
    /// 操作名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    /// 最大接收事件数
    #[serde(default)]
    pub max_events: u32,
    /// 订阅超时（毫秒）
    #[serde(default)]
    pub timeout_ms: u64,
}

/// GraphQL Schema 探索节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphqlIntrospectConfig {
    /// 连接 ID（引用 `graphql_connect` 节点的 UUID）
    pub connection_id: String,
    /// 探索超时（毫秒）
    #[serde(default)]
    pub timeout_ms: u64,
}

/// GraphQL 断开连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphqlDisconnectConfig {
    /// 连接 ID（引用 `graphql_connect` 节点的 UUID）
    pub connection_id: String,
}

// ============================================================================
// AMQP 0-9-1 节点配置
// ============================================================================

/// AMQP 连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpConnectConfig {
    /// AMQP Broker URL
    pub url: String,
    /// 连接 ID（用于后续节点引用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// 心跳间隔（秒）
    #[serde(default)]
    pub heartbeat: u16,
    /// 最大通道数
    #[serde(default)]
    pub channel_max: u16,
    /// 最大帧大小
    #[serde(default)]
    pub frame_max: u32,
    /// 连接超时（毫秒）
    #[serde(default)]
    pub connection_timeout_ms: u64,
    /// TLS 模式
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_mode: Option<String>,
    /// 认证机制
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_mechanism: Option<String>,
    /// 是否启用自动恢复
    #[serde(default)]
    pub auto_recover: bool,
    /// 最大重连重试次数
    #[serde(default)]
    pub reconnect_max_retries: u32,
}

/// AMQP 关闭连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpCloseConfig {
    /// 连接 ID（引用 `amqp_connect` 节点的 UUID）
    pub connection_id: String,
    /// 关闭回复码
    #[serde(default)]
    pub reply_code: u16,
    /// 关闭回复文本
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_text: Option<String>,
}

/// AMQP 声明交换器节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpExchangeDeclareConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 交换器名称
    pub exchange: String,
    /// 交换器类型："direct"/"fanout"/"topic"/"headers"/"custom"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_type: Option<String>,
    /// 自定义交换器类型（当 `exchange_type` 为 "custom" 时使用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_type: Option<String>,
    /// 是否被动声明（仅检查是否存在）
    #[serde(default)]
    pub passive: bool,
    /// 是否持久化
    #[serde(default)]
    pub durable: bool,
    /// 是否自动删除
    #[serde(default)]
    pub auto_delete: bool,
    /// 是否为内部交换器
    #[serde(default)]
    pub internal: bool,
    /// 额外参数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
}

/// AMQP 删除交换器节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpExchangeDeleteConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 交换器名称
    pub exchange: String,
    /// 是否仅在不使用时删除
    #[serde(default)]
    pub if_unused: bool,
}

/// AMQP 声明队列节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpQueueDeclareConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 队列名称（空字符串表示自动生成）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<String>,
    /// 是否被动声明
    #[serde(default)]
    pub passive: bool,
    /// 是否持久化
    #[serde(default)]
    pub durable: bool,
    /// 是否排他
    #[serde(default)]
    pub exclusive: bool,
    /// 是否自动删除
    #[serde(default)]
    pub auto_delete: bool,
    /// 额外参数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
}

/// AMQP 删除队列节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpQueueDeleteConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 队列名称
    pub queue: String,
    /// 操作类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// 是否仅在不使用时删除
    #[serde(default)]
    pub if_unused: bool,
    /// 是否仅在队列为空时删除
    #[serde(default)]
    pub if_empty: bool,
}

/// AMQP 绑定队列节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpQueueBindConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 队列名称
    pub queue: String,
    /// 交换器名称
    pub exchange: String,
    /// 路由键
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_key: Option<String>,
    /// 额外参数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
}

/// AMQP 发布消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpPublishConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 目标交换器
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
    /// 路由键
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_key: Option<String>,
    /// 消息体
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// 消息体类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_type: Option<String>,
    /// Content-Type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Content-Encoding
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_encoding: Option<String>,
    /// 投递模式（1=非持久化，2=持久化）
    #[serde(default)]
    pub delivery_mode: u8,
    /// 优先级
    #[serde(default)]
    pub priority: u8,
    /// 消息 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// 关联 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// 回复队列
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// 过期时间
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
    /// 时间戳
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    /// 消息类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    /// 用户 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// 应用 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    /// 集群 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<String>,
    /// 消息头
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, serde_json::Value>>,
    /// 是否强制路由
    #[serde(default)]
    pub mandatory: bool,
    /// 确认模式
    #[serde(default)]
    pub confirm_mode: String,
}

/// AMQP 消费消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpConsumeConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 队列名称
    pub queue: String,
    /// 消费者标签
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_tag: Option<String>,
    /// 确认模式
    #[serde(default)]
    pub ack_mode: String,
    /// 是否排他消费
    #[serde(default)]
    pub exclusive: bool,
    /// 不接收自身发布的消息
    #[serde(default)]
    pub no_local: bool,
    /// 预取数量
    #[serde(default)]
    pub prefetch_count: u16,
    /// 最大接收消息数
    #[serde(default)]
    pub max_messages: u32,
    /// 消费超时（毫秒）
    #[serde(default)]
    pub timeout_ms: u64,
    /// 成功时自动确认
    #[serde(default)]
    pub auto_ack_on_success: bool,
    /// 失败时自动拒绝
    #[serde(default)]
    pub auto_reject_on_failure: bool,
    /// 拒绝后是否重新入队
    #[serde(default)]
    pub reject_requeue: bool,
}

/// AMQP 获取消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpGetConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 队列名称
    pub queue: String,
    /// 是否自动确认
    #[serde(default)]
    pub no_ack: bool,
}

/// AMQP 确认消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpAckConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 操作类型："ack"/"nack"/"reject"
    pub action: String,
    /// 投递标签
    #[serde(default)]
    pub delivery_tag: u64,
    /// 是否批量确认
    #[serde(default)]
    pub multiple: bool,
    /// 拒绝后是否重新入队
    #[serde(default)]
    pub requeue: bool,
}

/// AMQP 管理节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AmqpManagementConfig {
    /// 管理 API URL
    pub management_url: String,
    /// 用户名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 密码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 管理操作
    pub action: String,
    /// 虚拟主机
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vhost: Option<String>,
    /// 队列名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_name: Option<String>,
    /// 连接 ID
    pub connection_id: String,
}

// ============================================================================
// AMQP 1.0 节点配置
// ============================================================================

/// AMQP 1.0 连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amqp10ConnectConfig {
    /// AMQP 1.0 节点 URL
    pub url: String,
    /// 连接 ID（用于后续节点引用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// TLS 模式
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_mode: Option<String>,
    /// 认证机制
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_mechanism: Option<String>,
    /// 容器 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
}

/// AMQP 1.0 消息属性
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amqp10MessageProperties {
    /// Content-Type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Content-Encoding
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_encoding: Option<String>,
    /// 关联 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// 消息 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// 回复地址
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// 主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// 目标地址
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// 绝对过期时间（Unix 时间戳毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absolute_expiry_time: Option<u64>,
    /// 创建时间（Unix 时间戳毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_time: Option<u64>,
}

/// AMQP 1.0 发送消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amqp10SendConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 目标地址
    pub address: String,
    /// 消息体
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_body: Option<String>,
    /// 消息属性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_properties: Option<Amqp10MessageProperties>,
    /// 消息注解
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_annotations: Option<HashMap<String, serde_json::Value>>,
}

/// AMQP 1.0 接收消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amqp10ReceiveConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 源地址
    pub address: String,
    /// 信用额度
    #[serde(default)]
    pub credit: u32,
    /// 最大接收消息数
    #[serde(default)]
    pub max_messages: u32,
    /// 接收超时（毫秒）
    #[serde(default)]
    pub timeout_ms: u64,
    /// 是否自动接受
    #[serde(default)]
    pub auto_accept: bool,
}

/// AMQP 1.0 确认消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amqp10AcceptConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 投递 ID
    #[serde(default)]
    pub delivery_id: u64,
}

/// AMQP 1.0 拒绝消息节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amqp10RejectConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 投递 ID
    #[serde(default)]
    pub delivery_id: u64,
    /// 错误条件
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_condition: Option<String>,
    /// 错误描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

/// AMQP 1.0 断开连接节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Amqp10DisconnectConfig {
    /// 连接 ID
    pub connection_id: String,
}

// ============================================================================
// SMTP 节点配置
// ============================================================================

/// SMTP Connect 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SmtpConnectConfig {
    /// SMTP 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
    /// TLS 模式 ("implicit" | "starttls" | "none")
    #[serde(default)]
    pub tls_mode: String,
    /// 认证配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<serde_json::Value>,
    /// 连接超时（毫秒）
    #[serde(default = "default_smtp_connect_timeout")]
    pub connect_timeout_ms: u64,
}

fn default_smtp_connect_timeout() -> u64 {
    30000
}

/// SMTP Send 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SmtpSendConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 发件人
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// 收件人列表
    #[serde(default)]
    pub to: Vec<String>,
    /// 抄送列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
    /// 密送列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bcc: Option<Vec<String>>,
    /// 主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// 正文类型 ("`plain_text`" | "html" | "both")
    #[serde(default)]
    pub body_type: String,
    /// 纯文本正文
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// HTML 正文
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    /// 附件列表
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<serde_json::Value>>,
    /// 发送超时（毫秒）
    #[serde(default = "default_smtp_send_timeout")]
    pub timeout_ms: u64,
}

fn default_smtp_send_timeout() -> u64 {
    30000
}

/// SMTP Disconnect 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SmtpDisconnectConfig {
    /// 连接 ID
    pub connection_id: String,
}

// ============================================================================
// IMAP 节点配置
// ============================================================================

/// IMAP Connect 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapConnectConfig {
    /// IMAP 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
    /// TLS 模式 ("implicit" | "starttls" | "none")
    #[serde(default)]
    pub tls_mode: String,
    /// 认证配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<serde_json::Value>,
    /// 连接超时（毫秒）
    #[serde(default = "default_imap_connect_timeout")]
    pub connect_timeout_ms: u64,
}

fn default_imap_connect_timeout() -> u64 {
    30000
}

/// IMAP Select 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapSelectConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 邮箱名称
    #[serde(default = "default_imap_mailbox")]
    pub mailbox: String,
    /// 是否只读模式
    #[serde(default)]
    pub readonly: bool,
}

fn default_imap_mailbox() -> String {
    "INBOX".to_string()
}

/// IMAP Search 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapSearchConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 邮箱名称（可选，不指定则使用当前已选择的邮箱）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mailbox: Option<String>,
    /// 搜索条件
    #[serde(default)]
    pub criteria: String,
    /// 最大返回结果数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_results: Option<usize>,
}

/// IMAP Fetch 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapFetchConfig {
    /// 连接 ID
    pub connection_id: String,
    /// UID 范围 ("1:5" or "101,102,105")
    #[serde(default)]
    pub uids: String,
    /// 获取项目 ("(ENVELOPE BODY.PEEK[] FLAGS)")
    #[serde(default = "default_imap_fetch_items")]
    pub fetch_items: String,
    /// 最大获取数量
    #[serde(default = "default_imap_max_fetch")]
    pub max_fetch_count: usize,
    /// 最大邮件大小（字节）
    #[serde(default = "default_imap_max_size")]
    pub max_size_bytes: usize,
}

fn default_imap_fetch_items() -> String {
    "(ENVELOPE BODY.PEEK[] FLAGS)".to_string()
}
fn default_imap_max_fetch() -> usize {
    50
}
fn default_imap_max_size() -> usize {
    5 * 1024 * 1024
}

/// IMAP Store Flags 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapStoreFlagsConfig {
    /// 连接 ID
    pub connection_id: String,
    /// UID 范围
    #[serde(default)]
    pub uids: String,
    /// 操作类型 ("set" | "add" | "remove")
    #[serde(default = "default_imap_store_operation")]
    pub operation: String,
    /// 标志列表
    #[serde(default)]
    pub flags: Vec<String>,
}

fn default_imap_store_operation() -> String {
    "add".to_string()
}

/// IMAP Copy 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapCopyConfig {
    /// 连接 ID
    pub connection_id: String,
    /// UID 范围
    #[serde(default)]
    pub uids: String,
    /// 目标邮箱
    pub target_mailbox: String,
}

/// IMAP Move 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapMoveConfig {
    /// 连接 ID
    pub connection_id: String,
    /// UID 范围
    #[serde(default)]
    pub uids: String,
    /// 目标邮箱
    pub target_mailbox: String,
}

/// IMAP Expunge 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapExpungeConfig {
    /// 连接 ID
    pub connection_id: String,
}

/// IMAP List 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapListConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 引用名称
    #[serde(default = "default_imap_list_reference")]
    pub reference_name: String,
    /// 匹配模式
    #[serde(default = "default_imap_list_pattern")]
    pub pattern: String,
}

fn default_imap_list_reference() -> String {
    String::new()
}
fn default_imap_list_pattern() -> String {
    "*".to_string()
}

/// IMAP Folder Create 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapFolderCreateConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 文件夹名称
    pub folder_name: String,
}

/// IMAP Folder Delete 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapFolderDeleteConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 文件夹名称
    pub folder_name: String,
}

/// IMAP IDLE 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapIdleConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 邮箱名称（可选）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mailbox: Option<String>,
    /// 超时时间（秒）
    #[serde(default = "default_imap_idle_timeout")]
    pub timeout_seconds: u64,
}

fn default_imap_idle_timeout() -> u64 {
    300
}

/// IMAP Close 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapCloseConfig {
    /// 连接 ID
    pub connection_id: String,
}

// ============================================================================
// Redis 节点配置
// ============================================================================

fn default_redis_port() -> u16 {
    6379
}

fn default_redis_connect_timeout() -> u64 {
    10000
}

/// Redis Connect 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RedisConnectConfig {
    /// 服务器地址
    pub host: String,
    /// 端口
    #[serde(default = "default_redis_port")]
    pub port: u16,
    /// 密码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 数据库索引
    #[serde(default)]
    pub db_index: i64,
    /// 连接超时（毫秒）
    #[serde(default = "default_redis_connect_timeout")]
    pub connect_timeout_ms: u64,
}

/// Redis Command 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RedisCommandConfig {
    /// 连接 ID
    pub connection_id: String,
    /// Redis 命令
    pub command: String,
    /// 命令参数
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

/// Redis Close 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RedisCloseConfig {
    /// 连接 ID
    pub connection_id: String,
}

// ============================================================================
// 代理设置节点配置
// ============================================================================

/// 代理模式
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMode {
    /// 清除代理（恢复直连）
    #[default]
    Clear,
    /// HTTP 代理（CONNECT 隧道）
    Http,
    /// SOCKS5 代理
    Socks5,
}

/// 代理设置节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 节点类型标识
    #[serde(default = "default_proxy_type")]
    pub r#type: String,
    /// 代理模式
    #[serde(default)]
    pub mode: ProxyMode,
    /// 代理服务器地址（仅 http/socks5 模式需要）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    /// 代理用户名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 代理密码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

fn default_proxy_type() -> String {
    "proxy".to_string()
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            r#type: default_proxy_type(),
            mode: ProxyMode::Clear,
            proxy_url: None,
            username: None,
            password: None,
        }
    }
}

// ============================================================================
// 变量提取节点配置
// ============================================================================

/// 变量提取节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VariableExtractorConfig {
    /// 节点类型标识
    #[serde(default = "default_variable_extractor_type")]
    pub r#type: String,
    /// 输出变量映射列表
    #[serde(default)]
    pub output_mappings: Vec<crate::variable::OutputMapping>,
    /// 可选，指定读取特定上游节点的输出（UUID）。
    /// 为 None 时读取 `last_node_output`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_node_id: Option<String>,
}

fn default_variable_extractor_type() -> String {
    "variable_extractor".to_string()
}

// ============================================================================
// End 节点配置
// ============================================================================

/// 结束节点配置（无字段，标记流程结束）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EndConfig;

// ============================================================================
// 节点数据配置统一枚举
// ============================================================================

/// 节点数据配置（强类型枚举）
///
/// 使用 `#[serde(tag = "type")]` 进行内部标签的反序列化。
/// 对于未知的 `type` 值，通过自定义反序列化回退到 `Plugin(Value)` 变体，
/// 保证向前兼容——反序列化未知节点类型不会报错。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NodeData {
    /// 入口节点配置
    Entry(EntryConfig),
    /// HTTP 请求节点配置
    Http(HttpConfig),
    /// 条件分支节点配置
    Conditional(ConditionalConfig),
    /// WebSocket 连接节点配置
    #[serde(rename = "ws_connect")]
    WsConnect(WsConnectConfig),
    /// WebSocket 发送采集节点配置
    #[serde(rename = "ws_send_collect")]
    WsSendCollect(WsSendCollectConfig),
    /// WebSocket 关闭节点配置
    #[serde(rename = "ws_close")]
    WsClose(WsCloseConfig),
    /// 断言节点配置
    Assertion(AssertionConfig),
    /// 数据库连接节点配置
    #[serde(rename = "db_connect")]
    DbConnect(DbConnectConfig),
    /// 数据库查询节点配置
    #[serde(rename = "db_query")]
    DbQuery(DbQueryConfig),
    /// 数据库事务开始节点配置
    #[serde(rename = "db_begin")]
    DbBegin(DbBeginConfig),
    /// 数据库事务提交节点配置
    #[serde(rename = "db_commit")]
    DbCommit(DbCommitConfig),
    /// 数据库事务回滚节点配置
    #[serde(rename = "db_rollback")]
    DbRollback(DbRollbackConfig),
    /// 数据库关闭节点配置
    #[serde(rename = "db_close")]
    DbClose(DbCloseConfig),
    /// 脚本节点配置
    #[serde(rename = "script")]
    Script(ScriptNodeConfig),
    /// gRPC 连接节点配置
    #[serde(rename = "grpc_connect")]
    GrpcConnect(GrpcConnectConfig),
    /// gRPC 调用节点配置
    #[serde(rename = "grpc_call")]
    GrpcCall(GrpcCallConfig),
    /// gRPC 关闭节点配置
    #[serde(rename = "grpc_close")]
    GrpcClose(GrpcCloseConfig),
    /// MCP 连接节点配置
    #[serde(rename = "mcp_connect")]
    McpConnect(McpConnectConfig),
    /// MCP 调用工具节点配置
    #[serde(rename = "mcp_call_tool")]
    McpCallTool(McpCallToolConfig),
    /// MCP 读取资源节点配置
    #[serde(rename = "mcp_read_resource")]
    McpReadResource(McpReadResourceConfig),
    /// MCP 关闭连接节点配置
    #[serde(rename = "mcp_close")]
    McpClose(McpCloseConfig),
    /// MCP 获取提示词节点配置
    #[serde(rename = "mcp_get_prompt")]
    McpGetPrompt(McpGetPromptConfig),
    /// TCP 连接节点配置
    #[serde(rename = "tcp_connect")]
    TcpConnect(TcpConnectConfig),
    /// TCP 发送节点配置
    #[serde(rename = "tcp_send")]
    TcpSend(TcpSendConfig),
    /// TCP 接收节点配置
    #[serde(rename = "tcp_receive")]
    TcpReceive(TcpReceiveConfig),
    /// TCP 关闭节点配置
    #[serde(rename = "tcp_close")]
    TcpClose(TcpCloseConfig),
    /// UDP 绑定节点配置
    #[serde(rename = "udp_bind")]
    UdpBind(UdpBindConfig),
    /// UDP 发送节点配置
    #[serde(rename = "udp_send_to")]
    UdpSendTo(UdpSendToConfig),
    /// UDP 接收节点配置
    #[serde(rename = "udp_recv_from")]
    UdpRecvFrom(UdpRecvFromConfig),
    /// UDP 关闭节点配置
    #[serde(rename = "udp_close")]
    UdpClose(UdpCloseConfig),
    /// MQTT 连接节点配置
    #[serde(rename = "mqtt_connect")]
    MqttConnect(MqttConnectConfig),
    /// MQTT 发布节点配置
    #[serde(rename = "mqtt_publish")]
    MqttPublish(MqttPublishConfig),
    /// MQTT 订阅节点配置
    #[serde(rename = "mqtt_subscribe")]
    MqttSubscribe(MqttSubscribeConfig),
    /// MQTT 取消订阅节点配置
    #[serde(rename = "mqtt_unsubscribe")]
    MqttUnsubscribe(MqttUnsubscribeConfig),
    /// MQTT 断开连接节点配置
    #[serde(rename = "mqtt_disconnect")]
    MqttDisconnect(MqttDisconnectConfig),
    /// MQTT 等待消息节点配置
    #[serde(rename = "mqtt_wait_message")]
    MqttWaitMessage(MqttWaitMessageConfig),
    /// MQTT 请求-响应节点配置
    #[serde(rename = "mqtt_request")]
    MqttRequest(MqttRequestConfig),
    /// Kafka 连接节点配置
    #[serde(rename = "kafka_connect")]
    KafkaConnect(KafkaConnectConfig),
    /// Kafka 生产消息节点配置
    #[serde(rename = "kafka_produce")]
    KafkaProduce(KafkaProduceConfig),
    /// Kafka 消费消息节点配置
    #[serde(rename = "kafka_consume")]
    KafkaConsume(KafkaConsumeConfig),
    /// Kafka 批量消费消息节点配置
    #[serde(rename = "kafka_consume_batch")]
    KafkaConsumeBatch(KafkaConsumeBatchConfig),
    /// Kafka 管理操作节点配置
    #[serde(rename = "kafka_admin")]
    KafkaAdmin(KafkaAdminConfig),
    /// Kafka Schema Registry 节点配置
    #[serde(rename = "kafka_schema_registry")]
    KafkaSchemaRegistry(KafkaSchemaRegistryConfig),
    /// Kafka 关闭连接节点配置
    #[serde(rename = "kafka_close")]
    KafkaClose(KafkaCloseConfig),
    // SSE 节点
    /// SSE 连接节点配置
    #[serde(rename = "sse_connect")]
    SseConnect(SseConnectConfig),
    /// SSE 监听事件节点配置
    #[serde(rename = "sse_listen")]
    SseListen(SseListenConfig),
    /// SSE 断开连接节点配置
    #[serde(rename = "sse_disconnect")]
    SseDisconnect(SseDisconnectConfig),
    // GraphQL 节点
    /// GraphQL 连接节点配置
    #[serde(rename = "graphql_connect")]
    GraphqlConnect(GraphqlConnectConfig),
    /// GraphQL 查询节点配置
    #[serde(rename = "graphql_query")]
    GraphqlQuery(GraphqlQueryConfig),
    /// GraphQL 订阅节点配置
    #[serde(rename = "graphql_subscribe")]
    GraphqlSubscribe(GraphqlSubscribeConfig),
    /// GraphQL Schema 探索节点配置
    #[serde(rename = "graphql_introspect")]
    GraphqlIntrospect(GraphqlIntrospectConfig),
    /// GraphQL 断开连接节点配置
    #[serde(rename = "graphql_disconnect")]
    GraphqlDisconnect(GraphqlDisconnectConfig),
    // AMQP 0-9-1 节点
    /// AMQP 连接节点配置
    #[serde(rename = "amqp_connect")]
    AmqpConnect(AmqpConnectConfig),
    /// AMQP 关闭连接节点配置
    #[serde(rename = "amqp_close")]
    AmqpClose(AmqpCloseConfig),
    /// AMQP 声明交换器节点配置
    #[serde(rename = "amqp_exchange_declare")]
    AmqpExchangeDeclare(AmqpExchangeDeclareConfig),
    /// AMQP 删除交换器节点配置
    #[serde(rename = "amqp_exchange_delete")]
    AmqpExchangeDelete(AmqpExchangeDeleteConfig),
    /// AMQP 声明队列节点配置
    #[serde(rename = "amqp_queue_declare")]
    AmqpQueueDeclare(AmqpQueueDeclareConfig),
    /// AMQP 删除队列节点配置
    #[serde(rename = "amqp_queue_delete")]
    AmqpQueueDelete(AmqpQueueDeleteConfig),
    /// AMQP 绑定队列节点配置
    #[serde(rename = "amqp_queue_bind")]
    AmqpQueueBind(AmqpQueueBindConfig),
    /// AMQP 发布消息节点配置
    #[serde(rename = "amqp_publish")]
    AmqpPublish(AmqpPublishConfig),
    /// AMQP 消费消息节点配置
    #[serde(rename = "amqp_consume")]
    AmqpConsume(AmqpConsumeConfig),
    /// AMQP 获取消息节点配置
    #[serde(rename = "amqp_get")]
    AmqpGet(AmqpGetConfig),
    /// AMQP 确认消息节点配置
    #[serde(rename = "amqp_ack")]
    AmqpAck(AmqpAckConfig),
    /// AMQP 管理节点配置
    #[serde(rename = "amqp_management")]
    AmqpManagement(AmqpManagementConfig),
    // AMQP 1.0 节点
    /// AMQP 1.0 连接节点配置
    #[serde(rename = "amqp10_connect")]
    Amqp10Connect(Amqp10ConnectConfig),
    /// AMQP 1.0 发送消息节点配置
    #[serde(rename = "amqp10_send")]
    Amqp10Send(Amqp10SendConfig),
    /// AMQP 1.0 接收消息节点配置
    #[serde(rename = "amqp10_receive")]
    Amqp10Receive(Amqp10ReceiveConfig),
    /// AMQP 1.0 确认消息节点配置
    #[serde(rename = "amqp10_accept")]
    Amqp10Accept(Amqp10AcceptConfig),
    /// AMQP 1.0 拒绝消息节点配置
    #[serde(rename = "amqp10_reject")]
    Amqp10Reject(Amqp10RejectConfig),
    /// AMQP 1.0 断开连接节点配置
    #[serde(rename = "amqp10_disconnect")]
    Amqp10Disconnect(Amqp10DisconnectConfig),
    // SMTP 节点
    /// SMTP 连接节点配置
    #[serde(rename = "smtp_connect")]
    SmtpConnect(SmtpConnectConfig),
    /// SMTP 发送邮件节点配置
    #[serde(rename = "smtp_send")]
    SmtpSend(SmtpSendConfig),
    /// SMTP 断开连接节点配置
    #[serde(rename = "smtp_disconnect")]
    SmtpDisconnect(SmtpDisconnectConfig),
    // IMAP 节点
    /// IMAP 连接节点配置
    #[serde(rename = "imap_connect")]
    ImapConnect(ImapConnectConfig),
    /// IMAP 选择邮箱节点配置
    #[serde(rename = "imap_select")]
    ImapSelect(ImapSelectConfig),
    /// IMAP 搜索邮件节点配置
    #[serde(rename = "imap_search")]
    ImapSearch(ImapSearchConfig),
    /// IMAP 获取邮件节点配置
    #[serde(rename = "imap_fetch")]
    ImapFetch(ImapFetchConfig),
    /// IMAP 设置标志节点配置
    #[serde(rename = "imap_store_flags")]
    ImapStoreFlags(ImapStoreFlagsConfig),
    /// IMAP 复制邮件节点配置
    #[serde(rename = "imap_copy")]
    ImapCopy(ImapCopyConfig),
    /// IMAP 移动邮件节点配置
    #[serde(rename = "imap_move")]
    ImapMove(ImapMoveConfig),
    /// IMAP 永久删除节点配置
    #[serde(rename = "imap_expunge")]
    ImapExpunge(ImapExpungeConfig),
    /// IMAP 列出邮箱节点配置
    #[serde(rename = "imap_list")]
    ImapList(ImapListConfig),
    /// IMAP 创建文件夹节点配置
    #[serde(rename = "imap_folder_create")]
    ImapFolderCreate(ImapFolderCreateConfig),
    /// IMAP 删除文件夹节点配置
    #[serde(rename = "imap_folder_delete")]
    ImapFolderDelete(ImapFolderDeleteConfig),
    /// IMAP IDLE 监听节点配置
    #[serde(rename = "imap_idle")]
    ImapIdle(ImapIdleConfig),
    /// IMAP 关闭连接节点配置
    #[serde(rename = "imap_close")]
    ImapClose(ImapCloseConfig),
    // Redis 节点
    /// Redis 连接节点配置
    #[serde(rename = "redis_connect")]
    RedisConnect(RedisConnectConfig),
    /// Redis 命令节点配置
    #[serde(rename = "redis_command")]
    RedisCommand(RedisCommandConfig),
    /// Redis 关闭连接节点配置
    #[serde(rename = "redis_close")]
    RedisClose(RedisCloseConfig),
    /// 代理设置节点配置
    #[serde(rename = "proxy")]
    Proxy(ProxyConfig),
    /// 变量提取节点配置
    #[serde(rename = "variable_extractor")]
    VariableExtractor(VariableExtractorConfig),
    /// 结束节点配置
    End(EndConfig),
    /// 插件节点配置（未知类型的兜底捕获）
    ///
    /// 当反序列化遇到不认识的 `type` 值时，整个 JSON 对象会被保留在此变体中。
    /// 此变体必须是枚举的最后一个。
    Plugin(serde_json::Value),
}

/// 自定义 `NodeData` 反序列化：已知 type 走标准 serde 内部标签逻辑，
/// 未知 type 回退到 `Plugin(Value)`。
impl<'de> serde::Deserialize<'de> for NodeData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // 先反序列化为通用的 Map，提取 type 字段
        let value = serde_json::Value::deserialize(deserializer)?;
        let type_str = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 尝试按已知类型反序列化
        match type_str {
            "entry" => serde_json::from_value::<EntryConfig>(value.clone())
                .map(NodeData::Entry)
                .map_err(serde::de::Error::custom),
            "http" => serde_json::from_value::<HttpConfig>(value.clone())
                .map(NodeData::Http)
                .map_err(serde::de::Error::custom),
            "conditional" => serde_json::from_value::<ConditionalConfig>(value.clone())
                .map(NodeData::Conditional)
                .map_err(serde::de::Error::custom),
            "ws_connect" => serde_json::from_value::<WsConnectConfig>(value.clone())
                .map(NodeData::WsConnect)
                .map_err(serde::de::Error::custom),
            "ws_send_collect" => serde_json::from_value::<WsSendCollectConfig>(value.clone())
                .map(NodeData::WsSendCollect)
                .map_err(serde::de::Error::custom),
            "ws_close" => serde_json::from_value::<WsCloseConfig>(value.clone())
                .map(NodeData::WsClose)
                .map_err(serde::de::Error::custom),
            "assertion" => serde_json::from_value::<AssertionConfig>(value.clone())
                .map(NodeData::Assertion)
                .map_err(serde::de::Error::custom),
            "db_connect" => serde_json::from_value::<DbConnectConfig>(value.clone())
                .map(NodeData::DbConnect)
                .map_err(serde::de::Error::custom),
            "db_query" => serde_json::from_value::<DbQueryConfig>(value.clone())
                .map(NodeData::DbQuery)
                .map_err(serde::de::Error::custom),
            "db_begin" => serde_json::from_value::<DbBeginConfig>(value.clone())
                .map(NodeData::DbBegin)
                .map_err(serde::de::Error::custom),
            "db_commit" => serde_json::from_value::<DbCommitConfig>(value.clone())
                .map(NodeData::DbCommit)
                .map_err(serde::de::Error::custom),
            "db_rollback" => serde_json::from_value::<DbRollbackConfig>(value.clone())
                .map(NodeData::DbRollback)
                .map_err(serde::de::Error::custom),
            "db_close" => serde_json::from_value::<DbCloseConfig>(value.clone())
                .map(NodeData::DbClose)
                .map_err(serde::de::Error::custom),
            "script" => serde_json::from_value::<ScriptNodeConfig>(value.clone())
                .map(NodeData::Script)
                .map_err(serde::de::Error::custom),
            "grpc_connect" => serde_json::from_value::<GrpcConnectConfig>(value.clone())
                .map(NodeData::GrpcConnect)
                .map_err(serde::de::Error::custom),
            "grpc_call" => serde_json::from_value::<GrpcCallConfig>(value.clone())
                .map(NodeData::GrpcCall)
                .map_err(serde::de::Error::custom),
            "grpc_close" => serde_json::from_value::<GrpcCloseConfig>(value.clone())
                .map(NodeData::GrpcClose)
                .map_err(serde::de::Error::custom),
            "mcp_connect" => serde_json::from_value::<McpConnectConfig>(value.clone())
                .map(NodeData::McpConnect)
                .map_err(serde::de::Error::custom),
            "mcp_call_tool" => serde_json::from_value::<McpCallToolConfig>(value.clone())
                .map(NodeData::McpCallTool)
                .map_err(serde::de::Error::custom),
            "mcp_read_resource" => serde_json::from_value::<McpReadResourceConfig>(value.clone())
                .map(NodeData::McpReadResource)
                .map_err(serde::de::Error::custom),
            "mcp_close" => serde_json::from_value::<McpCloseConfig>(value.clone())
                .map(NodeData::McpClose)
                .map_err(serde::de::Error::custom),
            "mcp_get_prompt" => serde_json::from_value::<McpGetPromptConfig>(value.clone())
                .map(NodeData::McpGetPrompt)
                .map_err(serde::de::Error::custom),
            "tcp_connect" => serde_json::from_value::<TcpConnectConfig>(value.clone())
                .map(NodeData::TcpConnect)
                .map_err(serde::de::Error::custom),
            "tcp_send" => serde_json::from_value::<TcpSendConfig>(value.clone())
                .map(NodeData::TcpSend)
                .map_err(serde::de::Error::custom),
            "tcp_receive" => serde_json::from_value::<TcpReceiveConfig>(value.clone())
                .map(NodeData::TcpReceive)
                .map_err(serde::de::Error::custom),
            "tcp_close" => serde_json::from_value::<TcpCloseConfig>(value.clone())
                .map(NodeData::TcpClose)
                .map_err(serde::de::Error::custom),
            "udp_bind" => serde_json::from_value::<UdpBindConfig>(value.clone())
                .map(NodeData::UdpBind)
                .map_err(serde::de::Error::custom),
            "udp_send_to" => serde_json::from_value::<UdpSendToConfig>(value.clone())
                .map(NodeData::UdpSendTo)
                .map_err(serde::de::Error::custom),
            "udp_recv_from" => serde_json::from_value::<UdpRecvFromConfig>(value.clone())
                .map(NodeData::UdpRecvFrom)
                .map_err(serde::de::Error::custom),
            "udp_close" => serde_json::from_value::<UdpCloseConfig>(value.clone())
                .map(NodeData::UdpClose)
                .map_err(serde::de::Error::custom),
            "mqtt_connect" => serde_json::from_value::<MqttConnectConfig>(value.clone())
                .map(NodeData::MqttConnect)
                .map_err(serde::de::Error::custom),
            "mqtt_publish" => serde_json::from_value::<MqttPublishConfig>(value.clone())
                .map(NodeData::MqttPublish)
                .map_err(serde::de::Error::custom),
            "mqtt_subscribe" => serde_json::from_value::<MqttSubscribeConfig>(value.clone())
                .map(NodeData::MqttSubscribe)
                .map_err(serde::de::Error::custom),
            "mqtt_unsubscribe" => serde_json::from_value::<MqttUnsubscribeConfig>(value.clone())
                .map(NodeData::MqttUnsubscribe)
                .map_err(serde::de::Error::custom),
            "mqtt_disconnect" => serde_json::from_value::<MqttDisconnectConfig>(value.clone())
                .map(NodeData::MqttDisconnect)
                .map_err(serde::de::Error::custom),
            "mqtt_wait_message" => serde_json::from_value::<MqttWaitMessageConfig>(value.clone())
                .map(NodeData::MqttWaitMessage)
                .map_err(serde::de::Error::custom),
            "mqtt_request" => serde_json::from_value::<MqttRequestConfig>(value.clone())
                .map(NodeData::MqttRequest)
                .map_err(serde::de::Error::custom),
            "kafka_connect" => serde_json::from_value::<KafkaConnectConfig>(value.clone())
                .map(NodeData::KafkaConnect)
                .map_err(serde::de::Error::custom),
            "kafka_produce" => serde_json::from_value::<KafkaProduceConfig>(value.clone())
                .map(NodeData::KafkaProduce)
                .map_err(serde::de::Error::custom),
            "kafka_consume" => serde_json::from_value::<KafkaConsumeConfig>(value.clone())
                .map(NodeData::KafkaConsume)
                .map_err(serde::de::Error::custom),
            "kafka_consume_batch" => serde_json::from_value::<KafkaConsumeBatchConfig>(value.clone())
                .map(NodeData::KafkaConsumeBatch)
                .map_err(serde::de::Error::custom),
            "kafka_admin" => serde_json::from_value::<KafkaAdminConfig>(value.clone())
                .map(NodeData::KafkaAdmin)
                .map_err(serde::de::Error::custom),
            "kafka_schema_registry" => serde_json::from_value::<KafkaSchemaRegistryConfig>(value.clone())
                .map(NodeData::KafkaSchemaRegistry)
                .map_err(serde::de::Error::custom),
            "kafka_close" => serde_json::from_value::<KafkaCloseConfig>(value.clone())
                .map(NodeData::KafkaClose)
                .map_err(serde::de::Error::custom),
            // SSE 节点
            "sse_connect" => serde_json::from_value::<SseConnectConfig>(value.clone())
                .map(NodeData::SseConnect)
                .map_err(serde::de::Error::custom),
            "sse_listen" => serde_json::from_value::<SseListenConfig>(value.clone())
                .map(NodeData::SseListen)
                .map_err(serde::de::Error::custom),
            "sse_disconnect" => serde_json::from_value::<SseDisconnectConfig>(value.clone())
                .map(NodeData::SseDisconnect)
                .map_err(serde::de::Error::custom),
            // GraphQL 节点
            "graphql_connect" => serde_json::from_value::<GraphqlConnectConfig>(value.clone())
                .map(NodeData::GraphqlConnect)
                .map_err(serde::de::Error::custom),
            "graphql_query" => serde_json::from_value::<GraphqlQueryConfig>(value.clone())
                .map(NodeData::GraphqlQuery)
                .map_err(serde::de::Error::custom),
            "graphql_subscribe" => serde_json::from_value::<GraphqlSubscribeConfig>(value.clone())
                .map(NodeData::GraphqlSubscribe)
                .map_err(serde::de::Error::custom),
            "graphql_introspect" => serde_json::from_value::<GraphqlIntrospectConfig>(value.clone())
                .map(NodeData::GraphqlIntrospect)
                .map_err(serde::de::Error::custom),
            "graphql_disconnect" => serde_json::from_value::<GraphqlDisconnectConfig>(value.clone())
                .map(NodeData::GraphqlDisconnect)
                .map_err(serde::de::Error::custom),
            // AMQP 0-9-1 节点
            "amqp_connect" => serde_json::from_value::<AmqpConnectConfig>(value.clone())
                .map(NodeData::AmqpConnect)
                .map_err(serde::de::Error::custom),
            "amqp_close" => serde_json::from_value::<AmqpCloseConfig>(value.clone())
                .map(NodeData::AmqpClose)
                .map_err(serde::de::Error::custom),
            "amqp_exchange_declare" => serde_json::from_value::<AmqpExchangeDeclareConfig>(value.clone())
                .map(NodeData::AmqpExchangeDeclare)
                .map_err(serde::de::Error::custom),
            "amqp_exchange_delete" => serde_json::from_value::<AmqpExchangeDeleteConfig>(value.clone())
                .map(NodeData::AmqpExchangeDelete)
                .map_err(serde::de::Error::custom),
            "amqp_queue_declare" => serde_json::from_value::<AmqpQueueDeclareConfig>(value.clone())
                .map(NodeData::AmqpQueueDeclare)
                .map_err(serde::de::Error::custom),
            "amqp_queue_delete" => serde_json::from_value::<AmqpQueueDeleteConfig>(value.clone())
                .map(NodeData::AmqpQueueDelete)
                .map_err(serde::de::Error::custom),
            "amqp_queue_bind" => serde_json::from_value::<AmqpQueueBindConfig>(value.clone())
                .map(NodeData::AmqpQueueBind)
                .map_err(serde::de::Error::custom),
            "amqp_publish" => serde_json::from_value::<AmqpPublishConfig>(value.clone())
                .map(NodeData::AmqpPublish)
                .map_err(serde::de::Error::custom),
            "amqp_consume" => serde_json::from_value::<AmqpConsumeConfig>(value.clone())
                .map(NodeData::AmqpConsume)
                .map_err(serde::de::Error::custom),
            "amqp_get" => serde_json::from_value::<AmqpGetConfig>(value.clone())
                .map(NodeData::AmqpGet)
                .map_err(serde::de::Error::custom),
            "amqp_ack" => serde_json::from_value::<AmqpAckConfig>(value.clone())
                .map(NodeData::AmqpAck)
                .map_err(serde::de::Error::custom),
            "amqp_management" => serde_json::from_value::<AmqpManagementConfig>(value.clone())
                .map(NodeData::AmqpManagement)
                .map_err(serde::de::Error::custom),
            // AMQP 1.0 节点
            "amqp10_connect" => serde_json::from_value::<Amqp10ConnectConfig>(value.clone())
                .map(NodeData::Amqp10Connect)
                .map_err(serde::de::Error::custom),
            "amqp10_send" => serde_json::from_value::<Amqp10SendConfig>(value.clone())
                .map(NodeData::Amqp10Send)
                .map_err(serde::de::Error::custom),
            "amqp10_receive" => serde_json::from_value::<Amqp10ReceiveConfig>(value.clone())
                .map(NodeData::Amqp10Receive)
                .map_err(serde::de::Error::custom),
            "amqp10_accept" => serde_json::from_value::<Amqp10AcceptConfig>(value.clone())
                .map(NodeData::Amqp10Accept)
                .map_err(serde::de::Error::custom),
            "amqp10_reject" => serde_json::from_value::<Amqp10RejectConfig>(value.clone())
                .map(NodeData::Amqp10Reject)
                .map_err(serde::de::Error::custom),
            "amqp10_disconnect" => serde_json::from_value::<Amqp10DisconnectConfig>(value.clone())
                .map(NodeData::Amqp10Disconnect)
                .map_err(serde::de::Error::custom),
            // SMTP 节点
            "smtp_connect" => serde_json::from_value::<SmtpConnectConfig>(value.clone())
                .map(NodeData::SmtpConnect)
                .map_err(serde::de::Error::custom),
            "smtp_send" => serde_json::from_value::<SmtpSendConfig>(value.clone())
                .map(NodeData::SmtpSend)
                .map_err(serde::de::Error::custom),
            "smtp_disconnect" => serde_json::from_value::<SmtpDisconnectConfig>(value.clone())
                .map(NodeData::SmtpDisconnect)
                .map_err(serde::de::Error::custom),
            // IMAP 节点
            "imap_connect" => serde_json::from_value::<ImapConnectConfig>(value.clone())
                .map(NodeData::ImapConnect)
                .map_err(serde::de::Error::custom),
            "imap_select" => serde_json::from_value::<ImapSelectConfig>(value.clone())
                .map(NodeData::ImapSelect)
                .map_err(serde::de::Error::custom),
            "imap_search" => serde_json::from_value::<ImapSearchConfig>(value.clone())
                .map(NodeData::ImapSearch)
                .map_err(serde::de::Error::custom),
            "imap_fetch" => serde_json::from_value::<ImapFetchConfig>(value.clone())
                .map(NodeData::ImapFetch)
                .map_err(serde::de::Error::custom),
            "imap_store_flags" => serde_json::from_value::<ImapStoreFlagsConfig>(value.clone())
                .map(NodeData::ImapStoreFlags)
                .map_err(serde::de::Error::custom),
            "imap_copy" => serde_json::from_value::<ImapCopyConfig>(value.clone())
                .map(NodeData::ImapCopy)
                .map_err(serde::de::Error::custom),
            "imap_move" => serde_json::from_value::<ImapMoveConfig>(value.clone())
                .map(NodeData::ImapMove)
                .map_err(serde::de::Error::custom),
            "imap_expunge" => serde_json::from_value::<ImapExpungeConfig>(value.clone())
                .map(NodeData::ImapExpunge)
                .map_err(serde::de::Error::custom),
            "imap_list" => serde_json::from_value::<ImapListConfig>(value.clone())
                .map(NodeData::ImapList)
                .map_err(serde::de::Error::custom),
            "imap_folder_create" => serde_json::from_value::<ImapFolderCreateConfig>(value.clone())
                .map(NodeData::ImapFolderCreate)
                .map_err(serde::de::Error::custom),
            "imap_folder_delete" => serde_json::from_value::<ImapFolderDeleteConfig>(value.clone())
                .map(NodeData::ImapFolderDelete)
                .map_err(serde::de::Error::custom),
            "imap_idle" => serde_json::from_value::<ImapIdleConfig>(value.clone())
                .map(NodeData::ImapIdle)
                .map_err(serde::de::Error::custom),
            "imap_close" => serde_json::from_value::<ImapCloseConfig>(value.clone())
                .map(NodeData::ImapClose)
                .map_err(serde::de::Error::custom),
            // Redis 节点
            "redis_connect" => serde_json::from_value::<RedisConnectConfig>(value.clone())
                .map(NodeData::RedisConnect)
                .map_err(serde::de::Error::custom),
            "redis_command" => serde_json::from_value::<RedisCommandConfig>(value.clone())
                .map(NodeData::RedisCommand)
                .map_err(serde::de::Error::custom),
            "redis_close" => serde_json::from_value::<RedisCloseConfig>(value.clone())
                .map(NodeData::RedisClose)
                .map_err(serde::de::Error::custom),
            "proxy" => serde_json::from_value::<ProxyConfig>(value.clone())
                .map(NodeData::Proxy)
                .map_err(serde::de::Error::custom),
            "variable_extractor" => serde_json::from_value::<VariableExtractorConfig>(value.clone())
                .map(NodeData::VariableExtractor)
                .map_err(serde::de::Error::custom),
            "end" => Ok(NodeData::End(EndConfig)),
            // 未知 type → 兜底到 Plugin
            _ => Ok(NodeData::Plugin(value)),
        }
    }
}

impl NodeData {
    /// 获取节点类型名称
    pub fn type_name(&self) -> String {
        match self {
            NodeData::Entry(_) => "entry".to_string(),
            NodeData::Http(_) => "http".to_string(),
            NodeData::Conditional(_) => "conditional".to_string(),
            NodeData::WsConnect(_) => "ws_connect".to_string(),
            NodeData::WsSendCollect(_) => "ws_send_collect".to_string(),
            NodeData::WsClose(_) => "ws_close".to_string(),
            NodeData::Assertion(_) => "assertion".to_string(),
            NodeData::DbConnect(_) => "db_connect".to_string(),
            NodeData::DbQuery(_) => "db_query".to_string(),
            NodeData::DbBegin(_) => "db_begin".to_string(),
            NodeData::DbCommit(_) => "db_commit".to_string(),
            NodeData::DbRollback(_) => "db_rollback".to_string(),
            NodeData::DbClose(_) => "db_close".to_string(),
            NodeData::Script(_) => "script".to_string(),
            NodeData::GrpcConnect(_) => "grpc_connect".to_string(),
            NodeData::GrpcCall(_) => "grpc_call".to_string(),
            NodeData::GrpcClose(_) => "grpc_close".to_string(),
            NodeData::McpConnect(_) => "mcp_connect".to_string(),
            NodeData::McpCallTool(_) => "mcp_call_tool".to_string(),
            NodeData::McpReadResource(_) => "mcp_read_resource".to_string(),
            NodeData::McpClose(_) => "mcp_close".to_string(),
            NodeData::McpGetPrompt(_) => "mcp_get_prompt".to_string(),
            NodeData::TcpConnect(_) => "tcp_connect".to_string(),
            NodeData::TcpSend(_) => "tcp_send".to_string(),
            NodeData::TcpReceive(_) => "tcp_receive".to_string(),
            NodeData::TcpClose(_) => "tcp_close".to_string(),
            NodeData::UdpBind(_) => "udp_bind".to_string(),
            NodeData::UdpSendTo(_) => "udp_send_to".to_string(),
            NodeData::UdpRecvFrom(_) => "udp_recv_from".to_string(),
            NodeData::UdpClose(_) => "udp_close".to_string(),
            NodeData::MqttConnect(_) => "mqtt_connect".to_string(),
            NodeData::MqttPublish(_) => "mqtt_publish".to_string(),
            NodeData::MqttSubscribe(_) => "mqtt_subscribe".to_string(),
            NodeData::MqttUnsubscribe(_) => "mqtt_unsubscribe".to_string(),
            NodeData::MqttDisconnect(_) => "mqtt_disconnect".to_string(),
            NodeData::MqttWaitMessage(_) => "mqtt_wait_message".to_string(),
            NodeData::MqttRequest(_) => "mqtt_request".to_string(),
            NodeData::KafkaConnect(_) => "kafka_connect".to_string(),
            NodeData::KafkaProduce(_) => "kafka_produce".to_string(),
            NodeData::KafkaConsume(_) => "kafka_consume".to_string(),
            NodeData::KafkaConsumeBatch(_) => "kafka_consume_batch".to_string(),
            NodeData::KafkaAdmin(_) => "kafka_admin".to_string(),
            NodeData::KafkaSchemaRegistry(_) => "kafka_schema_registry".to_string(),
            NodeData::KafkaClose(_) => "kafka_close".to_string(),
            // SSE 节点
            NodeData::SseConnect(_) => "sse_connect".to_string(),
            NodeData::SseListen(_) => "sse_listen".to_string(),
            NodeData::SseDisconnect(_) => "sse_disconnect".to_string(),
            // GraphQL 节点
            NodeData::GraphqlConnect(_) => "graphql_connect".to_string(),
            NodeData::GraphqlQuery(_) => "graphql_query".to_string(),
            NodeData::GraphqlSubscribe(_) => "graphql_subscribe".to_string(),
            NodeData::GraphqlIntrospect(_) => "graphql_introspect".to_string(),
            NodeData::GraphqlDisconnect(_) => "graphql_disconnect".to_string(),
            // AMQP 0-9-1 节点
            NodeData::AmqpConnect(_) => "amqp_connect".to_string(),
            NodeData::AmqpClose(_) => "amqp_close".to_string(),
            NodeData::AmqpExchangeDeclare(_) => "amqp_exchange_declare".to_string(),
            NodeData::AmqpExchangeDelete(_) => "amqp_exchange_delete".to_string(),
            NodeData::AmqpQueueDeclare(_) => "amqp_queue_declare".to_string(),
            NodeData::AmqpQueueDelete(_) => "amqp_queue_delete".to_string(),
            NodeData::AmqpQueueBind(_) => "amqp_queue_bind".to_string(),
            NodeData::AmqpPublish(_) => "amqp_publish".to_string(),
            NodeData::AmqpConsume(_) => "amqp_consume".to_string(),
            NodeData::AmqpGet(_) => "amqp_get".to_string(),
            NodeData::AmqpAck(_) => "amqp_ack".to_string(),
            NodeData::AmqpManagement(_) => "amqp_management".to_string(),
            // AMQP 1.0 节点
            NodeData::Amqp10Connect(_) => "amqp10_connect".to_string(),
            NodeData::Amqp10Send(_) => "amqp10_send".to_string(),
            NodeData::Amqp10Receive(_) => "amqp10_receive".to_string(),
            NodeData::Amqp10Accept(_) => "amqp10_accept".to_string(),
            NodeData::Amqp10Reject(_) => "amqp10_reject".to_string(),
            NodeData::Amqp10Disconnect(_) => "amqp10_disconnect".to_string(),
            // SMTP 节点
            NodeData::SmtpConnect(_) => "smtp_connect".to_string(),
            NodeData::SmtpSend(_) => "smtp_send".to_string(),
            NodeData::SmtpDisconnect(_) => "smtp_disconnect".to_string(),
            // IMAP 节点
            NodeData::ImapConnect(_) => "imap_connect".to_string(),
            NodeData::ImapSelect(_) => "imap_select".to_string(),
            NodeData::ImapSearch(_) => "imap_search".to_string(),
            NodeData::ImapFetch(_) => "imap_fetch".to_string(),
            NodeData::ImapStoreFlags(_) => "imap_store_flags".to_string(),
            NodeData::ImapCopy(_) => "imap_copy".to_string(),
            NodeData::ImapMove(_) => "imap_move".to_string(),
            NodeData::ImapExpunge(_) => "imap_expunge".to_string(),
            NodeData::ImapList(_) => "imap_list".to_string(),
            NodeData::ImapFolderCreate(_) => "imap_folder_create".to_string(),
            NodeData::ImapFolderDelete(_) => "imap_folder_delete".to_string(),
            NodeData::ImapIdle(_) => "imap_idle".to_string(),
            NodeData::ImapClose(_) => "imap_close".to_string(),
            // Redis 节点
            NodeData::RedisConnect(_) => "redis_connect".to_string(),
            NodeData::RedisCommand(_) => "redis_command".to_string(),
            NodeData::RedisClose(_) => "redis_close".to_string(),
            NodeData::Proxy(_) => "proxy".to_string(),
            NodeData::VariableExtractor(_) => "variable_extractor".to_string(),
            NodeData::End(_) => "end".to_string(),
            NodeData::Plugin(val) => val
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("plugin")
                .to_string(),
        }
    }

    /// 创建空的 Entry 节点配置
    pub fn entry() -> Self {
        Self::Entry(EntryConfig::default())
    }

    /// 创建 Http 节点配置
    pub fn http(url: impl Into<String>) -> Self {
        Self::Http(HttpConfig {
            url: url.into(),
            ..Default::default()
        })
    }

    /// 创建 Conditional 节点配置（简单条件）
    pub fn conditional_simple(
        field_path: impl Into<String>,
        operator: ConditionOperator,
        expected_value: serde_json::Value,
    ) -> Self {
        Self::Conditional(ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some(field_path.into()),
            operator: Some(operator),
            expected_value: Some(expected_value),
            advanced_expression: None,
            expected_value_type: None,
            source_node_id: None,
        })
    }

    /// 创建 Conditional 节点配置（高级条件）
    pub fn conditional_advanced(expression: impl Into<String>) -> Self {
        Self::Conditional(ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some(expression.into()),
            expected_value_type: None,
            source_node_id: None,
        })
    }

    /// 创建 `WsConnect` 节点配置
    pub fn ws_connect(url: impl Into<String>) -> Self {
        Self::WsConnect(WsConnectConfig {
            url: url.into(),
            ..Default::default()
        })
    }

    /// 创建 `WsSendCollect` 节点配置
    pub fn ws_send_collect(connection_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::WsSendCollect(WsSendCollectConfig {
            connection_id: connection_id.into(),
            message: message.into(),
            ..Default::default()
        })
    }

    /// 创建 `WsClose` 节点配置
    pub fn ws_close(connection_id: impl Into<String>) -> Self {
        Self::WsClose(WsCloseConfig {
            connection_id: connection_id.into(),
            ..Default::default()
        })
    }

    /// 创建 `GrpcConnect` 节点配置
    pub fn grpc_connect(url: impl Into<String>) -> Self {
        Self::GrpcConnect(GrpcConnectConfig {
            url: url.into(),
            ..Default::default()
        })
    }

    /// 创建 `GrpcCall` 节点配置
    pub fn grpc_call(connection_id: impl Into<String>, service_name: impl Into<String>, method_name: impl Into<String>) -> Self {
        Self::GrpcCall(GrpcCallConfig {
            connection_id: connection_id.into(),
            service_name: service_name.into(),
            method_name: method_name.into(),
            ..Default::default()
        })
    }

    /// 创建 `GrpcClose` 节点配置
    pub fn grpc_close(connection_id: impl Into<String>) -> Self {
        Self::GrpcClose(GrpcCloseConfig {
            connection_id: connection_id.into(),
        })
    }

    /// 创建 MCP Connect 节点配置
    pub fn mcp_connect(url: impl Into<String>) -> Self {
        Self::McpConnect(McpConnectConfig {
            url: url.into(),
            ..Default::default()
        })
    }

    /// 创建 MCP Call Tool 节点配置
    pub fn mcp_call_tool(connection_id: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self::McpCallTool(McpCallToolConfig {
            connection_id: connection_id.into(),
            tool_name: tool_name.into(),
            ..Default::default()
        })
    }

    /// 创建 MCP Read Resource 节点配置
    pub fn mcp_read_resource(connection_id: impl Into<String>, resource_uri: impl Into<String>) -> Self {
        Self::McpReadResource(McpReadResourceConfig {
            connection_id: connection_id.into(),
            resource_uri: resource_uri.into(),
        })
    }

    /// 创建 MCP Close 节点配置
    pub fn mcp_close(connection_id: impl Into<String>) -> Self {
        Self::McpClose(McpCloseConfig {
            connection_id: connection_id.into(),
        })
    }

    /// 创建 MCP Get Prompt 节点配置
    pub fn mcp_get_prompt(connection_id: impl Into<String>, prompt_name: impl Into<String>) -> Self {
        Self::McpGetPrompt(McpGetPromptConfig {
            connection_id: connection_id.into(),
            prompt_name: prompt_name.into(),
            arguments: std::collections::HashMap::new(),
        })
    }

    /// 创建 Assertion 节点配置
    pub fn assertion() -> Self {
        Self::Assertion(AssertionConfig::default())
    }

    /// 创建 `DbConnect` 节点配置
    pub fn db_connect(connection_uuid: impl Into<String>) -> Self {
        Self::DbConnect(DbConnectConfig {
            connection_uuid: connection_uuid.into(),
            driver: None,
            connection_string: None,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            min_connections: default_min_connections(),
            max_connections: default_max_connections(),
            test_query: None,
            connect_timeout_ms: None,
        })
    }

    /// 创建 `DbQuery` 节点配置
    pub fn db_query(connection_uuid: impl Into<String>) -> Self {
        Self::DbQuery(DbQueryConfig {
            connection_uuid: connection_uuid.into(),
            ..Default::default()
        })
    }

    /// 创建 `DbBegin` 节点配置
    pub fn db_begin(connection_uuid: impl Into<String>) -> Self {
        Self::DbBegin(DbBeginConfig {
            connection_uuid: connection_uuid.into(),
            ..Default::default()
        })
    }

    /// 创建 `DbCommit` 节点配置
    pub fn db_commit(connection_uuid: impl Into<String>) -> Self {
        Self::DbCommit(DbCommitConfig {
            connection_uuid: connection_uuid.into(),
        })
    }

    /// 创建 `DbRollback` 节点配置
    pub fn db_rollback(connection_uuid: impl Into<String>) -> Self {
        Self::DbRollback(DbRollbackConfig {
            connection_uuid: connection_uuid.into(),
        })
    }

    /// 创建 `DbClose` 节点配置
    pub fn db_close(connection_uuid: impl Into<String>) -> Self {
        Self::DbClose(DbCloseConfig {
            connection_uuid: connection_uuid.into(),
        })
    }

    /// 创建 Script 节点配置
    pub fn script(script: impl Into<String>) -> Self {
        Self::Script(ScriptNodeConfig {
            script: script.into(),
            ..Default::default()
        })
    }

    /// 创建 `VariableExtractor` 节点配置
    pub fn variable_extractor() -> Self {
        Self::VariableExtractor(VariableExtractorConfig::default())
    }

    /// 创建 `Proxy` 节点配置
    pub fn proxy() -> Self {
        Self::Proxy(ProxyConfig::default())
    }

    /// 创建空的 End 节点配置
    pub fn end() -> Self {
        Self::End(EndConfig)
    }
}

/// 为 `NodeData` 实现 Default，使用 Entry 作为默认值
impl Default for NodeData {
    fn default() -> Self {
        Self::entry()
    }
}

// ============================================================================
// 辅助宏和函数
// ============================================================================

/// 为 HTTP 配置添加请求头的便捷宏
#[macro_export]
macro_rules! http_config {
    ($url:expr) => {
        NodeData::http($url)
    };
    ($url:expr, headers: { $($key:expr => $val:expr),* $(,)? }) => {{
        let mut headers_map = HashMap::new();
        $(
            headers_map.insert($key.to_string(), $val.to_string());
        )*
        NodeData::Http(HttpConfig {
            url: $url.to_string(),
            headers: if headers_map.is_empty() { None } else { Some(headers_map) },
            ..Default::default()
        })
    }};
    ($url:expr, method: $method:expr) => {
        NodeData::Http(HttpConfig {
            url: $url.to_string(),
            method: $method,
            ..Default::default()
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_entry_config_serialization() {
        let config = EntryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_entry_config_empty_structure() {
        // 测试 EntryConfig 默认值
        let config = EntryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, "{}"); // initial_variables 为空时跳过序列化
    }

    #[test]
    fn test_entry_config_default_impl() {
        // 测试 Default impl
        let config = EntryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_http_config_serialization() {
        let config = HttpConfig {
            url: "https://api.example.com/users".to_string(),
            method: HttpMethod::Get,
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("https://api.example.com/users"));
        assert!(json.contains("get"));

        // 反序列化测试
        let deserialized: HttpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, config.url);
        assert_eq!(deserialized.method, HttpMethod::Get);
    }

    #[test]
    fn test_conditional_config_simple() {
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("http_response.status".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(json!(200)),
            advanced_expression: None,
            expected_value_type: None,
            source_node_id: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("simple"));
        assert!(json.contains("http_response.status"));
        assert!(json.contains("equal"));
        assert!(json.contains("200"));
    }

    #[test]
    fn test_conditional_config_advanced() {
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some(
                "http_response.status >= 200 && http_response.status < 300".to_string(),
            ),
            expected_value_type: None,
            source_node_id: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("advanced"));
        assert!(json.contains("http_response.status >= 200"));
    }

    #[test]
    fn test_node_data_type_name() {
        assert_eq!(NodeData::entry().type_name(), "entry");
        assert_eq!(NodeData::http("https://example.com").type_name(), "http");
        assert_eq!(
            NodeData::conditional_simple("field", ConditionOperator::Equal, json!(1)).type_name(),
            "conditional"
        );
    }

    #[test]
    fn test_node_data_serialization_with_tag() {
        let data = NodeData::http("https://api.example.com");
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""type":"http""#));
        assert!(json.contains("https://api.example.com"));

        // 反序列化测试
        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        match deserialized {
            NodeData::Http(config) => {
                assert_eq!(config.url, "https://api.example.com");
                assert_eq!(config.method, HttpMethod::Get);
            }
            _ => panic!("Expected Http variant"),
        }
    }

    #[test]
    fn test_http_config_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer xxx".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let config = HttpConfig {
            url: "https://api.example.com".to_string(),
            method: HttpMethod::Post,
            headers: Some(headers),
            body: Some(r#"{\"key\": \"value\"}"#.to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("Authorization"));
        assert!(json.contains("Bearer xxx"));
        assert!(json.contains("Content-Type"));
        assert!(json.contains("application/json"));
        assert!(json.contains("post"));
        assert!(json.contains("key"));
        assert!(json.contains("value"));
    }

    // ========================================================================
    // WebSocket 类型测试
    // ========================================================================

    #[test]
    fn test_ws_collect_mode_default() {
        let mode = WsCollectMode::default();
        assert_eq!(mode, WsCollectMode::FirstMatch);
    }

    #[test]
    fn test_ws_collect_mode_serialization() {
        let mode = WsCollectMode::FirstMatch;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, r#""first_match""#);

        let deserialized: WsCollectMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, WsCollectMode::FirstMatch);
    }

    #[test]
    fn test_ws_match_type_default() {
        let match_type = WsMatchType::default();
        assert_eq!(match_type, WsMatchType::JsonPath);
    }

    #[test]
    fn test_ws_message_type_default() {
        let msg_type = WsMessageType::default();
        assert_eq!(msg_type, WsMessageType::Text);
    }

    #[test]
    fn test_ws_message_type_serialization() {
        let msg_type = WsMessageType::Binary;
        let json = serde_json::to_string(&msg_type).unwrap();
        assert_eq!(json, r#""binary""#);

        let deserialized: WsMessageType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, WsMessageType::Binary);
    }

    #[test]
    fn test_ws_collect_condition() {
        let condition = WsCollectCondition {
            match_type: WsMatchType::Regex,
            expression: r#"status == 200"#.to_string(),
        };

        let json = serde_json::to_string(&condition).unwrap();
        assert!(json.contains("regex"));
        assert!(json.contains("status == 200"));

        let deserialized: WsCollectCondition = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.match_type, WsMatchType::Regex);
        assert_eq!(deserialized.expression, "status == 200");
    }

    #[test]
    fn test_ws_connect_config_default() {
        let config = WsConnectConfig::default();
        assert!(config.url.is_empty());
        assert!(config.headers.is_empty());
        assert!(config.subprotocols.is_empty());
        assert_eq!(config.connect_timeout_ms, 30000);
        assert!(!config.collect_message);
        assert_eq!(config.collect_mode, WsCollectMode::FirstMatch);
        assert!(config.collect_condition.is_none());
        assert_eq!(config.collect_timeout_ms, 10000);
    }

    #[test]
    fn test_ws_connect_config_serialization() {
        let config = WsConnectConfig {
            url: "wss://example.com/ws".to_string(),
            headers: vec![FormField {
                key: "Authorization".to_string(),
                value: "Bearer token".to_string(),
                field_type: FormFieldType::Text,
                file_path: None,
            }],
            subprotocols: vec!["chat".to_string()],
            connect_timeout_ms: 5000,
            collect_message: true,
            collect_mode: WsCollectMode::FirstMatch,
            collect_condition: None,
            collect_timeout_ms: 15000,
            use_tls: false,
            tls_skip_verify: false,
            ping_interval_ms: None,
            ping_timeout_ms: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("wss://example.com/ws"));
        assert!(json.contains("Authorization"));
        assert!(json.contains("chat"));
        assert!(json.contains("first_match"));

        let deserialized: WsConnectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, "wss://example.com/ws");
        assert_eq!(deserialized.headers.len(), 1);
        assert_eq!(deserialized.subprotocols.len(), 1);
        assert_eq!(deserialized.connect_timeout_ms, 5000);
    }

    #[test]
    fn test_ws_send_collect_config_default() {
        let config = WsSendCollectConfig::default();
        assert!(config.connection_id.is_empty());
        assert!(config.message.is_empty());
        assert_eq!(config.message_type, WsMessageType::Text);
        assert_eq!(config.send_timeout_ms, 10000);
        assert!(config.collect_response);
        assert_eq!(config.collect_mode, WsCollectMode::FirstMatch);
        assert_eq!(config.collect_timeout_ms, 10000);
    }

    #[test]
    fn test_ws_send_collect_config_serialization() {
        let config = WsSendCollectConfig {
            connection_id: "conn-123".to_string(),
            message: r#"{"type":"ping"}"#.to_string(),
            message_type: WsMessageType::Text,
            send_timeout_ms: 5000,
            collect_response: true,
            collect_mode: WsCollectMode::ConditionMatch,
            collect_condition: Some(WsCollectCondition {
                match_type: WsMatchType::JsonPath,
                expression: "$.type == 'pong'".to_string(),
            }),
            collect_timeout_ms: 8000,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("conn-123"));
        assert!(json.contains("ping"));
        assert!(json.contains("condition_match"));
        assert!(json.contains("$.type == 'pong'"));

        let deserialized: WsSendCollectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.connection_id, "conn-123");
        assert_eq!(deserialized.collect_mode, WsCollectMode::ConditionMatch);
        assert!(deserialized.collect_condition.is_some());
    }

    #[test]
    fn test_ws_close_config_default() {
        let config = WsCloseConfig::default();
        assert!(config.connection_id.is_empty());
        assert_eq!(config.close_code, 1000);
        assert!(config.close_reason.is_none());
        assert_eq!(config.close_timeout_ms, 5000);
    }

    #[test]
    fn test_ws_close_config_serialization() {
        let config = WsCloseConfig {
            connection_id: "conn-456".to_string(),
            close_code: 1001,
            close_reason: Some("Going Away".to_string()),
            close_timeout_ms: 3000,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("conn-456"));
        assert!(json.contains("1001"));
        assert!(json.contains("Going Away"));

        let deserialized: WsCloseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.close_code, 1001);
        assert_eq!(deserialized.close_reason, Some("Going Away".to_string()));
    }

    #[test]
    fn test_node_data_ws_connect() {
        let data = NodeData::ws_connect("wss://example.com/ws");
        assert_eq!(data.type_name(), "ws_connect");

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""type":"ws_connect""#));
        assert!(json.contains("wss://example.com/ws"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        match deserialized {
            NodeData::WsConnect(config) => {
                assert_eq!(config.url, "wss://example.com/ws");
            }
            _ => panic!("Expected WsConnect variant"),
        }
    }

    #[test]
    fn test_node_data_ws_send_collect() {
        let data = NodeData::ws_send_collect("conn-123", r#"{"type":"ping"}"#);
        assert_eq!(data.type_name(), "ws_send_collect");

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""type":"ws_send_collect""#));
        assert!(json.contains("conn-123"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        match deserialized {
            NodeData::WsSendCollect(config) => {
                assert_eq!(config.connection_id, "conn-123");
                assert_eq!(config.message, r#"{"type":"ping"}"#);
            }
            _ => panic!("Expected WsSendCollect variant"),
        }
    }

    #[test]
    fn test_node_data_ws_close() {
        let data = NodeData::ws_close("conn-456");
        assert_eq!(data.type_name(), "ws_close");

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""type":"ws_close""#));
        assert!(json.contains("conn-456"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        match deserialized {
            NodeData::WsClose(config) => {
                assert_eq!(config.connection_id, "conn-456");
                assert_eq!(config.close_code, 1000); // default
            }
            _ => panic!("Expected WsClose variant"),
        }
    }

    // ========================================================================
    // Script 节点测试
    // ========================================================================

    #[test]
    fn test_script_execution_phase_default() {
        let phase = ScriptExecutionPhase::default();
        assert_eq!(phase, ScriptExecutionPhase::PostRequest);
    }

    #[test]
    fn test_script_execution_phase_serialization() {
        let phase = ScriptExecutionPhase::PreRequest;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, r#""pre_request""#);

        let deserialized: ScriptExecutionPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ScriptExecutionPhase::PreRequest);
    }

    #[test]
    fn test_script_error_strategy_default() {
        let strategy = ScriptErrorStrategy::default();
        assert_eq!(strategy, ScriptErrorStrategy::Recoverable);
    }

    #[test]
    fn test_assertion_fail_strategy_default() {
        let strategy = AssertionFailStrategy::default();
        assert_eq!(strategy, AssertionFailStrategy::RouteFalse);
    }

    #[test]
    fn test_script_node_config_default() {
        let config = ScriptNodeConfig::default();
        assert!(config.script.is_empty());
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.memory_limit_mb, 10);
        assert_eq!(config.execution_phase, ScriptExecutionPhase::PostRequest);
        assert_eq!(config.on_error, ScriptErrorStrategy::Recoverable);
    }

    #[test]
    fn test_script_node_config_serialization() {
        let config = ScriptNodeConfig {
            script: "return status === 200;".to_string(),
            timeout_ms: 10000,
            memory_limit_mb: 20,
            execution_phase: ScriptExecutionPhase::Standalone,
            on_error: ScriptErrorStrategy::Fatal,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("return status === 200;"));
        assert!(json.contains("10000"));
        assert!(json.contains("20"));
        assert!(json.contains("standalone"));
        assert!(json.contains("fatal"));

        let deserialized: ScriptNodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.script, "return status === 200;");
        assert_eq!(deserialized.timeout_ms, 10000);
        assert_eq!(deserialized.memory_limit_mb, 20);
        assert_eq!(
            deserialized.execution_phase,
            ScriptExecutionPhase::Standalone
        );
        assert_eq!(deserialized.on_error, ScriptErrorStrategy::Fatal);
    }

    #[test]
    fn test_node_data_script() {
        let data = NodeData::script("console.log('hello');");
        assert_eq!(data.type_name(), "script");

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""type":"script""#));
        assert!(json.contains("console.log('hello');"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        match deserialized {
            NodeData::Script(config) => {
                assert_eq!(config.script, "console.log('hello');");
                assert_eq!(config.timeout_ms, 5000); // default
            }
            _ => panic!("Expected Script variant"),
        }
    }

    // ========================================================================
    // Plugin 兜底变体测试
    // ========================================================================

    #[test]
    fn test_node_data_plugin_catch_all() {
        // 未知 type 字符串应反序列化为 Plugin 变体
        let json = r#"{"type":"grpc_request","url":"http://example.com"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        match data {
            NodeData::Plugin(val) => {
                // 原始 JSON（含 type 字段）应完整保留
                assert_eq!(
                    val.get("type").unwrap().as_str(),
                    Some("grpc_request")
                );
                assert_eq!(
                    val.get("url").unwrap().as_str(),
                    Some("http://example.com")
                );
            }
            _ => panic!("Expected Plugin variant for unknown type"),
        }
    }

    #[test]
    fn test_node_data_plugin_type_name() {
        // Plugin 变体应从 JSON 的 "type" 字段提取类型名称
        let json = r#"{"type":"custom_node","foo":"bar"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "custom_node");

        // 无 "type" 字段时回退到 "plugin"
        let json = r#"{"foo":"bar"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "plugin");
    }

    #[test]
    fn test_node_data_existing_types_unchanged() {
        // HTTP 应正常反序列化
        let json = r#"{"type":"http","url":"https://example.com","method":"get"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        match data {
            NodeData::Http(config) => assert_eq!(config.url, "https://example.com"),
            _ => panic!("Expected Http variant"),
        }

        // ws_connect 应正常反序列化
        let json = r#"{"type":"ws_connect","url":"wss://example.com"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        match data {
            NodeData::WsConnect(config) => assert_eq!(config.url, "wss://example.com"),
            _ => panic!("Expected WsConnect variant"),
        }

        // entry 应正常反序列化
        let json = r#"{"type":"entry"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert!(matches!(data, NodeData::Entry(_)));

        // script 应正常反序列化
        let json = r#"{"type":"script","script":"return 1;"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        match data {
            NodeData::Script(config) => assert_eq!(config.script, "return 1;"),
            _ => panic!("Expected Script variant"),
        }

        // db_connect 应正常反序列化
        let json = r#"{"type":"db_connect","driver":"postgres","connection_string":"postgres://localhost/test"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        match data {
            NodeData::DbConnect(config) => {
                assert_eq!(config.connection_string, Some("postgres://localhost/test".to_string()));
            }
            _ => panic!("Expected DbConnect variant"),
        }
    }

    // ========================================================================
    // B5: .mpf 格式兼容性验证 + 未知节点类型反序列化测试
    // ========================================================================

    #[test]
    fn test_unknown_node_type_deserialize() {
        // 未知节点类型应反序列化为 Plugin 变体，保留完整 JSON
        let json = r#"{"type": "custom_grpc", "endpoint": "localhost:50051"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();

        match &data {
            NodeData::Plugin(val) => {
                assert_eq!(val.get("type").and_then(|v| v.as_str()), Some("custom_grpc"));
                assert_eq!(val.get("endpoint").and_then(|v| v.as_str()), Some("localhost:50051"));
            }
            _ => panic!("Expected Plugin variant for unknown type"),
        }

        // type_name() 应返回 JSON 中的实际类型
        assert_eq!(data.type_name(), "custom_grpc");
    }

    #[test]
    fn test_plugin_type_name_from_json() {
        // Plugin 带 type 字段应返回 type 值
        let json = r#"{"type": "my_custom_node", "some_config": "value"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "my_custom_node");

        // Plugin 无 type 字段应回退到 "plugin"
        let json_no_type = r#"{"config": "value"}"#;
        let data_no_type: NodeData = serde_json::from_str(json_no_type).unwrap();
        assert_eq!(data_no_type.type_name(), "plugin");
    }

    #[test]
    fn test_node_with_unknown_type_roundtrip() {
        use crate::node::{Node, NodePosition};

        // 未知类型节点应正确序列化/反序列化
        let node = Node {
            uuid: "test-uuid".to_string(),
            node_type: "custom_grpc".to_string(),
            name: "Custom Node".to_string(),
            position: NodePosition { x: 100.0, y: 200.0 },
            data: NodeData::Plugin(json!({
                "type": "custom_grpc",
                "endpoint": "localhost:50051"
            })),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        // 序列化
        let json = serde_json::to_string(&node).unwrap();

        // 验证序列化格式保留 type 字段
        assert!(json.contains(r#""type":"custom_grpc""#));

        // 反序列化
        let deserialized: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.node_type, "custom_grpc");
        assert_eq!(deserialized.uuid, "test-uuid");
        assert_eq!(deserialized.name, "Custom Node");
        assert_eq!(deserialized.position.x, 100.0);
        assert_eq!(deserialized.position.y, 200.0);
    }

    #[test]
    fn test_known_node_type_serialization_unchanged() {
        use crate::node::{Node, NodePosition};

        // 已知类型序列化格式不应改变
        let node = Node {
            uuid: "test-http".to_string(),
            node_type: "http".to_string(),
            name: "HTTP Request".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let json = serde_json::to_string(&node).unwrap();

        // 序列化后 JSON 应包含 "type":"http"
        assert!(json.contains(r#""type":"http""#));

        // 往返验证
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_type, "http");
        assert_eq!(back.uuid, "test-http");
    }

    // ========================================================================
    // TCP/UDP 节点 serde 往返测试
    // ========================================================================

    #[test]
    fn test_tcp_connect_config_serde_roundtrip() {
        let config = TcpConnectConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            use_tls: true,
            tls_skip_verify: false,
            connect_timeout_ms: 15000,
            no_delay: false,
            keepalive: None,
            tls_ca_cert: None,
            framing_mode: TcpFramingMode::Delimiter {
                delimiter: "\n".to_string(),
            },
            framing_config: FramingConfig::default(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: TcpConnectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.host, "127.0.0.1");
        assert_eq!(back.port, 8080);
        assert!(back.use_tls);
        assert_eq!(back.connect_timeout_ms, 15000);
    }

    #[test]
    fn test_tcp_send_config_serde_roundtrip() {
        let config = TcpSendConfig {
            connection_id: "conn-1".to_string(),
            data: "hello".to_string(),
            data_encoding: "text".to_string(),
            framing_mode: TcpFramingMode::Manual,
            framing_config: FramingConfig::default(),
            send_timeout_ms: 5000,
            parser_config: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: TcpSendConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.connection_id, "conn-1");
        assert_eq!(back.data, "hello");
        assert_eq!(back.send_timeout_ms, 5000);
    }

    #[test]
    fn test_tcp_close_config_serde_roundtrip() {
        let config = TcpCloseConfig {
            connection_id: "conn-1".to_string(),
            close_timeout_ms: 3000,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: TcpCloseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.connection_id, "conn-1");
        assert_eq!(back.close_timeout_ms, 3000);
    }

    #[test]
    fn test_udp_bind_config_serde_roundtrip() {
        let config = UdpBindConfig {
            local_addr: "0.0.0.0".to_string(),
            local_port: 9090,
            reuse_addr: true,
            broadcast: false,
            multicast_group: None,
            multicast_ttl: None,
            multicast_loop: None,
            multicast_interface: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: UdpBindConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.local_addr, "0.0.0.0");
        assert_eq!(back.local_port, 9090);
        assert!(back.reuse_addr);
    }

    #[test]
    fn test_udp_send_to_config_serde_roundtrip() {
        let config = UdpSendToConfig {
            connection_id: "conn-1".to_string(),
            data: "ping".to_string(),
            data_encoding: "hex".to_string(),
            target_addr: "192.168.1.1".to_string(),
            target_port: 5000,
            send_timeout_ms: 10000,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: UdpSendToConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.connection_id, "conn-1");
        assert_eq!(back.data_encoding, "hex");
        assert_eq!(back.target_addr, "192.168.1.1");
        assert_eq!(back.target_port, 5000);
    }

    #[test]
    fn test_udp_recv_from_config_serde_roundtrip() {
        let config = UdpRecvFromConfig {
            connection_id: "conn-1".to_string(),
            receive_timeout_ms: 8000,
            max_size: 32768,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: UdpRecvFromConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.connection_id, "conn-1");
        assert_eq!(back.receive_timeout_ms, 8000);
        assert_eq!(back.max_size, 32768);
    }

    #[test]
    fn test_udp_close_config_serde_roundtrip() {
        let config = UdpCloseConfig {
            connection_id: "conn-1".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: UdpCloseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.connection_id, "conn-1");
    }

    #[test]
    fn test_node_data_tcp_connect_roundtrip() {
        let json = r#"{"type":"tcp_connect","host":"127.0.0.1","port":8080,"use_tls":false,"tls_skip_verify":false,"connect_timeout_ms":30000,"framing_mode":{"length_prefix":{"prefix_bytes":4}},"framing_config":{"mode":{"length_prefix":{"prefix_bytes":4}},"max_frame_size":1048576,"include_framing_in_data":false}}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "tcp_connect");
        match &data {
            NodeData::TcpConnect(cfg) => {
                assert_eq!(cfg.host, "127.0.0.1");
                assert_eq!(cfg.port, 8080);
            }
            _ => panic!("Expected TcpConnect variant"),
        }
        // 往返验证
        let serialized = serde_json::to_string(&data).unwrap();
        let back: NodeData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(back.type_name(), "tcp_connect");
    }

    #[test]
    fn test_node_data_tcp_send_roundtrip() {
        let json = r#"{"type":"tcp_send","connection_id":"c1","data":"hello","data_encoding":"text","send_timeout_ms":30000}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "tcp_send");
        match &data {
            NodeData::TcpSend(cfg) => {
                assert_eq!(cfg.connection_id, "c1");
                assert_eq!(cfg.data, "hello");
            }
            _ => panic!("Expected TcpSend variant"),
        }
    }

    #[test]
    fn test_node_data_tcp_close_roundtrip() {
        let json = r#"{"type":"tcp_close","connection_id":"c1","close_timeout_ms":5000}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "tcp_close");
        match &data {
            NodeData::TcpClose(cfg) => {
                assert_eq!(cfg.connection_id, "c1");
            }
            _ => panic!("Expected TcpClose variant"),
        }
    }

    #[test]
    fn test_node_data_udp_bind_roundtrip() {
        let json = r#"{"type":"udp_bind","local_addr":"0.0.0.0","local_port":9090,"reuse_addr":true}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "udp_bind");
        match &data {
            NodeData::UdpBind(cfg) => {
                assert_eq!(cfg.local_port, 9090);
            }
            _ => panic!("Expected UdpBind variant"),
        }
    }

    #[test]
    fn test_node_data_udp_send_to_roundtrip() {
        let json = r#"{"type":"udp_send_to","connection_id":"c1","data":"ping","target_addr":"10.0.0.1","target_port":5000,"send_timeout_ms":30000}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "udp_send_to");
        match &data {
            NodeData::UdpSendTo(cfg) => {
                assert_eq!(cfg.target_addr, "10.0.0.1");
                assert_eq!(cfg.target_port, 5000);
            }
            _ => panic!("Expected UdpSendTo variant"),
        }
    }

    #[test]
    fn test_node_data_udp_recv_from_roundtrip() {
        let json = r#"{"type":"udp_recv_from","connection_id":"c1","receive_timeout_ms":5000,"max_size":65536}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "udp_recv_from");
        match &data {
            NodeData::UdpRecvFrom(cfg) => {
                assert_eq!(cfg.receive_timeout_ms, 5000);
                assert_eq!(cfg.max_size, 65536);
            }
            _ => panic!("Expected UdpRecvFrom variant"),
        }
    }

    #[test]
    fn test_node_data_udp_close_roundtrip() {
        let json = r#"{"type":"udp_close","connection_id":"c1"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.type_name(), "udp_close");
        match &data {
            NodeData::UdpClose(cfg) => {
                assert_eq!(cfg.connection_id, "c1");
            }
            _ => panic!("Expected UdpClose variant"),
        }
    }

    #[test]
    fn test_tcp_udp_defaults() {
        // TCP Connect 默认值
        let cfg = TcpConnectConfig::default();
        assert_eq!(cfg.connect_timeout_ms, 30000);
        assert!(!cfg.use_tls);

        // TCP Send 默认值
        let cfg = TcpSendConfig::default();
        assert_eq!(cfg.data_encoding, "text");
        assert_eq!(cfg.send_timeout_ms, 30000);

        // TCP Close 默认值
        let cfg = TcpCloseConfig::default();
        assert_eq!(cfg.close_timeout_ms, 5000);

        // UDP Bind 默认值
        let cfg = UdpBindConfig::default();
        assert_eq!(cfg.local_addr, "0.0.0.0");
        assert!(cfg.reuse_addr);

        // UDP SendTo 默认值
        let cfg = UdpSendToConfig::default();
        assert_eq!(cfg.send_timeout_ms, 30000);

        // UDP RecvFrom 默认值
        let cfg = UdpRecvFromConfig::default();
        assert_eq!(cfg.receive_timeout_ms, 5000);
        assert_eq!(cfg.max_size, 65536);

        // UDP Close 默认值
        let cfg = UdpCloseConfig::default();
        assert!(cfg.connection_id.is_empty());
    }

    // ========================================================================
    // EntryConfig dataset 字段向后兼容测试
    // ========================================================================

    #[test]
    fn test_entry_config_backward_compat_without_dataset_fields() {
        // 旧格式 JSON（无 dataset_path/dataset_encoding）应正常反序列化
        let json = r#"{"initial_variables":[]}"#;
        let config: EntryConfig = serde_json::from_str(json).unwrap();
        assert!(config.initial_variables.is_empty());
        assert_eq!(config.dataset_path, None);
        assert_eq!(config.dataset_encoding, None);
    }

    #[test]
    fn test_entry_config_with_dataset_fields() {
        // 新格式 JSON（含 dataset_path/dataset_encoding）应正常反序列化
        let json = r#"{"initial_variables":[],"dataset_path":"/path/file.csv","dataset_encoding":"utf-8"}"#;
        let config: EntryConfig = serde_json::from_str(json).unwrap();
        assert!(config.initial_variables.is_empty());
        assert_eq!(config.dataset_path, Some("/path/file.csv".to_string()));
        assert_eq!(config.dataset_encoding, Some("utf-8".to_string()));
    }

    #[test]
    fn test_entry_config_default_dataset_fields_none() {
        // Default 应将新字段设为 None
        let config = EntryConfig::default();
        assert!(config.initial_variables.is_empty());
        assert_eq!(config.dataset_path, None);
        assert_eq!(config.dataset_encoding, None);
    }

    #[test]
    fn test_entry_config_skip_serializing_none_dataset() {
        // dataset_path/dataset_encoding 为 None 时不应出现在序列化输出中
        let config = EntryConfig {
            initial_variables: vec![],
            dataset_path: None,
            dataset_encoding: None,
            hooks: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("dataset_path"));
        assert!(!json.contains("dataset_encoding"));
    }

    #[test]
    fn test_entry_config_serializes_dataset_fields_when_set() {
        // dataset_path/dataset_encoding 有值时应出现在序列化输出中
        let config = EntryConfig {
            initial_variables: vec![],
            dataset_path: Some("/data/test.csv".to_string()),
            dataset_encoding: Some("gbk".to_string()),
            hooks: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("dataset_path"));
        assert!(json.contains("/data/test.csv"));
        assert!(json.contains("dataset_encoding"));
        assert!(json.contains("gbk"));
    }

    #[test]
    fn test_entry_config_only_dataset_path() {
        // 仅设置 dataset_path，不设置 dataset_encoding
        let json = r#"{"dataset_path":"/path/data.csv"}"#;
        let config: EntryConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.dataset_path, Some("/path/data.csv".to_string()));
        assert_eq!(config.dataset_encoding, None);
        assert!(config.initial_variables.is_empty());
    }

    // ========================================================================
    // SMTP/IMAP 节点配置序列化往返测试
    // ========================================================================

    #[test]
    fn test_smtp_connect_roundtrip() {
        let data = NodeData::SmtpConnect(SmtpConnectConfig {
            host: "smtp.gmail.com".to_string(),
            port: 465,
            tls_mode: "implicit".to_string(),
            auth: None,
            connect_timeout_ms: 30000,
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"smtp_connect\""));
        assert!(json.contains("smtp.gmail.com"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "smtp_connect");
    }

    #[test]
    fn test_smtp_send_roundtrip() {
        let data = NodeData::SmtpSend(SmtpSendConfig {
            connection_id: "conn-1".to_string(),
            from: Some("sender@example.com".to_string()),
            to: vec!["recipient@example.com".to_string()],
            cc: None,
            bcc: None,
            subject: Some("Test Subject".to_string()),
            body_type: "plain_text".to_string(),
            body: Some("Hello World".to_string()),
            html_body: None,
            attachments: None,
            timeout_ms: 30000,
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"smtp_send\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "smtp_send");
    }

    #[test]
    fn test_smtp_disconnect_roundtrip() {
        let data = NodeData::SmtpDisconnect(SmtpDisconnectConfig {
            connection_id: "conn-1".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"smtp_disconnect\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "smtp_disconnect");
    }

    #[test]
    fn test_imap_connect_roundtrip() {
        let data = NodeData::ImapConnect(ImapConnectConfig {
            host: "imap.gmail.com".to_string(),
            port: 993,
            tls_mode: "implicit".to_string(),
            auth: None,
            connect_timeout_ms: 30000,
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_connect\""));
        assert!(json.contains("imap.gmail.com"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_connect");
    }

    #[test]
    fn test_imap_select_roundtrip() {
        let data = NodeData::ImapSelect(ImapSelectConfig {
            connection_id: "conn-1".to_string(),
            mailbox: "INBOX".to_string(),
            readonly: false,
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_select\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_select");
    }

    #[test]
    fn test_imap_search_roundtrip() {
        let data = NodeData::ImapSearch(ImapSearchConfig {
            connection_id: "conn-1".to_string(),
            mailbox: Some("INBOX".to_string()),
            criteria: "UNSEEN".to_string(),
            max_results: Some(10),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_search\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_search");
    }

    #[test]
    fn test_imap_fetch_roundtrip() {
        let data = NodeData::ImapFetch(ImapFetchConfig {
            connection_id: "conn-1".to_string(),
            uids: "1:10".to_string(),
            fetch_items: "(ENVELOPE BODY.PEEK[] FLAGS)".to_string(),
            max_fetch_count: 50,
            max_size_bytes: 5 * 1024 * 1024,
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_fetch\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_fetch");
    }

    #[test]
    fn test_imap_store_flags_roundtrip() {
        let data = NodeData::ImapStoreFlags(ImapStoreFlagsConfig {
            connection_id: "conn-1".to_string(),
            uids: "1,2,3".to_string(),
            operation: "add".to_string(),
            flags: vec!["\\Seen".to_string(), "\\Flagged".to_string()],
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_store_flags\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_store_flags");
    }

    #[test]
    fn test_imap_copy_roundtrip() {
        let data = NodeData::ImapCopy(ImapCopyConfig {
            connection_id: "conn-1".to_string(),
            uids: "1:5".to_string(),
            target_mailbox: "Archive".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_copy\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_copy");
    }

    #[test]
    fn test_imap_move_roundtrip() {
        let data = NodeData::ImapMove(ImapMoveConfig {
            connection_id: "conn-1".to_string(),
            uids: "1:3".to_string(),
            target_mailbox: "Trash".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_move\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_move");
    }

    #[test]
    fn test_imap_expunge_roundtrip() {
        let data = NodeData::ImapExpunge(ImapExpungeConfig {
            connection_id: "conn-1".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_expunge\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_expunge");
    }

    #[test]
    fn test_imap_list_roundtrip() {
        let data = NodeData::ImapList(ImapListConfig {
            connection_id: "conn-1".to_string(),
            reference_name: String::new(),
            pattern: "*".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_list\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_list");
    }

    #[test]
    fn test_imap_folder_create_roundtrip() {
        let data = NodeData::ImapFolderCreate(ImapFolderCreateConfig {
            connection_id: "conn-1".to_string(),
            folder_name: "NewFolder".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_folder_create\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_folder_create");
    }

    #[test]
    fn test_imap_folder_delete_roundtrip() {
        let data = NodeData::ImapFolderDelete(ImapFolderDeleteConfig {
            connection_id: "conn-1".to_string(),
            folder_name: "OldFolder".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_folder_delete\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_folder_delete");
    }

    #[test]
    fn test_imap_idle_roundtrip() {
        let data = NodeData::ImapIdle(ImapIdleConfig {
            connection_id: "conn-1".to_string(),
            mailbox: Some("INBOX".to_string()),
            timeout_seconds: 300,
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_idle\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_idle");
    }

    #[test]
    fn test_imap_close_roundtrip() {
        let data = NodeData::ImapClose(ImapCloseConfig {
            connection_id: "conn-1".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"imap_close\""));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.type_name(), "imap_close");
    }
}
