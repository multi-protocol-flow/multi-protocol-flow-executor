use thiserror::Error;

/// HTTP 执行错误类型
#[derive(Debug, Error)]
pub enum HttpError {
    // URL 错误
    /// URL 格式无效或无法解析
    #[error("无效的 URL: {0}")]
    InvalidUrl(String),

    /// 服务器主机名无效（通常为空或包含非法字符）
    #[error("无效的服务器名称: {0}")]
    InvalidServerName(String),

    // DNS 错误
    /// DNS 解析超过指定超时时间
    #[error("DNS 解析超时")]
    DnsTimeout,

    /// DNS 解析过程中发生错误（如网络不可达）
    #[error("DNS 解析失败: {0}")]
    DnsFailed(String),

    /// DNS 解析成功但未返回任何可用地址
    #[error("未找到主机地址")]
    DnsNoAddress,

    // TCP 错误
    /// TCP 连接超过指定超时时间
    #[error("TCP 连接超时")]
    TcpTimeout,

    /// TCP 连接建立失败（如目标拒绝连接）
    #[error("TCP 连接失败: {0}")]
    TcpFailed(String),

    // TLS 错误
    /// TLS 握手超过指定超时时间
    #[error("TLS 握手超时")]
    TlsTimeout,

    /// TLS 握手失败（如证书验证不通过）
    #[error("TLS 握手失败: {0}")]
    TlsFailed(String),

    // HTTP 错误
    /// 写入 HTTP 请求数据到连接时失败
    #[error("写入失败: {0}")]
    WriteFailed(String),

    /// 从连接读取 HTTP 响应数据时失败
    #[error("读取失败: {0}")]
    ReadFailed(String),

    /// 远端服务器在交互过程中意外关闭连接
    #[error("连接意外关闭")]
    ConnectionClosed,

    /// HTTP 响应头格式无效或无法解析
    #[error("无效的响应头: {0}")]
    InvalidHeader(String),

    /// 使用了不支持的 HTTP 方法
    #[error("不支持的方法: {0}")]
    UnsupportedMethod(String),

    /// 构建请求体时失败（如序列化 JSON 或编码 multipart 数据出错）
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
