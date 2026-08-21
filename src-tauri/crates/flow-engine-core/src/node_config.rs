// 节点配置模块 - 强类型节点配置定义

use crate::traits::{FramingConfig, GraphqlQueryTemplate, TcpFramingMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// ============================================================================
// Entry node配置
// ============================================================================

/// 入口节点配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntryConfig {
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
    #[serde(default)]
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

/// Failed行为
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnFailureMode {
    /// 路由到 false 端口
    #[default]
    RouteFalse,
    /// 抛出Error
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
    /// Failed行为
    #[serde(default)]
    pub on_failure: OnFailureMode,
    /// Failed后是否继续执行后续断言
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
    /// Error信息
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
    /// Failed数
    pub failed_count: usize,
    /// 所有断言结果
    pub results: Vec<AssertionResult>,
    /// Failed的断言
    pub failed_assertions: Vec<AssertionResult>,
    /// 输入数据
    pub input: serde_json::Value,
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

/// 脚本Error处理策略
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptErrorStrategy {
    /// 可恢复Error → 路由到 false 端口（默认）
    #[default]
    Recoverable,
    /// 致命Error → 终止整个流程
    Fatal,
}

/// 断言Failed处理策略
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
    /// Error处理策略
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

fn default_true() -> bool {
    true
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

/// 代理模式
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
#[allow(clippy::large_enum_variant)]
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
    /// 脚本节点配置
    #[serde(rename = "script")]
    Script(ScriptNodeConfig),
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
        let type_str = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

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
            "script" => serde_json::from_value::<ScriptNodeConfig>(value.clone())
                .map(NodeData::Script)
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
            "graphql_introspect" => {
                serde_json::from_value::<GraphqlIntrospectConfig>(value.clone())
                    .map(NodeData::GraphqlIntrospect)
                    .map_err(serde::de::Error::custom)
            }
            "graphql_disconnect" => {
                serde_json::from_value::<GraphqlDisconnectConfig>(value.clone())
                    .map(NodeData::GraphqlDisconnect)
                    .map_err(serde::de::Error::custom)
            }
            "proxy" => serde_json::from_value::<ProxyConfig>(value.clone())
                .map(NodeData::Proxy)
                .map_err(serde::de::Error::custom),
            "variable_extractor" => {
                serde_json::from_value::<VariableExtractorConfig>(value.clone())
                    .map(NodeData::VariableExtractor)
                    .map_err(serde::de::Error::custom)
            }
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
            NodeData::Script(_) => "script".to_string(),
            NodeData::TcpConnect(_) => "tcp_connect".to_string(),
            NodeData::TcpSend(_) => "tcp_send".to_string(),
            NodeData::TcpReceive(_) => "tcp_receive".to_string(),
            NodeData::TcpClose(_) => "tcp_close".to_string(),
            NodeData::UdpBind(_) => "udp_bind".to_string(),
            NodeData::UdpSendTo(_) => "udp_send_to".to_string(),
            NodeData::UdpRecvFrom(_) => "udp_recv_from".to_string(),
            NodeData::UdpClose(_) => "udp_close".to_string(),
            NodeData::SseConnect(_) => "sse_connect".to_string(),
            NodeData::SseListen(_) => "sse_listen".to_string(),
            NodeData::SseDisconnect(_) => "sse_disconnect".to_string(),
            // GraphQL 节点
            NodeData::GraphqlConnect(_) => "graphql_connect".to_string(),
            NodeData::GraphqlQuery(_) => "graphql_query".to_string(),
            NodeData::GraphqlSubscribe(_) => "graphql_subscribe".to_string(),
            NodeData::GraphqlIntrospect(_) => "graphql_introspect".to_string(),
            NodeData::GraphqlDisconnect(_) => "graphql_disconnect".to_string(),
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

    /// 获取节点执行超时时间（毫秒）
    ///
    /// 返回节点执行器应该等待节点完成的最长时间。
    /// 执行器层使用此值作为 `tokio::time::timeout` 的安全网。
    /// 返回 `None` 表示不施加超时，依赖协议插件自身的超时机制。
    pub fn execution_timeout_ms(&self) -> Option<u64> {
        match self {
            NodeData::Http(cfg) => cfg.timeout_ms,
            NodeData::Script(cfg) => Some(cfg.timeout_ms),
            NodeData::SseConnect(cfg) => Some(cfg.timeout_ms),
            NodeData::SseListen(cfg) => Some(cfg.timeout_ms),
            NodeData::TcpConnect(cfg) => Some(cfg.connect_timeout_ms),
            NodeData::TcpSend(cfg) => Some(cfg.send_timeout_ms),
            NodeData::TcpReceive(cfg) => Some(cfg.receive_timeout_ms),
            NodeData::TcpClose(cfg) => Some(cfg.close_timeout_ms),
            NodeData::UdpSendTo(cfg) => Some(cfg.send_timeout_ms),
            NodeData::UdpRecvFrom(cfg) => Some(cfg.receive_timeout_ms),
            NodeData::WsConnect(cfg) => Some(cfg.connect_timeout_ms),
            // GraphqlConnect - connection_timeout_ms: u64
            NodeData::GraphqlConnect(cfg) => Some(cfg.connection_timeout_ms),
            // GraphqlQuery - timeout_ms: u64
            NodeData::GraphqlQuery(cfg) => Some(cfg.timeout_ms),
            // GraphqlSubscribe - timeout_ms: u64
            NodeData::GraphqlSubscribe(cfg) => Some(cfg.timeout_ms),
            // GraphqlIntrospect - timeout_ms: u64
            NodeData::GraphqlIntrospect(cfg) => Some(cfg.timeout_ms),
            // WsSendCollect - send_timeout_ms: u64
            NodeData::WsSendCollect(cfg) => Some(cfg.send_timeout_ms),
            // WsClose - close_timeout_ms: u64
            NodeData::WsClose(cfg) => Some(cfg.close_timeout_ms),
            // 其他节点类型没有明确的"整体执行超时"语义
            _ => None,
        }
    }

    /// 创建空的 Entry node配置
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

    /// 创建 Assertion 节点配置
    pub fn assertion() -> Self {
        Self::Assertion(AssertionConfig::default())
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
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_entry_config_empty_structure() {
        // 测试 EntryConfig 默认值
        let config = EntryConfig::default();
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_entry_config_default_impl() {
        // 测试 Default impl
        let config = EntryConfig::default();
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_http_config_serialization() {
        let config = HttpConfig {
            url: "https://api.example.com/users".to_string(),
            method: HttpMethod::Get,
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("https://api.example.com/users"));
        assert!(json.contains("get"));

        // 反序列化测试
        let deserialized: HttpConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&data).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains(r#""type":"http""#));
        assert!(json.contains("https://api.example.com"));

        // 反序列化测试
        let deserialized: NodeData = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&mode).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, r#""first_match""#);

        let deserialized: WsCollectMode =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&msg_type).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, r#""binary""#);

        let deserialized: WsMessageType =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized, WsMessageType::Binary);
    }

    #[test]
    fn test_ws_collect_condition() {
        let condition = WsCollectCondition {
            match_type: WsMatchType::Regex,
            expression: r#"status == 200"#.to_string(),
        };

        let json = serde_json::to_string(&condition).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("regex"));
        assert!(json.contains("status == 200"));

        let deserialized: WsCollectCondition =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("wss://example.com/ws"));
        assert!(json.contains("Authorization"));
        assert!(json.contains("chat"));
        assert!(json.contains("first_match"));

        let deserialized: WsConnectConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("conn-123"));
        assert!(json.contains("ping"));
        assert!(json.contains("condition_match"));
        assert!(json.contains("$.type == 'pong'"));

        let deserialized: WsSendCollectConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("conn-456"));
        assert!(json.contains("1001"));
        assert!(json.contains("Going Away"));

        let deserialized: WsCloseConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.close_code, 1001);
        assert_eq!(deserialized.close_reason, Some("Going Away".to_string()));
    }

    #[test]
    fn test_node_data_ws_connect() {
        let data = NodeData::ws_connect("wss://example.com/ws");
        assert_eq!(data.type_name(), "ws_connect");

        let json = serde_json::to_string(&data).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains(r#""type":"ws_connect""#));
        assert!(json.contains("wss://example.com/ws"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string(&data).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains(r#""type":"ws_send_collect""#));
        assert!(json.contains("conn-123"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string(&data).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains(r#""type":"ws_close""#));
        assert!(json.contains("conn-456"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&phase).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, r#""pre_request""#);

        let deserialized: ScriptExecutionPhase =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string_pretty(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("return status === 200;"));
        assert!(json.contains("10000"));
        assert!(json.contains("20"));
        assert!(json.contains("standalone"));
        assert!(json.contains("fatal"));

        let deserialized: ScriptNodeConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string(&data).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains(r#""type":"script""#));
        assert!(json.contains("console.log('hello');"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        match data {
            NodeData::Plugin(val) => {
                // 原始 JSON（含 type 字段）应完整保留
                assert_eq!(
                    val.get("type")
                        .unwrap_or_else(|| panic!("expected Some"))
                        .as_str(),
                    Some("grpc_request")
                );
                assert_eq!(
                    val.get("url")
                        .unwrap_or_else(|| panic!("expected Some"))
                        .as_str(),
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
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(data.type_name(), "custom_node");

        // 无 "type" 字段时回退到 "plugin"
        let json = r#"{"foo":"bar"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(data.type_name(), "plugin");
    }

    #[test]
    fn test_node_data_existing_types_unchanged() {
        // HTTP 应正常反序列化
        let json = r#"{"type":"http","url":"https://example.com","method":"get"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        match data {
            NodeData::Http(config) => assert_eq!(config.url, "https://example.com"),
            _ => panic!("Expected Http variant"),
        }

        // ws_connect 应正常反序列化
        let json = r#"{"type":"ws_connect","url":"wss://example.com"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        match data {
            NodeData::WsConnect(config) => assert_eq!(config.url, "wss://example.com"),
            _ => panic!("Expected WsConnect variant"),
        }

        // entry 应正常反序列化
        let json = r#"{"type":"entry"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(data, NodeData::Entry(_)));

        // script 应正常反序列化
        let json = r#"{"type":"script","script":"return 1;"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        match data {
            NodeData::Script(config) => assert_eq!(config.script, "return 1;"),
            _ => panic!("Expected Script variant"),
        }

        // 内置 db 已移除（plugins/db 插件化）：db_connect 现走 Plugin(Value)
        // 兜底，不再映射为显式 NodeData 变体。
    }

    // ========================================================================
    // B5: .mpf 格式兼容性验证 + 未知节点类型反序列化测试
    // ========================================================================

    #[test]
    fn test_unknown_node_type_deserialize() {
        // 未知节点类型应反序列化为 Plugin 变体，保留完整 JSON
        let json = r#"{"type": "custom_grpc", "endpoint": "localhost:50051"}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));

        match &data {
            NodeData::Plugin(val) => {
                assert_eq!(
                    val.get("type").and_then(|v| v.as_str()),
                    Some("custom_grpc")
                );
                assert_eq!(
                    val.get("endpoint").and_then(|v| v.as_str()),
                    Some("localhost:50051")
                );
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
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(data.type_name(), "my_custom_node");

        // Plugin 无 type 字段应回退到 "plugin"
        let json_no_type = r#"{"config": "value"}"#;
        let data_no_type: NodeData =
            serde_json::from_str(json_no_type).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&node).unwrap_or_else(|e| panic!("{e}"));

        // 验证序列化格式保留 type 字段
        assert!(json.contains(r#""type":"custom_grpc""#));

        // 反序列化
        let deserialized: Node = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string(&node).unwrap_or_else(|e| panic!("{e}"));

        // 序列化后 JSON 应包含 "type":"http"
        assert!(json.contains(r#""type":"http""#));

        // 往返验证
        let back: Node = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let back: TcpConnectConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let back: TcpSendConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let back: TcpCloseConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let back: UdpBindConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let back: UdpSendToConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let back: UdpRecvFromConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back.connection_id, "conn-1");
        assert_eq!(back.receive_timeout_ms, 8000);
        assert_eq!(back.max_size, 32768);
    }

    #[test]
    fn test_udp_close_config_serde_roundtrip() {
        let config = UdpCloseConfig {
            connection_id: "conn-1".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let back: UdpCloseConfig = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back.connection_id, "conn-1");
    }

    #[test]
    fn test_node_data_tcp_connect_roundtrip() {
        let json = r#"{"type":"tcp_connect","host":"127.0.0.1","port":8080,"use_tls":false,"tls_skip_verify":false,"connect_timeout_ms":30000,"framing_mode":{"length_prefix":{"prefix_bytes":4}},"framing_config":{"mode":{"length_prefix":{"prefix_bytes":4}},"max_frame_size":1048576,"include_framing_in_data":false}}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(data.type_name(), "tcp_connect");
        match &data {
            NodeData::TcpConnect(cfg) => {
                assert_eq!(cfg.host, "127.0.0.1");
                assert_eq!(cfg.port, 8080);
            }
            _ => panic!("Expected TcpConnect variant"),
        }
        // 往返验证
        let serialized = serde_json::to_string(&data).unwrap_or_else(|e| panic!("{e}"));
        let back: NodeData = serde_json::from_str(&serialized).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back.type_name(), "tcp_connect");
    }

    #[test]
    fn test_node_data_tcp_send_roundtrip() {
        let json = r#"{"type":"tcp_send","connection_id":"c1","data":"hello","data_encoding":"text","send_timeout_ms":30000}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
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
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
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
        let json =
            r#"{"type":"udp_bind","local_addr":"0.0.0.0","local_port":9090,"reuse_addr":true}"#;
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
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
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
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
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
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
        let data: NodeData = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
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
    fn test_entry_config_backward_compat_empty() {
        // 空 JSON 应正常反序列化
        let json = r#"{}"#;
        let config: EntryConfig = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(config.dataset_path, None);
        assert_eq!(config.dataset_encoding, None);
    }

    #[test]
    fn test_entry_config_with_dataset_fields() {
        let json = r#"{"dataset_path":"/path/file.csv","dataset_encoding":"utf-8"}"#;
        let config: EntryConfig = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(config.dataset_path, Some("/path/file.csv".to_string()));
        assert_eq!(config.dataset_encoding, Some("utf-8".to_string()));
    }

    #[test]
    fn test_entry_config_default_fields_none() {
        let config = EntryConfig::default();
        assert_eq!(config.dataset_path, None);
        assert_eq!(config.dataset_encoding, None);
    }

    #[test]
    fn test_entry_config_skip_serializing_none_dataset() {
        let config = EntryConfig {
            dataset_path: None,
            dataset_encoding: None,
            hooks: None,
        };
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(!json.contains("dataset_path"));
        assert!(!json.contains("dataset_encoding"));
    }

    #[test]
    fn test_entry_config_serializes_dataset_fields_when_set() {
        let config = EntryConfig {
            dataset_path: Some("/data/test.csv".to_string()),
            dataset_encoding: Some("gbk".to_string()),
            hooks: None,
        };
        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("dataset_path"));
        assert!(json.contains("/data/test.csv"));
        assert!(json.contains("dataset_encoding"));
        assert!(json.contains("gbk"));
    }

    #[test]
    fn test_entry_config_only_dataset_path() {
        let json = r#"{"dataset_path":"/path/data.csv"}"#;
        let config: EntryConfig = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(config.dataset_path, Some("/path/data.csv".to_string()));
        assert_eq!(config.dataset_encoding, None);
    }

}
