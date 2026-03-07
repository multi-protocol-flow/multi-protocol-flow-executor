// 错误模块 - 错误详情和错误类型

use serde::{Deserialize, Serialize};

/// 错误详情结构（用于执行调试）
///
/// 提供结构化的错误信息，包括错误类型、错误代码、消息、堆栈跟踪和上下文快照。
/// 这些信息用于调试和错误分析。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// 错误类型（决定错误处理策略，同时标识具体错误类型）
    pub error_kind: ErrorKind,

    /// 错误代码（可选，用于程序化处理）
    pub error_code: Option<String>,

    /// 错误消息（人类可读）
    pub message: String,

    /// 堆栈跟踪（可选）
    pub stack_trace: Option<String>,

    /// 错误发生时的上下文快照（可选）
    pub context: Option<serde_json::Value>,
}

impl ErrorDetail {
    /// 创建新的错误详情
    ///
    /// # 参数
    ///
    /// * `error_kind` - 错误类型
    /// * `message` - 错误消息
    ///
    /// # 返回
    ///
    /// 新的错误详情，不包含错误代码、堆栈跟踪和上下文
    pub fn new(error_kind: ErrorKind, message: String) -> Self {
        Self {
            error_kind,
            error_code: None,
            message,
            stack_trace: None,
            context: None,
        }
    }

    /// 设置错误代码
    ///
    /// # 参数
    ///
    /// * `code` - 错误代码
    ///
    /// # 返回
    ///
    /// 自身的可变引用，支持链式调用
    pub fn with_error_code(mut self, code: String) -> Self {
        self.error_code = Some(code);
        self
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

    /// 判断是否为致命错误
    ///
    /// # 返回
    ///
    /// 如果是致命错误（需要立即终止流程）则返回 true
    pub fn is_fatal(&self) -> bool {
        self.error_kind.is_fatal()
    }
}

/// 错误类型枚举（决定错误处理策略，同时标识具体错误类型）
///
/// 错误分为两类：
/// - **致命错误**：配置错误和条件语法错误，需要立即终止流程
/// - **可恢复错误**：执行错误、验证错误、解析错误等，在容错模式下可继续执行
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    // ===== 配置错误（立即终止流程） =====
    /// 通用配置错误
    Configuration,

    /// 条件表达式语法错误（如表达式语法错误、字段配置缺失）
    ConditionSyntax,

    // ===== 执行错误（容错模式，继续执行） =====
    /// 通用执行错误
    Execution,

    /// 网络连接失败
    Connection,

    /// 操作超时
    Timeout,

    /// 条件评估运行时错误（如字段不存在、类型不匹配）
    ConditionEvaluation,

    // ===== 验证错误（容错模式，继续执行） =====
    /// 参数验证失败
    Validation,

    // ===== 解析错误（容错模式，继续执行） =====
    /// 数据解析失败（如 JSON 解析失败、数据格式转换失败）
    Parse,

    // ===== WebSocket 错误（容错模式，继续执行） =====
    /// WebSocket 连接失败
    WebSocketConnection,
    /// WebSocket 超时
    WebSocketTimeout,
    /// WebSocket 消息错误
    WebSocketMessage,
    /// WebSocket 关闭错误
    WebSocketClose,
    /// WebSocket 协议错误
    WebSocketProtocol,
}

