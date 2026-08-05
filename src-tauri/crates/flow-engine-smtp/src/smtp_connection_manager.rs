//! SMTP 连接管理器
//!
//! 实现 [`SmtpProvider`] trait，管理 SMTP 连接的生命周期。
//! 使用 lettre [`AsyncSmtpTransport`] 进行 SMTP 通信，
//! 连接池使用 `Arc<RwLock<HashMap>>` 保护。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::AsyncSmtpTransportBuilder;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use log::{info, warn};
use tokio::sync::RwLock;

use flow_engine_core::email_auth::xoauth2_string;
use flow_engine_core::email_types::{EmailAuthConfig, EmailOAuthTokens, SmtpConfig, TlsMode};
use flow_engine_core::traits::{
    SmtpConnectOutput, SmtpDisconnectOutput, SmtpProvider, SmtpSendOutput,
};

// ---------------------------------------------------------------------------
// SmtpConnection
// ---------------------------------------------------------------------------

/// SMTP 连接
///
/// 持有 lettre 异步 SMTP transport 和元数据。
#[allow(dead_code)]
pub struct SmtpConnection {
    /// lettre 异步 SMTP transport
    transport: AsyncSmtpTransport<Tokio1Executor>,
    /// 服务器地址
    host: String,
    /// 端口号
    port: u16,
}

// ---------------------------------------------------------------------------
// SmtpConnectionManager
// ---------------------------------------------------------------------------

/// SMTP 连接池管理器
///
/// 管理流程级别的 SMTP 连接池，实现 [`SmtpProvider`] trait。
/// 线程安全，使用 `Arc<RwLock<HashMap>>` 保护连接池。
pub struct SmtpConnectionManager {
    /// 活跃连接池: `connection_id` → [`SmtpConnection`]
    connections: Arc<RwLock<HashMap<String, SmtpConnection>>>,
}

// ---------------------------------------------------------------------------
// 构造器
// ---------------------------------------------------------------------------

