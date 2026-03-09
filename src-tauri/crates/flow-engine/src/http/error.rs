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
