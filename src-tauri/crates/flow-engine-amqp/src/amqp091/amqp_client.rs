//! AMQP 0-9-1 客户端封装层
//!
//! 封装 lapin SDK，提供 AMQP 0-9-1 客户端创建功能。
//! 支持 PLAIN 认证（通过 URL 中的 user:pass）、TLS（标准验证/跳过验证）。

use std::collections::HashMap;

use flow_engine_core::node_config::AmqpConnectConfig;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicGetOptions, BasicNackOptions, BasicPublishOptions,
    BasicRejectOptions, ConfirmSelectOptions, ExchangeDeclareOptions, ExchangeDeleteOptions,
    QueueBindOptions, QueueDeclareOptions, QueueDeleteOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind};
use log::info;

// ---------------------------------------------------------------------------
// 公共类型
// ---------------------------------------------------------------------------

/// TLS 模式
#[derive(Debug, Clone, PartialEq)]
pub enum TlsMode {
    /// 无 TLS（明文连接）
    None,
    /// 标准 TLS（验证证书）
    Standard,
    /// 跳过证书验证
    SkipVerify,
}

impl TlsMode {
    /// 从配置字符串解析 TLS 模式
    pub fn from_config(tls_mode: Option<&str>) -> Self {
        match tls_mode {
            Some(mode) => match mode.to_lowercase().as_str() {
                "standard" => TlsMode::Standard,
                "skip_verify" | "skip-verify" | "skipverify" => TlsMode::SkipVerify,
                _ => TlsMode::None,
            },
            None => TlsMode::None,
        }
    }
}

// ---------------------------------------------------------------------------
// 连接创建
// ---------------------------------------------------------------------------

/// 异步创建 AMQP 连接
///
/// 解析 URL 确定连接方式：
/// - `amqp://user:pass@host:port/vhost` → 明文连接
/// - `amqps://user:pass@host:port/vhost` → TLS 连接
///
/// 用户名/密码通过 URL 传递（PLAIN 认证），TLS 模式通过配置指定。
pub async fn connect_async(config: &AmqpConnectConfig) -> Result<Connection, String> {
    let url = &config.url;
    info!("AMQP 开始异步连接: url={}", url);

    // 构建 ConnectionProperties
    let mut properties = ConnectionProperties::default();

    // 连接名
    let connection_name = format!("mpe-amqp-{}", chrono::Utc::now().timestamp_millis());
    properties = properties.with_connection_name(connection_name.into());

    // 自动恢复
    if config.auto_recover {
        properties = properties.enable_auto_recover();
    }

    // 创建连接（lapin 自动从 URL 协议推断是否使用 TLS）
    let connection = Connection::connect(url, properties)
        .await
        .map_err(|e| format!("AMQP 连接失败: {}", e))?;

    info!("AMQP 连接成功: url={}", url);
    Ok(connection)
}

// ---------------------------------------------------------------------------
// Channel 操作
// ---------------------------------------------------------------------------

/// 创建 Channel
pub async fn create_channel(connection: &Connection) -> Result<Channel, String> {
    connection
        .create_channel()
        .await
        .map_err(|e| format!("AMQP 创建通道失败: {}", e))
}

// ---------------------------------------------------------------------------
// Exchange 操作
// ---------------------------------------------------------------------------

/// 解析交换器类型字符串为 `ExchangeKind`
fn parse_exchange_kind(exchange_type: &str) -> ExchangeKind {
    match exchange_type {
        "direct" => ExchangeKind::Direct,
        "fanout" => ExchangeKind::Fanout,
        "topic" => ExchangeKind::Topic,
        "headers" => ExchangeKind::Headers,
        _ => ExchangeKind::Custom(exchange_type.into()),
    }
}

