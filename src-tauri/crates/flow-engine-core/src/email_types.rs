// 邮件协议共享类型定义

use serde::{Deserialize, Serialize};

/// TLS 模式
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    /// 隐式 TLS（端口 465 for SMTP, 993 for IMAP）
    #[default]
    Implicit,
    /// STARTTLS（端口 587 for SMTP, 143 for IMAP）
    StartTls,
    /// 无 TLS（仅用于测试）
    None,
}

/// 邮件地址
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EmailAddress {
    /// 显示名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 邮箱地址
    pub address: String,
}

/// 邮件内容类型
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailBodyType {
    /// 纯文本
    #[default]
    PlainText,
    /// HTML
    Html,
    /// 纯文本 + HTML（multipart/alternative）
    Both,
}

/// 邮件附件
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EmailAttachment {
    /// 文件名
    pub filename: String,
    /// Content-Type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// 文件内容（base64 编码）
    pub content_base64: String,
}

/// SMTP 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
    /// TLS 模式
    #[serde(default)]
    pub tls_mode: TlsMode,
    /// 认证配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<EmailAuthConfig>,
}

/// IMAP 配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImapConfig {
    /// IMAP 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
    /// TLS 模式
    #[serde(default)]
    pub tls_mode: TlsMode,
    /// 认证配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<EmailAuthConfig>,
}

/// 邮件认证配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmailAuthConfig {
    /// Plain 认证
    Plain {
        /// 用户名
        username: String,
        /// 加密密码
        encrypted_password: String,
    },
    /// Login 认证
    Login {
        /// 用户名
        username: String,
        /// 加密密码
        encrypted_password: String,
    },
    /// `OAuth2` 认证
    OAuth2 {
        /// 邮件服务商
        provider: EmailProvider,
        /// 客户端 ID
        client_id: String,
        /// 加密的客户端密钥
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encrypted_client_secret: Option<String>,
        /// 加密的 `OAuth2` Token
        encrypted_tokens: String,
        /// `OAuth2` 方法
        #[serde(default)]
        method: OAuth2Method,
    },
}

impl Default for EmailAuthConfig {
    fn default() -> Self {
        EmailAuthConfig::Plain {
            username: String::new(),
            encrypted_password: String::new(),
        }
    }
}

/// 邮件服务商
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailProvider {
    /// Gmail
    #[default]
    Gmail,
    /// Outlook
    Outlook,
    /// Yahoo
    Yahoo,
    /// 自定义
    Custom {
        /// IMAP 主机
        imap_host: String,
        /// SMTP 主机
        smtp_host: String,
        /// OAuth 授权 URL
        #[serde(default, skip_serializing_if = "Option::is_none")]
        auth_url: Option<String>,
        /// Token URL
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token_url: Option<String>,
    },
}

/// `OAuth2` 方法
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuth2Method {
    /// XOAUTH2 SASL
    #[default]
    XOAuth2,
    /// 自动选择
    Auto,
}

/// `OAuth2` Token 数据
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EmailOAuthTokens {
    /// Access Token
    pub access_token: String,
    /// Refresh Token
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// 过期时间（Unix 时间戳毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// OAuth Scope
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_mode_default() {
        let mode = TlsMode::default();
        assert_eq!(mode, TlsMode::Implicit);
    }

    #[test]
    fn test_tls_mode_serialization() {
        let json = serde_json::to_string(&TlsMode::StartTls).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "\"start_tls\"");

        let json = serde_json::to_string(&TlsMode::None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "\"none\"");

        let deserialized: TlsMode =
            serde_json::from_str("\"implicit\"").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized, TlsMode::Implicit);
    }

    #[test]
    fn test_email_body_type_serialization() {
        assert_eq!(
            serde_json::to_string(&EmailBodyType::PlainText).unwrap_or_else(|e| panic!("{e}")),
            "\"plain_text\""
        );
        assert_eq!(
            serde_json::to_string(&EmailBodyType::Html).unwrap_or_else(|e| panic!("{e}")),
            "\"html\""
        );
        assert_eq!(
            serde_json::to_string(&EmailBodyType::Both).unwrap_or_else(|e| panic!("{e}")),
            "\"both\""
        );
    }

    #[test]
    fn test_smtp_config_serialization() {
        let config = SmtpConfig {
            host: "smtp.gmail.com".to_string(),
            port: 465,
            tls_mode: TlsMode::Implicit,
            auth: None,
        };

        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("smtp.gmail.com"));
        assert!(json.contains("465"));

        let deserialized: SmtpConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.host, "smtp.gmail.com");
        assert_eq!(deserialized.port, 465);
    }

    #[test]
    fn test_imap_config_serialization() {
        let config = ImapConfig {
            host: "imap.gmail.com".to_string(),
            port: 993,
            tls_mode: TlsMode::Implicit,
            auth: None,
        };

        let json = serde_json::to_string(&config).unwrap_or_else(|e| panic!("{e}"));
        let deserialized: ImapConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.host, "imap.gmail.com");
    }

    #[test]
    fn test_email_auth_config_plain() {
        let auth = EmailAuthConfig::Plain {
            username: "user@example.com".to_string(),
            encrypted_password: "encrypted".to_string(),
        };

        let json = serde_json::to_string(&auth).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("\"type\":\"plain\""));

        let deserialized: EmailAuthConfig =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(auth, deserialized);
    }

    #[test]
    fn test_email_auth_config_default() {
        let auth = EmailAuthConfig::default();
        match auth {
            EmailAuthConfig::Plain {
                username,
                encrypted_password,
            } => {
                assert!(username.is_empty());
                assert!(encrypted_password.is_empty());
            }
            _ => panic!("默认应为 Plain 认证"),
        }
    }

    #[test]
    fn test_email_provider_serialization() {
        assert_eq!(
            serde_json::to_string(&EmailProvider::Gmail).unwrap_or_else(|e| panic!("{e}")),
            "\"gmail\""
        );
        assert_eq!(
            serde_json::to_string(&EmailProvider::Outlook).unwrap_or_else(|e| panic!("{e}")),
            "\"outlook\""
        );
    }

    #[test]
    fn test_oauth2_method_default() {
        assert_eq!(OAuth2Method::default(), OAuth2Method::XOAuth2);
    }

    #[test]
    fn test_email_oauth_tokens_roundtrip() {
        let tokens = EmailOAuthTokens {
            access_token: "ya29.token".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(1700000000000),
            scope: Some("https://mail.google.com/".to_string()),
        };

        let json = serde_json::to_string(&tokens).unwrap_or_else(|e| panic!("{e}"));
        let deserialized: EmailOAuthTokens =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(tokens, deserialized);
    }

    #[test]
    fn test_email_attachment_roundtrip() {
        let attachment = EmailAttachment {
            filename: "test.pdf".to_string(),
            content_type: Some("application/pdf".to_string()),
            content_base64: "SGVsbG8=".to_string(),
        };

        let json = serde_json::to_string(&attachment).unwrap_or_else(|e| panic!("{e}"));
        let deserialized: EmailAttachment =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(attachment, deserialized);
    }

    #[test]
    fn test_email_address_roundtrip() {
        let addr = EmailAddress {
            name: Some("Test User".to_string()),
            address: "test@example.com".to_string(),
        };

        let json = serde_json::to_string(&addr).unwrap_or_else(|e| panic!("{e}"));
        let deserialized: EmailAddress =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(addr, deserialized);
    }
}
