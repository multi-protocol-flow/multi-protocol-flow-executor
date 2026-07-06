// OAuth2 + SASL 认证工具模块
//
// 提供邮件服务商预设配置和 SASL 认证字符串构建函数。
// 不包含网络请求（token 刷新等应在 SMTP/IMAP crate 中实现）。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;

use crate::email_types::EmailProvider;

// ────────────────────────────────────────────────────────────────────────────
// 服务商预设配置
// ────────────────────────────────────────────────────────────────────────────

/// 已知邮件服务商的预设配置
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EmailProviderPreset {
    /// SMTP 服务器地址
    pub smtp_host: &'static str,
    /// SMTP 端口（隐式 TLS）
    pub smtp_port: u16,
    /// SMTP 端口（STARTTLS）
    pub smtp_port_starttls: u16,
    /// IMAP 服务器地址
    pub imap_host: &'static str,
    /// IMAP 端口（隐式 TLS）
    pub imap_port: u16,
    /// IMAP 端口（STARTTLS）
    pub imap_port_starttls: u16,
    /// `OAuth2` Scope
    pub oauth_scope: &'static str,
    /// `OAuth2` 授权 URL
    pub auth_url: &'static str,
    /// `OAuth2` Token URL
    pub token_url: &'static str,
}

/// Gmail 预设配置
const GMAIL_PRESET: EmailProviderPreset = EmailProviderPreset {
    smtp_host: "smtp.gmail.com",
    smtp_port: 465,
    smtp_port_starttls: 587,
    imap_host: "imap.gmail.com",
    imap_port: 993,
    imap_port_starttls: 143,
    oauth_scope: "https://mail.google.com/",
    auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
};

/// Outlook 预设配置
const OUTLOOK_PRESET: EmailProviderPreset = EmailProviderPreset {
    smtp_host: "smtp.office365.com",
    smtp_port: 587,
    smtp_port_starttls: 587,
    imap_host: "outlook.office365.com",
    imap_port: 993,
    imap_port_starttls: 143,
    oauth_scope: "https://outlook.office.com/IMAP.AccessAsUser.All https://outlook.office.com/SMTP.Send offline_access",
    auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
    token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
};

/// Yahoo 预设配置
const YAHOO_PRESET: EmailProviderPreset = EmailProviderPreset {
    smtp_host: "smtp.mail.yahoo.com",
    smtp_port: 465,
    smtp_port_starttls: 587,
    imap_host: "imap.mail.yahoo.com",
    imap_port: 993,
    imap_port_starttls: 143,
    oauth_scope: "https://mail.yahoo.com/",
    auth_url: "https://api.login.yahoo.com/oauth2/request_auth",
    token_url: "https://api.login.yahoo.com/oauth2/get_token",
};

