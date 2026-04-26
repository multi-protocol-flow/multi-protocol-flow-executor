//! WebSocket 协议测试服务器
//!
//! 为 flow-engine 的 WebSocket 节点提供集成测试目标服务。
//! 支持回显、JSON 命令模式、广播、主题订阅/发布。

use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::accept_async;
use tracing::{error, info};
use tokio_tungstenite::tungstenite::Message;

// ============================================================================
// 类型定义
// ============================================================================

/// 发送给客户端的消息类型（文本或二进制）
#[derive(Debug)]
enum Outgoing {
    Text(String),
    Binary(Vec<u8>),
}

/// 客户端发送的 JSON 命令
#[derive(Debug, Deserialize)]
struct WsCommand {
    cmd: String,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    topic: Option<String>,
}

/// JSON 命令响应
#[derive(Debug, Serialize)]
struct WsResponse {
    #[serde(rename = "type")]
    resp_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
}

/// 主题订阅表（topic → 各客户端的发送端）
type TopicSubs = Arc<Mutex<HashMap<String, Vec<mpsc::UnboundedSender<Outgoing>>>>>;

/// 共享服务状态
struct ServerState {
    /// 广播通道，用于 broadcast 命令
    tx: broadcast::Sender<String>,
    /// 主题订阅表
    topic_subs: TopicSubs,
    /// 连接计数器
    conn_counter: Arc<AtomicU64>,
}

// ============================================================================
// 入口: 启动 WebSocket 测试服务器
// ============================================================================

/// 启动 WebSocket 测试服务器
///
/// 默认监听传入的 port 参数，可通过 `WS_PORT` 环境变量覆盖。
/// 阻塞运行直到 listener 出错或被外部取消。
pub async fn run(port: u16) -> Result<()> {
    let actual_port = env::var("WS_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(port);

    let listener = TcpListener::bind(("0.0.0.0", actual_port))
        .await
        .with_context(|| format!("WebSocket: 绑定端口 {} 失败", actual_port))?;
    info!("WebSocket 测试服务启动 → 0.0.0.0:{}", actual_port);

    let (broadcast_tx, _) = broadcast::channel::<String>(256);
    let topic_subs: TopicSubs = Arc::new(Mutex::new(HashMap::new()));
    let conn_counter = Arc::new(AtomicU64::new(0));

    let state = Arc::new(ServerState {
        tx: broadcast_tx,
        topic_subs,
        conn_counter,
    });

    loop {
        let (stream, addr) = listener.accept().await?;
        let conn_id = state.conn_counter.fetch_add(1, Ordering::Relaxed);
        info!(conn_id, remote = %addr, "WebSocket: 新连接 (等待升级)");

        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, conn_id, &state).await {
                error!(conn_id, remote = %addr, "WebSocket: 连接错误: {}", e);
            }
        });
    }
}

// ============================================================================
// 连接处理
// ============================================================================

