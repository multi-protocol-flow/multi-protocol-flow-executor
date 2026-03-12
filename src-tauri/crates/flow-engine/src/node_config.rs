// 节点配置模块 - 强类型节点配置定义

use crate::variable::{InitialVariable, OutputMapping};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// ============================================================================
// Entry 节点配置
// ============================================================================

/// 入口节点配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntryConfig {
    /// 初始变量列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_variables: Vec<InitialVariable>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionType {
    /// 简单条件（字段比较）
    #[default]
    Simple,
    /// 高级条件（表达式）
    Advanced,
}

/// 条件操作符枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// WebSocket 等待模式
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsWaitMode {
    /// 等待第一条消息
    #[default]
    FirstMessage,
    /// 等待满足条件
    Condition,
    /// 等待指定时长
    Duration,
}

/// WebSocket 匹配类型
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsMatchType {
    /// JSONPath 匹配
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

/// WebSocket 等待条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WsWaitCondition {
    /// 匹配类型
    pub match_type: WsMatchType,
    /// 匹配表达式
    pub expression: String,
}

impl PartialEq for WsWaitCondition {
    fn eq(&self, other: &Self) -> bool {
        self.match_type == other.match_type && self.expression == other.expression
    }
}

/// WebSocket Connect 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// 是否等待消息
    #[serde(default)]
    pub wait_for_message: bool,
    /// 等待模式
    #[serde(default)]
    pub wait_mode: WsWaitMode,
    /// 等待条件
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_condition: Option<WsWaitCondition>,
    /// 消息超时（毫秒）
    #[serde(default = "default_message_timeout")]
    pub message_timeout_ms: u64,
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
            wait_for_message: false,
            wait_mode: WsWaitMode::default(),
            wait_condition: None,
            message_timeout_ms: default_message_timeout(),
            output_mappings: Vec::new(),
        }
    }
}

/// WebSocket Send+Wait 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSendWaitConfig {
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
    /// 是否等待响应
    #[serde(default)]
    pub wait_for_response: bool,
    /// 等待模式
    #[serde(default)]
    pub wait_mode: WsWaitMode,
    /// 等待条件
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_condition: Option<WsWaitCondition>,
    /// 消息超时（毫秒）
    #[serde(default = "default_message_timeout")]
    pub message_timeout_ms: u64,
    /// 输出变量映射
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_mappings: Vec<OutputMapping>,
}

impl Default for WsSendWaitConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            message: String::new(),
            message_type: WsMessageType::default(),
            send_timeout_ms: default_send_timeout(),
            wait_for_response: true,
            wait_mode: WsWaitMode::default(),
            wait_condition: None,
            message_timeout_ms: default_message_timeout(),
            output_mappings: Vec::new(),
        }
    }
}

/// WebSocket Close 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCloseConfig {
    /// 连接 ID
    pub connection_id: String,
    /// 关闭码
    #[serde(default = "default_close_code")]
    pub close_code: u16,
    /// 关闭原因
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<String>,
    /// 等待关闭超时（毫秒）
    #[serde(default = "default_close_timeout")]
    pub wait_close_timeout_ms: u64,
}

impl Default for WsCloseConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            close_code: default_close_code(),
            close_reason: None,
            wait_close_timeout_ms: default_close_timeout(),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbConnectConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
    /// 测试查询语句
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_query: Option<String>,
    /// 连接超时（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_timeout_ms: Option<u64>,
}

/// 数据库查询节点配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbBeginConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
    /// 隔离级别
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolation_level: Option<String>,
}

/// 数据库事务提交节点配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbCommitConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
}

/// 数据库事务回滚节点配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbRollbackConfig {
    /// 连接 UUID
    #[serde(default)]
    pub connection_uuid: String,
}

/// 数据库关闭节点配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
// 节点数据配置统一枚举
// ============================================================================

/// 节点数据配置（强类型枚举）
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// WebSocket 发送等待节点配置
    #[serde(rename = "ws_send_wait")]
    WsSendWait(WsSendWaitConfig),
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
}

