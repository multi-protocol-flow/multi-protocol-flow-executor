// Error模块 - Error详情和Error类型

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Error详情结构（用于执行调试）
///
/// 提供结构化的Error信息，包括Error类型、Error代码、消息、堆栈跟踪和上下文快照。
/// 这些信息用于调试和Error分析。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Error类型（决定Error处理策略，同时标识具体Error类型）
    pub error_kind: ErrorKind,

    /// Error代码（可选，用于程序化处理）
    pub error_code: Option<ErrorCode>,

    /// i18n 翻译参数（可选，用于前端Error消息展示）
    pub i18n_args: Option<HashMap<String, String>>,

    /// Error消息（人类可读）
    pub message: String,

    /// 堆栈跟踪（可选）
    pub stack_trace: Option<String>,

    /// Error发生时的上下文快照（可选）
    pub context: Option<serde_json::Value>,
}

impl ErrorDetail {
    /// 创建新的Error详情
    ///
    /// # 参数
    ///
    /// * `error_kind` - Error类型
    /// * `message` - Error消息
    ///
    /// # 返回
    ///
    /// 新的Error详情，不包含Error代码、堆栈跟踪和上下文
    pub fn new(error_kind: ErrorKind, message: String) -> Self {
        Self {
            error_kind,
            error_code: None,
            i18n_args: None,
            message,
            stack_trace: None,
            context: None,
        }
    }

    /// 设置类型化Error代码
    ///
    /// # 参数
    ///
    /// * `code` - `ErrorCode` 枚举值
    ///
    /// # 返回
    ///
    /// 自身的可变引用，支持链式调用
    pub fn with_error_code_typed(mut self, code: ErrorCode) -> Self {
        self.error_code = Some(code);
        self
    }

    /// 添加 i18n 翻译参数
    ///
    /// 用于前端Error消息的翻译插值。例如：`with_i18n_arg("url", "https://example.com")`
    ///
    /// # 参数
    ///
    /// * `key` - 参数键名
    /// * `value` - 参数值
    ///
    /// # 返回
    ///
    /// 自身的可变引用，支持链式调用
    pub fn with_i18n_arg(mut self, key: &str, value: String) -> Self {
        self.i18n_args
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
        self
    }

    /// 创建通用执行Error
    ///
    /// 使用 `ErrorKind::Execution` 和给定的消息快速创建Error详情。
    ///
    /// # 参数
    ///
    /// * `message` - Error消息
    ///
    /// # 返回
    ///
    /// 新的Error详情
    pub fn execution_error(message: String) -> Self {
        Self::new(ErrorKind::Execution, message)
    }

    /// 设置堆栈跟踪
    ///
    /// # 参数
    ///
    /// * `trace` - 堆栈跟踪字符串
    ///
    /// # 返回
    ///
    /// 自身的可变引用，支持链式调用
    pub fn with_stack_trace(mut self, trace: String) -> Self {
        self.stack_trace = Some(trace);
        self
    }

    /// 设置上下文快照
    ///
    /// # 参数
    ///
    /// * `context` - 上下文快照（JSON 值）
    ///
    /// # 返回
    ///
    /// 自身的可变引用，支持链式调用
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// 判断是否为致命Error
    ///
    /// # 返回
    ///
    /// 如果是致命Error（需要立即终止流程）则返回 true
    pub fn is_fatal(&self) -> bool {
        self.error_kind.is_fatal()
    }
}

impl std::fmt::Display for ErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Error类型枚举（决定Error处理策略，同时标识具体Error类型）
///
/// Error分为两类：
/// - **致命Error**：配置Error和条件语法Error，需要立即终止流程
/// - **可恢复Error**：执行Error、验证Error、解析Error等，在容错模式下可继续执行
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    // ===== 配置Error（立即终止流程） =====
    /// 通用配置Error
    Configuration,

    /// 条件表达式语法Error（如表达式语法Error、字段配置缺失）
    ConditionSyntax,

    // ===== 执行Error（容错模式，继续执行） =====
    /// 通用执行Error
    Execution,

    /// 网络connection failed
    Connection,

    /// 操作超时
    Timeout,

    /// 条件评估运行时Error（如字段不存在、类型不匹配）
    ConditionEvaluation,

    // ===== 验证Error（容错模式，继续执行） =====
    /// 参数验证Failed
    Validation,

    // ===== 解析Error（容错模式，继续执行） =====
    /// 数据解析Failed（如 JSON 解析Failed、数据格式转换Failed）
    Parse,

    // ===== WebSocket Error（容错模式，继续执行） =====
    /// WebSocket connection failed
    WebSocketConnection,
    /// WebSocket 超时
    WebSocketTimeout,
    /// WebSocket 消息Error
    WebSocketMessage,
    /// WebSocket 关闭Error
    WebSocketClose,
    /// WebSocket 协议Error
    WebSocketProtocol,

    // ===== gRPC Error（容错模式，继续执行） =====
    /// gRPC connection failed
    GrpcConnection,
    /// gRPC 超时
    GrpcTimeout,
    /// gRPC 消息Error
    GrpcMessage,
    /// gRPC 状态Error
    GrpcStatus,
    /// gRPC 关闭Error
    GrpcClose,

    // ===== MCP Error（容错模式，继续执行） =====
    /// MCP connection failed
    McpConnection,
    /// MCP 工具调用Failed
    McpToolCall,
    /// MCP 资源读取Failed
    McpResourceRead,
    /// MCP 连接未找到
    McpNotConnected,

    // ===== 断言Error（容错模式，继续执行） =====
    /// 断言Failed
    Assertion,

    // ===== 脚本Error（根据 on_error 配置决定是否致命） =====
    /// 脚本语法Error
    ScriptSyntax,
    /// 脚本运行时Error
    ScriptRuntime,
    /// 脚本执行超时
    ScriptTimeout,
    /// 断言Failed
    ScriptAssertion,
    /// 用户调用 `fn.flow.abort()`
    ScriptAbort,

    // ===== TCP Error（容错模式，继续执行） =====
    /// TCP connection failed
    TcpConnection,
    /// TCP 操作超时
    TcpTimeout,
    /// TCP 发送数据Failed
    TcpSend,
    /// TCP 接收数据Failed
    TcpReceive,
    /// TCP 连接关闭Error
    TcpClose,

    // ===== UDP Error（容错模式，继续执行） =====
    /// UDP 绑定Failed
    UdpBind,
    /// UDP 发送数据Failed
    UdpSend,
    /// UDP 接收数据Failed
    UdpReceive,
    /// UDP 连接关闭Error
    UdpClose,

    // ===== SSE Error（容错模式，继续执行） =====
    /// SSE connection failed
    SseConnection,
    /// SSE 超时
    SseTimeout,
    /// SSE 事件Error
    SseEvent,

    // ===== GraphQL Error（容错模式，继续执行） =====
    /// GraphQL connection failed
    GraphqlConnection,
    /// GraphQL 查询Error
    GraphqlQuery,
    /// GraphQL 订阅Error
    GraphqlSubscription,
    /// GraphQL Schema 探索Error
    GraphqlIntrospection,
    /// GraphQL 关闭Error
    GraphqlClose,

    // ===== AMQP Error（容错模式，继续执行） =====
    /// AMQP connection failed
    AmqpConnection,
    /// AMQP 通道Error
    AmqpChannel,
    /// AMQP 发布Error
    AmqpPublish,
    /// AMQP 消费Error
    AmqpConsume,
    /// AMQP 队列操作Error
    AmqpQueue,
    /// AMQP 交换器操作Error
    AmqpExchange,
    /// AMQP 确认Error
    AmqpAck,
    /// AMQP 管理操作Error
    AmqpManagement,
    /// AMQP 协议Error
    AmqpProtocol,

    // ===== 取消Error（容错模式，继续执行） =====
    /// 执行被取消
    Cancelled,
}

impl ErrorKind {
    /// 判断是否为致命Error（需要立即终止流程）
    ///
    /// # 返回
    ///
    /// 如果是致命Error则返回 true
    ///
    /// # 致命Error类型
    ///
    /// - `ConfigurationError` - 配置Error
    /// - `ConditionSyntaxError` - 条件语法Error
    pub fn is_fatal(&self) -> bool {
        matches!(self, ErrorKind::Configuration | ErrorKind::ConditionSyntax)
    }
}

// ===== ErrorCode enum for i18n translation and programmatic handling =====

