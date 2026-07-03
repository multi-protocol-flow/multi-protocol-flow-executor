//! AMQP 1.0 客户端封装层
//!
//! 封装 fe2o3-amqp SDK，提供 AMQP 1.0 客户端操作。
//! AMQP 1.0 使用 Connection → Session → Link (Sender/Receiver) 模型。
//!
//! # 核心概念
//!
//! - **Connection**: 到 AMQP 1.0 容器（如 Azure Service Bus）的网络连接
//! - **Session**: 连接内的逻辑会话
//! - **Sender Link**: 发送消息到目标地址
//! - **Receiver Link**: 从源地址接收消息
//!
//! # 认证支持
//!
//! - SASL PLAIN: `amqp://user:pass@host:port`
//! - SASL ANONYMOUS: `amqp://host:port`
//! - TLS: `amqps://` 前缀（通过 rustls feature 启用）

use fe2o3_amqp::connection::ConnectionHandle;
use fe2o3_amqp::session::SessionHandle;
use fe2o3_amqp::types::messaging::{AmqpValue, Body, Message, Outcome, Properties};
use fe2o3_amqp::{Connection, Delivery, Receiver, Sender, Session};
use log::info;

use flow_engine_core::node_config::Amqp10MessageProperties;

// ---------------------------------------------------------------------------
// 类型别名
// ---------------------------------------------------------------------------

/// AMQP 1.0 连接类型
pub type Amqp10Connection = ConnectionHandle<()>;

/// AMQP 1.0 会话类型
pub type Amqp10Session = SessionHandle<()>;

// ---------------------------------------------------------------------------
// 公共类型
// ---------------------------------------------------------------------------

/// TLS 模式
#[derive(Debug, Clone, PartialEq)]
pub enum Amqp10TlsMode {
    /// 无 TLS（明文连接，amqp://）
    None,
    /// 标准 TLS（验证证书，amqps://）
    Standard,
    /// 跳过证书验证
    SkipVerify,
}

impl Amqp10TlsMode {
    /// 从配置字符串解析 TLS 模式
    pub fn from_config(tls_mode: Option<&str>) -> Self {
        match tls_mode {
            Some(mode) => match mode.to_lowercase().as_str() {
                "standard" => Amqp10TlsMode::Standard,
                "skip_verify" | "skip-verify" | "skipverify" => Amqp10TlsMode::SkipVerify,
                _ => Amqp10TlsMode::None,
            },
            None => Amqp10TlsMode::None,
        }
    }
}

/// 认证机制
#[derive(Debug, Clone, PartialEq)]
pub enum Amqp10AuthMechanism {
    /// SASL PLAIN（用户名/密码通过 URL 传递）
    Plain,
    /// SASL ANONYMOUS
    Anonymous,
    /// SASL EXTERNAL（基于 TLS 客户端证书）
    External,
}

impl Amqp10AuthMechanism {
    /// 从配置字符串解析认证机制
    pub fn from_config(mechanism: Option<&str>) -> Self {
        match mechanism {
            Some(m) => match m.to_lowercase().as_str() {
                "plain" => Amqp10AuthMechanism::Plain,
                "anonymous" => Amqp10AuthMechanism::Anonymous,
                "external" => Amqp10AuthMechanism::External,
                _ => Amqp10AuthMechanism::Plain,
            },
            None => Amqp10AuthMechanism::Plain,
        }
    }
}

// ---------------------------------------------------------------------------
// 连接创建
// ---------------------------------------------------------------------------

/// 异步创建 AMQP 1.0 连接
///
/// fe2o3-amqp 通过 URL 格式处理 TLS 和 SASL 认证：
/// - `amqp://user:pass@host:port` → SASL PLAIN 明文连接
/// - `amqps://user:pass@host:port` → SASL PLAIN + TLS
/// - `amqp://host:port` → SASL ANONYMOUS 明文连接
pub async fn connect_async(
    container_id: &str,
    url: &str,
) -> Result<Amqp10Connection, String> {
    info!("AMQP 1.0 开始连接: url={}, container={}", url, container_id);

    let connection = Connection::open(container_id, url)
        .await
        .map_err(|e| format!("AMQP 1.0 connection failed: {}", e))?;

    info!("AMQP 1.0 连接Success: url={}", url);
    Ok(connection)
}

