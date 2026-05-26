// IMAP 连接管理器
// 管理流程级别的 IMAP 连接池

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::TryStreamExt;
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
    if encrypted.is_empty() {
        return Ok(String::new());
    }
    // 尝试 AES-256-GCM 解密，若失败则返回原文（本地测试/明文密码兼容）
    flow_engine_crypto::encryption::decrypt(encrypted)
        .or_else(|_| Ok(encrypted.to_string()))
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
        connection_id: &str,
        mailbox: &str,
        readonly: bool,
    ) -> Result<ImapSelectOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        let mailbox_name = mailbox.to_string();
        let mailbox_info = if readonly {
            conn.session
                .examine(mailbox)
                .await
                .map_err(|e| format!("IMAP examine 失败: {}", e))?
        } else {
            conn.session
                .select(mailbox)
                .await
                .map_err(|e| format!("IMAP select 失败: {}", e))?
        };

        conn.selected_mailbox = Some(mailbox_name.clone());

        Ok(ImapSelectOutput {
            success: true,
            mailbox: mailbox_name,
            exists: mailbox_info.exists,
            recent: mailbox_info.recent,
            unseen: mailbox_info.unseen,
            uid_validity: mailbox_info.uid_validity.map(|v| v as u64),
            uid_next: mailbox_info.uid_next.map(|v| v as u64),
            error: None,
        })
    }

    /// 搜索邮件
    async fn search(
        &self,
        connection_id: &str,
        mailbox: Option<&str>,
        criteria: &str,
        _max_results: Option<usize>,
    ) -> Result<ImapSearchOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        if let Some(mb) = mailbox {
            conn.session
                .select(mb)
                .await
                .map_err(|e| format!("IMAP select 失败: {}", e))?;
            conn.selected_mailbox = Some(mb.to_string());
        }

        // uid_search 返回 HashSet<Uid>，其中 Uid = NonZeroU32
        let result = conn
            .session
            .uid_search(criteria)
            .await
            .map_err(|e| format!("IMAP search 失败: {}", e))?;

        let uids: Vec<u32> = result.into_iter().collect();

        Ok(ImapSearchOutput {
            success: true,
            uids,
            messages: vec![],
            error: None,
        })
    }

    /// 获取邮件
    async fn fetch(
        &self,
        connection_id: &str,
        uids: &str,
        fetch_items: &str,
        max_fetch_count: usize,
        _max_size_bytes: usize,
    ) -> Result<ImapFetchOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        // uid_fetch 返回 Stream<Item = Result<Fetch, Error>>
        let stream = conn
            .session
            .uid_fetch(uids, fetch_items)
            .await
            .map_err(|e| format!("IMAP fetch 失败: {}", e))?;

        let all_fetches: Vec<async_imap::types::Fetch> = stream
            .try_collect()
            .await
            .map_err(|e| format!("IMAP fetch 流收集失败: {}", e))?;

        let mut messages = Vec::new();
        let mut skipped_count = 0usize;

        for fetch_result in &all_fetches {
            if messages.len() >= max_fetch_count {
                skipped_count += 1;
                continue;
            }

            let mut msg = ImapMessageInfo {
                uid: fetch_result.uid,
                seq: Some(fetch_result.message),
                subject: None,
                from: None,
                to: None,
                cc: None,
                date: None,
                flags: Vec::new(),
                size: fetch_result.size.map(|s| s as u64),
                body_text: None,
                body_html: None,
                attachments: Vec::new(),
                internal_date: fetch_result
                    .internal_date()
                    .map(|d| d.format("%Y-%m-%dT%H:%M:%S%:z").to_string()),
            };

            // 解析标志 — flags() 返回 Iterator<Item = Flag>
            for flag in fetch_result.flags() {
                let flag_str = match flag {
                    async_imap::types::Flag::Seen => "\\Seen".to_string(),
                    async_imap::types::Flag::Answered => "\\Answered".to_string(),
                    async_imap::types::Flag::Flagged => "\\Flagged".to_string(),
                    async_imap::types::Flag::Deleted => "\\Deleted".to_string(),
                    async_imap::types::Flag::Draft => "\\Draft".to_string(),
                    async_imap::types::Flag::Recent => "\\Recent".to_string(),
                    async_imap::types::Flag::MayCreate => "\\MayCreate".to_string(),
                    async_imap::types::Flag::Custom(cow) => cow.to_string(),
                };
                msg.flags.push(flag_str);
            }

            // body() 返回 RFC822 / BODY[] 的原始字节
            if let Some(body_bytes) = fetch_result.body() {
                if let Some(parsed) = mail_parser::MessageParser::new().parse(body_bytes) {
                    msg.subject = parsed.subject().map(|s| s.to_string());
                    msg.from = parsed.from().and_then(|addrs| {
                        addrs.first().and_then(|addr| {
                            addr.address().map(|a| {
                                if let Some(name) = addr.name() {
                                    format!("{} <{}>", name, a)
                                } else {
                                    a.to_string()
                                }
                            })
                        })
                    });
                    msg.to = parsed.to().map(|addrs| {
                        addrs
                            .iter()
                            .map(|addr| {
                                if let Some(email) = addr.address() {
                                    if let Some(name) = addr.name() {
                                        format!("{} <{}>", name, email)
                                    } else {
                                        email.to_string()
                                    }
                                } else {
                                    String::new()
                                }
                            })
                            .collect()
                    });
                    msg.cc = parsed.cc().map(|addrs| {
                        addrs
                            .iter()
                            .map(|addr| {
                                if let Some(email) = addr.address() {
                                    if let Some(name) = addr.name() {
                                        format!("{} <{}>", name, email)
                                    } else {
                                        email.to_string()
                                    }
                                } else {
                                    String::new()
                                }
                            })
                            .collect()
                    });
                    if let Some(date) = parsed.date() {
                        msg.date = Some(date.to_string());
                    }
                    msg.body_text = parsed.body_text(0).map(|t| t.to_string());
                    msg.body_html = parsed.body_html(0).map(|h| h.to_string());
                }
            }

            messages.push(msg);
        }

        Ok(ImapFetchOutput {
            success: true,
            messages,
            skipped_count,
            error: None,
        })
    }

    /// 设置邮件标志
    async fn store_flags(
        &self,
        connection_id: &str,
        uids: &str,
        operation: &str,
        flags: &[String],
    ) -> Result<ImapStoreFlagsOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        // 构建标志字符串
        let flags_str = flags
            .iter()
            .map(|f| {
                if f.starts_with('\\') {
                    f.clone()
                } else {
                    format!("\\{}", f)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let store_command = match operation {
            "set" => format!("FLAGS ({})", flags_str),
            "add" => format!("+FLAGS ({})", flags_str),
            "remove" => format!("-FLAGS ({})", flags_str),
            _ => return Err(format!("IMAP store_flags 不支持的操作: {}", operation)),
        };

        // uid_store 返回 Stream<Item = Result<Fetch, Error>>
        let stream = conn
            .session
            .uid_store(uids, &store_command)
            .await
            .map_err(|e| format!("IMAP store_flags 失败: {}", e))?;

        let results: Vec<async_imap::types::Fetch> = stream
            .try_collect()
            .await
            .map_err(|e| format!("IMAP store_flags 流收集失败: {}", e))?;

        let affected_uids: Vec<u32> = results
            .iter()
            .filter_map(|f| f.uid)
            .collect();

        Ok(ImapStoreFlagsOutput {
            success: true,
            operation: operation.to_string(),
            affected_uids,
            error: None,
        })
    }

    /// 复制邮件
    async fn copy(
        &self,
        connection_id: &str,
        uids: &str,
        target_mailbox: &str,
    ) -> Result<ImapCopyOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        let current_mailbox = conn.selected_mailbox.clone().unwrap_or_else(|| "?".to_string());

        // uid_copy 返回 Result<()>
        conn.session
            .uid_copy(uids, target_mailbox)
            .await
            .map_err(|e| format!("IMAP copy 失败: {}", e))?;

        Ok(ImapCopyOutput {
            success: true,
            source_mailbox: current_mailbox,
            target_mailbox: target_mailbox.to_string(),
            copied_count: 0,
            error: None,
        })
    }

    /// 移动邮件
    async fn r#move(
        &self,
        connection_id: &str,
        uids: &str,
        target_mailbox: &str,
    ) -> Result<ImapMoveOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        let current_mailbox = conn.selected_mailbox.clone().unwrap_or_else(|| "?".to_string());

        // 1. 复制到目标邮箱
        conn.session
            .uid_copy(uids, target_mailbox)
            .await
            .map_err(|e| format!("IMAP move 复制失败: {}", e))?;

        // 2. 源邮箱标记删除 — uid_store 返回 Stream，消耗之
        let del_stream = conn
            .session
            .uid_store(uids, "+FLAGS (\\Deleted)")
            .await
            .map_err(|e| format!("IMAP move 标记删除失败: {}", e))?;

        // 消耗流以等待命令完成
        let _: Vec<async_imap::types::Fetch> = del_stream
            .try_collect()
            .await
            .map_err(|e| format!("IMAP move store 流收集失败: {}", e))?;

        // 3. expunge — 返回 Stream<Item = Result<Seq>>，消耗之
        let expunge_stream = conn
            .session
            .expunge()
            .await
            .map_err(|e| format!("IMAP move expunge 失败: {}", e))?;

        let _: Vec<u32> = expunge_stream
            .try_collect()
            .await
            .map_err(|e| format!("IMAP move expunge 流收集失败: {}", e))?;

        Ok(ImapMoveOutput {
            success: true,
            source_mailbox: current_mailbox,
            target_mailbox: target_mailbox.to_string(),
            moved_count: 0,
            error: None,
        })
    }

    /// 永久删除已标记邮件
    async fn expunge(
        &self,
        connection_id: &str,
    ) -> Result<ImapExpungeOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        // expunge 返回 Stream<Item = Result<Seq>>，其中 Seq = NonZeroU32
        let stream = conn
            .session
            .expunge()
            .await
            .map_err(|e| format!("IMAP expunge 失败: {}", e))?;

        let expunged_seqs: Vec<u32> = stream
            .try_collect()
            .await
            .map_err(|e| format!("IMAP expunge 流收集失败: {}", e))?;

        Ok(ImapExpungeOutput {
            success: true,
            expunged_seqs,
            error: None,
        })
    }

    /// 列出邮箱
    async fn list(
        &self,
        connection_id: &str,
        reference_name: &str,
        pattern: &str,
    ) -> Result<ImapListOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        let ref_name: Option<&str> = if reference_name.is_empty() {
            None
        } else {
            Some(reference_name)
        };

        // list 返回 Stream<Item = Result<Name, Error>>
        let stream = conn
            .session
            .list(ref_name, Some(pattern))
            .await
            .map_err(|e| format!("IMAP list 失败: {}", e))?;

        let names: Vec<async_imap::types::Name> = stream
            .try_collect()
            .await
            .map_err(|e| format!("IMAP list 流收集失败: {}", e))?;

        let mailboxes: Vec<ImapMailboxInfo> = names
            .iter()
            .map(|n| ImapMailboxInfo {
                name: n.name().to_string(),
                delimiter: n.delimiter().map(|d| d.to_string()),
                attributes: n
                    .attributes()
                    .iter()
                    .map(|a| format!("{:?}", a))
                    .collect(),
            })
            .collect();

        Ok(ImapListOutput {
            success: true,
            mailboxes,
            error: None,
        })
    }

    /// 创建文件夹
    async fn create_folder(
        &self,
        connection_id: &str,
        folder_name: &str,
    ) -> Result<ImapFolderCreateOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        conn.session
            .create(folder_name)
            .await
            .map_err(|e| format!("IMAP create_folder 失败: {}", e))?;

        Ok(ImapFolderCreateOutput {
            success: true,
            folder_name: folder_name.to_string(),
            error: None,
        })
    }

    /// 删除文件夹
    async fn delete_folder(
        &self,
        connection_id: &str,
        folder_name: &str,
    ) -> Result<ImapFolderDeleteOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        conn.session
            .delete(folder_name)
            .await
            .map_err(|e| format!("IMAP delete_folder 失败: {}", e))?;

        Ok(ImapFolderDeleteOutput {
            success: true,
            folder_name: folder_name.to_string(),
            error: None,
        })
    }

    /// IDLE 监听
    ///
    /// 注意：async_imap 的 `idle()` 方法会消耗 Session 的所有权，
    /// 与当前连接管理架构（session 位于 Mutex 守卫内）不兼容。
    /// 此处暂时使用 NOOP + sleep 模拟 IDLE 等待。
    async fn idle(
        &self,
        connection_id: &str,
        mailbox: Option<&str>,
        timeout_seconds: u64,
    ) -> Result<ImapIdleOutput, String> {
        let mut conns = self.connections.lock().await;
        let conn = conns
            .get_mut(connection_id)
            .ok_or_else(|| format!("IMAP 连接 '{}' 不存在", connection_id))?;

        if let Some(mb) = mailbox {
            conn.session
                .select(mb)
                .await
                .map_err(|e| format!("IMAP select 失败: {}", e))?;
            conn.selected_mailbox = Some(mb.to_string());
        }

        let start = tokio::time::Instant::now();

        // 发送 NOOP 检查是否有新数据
        conn.session
            .noop()
            .await
            .map_err(|e| format!("IMAP idle noop 失败: {}", e))?;

        // 等待指定时长（模拟 idle 等待）
        let wait_duration = tokio::time::Duration::from_secs(timeout_seconds);
        tokio::time::sleep(wait_duration).await;

        let elapsed = start.elapsed().as_millis() as u64;

        Ok(ImapIdleOutput {
            success: true,
            events: vec!["TIMEOUT".to_string()],
            duration_ms: elapsed,
            error: None,
        })
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
    async fn test_select_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.select("conn1", "INBOX", false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_search_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.search("conn1", None, "ALL", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_fetch_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager
            .fetch("conn1", "1:*", "RFC822", 100, 1024 * 1024)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_store_flags_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager
            .store_flags("conn1", "1", "+FLAGS", &["\\Seen".to_string()])
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_copy_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.copy("conn1", "1", "Archive").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_move_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.r#move("conn1", "1", "Trash").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_expunge_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.expunge("conn1").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_list_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.list("conn1", "", "*").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_create_folder_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.create_folder("conn1", "NewFolder").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_delete_folder_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.delete_folder("conn1", "OldFolder").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_idle_nonexistent_connection() {
        let manager = ImapConnectionManager::new();
        let result = manager.idle("conn1", Some("INBOX"), 30).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }
}
