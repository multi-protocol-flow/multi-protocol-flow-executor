use thiserror::Error;

/// HTTP execution error type
#[derive(Debug, Error)]
pub enum HttpError {
    // URL errors
    /// Invalid or unparseable URL format
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Invalid server hostname (usually empty or contains illegal characters)
    #[error("Invalid server name: {0}")]
    InvalidServerName(String),

    // DNS errors
    /// DNS resolution exceeded the specified timeout
    #[error("DNS resolution timeout")]
    DnsTimeout,

    /// DNS resolution failed (e.g., network unreachable)
    #[error("DNS resolution failed: {0}")]
    DnsFailed(String),

    /// DNS resolved successfully but no addresses returned
    #[error("No host address found")]
    DnsNoAddress,

    // TCP errors
    /// TCP connection exceeded the specified timeout
    #[error("TCP connection timeout")]
    TcpTimeout,

    /// TCP connection failed (e.g., connection refused)
    #[error("TCP connection failed: {0}")]
    TcpFailed(String),

    // TLS errors
    /// TLS handshake exceeded the specified timeout
    #[error("TLS handshake timeout")]
    TlsTimeout,

    /// TLS handshake failed (e.g., certificate validation failure)
    #[error("TLS handshake failed: {0}")]
    TlsFailed(String),

    // HTTP errors
    /// Failed to write HTTP request data to the connection
    #[error("Write failed: {0}")]
    WriteFailed(String),

    /// Failed to read HTTP response data from the connection
    #[error("Read failed: {0}")]
    ReadFailed(String),

    /// Remote server unexpectedly closed the connection
    #[error("Connection unexpectedly closed")]
    ConnectionClosed,

    /// Invalid or unparseable HTTP response header
    #[error("Invalid response header: {0}")]
    InvalidHeader(String),

    /// Unsupported HTTP method used
    #[error("Unsupported method: {0}")]
    UnsupportedMethod(String),

    /// Failed to build request body (e.g., JSON serialization or multipart encoding error)
    #[error("Request body build failed: {0}")]
    BodyBuildFailed(String),

    // Proxy errors
    /// Proxy connection failed (HTTP CONNECT or SOCKS5 handshake failed)
    #[error("Proxy connection failed: {0}")]
    ProxyFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test creation and message formatting of all error variants
    #[test]
    fn test_error_variants_creation() {
        // URL errors
        let err = HttpError::InvalidUrl("bad url".to_string());
        assert_eq!(err.to_string(), "Invalid URL: bad url");

        let err = HttpError::InvalidServerName("example.com".to_string());
        assert_eq!(err.to_string(), "Invalid server name: example.com");

        // DNS errors
        let err = HttpError::DnsTimeout;
        assert_eq!(err.to_string(), "DNS resolution timeout");

        let err = HttpError::DnsFailed("timeout".to_string());
        assert_eq!(err.to_string(), "DNS resolution failed: timeout");

        let err = HttpError::DnsNoAddress;
        assert_eq!(err.to_string(), "No host address found");

        // TCP errors
        let err = HttpError::TcpTimeout;
        assert_eq!(err.to_string(), "TCP connection timeout");

        let err = HttpError::TcpFailed("connection refused".to_string());
        assert_eq!(err.to_string(), "TCP connection failed: connection refused");

        // TLS errors
        let err = HttpError::TlsTimeout;
        assert_eq!(err.to_string(), "TLS handshake timeout");

        let err = HttpError::TlsFailed("cert invalid".to_string());
        assert_eq!(err.to_string(), "TLS handshake failed: cert invalid");

        // HTTP errors
        let err = HttpError::WriteFailed("io error".to_string());
        assert_eq!(err.to_string(), "Write failed: io error");

        let err = HttpError::ReadFailed("eof".to_string());
        assert_eq!(err.to_string(), "Read failed: eof");

        let err = HttpError::ConnectionClosed;
        assert_eq!(err.to_string(), "Connection unexpectedly closed");

        let err = HttpError::InvalidHeader("bad header".to_string());
        assert_eq!(err.to_string(), "Invalid response header: bad header");

        let err = HttpError::UnsupportedMethod("PATCH".to_string());
        assert_eq!(err.to_string(), "Unsupported method: PATCH");

        let err = HttpError::BodyBuildFailed("invalid json".to_string());
        assert_eq!(err.to_string(), "Request body build failed: invalid json");
    }

    /// Test Debug trait implementation
    #[test]
    fn test_error_debug() {
        let err = HttpError::InvalidUrl("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidUrl"));
    }

    /// Test that all error variants produce non-empty English messages
    #[test]
    fn test_error_messages_are_english() {
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
            // All error messages should be non-empty
            assert!(
                !msg.is_empty(),
                "Error message should not be empty: {:?}",
                err
            );
        }
    }

    /// Test error message formatting with dynamic content
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
