// 节点配置模块 - 强类型节点配置定义

use crate::traits::{FramingConfig, GrpcServiceInfo, TcpFramingMode};
use crate::variable::{InitialVariable, OutputMapping};
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
    /// 输出变量映射
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_mappings: Vec<OutputMapping>,
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
            output_mappings: Vec::new(),
        }
    }
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
    /// 输出变量映射
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_mappings: Vec<OutputMapping>,
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
            output_mappings: Vec::new(),
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
    /// 输出变量映射
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_mappings: Vec<OutputMapping>,
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
            output_mappings: Vec::new(),
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
    /// 输出变量映射
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_mappings: Vec<OutputMapping>,
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
    /// 断言失败时的行为
    #[serde(default)]
    pub on_assertion_fail: AssertionFailStrategy,
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
            on_assertion_fail: AssertionFailStrategy::default(),
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
pub struct McpCallToolConfig {
    /// 连接 ID（引用 mcp_connect 节点的 UUID）
    pub connection_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 工具参数（JSON）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

impl Default for McpCallToolConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            tool_name: String::new(),
            arguments: None,
        }
    }
}

/// MCP Read Resource 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpReadResourceConfig {
    /// 连接 ID（引用 mcp_connect 节点的 UUID）
    pub connection_id: String,
    /// 资源 URI
    pub resource_uri: String,
}

impl Default for McpReadResourceConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            resource_uri: String::new(),
        }
    }
}

/// MCP Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct McpCloseConfig {
    /// 连接 ID（引用 mcp_connect 节点的 UUID）
    pub connection_id: String,
}

/// MCP Get Prompt 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpGetPromptConfig {
    /// 连接 ID（引用 mcp_connect 节点的 UUID）
    pub connection_id: String,
    /// 提示词名称
    pub prompt_name: String,
    /// 提示词参数
    #[serde(default)]
    pub arguments: std::collections::HashMap<String, String>,
}

impl Default for McpGetPromptConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            prompt_name: String::new(),
            arguments: std::collections::HashMap::new(),
        }
    }
}

// ============================================================================
// TCP 节点配置
// ============================================================================

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
            framing_mode: TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
            framing_config: FramingConfig::default(),
        }
    }
}

/// TCP Send 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpSendConfig {
    /// 连接 ID（引用 tcp_connect 节点的 UUID）
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
        }
    }
}

/// TCP Close 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpCloseConfig {
    /// 连接 ID（引用 tcp_connect 节点的 UUID）
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
        }
    }
}

/// UDP SendTo 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UdpSendToConfig {
    /// 连接 ID（引用 udp_bind 节点的 UUID）
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

/// UDP RecvFrom 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UdpRecvFromConfig {
    /// 连接 ID（引用 udp_bind 节点的 UUID）
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
    /// 连接 ID（引用 udp_bind 节点的 UUID）
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
    /// 遗嘱 QoS
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
    /// 连接 ID（引用 mqtt_connect 节点的 UUID）
    pub connection_id: String,
    /// 发布主题
    pub topic: String,
    /// 发布载荷
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// QoS 等级
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
    /// 连接 ID（引用 mqtt_connect 节点的 UUID）
    pub connection_id: String,
    /// 订阅主题
    pub topic: String,
    /// QoS 等级
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
    /// 连接 ID（引用 mqtt_connect 节点的 UUID）
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
    /// 连接 ID（引用 mqtt_connect 节点的 UUID）
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

/// MQTT WaitMessage 节点配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttWaitMessageConfig {
    /// 节点类型标识
    #[serde(default = "default_mqtt_wait_message_type")]
    pub r#type: String,
    /// 连接 ID（引用 mqtt_connect 节点的 UUID）
    pub connection_id: String,
    /// 等待主题
    pub topic: String,
    /// 匹配条件（包含 match_type + expression）
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
    /// 连接 ID（引用 mqtt_connect 节点的 UUID）
    pub connection_id: String,
    /// 请求主题
    pub request_topic: String,
    /// 响应主题
    pub response_topic: String,
    /// 请求载荷
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// QoS 等级
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
        assert!(config.output_mappings.is_empty());
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
            output_mappings: vec![],
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
            output_mappings: vec![],
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
        assert_eq!(config.on_assertion_fail, AssertionFailStrategy::RouteFalse);
    }

    #[test]
    fn test_script_node_config_serialization() {
        let config = ScriptNodeConfig {
            script: "return status === 200;".to_string(),
            timeout_ms: 10000,
            memory_limit_mb: 20,
            execution_phase: ScriptExecutionPhase::Standalone,
            on_error: ScriptErrorStrategy::Fatal,
            on_assertion_fail: AssertionFailStrategy::Abort,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("return status === 200;"));
        assert!(json.contains("10000"));
        assert!(json.contains("20"));
        assert!(json.contains("standalone"));
        assert!(json.contains("fatal"));
        assert!(json.contains("abort"));

        let deserialized: ScriptNodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.script, "return status === 200;");
        assert_eq!(deserialized.timeout_ms, 10000);
        assert_eq!(deserialized.memory_limit_mb, 20);
        assert_eq!(
            deserialized.execution_phase,
            ScriptExecutionPhase::Standalone
        );
        assert_eq!(deserialized.on_error, ScriptErrorStrategy::Fatal);
        assert_eq!(deserialized.on_assertion_fail, AssertionFailStrategy::Abort);
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
}