/// 获取已知服务商的预设配置
///
/// # 参数
/// - `provider`: 邮件服务商枚举值
///
/// # 返回
/// - `Some(EmailProviderPreset)`: 已知服务商的预设配置
/// - `None`: 自定义服务商或未知服务商
pub fn get_provider_preset(provider: &EmailProvider) -> Option<EmailProviderPreset> {
    match provider {
        EmailProvider::Gmail => Some(GMAIL_PRESET),
        EmailProvider::Outlook => Some(OUTLOOK_PRESET),
        EmailProvider::Yahoo => Some(YAHOO_PRESET),
        EmailProvider::Custom { .. } => None,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// SASL 认证字符串构建
// ────────────────────────────────────────────────────────────────────────────

/// 生成 XOAUTH2 SASL 字符串
///
/// 格式: `base64("user={email}\x01auth=Bearer {token}\x01\x01")`
///
/// # 参数
/// - `user`: 用户邮箱地址
/// - `token`: `OAuth2` Access Token
///
/// # 返回
/// Base64 编码的 XOAUTH2 SASL 字符串
pub fn xoauth2_string(user: &str, token: &str) -> String {
    // XOAUTH2 格式: user={user}\x01auth=Bearer {token}\x01\x01
    let raw = format!("user={}\x01auth=Bearer {}\x01\x01", user, token);
    BASE64.encode(raw.as_bytes())
}

/// 生成 PLAIN SASL 字符串
///
/// 格式: `base64("\0{user}\0{password}")`
///
/// # 参数
/// - `user`: 用户名
/// - `password`: 密码
///
/// # 返回
/// Base64 编码的 PLAIN SASL 字符串
pub fn plain_string(user: &str, password: &str) -> String {
    // PLAIN 格式: \0user\0password
    let mut raw = Vec::with_capacity(1 + user.len() + 1 + password.len());
    raw.push(0);
    raw.extend_from_slice(user.as_bytes());
    raw.push(0);
    raw.extend_from_slice(password.as_bytes());
    BASE64.encode(&raw)
}

/// 生成 LOGIN SASL 字符串对
///
/// LOGIN 分两步: 先发 base64(user)，再发 base64(password)
///
/// # 参数
/// - `user`: 用户名
/// - `password`: 密码
///
/// # 返回
/// (base64 编码的用户名, base64 编码的密码)
pub fn login_strings(user: &str, password: &str) -> (String, String) {
    let user_b64 = BASE64.encode(user.as_bytes());
    let pass_b64 = BASE64.encode(password.as_bytes());
    (user_b64, pass_b64)
}

// ────────────────────────────────────────────────────────────────────────────
// 测试
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    // ── XOAUTH2 ──────────────────────────────────────────────────────────

    #[test]
    fn test_xoauth2_string() {
        let result = xoauth2_string("user@gmail.com", "ya29.token");
        let decoded = BASE64.decode(&result).unwrap();
        let decoded_str = String::from_utf8_lossy(&decoded);

        assert!(decoded_str.starts_with("user=user@gmail.com"));
        assert!(decoded_str.contains("auth=Bearer ya29.token"));
        // 验证 \x01 分隔符存在
        assert!(decoded.contains(&0x01u8));
        // 验证末尾 \x01\x01
        assert_eq!(&decoded[decoded.len() - 2..], &[0x01, 0x01]);
    }

    #[test]
    fn test_xoauth2_string_empty_token() {
        let result = xoauth2_string("user@example.com", "");
        let decoded = BASE64.decode(&result).unwrap();
        let decoded_str = String::from_utf8_lossy(&decoded);
        assert!(decoded_str.contains("auth=Bearer "));
    }

    // ── PLAIN ────────────────────────────────────────────────────────────

    #[test]
    fn test_plain_string() {
        let result = plain_string("user@example.com", "password123");
        let decoded = BASE64.decode(&result).unwrap();

        // 格式: \0user\0password
        assert_eq!(decoded[0], 0);
        assert!(decoded
            .windows(b"user@example.com".len())
            .any(|w| w == b"user@example.com"));
        assert!(decoded
            .windows(b"password123".len())
            .any(|w| w == b"password123"));
    }

    #[test]
    fn test_plain_string_empty_values() {
        let result = plain_string("", "");
        let decoded = BASE64.decode(&result).unwrap();
        // \0\0
        assert_eq!(decoded, vec![0, 0]);
    }

    // ── LOGIN ────────────────────────────────────────────────────────────

    #[test]
    fn test_login_strings() {
        let (user_b64, pass_b64) = login_strings("user@example.com", "password123");

        let decoded_user = BASE64.decode(&user_b64).unwrap();
        let decoded_pass = BASE64.decode(&pass_b64).unwrap();

        assert_eq!(decoded_user, b"user@example.com");
        assert_eq!(decoded_pass, b"password123");
    }

    #[test]
    fn test_login_strings_special_chars() {
        let (user_b64, pass_b64) = login_strings("user@例え.jp", "p@ss:w0rd/特殊");

        let decoded_user = BASE64.decode(&user_b64).unwrap();
        let decoded_pass = BASE64.decode(&pass_b64).unwrap();

        assert_eq!(String::from_utf8(decoded_user).unwrap(), "user@例え.jp");
        assert_eq!(String::from_utf8(decoded_pass).unwrap(), "p@ss:w0rd/特殊");
    }

    // ── Provider 预设 ────────────────────────────────────────────────────

    #[test]
    fn test_gmail_preset() {
        let preset = get_provider_preset(&EmailProvider::Gmail).unwrap();
        assert_eq!(preset.smtp_host, "smtp.gmail.com");
        assert_eq!(preset.smtp_port, 465);
        assert_eq!(preset.smtp_port_starttls, 587);
        assert_eq!(preset.imap_host, "imap.gmail.com");
        assert_eq!(preset.imap_port, 993);
        assert_eq!(preset.oauth_scope, "https://mail.google.com/");
    }

    #[test]
    fn test_outlook_preset() {
        let preset = get_provider_preset(&EmailProvider::Outlook).unwrap();
        assert_eq!(preset.smtp_host, "smtp.office365.com");
        assert_eq!(preset.imap_host, "outlook.office365.com");
        assert!(preset.oauth_scope.contains("IMAP.AccessAsUser.All"));
        assert!(preset.oauth_scope.contains("SMTP.Send"));
        assert!(preset.oauth_scope.contains("offline_access"));
    }

    #[test]
    fn test_yahoo_preset() {
        let preset = get_provider_preset(&EmailProvider::Yahoo).unwrap();
        assert_eq!(preset.smtp_host, "smtp.mail.yahoo.com");
        assert_eq!(preset.imap_host, "imap.mail.yahoo.com");
        assert_eq!(preset.imap_port, 993);
    }

    #[test]
    fn test_custom_preset_returns_none() {
        let provider = EmailProvider::Custom {
            imap_host: "imap.example.com".to_string(),
            smtp_host: "smtp.example.com".to_string(),
            auth_url: None,
            token_url: None,
        };
        assert!(get_provider_preset(&provider).is_none());
    }

    #[test]
    fn test_preset_clone_and_equality() {
        let preset1 = get_provider_preset(&EmailProvider::Gmail).unwrap();
        let preset2 = preset1.clone();
        assert_eq!(preset1, preset2);
    }

    #[test]
    fn test_preset_serialization() {
        let preset = get_provider_preset(&EmailProvider::Gmail).unwrap();
        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("smtp.gmail.com"));
        assert!(json.contains("imap.gmail.com"));
        assert!(json.contains("https://mail.google.com/"));
        assert!(json.contains("465"));
        assert!(json.contains("993"));
    }
}