// ---------------------------------------------------------------------------
// Session 操作
// ---------------------------------------------------------------------------

/// 创建 AMQP 1.0 Session
pub async fn begin_session(
    connection: &mut Amqp10Connection,
) -> Result<Amqp10Session, String> {
    Session::begin(connection)
        .await
        .map_err(|e| format!("AMQP 1.0 创建会话Failed: {}", e))
}

// ---------------------------------------------------------------------------
// Link 操作
// ---------------------------------------------------------------------------

/// 创建 Sender Link
///
/// `name` 为链接名称，`address` 为目标地址。
pub async fn attach_sender(
    session: &mut Amqp10Session,
    name: &str,
    address: &str,
) -> Result<Sender, String> {
    info!("AMQP 1.0 创建 Sender: name={}, address={}", name, address);
    Sender::attach(session, name, address)
        .await
        .map_err(|e| format!("AMQP 1.0 创建 Sender '{}' Failed: {}", name, e))
}

/// 创建 Receiver Link
///
/// `name` 为链接名称，`address` 为源地址。
pub async fn attach_receiver(
    session: &mut Amqp10Session,
    name: &str,
    address: &str,
) -> Result<Receiver, String> {
    info!("AMQP 1.0 创建 Receiver: name={}, address={}", name, address);
    Receiver::attach(session, name, address)
        .await
        .map_err(|e| format!("AMQP 1.0 创建 Receiver '{}' Failed: {}", name, e))
}

// ---------------------------------------------------------------------------
// Send / Receive
// ---------------------------------------------------------------------------

