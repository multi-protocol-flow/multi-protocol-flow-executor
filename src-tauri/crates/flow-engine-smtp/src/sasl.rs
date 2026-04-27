// SASL 认证构建器
//
// 将 EmailAuthConfig 映射到 lettre 的 Credentials 和 Mechanism。
// 核心 email_auth 模块提供原始 SASL 字符串构建，
// 此模块提供 lettre 传输层集成。
//
// 注意: EmailAuthConfig 中的密码和 token 是加密存储的，
// 实际解密在连接管理器中完成。此模块的函数接受已解密的凭证。

use lettre::transport::smtp::authentication::{Credentials, Mechanism};

use flow_engine_core::email_auth;
use flow_engine_core::email_types::EmailAuthConfig;

// ────────────────────────────────────────────────────────────────────────────
// Mechanism 解析
// ────────────────────────────────────────────────────────────────────────────

/// 根据 EmailAuthConfig 返回对应的 lettre SASL Mechanism
pub fn resolve_auth_mechanism(auth: &EmailAuthConfig) -> Mechanism {
    match auth {
        EmailAuthConfig::Plain { .. } => Mechanism::Plain,
        EmailAuthConfig::Login { .. } => Mechanism::Login,
        EmailAuthConfig::OAuth2 { .. } => Mechanism::Xoauth2,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Credentials 构建（已解密的凭证）
// ────────────────────────────────────────────────────────────────────────────

/// 使用已解密的用户名和密码构建 lettre Credentials
///
/// 适用于 Plain 和 Login 认证方式。
/// 调用方需负责在调用前完成密码解密。
pub fn build_lettre_credentials_decrypted(username: &str, password: &str) -> Credentials {
    Credentials::new(username.to_string(), password.to_string())
}

/// 使用 XOAUTH2 构建 lettre Credentials
///
/// XOAUTH2 的 "password" 字段实际上是 base64 编码的 SASL 字符串，
/// 格式为 `user={user}\x01auth=Bearer {token}\x01\x01`。
///
/// 调用方需负责在调用前完成 access_token 的解密。
pub fn build_xoauth2_credentials(user: &str, access_token: &str) -> Credentials {
    let sasl_string = email_auth::xoauth2_string(user, access_token);
    Credentials::new(user.to_string(), sasl_string)
}

// ────────────────────────────────────────────────────────────────────────────
// 辅助函数
// ────────────────────────────────────────────────────────────────────────────

/// 获取认证用户名
pub fn get_auth_username(auth: &EmailAuthConfig) -> &str {
    match auth {
        EmailAuthConfig::Plain { username, .. } => username,
        EmailAuthConfig::Login { username, .. } => username,
        EmailAuthConfig::OAuth2 { .. } => {
            // OAuth2 的用户名需要从其他来源获取（如 token 中的 email），
            // EmailAuthConfig 不直接存储 username。
            // 返回空字符串，调用方应从 token 或配置中获取。
            ""
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// 测试
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::email_types::{EmailProvider, OAuth2Method};

    // ── resolve_auth_mechanism ─────────────────────────────────────────────

    #[test]
    fn test_resolve_plain_mechanism() {
        let auth = EmailAuthConfig::Plain {
            username: "user@example.com".to_string(),
            encrypted_password: "encrypted".to_string(),
        };
        assert_eq!(resolve_auth_mechanism(&auth), Mechanism::Plain);
    }

    #[test]
    fn test_resolve_login_mechanism() {
        let auth = EmailAuthConfig::Login {
            username: "user@example.com".to_string(),
            encrypted_password: "encrypted".to_string(),
        };
        assert_eq!(resolve_auth_mechanism(&auth), Mechanism::Login);
    }

    #[test]
    fn test_resolve_oauth2_mechanism() {
        let auth = EmailAuthConfig::OAuth2 {
            provider: EmailProvider::Gmail,
            client_id: "xxx".to_string(),
            encrypted_client_secret: None,
            encrypted_tokens: "encrypted_tokens".to_string(),
            method: OAuth2Method::XOAuth2,
        };
        assert_eq!(resolve_auth_mechanism(&auth), Mechanism::Xoauth2);
    }

    #[test]
    fn test_resolve_oauth2_auto_method_still_xoauth2() {
        // Auto method 也映射到 Xoauth2（lettre 支持的最接近的机制）
        let auth = EmailAuthConfig::OAuth2 {
            provider: EmailProvider::Outlook,
            client_id: "xxx".to_string(),
            encrypted_client_secret: None,
            encrypted_tokens: "encrypted_tokens".to_string(),
            method: OAuth2Method::Auto,
        };
        assert_eq!(resolve_auth_mechanism(&auth), Mechanism::Xoauth2);
    }

    // ── build_lettre_credentials_decrypted ─────────────────────────────────

    #[test]
    fn test_build_plain_credentials() {
        let creds = build_lettre_credentials_decrypted("user@example.com", "mypassword");
        // Credentials 不暴露内部字段，验证不 panic 即可
        assert_eq!(
            format!("{:?}", creds).contains("user@example.com"),
            false // Credentials 的 Debug 不泄露密码
        );
    }

    #[test]
    fn test_build_plain_credentials_empty() {
        let _creds = build_lettre_credentials_decrypted("", "");
        // 空值也不应 panic
    }

    // ── build_xoauth2_credentials ──────────────────────────────────────────

    #[test]
    fn test_build_xoauth2_credentials() {
        let creds = build_xoauth2_credentials("user@gmail.com", "ya29.token");
        // 验证不 panic，内部使用 xoauth2_string 构建 SASL 字符串
        let _ = creds;
    }

    #[test]
    fn test_build_xoauth2_credentials_empty_token() {
        let creds = build_xoauth2_credentials("user@gmail.com", "");
        // 空 token 也能构建（实际认证会失败，但构建不应 panic）
        let _ = creds;
    }

    // ── get_auth_username ──────────────────────────────────────────────────

    #[test]
    fn test_get_auth_username_plain() {
        let auth = EmailAuthConfig::Plain {
            username: "user@example.com".to_string(),
            encrypted_password: "encrypted".to_string(),
        };
        assert_eq!(get_auth_username(&auth), "user@example.com");
    }

    #[test]
    fn test_get_auth_username_login() {
        let auth = EmailAuthConfig::Login {
            username: "admin@test.com".to_string(),
            encrypted_password: "encrypted".to_string(),
        };
        assert_eq!(get_auth_username(&auth), "admin@test.com");
    }

    #[test]
    fn test_get_auth_username_oauth2_returns_empty() {
        // OAuth2 变体不直接存储 username，返回空字符串
        let auth = EmailAuthConfig::OAuth2 {
            provider: EmailProvider::Gmail,
            client_id: "xxx".to_string(),
            encrypted_client_secret: None,
            encrypted_tokens: "encrypted_tokens".to_string(),
            method: OAuth2Method::XOAuth2,
        };
        assert_eq!(get_auth_username(&auth), "");
    }
}