/// 处理单个 WebSocket 连接
///
/// 架构: 三个并发任务
/// 1. 主循环 — 读取客户端消息，处理后发送到 out_tx
/// 2. writer_task — 从 out_rx 读取消息并写入 WebSocket
/// 3. broadcast_relay — 将广播通道消息转发到 out_tx
async fn handle_connection(
    stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    conn_id: u64,
    state: &Arc<ServerState>,
) -> Result<()> {
    let ws_stream = accept_async(stream)
        .await
        .with_context(|| format!("WebSocket: 升级失败, conn_id={}", conn_id))?;
    info!(conn_id, remote = %addr, "WebSocket: 握手完成");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // 发送欢迎消息
    let welcome = json!({
        "type": "welcome",
        "conn_id": conn_id,
        "message": "connected to ws test server"
    })
    .to_string();
    ws_sender.send(Message::Text(welcome.into())).await?;

    // 统一的输出通道：汇聚所有需要发给客户端的消息
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Outgoing>();

    // 订阅广播通道
    let mut broadcast_rx = state.tx.subscribe();

    // 写入任务：从 out_rx 读取消息并写入 WebSocket
    let writer_task = tokio::spawn(async move {
        while let Some(outgoing) = out_rx.recv().await {
            let msg = match outgoing {
                Outgoing::Text(text) => Message::Text(text.into()),
                Outgoing::Binary(data) => Message::Binary(data.into()),
            };
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 广播转发任务：将广播通道消息送入 out_tx
    let out_tx_for_relay = out_tx.clone();
    let broadcast_relay = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if out_tx_for_relay.send(Outgoing::Text(msg)).is_err() {
                break;
            }
        }
    });

    // 主消息循环：读取客户端消息并处理
    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                handle_text_message(&text, conn_id, addr, state, &out_tx).await;
            }
            Ok(Message::Binary(data)) => {
                info!(
                    conn_id,
                    remote = %addr,
                    bytes = data.len(),
                    "WebSocket: 回显二进制消息"
                );
                let _ = out_tx.send(Outgoing::Binary(data.to_vec()));
            }
            Ok(Message::Ping(_)) => {
                // tungstenite 自动回复 Pong
                info!(conn_id, remote = %addr, "WebSocket: 收到 Ping");
            }
            Ok(Message::Close(_)) => {
                info!(conn_id, remote = %addr, "WebSocket: 客户端关闭连接");
                break;
            }
            Ok(_) => {
                // 忽略 Pong 等其他消息类型
            }
            Err(e) => {
                error!(conn_id, remote = %addr, "WebSocket: 接收错误: {}", e);
                break;
            }
        }
    }

    // 清理：取消写入和广播任务
    writer_task.abort();
    broadcast_relay.abort();

    // 从所有主题中移除该客户端的发送端
    cleanup_topic_subs(&state.topic_subs, &out_tx).await;

    info!(conn_id, remote = %addr, "WebSocket: 连接已关闭");
    Ok(())
}

// ============================================================================
// 消息处理
// ============================================================================

/// 处理文本消息：尝试解析为 JSON 命令，失败则回显
async fn handle_text_message(
    text: &str,
    conn_id: u64,
    addr: std::net::SocketAddr,
    state: &Arc<ServerState>,
    out_tx: &mpsc::UnboundedSender<Outgoing>,
) {
    // 尝试解析为 JSON 命令
    if let Ok(cmd) = serde_json::from_str::<WsCommand>(text) {
        info!(conn_id, remote = %addr, cmd = %cmd.cmd, "WebSocket: 处理 JSON 命令");
        let response = process_command(&cmd, conn_id, state, out_tx).await;
        match serde_json::to_string(&response) {
            Ok(response_text) => {
                let _ = out_tx.send(Outgoing::Text(response_text));
            }
            Err(e) => {
                error!(conn_id, "WebSocket: 序列化响应失败: {}", e);
            }
        }
    } else {
        // 非 JSON 文本，回显
        info!(conn_id, remote = %addr, "WebSocket: 回显文本消息");
        let _ = out_tx.send(Outgoing::Text(text.to_string()));
    }
}