/// Error代码枚举（用于 i18n 翻译和程序化处理）
///
/// 命名规范：
/// - 通用Error（无特定协议）：`E_<CATEGORY>_<DESCRIPTION>`（如 `E_CONN_TIMEOUT`）
/// - 协议特定Error：`E_<PROTOCOL>_<DESCRIPTION>`（如 `E_WS_TIMEOUT`）
/// - 两者冲突时，使用协议特定变体（如 WebSocket 超时用 `E_WS_TIMEOUT` 而非 `E_CONN_TIMEOUT`）
/// - 覆盖要求：每个 `ErrorKind` 变体至少对应一个 `ErrorCode` 变体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum ErrorCode {
    // ===== General errors =====
    /// Configuration error.
    E_CONFIG_ERROR,
    /// Condition expression syntax error.
    E_CONDITION_SYNTAX,
    /// General execution error.
    E_EXECUTION_ERROR,
    /// Operation timed out.
    E_TIMEOUT,
    /// Condition evaluation runtime error.
    E_CONDITION_EVALUATION,
    /// Unknown error.
    E_UNKNOWN,
    /// Execution was cancelled.
    E_CANCELLED,
    /// Data parse error.
    E_PARSE,
    /// Assertion failed.
    E_ASSERTION,
    /// Script aborted via `flow.abort()`.
    E_SCRIPT_ABORT,

    // ===== Connection errors =====
    /// Connection timed out.
    E_CONN_TIMEOUT,
    /// Connection refused.
    E_CONN_REFUSED,
    /// Connection closed.
    E_CONN_CLOSED,

    // ===== Validation errors =====
    /// Duplicate port detected.
    E_VAL_PORT_DUPLICATE,
    /// Cycle detected in flow.
    E_VAL_CYCLE_DETECTED,
    /// Missing entry node.
    E_VAL_MISSING_ENTRY,
    /// Duplicate node UUID detected.
    E_VAL_DUPLICATE_NODE_ID,
    /// Connection references non-existent node.
    E_VAL_CONN_NODE_NOT_FOUND,
    /// Connection references non-existent port.
    E_VAL_PORT_NOT_FOUND,
    /// Node is not reachable from any Entry node.
    E_VAL_NODE_UNREACHABLE,
    /// Node data contains unknown/unexpected fields.
    E_VAL_UNKNOWN_FIELD,
    /// Node is missing a required field.
    E_VAL_MISSING_REQUIRED_FIELD,
    /// Node data field type mismatch.
    E_VAL_TYPE_MISMATCH,
    /// Unknown/unsupported node type.
    E_VAL_UNKNOWN_NODE_TYPE,

    // ===== HTTP errors =====
    /// HTTP request failed.
    E_HTTP_REQUEST_FAILED,
    /// HTTP response status error.
    E_HTTP_STATUS_ERROR,
    /// Invalid HTTP URL.
    E_HTTP_INVALID_URL,

    // ===== WebSocket errors =====
    /// WebSocket connection failed.
    E_WS_CONNECTION,
    /// WebSocket operation timed out.
    E_WS_TIMEOUT,
    /// WebSocket message error.
    E_WS_MESSAGE,
    /// WebSocket close error.
    E_WS_CLOSE,
    /// WebSocket protocol error.
    E_WS_PROTOCOL,

    // ===== Database errors =====
    /// Database connection failed.
    E_DB_CONNECTION,
    /// Database query error.
    E_DB_QUERY,
    /// Database operation timed out.
    E_DB_TIMEOUT,

    // ===== Script errors =====
    /// Script syntax error.
    E_SCRIPT_SYNTAX,
    /// Script runtime error.
    E_SCRIPT_RUNTIME,
    /// Script execution timed out.
    E_SCRIPT_TIMEOUT,
    /// Script assertion failed.
    E_SCRIPT_ASSERTION,

    // ===== gRPC errors =====
    /// gRPC connection failed.
    E_GRPC_CONNECTION,
    /// gRPC operation timed out.
    E_GRPC_TIMEOUT,
    /// gRPC message error.
    E_GRPC_MESSAGE,
    /// gRPC status error.
    E_GRPC_STATUS,
    /// gRPC close error.
    E_GRPC_CLOSE,

    // ===== MCP errors =====
    /// MCP connection failed.
    E_MCP_CONNECTION,
    /// MCP tool call failed.
    E_MCP_TOOL_CALL,
    /// MCP resource read failed.
    E_MCP_RESOURCE_READ,
    /// MCP not connected.
    E_MCP_NOT_CONNECTED,

    // ===== TCP errors =====
    /// TCP connection failed.
    E_TCP_CONNECTION,
    /// TCP operation timed out.
    E_TCP_TIMEOUT,
    /// TCP send error.
    E_TCP_SEND,
    /// TCP receive error.
    E_TCP_RECEIVE,
    /// TCP close error.
    E_TCP_CLOSE,

    // ===== UDP errors =====
    /// UDP bind failed.
    E_UDP_BIND,
    /// UDP send error.
    E_UDP_SEND,
    /// UDP receive error.
    E_UDP_RECEIVE,
    /// UDP close error.
    E_UDP_CLOSE,

    // ===== SSE errors =====
    /// SSE connection failed.
    E_SSE_CONNECTION,
    /// SSE operation timed out.
    E_SSE_TIMEOUT,
    /// SSE event error.
    E_SSE_EVENT,

    // ===== GraphQL errors =====
    /// GraphQL connection failed.
    E_GRAPHQL_CONNECTION,
    /// GraphQL query error.
    E_GRAPHQL_QUERY,
    /// GraphQL subscription error.
    E_GRAPHQL_SUBSCRIPTION,
    /// GraphQL introspection error.
    E_GRAPHQL_INTROSPECTION,
    /// GraphQL close error.
    E_GRAPHQL_CLOSE,

    // ===== AMQP errors =====
    /// AMQP connection failed.
    E_AMQP_CONNECTION,
    /// AMQP channel error.
    E_AMQP_CHANNEL,
    /// AMQP publish error.
    E_AMQP_PUBLISH,
    /// AMQP consume error.
    E_AMQP_CONSUME,
    /// AMQP queue operation error.
    E_AMQP_QUEUE,
    /// AMQP exchange operation error.
    E_AMQP_EXCHANGE,
    /// AMQP ack error.
    E_AMQP_ACK,
    /// AMQP management operation error.
    E_AMQP_MANAGEMENT,
    /// AMQP protocol error.
    E_AMQP_PROTOCOL,

    // ===== Redis errors =====
    /// Redis connection failed.
    E_REDIS_CONNECTION,
    /// Redis command error.
    E_REDIS_COMMAND,
    /// Redis operation timed out.
    E_REDIS_TIMEOUT,
    /// Redis authentication failed.
    E_REDIS_AUTH,

    // ===== SMTP errors =====
    /// SMTP connection failed.
    E_SMTP_CONNECTION,
    /// SMTP send error.
    E_SMTP_SEND,
    /// SMTP authentication failed.
    E_SMTP_AUTH,
    /// SMTP operation timed out.
    E_SMTP_TIMEOUT,

    // ===== IMAP errors =====
    /// IMAP connection failed.
    E_IMAP_CONNECTION,
    /// IMAP fetch error.
    E_IMAP_FETCH,
    /// IMAP search error.
    E_IMAP_SEARCH,
    /// IMAP authentication failed.
    E_IMAP_AUTH,
    /// IMAP store operation error.
    E_IMAP_STORE,
    /// IMAP operation timed out.
    E_IMAP_TIMEOUT,

    // ===== Non-protocol node errors =====
    /// Variable extractor execution failed.
    E_VARIABLE_EXTRACTOR_FAILED,
    /// Proxy node execution failed.
    E_PROXY_FAILED,

    // ===== Plugin errors =====
    /// Sidecar plugin process crashed or exited abnormally.
    ///
    /// Serde name stays `SCREAMING_SNAKE_CASE` (`E_PLUGIN_CRASHED`) to match
    /// the i18n key and the frontend `ErrorCode` union conventions; the Rust
    /// variant name is `PluginCrashed` per the plugin-architecture plan.
    #[serde(rename = "E_PLUGIN_CRASHED")]
    PluginCrashed,
}

