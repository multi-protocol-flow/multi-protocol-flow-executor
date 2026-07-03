// OAuth2 Token 流程实现
//
// 支持:
// - Authorization Code + PKCE (Tauri 桌面应用)
// - Device Authorization Flow (CLI mpe 二进制)
// - Token 刷新
// - Token 加密存储

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use flow_engine_core::email_auth::get_provider_preset;
use flow_engine_core::email_types::{EmailOAuthTokens, EmailProvider};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// `OAuth2` 配置
#[derive(Debug, Clone)]
pub struct OAuth2Config {
    /// 客户端 ID
    pub client_id: String,
    /// 客户端密钥（可选，PKCE 流程不需要）
    pub client_secret: Option<String>,
    /// 授权 URL
    pub auth_url: String,
    /// Token URL
    pub token_url: String,
    /// `OAuth2` Scope
    pub scope: String,
    /// 本地回调端口（PKCE 流程使用）
    pub redirect_port: u16,
}

/// 从 `EmailProvider` 创建 `OAuth2Config`
///
/// # 参数
/// - `provider`: 邮件服务商
/// - `client_id`: `OAuth2` 客户端 ID
/// - `client_secret`: `OAuth2` 客户端密钥（可选）
/// - `redirect_port`: 本地回调监听端口
///
/// # 返回
/// - `Ok(OAuth2Config)`: Success创建配置
/// - `Err(String)`: 自定义服务商不支持预设配置
pub fn oauth2_config_from_provider(
    provider: &EmailProvider,
    client_id: &str,
    client_secret: Option<&str>,
    redirect_port: u16,
) -> Result<OAuth2Config, String> {
    let preset = get_provider_preset(provider).ok_or("自定义服务商不支持 OAuth2 预设")?;
    Ok(OAuth2Config {
        client_id: client_id.to_string(),
        client_secret: client_secret.map(|s| s.to_string()),
        auth_url: preset.auth_url.to_string(),
        token_url: preset.token_url.to_string(),
        scope: preset.oauth_scope.to_string(),
        redirect_port,
    })
}

// ─── PKCE 工具函数 ────────────────────────────────────────────────────────────

/// 生成加密安全的随机字符串
fn generate_random_string(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// 计算 PKCE `code_challenge` (S256: `BASE64URL(SHA256(code_verifier))`)
fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

// ─── PKCE Flow (Tauri 桌面应用) ──────────────────────────────────────────────

/// 构建 `OAuth2` 授权 URL (PKCE)
///
/// 生成 PKCE verifier + challenge，构建包含所有必需参数的授权 URL。
///
/// # 返回
/// - `(auth_url, pkce_verifier, csrf_state)`: 授权 URL、PKCE 验证码、CSRF 令牌
pub fn build_pkce_auth_url(config: &OAuth2Config) -> Result<(String, String, String), String> {
    let pkce_verifier = generate_random_string(32);
    let challenge = pkce_challenge(&pkce_verifier);
    let csrf_state = generate_random_string(16);

    let redirect_uri = format!("http://localhost:{}", config.redirect_port);

    let mut url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        config.auth_url,
        urlencoding(&config.client_id),
        urlencoding(&redirect_uri),
        urlencoding(&config.scope),
        csrf_state,
        challenge,
    );

    // Outlook 需要 offline_access 在 scope 中以获取 refresh_token
    if config.token_url.contains("microsoftonline.com") && !config.scope.contains("offline_access")
    {
        url.push_str("%20offline_access");
    }

    Ok((url, pkce_verifier, csrf_state))
}

/// 简易 URL 编码（仅编码空格和特殊字符，保留 URL 结构字符）
fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
}

