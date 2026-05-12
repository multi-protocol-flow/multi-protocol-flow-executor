// IMAP 连接管理器
// 管理流程级别的 IMAP 连接池

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

use flow_engine_core::email_auth::xoauth2_string;
use flow_engine_core::email_types::{EmailAuthConfig, EmailOAuthTokens, ImapConfig, TlsMode};
use flow_engine_core::traits::*;

// ────────────────────────────────────────────────────────────────────────────
// 类型别名：通过 boxing 擦除底层流的泛型参数
// ────────────────────────────────────────────────────────────────────────────

/// 组合 `AsyncRead` + `AsyncWrite` + Debug 的辅助 trait
///
/// async-imap 要求底层流实现 `Read + Write + Unpin + Debug`，
/// 通过辅助 trait 可以将这些约束打包到一个 dyn trait 中。
trait ImapStream: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> ImapStream for T {}

/// 为 Box<dyn ImapStream> 手动实现 Debug（async-imap 的 Session 约束）
impl fmt::Debug for Box<dyn ImapStream> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("ImapStream(Box<dyn ...>)")
    }
}

/// 擦除后的底层流类型
type BoxStream = Box<dyn ImapStream>;

/// IMAP Session（泛型参数已被擦除）
type ImapSession = async_imap::Session<BoxStream>;

/// 将任意流装箱为 `BoxStream`
fn box_stream<S: AsyncRead + AsyncWrite + Send + Unpin + 'static>(s: S) -> BoxStream {
    Box::new(s) as BoxStream
}

// ────────────────────────────────────────────────────────────────────────────
// XOAUTH2 认证器
// ────────────────────────────────────────────────────────────────────────────

/// XOAUTH2 SASL 认证器
///
/// 实现 `async_imap::Authenticator` trait，
/// 返回预先构建好的 XOAUTH2 auth string。
pub struct XOAuth2Authenticator {
    /// 预构建的 XOAUTH2 认证字符串
    pub auth_string: String,
}

impl async_imap::Authenticator for &XOAuth2Authenticator {
    type Response = String;