impl ErrorKind {
    /// 判断是否为致命错误（需要立即终止流程）
    ///
    /// # 返回
    ///
    /// 如果是致命错误则返回 true
    ///
    /// # 致命错误类型
    ///
    /// - `ConfigurationError` - 配置错误
    /// - `ConditionSyntaxError` - 条件语法错误
    pub fn is_fatal(&self) -> bool {
        matches!(self, ErrorKind::Configuration | ErrorKind::ConditionSyntax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_detail_creation() {
        let error = ErrorDetail::new(ErrorKind::Execution, "执行失败".to_string());

        assert_eq!(error.error_kind, ErrorKind::Execution);
        assert_eq!(error.message, "执行失败");
        assert!(error.error_code.is_none());
        assert!(error.stack_trace.is_none());
        assert!(error.context.is_none());
    }

    #[test]
    fn test_error_detail_builder_pattern() {
        let error = ErrorDetail::new(ErrorKind::Validation, "验证失败".to_string())
            .with_error_code("VAL001".to_string())
            .with_stack_trace("at line 42".to_string())
            .with_context(serde_json::json!({"field": "email"}));

        assert_eq!(error.error_kind, ErrorKind::Validation);
        assert_eq!(error.error_code, Some("VAL001".to_string()));
        assert_eq!(error.stack_trace, Some("at line 42".to_string()));
        assert_eq!(error.context, Some(serde_json::json!({"field": "email"})));
    }

    #[test]
    fn test_error_detail_serialization() {
        let error = ErrorDetail::new(ErrorKind::Connection, "连接失败".to_string())
            .with_error_code("CONN001".to_string());

        // 序列化
        let json = serde_json::to_string(&error).expect("序列化失败");
        assert!(json.contains(r#""connection""#));
        assert!(json.contains("连接失败"));

        // 反序列化
        let deserialized: ErrorDetail = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(deserialized.error_kind, ErrorKind::Connection);
        assert_eq!(deserialized.message, "连接失败");
        assert_eq!(deserialized.error_code, Some("CONN001".to_string()));
    }

    #[test]
    fn test_error_kind_is_fatal() {
        // 致命错误
        assert!(ErrorKind::Configuration.is_fatal());
        assert!(ErrorKind::ConditionSyntax.is_fatal());
        assert!(!ErrorKind::Execution.is_fatal());
        assert!(!ErrorKind::Connection.is_fatal());
        assert!(!ErrorKind::Timeout.is_fatal());
        assert!(!ErrorKind::ConditionEvaluation.is_fatal());
        assert!(!ErrorKind::Validation.is_fatal());
        assert!(!ErrorKind::Parse.is_fatal());
        // WebSocket 错误非致命
        assert!(!ErrorKind::WebSocketConnection.is_fatal());
        assert!(!ErrorKind::WebSocketTimeout.is_fatal());
        assert!(!ErrorKind::WebSocketMessage.is_fatal());
        assert!(!ErrorKind::WebSocketClose.is_fatal());
        assert!(!ErrorKind::WebSocketProtocol.is_fatal());
    }

    #[test]
    fn test_error_kind_serialization_lowercase() {
        // 测试序列化使用小写形式
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
        ];

        for kind in kinds {
            let json = serde_json::to_string(&kind).expect("序列化失败");
            let deserialized: ErrorKind = serde_json::from_str(&json).expect("反序列化失败");
            assert_eq!(kind, deserialized);

            // 验证是小写形式
            let json_str = json.to_lowercase();
            assert_eq!(json, json_str);
        }
    }

    #[test]
    fn test_error_detail_is_fatal() {
        let fatal_error = ErrorDetail::new(ErrorKind::Configuration, "配置错误".to_string());
        assert!(fatal_error.is_fatal());

        let recoverable_error = ErrorDetail::new(ErrorKind::Execution, "执行错误".to_string());
        assert!(!recoverable_error.is_fatal());
    }

    #[test]
    fn test_all_error_kinds() {
        // 验证所有错误类型都能正常创建和序列化
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
        ];

        for kind in kinds {
            let error = ErrorDetail::new(kind.clone(), format!("{:?} 错误", kind));

            let json = serde_json::to_string(&error).expect("序列化失败");
            let deserialized: ErrorDetail = serde_json::from_str(&json).expect("反序列化失败");

            assert_eq!(error.error_kind, deserialized.error_kind);
            assert_eq!(error.message, deserialized.message);
        }
    }

    #[test]
    fn test_websocket_error_kinds() {
        // 测试 WebSocket 错误类型的创建和序列化
        let ws_errors = vec![
            (ErrorKind::WebSocketConnection, "WebSocket 连接失败"),
            (ErrorKind::WebSocketTimeout, "WebSocket 操作超时"),
            (ErrorKind::WebSocketMessage, "WebSocket 消息发送失败"),
            (ErrorKind::WebSocketClose, "WebSocket 连接异常关闭"),
            (ErrorKind::WebSocketProtocol, "WebSocket 协议错误"),
        ];

        for (kind, msg) in ws_errors {
            let error = ErrorDetail::new(kind.clone(), msg.to_string());

            // 验证非致命
            assert!(!error.is_fatal(), "{:?} 不应为致命错误", kind);

            // 验证序列化和反序列化
            let json = serde_json::to_string(&error).expect("序列化失败");
            let deserialized: ErrorDetail = serde_json::from_str(&json).expect("反序列化失败");

            assert_eq!(error.error_kind, deserialized.error_kind);
            assert_eq!(error.message, deserialized.message);

            // 验证 JSON 中使用小写形式
            assert!(
                json.contains(&format!("{:?}", kind).to_lowercase()),
                "JSON 应包含小写形式的错误类型"
            );
        }
    }
}