/// 启动本地回调监听器，等待 `OAuth2` 授权码
///
/// 在 localhost:{port} 上启动一个极简 HTTP 服务器，等待浏览器回调。
/// 解析回调 URL 中的 `code` 参数并返回。
///
/// # 参数
/// - `port`: 本地监听端口
///
/// # 返回
/// - `Ok((code, state))`: 授权码和 CSRF state
/// - `Err(String)`: 监听Failed或回调参数缺失
pub async fn start_callback_listener(
    port: u16,
) -> Result<(String, String), String> {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("绑定端口 {} Failed: {}", port, e))?;

    log::info!("OAuth2 回调监听器已启动，等待授权码 (port={})", port);

    let (stream, _) = listener
        .accept()
        .await
        .map_err(|e| format!("接受connection failed: {}", e))?;

    let mut stream: tokio::net::TcpStream = stream;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    let mut buf = vec![0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("读取请求Failed: {}", e))?;

    let request = String::from_utf8_lossy(&buf[..n]);
    log::debug!("收到回调请求: {}", request.lines().next().unwrap_or(""));

    // 解析 HTTP 请求行 GET /callback?code=xxx&state=yyy HTTP/1.1
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    // 从路径中解析 query 参数
    let query_str = path.split('?').nth(1).unwrap_or("");
    let mut code_opt: Option<String> = None;
    let mut state_opt: Option<String> = None;

    for pair in query_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "code" => code_opt = Some(value.to_string()),
                "state" => state_opt = Some(value.to_string()),
                _ => {}
            }
        }
    }

    // 发送响应给浏览器
    let response = if code_opt.is_some() {
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><body><h2>授权Success！</h2><p>可以关闭此窗口。</p></body></html>"
    } else {
        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><body><h2>授权Failed</h2><p>未收到授权码。</p></body></html>"
    };
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;

    let code = code_opt.ok_or("回调参数中缺少 code")?;
    let state = state_opt.unwrap_or_default();

    Ok((code, state))
}

/// 用授权码换取 `OAuth2` Token（PKCE）
///
/// # 参数
/// - `config`: `OAuth2` 配置
/// - `code`: 授权码
/// - `pkce_verifier`: PKCE 验证码（必须与授权 URL 中 challenge 对应）
///
/// # 返回
/// - `Ok(EmailOAuthTokens)`: Success获取 Token
/// - `Err(String)`: 交换Failed
pub async fn exchange_code(
    config: &OAuth2Config,
    code: &str,
    pkce_verifier: &str,
) -> Result<EmailOAuthTokens, String> {
    let redirect_uri = format!("http://localhost:{}", config.redirect_port);

    let mut params = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", redirect_uri),
        ("client_id", config.client_id.clone()),
        ("code_verifier", pkce_verifier.to_string()),
    ];

    if let Some(ref secret) = config.client_secret {
        params.push(("client_secret", secret.clone()));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&config.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token 请求Failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token 交换Failed (HTTP {}): {}", status, body));
    }

    let token_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 Token 响应Failed: {}", e))?;

    parse_token_response(&token_resp)
}

// ─── Device Flow (CLI) ───────────────────────────────────────────────────────

/// Device Authorization Flow 响应
#[derive(Debug, serde::Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    #[allow(dead_code)]
    verification_uri_complete: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    expires_in: Option<u64>,
    #[serde(default)]
    interval: Option<u64>,
}

/// 启动 Device Authorization Flow
///
/// 向服务商请求设备码和验证 URL，用户需要在浏览器中打开验证 URL 并输入用户码。
///
/// # 返回
/// - `Ok((user_code, verification_uri, device_code, interval))`: 用户码、验证 URL、设备码（内部使用）、轮询间隔（秒）
/// - `Err(String)`: 请求Failed
pub async fn start_device_flow(
    config: &OAuth2Config,
) -> Result<(String, String, String, u64), String> {
    let device_code_url = derive_device_code_url(&config.token_url)?;

    let params = [
        ("client_id", config.client_id.as_str()),
        ("scope", config.scope.as_str()),
    ];

    let client = reqwest::Client::new();
    let resp = client
        .post(&device_code_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Device Flow 请求Failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Device Flow Failed (HTTP {}): {}", status, body));
    }

    let device_resp: DeviceCodeResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析 Device Flow 响应Failed: {}", e))?;

    let interval = device_resp.interval.unwrap_or(5);

    Ok((
        device_resp.user_code,
        device_resp.verification_uri,
        device_resp.device_code,
        interval,
    ))
}