impl ErrorCode {
    /// Returns the expected i18n argument keys for this error code variant.
    ///
    /// These keys can be used to construct translation templates in the frontend.
    pub fn expected_i18n_args(&self) -> &[&str] {
        match self {
            // General
            Self::E_CONFIG_ERROR => &["message"],
            Self::E_CONDITION_SYNTAX => &["expression", "message"],
            Self::E_EXECUTION_ERROR => &["message"],
            Self::E_TIMEOUT => &["duration", "message"],
            Self::E_CONDITION_EVALUATION => &["expression", "message"],
            Self::E_UNKNOWN => &["message"],
            Self::E_CANCELLED => &["message"],
            Self::E_PARSE => &["input", "message"],
            Self::E_ASSERTION => &["expected", "actual", "message"],
            Self::E_SCRIPT_ABORT => &["message"],
            // Connection
            Self::E_CONN_TIMEOUT => &["host", "port", "duration"],
            Self::E_CONN_REFUSED => &["host", "port"],
            Self::E_CONN_CLOSED => &["host", "port"],
            // Validation
            Self::E_VAL_PORT_DUPLICATE => &["port_id", "node_id"],
            Self::E_VAL_CYCLE_DETECTED => &["node_id"],
            Self::E_VAL_MISSING_ENTRY => &["message"],
            Self::E_VAL_DUPLICATE_NODE_ID => &["name", "uuid"],
            Self::E_VAL_CONN_NODE_NOT_FOUND => &["id", "uuid"],
            Self::E_VAL_PORT_NOT_FOUND => &["name", "port_id"],
            Self::E_VAL_NODE_UNREACHABLE => &["name"],
            Self::E_VAL_UNKNOWN_FIELD => &["message"],
            Self::E_VAL_MISSING_REQUIRED_FIELD => &["name", "field"],
            Self::E_VAL_TYPE_MISMATCH => &["message"],
            Self::E_VAL_UNKNOWN_NODE_TYPE => &["name", "type"],
            // HTTP
            Self::E_HTTP_REQUEST_FAILED => &["url", "method", "message"],
            Self::E_HTTP_STATUS_ERROR => &["url", "status", "message"],
            Self::E_HTTP_INVALID_URL => &["url", "message"],
            // WebSocket
            Self::E_WS_CONNECTION => &["url", "message"],
            Self::E_WS_TIMEOUT => &["url", "duration"],
            Self::E_WS_MESSAGE => &["url", "message"],
            Self::E_WS_CLOSE => &["url", "code", "reason"],
            Self::E_WS_PROTOCOL => &["url", "message"],
            // Database
            Self::E_DB_CONNECTION => &["driver", "message"],
            Self::E_DB_QUERY => &["driver", "query", "message"],
            Self::E_DB_TIMEOUT => &["driver", "duration"],
            // Script
            Self::E_SCRIPT_SYNTAX => &["message"],
            Self::E_SCRIPT_RUNTIME => &["message"],
            Self::E_SCRIPT_TIMEOUT => &["duration"],
            Self::E_SCRIPT_ASSERTION => &["expected", "actual", "message"],
            // gRPC
            Self::E_GRPC_CONNECTION => &["host", "port", "message"],
            Self::E_GRPC_TIMEOUT => &["host", "duration"],
            Self::E_GRPC_MESSAGE => &["host", "message"],
            Self::E_GRPC_STATUS => &["host", "status", "message"],
            Self::E_GRPC_CLOSE => &["host", "message"],
            // MCP
            Self::E_MCP_CONNECTION => &["server", "message"],
            Self::E_MCP_TOOL_CALL => &["tool", "message"],
            Self::E_MCP_RESOURCE_READ => &["resource", "message"],
            Self::E_MCP_NOT_CONNECTED => &["server"],
            // TCP
            Self::E_TCP_CONNECTION => &["host", "port", "message"],
            Self::E_TCP_TIMEOUT => &["host", "port", "duration"],
            Self::E_TCP_SEND => &["host", "port", "message"],
            Self::E_TCP_RECEIVE => &["host", "port", "message"],
            Self::E_TCP_CLOSE => &["host", "port", "message"],
            // UDP
            Self::E_UDP_BIND => &["host", "port", "message"],
            Self::E_UDP_SEND => &["host", "port", "message"],
            Self::E_UDP_RECEIVE => &["host", "port", "message"],
            Self::E_UDP_CLOSE => &["host", "port"],
            // SSE
            Self::E_SSE_CONNECTION => &["url", "message"],
            Self::E_SSE_TIMEOUT => &["url", "duration"],
            Self::E_SSE_EVENT => &["url", "message"],
            // GraphQL
            Self::E_GRAPHQL_CONNECTION => &["url", "message"],
            Self::E_GRAPHQL_QUERY => &["url", "message"],
            Self::E_GRAPHQL_SUBSCRIPTION => &["url", "message"],
            Self::E_GRAPHQL_INTROSPECTION => &["url", "message"],
            Self::E_GRAPHQL_CLOSE => &["url", "message"],
            // AMQP
            Self::E_AMQP_CONNECTION => &["host", "vhost", "message"],
            Self::E_AMQP_CHANNEL => &["host", "channel_id", "message"],
            Self::E_AMQP_PUBLISH => &["exchange", "routing_key", "message"],
            Self::E_AMQP_CONSUME => &["queue", "message"],
            Self::E_AMQP_QUEUE => &["queue", "message"],
            Self::E_AMQP_EXCHANGE => &["exchange", "message"],
            Self::E_AMQP_ACK => &["queue", "message"],
            Self::E_AMQP_MANAGEMENT => &["message"],
            Self::E_AMQP_PROTOCOL => &["message"],
            // Redis
            Self::E_REDIS_CONNECTION => &["host", "port", "message"],
            Self::E_REDIS_COMMAND => &["command", "message"],
            Self::E_REDIS_TIMEOUT => &["host", "duration"],
            Self::E_REDIS_AUTH => &["host", "message"],
            // SMTP
            Self::E_SMTP_CONNECTION => &["host", "port", "message"],
            Self::E_SMTP_SEND => &["host", "message"],
            Self::E_SMTP_AUTH => &["host", "message"],
            Self::E_SMTP_TIMEOUT => &["host", "duration"],
            // IMAP
            Self::E_IMAP_CONNECTION => &["host", "port", "message"],
            Self::E_IMAP_FETCH => &["host", "message"],
            Self::E_IMAP_SEARCH => &["host", "message"],
            Self::E_IMAP_AUTH => &["host", "message"],
            Self::E_IMAP_STORE => &["host", "message"],
            Self::E_IMAP_TIMEOUT => &["host", "duration"],
            // Non-protocol nodes
            Self::E_VARIABLE_EXTRACTOR_FAILED => &["expression", "message"],
            Self::E_PROXY_FAILED => &["message"],
            // Plugin
            Self::PluginCrashed => &["plugin_name", "message"],
        }
    }

    /// Returns the i18n translation key (same as `SCREAMING_SNAKE_CASE` serialized name).
    pub fn i18n_key(&self) -> &str {
        match self {
            Self::E_CONFIG_ERROR => "E_CONFIG_ERROR",
            Self::E_CONDITION_SYNTAX => "E_CONDITION_SYNTAX",
            Self::E_EXECUTION_ERROR => "E_EXECUTION_ERROR",
            Self::E_TIMEOUT => "E_TIMEOUT",
            Self::E_CONDITION_EVALUATION => "E_CONDITION_EVALUATION",
            Self::E_UNKNOWN => "E_UNKNOWN",
            Self::E_CANCELLED => "E_CANCELLED",
            Self::E_PARSE => "E_PARSE",
            Self::E_ASSERTION => "E_ASSERTION",
            Self::E_SCRIPT_ABORT => "E_SCRIPT_ABORT",
            Self::E_CONN_TIMEOUT => "E_CONN_TIMEOUT",
            Self::E_CONN_REFUSED => "E_CONN_REFUSED",
            Self::E_CONN_CLOSED => "E_CONN_CLOSED",
            Self::E_VAL_PORT_DUPLICATE => "E_VAL_PORT_DUPLICATE",
            Self::E_VAL_CYCLE_DETECTED => "E_VAL_CYCLE_DETECTED",
            Self::E_VAL_MISSING_ENTRY => "E_VAL_MISSING_ENTRY",
            Self::E_VAL_DUPLICATE_NODE_ID => "E_VAL_DUPLICATE_NODE_ID",
            Self::E_VAL_CONN_NODE_NOT_FOUND => "E_VAL_CONN_NODE_NOT_FOUND",
            Self::E_VAL_PORT_NOT_FOUND => "E_VAL_PORT_NOT_FOUND",
            Self::E_VAL_NODE_UNREACHABLE => "E_VAL_NODE_UNREACHABLE",
            Self::E_VAL_UNKNOWN_FIELD => "E_VAL_UNKNOWN_FIELD",
            Self::E_VAL_MISSING_REQUIRED_FIELD => "E_VAL_MISSING_REQUIRED_FIELD",
            Self::E_VAL_TYPE_MISMATCH => "E_VAL_TYPE_MISMATCH",
            Self::E_VAL_UNKNOWN_NODE_TYPE => "E_VAL_UNKNOWN_NODE_TYPE",
            Self::E_HTTP_REQUEST_FAILED => "E_HTTP_REQUEST_FAILED",
            Self::E_HTTP_STATUS_ERROR => "E_HTTP_STATUS_ERROR",
            Self::E_HTTP_INVALID_URL => "E_HTTP_INVALID_URL",
            Self::E_WS_CONNECTION => "E_WS_CONNECTION",
            Self::E_WS_TIMEOUT => "E_WS_TIMEOUT",
            Self::E_WS_MESSAGE => "E_WS_MESSAGE",
            Self::E_WS_CLOSE => "E_WS_CLOSE",
            Self::E_WS_PROTOCOL => "E_WS_PROTOCOL",
            Self::E_DB_CONNECTION => "E_DB_CONNECTION",
            Self::E_DB_QUERY => "E_DB_QUERY",
            Self::E_DB_TIMEOUT => "E_DB_TIMEOUT",
            Self::E_SCRIPT_SYNTAX => "E_SCRIPT_SYNTAX",
            Self::E_SCRIPT_RUNTIME => "E_SCRIPT_RUNTIME",
            Self::E_SCRIPT_TIMEOUT => "E_SCRIPT_TIMEOUT",
            Self::E_SCRIPT_ASSERTION => "E_SCRIPT_ASSERTION",
            Self::E_GRPC_CONNECTION => "E_GRPC_CONNECTION",
            Self::E_GRPC_TIMEOUT => "E_GRPC_TIMEOUT",
            Self::E_GRPC_MESSAGE => "E_GRPC_MESSAGE",
            Self::E_GRPC_STATUS => "E_GRPC_STATUS",
            Self::E_GRPC_CLOSE => "E_GRPC_CLOSE",
            Self::E_MCP_CONNECTION => "E_MCP_CONNECTION",
            Self::E_MCP_TOOL_CALL => "E_MCP_TOOL_CALL",
            Self::E_MCP_RESOURCE_READ => "E_MCP_RESOURCE_READ",
            Self::E_MCP_NOT_CONNECTED => "E_MCP_NOT_CONNECTED",
            Self::E_TCP_CONNECTION => "E_TCP_CONNECTION",
            Self::E_TCP_TIMEOUT => "E_TCP_TIMEOUT",
            Self::E_TCP_SEND => "E_TCP_SEND",
            Self::E_TCP_RECEIVE => "E_TCP_RECEIVE",
            Self::E_TCP_CLOSE => "E_TCP_CLOSE",
            Self::E_UDP_BIND => "E_UDP_BIND",
            Self::E_UDP_SEND => "E_UDP_SEND",
            Self::E_UDP_RECEIVE => "E_UDP_RECEIVE",
            Self::E_UDP_CLOSE => "E_UDP_CLOSE",
            Self::E_SSE_CONNECTION => "E_SSE_CONNECTION",
            Self::E_SSE_TIMEOUT => "E_SSE_TIMEOUT",
            Self::E_SSE_EVENT => "E_SSE_EVENT",
            Self::E_GRAPHQL_CONNECTION => "E_GRAPHQL_CONNECTION",
            Self::E_GRAPHQL_QUERY => "E_GRAPHQL_QUERY",
            Self::E_GRAPHQL_SUBSCRIPTION => "E_GRAPHQL_SUBSCRIPTION",
            Self::E_GRAPHQL_INTROSPECTION => "E_GRAPHQL_INTROSPECTION",
            Self::E_GRAPHQL_CLOSE => "E_GRAPHQL_CLOSE",
            Self::E_AMQP_CONNECTION => "E_AMQP_CONNECTION",
            Self::E_AMQP_CHANNEL => "E_AMQP_CHANNEL",
            Self::E_AMQP_PUBLISH => "E_AMQP_PUBLISH",
            Self::E_AMQP_CONSUME => "E_AMQP_CONSUME",
            Self::E_AMQP_QUEUE => "E_AMQP_QUEUE",
            Self::E_AMQP_EXCHANGE => "E_AMQP_EXCHANGE",
            Self::E_AMQP_ACK => "E_AMQP_ACK",
            Self::E_AMQP_MANAGEMENT => "E_AMQP_MANAGEMENT",
            Self::E_AMQP_PROTOCOL => "E_AMQP_PROTOCOL",
            Self::E_REDIS_CONNECTION => "E_REDIS_CONNECTION",
            Self::E_REDIS_COMMAND => "E_REDIS_COMMAND",
            Self::E_REDIS_TIMEOUT => "E_REDIS_TIMEOUT",
            Self::E_REDIS_AUTH => "E_REDIS_AUTH",
            Self::E_SMTP_CONNECTION => "E_SMTP_CONNECTION",
            Self::E_SMTP_SEND => "E_SMTP_SEND",
            Self::E_SMTP_AUTH => "E_SMTP_AUTH",
            Self::E_SMTP_TIMEOUT => "E_SMTP_TIMEOUT",
            Self::E_IMAP_CONNECTION => "E_IMAP_CONNECTION",
            Self::E_IMAP_FETCH => "E_IMAP_FETCH",
            Self::E_IMAP_SEARCH => "E_IMAP_SEARCH",
            Self::E_IMAP_AUTH => "E_IMAP_AUTH",
            Self::E_IMAP_STORE => "E_IMAP_STORE",
            Self::E_IMAP_TIMEOUT => "E_IMAP_TIMEOUT",
            Self::E_VARIABLE_EXTRACTOR_FAILED => "E_VARIABLE_EXTRACTOR_FAILED",
            Self::E_PROXY_FAILED => "E_PROXY_FAILED",
            Self::PluginCrashed => "E_PLUGIN_CRASHED",
        }
    }