/// 发送字符串消息
pub async fn send_string(sender: &mut Sender, body: &str) -> Result<(), String> {
    let outcome: Outcome = sender
        .send(body)
        .await
        .map_err(|e| format!("AMQP 1.0 发送消息Failed: {}", e))?;

    outcome
        .accepted_or_else(|state| format!("AMQP 1.0 消息被 Broker 拒绝: {:?}", state))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 发送带属性的字符串消息
pub async fn send_message_with_properties(
    sender: &mut Sender,
    body: &str,
    properties: Properties,
) -> Result<(), String> {
    let message: Message<Body<String>> = Message {
        header: None,
        delivery_annotations: None,
        message_annotations: None,
        properties: Some(properties),
        application_properties: None,
        body: Body::Value(AmqpValue(body.to_string())),
        footer: None,
    };
    let outcome: Outcome = sender
        .send(message)
        .await
        .map_err(|e| format!("AMQP 1.0 发送消息Failed: {}", e))?;

    outcome
        .accepted_or_else(|state| format!("AMQP 1.0 消息被 Broker 拒绝: {:?}", state))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 接收字符串消息
pub async fn receive_string(receiver: &mut Receiver) -> Result<Delivery<String>, String> {
    let delivery = receiver
        .recv::<String>()
        .await
        .map_err(|e| format!("AMQP 1.0 接收消息Failed: {}", e))?;

    info!("AMQP 1.0 接收消息Success: delivery_id={}", delivery.delivery_id());
    Ok(delivery)
}

// ---------------------------------------------------------------------------
// Accept / Reject
// ---------------------------------------------------------------------------

/// 确认消息（Accept）
pub async fn accept_delivery(
    receiver: &mut Receiver,
    delivery: &Delivery<String>,
) -> Result<(), String> {
    receiver
        .accept(delivery)
        .await
        .map_err(|e| format!("AMQP 1.0 确认消息Failed: {}", e))?;
    info!("AMQP 1.0 确认消息Success: delivery_id={}", delivery.delivery_id());
    Ok(())
}

/// 拒绝消息（Reject）
pub async fn reject_delivery(
    receiver: &mut Receiver,
    delivery: &Delivery<String>,
    error: Option<fe2o3_amqp::types::definitions::Error>,
) -> Result<(), String> {
    receiver
        .reject(delivery, error)
        .await
        .map_err(|e| format!("AMQP 1.0 拒绝消息Failed: {}", e))?;
    info!("AMQP 1.0 拒绝消息Success: delivery_id={}", delivery.delivery_id());
    Ok(())
}

// ---------------------------------------------------------------------------
// Connection / Session 关闭
// ---------------------------------------------------------------------------

/// 关闭 AMQP 1.0 连接
pub async fn close_connection(connection: &mut Amqp10Connection) -> Result<(), String> {
    connection
        .close()
        .await
        .map_err(|e| format!("AMQP 1.0 关闭connection failed: {}", e))?;
    info!("AMQP 1.0 连接已关闭");
    Ok(())
}

/// 关闭 AMQP 1.0 会话
pub async fn end_session(session: &mut Amqp10Session) -> Result<(), String> {
    session
        .end()
        .await
        .map_err(|e| format!("AMQP 1.0 关闭会话Failed: {}", e))?;
    info!("AMQP 1.0 会话已关闭");
    Ok(())
}

// ---------------------------------------------------------------------------
// 消息构建辅助
// ---------------------------------------------------------------------------

/// 生成容器 ID
pub fn generate_container_id() -> String {
    format!("mpe-amqp10-{}", chrono::Utc::now().timestamp_millis())
}

/// 生成链接名称
pub fn generate_link_name(prefix: &str) -> String {
    format!(
        "{}-{}",
        prefix,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    )
}

/// 构建字符串消息体（仅用于测试）
#[cfg(test)]
fn build_string_message(body: &str, properties: Option<Properties>) -> Message<Body<String>> {
    Message {
        header: None,
        delivery_annotations: None,
        message_annotations: None,
        properties,
        application_properties: None,
        body: Body::Value(AmqpValue(body.to_string())),
        footer: None,
    }
}

/// 从配置构建 AMQP 1.0 Properties
pub fn build_properties(config: &Amqp10MessageProperties) -> Properties {
    let mut props = Properties::default();

    if let Some(id) = &config.message_id {
        props.message_id = Some(fe2o3_amqp::types::messaging::MessageId::String(id.clone()));
    }
    if let Some(ct) = &config.content_type {
        props.content_type = Some(ct.as_str().into());
    }
    if let Some(ce) = &config.content_encoding {
        props.content_encoding = Some(ce.as_str().into());
    }
    if let Some(cid) = &config.correlation_id {
        props.correlation_id = Some(fe2o3_amqp::types::messaging::MessageId::String(
            cid.clone(),
        ));
    }
    if let Some(rt) = &config.reply_to {
        props.reply_to = Some(rt.as_str().into());
    }
    if let Some(sub) = &config.subject {
        props.subject = Some(sub.clone());
    }
    if let Some(to) = &config.to {
        props.to = Some(to.as_str().into());
    }
    if let Some(ts) = config.absolute_expiry_time {
        props.absolute_expiry_time =
            Some(fe2o3_amqp::types::primitives::Timestamp::from_milliseconds(ts as i64));
    }
    if let Some(ts) = config.creation_time {
        props.creation_time =
            Some(fe2o3_amqp::types::primitives::Timestamp::from_milliseconds(ts as i64));
    }

    props
}

/// 将 Delivery 的属性提取为 JSON
pub fn extract_delivery_properties(delivery: &Delivery<String>) -> Option<serde_json::Value> {
    let msg = delivery.message();
    msg.properties.as_ref().and_then(|p| serde_json::to_value(p).ok())
}

/// 将 Delivery 的消息注解提取为 JSON
pub fn extract_delivery_annotations(delivery: &Delivery<String>) -> Option<serde_json::Value> {
    let msg = delivery.message();
    msg.message_annotations
        .as_ref()
        .and_then(|a| serde_json::to_value(a).ok())
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- TLS 模式解析测试 ----

    #[test]
    fn test_tls_mode_none() {
        assert_eq!(Amqp10TlsMode::from_config(None), Amqp10TlsMode::None);
        assert_eq!(Amqp10TlsMode::from_config(Some("none")), Amqp10TlsMode::None);
        assert_eq!(Amqp10TlsMode::from_config(Some("")), Amqp10TlsMode::None);
    }

    #[test]
    fn test_tls_mode_standard() {
        assert_eq!(
            Amqp10TlsMode::from_config(Some("standard")),
            Amqp10TlsMode::Standard
        );
        assert_eq!(
            Amqp10TlsMode::from_config(Some("Standard")),
            Amqp10TlsMode::Standard
        );
    }

    #[test]
    fn test_tls_mode_skip_verify() {
        assert_eq!(
            Amqp10TlsMode::from_config(Some("skip_verify")),
            Amqp10TlsMode::SkipVerify
        );
        assert_eq!(
            Amqp10TlsMode::from_config(Some("skip-verify")),
            Amqp10TlsMode::SkipVerify
        );
    }

    // ---- 认证机制解析测试 ----

    #[test]
    fn test_auth_mechanism_default() {
        assert_eq!(
            Amqp10AuthMechanism::from_config(None),
            Amqp10AuthMechanism::Plain
        );
    }

    #[test]
    fn test_auth_mechanism_variants() {
        assert_eq!(
            Amqp10AuthMechanism::from_config(Some("plain")),
            Amqp10AuthMechanism::Plain
        );
        assert_eq!(
            Amqp10AuthMechanism::from_config(Some("anonymous")),
            Amqp10AuthMechanism::Anonymous
        );
        assert_eq!(
            Amqp10AuthMechanism::from_config(Some("external")),
            Amqp10AuthMechanism::External
        );
    }

    // ---- 容器 ID 和链接名称生成测试 ----

    #[test]
    fn test_generate_container_id() {
        let id = generate_container_id();
        assert!(id.starts_with("mpe-amqp10-"));
        assert!(id.len() > "mpe-amqp10-".len());
    }

    #[test]
    fn test_generate_link_name() {
        let name = generate_link_name("sender");
        assert!(name.starts_with("sender-"));
    }

    // ---- 消息构建测试 ----

    #[test]
    fn test_build_string_message_body_only() {
        let msg = build_string_message("hello AMQP 1.0", None);
        match &msg.body {
            Body::Value(val) => assert_eq!(val.0, "hello AMQP 1.0"),
            _ => panic!("预期 Body::Value"),
        }
        assert!(msg.properties.is_none());
    }

    #[test]
    fn test_build_string_message_with_properties() {
        let props = Properties::default();
        let msg = build_string_message("body", Some(props));
        assert!(msg.properties.is_some());
    }

    // ---- Properties 构建测试 ----

    #[test]
    fn test_build_properties_empty() {
        let config = Amqp10MessageProperties::default();
        let props = build_properties(&config);
        assert!(props.message_id.is_none());
        assert!(props.content_type.is_none());
    }

    #[test]
    fn test_build_properties_with_fields() {
        let config = Amqp10MessageProperties {
            message_id: Some("msg-001".to_string()),
            content_type: Some("application/json".to_string()),
            subject: Some("test".to_string()),
            ..Default::default()
        };
        let props = build_properties(&config);
        assert!(props.message_id.is_some());
        assert!(props.content_type.is_some());
        assert_eq!(props.subject, Some("test".to_string()));
    }

    #[test]
    fn test_build_properties_with_all_fields() {
        let config = Amqp10MessageProperties {
            content_type: Some("text/plain".to_string()),
            content_encoding: Some("utf-8".to_string()),
            correlation_id: Some("corr-123".to_string()),
            message_id: Some("msg-456".to_string()),
            reply_to: Some("reply-addr".to_string()),
            subject: Some("subject".to_string()),
            to: Some("target-addr".to_string()),
            absolute_expiry_time: Some(9999999999999),
            creation_time: Some(1234567890000),
        };
        let props = build_properties(&config);
        assert!(props.message_id.is_some());
        assert!(props.to.is_some());
        assert!(props.absolute_expiry_time.is_some());
        assert!(props.creation_time.is_some());
    }

    // ---- AmqpValue 测试 ----

    #[test]
    fn test_amqp_value_string() {
        let val = AmqpValue("hello".to_string());
        assert_eq!(val.0, "hello");
    }
}