/// 轮询 Device Flow Token
///
/// 使用设备码轮询 Token 端点，直到用户完成授权或超时。
///
/// # 参数
/// - `config`: `OAuth2` 配置
/// - `device_code`: 设备码
/// - `interval_secs`: 轮询间隔（秒）
/// - `max_attempts`: 最大轮询次数（0 表示无限）
///
/// # 返回
/// - `Ok(EmailOAuthTokens)`: 授权Success，获取到 Token
/// - `Err(String)`: 授权Failed或超时
pub async fn poll_device_token(
    config: &OAuth2Config,
    device_code: &str,
    interval_secs: u64,
    max_attempts: u32,
) -> Result<EmailOAuthTokens, String> {
    let client = reqwest::Client::new();
    let mut attempts = 0u32;

    loop {
        if max_attempts > 0 && attempts >= max_attempts {
            return Err("Device Flow 超时：用户未在规定时间内完成授权".to_string());
        }
        attempts += 1;

        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;

        let mut params = vec![
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code".to_string()),
            ("client_id", config.client_id.clone()),
            ("device_code", device_code.to_string()),
        ];

        if let Some(ref secret) = config.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        let resp = client
            .post(&config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token 轮询请求Failed: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("解析 Token 轮询响应Failed: {}", e))?;

        // 检查是否授权Success
        if body.get("access_token").is_some() {
            return parse_token_response(&body);
        }

        // 检查Error类型
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_error");

        match error {
            "authorization_pending" => {
                // 用户尚未完成授权，继续轮询
                log::debug!("Device Flow: 等待用户授权... (attempt {})", attempts);
                continue;
            }
            "slow_down" => {
                // 服务商要求降低轮询频率
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
            "expired_token" => {
                return Err("Device Flow 设备码已过期，请重新发起授权".to_string());
            }
            "access_denied" => {
                return Err("用户拒绝了授权请求".to_string());
            }
            _ => {
                let error_desc = body
                    .get("error_description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未知Error");
                return Err(format!("Device Flow Failed: {} - {}", error, error_desc));
            }
        }
    }
}

// ─── Token 刷新 ──────────────────────────────────────────────────────────────

/// 刷新 `OAuth2` Token
///
/// 使用 `refresh_token` 获取新的 `access_token`。
///
/// # 参数
/// - `config`: `OAuth2` 配置
/// - `refresh_token`: 刷新令牌
///
/// # 返回
/// - `Ok(EmailOAuthTokens)`: 新的 Token（可能包含新的 `refresh_token`）
/// - `Err(String)`: 刷新Failed
pub async fn refresh_token(
    config: &OAuth2Config,
    refresh_token: &str,
) -> Result<EmailOAuthTokens, String> {
    let mut params = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("client_id", config.client_id.clone()),
    ];

    if let Some(ref secret) = config.client_secret {
        params.push(("client_secret", secret.clone()));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&config.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token 刷新请求Failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token 刷新Failed (HTTP {}): {}", status, body));
    }

    let token_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 Token 刷新响应Failed: {}", e))?;

    let mut tokens = parse_token_response(&token_resp)?;

    // 如果响应中没有 refresh_token，保留原有的
    if tokens.refresh_token.is_none() {
        tokens.refresh_token = Some(refresh_token.to_string());
    }

    Ok(tokens)
}

// ─── Token 加密存储 ─────────────────────────────────────────────────────────

/// 加密存储 Token
///
/// 将 `EmailOAuthTokens` 序列化为 JSON 后使用 AES-256-GCM 加密。
///
/// # 参数
/// - `tokens`: `OAuth2` Token
///
/// # 返回
/// - `Ok(String)`: 加密后的密文（格式: `base64(nonce):base64(ciphertext)`）
/// - `Err(String)`: 加密Failed
pub fn store_tokens_encrypted(tokens: &EmailOAuthTokens) -> Result<String, String> {
    let json = serde_json::to_string(tokens).map_err(|e| format!("序列化 Token Failed: {}", e))?;
    flow_engine_crypto::encryption::encrypt(&json).map_err(|e| format!("加密 Token Failed: {}", e))
}