    /// Returns the associated `ErrorKind` for this error code.
    pub fn error_kind_hint(&self) -> ErrorKind {
        match self {
            Self::E_CONFIG_ERROR => ErrorKind::Configuration,
            Self::E_CONDITION_SYNTAX => ErrorKind::ConditionSyntax,
            Self::E_EXECUTION_ERROR => ErrorKind::Execution,
            Self::E_TIMEOUT => ErrorKind::Timeout,
            Self::E_CONDITION_EVALUATION => ErrorKind::ConditionEvaluation,
            Self::E_UNKNOWN => ErrorKind::Execution,
            Self::E_CANCELLED => ErrorKind::Cancelled,
            Self::E_PARSE => ErrorKind::Parse,
            Self::E_ASSERTION => ErrorKind::Assertion,
            Self::E_SCRIPT_ABORT => ErrorKind::ScriptAbort,
            Self::E_CONN_TIMEOUT => ErrorKind::Timeout,
            Self::E_CONN_REFUSED => ErrorKind::Connection,
            Self::E_CONN_CLOSED => ErrorKind::Connection,
            Self::E_VAL_PORT_DUPLICATE => ErrorKind::Validation,
            Self::E_VAL_CYCLE_DETECTED => ErrorKind::Validation,
            Self::E_VAL_MISSING_ENTRY => ErrorKind::Validation,
            Self::E_VAL_DUPLICATE_NODE_ID => ErrorKind::Validation,
            Self::E_VAL_CONN_NODE_NOT_FOUND => ErrorKind::Validation,
            Self::E_VAL_PORT_NOT_FOUND => ErrorKind::Validation,
            Self::E_VAL_NODE_UNREACHABLE => ErrorKind::Validation,
            Self::E_VAL_UNKNOWN_FIELD => ErrorKind::Validation,
            Self::E_VAL_MISSING_REQUIRED_FIELD => ErrorKind::Validation,
            Self::E_VAL_TYPE_MISMATCH => ErrorKind::Validation,
            Self::E_VAL_UNKNOWN_NODE_TYPE => ErrorKind::Validation,
            Self::E_HTTP_REQUEST_FAILED => ErrorKind::Execution,
            Self::E_HTTP_STATUS_ERROR => ErrorKind::Execution,
            Self::E_HTTP_INVALID_URL => ErrorKind::Validation,
            Self::E_WS_CONNECTION => ErrorKind::WebSocketConnection,
            Self::E_WS_TIMEOUT => ErrorKind::WebSocketTimeout,
            Self::E_WS_MESSAGE => ErrorKind::WebSocketMessage,
            Self::E_WS_CLOSE => ErrorKind::WebSocketClose,
            Self::E_WS_PROTOCOL => ErrorKind::WebSocketProtocol,
            Self::E_DB_CONNECTION => ErrorKind::Connection,
            Self::E_DB_QUERY => ErrorKind::Execution,
            Self::E_DB_TIMEOUT => ErrorKind::Timeout,
            Self::E_SCRIPT_SYNTAX => ErrorKind::ScriptSyntax,
            Self::E_SCRIPT_RUNTIME => ErrorKind::ScriptRuntime,
            Self::E_SCRIPT_TIMEOUT => ErrorKind::ScriptTimeout,
            Self::E_SCRIPT_ASSERTION => ErrorKind::ScriptAssertion,
            Self::E_GRPC_CONNECTION => ErrorKind::GrpcConnection,
            Self::E_GRPC_TIMEOUT => ErrorKind::GrpcTimeout,
            Self::E_GRPC_MESSAGE => ErrorKind::GrpcMessage,
            Self::E_GRPC_STATUS => ErrorKind::GrpcStatus,
            Self::E_GRPC_CLOSE => ErrorKind::GrpcClose,
            Self::E_MCP_CONNECTION => ErrorKind::McpConnection,
            Self::E_MCP_TOOL_CALL => ErrorKind::McpToolCall,
            Self::E_MCP_RESOURCE_READ => ErrorKind::McpResourceRead,
            Self::E_MCP_NOT_CONNECTED => ErrorKind::McpNotConnected,
            Self::E_TCP_CONNECTION => ErrorKind::TcpConnection,
            Self::E_TCP_TIMEOUT => ErrorKind::TcpTimeout,
            Self::E_TCP_SEND => ErrorKind::TcpSend,
            Self::E_TCP_RECEIVE => ErrorKind::TcpReceive,
            Self::E_TCP_CLOSE => ErrorKind::TcpClose,
            Self::E_UDP_BIND => ErrorKind::UdpBind,
            Self::E_UDP_SEND => ErrorKind::UdpSend,
            Self::E_UDP_RECEIVE => ErrorKind::UdpReceive,
            Self::E_UDP_CLOSE => ErrorKind::UdpClose,
            Self::E_SSE_CONNECTION => ErrorKind::SseConnection,
            Self::E_SSE_TIMEOUT => ErrorKind::SseTimeout,
            Self::E_SSE_EVENT => ErrorKind::SseEvent,
            Self::E_GRAPHQL_CONNECTION => ErrorKind::GraphqlConnection,
            Self::E_GRAPHQL_QUERY => ErrorKind::GraphqlQuery,
            Self::E_GRAPHQL_SUBSCRIPTION => ErrorKind::GraphqlSubscription,
            Self::E_GRAPHQL_INTROSPECTION => ErrorKind::GraphqlIntrospection,
            Self::E_GRAPHQL_CLOSE => ErrorKind::GraphqlClose,
            Self::E_AMQP_CONNECTION => ErrorKind::AmqpConnection,
            Self::E_AMQP_CHANNEL => ErrorKind::AmqpChannel,
            Self::E_AMQP_PUBLISH => ErrorKind::AmqpPublish,
            Self::E_AMQP_CONSUME => ErrorKind::AmqpConsume,
            Self::E_AMQP_QUEUE => ErrorKind::AmqpQueue,
            Self::E_AMQP_EXCHANGE => ErrorKind::AmqpExchange,
            Self::E_AMQP_ACK => ErrorKind::AmqpAck,
            Self::E_AMQP_MANAGEMENT => ErrorKind::AmqpManagement,
            Self::E_AMQP_PROTOCOL => ErrorKind::AmqpProtocol,
            Self::E_REDIS_CONNECTION => ErrorKind::Connection,
            Self::E_REDIS_COMMAND => ErrorKind::Execution,
            Self::E_REDIS_TIMEOUT => ErrorKind::Timeout,
            Self::E_REDIS_AUTH => ErrorKind::Execution,
            Self::E_SMTP_CONNECTION => ErrorKind::Connection,
            Self::E_SMTP_SEND => ErrorKind::Execution,
            Self::E_SMTP_AUTH => ErrorKind::Execution,
            Self::E_SMTP_TIMEOUT => ErrorKind::Timeout,
            Self::E_IMAP_CONNECTION => ErrorKind::Connection,
            Self::E_IMAP_FETCH => ErrorKind::Execution,
            Self::E_IMAP_SEARCH => ErrorKind::Execution,
            Self::E_IMAP_AUTH => ErrorKind::Execution,
            Self::E_IMAP_STORE => ErrorKind::Execution,
            Self::E_IMAP_TIMEOUT => ErrorKind::Timeout,
            Self::E_VARIABLE_EXTRACTOR_FAILED => ErrorKind::Execution,
            Self::E_PROXY_FAILED => ErrorKind::Execution,
            Self::PluginCrashed => ErrorKind::Execution,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::E_CONFIG_ERROR => write!(f, "Configuration error"),
            Self::E_CONDITION_SYNTAX => write!(f, "Condition expression syntax error"),
            Self::E_EXECUTION_ERROR => write!(f, "Execution error"),
            Self::E_TIMEOUT => write!(f, "Operation timed out"),
            Self::E_CONDITION_EVALUATION => write!(f, "Condition evaluation error"),
            Self::E_UNKNOWN => write!(f, "Unknown error"),
            Self::E_CANCELLED => write!(f, "Execution cancelled"),
            Self::E_PARSE => write!(f, "Parse error"),
            Self::E_ASSERTION => write!(f, "Assertion failed"),
            Self::E_SCRIPT_ABORT => write!(f, "Script execution aborted"),
            Self::E_CONN_TIMEOUT => write!(f, "Connection timed out"),
            Self::E_CONN_REFUSED => write!(f, "Connection refused"),
            Self::E_CONN_CLOSED => write!(f, "Connection closed"),
            Self::E_VAL_PORT_DUPLICATE => write!(f, "Duplicate port detected"),
            Self::E_VAL_CYCLE_DETECTED => write!(f, "Cycle detected in flow"),
            Self::E_VAL_MISSING_ENTRY => write!(f, "Missing entry node"),
            Self::E_VAL_DUPLICATE_NODE_ID => write!(f, "Duplicate node UUID detected"),
            Self::E_VAL_CONN_NODE_NOT_FOUND => write!(f, "Connection references non-existent node"),
            Self::E_VAL_PORT_NOT_FOUND => write!(f, "Connection references non-existent port"),
            Self::E_VAL_NODE_UNREACHABLE => write!(f, "Node is not reachable from any Entry node"),
            Self::E_VAL_UNKNOWN_FIELD => write!(f, "Node data contains unknown fields"),
            Self::E_VAL_MISSING_REQUIRED_FIELD => write!(f, "Node is missing a required field"),
            Self::E_VAL_TYPE_MISMATCH => write!(f, "Node data field type mismatch"),
            Self::E_VAL_UNKNOWN_NODE_TYPE => write!(f, "Unknown or unsupported node type"),
            Self::E_HTTP_REQUEST_FAILED => write!(f, "HTTP request failed"),
            Self::E_HTTP_STATUS_ERROR => write!(f, "HTTP status error"),
            Self::E_HTTP_INVALID_URL => write!(f, "Invalid HTTP URL"),
            Self::E_WS_CONNECTION => write!(f, "WebSocket connection failed"),
            Self::E_WS_TIMEOUT => write!(f, "WebSocket operation timed out"),
            Self::E_WS_MESSAGE => write!(f, "WebSocket message error"),
            Self::E_WS_CLOSE => write!(f, "WebSocket close error"),
            Self::E_WS_PROTOCOL => write!(f, "WebSocket protocol error"),
            Self::E_DB_CONNECTION => write!(f, "Database connection failed"),
            Self::E_DB_QUERY => write!(f, "Database query error"),
            Self::E_DB_TIMEOUT => write!(f, "Database operation timed out"),
            Self::E_SCRIPT_SYNTAX => write!(f, "Script syntax error"),
            Self::E_SCRIPT_RUNTIME => write!(f, "Script runtime error"),
            Self::E_SCRIPT_TIMEOUT => write!(f, "Script execution timed out"),
            Self::E_SCRIPT_ASSERTION => write!(f, "Script assertion failed"),
            Self::E_GRPC_CONNECTION => write!(f, "gRPC connection failed"),
            Self::E_GRPC_TIMEOUT => write!(f, "gRPC operation timed out"),
            Self::E_GRPC_MESSAGE => write!(f, "gRPC message error"),
            Self::E_GRPC_STATUS => write!(f, "gRPC status error"),
            Self::E_GRPC_CLOSE => write!(f, "gRPC close error"),
            Self::E_MCP_CONNECTION => write!(f, "MCP connection failed"),
            Self::E_MCP_TOOL_CALL => write!(f, "MCP tool call failed"),
            Self::E_MCP_RESOURCE_READ => write!(f, "MCP resource read failed"),
            Self::E_MCP_NOT_CONNECTED => write!(f, "MCP not connected"),
            Self::E_TCP_CONNECTION => write!(f, "TCP connection failed"),
            Self::E_TCP_TIMEOUT => write!(f, "TCP operation timed out"),
            Self::E_TCP_SEND => write!(f, "TCP send error"),
            Self::E_TCP_RECEIVE => write!(f, "TCP receive error"),
            Self::E_TCP_CLOSE => write!(f, "TCP close error"),
            Self::E_UDP_BIND => write!(f, "UDP bind failed"),
            Self::E_UDP_SEND => write!(f, "UDP send error"),
            Self::E_UDP_RECEIVE => write!(f, "UDP receive error"),
            Self::E_UDP_CLOSE => write!(f, "UDP close error"),
            Self::E_SSE_CONNECTION => write!(f, "SSE connection failed"),
            Self::E_SSE_TIMEOUT => write!(f, "SSE operation timed out"),
            Self::E_SSE_EVENT => write!(f, "SSE event error"),
            Self::E_GRAPHQL_CONNECTION => write!(f, "GraphQL connection failed"),
            Self::E_GRAPHQL_QUERY => write!(f, "GraphQL query error"),
            Self::E_GRAPHQL_SUBSCRIPTION => write!(f, "GraphQL subscription error"),
            Self::E_GRAPHQL_INTROSPECTION => write!(f, "GraphQL introspection error"),
            Self::E_GRAPHQL_CLOSE => write!(f, "GraphQL close error"),
            Self::E_AMQP_CONNECTION => write!(f, "AMQP connection failed"),
            Self::E_AMQP_CHANNEL => write!(f, "AMQP channel error"),
            Self::E_AMQP_PUBLISH => write!(f, "AMQP publish error"),
            Self::E_AMQP_CONSUME => write!(f, "AMQP consume error"),
            Self::E_AMQP_QUEUE => write!(f, "AMQP queue error"),
            Self::E_AMQP_EXCHANGE => write!(f, "AMQP exchange error"),
            Self::E_AMQP_ACK => write!(f, "AMQP ack error"),
            Self::E_AMQP_MANAGEMENT => write!(f, "AMQP management error"),
            Self::E_AMQP_PROTOCOL => write!(f, "AMQP protocol error"),
            Self::E_REDIS_CONNECTION => write!(f, "Redis connection failed"),
            Self::E_REDIS_COMMAND => write!(f, "Redis command error"),
            Self::E_REDIS_TIMEOUT => write!(f, "Redis operation timed out"),
            Self::E_REDIS_AUTH => write!(f, "Redis authentication failed"),
            Self::E_SMTP_CONNECTION => write!(f, "SMTP connection failed"),
            Self::E_SMTP_SEND => write!(f, "SMTP send error"),
            Self::E_SMTP_AUTH => write!(f, "SMTP authentication failed"),
            Self::E_SMTP_TIMEOUT => write!(f, "SMTP operation timed out"),
            Self::E_IMAP_CONNECTION => write!(f, "IMAP connection failed"),
            Self::E_IMAP_FETCH => write!(f, "IMAP fetch error"),
            Self::E_IMAP_SEARCH => write!(f, "IMAP search error"),
            Self::E_IMAP_AUTH => write!(f, "IMAP authentication failed"),
            Self::E_IMAP_STORE => write!(f, "IMAP store error"),
            Self::E_IMAP_TIMEOUT => write!(f, "IMAP operation timed out"),
            Self::E_VARIABLE_EXTRACTOR_FAILED => write!(f, "Variable extractor failed"),
            Self::E_PROXY_FAILED => write!(f, "Proxy node execution failed"),
            Self::PluginCrashed => write!(f, "Plugin process crashed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_detail_creation() {
        let error = ErrorDetail::new(ErrorKind::Execution, "execution failed".to_string());

        assert_eq!(error.error_kind, ErrorKind::Execution);
        assert_eq!(error.message, "execution failed");
        assert!(error.error_code.is_none());
        assert!(error.i18n_args.is_none());
        assert!(error.stack_trace.is_none());
        assert!(error.context.is_none());
    }

    #[test]
    fn test_error_detail_builder_pattern() {
        let error = ErrorDetail::new(ErrorKind::Validation, "validation failed".to_string())
            .with_error_code_typed(ErrorCode::E_VAL_PORT_DUPLICATE)
            .with_stack_trace("at line 42".to_string())
            .with_context(serde_json::json!({"field": "email"}));

        assert_eq!(error.error_kind, ErrorKind::Validation);
        assert_eq!(error.error_code, Some(ErrorCode::E_VAL_PORT_DUPLICATE));
        assert_eq!(error.stack_trace, Some("at line 42".to_string()));
        assert_eq!(error.context, Some(serde_json::json!({"field": "email"})));
    }

    #[test]
    fn test_error_detail_serialization() {
        let error = ErrorDetail::new(ErrorKind::Connection, "connection failed".to_string())
            .with_error_code_typed(ErrorCode::E_CONN_REFUSED);

        // serialize
        let json =
            serde_json::to_string(&error).unwrap_or_else(|e| panic!("serialization failed: {e}"));
        assert!(json.contains(r#""connection""#));
        assert!(json.contains("connection failed"));
        assert!(json.contains("E_CONN_REFUSED"));

        // deserialize
        let deserialized: ErrorDetail =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialization failed: {e}"));
        assert_eq!(deserialized.error_kind, ErrorKind::Connection);
        assert_eq!(deserialized.message, "connection failed");
        assert_eq!(deserialized.error_code, Some(ErrorCode::E_CONN_REFUSED));
    }

    #[test]
    fn test_error_detail_with_i18n_arg() {
        let error = ErrorDetail::new(ErrorKind::Connection, "connection failed".to_string())
            .with_error_code_typed(ErrorCode::E_CONN_REFUSED)
            .with_i18n_arg("host", "example.com".to_string())
            .with_i18n_arg("port", "443".to_string());

        assert_eq!(error.error_kind, ErrorKind::Connection);
        assert_eq!(error.error_code, Some(ErrorCode::E_CONN_REFUSED));
        let args = error
            .i18n_args
            .as_ref()
            .unwrap_or_else(|| panic!("i18n_args should be Some"));
        assert_eq!(
            args.get("host").unwrap_or_else(|| panic!("expected Some")),
            "example.com"
        );
        assert_eq!(
            args.get("port").unwrap_or_else(|| panic!("expected Some")),
            "443"
        );
        assert_eq!(args.len(), 2);

        // Serialize and verify roundtrip with i18n_args
        let json = serde_json::to_string(&error).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("E_CONN_REFUSED"));
        assert!(json.contains("example.com"));
        assert!(json.contains("443"));

        let deserialized: ErrorDetail =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.error_code, Some(ErrorCode::E_CONN_REFUSED));
        let deserialized_args = deserialized
            .i18n_args
            .as_ref()
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(
            deserialized_args
                .get("host")
                .unwrap_or_else(|| panic!("expected Some")),
            "example.com"
        );
        assert_eq!(
            deserialized_args
                .get("port")
                .unwrap_or_else(|| panic!("expected Some")),
            "443"
        );
    }

    #[test]
    fn test_error_detail_i18n_args_is_none_by_default() {
        let error = ErrorDetail::new(ErrorKind::Execution, "test".to_string());
        assert!(error.i18n_args.is_none());
    }

    #[test]
    fn test_error_detail_execution_error() {
        let error = ErrorDetail::execution_error("something went wrong".to_string());
        assert_eq!(error.error_kind, ErrorKind::Execution);
        assert_eq!(error.message, "something went wrong");
        assert!(error.error_code.is_none());
        assert!(error.i18n_args.is_none());
    }

    #[test]
    fn test_error_detail_with_error_code_typed_compat() {
        // with_error_code_typed should accept an ErrorCode enum value
        let error = ErrorDetail::new(ErrorKind::Execution, "test".to_string())
            .with_error_code_typed(ErrorCode::E_HTTP_REQUEST_FAILED);
        assert_eq!(error.error_code, Some(ErrorCode::E_HTTP_REQUEST_FAILED));
    }

    #[test]
    fn test_error_detail_with_error_code_typed() {
        let error = ErrorDetail::new(ErrorKind::Timeout, "timeout".to_string())
            .with_error_code_typed(ErrorCode::E_CONN_TIMEOUT);
        assert_eq!(error.error_code, Some(ErrorCode::E_CONN_TIMEOUT));

        // Serialize and verify SCREAMING_SNAKE_CASE
        let json = serde_json::to_string(&error).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("E_CONN_TIMEOUT"));

        // Deserialize and verify roundtrip
        let deserialized: ErrorDetail =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.error_code, Some(ErrorCode::E_CONN_TIMEOUT));
    }