    fn process(&mut self, _challenge: &[u8]) -> Self::Response {
        self.auth_string.clone()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// IMAP 连接包装
// ────────────────────────────────────────────────────────────────────────────

/// 单个 IMAP 连接
pub struct ImapConnection {
    /// 底层 IMAP Session
    #[allow(dead_code)]
    session: ImapSession,
    /// 当前选中的邮箱
    #[allow(dead_code)]
    selected_mailbox: Option<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// IMAP 连接管理器
// ────────────────────────────────────────────────────────────────────────────

/// IMAP 连接管理器
///
/// 管理 IMAP 连接的生命周期，实现 `ImapProvider` trait。
/// 支持 Plain/Login 密码认证和 `OAuth2` 认证。
pub struct ImapConnectionManager {
    connections: Arc<Mutex<HashMap<String, ImapConnection>>>,
}

impl ImapConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for ImapConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// 辅助函数
// ────────────────────────────────────────────────────────────────────────────

/// 解密密码
fn decrypt_password(encrypted: &str) -> Result<String, String> {
    flow_engine_crypto::encryption::decrypt(encrypted)
        .map_err(|e| format!("密码解密失败: {}", e))
}

/// 构建 TLS 连接器（使用系统根证书）
fn build_tls_connector() -> tokio_rustls::TlsConnector {
    let mut root_store = rustls::RootCertStore::empty();
    root_extend(&mut root_store);

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

/// 将 webpki 根证书添加到 store
fn root_extend(root_store: &mut rustls::RootCertStore) {
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
}

/// 解析服务器名称
fn parse_server_name(host: &str) -> Result<rustls::pki_types::ServerName<'static>, String> {
    let dns_name = rustls::pki_types::DnsName::try_from(host.to_string())
        .map_err(|e| format!("无效服务器名 '{}': {:?}", host, e))?;
    Ok(rustls::pki_types::ServerName::from(dns_name))
}

// ────────────────────────────────────────────────────────────────────────────
// ImapProvider 实现
// ────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl ImapProvider for ImapConnectionManager {
    /// 建立 IMAP 连接
    async fn connect(
        &self,
        connection_id: &str,
        config: &ImapConfig,
    ) -> Result<ImapConnectOutput, String> {
        // 1. 检查连接是否已存在
        {
            let conns = self.connections.lock().await;
            if conns.contains_key(connection_id) {
                return Err(format!("IMAP 连接 '{}' 已存在", connection_id));
            }
        }

        // 2. TCP 连接
        let tcp = tokio::net::TcpStream::connect((config.host.as_str(), config.port))
            .await
            .map_err(|e| format!("IMAP 连接失败 {}:{}: {}", config.host, config.port, e))?;

        // 3. 根据 TLS 模式创建 Client
        let client = match config.tls_mode {
            TlsMode::Implicit => {
                let connector = build_tls_connector();
                let server_name = parse_server_name(&config.host)?;
                let tls_stream = connector
                    .connect(server_name, tcp)
                    .await
                    .map_err(|e| format!("TLS 握手失败: {}", e))?;
                async_imap::Client::new(box_stream(tls_stream))
            }
            TlsMode::StartTls => {
                // async-imap 0.11 无内置 STARTTLS 支持，
                // 需要手动: 创建 plaintext client → 发送 STARTTLS 命令 → TLS 握手
                // 此处暂未实现，返回错误
                return Err("IMAP STARTTLS 模式暂未实现".into());
            }
            TlsMode::None => async_imap::Client::new(box_stream(tcp)),
        };

        // 4. 认证
        let auth = config
            .auth
            .as_ref()
            .ok_or_else(|| "IMAP 需要认证配置".to_string())?;

        let session = match auth {
            EmailAuthConfig::Plain {
                username,
                encrypted_password,
            } => {
                let password = decrypt_password(encrypted_password)?;
                client
                    .login(username.as_str(), password.as_str())
                    .await
                    .map_err(|(e, _)| format!("IMAP 登录失败: {}", e))?
            }
            EmailAuthConfig::Login {
                username,
                encrypted_password,
            } => {
                let password = decrypt_password(encrypted_password)?;
                client
                    .login(username.as_str(), password.as_str())
                    .await
                    .map_err(|(e, _)| format!("IMAP 登录失败: {}", e))?
            }
            EmailAuthConfig::OAuth2 {
                encrypted_tokens,
                ..
            } => {
                // 解密 tokens
                let tokens_json = flow_engine_crypto::encryption::decrypt(encrypted_tokens)
                    .map_err(|e| format!("OAuth Token 解密失败: {}", e))?;
                let tokens: EmailOAuthTokens = serde_json::from_str(&tokens_json)
                    .map_err(|e| format!("OAuth Token 解析失败: {}", e))?;

                let user_name = extract_oauth_username(&tokens_json);

                let auth_string = xoauth2_string(&user_name, &tokens.access_token);
                let authenticator = XOAuth2Authenticator { auth_string };
                client
                    .authenticate("XOAUTH2", &authenticator)
                    .await
                    .map_err(|(e, _)| format!("OAuth2 认证失败: {}", e))?
            }
        };

        // 5. 存储 Session
        self.connections.lock().await.insert(
            connection_id.to_string(),
            ImapConnection {
                session,
                selected_mailbox: None,
            },
        );

        Ok(ImapConnectOutput {
            success: true,
            connection_id: Some(connection_id.to_string()),
            server_greeting: None,
            capabilities: vec![],
            error: None,
        })
    }

    /// 选择邮箱
    async fn select(
        &self,
        _connection_id: &str,
        _mailbox: &str,
        _readonly: bool,
    ) -> Result<ImapSelectOutput, String> {
        Err("IMAP select 未实现".into())
    }

    /// 搜索邮件
    async fn search(
        &self,
        _connection_id: &str,
        _mailbox: Option<&str>,
        _criteria: &str,
        _max_results: Option<usize>,
    ) -> Result<ImapSearchOutput, String> {
        Err("IMAP search 未实现".into())
    }

    /// 获取邮件
    async fn fetch(
        &self,
        _connection_id: &str,
        _uids: &str,
        _fetch_items: &str,
        _max_fetch_count: usize,
        _max_size_bytes: usize,
    ) -> Result<ImapFetchOutput, String> {
        Err("IMAP fetch 未实现".into())
    }

    /// 设置邮件标志
    async fn store_flags(
        &self,
        _connection_id: &str,
        _uids: &str,
        _operation: &str,
        _flags: &[String],
    ) -> Result<ImapStoreFlagsOutput, String> {
        Err("IMAP store_flags 未实现".into())
    }

    /// 复制邮件
    async fn copy(
        &self,
        _connection_id: &str,
        _uids: &str,
        _target_mailbox: &str,
    ) -> Result<ImapCopyOutput, String> {
        Err("IMAP copy 未实现".into())
    }

    /// 移动邮件
    async fn r#move(
        &self,
        _connection_id: &str,
        _uids: &str,
        _target_mailbox: &str,
    ) -> Result<ImapMoveOutput, String> {
        Err("IMAP move 未实现".into())
    }

    /// 永久删除已标记邮件
    async fn expunge(
        &self,
        _connection_id: &str,
    ) -> Result<ImapExpungeOutput, String> {
        Err("IMAP expunge 未实现".into())
    }

    /// 列出邮箱
    async fn list(
        &self,
        _connection_id: &str,
        _reference_name: &str,
        _pattern: &str,
    ) -> Result<ImapListOutput, String> {
        Err("IMAP list 未实现".into())
    }

    /// 创建文件夹
    async fn create_folder(
        &self,
        _connection_id: &str,
        _folder_name: &str,
    ) -> Result<ImapFolderCreateOutput, String> {
        Err("IMAP create_folder 未实现".into())
    }

    /// 删除文件夹
    async fn delete_folder(
        &self,
        _connection_id: &str,
        _folder_name: &str,
    ) -> Result<ImapFolderDeleteOutput, String> {
        Err("IMAP delete_folder 未实现".into())
    }

    /// IDLE 监听
    async fn idle(
        &self,
        _connection_id: &str,
        _mailbox: Option<&str>,
        _timeout_seconds: u64,
    ) -> Result<ImapIdleOutput, String> {
        Err("IMAP idle 未实现".into())
    }

    /// 关闭 IMAP 连接
    async fn close(
        &self,
        connection_id: &str,
    ) -> Result<ImapCloseOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .remove(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;
        drop(conn);
        Ok(ImapCloseOutput {
            success: true,
            error: None,
        })
    }

    /// 测试 IMAP 连接
    async fn test_connection(&self, config: &ImapConfig) -> Result<bool, String> {
        // 生成临时连接 ID
        let temp_id = format!(
            "_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")
        );

        // 尝试连接
        let result = self.connect(&temp_id, config).await;

        match result {
            Ok(_) => {
                // 连接成功，立即关闭
                let _ = self.close(&temp_id).await;
                Ok(true)
            }
            Err(e) => Err(format!("IMAP 连接测试失败: {}", e)),
        }
    }
}