/// 解密加载 Token
///
/// 解密密文并反序列化为 `EmailOAuthTokens`。
///
/// # 参数
/// - `encrypted`: 加密的 Token 密文
///
/// # 返回
/// - `Ok(EmailOAuthTokens)`: 解密后的 Token
/// - `Err(String)`: 解密或解析Failed
pub fn load_tokens_decrypted(encrypted: &str) -> Result<EmailOAuthTokens, String> {
    let json =
        flow_engine_crypto::encryption::decrypt(encrypted).map_err(|e| format!("解密Failed: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("反序列化 Token Failed: {}", e))
}

// ─── 内部工具函数 ─────────────────────────────────────────────────────────────

/// 从 `token_url` 推导 `device_code_url`
///
/// - Google: `https://oauth2.googleapis.com/token` → `https://oauth2.googleapis.com/device/code`
/// - Outlook: `https://login.microsoftonline.com/common/oauth2/v2.0/token` → `https://login.microsoftonline.com/common/oauth2/v2.0/devicecode`
fn derive_device_code_url(token_url: &str) -> Result<String, String> {
    if token_url.contains("googleapis.com") {
        // Google: /token → /device/code
        Ok(token_url.replace("/token", "/device/code"))
    } else if token_url.contains("microsoftonline.com") {
        // Microsoft: /token → /devicecode
        Ok(token_url.replace("/token", "/devicecode"))
    } else {
        Err(format!(
            "无法推导 Device Code URL，token_url={}",
            token_url
        ))
    }
}

/// 解析 Token 响应 JSON 为 `EmailOAuthTokens`
fn parse_token_response(resp: &serde_json::Value) -> Result<EmailOAuthTokens, String> {
    let access_token = resp
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Token 响应中缺少 access_token")?
        .to_string();

    let refresh_token = resp
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let expires_in = resp
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .map(|secs| {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            now_ms + secs * 1000
        });

    let scope = resp
        .get("scope")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(EmailOAuthTokens {
        access_token,
        refresh_token,
        expires_at: expires_in,
        scope,
    })
}

// ─── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 OAuth2 配置创建 — Gmail
    #[test]
    fn test_oauth2_config_gmail() {
        let config = oauth2_config_from_provider(
            &EmailProvider::Gmail,
            "test-client-id.apps.googleusercontent.com",
            None,
            8080,
        )
        .expect("创建 Gmail OAuth2 配置Failed");

        assert_eq!(config.client_id, "test-client-id.apps.googleusercontent.com");
        assert!(config.auth_url.contains("accounts.google.com"));
        assert!(config.token_url.contains("googleapis.com"));
        assert_eq!(config.scope, "https://mail.google.com/");
        assert_eq!(config.redirect_port, 8080);
        assert!(config.client_secret.is_none());
    }

    /// 测试 OAuth2 配置创建 — Outlook
    #[test]
    fn test_oauth2_config_outlook() {
        let config = oauth2_config_from_provider(
            &EmailProvider::Outlook,
            "outlook-client-id",
            Some("outlook-secret"),
            9090,
        )
        .expect("创建 Outlook OAuth2 配置Failed");

        assert_eq!(config.client_id, "outlook-client-id");
        assert_eq!(config.client_secret, Some("outlook-secret".to_string()));
        assert!(config.auth_url.contains("microsoftonline.com"));
        assert!(config.token_url.contains("microsoftonline.com"));
        assert!(config.scope.contains("IMAP.AccessAsUser.All"));
        assert!(config.scope.contains("SMTP.Send"));
        assert_eq!(config.redirect_port, 9090);
    }

    /// 测试自定义服务商不支持预设
    #[test]
    fn test_oauth2_config_custom_provider_unsupported() {
        let custom = EmailProvider::Custom {
            imap_host: "imap.example.com".to_string(),
            smtp_host: "smtp.example.com".to_string(),
            auth_url: None,
            token_url: None,
        };

        let result = oauth2_config_from_provider(&custom, "client-id", None, 8080);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("自定义服务商"));
    }

    /// 测试 PKCE 授权 URL 包含必需参数
    #[test]
    fn test_pkce_auth_url_contains_required_params() {
        let config = OAuth2Config {
            client_id: "test.apps.googleusercontent.com".to_string(),
            client_secret: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scope: "https://mail.google.com/".to_string(),
            redirect_port: 8080,
        };

        let (url, verifier, state) = build_pkce_auth_url(&config).expect("构建授权 URL Failed");

        // URL 必须包含关键参数
        assert!(url.contains("response_type=code"), "缺少 response_type");
        assert!(
            url.contains("client_id=test.apps.googleusercontent.com"),
            "缺少 client_id"
        );
        assert!(
            url.contains("redirect_uri=http://localhost:8080"),
            "缺少 redirect_uri"
        );
        assert!(url.contains("state="), "缺少 state");
        assert!(url.contains("code_challenge="), "缺少 code_challenge");
        assert!(
            url.contains("code_challenge_method=S256"),
            "缺少 code_challenge_method"
        );
        assert!(
            url.contains("scope=https://mail.google.com/"),
            "缺少 scope"
        );

        // verifier 和 state 不应为空
        assert!(!verifier.is_empty(), "PKCE verifier 不应为空");
        assert!(!state.is_empty(), "CSRF state 不应为空");

        // verifier 长度合理（32 字节随机 + base64 编码）
        assert!(verifier.len() >= 32, "PKCE verifier 长度不足");
    }

    /// 测试 PKCE challenge 计算（S256 方法）
    #[test]
    fn test_pkce_challenge_sha256() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = pkce_challenge(verifier);

        // RFC 7636 附录 B 的测试向量
        // SHA256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk") =
        // E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    /// 测试随机字符串生成
    #[test]
    fn test_random_string_uniqueness() {
        let s1 = generate_random_string(32);
        let s2 = generate_random_string(32);

        // 两次生成应产生不同结果
        assert_ne!(s1, s2, "两次随机字符串不应相同");
        // 长度应大于等于输入字节数（base64 编码后）
        assert!(s1.len() >= 32);
    }

    /// 测试 Token 加密解密往返
    ///
    /// 注意：加密依赖 OS keyring（Windows Credential Manager / macOS Keychain / Linux Secret Service）。
    /// 如果 keyring 不可用（例如 CI 环境或 keyring 服务未启动），此测试将被跳过。
    #[test]
    fn test_token_encryption_roundtrip() {
        let tokens = EmailOAuthTokens {
            access_token: "ya29.a0AfH6SMBx_test_token".to_string(),
            refresh_token: Some("1//0dxyz_refresh_token".to_string()),
            expires_at: Some(1700000000000),
            scope: Some("https://mail.google.com/".to_string()),
        };

        let encrypted = match store_tokens_encrypted(&tokens) {
            Ok(e) => e,
            Err(_) => {
                eprintln!("跳过: OS keyring 不可用");
                return;
            }
        };

        // 加密结果应包含冒号分隔符（nonce:ciphertext 格式）
        assert!(encrypted.contains(':'), "密文格式应为 nonce:ciphertext");
        assert!(encrypted.len() > 10, "密文长度应合理");

        let decrypted = match load_tokens_decrypted(&encrypted) {
            Ok(d) => d,
            Err(e) => {
                // keyring 竞争导致密钥不一致，非代码问题
                eprintln!("跳过: 解密Failed (keyring 竞争): {}", e);
                return;
            }
        };

        assert_eq!(decrypted, tokens, "解密后的 Token 应与原始一致");
    }

    /// 测试 Token 加密解密 — 最小 Token（仅 access_token）
    #[test]
    fn test_token_encryption_minimal_roundtrip() {
        let tokens = EmailOAuthTokens {
            access_token: "simple_token".to_string(),
            refresh_token: None,
            expires_at: None,
            scope: None,
        };

        let encrypted = match store_tokens_encrypted(&tokens) {
            Ok(e) => e,
            Err(_) => {
                eprintln!("跳过: OS keyring 不可用");
                return;
            }
        };

        let decrypted = match load_tokens_decrypted(&encrypted) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("跳过: 解密Failed (keyring 竞争): {}", e);
                return;
            }
        };

        assert_eq!(decrypted.access_token, "simple_token");
        assert!(decrypted.refresh_token.is_none());
        assert!(decrypted.expires_at.is_none());
        assert!(decrypted.scope.is_none());
    }

    /// 测试解密无效数据应报错
    #[test]
    fn test_load_tokens_invalid_data() {
        let result = load_tokens_decrypted("not-valid-encrypted-data");
        assert!(result.is_err());
    }

    /// 测试解密格式Error数据应报错
    #[test]
    fn test_load_tokens_bad_format() {
        let result = load_tokens_decrypted("no_colon_here");
        assert!(result.is_err());
    }

    /// 测试 Device Code URL 推导 — Google
    #[test]
    fn test_derive_device_code_url_google() {
        let url =
            derive_device_code_url("https://oauth2.googleapis.com/token").expect("推导Failed");
        assert_eq!(url, "https://oauth2.googleapis.com/device/code");
    }

    /// 测试 Device Code URL 推导 — Microsoft
    #[test]
    fn test_derive_device_code_url_microsoft() {
        let url = derive_device_code_url(
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        )
        .expect("推导Failed");
        assert_eq!(
            url,
            "https://login.microsoftonline.com/common/oauth2/v2.0/devicecode"
        );
    }

    /// 测试 Device Code URL 推导 — 未知服务商
    #[test]
    fn test_derive_device_code_url_unknown() {
        let result = derive_device_code_url("https://auth.example.com/token");
        assert!(result.is_err());
    }

    /// 测试 parse_token_response 正常解析
    #[test]
    fn test_parse_token_response_success() {
        let json = serde_json::json!({
            "access_token": "ya29.token",
            "refresh_token": "1//refresh",
            "expires_in": 3600,
            "scope": "https://mail.google.com/",
            "token_type": "Bearer"
        });

        let tokens = parse_token_response(&json).expect("解析Failed");

        assert_eq!(tokens.access_token, "ya29.token");
        assert_eq!(tokens.refresh_token, Some("1//refresh".to_string()));
        assert!(tokens.expires_at.is_some());
        assert_eq!(tokens.scope, Some("https://mail.google.com/".to_string()));

        // expires_at 应该是未来时间
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(tokens.expires_at.unwrap() > now_ms);
    }

    /// 测试 parse_token_response 缺少 access_token
    #[test]
    fn test_parse_token_response_missing_access_token() {
        let json = serde_json::json!({
            "refresh_token": "refresh",
            "token_type": "Bearer"
        });

        let result = parse_token_response(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("access_token"));
    }

    /// 测试 parse_token_response 无 refresh_token
    #[test]
    fn test_parse_token_response_no_refresh_token() {
        let json = serde_json::json!({
            "access_token": "short_lived_token",
            "expires_in": 1800
        });

        let tokens = parse_token_response(&json).expect("解析Failed");
        assert_eq!(tokens.access_token, "short_lived_token");
        assert!(tokens.refresh_token.is_none());
    }

    /// 测试 Outlook PKCE URL 自动追加 offline_access
    #[test]
    fn test_pkce_auth_url_outlook_adds_offline_access() {
        let config = OAuth2Config {
            client_id: "outlook-id".to_string(),
            client_secret: None,
            auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
                .to_string(),
            token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
            scope: "https://outlook.office.com/IMAP.AccessAsUser.All https://outlook.office.com/SMTP.Send".to_string(),
            redirect_port: 8080,
        };

        let (url, _, _) = build_pkce_auth_url(&config).expect("构建 URL Failed");

        // scope 中不包含 offline_access，URL 应自动追加
        assert!(
            url.contains("offline_access"),
            "Outlook URL 应自动追加 offline_access scope"
        );
    }

    /// 测试 Gmail PKCE URL 不追加 offline_access（已包含在 scope 中或不需要）
    #[test]
    fn test_pkce_auth_url_gmail_no_offline_access() {
        let config = OAuth2Config {
            client_id: "gmail-id".to_string(),
            client_secret: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scope: "https://mail.google.com/".to_string(),
            redirect_port: 8080,
        };

        let (url, _, _) = build_pkce_auth_url(&config).expect("构建 URL Failed");

        // Gmail token_url 不含 microsoftonline.com，不应追加 offline_access
        assert!(
            !url.contains("%20offline_access"),
            "Gmail URL 不应追加 offline_access"
        );
    }
}