    #[test]
    fn test_error_kind_is_fatal() {
        // fatal errors
        assert!(ErrorKind::Configuration.is_fatal());
        assert!(ErrorKind::ConditionSyntax.is_fatal());
        assert!(!ErrorKind::Execution.is_fatal());
        assert!(!ErrorKind::Connection.is_fatal());
        assert!(!ErrorKind::Timeout.is_fatal());
        assert!(!ErrorKind::ConditionEvaluation.is_fatal());
        assert!(!ErrorKind::Validation.is_fatal());
        assert!(!ErrorKind::Parse.is_fatal());
        // WebSocket errors are non-fatal
        assert!(!ErrorKind::WebSocketConnection.is_fatal());
        assert!(!ErrorKind::WebSocketTimeout.is_fatal());
        assert!(!ErrorKind::WebSocketMessage.is_fatal());
        assert!(!ErrorKind::WebSocketClose.is_fatal());
        assert!(!ErrorKind::WebSocketProtocol.is_fatal());
        // Script errors are non-fatal (config-dependent)
        assert!(!ErrorKind::ScriptSyntax.is_fatal());
        assert!(!ErrorKind::ScriptRuntime.is_fatal());
        assert!(!ErrorKind::ScriptTimeout.is_fatal());
        assert!(!ErrorKind::ScriptAssertion.is_fatal());
        assert!(!ErrorKind::ScriptAbort.is_fatal());
        // TCP/UDP errors are non-fatal
        assert!(!ErrorKind::TcpConnection.is_fatal());
        assert!(!ErrorKind::TcpTimeout.is_fatal());
        assert!(!ErrorKind::TcpSend.is_fatal());
        assert!(!ErrorKind::TcpReceive.is_fatal());
        assert!(!ErrorKind::TcpClose.is_fatal());
        assert!(!ErrorKind::UdpBind.is_fatal());
        assert!(!ErrorKind::UdpSend.is_fatal());
        assert!(!ErrorKind::UdpReceive.is_fatal());
        assert!(!ErrorKind::UdpClose.is_fatal());
        // SSE errors are non-fatal
        assert!(!ErrorKind::SseConnection.is_fatal());
        assert!(!ErrorKind::SseTimeout.is_fatal());
        assert!(!ErrorKind::SseEvent.is_fatal());
        // GraphQL errors are non-fatal
        assert!(!ErrorKind::GraphqlConnection.is_fatal());
        assert!(!ErrorKind::GraphqlQuery.is_fatal());
        assert!(!ErrorKind::GraphqlSubscription.is_fatal());
        assert!(!ErrorKind::GraphqlIntrospection.is_fatal());
        assert!(!ErrorKind::GraphqlClose.is_fatal());
        // AMQP errors are non-fatal
        assert!(!ErrorKind::AmqpConnection.is_fatal());
        assert!(!ErrorKind::AmqpChannel.is_fatal());
        assert!(!ErrorKind::AmqpPublish.is_fatal());
        assert!(!ErrorKind::AmqpConsume.is_fatal());
        assert!(!ErrorKind::AmqpQueue.is_fatal());
        assert!(!ErrorKind::AmqpExchange.is_fatal());
        assert!(!ErrorKind::AmqpAck.is_fatal());
        assert!(!ErrorKind::AmqpManagement.is_fatal());
        assert!(!ErrorKind::AmqpProtocol.is_fatal());
    }