/// 处理 JSON 命令，返回响应
async fn process_command(
    cmd: &WsCommand,
    conn_id: u64,
    state: &Arc<ServerState>,
    out_tx: &mpsc::UnboundedSender<Outgoing>,
) -> WsResponse {
    match cmd.cmd.as_str() {
        // ---- 回显 ----
        "echo" => WsResponse {
            resp_type: "echo".to_string(),
            data: cmd.data.clone(),
            count: None,
            timestamp: None,
        },

        // ---- 广播给所有连接 ----
        "broadcast" => {
            let msg = json!({
                "type": "broadcast",
                "from_conn_id": conn_id,
                "data": cmd.data,
            })
            .to_string();
            let receiver_count = state.tx.send(msg).unwrap_or(0);
            info!(conn_id, receivers = receiver_count, "WebSocket: 广播消息");
            WsResponse {
                resp_type: "broadcast_sent".to_string(),
                data: Some(json!(receiver_count)),
                count: None,
                timestamp: None,
            }
        }

        // ---- 心跳 ----
        "ping" => WsResponse {
            resp_type: "pong".to_string(),
            data: None,
            count: None,
            timestamp: Some(now_timestamp()),
        },

        // ---- 文本变换 ----
        "upper" => {
            let upper = cmd
                .data
                .as_ref()
                .and_then(|v| v.as_str())
                .map(|s| s.to_uppercase());
            WsResponse {
                resp_type: "upper".to_string(),
                data: upper.map(Value::String),
                count: None,
                timestamp: None,
            }
        }

        "lower" => {
            let lower = cmd
                .data
                .as_ref()
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase());
            WsResponse {
                resp_type: "lower".to_string(),
                data: lower.map(Value::String),
                count: None,
                timestamp: None,
            }
        }

        "reverse" => {
            let reversed = cmd
                .data
                .as_ref()
                .and_then(|v| v.as_str())
                .map(|s| s.chars().rev().collect::<String>());
            WsResponse {
                resp_type: "reverse".to_string(),
                data: reversed.map(Value::String),
                count: None,
                timestamp: None,
            }
        }

        "count" => {
            let char_count = cmd
                .data
                .as_ref()
                .and_then(|v| v.as_str())
                .map(|s| s.len() as u64)
                .unwrap_or(0);
            WsResponse {
                resp_type: "count".to_string(),
                data: cmd.data.clone(),
                count: Some(char_count),
                timestamp: None,
            }
        }

        // ---- 连接信息 ----
        "client_count" => {
            let count = state.conn_counter.load(Ordering::Relaxed);
            WsResponse {
                resp_type: "client_count".to_string(),
                data: None,
                count: Some(count),
                timestamp: None,
            }
        }

        // ---- 主题订阅/发布 ----
        "subscribe" => {
            if let Some(topic) = &cmd.topic {
                let mut subs = state.topic_subs.lock().await;
                subs.entry(topic.clone())
                    .or_default()
                    .push(out_tx.clone());
                let subscriber_count = subs.get(topic).map(|v| v.len()).unwrap_or(0);
                info!(
                    conn_id,
                    topic,
                    subscribers = subscriber_count,
                    "WebSocket: 订阅主题"
                );
                WsResponse {
                    resp_type: "subscribed".to_string(),
                    data: Some(json!(topic)),
                    count: Some(subscriber_count as u64),
                    timestamp: None,
                }
            } else {
                WsResponse {
                    resp_type: "error".to_string(),
                    data: Some(json!("subscribe requires 'topic' field")),
                    count: None,
                    timestamp: None,
                }
            }
        }

        "publish" => {
            if let Some(topic) = &cmd.topic {
                let msg = json!({
                    "type": "topic_message",
                    "topic": topic,
                    "data": cmd.data,
                })
                .to_string();
                let subs = state.topic_subs.lock().await;
                let mut sent_count: u64 = 0;
                if let Some(senders) = subs.get(topic) {
                    for sender in senders {
                        if sender.send(Outgoing::Text(msg.clone())).is_ok() {
                            sent_count += 1;
                        }
                    }
                }
                info!(
                    conn_id,
                    topic,
                    recipients = sent_count,
                    "WebSocket: 发布主题消息"
                );
                WsResponse {
                    resp_type: "published".to_string(),
                    data: Some(json!({ "topic": topic, "recipients": sent_count })),
                    count: None,
                    timestamp: None,
                }
            } else {
                WsResponse {
                    resp_type: "error".to_string(),
                    data: Some(json!("publish requires 'topic' field")),
                    count: None,
                    timestamp: None,
                }
            }
        }

        // ---- 未知命令 ----
        _ => WsResponse {
            resp_type: "error".to_string(),
            data: Some(json!(format!("unknown command: {}", cmd.cmd))),
            count: None,
            timestamp: None,
        },
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 清理主题订阅表中已断开连接的客户端
async fn cleanup_topic_subs(
    topic_subs: &TopicSubs,
    closed_tx: &mpsc::UnboundedSender<Outgoing>,
) {
    let mut subs = topic_subs.lock().await;
    // 从所有主题中移除该客户端
    for (_topic, senders) in subs.iter_mut() {
        senders.retain(|sender| !sender.same_channel(closed_tx));
    }
    // 移除没有订阅者的空主题
    subs.retain(|_, senders| !senders.is_empty());
}

/// 获取当前时间的 Unix 时间戳字符串（含毫秒）
fn now_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}s", dur.as_secs(), dur.subsec_millis())
}
