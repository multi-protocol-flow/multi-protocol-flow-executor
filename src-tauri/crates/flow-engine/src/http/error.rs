use thiserror::Error;

/// HTTP 执行错误类型
#[derive(Debug, Error)]
pub enum HttpError {
    // URL 错误
    #[error("无效的 URL: {0}")]
    InvalidUrl(String),

    #[error("无效的服务器名称: {0}")]
    InvalidServerName(String),

    // DNS 错误
    #[error("DNS 解析超时")]
    DnsTimeout,

    #[error("DNS 解析失败: {0}")]
    DnsFailed(String),

    #[error("未找到主机地址")]
    DnsNoAddress,

    // TCP 错误
    #[error("TCP 连接超时")]
    TcpTimeout,

    #[error("TCP 连接失败: {0}")]
    TcpFailed(String),

    // TLS 错误
    #[error("TLS 握手超时")]
    TlsTimeout,

    #[error("TLS 握手失败: {0}")]
    TlsFailed(String),

    // HTTP 错误
    #[error("写入失败: {0}")]
    WriteFailed(String),

    #[error("读取失败: {0}")]
    ReadFailed(String),

    #[error("连接意外关闭")]
    ConnectionClosed,

    #[error("无效的响应头: {0}")]
    InvalidHeader(String),

    #[error("不支持的方法: {0}")]
    UnsupportedMethod(String),

    #[error("请求体构建失败: {0}")]
    BodyBuildFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试所有错误变体的创建和消息格式化
    #[test]
    fn test_error_variants_creation() {
        // URL 错误
        let err = HttpError::InvalidUrl("bad url".to_string());
        assert_eq!(err.to_string(), "无效的 URL: bad url");

        let err = HttpError::InvalidServerName("example.com".to_string());
        assert_eq!(err.to_string(), "无效的服务器名称: example.com");

        // DNS 错误
        let err = HttpError::DnsTimeout;
        assert_eq!(err.to_string(), "DNS 解析超时");

        let err = HttpError::DnsFailed("timeout".to_string());
        assert_eq!(err.to_string(), "DNS 解析失败: timeout");

        let err = HttpError::DnsNoAddress;
        assert_eq!(err.to_string(), "未找到主机地址");

        // TCP 错误
        let err = HttpError::TcpTimeout;
        assert_eq!(err.to_string(), "TCP 连接超时");

        let err = HttpError::TcpFailed("connection refused".to_string());
        assert_eq!(err.to_string(), "TCP 连接失败: connection refused");

        // TLS 错误
        let err = HttpError::TlsTimeout;
        assert_eq!(err.to_string(), "TLS 握手超时");

        let err = HttpError::TlsFailed("cert invalid".to_string());
        assert_eq!(err.to_string(), "TLS 握手失败: cert invalid");

        // HTTP 错误
        let err = HttpError::WriteFailed("io error".to_string());
        assert_eq!(err.to_string(), "写入失败: io error");

        let err = HttpError::ReadFailed("eof".to_string());
        assert_eq!(err.to_string(), "读取失败: eof");

        let err = HttpError::ConnectionClosed;
        assert_eq!(err.to_string(), "连接意外关闭");

        let err = HttpError::InvalidHeader("bad header".to_string());
        assert_eq!(err.to_string(), "无效的响应头: bad header");

        let err = HttpError::UnsupportedMethod("PATCH".to_string());
        assert_eq!(err.to_string(), "不支持的方法: PATCH");

        let err = HttpError::BodyBuildFailed("invalid json".to_string());
        assert_eq!(err.to_string(), "请求体构建失败: invalid json");
    }

    /// 测试 Debug trait 实现
    #[test]
    fn test_error_debug() {
        let err = HttpError::InvalidUrl("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidUrl"));
    }

    /// 测试错误类型的 Display 实现使用中文消息
    #[test]
    fn test_error_messages_are_chinese() {
        let errors: Vec<HttpError> = vec![
            HttpError::InvalidUrl("".to_string()),
            HttpError::InvalidServerName("".to_string()),
            HttpError::DnsTimeout,
            HttpError::DnsFailed("".to_string()),
            HttpError::DnsNoAddress,
            HttpError::TcpTimeout,
            HttpError::TcpFailed("".to_string()),
            HttpError::TlsTimeout,
            HttpError::TlsFailed("".to_string()),
            HttpError::WriteFailed("".to_string()),
            HttpError::ReadFailed("".to_string()),
            HttpError::ConnectionClosed,
            HttpError::InvalidHeader("".to_string()),
            HttpError::UnsupportedMethod("".to_string()),
            HttpError::BodyBuildFailed("".to_string()),
        ];

        for err in errors {
            let msg = err.to_string();
            // 所有错误消息都应包含中文字符
            assert!(
                msg.chars().any(|c| '\u{4E00}' <= c && c <= '\u{9FFF}'),
                "错误消息应包含中文: {}",
                msg
            );
        }
    }

    /// 测试带参数的错误消息格式化
    #[test]
    fn test_error_with_dynamic_content() {
        let url = "https://example.com/api/v1/users?id=123";
        let err = HttpError::InvalidUrl(url.to_string());
        assert!(err.to_string().contains(url));

        let reason = "certificate expired on 2024-01-01";
        let err = HttpError::TlsFailed(reason.to_string());
        assert!(err.to_string().contains(reason));
    }
}