    #[test]
    fn test_error_kind_serialization_lowercase() {
        // verify serialization uses lowercase
        let kinds = vec![
            ErrorKind::Configuration,
            ErrorKind::ConditionSyntax,
            ErrorKind::Execution,
            ErrorKind::Connection,
            ErrorKind::Timeout,
            ErrorKind::ConditionEvaluation,
            ErrorKind::Validation,
            ErrorKind::Parse,
            ErrorKind::WebSocketConnection,
            ErrorKind::WebSocketTimeout,
            ErrorKind::WebSocketMessage,
            ErrorKind::WebSocketClose,
            ErrorKind::WebSocketProtocol,
            // 脚本Error类型
            ErrorKind::ScriptSyntax,
            ErrorKind::ScriptRuntime,
            ErrorKind::ScriptTimeout,
            ErrorKind::ScriptAssertion,
            ErrorKind::ScriptAbort,
            // TCP/UDP Error类型
            ErrorKind::TcpConnection,
            ErrorKind::TcpTimeout,
            ErrorKind::TcpSend,
            ErrorKind::TcpReceive,
            ErrorKind::TcpClose,
            ErrorKind::UdpBind,
            ErrorKind::UdpSend,
            ErrorKind::UdpReceive,
            ErrorKind::UdpClose,
            // SSE Error类型
            ErrorKind::SseConnection,
            ErrorKind::SseTimeout,
            ErrorKind::SseEvent,
            // GraphQL Error类型
            ErrorKind::GraphqlConnection,
            ErrorKind::GraphqlQuery,
            ErrorKind::GraphqlSubscription,
            ErrorKind::GraphqlIntrospection,
            ErrorKind::GraphqlClose,
            // AMQP Error类型
            ErrorKind::AmqpConnection,
            ErrorKind::AmqpChannel,
            ErrorKind::AmqpPublish,
            ErrorKind::AmqpConsume,
            ErrorKind::AmqpQueue,
            ErrorKind::AmqpExchange,
            ErrorKind::AmqpAck,
            ErrorKind::AmqpManagement,
            ErrorKind::AmqpProtocol,
        ];

        for kind in kinds {
            let json = serde_json::to_string(&kind)
                .unwrap_or_else(|e| panic!("serialization failed: {e}"));
            let deserialized: ErrorKind = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialization failed: {e}"));
            assert_eq!(kind, deserialized);

            // verify lowercase format
            let json_str = json.to_lowercase();
            assert_eq!(json, json_str);
        }
    }

    #[test]
    fn test_error_detail_is_fatal() {
        let fatal_error =
            ErrorDetail::new(ErrorKind::Configuration, "configuration error".to_string());
        assert!(fatal_error.is_fatal());

        let recoverable_error =
            ErrorDetail::new(ErrorKind::Execution, "execution error".to_string());
        assert!(!recoverable_error.is_fatal());
    }

    #[test]
    fn test_all_error_kinds() {
        // verify all error kinds can be created and serialized
        let kinds = [
            ErrorKind::Configuration,
            ErrorKind::ConditionSyntax,
            ErrorKind::Execution,
            ErrorKind::Connection,
            ErrorKind::Timeout,
            ErrorKind::ConditionEvaluation,
            ErrorKind::Validation,
            ErrorKind::Parse,
            ErrorKind::WebSocketConnection,
            ErrorKind::WebSocketTimeout,
            ErrorKind::WebSocketMessage,
            ErrorKind::WebSocketClose,
            ErrorKind::WebSocketProtocol,
            // 脚本Error类型
            ErrorKind::ScriptSyntax,
            ErrorKind::ScriptRuntime,
            ErrorKind::ScriptTimeout,
            ErrorKind::ScriptAssertion,
            ErrorKind::ScriptAbort,
            // TCP/UDP Error类型
            ErrorKind::TcpConnection,
            ErrorKind::TcpTimeout,
            ErrorKind::TcpSend,
            ErrorKind::TcpReceive,
            ErrorKind::TcpClose,
            ErrorKind::UdpBind,
            ErrorKind::UdpSend,
            ErrorKind::UdpReceive,
            ErrorKind::UdpClose,
            // SSE Error类型
            ErrorKind::SseConnection,
            ErrorKind::SseTimeout,
            ErrorKind::SseEvent,
            // GraphQL Error类型
            ErrorKind::GraphqlConnection,
            ErrorKind::GraphqlQuery,
            ErrorKind::GraphqlSubscription,
            ErrorKind::GraphqlIntrospection,
            ErrorKind::GraphqlClose,
            // AMQP Error类型
            ErrorKind::AmqpConnection,
            ErrorKind::AmqpChannel,
            ErrorKind::AmqpPublish,
            ErrorKind::AmqpConsume,
            ErrorKind::AmqpQueue,
            ErrorKind::AmqpExchange,
            ErrorKind::AmqpAck,
            ErrorKind::AmqpManagement,
            ErrorKind::AmqpProtocol,
            // 取消
            ErrorKind::Cancelled,
        ];

        for kind in kinds {
            let error = ErrorDetail::new(kind.clone(), format!("{:?} error", kind));

            let json = serde_json::to_string(&error)
                .unwrap_or_else(|e| panic!("serialization failed: {e}"));
            let deserialized: ErrorDetail = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialization failed: {e}"));

            assert_eq!(error.error_kind, deserialized.error_kind);
            assert_eq!(error.message, deserialized.message);
        }
    }