impl SmtpConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for SmtpConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SmtpConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接（非阻塞）
        if let Ok(mut connections) = self.connections.try_write() {
            for (id, conn) in connections.drain() {
                // 注意: AsyncSmtpTransport 没有 abort，直接 drop 即可
                drop(conn);
                info!("SMTP 连接 {} 已随 Manager 释放而清理", id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Transport 构建
// ---------------------------------------------------------------------------

impl SmtpConnectionManager {
    /// 根据 TLS 模式创建 transport builder
    ///
    /// TLS 模式映射:
    /// - `TlsMode::Implicit` → `relay`（隐式 TLS，默认端口 465）
    /// - `TlsMode::StartTls` → `starttls_relay`（STARTTLS，默认端口 587）
    /// - `TlsMode::None` → `builder_dangerous`（明文，默认端口 25）
    fn create_builder(config: &SmtpConfig) -> Result<AsyncSmtpTransportBuilder, String> {
        let host = &config.host;
        let port = Self::effective_port(config);

        let builder = match config.tls_mode {
            TlsMode::Implicit => AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                .map_err(|e| format!("构建 Implicit TLS transport Failed: {}", e))?
                .port(port),
            TlsMode::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                .map_err(|e| format!("构建 StartTLS transport Failed: {}", e))?
                .port(port),
            TlsMode::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port)
            }
        };

        Ok(builder)
    }

    /// 根据认证配置，为 transport builder 设置 credentials 和 mechanism
    ///
    /// 返回配置好认证信息的 builder。
    fn apply_auth(
        builder: AsyncSmtpTransportBuilder,
        auth: &EmailAuthConfig,
    ) -> Result<AsyncSmtpTransportBuilder, String> {
        match auth {
            EmailAuthConfig::Plain {
                username,
                encrypted_password,
            } => {
                let password = Self::decrypt_password(encrypted_password)?;
                let credentials = Credentials::new(username.clone(), password);
                Ok(builder
                    .credentials(credentials)
                    .authentication(vec![Mechanism::Plain]))
            }
            EmailAuthConfig::Login {
                username,
                encrypted_password,
            } => {
                let password = Self::decrypt_password(encrypted_password)?;
                let credentials = Credentials::new(username.clone(), password);
                Ok(builder
                    .credentials(credentials)
                    .authentication(vec![Mechanism::Login]))
            }
            EmailAuthConfig::OAuth2 {
                provider: _,
                client_id: _,
                encrypted_client_secret: _,
                encrypted_tokens,
                method: _,
            } => {
                let tokens_json = flow_engine_crypto::encryption::decrypt(encrypted_tokens)
                    .map_err(|e| format!("解密 OAuth2 token Failed: {}", e))?;
                let tokens: EmailOAuthTokens = serde_json::from_str(&tokens_json)
                    .map_err(|e| format!("解析 OAuth2 token JSON Failed: {}", e))?;

                let xoauth2_str = xoauth2_string("", &tokens.access_token);
                let credentials = Credentials::new(String::new(), xoauth2_str);
                Ok(builder
                    .credentials(credentials)
                    .authentication(vec![Mechanism::Xoauth2]))
            }
        }
    }

    /// 解密密码
    fn decrypt_password(encrypted: &str) -> Result<String, String> {
        if encrypted.is_empty() {
            return Ok(String::new());
        }
        flow_engine_crypto::encryption::decrypt(encrypted)
            .map_err(|e| format!("解密密码Failed: {}", e))
    }

    /// 解析有效的端口号
    fn effective_port(config: &SmtpConfig) -> u16 {
        if config.port > 0 {
            config.port
        } else {
            match config.tls_mode {
                TlsMode::Implicit => 465,
                TlsMode::StartTls => 587,
                TlsMode::None => 25,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SmtpProvider 实现
// ---------------------------------------------------------------------------

#[async_trait]
impl SmtpProvider for SmtpConnectionManager {
    /// 建立 SMTP 连接
    ///
    /// 根据配置构建 transport，测试连接，存入连接池。
    async fn connect(
        &self,
        connection_id: &str,
        config: &SmtpConfig,
    ) -> Result<SmtpConnectOutput, String> {
        info!(
            "SMTP 开始连接: host={}:{}, id={}",
            config.host, config.port, connection_id
        );

        // 连接去重：如果已存在直接返回Success
        {
            let conns = self.connections.read().await;
            if conns.contains_key(connection_id) {
                info!("SMTP 复用已有连接: id={}", connection_id);
                return Ok(SmtpConnectOutput {
                    success: true,
                    connection_id: Some(connection_id.to_string()),
                    server_greeting: None,
                    error: None,
                });
            }
        } // 读锁释放

        // 构建 transport
        let builder = Self::create_builder(config)?;

        // 设置认证（如果配置了）
        let builder = if let Some(ref auth) = config.auth {
            Self::apply_auth(builder, auth)?
        } else {
            builder
        };

        let transport = builder.build::<Tokio1Executor>();

        // 测试连接
        let test_result = transport.test_connection().await;
        match test_result {
            Ok(true) => {
                info!("SMTP 连接测试Success: id={}", connection_id);
            }
            Ok(false) => {
                warn!("SMTP 连接测试Failed: id={}", connection_id);
                return Ok(SmtpConnectOutput {
                    success: false,
                    connection_id: None,
                    server_greeting: None,
                    error: Some("连接测试Failed".to_string()),
                });
            }
            Err(e) => {
                warn!("SMTP 连接测试Error: id={}, error={}", connection_id, e);
                return Ok(SmtpConnectOutput {
                    success: false,
                    connection_id: None,
                    server_greeting: None,
                    error: Some(format!("connection failed: {}", e)),
                });
            }
        }

        let port = Self::effective_port(config);

        let connection = SmtpConnection {
            transport,
            host: config.host.clone(),
            port,
        };

        // 存入连接池
        {
            let mut conns = self.connections.write().await;
            info!("SMTP 添加连接: id={}", connection_id);
            conns.insert(connection_id.to_string(), connection);
        }

        Ok(SmtpConnectOutput {
            success: true,
            connection_id: Some(connection_id.to_string()),
            server_greeting: None,
            error: None,
        })
    }

    /// 发送邮件
    ///
    /// 从连接池获取 transport，构建邮件消息并发送。
    async fn send(
        &self,
        connection_id: &str,
        from: &str,
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: &str,
        body_type: &str,
        body: &str,
        html_body: &str,
        attachments: &[serde_json::Value],
    ) -> Result<SmtpSendOutput, String> {
        // 从连接池获取 transport
        let transport = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("SMTP 连接 '{}' 不存在", connection_id))?;
            conn.transport.clone()
        }; // 读锁释放

        // 构建 from 地址
        let from_addr = from
            .parse()
            .map_err(|e| format!("发件人地址格式Error '{}': {}", from, e))?;

        // 构建消息 builder
        let mut builder = Message::builder()
            .from(from_addr)
            .subject(subject.to_string());

        // 添加收件人
        for addr in to {
            let parsed: lettre::Address = addr
                .parse()
                .map_err(|e| format!("收件人地址格式Error '{}': {}", addr, e))?;
            builder = builder.to(parsed.into());
        }

        // 添加抄送
        for addr in cc {
            let parsed: lettre::Address = addr
                .parse()
                .map_err(|e| format!("抄送地址格式Error '{}': {}", addr, e))?;
            builder = builder.cc(parsed.into());
        }

        // 添加密送
        for addr in bcc {
            let parsed: lettre::Address = addr
                .parse()
                .map_err(|e| format!("密送地址格式Error '{}': {}", addr, e))?;
            builder = builder.bcc(parsed.into());
        }

        // 构建邮件正文 + 附件
        let has_attachments = !attachments.is_empty();
        let message = if has_attachments {
            // 带附件：使用 multipart/mixed
            let body_part = Self::build_body_multipart(body_type, body, html_body)?;

            let mut mixed = MultiPart::mixed().multipart(body_part);

            // 添加附件
            for attachment_val in attachments {
                let part = Self::build_attachment_part(attachment_val)?;
                mixed = mixed.singlepart(part);
            }

            builder
                .multipart(mixed)
                .map_err(|e| format!("构建邮件Failed: {}", e))?
        } else {
            // 无附件
            Self::build_simple_message(builder, body_type, body, html_body)?
        };

        // 发送邮件
        match transport.send(message).await {
            Ok(response) => {
                let code_str = response.code().to_string();
                let code: u16 = code_str.parse().unwrap_or(0);
                let message_id = response.first_line().map(|s| s.to_string());

                info!("SMTP 邮件发送Success: id={}, code={}", connection_id, code);

                Ok(SmtpSendOutput {
                    success: true,
                    message_id,
                    response_code: Some(code),
                    error: None,
                })
            }
            Err(e) => {
                warn!("SMTP 邮件发送Failed: id={}, error={}", connection_id, e);
                Ok(SmtpSendOutput {
                    success: false,
                    message_id: None,
                    response_code: None,
                    error: Some(format!("发送Failed: {}", e)),
                })
            }
        }
    }

    /// 断开 SMTP 连接
    ///
    /// 从连接池中移除连接。
    async fn disconnect(&self, connection_id: &str) -> Result<SmtpDisconnectOutput, String> {
        let mut conns = self.connections.write().await;
        match conns.remove(connection_id) {
            Some(_conn) => {
                info!("SMTP 断开连接: id={}", connection_id);
                Ok(SmtpDisconnectOutput {
                    success: true,
                    error: None,
                })
            }
            None => {
                warn!(
                    "SMTP 断开connection failed: 连接 '{}' 不存在",
                    connection_id
                );
                Ok(SmtpDisconnectOutput {
                    success: false,
                    error: Some(format!("SMTP 连接 '{}' 不存在", connection_id)),
                })
            }
        }
    }

    /// 测试 SMTP 连接
    ///
    /// 创建临时 transport，测试连接，不存入连接池。
    async fn test_connection(&self, config: &SmtpConfig) -> Result<bool, String> {
        info!("SMTP 测试连接: host={}:{}", config.host, config.port);

        let builder = Self::create_builder(config)?;

        let builder = if let Some(ref auth) = config.auth {
            Self::apply_auth(builder, auth)?
        } else {
            builder
        };

        let transport = builder.build::<Tokio1Executor>();

        transport
            .test_connection()
            .await
            .map_err(|e| format!("连接测试Failed: {}", e))
    }
}

// ---------------------------------------------------------------------------
// 邮件构建辅助方法
// ---------------------------------------------------------------------------

impl SmtpConnectionManager {
    /// 构建无附件的邮件消息
    fn build_simple_message(
        builder: lettre::message::MessageBuilder,
        body_type: &str,
        body: &str,
        html_body: &str,
    ) -> Result<Message, String> {
        match body_type {
            "html" => builder
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body.to_string()),
                )
                .map_err(|e| format!("构建 HTML 邮件Failed: {}", e)),
            "both" => builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(body.to_string()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html_body.to_string()),
                        ),
                )
                .map_err(|e| format!("构建 multipart 邮件Failed: {}", e)),
            _ => {
                // 默认纯文本
                builder
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(body.to_string()),
                    )
                    .map_err(|e| format!("构建纯文本邮件Failed: {}", e))
            }
        }
    }

    /// 构建邮件正文 MultiPart（用于带附件的场景）
    fn build_body_multipart(
        body_type: &str,
        body: &str,
        html_body: &str,
    ) -> Result<MultiPart, String> {
        match body_type {
            "html" => Ok(MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.to_string()),
            )),
            "both" => Ok(MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(body.to_string()),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body.to_string()),
                )),
            _ => Ok(MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_PLAIN)
                    .body(body.to_string()),
            )),
        }
    }

    /// 从 JSON Value 构建附件 [`SinglePart`]
    fn build_attachment_part(attachment: &serde_json::Value) -> Result<SinglePart, String> {
        let filename = attachment
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("attachment")
            .to_string();

        let content_base64 = attachment
            .get("content_base64")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let content_type_str = attachment
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("application/octet-stream");

        // 解码 base64 内容
        let content = BASE64
            .decode(content_base64)
            .map_err(|e| format!("解码附件 base64 Failed '{}': {}", filename, e))?;

        let content_type: ContentType = content_type_str
            .parse()
            .map_err(|e| format!("解析附件 Content-Type Failed '{}': {}", filename, e))?;

        let attachment_part = Attachment::new(filename).body(content, content_type);

        Ok(attachment_part)
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = SmtpConnectionManager::new();
        // 新创建的 manager 应该没有连接
        let conns = manager
            .connections
            .try_read()
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(conns.is_empty());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = SmtpConnectionManager::default();
        let conns = manager
            .connections
            .try_read()
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(conns.is_empty());
    }

    #[test]
    fn test_effective_port_explicit() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 2525,
            tls_mode: TlsMode::Implicit,
            auth: None,
        };
        assert_eq!(SmtpConnectionManager::effective_port(&config), 2525);
    }

    #[test]
    fn test_effective_port_default_implicit() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 0,
            tls_mode: TlsMode::Implicit,
            auth: None,
        };
        assert_eq!(SmtpConnectionManager::effective_port(&config), 465);
    }

    #[test]
    fn test_effective_port_default_starttls() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 0,
            tls_mode: TlsMode::StartTls,
            auth: None,
        };
        assert_eq!(SmtpConnectionManager::effective_port(&config), 587);
    }

    #[test]
    fn test_effective_port_default_none() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 0,
            tls_mode: TlsMode::None,
            auth: None,
        };
        assert_eq!(SmtpConnectionManager::effective_port(&config), 25);
    }

    #[test]
    fn test_decrypt_password_empty() {
        let result = SmtpConnectionManager::decrypt_password("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or_else(|e| panic!("{e}")), "");
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_connection() {
        let manager = SmtpConnectionManager::new();
        let result = manager.disconnect("non-existent").await;
        let output = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(!output.success);
        assert!(output
            .error
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("不存在"));
    }

    #[tokio::test]
    async fn test_send_nonexistent_connection() {
        let manager = SmtpConnectionManager::new();
        let result = manager
            .send(
                "non-existent",
                "from@test.com",
                &["to@test.com".to_string()],
                &[],
                &[],
                "Test",
                "plain",
                "Hello",
                "",
                &[],
            )
            .await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("不存在"));
    }

    #[test]
    fn test_build_body_multipart_plain() {
        let result = SmtpConnectionManager::build_body_multipart("plain", "Hello", "");
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_body_multipart_html() {
        let result = SmtpConnectionManager::build_body_multipart("html", "", "<p>Hello</p>");
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_body_multipart_both() {
        let result = SmtpConnectionManager::build_body_multipart("both", "Hello", "<p>Hello</p>");
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_attachment_part_valid() {
        let attachment = serde_json::json!({
            "filename": "test.txt",
            "content_base64": BASE64.encode(b"Hello, world!"),
            "content_type": "text/plain"
        });

        let result = SmtpConnectionManager::build_attachment_part(&attachment);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_attachment_part_invalid_base64() {
        let attachment = serde_json::json!({
            "filename": "test.txt",
            "content_base64": "!!!invalid!!!",
            "content_type": "text/plain"
        });

        let result = SmtpConnectionManager::build_attachment_part(&attachment);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("解码附件 base64 Failed"));
    }

    #[test]
    fn test_build_attachment_part_default_values() {
        // 缺少可选字段时使用默认值
        let attachment = serde_json::json!({
            "content_base64": BASE64.encode(b"test"),
        });

        let result = SmtpConnectionManager::build_attachment_part(&attachment);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_attachment_part_invalid_content_type() {
        let attachment = serde_json::json!({
            "filename": "test.txt",
            "content_base64": BASE64.encode(b"test"),
            "content_type": "invalid content type with spaces and special chars !!!"
        });

        let result = SmtpConnectionManager::build_attachment_part(&attachment);
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("解析附件 Content-Type Failed"));
    }

    #[test]
    fn test_create_builder_none_tls() {
        let config = SmtpConfig {
            host: "localhost".to_string(),
            port: 25,
            tls_mode: TlsMode::None,
            auth: None,
        };
        let result = SmtpConnectionManager::create_builder(&config);
        assert!(result.is_ok());
    }

    // 以下测试需要网络连接，标记为 ignore
    #[tokio::test]
    #[ignore = "需要网络连接"]
    async fn test_connect_real_server() {
        let manager = SmtpConnectionManager::new();
        let config = SmtpConfig {
            host: "smtp.gmail.com".to_string(),
            port: 587,
            tls_mode: TlsMode::StartTls,
            auth: None,
        };
        let result = manager.connect("test-conn-1", &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "需要网络连接"]
    async fn test_test_connection_real_server() {
        let manager = SmtpConnectionManager::new();
        let config = SmtpConfig {
            host: "smtp.gmail.com".to_string(),
            port: 587,
            tls_mode: TlsMode::StartTls,
            auth: None,
        };
        let result = manager.test_connection(&config).await;
        assert!(result.is_ok());
    }
}