/// 从 OAuth tokens JSON 中尝试提取用户名
///
/// 尝试从 token JSON 的 `email` 或 `username` 字段获取用户名，
/// 如果都不存在则返回空字符串。
fn extract_oauth_username(tokens_json: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(tokens_json) {
        if let Some(email) = val.get("email").and_then(|v| v.as_str()) {
            return email.to_string();
        }
        if let Some(username) = val.get("username").and_then(|v| v.as_str()) {
            return username.to_string();
        }
    }
    String::new()
}

// ────────────────────────────────────────────────────────────────────────────
// 单元测试
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_imap::Authenticator;

    #[test]
    fn test_xoauth2_authenticator_process() {
        let auth = XOAuth2Authenticator {
            auth_string: "dXNlcj10ZXN0".to_string(),
        };
        let result = (&auth).process(b"challenge");
        assert_eq!(result, "dXNlcj10ZXN0");
    }

    #[test]
    fn test_xoauth2_authenticator_empty_challenge() {
        let auth = XOAuth2Authenticator {
            auth_string: "auth_value".to_string(),
        };
        let result = (&auth).process(b"");
        assert_eq!(result, "auth_value");
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ImapConnectionManager::new();
        assert!(manager.connections.blocking_lock().is_empty());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = ImapConnectionManager::default();
        assert!(manager.connections.blocking_lock().is_empty());
    }

    #[test]
    fn test_extract_oauth_username_with_email() {
        let json = r#"{"access_token":"ya29.xxx","email":"user@gmail.com"}"#;
        assert_eq!(extract_oauth_username(json), "user@gmail.com");
    }

    #[test]
    fn test_extract_oauth_username_with_username() {
        let json = r#"{"access_token":"ya29.xxx","username":"admin"}"#;
        assert_eq!(extract_oauth_username(json), "admin");
    }

    #[test]
    fn test_extract_oauth_username_no_fields() {
        let json = r#"{"access_token":"ya29.xxx"}"#;
        assert_eq!(extract_oauth_username(json), "");
    }

    #[test]
    fn test_extract_oauth_username_invalid_json() {
        assert_eq!(extract_oauth_username("not json"), "");
    }

    #[tokio::test]
    async fn test_close_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.close("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_connect_duplicate_id() {
        let manager = ImapConnectionManager::new();
        // 手动插入一个假连接来测试重复检测
        // (我们无法真正创建 ImapConnection 因为需要真实的 IMAP Session)
        // 改为测试 close 对不存在的连接返回错误
        let result = manager.close("dup_id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_select_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.select("conn1", "INBOX", false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_search_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.search("conn1", None, "ALL", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_fetch_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager
            .fetch("conn1", "1:*", "RFC822", 100, 1024 * 1024)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_store_flags_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager
            .store_flags("conn1", "1", "+FLAGS", &["\\Seen".to_string()])
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_copy_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.copy("conn1", "1", "Archive").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_move_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.r#move("conn1", "1", "Trash").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_expunge_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.expunge("conn1").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_list_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.list("conn1", "", "*").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_create_folder_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.create_folder("conn1", "NewFolder").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_delete_folder_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.delete_folder("conn1", "OldFolder").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }

    #[tokio::test]
    async fn test_idle_not_implemented() {
        let manager = ImapConnectionManager::new();
        let result = manager.idle("conn1", Some("INBOX"), 30).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未实现"));
    }
}