    #[test]
    fn test_websocket_error_kinds() {
        // test WebSocket error kind creation and serialization
        let ws_errors = vec![
            (
                ErrorKind::WebSocketConnection,
                "WebSocket connection failed",
            ),
            (ErrorKind::WebSocketTimeout, "WebSocket operation timed out"),
            (ErrorKind::WebSocketMessage, "WebSocket message send failed"),
            (
                ErrorKind::WebSocketClose,
                "WebSocket connection closed abnormally",
            ),
            (ErrorKind::WebSocketProtocol, "WebSocket protocol error"),
        ];

        for (kind, msg) in ws_errors {
            let error = ErrorDetail::new(kind.clone(), msg.to_string());

            // verify non-fatal
            assert!(!error.is_fatal(), "{:?} should not be fatal", kind);

            // verify serialization roundtrip
            let json = serde_json::to_string(&error)
                .unwrap_or_else(|e| panic!("serialization failed: {e}"));
            let deserialized: ErrorDetail = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialization failed: {e}"));

            assert_eq!(error.error_kind, deserialized.error_kind);
            assert_eq!(error.message, deserialized.message);

            // verify JSON uses lowercase
            assert!(
                json.contains(&format!("{:?}", kind).to_lowercase()),
                "JSON should contain lowercase error kind"
            );
        }
    }

    #[test]
    fn test_script_error_kinds() {
        // test script error kind creation and serialization
        let script_errors = vec![
            (ErrorKind::ScriptSyntax, "script syntax error"),
            (ErrorKind::ScriptRuntime, "script runtime error"),
            (ErrorKind::ScriptTimeout, "script execution timed out"),
            (ErrorKind::ScriptAssertion, "assertion failed"),
            (ErrorKind::ScriptAbort, "user called abort()"),
        ];

        for (kind, msg) in script_errors {
            let error = ErrorDetail::new(kind.clone(), msg.to_string());

            // verify non-fatal (script errors depend on configuration)
            assert!(!error.is_fatal(), "{:?} should not be fatal", kind);

            // verify serialization roundtrip
            let json = serde_json::to_string(&error)
                .unwrap_or_else(|e| panic!("serialization failed: {e}"));
            let deserialized: ErrorDetail = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialization failed: {e}"));

            assert_eq!(error.error_kind, deserialized.error_kind);
            assert_eq!(error.message, deserialized.message);

            // verify JSON uses lowercase
            assert!(
                json.contains(&format!("{:?}", kind).to_lowercase()),
                "JSON should contain lowercase error kind"
            );
        }
    }

    #[test]
    fn test_error_kind_cancelled() {
        assert!(
            !ErrorKind::Cancelled.is_fatal(),
            "Cancelled should not be fatal"
        );

        let error = ErrorDetail::new(ErrorKind::Cancelled, "execution cancelled".to_string());
        assert!(!error.is_fatal());

        let json =
            serde_json::to_string(&error).unwrap_or_else(|e| panic!("serialization failed: {e}"));
        assert!(
            json.contains("cancelled"),
            "should serialize as 'cancelled'"
        );

        let deserialized: ErrorDetail =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialization failed: {e}"));
        assert_eq!(error.error_kind, deserialized.error_kind);
    }

    #[test]
    fn test_tcp_udp_error_kinds() {
        // test TCP/UDP error kind creation and serialization
        let tcp_udp_errors = vec![
            (ErrorKind::TcpConnection, "TCP connection failed"),
            (ErrorKind::TcpTimeout, "TCP operation timed out"),
            (ErrorKind::TcpSend, "TCP send failed"),
            (ErrorKind::TcpReceive, "TCP receive failed"),
            (ErrorKind::TcpClose, "TCP close error"),
            (ErrorKind::UdpBind, "UDP bind failed"),
            (ErrorKind::UdpSend, "UDP send failed"),
            (ErrorKind::UdpReceive, "UDP receive failed"),
            (ErrorKind::UdpClose, "UDP close error"),
        ];

        for (kind, msg) in tcp_udp_errors {
            let error = ErrorDetail::new(kind.clone(), msg.to_string());

            // verify non-fatal
            assert!(!error.is_fatal(), "{:?} should not be fatal", kind);

            // verify serialization roundtrip
            let json = serde_json::to_string(&error)
                .unwrap_or_else(|e| panic!("serialization failed: {e}"));
            let deserialized: ErrorDetail = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialization failed: {e}"));

            assert_eq!(error.error_kind, deserialized.error_kind);
            assert_eq!(error.message, deserialized.message);

            // verify JSON uses lowercase
            assert!(
                json.contains(&format!("{:?}", kind).to_lowercase()),
                "JSON should contain lowercase error kind"
            );
        }
    }

    // ===== ErrorCode tests =====

    /// Helper to collect all `ErrorCode` variants for exhaustive testing.
    fn all_error_codes() -> Vec<ErrorCode> {
        vec![
            // General
            ErrorCode::E_CONFIG_ERROR,
            ErrorCode::E_CONDITION_SYNTAX,
            ErrorCode::E_EXECUTION_ERROR,
            ErrorCode::E_TIMEOUT,
            ErrorCode::E_CONDITION_EVALUATION,
            ErrorCode::E_UNKNOWN,
            ErrorCode::E_CANCELLED,
            ErrorCode::E_PARSE,
            ErrorCode::E_ASSERTION,
            ErrorCode::E_SCRIPT_ABORT,
            // Connection
            ErrorCode::E_CONN_TIMEOUT,
            ErrorCode::E_CONN_REFUSED,
            ErrorCode::E_CONN_CLOSED,
            // Validation
            ErrorCode::E_VAL_PORT_DUPLICATE,
            ErrorCode::E_VAL_CYCLE_DETECTED,
            ErrorCode::E_VAL_MISSING_ENTRY,
            // HTTP
            ErrorCode::E_HTTP_REQUEST_FAILED,
            ErrorCode::E_HTTP_STATUS_ERROR,
            ErrorCode::E_HTTP_INVALID_URL,
            // WebSocket
            ErrorCode::E_WS_CONNECTION,
            ErrorCode::E_WS_TIMEOUT,
            ErrorCode::E_WS_MESSAGE,
            ErrorCode::E_WS_CLOSE,
            ErrorCode::E_WS_PROTOCOL,
            // Database
            ErrorCode::E_DB_CONNECTION,
            ErrorCode::E_DB_QUERY,
            ErrorCode::E_DB_TIMEOUT,
            // Script
            ErrorCode::E_SCRIPT_SYNTAX,
            ErrorCode::E_SCRIPT_RUNTIME,
            ErrorCode::E_SCRIPT_TIMEOUT,
            ErrorCode::E_SCRIPT_ASSERTION,
            // gRPC
            ErrorCode::E_GRPC_CONNECTION,
            ErrorCode::E_GRPC_TIMEOUT,
            ErrorCode::E_GRPC_MESSAGE,
            ErrorCode::E_GRPC_STATUS,
            ErrorCode::E_GRPC_CLOSE,
            // MCP
            ErrorCode::E_MCP_CONNECTION,
            ErrorCode::E_MCP_TOOL_CALL,
            ErrorCode::E_MCP_RESOURCE_READ,
            ErrorCode::E_MCP_NOT_CONNECTED,
            // TCP
            ErrorCode::E_TCP_CONNECTION,
            ErrorCode::E_TCP_TIMEOUT,
            ErrorCode::E_TCP_SEND,
            ErrorCode::E_TCP_RECEIVE,
            ErrorCode::E_TCP_CLOSE,
            // UDP
            ErrorCode::E_UDP_BIND,
            ErrorCode::E_UDP_SEND,
            ErrorCode::E_UDP_RECEIVE,
            ErrorCode::E_UDP_CLOSE,
            // SSE
            ErrorCode::E_SSE_CONNECTION,
            ErrorCode::E_SSE_TIMEOUT,
            ErrorCode::E_SSE_EVENT,
            // GraphQL
            ErrorCode::E_GRAPHQL_CONNECTION,
            ErrorCode::E_GRAPHQL_QUERY,
            ErrorCode::E_GRAPHQL_SUBSCRIPTION,
            ErrorCode::E_GRAPHQL_INTROSPECTION,
            ErrorCode::E_GRAPHQL_CLOSE,
            // AMQP
            ErrorCode::E_AMQP_CONNECTION,
            ErrorCode::E_AMQP_CHANNEL,
            ErrorCode::E_AMQP_PUBLISH,
            ErrorCode::E_AMQP_CONSUME,
            ErrorCode::E_AMQP_QUEUE,
            ErrorCode::E_AMQP_EXCHANGE,
            ErrorCode::E_AMQP_ACK,
            ErrorCode::E_AMQP_MANAGEMENT,
            ErrorCode::E_AMQP_PROTOCOL,
            // Redis
            ErrorCode::E_REDIS_CONNECTION,
            ErrorCode::E_REDIS_COMMAND,
            ErrorCode::E_REDIS_TIMEOUT,
            ErrorCode::E_REDIS_AUTH,
            // SMTP
            ErrorCode::E_SMTP_CONNECTION,
            ErrorCode::E_SMTP_SEND,
            ErrorCode::E_SMTP_AUTH,
            ErrorCode::E_SMTP_TIMEOUT,
            // IMAP
            ErrorCode::E_IMAP_CONNECTION,
            ErrorCode::E_IMAP_FETCH,
            ErrorCode::E_IMAP_SEARCH,
            ErrorCode::E_IMAP_AUTH,
            ErrorCode::E_IMAP_STORE,
            ErrorCode::E_IMAP_TIMEOUT,
            // Non-protocol nodes
            ErrorCode::E_VARIABLE_EXTRACTOR_FAILED,
            ErrorCode::E_PROXY_FAILED,
            // Plugin
            ErrorCode::PluginCrashed,
        ]
    }