impl NodeData {
    /// 获取节点类型名称
    pub fn type_name(&self) -> &'static str {
        match self {
            NodeData::Entry(_) => "entry",
            NodeData::Http(_) => "http",
            NodeData::Conditional(_) => "conditional",
            NodeData::WsConnect(_) => "ws_connect",
            NodeData::WsSendWait(_) => "ws_send_wait",
            NodeData::WsClose(_) => "ws_close",
            NodeData::Assertion(_) => "assertion",
            NodeData::DbConnect(_) => "db_connect",
            NodeData::DbQuery(_) => "db_query",
            NodeData::DbBegin(_) => "db_begin",
            NodeData::DbCommit(_) => "db_commit",
            NodeData::DbRollback(_) => "db_rollback",
            NodeData::DbClose(_) => "db_close",
            NodeData::Script(_) => "script",
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

    /// 创建 WsConnect 节点配置
    pub fn ws_connect(url: impl Into<String>) -> Self {
        Self::WsConnect(WsConnectConfig {
            url: url.into(),
            ..Default::default()
        })
    }

    /// 创建 WsSendWait 节点配置
    pub fn ws_send_wait(connection_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::WsSendWait(WsSendWaitConfig {
            connection_id: connection_id.into(),
            message: message.into(),
            ..Default::default()
        })
    }

    /// 创建 WsClose 节点配置
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

    /// 创建 DbConnect 节点配置
    pub fn db_connect(connection_uuid: impl Into<String>) -> Self {
        Self::DbConnect(DbConnectConfig {
            connection_uuid: connection_uuid.into(),
            ..Default::default()
        })
    }

    /// 创建 DbQuery 节点配置
    pub fn db_query(connection_uuid: impl Into<String>) -> Self {
        Self::DbQuery(DbQueryConfig {
            connection_uuid: connection_uuid.into(),
            ..Default::default()
        })
    }

    /// 创建 DbBegin 节点配置
    pub fn db_begin(connection_uuid: impl Into<String>) -> Self {
        Self::DbBegin(DbBeginConfig {
            connection_uuid: connection_uuid.into(),
            ..Default::default()
        })
    }

    /// 创建 DbCommit 节点配置
    pub fn db_commit(connection_uuid: impl Into<String>) -> Self {
        Self::DbCommit(DbCommitConfig {
            connection_uuid: connection_uuid.into(),
        })
    }

    /// 创建 DbRollback 节点配置
    pub fn db_rollback(connection_uuid: impl Into<String>) -> Self {
        Self::DbRollback(DbRollbackConfig {
            connection_uuid: connection_uuid.into(),
        })
    }

    /// 创建 DbClose 节点配置
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

/// 为 NodeData 实现 Default，使用 Entry 作为默认值
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
    fn test_ws_wait_mode_default() {
        let mode = WsWaitMode::default();
        assert_eq!(mode, WsWaitMode::FirstMessage);
    }

    #[test]
    fn test_ws_wait_mode_serialization() {
        let mode = WsWaitMode::FirstMessage;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, r#""first_message""#);

        let deserialized: WsWaitMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, WsWaitMode::FirstMessage);
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
    fn test_ws_wait_condition() {
        let condition = WsWaitCondition {
            match_type: WsMatchType::Regex,
            expression: r#"status == 200"#.to_string(),
        };

        let json = serde_json::to_string(&condition).unwrap();
        assert!(json.contains("regex"));
        assert!(json.contains("status == 200"));

        let deserialized: WsWaitCondition = serde_json::from_str(&json).unwrap();
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
        assert!(!config.wait_for_message);
        assert_eq!(config.wait_mode, WsWaitMode::FirstMessage);
        assert!(config.wait_condition.is_none());
        assert_eq!(config.message_timeout_ms, 10000);
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
            wait_for_message: true,
            wait_mode: WsWaitMode::FirstMessage,
            wait_condition: None,
            message_timeout_ms: 15000,
            output_mappings: vec![],
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("wss://example.com/ws"));
        assert!(json.contains("Authorization"));
        assert!(json.contains("chat"));
        assert!(json.contains("first_message"));

        let deserialized: WsConnectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, "wss://example.com/ws");
        assert_eq!(deserialized.headers.len(), 1);
        assert_eq!(deserialized.subprotocols.len(), 1);
        assert_eq!(deserialized.connect_timeout_ms, 5000);
    }

    #[test]
    fn test_ws_send_wait_config_default() {
        let config = WsSendWaitConfig::default();
        assert!(config.connection_id.is_empty());
        assert!(config.message.is_empty());
        assert_eq!(config.message_type, WsMessageType::Text);
        assert_eq!(config.send_timeout_ms, 10000);
        assert!(config.wait_for_response);
        assert_eq!(config.wait_mode, WsWaitMode::FirstMessage);
        assert_eq!(config.message_timeout_ms, 10000);
    }

    #[test]
    fn test_ws_send_wait_config_serialization() {
        let config = WsSendWaitConfig {
            connection_id: "conn-123".to_string(),
            message: r#"{"type":"ping"}"#.to_string(),
            message_type: WsMessageType::Text,
            send_timeout_ms: 5000,
            wait_for_response: true,
            wait_mode: WsWaitMode::Condition,
            wait_condition: Some(WsWaitCondition {
                match_type: WsMatchType::JsonPath,
                expression: "$.type == 'pong'".to_string(),
            }),
            message_timeout_ms: 8000,
            output_mappings: vec![],
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("conn-123"));
        assert!(json.contains("ping"));
        assert!(json.contains("condition"));
        assert!(json.contains("$.type == 'pong'"));

        let deserialized: WsSendWaitConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.connection_id, "conn-123");
        assert_eq!(deserialized.wait_mode, WsWaitMode::Condition);
        assert!(deserialized.wait_condition.is_some());
    }

    #[test]
    fn test_ws_close_config_default() {
        let config = WsCloseConfig::default();
        assert!(config.connection_id.is_empty());
        assert_eq!(config.close_code, 1000);
        assert!(config.close_reason.is_none());
        assert_eq!(config.wait_close_timeout_ms, 5000);
    }

    #[test]
    fn test_ws_close_config_serialization() {
        let config = WsCloseConfig {
            connection_id: "conn-456".to_string(),
            close_code: 1001,
            close_reason: Some("Going Away".to_string()),
            wait_close_timeout_ms: 3000,
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
    fn test_node_data_ws_send_wait() {
        let data = NodeData::ws_send_wait("conn-123", r#"{"type":"ping"}"#);
        assert_eq!(data.type_name(), "ws_send_wait");

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""type":"ws_send_wait""#));
        assert!(json.contains("conn-123"));

        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        match deserialized {
            NodeData::WsSendWait(config) => {
                assert_eq!(config.connection_id, "conn-123");
                assert_eq!(config.message, r#"{"type":"ping"}"#);
            }
            _ => panic!("Expected WsSendWait variant"),
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
}