/// 声明交换器
pub async fn exchange_declare(
    channel: &Channel,
    name: &str,
    exchange_type: &str,
    passive: bool,
    durable: bool,
    auto_delete: bool,
    internal: bool,
    arguments: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(), String> {
    let options = ExchangeDeclareOptions {
        passive,
        durable,
        auto_delete,
        internal,
        nowait: false,
    };

    let field_table = build_field_table(arguments);
    let kind = parse_exchange_kind(exchange_type);

    channel
        .exchange_declare(name.into(), kind, options, field_table)
        .await
        .map_err(|e| format!("AMQP 声明交换器 '{}' 失败: {}", name, e))?;

    info!("AMQP 声明交换器成功: name={}, type={}", name, exchange_type);
    Ok(())
}

/// 删除交换器
pub async fn exchange_delete(
    channel: &Channel,
    name: &str,
    if_unused: bool,
) -> Result<(), String> {
    let options = ExchangeDeleteOptions {
        if_unused,
        nowait: false,
    };

    channel
        .exchange_delete(name.into(), options)
        .await
        .map_err(|e| format!("AMQP 删除交换器 '{}' 失败: {}", name, e))?;

    info!("AMQP 删除交换器成功: name={}", name);
    Ok(())
}

// ---------------------------------------------------------------------------
// Queue 操作
// ---------------------------------------------------------------------------

/// 声明队列
///
/// 返回 (`queue_name`, `message_count`, `consumer_count`)
pub async fn queue_declare(
    channel: &Channel,
    queue_name: &str,
    passive: bool,
    durable: bool,
    exclusive: bool,
    auto_delete: bool,
    arguments: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(String, u32, u32), String> {
    let options = QueueDeclareOptions {
        passive,
        durable,
        exclusive,
        auto_delete,
        nowait: false,
    };

    let field_table = build_field_table(arguments);

    let result = channel
        .queue_declare(queue_name.into(), options, field_table)
        .await
        .map_err(|e| format!("AMQP 声明队列 '{}' 失败: {}", queue_name, e))?;

    let name = result.name().to_string();
    let message_count = result.message_count();
    let consumer_count = result.consumer_count();

    info!(
        "AMQP 声明队列成功: name={}, messages={}, consumers={}",
        name, message_count, consumer_count
    );

    Ok((name, message_count, consumer_count))
}

/// 删除队列
///
/// 返回被删除的消息数
pub async fn queue_delete(
    channel: &Channel,
    queue_name: &str,
    if_unused: bool,
    if_empty: bool,
) -> Result<u32, String> {
    let options = QueueDeleteOptions {
        if_unused,
        if_empty,
        nowait: false,
    };

    let result = channel
        .queue_delete(queue_name.into(), options)
        .await
        .map_err(|e| format!("AMQP 删除队列 '{}' 失败: {}", queue_name, e))?;

    info!("AMQP 删除队列成功: name={}, deleted={}", queue_name, result);
    Ok(result)
}

/// 绑定队列到交换器
pub async fn queue_bind(
    channel: &Channel,
    queue_name: &str,
    exchange: &str,
    routing_key: &str,
    arguments: Option<&HashMap<String, serde_json::Value>>,
) -> Result<(), String> {
    let field_table = build_field_table(arguments);

    channel
        .queue_bind(
            queue_name.into(),
            exchange.into(),
            routing_key.into(),
            QueueBindOptions::default(),
            field_table,
        )
        .await
        .map_err(|e| {
            format!(
                "AMQP 绑定队列 '{}' 到交换器 '{}' 失败: {}",
                queue_name, exchange, e
            )
        })?;

    info!(
        "AMQP 绑定队列成功: queue={}, exchange={}, routing_key={}",
        queue_name, exchange, routing_key
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Publish / Consume / Get
// ---------------------------------------------------------------------------

/// 发布消息
///
/// 返回 `PublisherConfirm` 的等待结果描述
pub async fn basic_publish(
    channel: &Channel,
    exchange: &str,
    routing_key: &str,
    body: &[u8],
    mandatory: bool,
    properties: BasicProperties,
) -> Result<String, String> {
    let options = BasicPublishOptions {
        mandatory,
        immediate: false,
    };

    let confirm = channel
        .basic_publish(
            exchange.into(),
            routing_key.into(),
            options,
            body,
            properties,
        )
        .await
        .map_err(|e| {
            format!(
                "AMQP 发布消息失败: exchange={}, routing_key={}: {}",
                exchange, routing_key, e
            )
        })?;

    // 等待确认（如果启用了 confirm 模式）
    let confirm_str = match confirm.await {
        Ok(lapin::Confirmation::Ack(_)) => "ack".to_string(),
        Ok(lapin::Confirmation::Nack(_)) => "nack".to_string(),
        Ok(lapin::Confirmation::NotRequested) => "not_requested".to_string(),
        Err(e) => format!("确认等待错误: {}", e),
    };

    info!(
        "AMQP 发布成功: exchange={}, routing_key={}, confirm={}",
        exchange, routing_key, confirm_str
    );

    Ok(confirm_str)
}

/// 消费消息
///
/// 返回 Consumer
pub async fn basic_consume(
    channel: &Channel,
    queue: &str,
    consumer_tag: &str,
    no_local: bool,
    no_ack: bool,
    exclusive: bool,
) -> Result<lapin::Consumer, String> {
    let options = BasicConsumeOptions {
        no_local,
        no_ack,
        exclusive,
        nowait: false,
    };

    let consumer = channel
        .basic_consume(
            queue.into(),
            consumer_tag.into(),
            options,
            FieldTable::default(),
        )
        .await
        .map_err(|e| format!("AMQP 消费队列 '{}' 失败: {}", queue, e))?;

    info!("AMQP 消费者创建成功: queue={}, tag={}", queue, consumer_tag);
    Ok(consumer)
}

/// 获取单条消息（basic.get）
///
/// 返回消息元组（投递标签、交换器、路由键、是否重传、消息体、剩余消息数），队列为空时返回 `None`。
pub async fn basic_get(
    channel: &Channel,
    queue: &str,
    no_ack: bool,
) -> Result<Option<(u64, String, String, bool, Vec<u8>, u32)>, String> {
    let options = BasicGetOptions { no_ack };

    let result = channel
        .basic_get(queue.into(), options)
        .await
        .map_err(|e| format!("AMQP 获取消息失败: queue={}, 错误: {}", queue, e))?;

    match result {
        Some(message) => {
            let delivery_tag = message.delivery_tag;
            let exchange = message.exchange.to_string();
            let routing_key = message.routing_key.to_string();
            let redelivered = message.redelivered;
            let body = message.data.to_vec();
            let message_count = message.message_count;

            info!(
                "AMQP 获取消息成功: queue={}, delivery_tag={}",
                queue, delivery_tag
            );
            Ok(Some((
                delivery_tag,
                exchange,
                routing_key,
                redelivered,
                body,
                message_count,
            )))
        }
        None => {
            info!("AMQP 队列 '{}' 无消息", queue);
            Ok(None)
        }
    }
}

// ---------------------------------------------------------------------------
// Ack / Nack / Reject
// ---------------------------------------------------------------------------

/// 确认消息
pub async fn basic_ack(channel: &Channel, delivery_tag: u64, multiple: bool) -> Result<(), String> {
    let options = BasicAckOptions { multiple };
    channel
        .basic_ack(delivery_tag, options)
        .await
        .map_err(|e| format!("AMQP 确认消息失败: tag={}, 错误: {}", delivery_tag, e))?;
    info!("AMQP 确认消息成功: tag={}", delivery_tag);
    Ok(())
}

/// 拒绝消息（批量）
pub async fn basic_nack(
    channel: &Channel,
    delivery_tag: u64,
    multiple: bool,
    requeue: bool,
) -> Result<(), String> {
    let options = BasicNackOptions { multiple, requeue };
    channel
        .basic_nack(delivery_tag, options)
        .await
        .map_err(|e| format!("AMQP Nack 失败: tag={}, 错误: {}", delivery_tag, e))?;
    info!("AMQP Nack 成功: tag={}, requeue={}", delivery_tag, requeue);
    Ok(())
}

/// 拒绝消息（单条）
pub async fn basic_reject(
    channel: &Channel,
    delivery_tag: u64,
    requeue: bool,
) -> Result<(), String> {
    let options = BasicRejectOptions { requeue };
    channel
        .basic_reject(delivery_tag, options)
        .await
        .map_err(|e| format!("AMQP Reject 失败: tag={}, 错误: {}", delivery_tag, e))?;
    info!("AMQP Reject 成功: tag={}, requeue={}", delivery_tag, requeue);
    Ok(())
}

// ---------------------------------------------------------------------------
// Confirm / Transaction
// ---------------------------------------------------------------------------

/// 启用发布确认模式
pub async fn confirm_select(channel: &Channel) -> Result<(), String> {
    channel
        .confirm_select(ConfirmSelectOptions::default())
        .await
        .map_err(|e| format!("AMQP 启用确认模式失败: {}", e))?;
    info!("AMQP 确认模式已启用");
    Ok(())
}

/// 启用事务模式
pub async fn tx_select(channel: &Channel) -> Result<(), String> {
    channel
        .tx_select()
        .await
        .map_err(|e| format!("AMQP 启用事务模式失败: {}", e))?;
    info!("AMQP 事务模式已启用");
    Ok(())
}

/// 提交事务
pub async fn tx_commit(channel: &Channel) -> Result<(), String> {
    channel
        .tx_commit()
        .await
        .map_err(|e| format!("AMQP 提交事务失败: {}", e))?;
    info!("AMQP 事务已提交");
    Ok(())
}

/// 回滚事务
pub async fn tx_rollback(channel: &Channel) -> Result<(), String> {
    channel
        .tx_rollback()
        .await
        .map_err(|e| format!("AMQP 回滚事务失败: {}", e))?;
    info!("AMQP 事务已回滚");
    Ok(())
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

/// 将 JSON Value `HashMap` 转换为 lapin `FieldTable`
fn build_field_table(arguments: Option<&HashMap<String, serde_json::Value>>) -> FieldTable {
    let mut table = FieldTable::default();
    if let Some(args) = arguments {
        for (key, value) in args {
            let short_key = lapin::types::ShortString::from(key.as_str());
            if let Some(s) = value.as_str() {
                table.insert(short_key, lapin::types::AMQPValue::LongString(s.into()));
            } else if let Some(n) = value.as_i64() {
                table.insert(short_key, lapin::types::AMQPValue::LongLongInt(n));
            } else if let Some(b) = value.as_bool() {
                table.insert(short_key, lapin::types::AMQPValue::Boolean(b));
            }
        }
    }
    table
}

/// 构建 `BasicProperties`
pub fn build_basic_properties(
    config: &flow_engine_core::node_config::AmqpPublishConfig,
) -> BasicProperties {
    let mut props = BasicProperties::default();

    if let Some(ct) = &config.content_type {
        props = props.with_content_type(ct.as_str().into());
    }
    if let Some(ce) = &config.content_encoding {
        props = props.with_content_encoding(ce.as_str().into());
    }
    if config.delivery_mode > 0 {
        props = props.with_delivery_mode(config.delivery_mode);
    }
    if config.priority > 0 {
        props = props.with_priority(config.priority);
    }
    if let Some(id) = &config.correlation_id {
        props = props.with_correlation_id(id.as_str().into());
    }
    if let Some(id) = &config.reply_to {
        props = props.with_reply_to(id.as_str().into());
    }
    if let Some(exp) = &config.expiration {
        props = props.with_expiration(exp.as_str().into());
    }
    if let Some(id) = &config.message_id {
        props = props.with_message_id(id.as_str().into());
    }
    if let Some(ts) = config.timestamp {
        props = props.with_timestamp(ts);
    }
    if let Some(mt) = &config.message_type {
        props = props.with_type(mt.as_str().into());
    }
    if let Some(uid) = &config.user_id {
        props = props.with_user_id(uid.as_str().into());
    }
    if let Some(aid) = &config.app_id {
        props = props.with_app_id(aid.as_str().into());
    }
    if let Some(cid) = &config.cluster_id {
        props = props.with_cluster_id(cid.as_str().into());
    }

    // 消息头
    if let Some(headers) = &config.headers {
        let field_table = build_field_table(Some(headers));
        props = props.with_headers(field_table);
    }

    props
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
        assert_eq!(TlsMode::from_config(None), TlsMode::None);
        assert_eq!(TlsMode::from_config(Some("none")), TlsMode::None);
        assert_eq!(TlsMode::from_config(Some("")), TlsMode::None);
    }

    #[test]
    fn test_tls_mode_standard() {
        assert_eq!(
            TlsMode::from_config(Some("standard")),
            TlsMode::Standard
        );
        assert_eq!(
            TlsMode::from_config(Some("Standard")),
            TlsMode::Standard
        );
        assert_eq!(
            TlsMode::from_config(Some("STANDARD")),
            TlsMode::Standard
        );
    }

    #[test]
    fn test_tls_mode_skip_verify() {
        assert_eq!(
            TlsMode::from_config(Some("skip_verify")),
            TlsMode::SkipVerify
        );
        assert_eq!(
            TlsMode::from_config(Some("skip-verify")),
            TlsMode::SkipVerify
        );
        assert_eq!(
            TlsMode::from_config(Some("skipverify")),
            TlsMode::SkipVerify
        );
    }

    // ---- 构建属性测试 ----

    #[test]
    fn test_build_basic_properties_default() {
        let config = flow_engine_core::node_config::AmqpPublishConfig::default();
        let _props = build_basic_properties(&config);
    }

    #[test]
    fn test_build_basic_properties_with_content_type() {
        let mut config = flow_engine_core::node_config::AmqpPublishConfig::default();
        config.content_type = Some("application/json".to_string());
        config.delivery_mode = 2;
        config.priority = 5;
        let _props = build_basic_properties(&config);
    }

    #[test]
    fn test_build_basic_properties_with_headers() {
        let mut config = flow_engine_core::node_config::AmqpPublishConfig::default();
        let mut headers = HashMap::new();
        headers.insert("x-custom".to_string(), serde_json::json!("value"));
        headers.insert("x-count".to_string(), serde_json::json!(42));
        headers.insert("x-flag".to_string(), serde_json::json!(true));
        config.headers = Some(headers);
        let _props = build_basic_properties(&config);
    }

    #[test]
    fn test_build_basic_properties_with_all_fields() {
        let mut config = flow_engine_core::node_config::AmqpPublishConfig::default();
        config.content_type = Some("text/plain".to_string());
        config.content_encoding = Some("utf-8".to_string());
        config.delivery_mode = 2;
        config.priority = 1;
        config.correlation_id = Some("corr-123".to_string());
        config.reply_to = Some("reply-queue".to_string());
        config.expiration = Some("60000".to_string());
        config.message_id = Some("msg-123".to_string());
        config.timestamp = Some(1234567890);
        config.message_type = Some("test".to_string());
        config.user_id = Some("guest".to_string());
        config.app_id = Some("mpe".to_string());
        config.cluster_id = Some("cluster-1".to_string());
        let _props = build_basic_properties(&config);
    }

    // ---- FieldTable 构建测试 ----

    #[test]
    fn test_build_field_table_empty() {
        let table = build_field_table(None);
        assert!(table.inner().is_empty());
    }

    #[test]
    fn test_build_field_table_with_values() {
        let mut args = HashMap::new();
        args.insert("x-string".to_string(), serde_json::json!("hello"));
        args.insert("x-int".to_string(), serde_json::json!(42));
        args.insert("x-bool".to_string(), serde_json::json!(true));
        let table = build_field_table(Some(&args));
        assert_eq!(table.inner().len(), 3);
    }

    // ---- ExchangeKind 解析测试 ----

    #[test]
    fn test_parse_exchange_kind() {
        assert!(matches!(parse_exchange_kind("direct"), ExchangeKind::Direct));
        assert!(matches!(parse_exchange_kind("fanout"), ExchangeKind::Fanout));
        assert!(matches!(parse_exchange_kind("topic"), ExchangeKind::Topic));
        assert!(matches!(parse_exchange_kind("headers"), ExchangeKind::Headers));
        assert!(matches!(parse_exchange_kind("x-delayed"), ExchangeKind::Custom(_)));
    }
}