    #[test]
    fn test_error_code_serde_roundtrip() {
        let all_codes = all_error_codes();
        assert_eq!(
            all_codes.len(),
            83,
            "ErrorCode should have exactly 83 variants"
        );

        for code in all_codes {
            let json = serde_json::to_string(&code)
                .unwrap_or_else(|e| panic!("serialization failed: {e}"));
            let deserialized: ErrorCode = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialization failed: {e}"));
            assert_eq!(code, deserialized, "roundtrip failed for {:?}", code);

            // Verify SCREAMING_SNAKE_CASE serialization
            let json_str = json.trim_matches('"');
            assert!(
                json_str.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "ErrorCode should be serialized as SCREAMING_SNAKE_CASE but got: {}",
                json_str
            );
        }
    }

    #[test]
    fn test_error_code_i18n_key_format() {
        for code in all_error_codes() {
            let key = code.i18n_key();
            // Should be all uppercase with underscores
            assert!(
                key.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "i18n_key should be SCREAMING_SNAKE_CASE but got: {}",
                key
            );
            // Should start with E_
            assert!(
                key.starts_with("E_"),
                "i18n_key should start with E_ but got: {}",
                key
            );
        }
    }

    #[test]
    fn test_error_code_display_english_only() {
        for code in all_error_codes() {
            let display = format!("{}", code);
            // Should not contain Chinese characters (CJK Unified Ideographs range)
            assert!(
                !display
                    .chars()
                    .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c)),
                "Display output should be English only but got: {}",
                display
            );
            // Should not be empty
            assert!(!display.is_empty(), "Display output should not be empty");
        }
    }

    #[test]
    fn test_error_code_error_kind_hint() {
        use ErrorKind::*;
        for code in all_error_codes() {
            let hint = code.error_kind_hint();
            // Verify the hint is a valid ErrorKind (non-panicking)
            let _ = format!("{:?}", hint);
        }

        // Spot-check specific mappings
        assert_eq!(ErrorCode::E_CONFIG_ERROR.error_kind_hint(), Configuration);
        assert_eq!(
            ErrorCode::E_CONDITION_SYNTAX.error_kind_hint(),
            ConditionSyntax
        );
        assert_eq!(ErrorCode::E_EXECUTION_ERROR.error_kind_hint(), Execution);
        assert_eq!(ErrorCode::E_CANCELLED.error_kind_hint(), Cancelled);
        assert_eq!(
            ErrorCode::E_WS_CONNECTION.error_kind_hint(),
            WebSocketConnection
        );
        assert_eq!(ErrorCode::E_GRPC_TIMEOUT.error_kind_hint(), GrpcTimeout);
        assert_eq!(ErrorCode::E_MCP_TOOL_CALL.error_kind_hint(), McpToolCall);
        assert_eq!(ErrorCode::E_TCP_SEND.error_kind_hint(), TcpSend);
        assert_eq!(ErrorCode::E_UDP_BIND.error_kind_hint(), UdpBind);
        assert_eq!(ErrorCode::E_SSE_EVENT.error_kind_hint(), SseEvent);
        assert_eq!(ErrorCode::E_GRAPHQL_QUERY.error_kind_hint(), GraphqlQuery);
        assert_eq!(ErrorCode::E_AMQP_PUBLISH.error_kind_hint(), AmqpPublish);
        assert_eq!(ErrorCode::E_REDIS_CONNECTION.error_kind_hint(), Connection);
        assert_eq!(ErrorCode::E_SMTP_AUTH.error_kind_hint(), Execution);
        assert_eq!(ErrorCode::E_IMAP_FETCH.error_kind_hint(), Execution);
        assert_eq!(
            ErrorCode::E_VARIABLE_EXTRACTOR_FAILED.error_kind_hint(),
            Execution
        );
        assert_eq!(ErrorCode::E_PROXY_FAILED.error_kind_hint(), Execution);
        assert_eq!(
            ErrorCode::E_HTTP_REQUEST_FAILED.error_kind_hint(),
            Execution
        );
    }

    #[test]
    fn test_error_code_expected_i18n_args_non_empty() {
        for code in all_error_codes() {
            let args = code.expected_i18n_args();
            assert!(
                !args.is_empty(),
                "expected_i18n_args should not be empty for {:?}",
                code
            );
            // Each arg key should be non-empty
            for arg in args {
                assert!(!arg.is_empty(), "Empty arg key in {:?}", code);
                // Should be lowercase with underscores (snake_case)
                assert!(
                    arg.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                    "Arg key should be snake_case but got: {}",
                    arg
                );
            }
        }
    }

    #[test]
    fn test_error_code_serialization_screaming_snake() {
        // Verify the JSON serialization is SCREAMING_SNAKE_CASE (different from ErrorKind's lowercase)
        let json = serde_json::to_string(&ErrorCode::E_HTTP_REQUEST_FAILED)
            .unwrap_or_else(|e| panic!("serialization failed: {e}"));
        assert_eq!(json, "\"E_HTTP_REQUEST_FAILED\"");
        assert_ne!(json, "\"e_http_request_failed\"");
        assert_ne!(json, "\"http_request_failed\"");
    }

    /// `ErrorCode::PluginCrashed` 应有合法的 i18n key、Display 输出与 serde 往返
    #[test]
    fn error_plugin_crashed_i18n() {
        // i18n key 与序列化名一致，且符合 SCREAMING_SNAKE_CASE + E_ 前缀约定
        assert_eq!(ErrorCode::PluginCrashed.i18n_key(), "E_PLUGIN_CRASHED");
        assert_eq!(
            serde_json::to_string(&ErrorCode::PluginCrashed)
                .unwrap_or_else(|e| panic!("serialization failed: {e}")),
            "\"E_PLUGIN_CRASHED\""
        );

        // Display 输出为英文且非空
        let display = format!("{}", ErrorCode::PluginCrashed);
        assert!(!display.is_empty(), "Display output should not be empty");

        // expected_i18n_args 非空
        let args = ErrorCode::PluginCrashed.expected_i18n_args();
        assert!(!args.is_empty(), "expected_i18n_args should not be empty");

        // error_kind_hint 非致命
        assert!(!ErrorCode::PluginCrashed.error_kind_hint().is_fatal());

        // serde 往返
        let json = serde_json::to_string(&ErrorCode::PluginCrashed)
            .unwrap_or_else(|e| panic!("serialization failed: {e}"));
        let deserialized: ErrorCode =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialization failed: {e}"));
        assert_eq!(deserialized, ErrorCode::PluginCrashed);
    }

    #[test]
    fn test_error_detail_serde_roundtrip_with_error_code() {
        // Comprehensive roundtrip test with multiple ErrorCode variants in ErrorDetail
        let test_cases = vec![
            (
                ErrorKind::Execution,
                "execution error",
                ErrorCode::E_EXECUTION_ERROR,
            ),
            (
                ErrorKind::Connection,
                "connection refused",
                ErrorCode::E_CONN_REFUSED,
            ),
            (
                ErrorKind::Timeout,
                "operation timed out",
                ErrorCode::E_TIMEOUT,
            ),
            (
                ErrorKind::Validation,
                "port duplicate",
                ErrorCode::E_VAL_PORT_DUPLICATE,
            ),
            (
                ErrorKind::WebSocketConnection,
                "ws connection failed",
                ErrorCode::E_WS_CONNECTION,
            ),
            (
                ErrorKind::ScriptRuntime,
                "script error",
                ErrorCode::E_SCRIPT_RUNTIME,
            ),
        ];

        for (kind, msg, code) in test_cases {
            let error =
                ErrorDetail::new(kind.clone(), msg.to_string()).with_error_code_typed(code.clone());

            let json = serde_json::to_string(&error)
                .unwrap_or_else(|e| panic!("serialization failed: {e}"));
            assert!(
                json.contains(
                    serde_json::to_string(&code)
                        .unwrap_or_else(|e| panic!("{e}"))
                        .trim_matches('"')
                ),
                "JSON should contain ErrorCode variant name for {:?}",
                code
            );

            let deserialized: ErrorDetail = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialization failed: {e}"));
            assert_eq!(deserialized.error_kind, kind);
            assert_eq!(deserialized.error_code, Some(code));
            assert_eq!(deserialized.message, msg);
        }
    }

    #[test]
    fn test_error_detail_i18n_args_serialization() {
        // Verify i18n_args JSON format and roundtrip
        let error = ErrorDetail::new(ErrorKind::Execution, "test".to_string())
            .with_error_code_typed(ErrorCode::E_HTTP_REQUEST_FAILED)
            .with_i18n_arg("url", "https://example.com/api".to_string())
            .with_i18n_arg("method", "POST".to_string())
            .with_i18n_arg("status", "500".to_string());

        let json = serde_json::to_string_pretty(&error).unwrap_or_else(|e| panic!("{e}"));

        // Verify JSON structure has i18n_args as an object with string values
        assert!(json.contains("\"i18n_args\": {"));
        assert!(json.contains("\"url\": \"https://example.com/api\""));
        assert!(json.contains("\"method\": \"POST\""));
        assert!(json.contains("\"status\": \"500\""));
        assert!(json.contains("\"E_HTTP_REQUEST_FAILED\""));

        // Roundtrip
        let deserialized: ErrorDetail =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        let args = deserialized
            .i18n_args
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(
            args.get("url").unwrap_or_else(|| panic!("expected Some")),
            "https://example.com/api"
        );
        assert_eq!(
            args.get("method")
                .unwrap_or_else(|| panic!("expected Some")),
            "POST"
        );
        assert_eq!(
            args.get("status")
                .unwrap_or_else(|| panic!("expected Some")),
            "500"
        );
        assert_eq!(args.len(), 3);
    }
}
