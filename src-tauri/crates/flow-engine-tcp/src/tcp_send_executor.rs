// TCP Send 节点执行器
//
// 实现 TCP 数据发送、可选响应接收、分帧编码和统一输出构建功能。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::TcpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::encoding::{build_binary_output, encode_to_base64, try_decode_text};
use crate::framing::TcpFramingEngine;

/// TCP Send 节点执行器
pub struct TcpSendExecutor {
    tcp_manager: Arc<dyn TcpProvider>,
}

impl TcpSendExecutor {
    /// 创建新的 TCP Send 执行器
    pub fn new(tcp_manager: Arc<dyn TcpProvider>) -> Self {
        Self { tcp_manager }
    }

    /// 解码用户输入数据为原始字节
    fn decode_user_data(data: &str, encoding: &str) -> Result<Vec<u8>, String> {
        match encoding {
            "text" => Ok(data.as_bytes().to_vec()),
            "hex" => hex::decode(data).map_err(|e| format!("Hex 解码失败: {}", e)),
            "base64" => BASE64
                .decode(data)
                .map_err(|e| format!("Base64 解码失败: {}", e)),
            other => Err(format!(
                "不支持的数据编码方式: '{}'（支持 text/hex/base64）",
                other
            )),
        }
    }

    /// 构建分帧描述字符串（用于输出 JSON）
    fn framing_description(
        framing_mode: &flow_engine_core::traits::TcpFramingMode,
    ) -> String {
        match framing_mode {
            flow_engine_core::traits::TcpFramingMode::LengthPrefix { prefix_bytes } => {
                format!("LengthPrefix({})", prefix_bytes)
            }
            flow_engine_core::traits::TcpFramingMode::Delimiter { delimiter } => {
                format!("Delimiter({:?})", delimiter)
            }
            flow_engine_core::traits::TcpFramingMode::FixedLength { length } => {
                format!("FixedLength({})", length)
            }
            flow_engine_core::traits::TcpFramingMode::Timeout { timeout_ms } => {
                format!("Timeout({}ms)", timeout_ms)
            }
            flow_engine_core::traits::TcpFramingMode::Manual => "Manual".to_string(),
        }
    }

    /// 构建错误输出数据
    fn build_error_output(
        connection_id: &str,
        error_msg: &str,
        send_duration_ms: Option<u64>,
        total_duration_ms: u64,
    ) -> serde_json::Value {
        json!({
            "success": false,
            "connection_id": connection_id,
            "sent": null,
            "received": null,
            "timing": {
                "send_ms": send_duration_ms,
                "receive_ms": null,
                "total_ms": total_duration_ms,
            },
            "error": error_msg,
        })
    }

    /// 构建 sent 字段
    fn build_sent_output(
        raw_data: &[u8],
        framed_data: &[u8],
        framing_desc: &str,
    ) -> serde_json::Value {
        json!({
            "data": encode_to_base64(framed_data),
            "encoding": "base64",
            "size": framed_data.len(),
            "framing": framing_desc,
            "raw_size": raw_data.len(),
        })
    }
}

#[async_trait::async_trait]
impl NodeExecutor for TcpSendExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::TcpSend(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 TcpSendConfig: {}", node.uuid)),
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 变量替换 data（仅对 text 编码有意义，hex/base64 可能包含 {{ 但通常不需要替换）
        let data_str = replace_variables(&config.data, context).map_err(|e| {
            let error_msg = format!("数据变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 4. 日志：开始执行
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: tcp_send, 连接ID: {}, 数据编码: {}, 数据长度: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            config.data_encoding,
            data_str.len()
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "TCP Send 节点 {} 开始执行 (连接: {}, 编码: {})",
                    node.name, connection_id, config.data_encoding
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 5. 解码用户数据
        let raw_data = match Self::decode_user_data(&data_str, &config.data_encoding) {
            Ok(data) => data,
            Err(e) => {
                let total_ms = start_time.elapsed().as_millis() as u64;
                let error_output =
                    Self::build_error_output(&connection_id, &e, None, total_ms);

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("数据解码失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 解码失败走 false 端口
                return route_to_false_port(node, connections, Some(error_output));
            }
        };

        // 6. 应用帧编码
        let framing_config = FramingConfig {
            mode: config.framing_mode.clone(),
            ..config.framing_config.clone()
        };

        let framed_data = match TcpFramingEngine::frame_data(&raw_data, &framing_config) {
            Ok(data) => data,
            Err(e) => {
                let total_ms = start_time.elapsed().as_millis() as u64;
                let error_output =
                    Self::build_error_output(&connection_id, &e, None, total_ms);

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("帧编码失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                return route_to_false_port(node, connections, Some(error_output));
            }
        };

        let framing_desc = Self::framing_description(&config.framing_mode);

        // 7. 发送数据
        let send_start = Instant::now();
        let send_result = self
            .tcp_manager
            .send(&connection_id, framed_data.clone(), config.send_timeout_ms)
            .await;
        let send_duration_ms = send_start.elapsed().as_millis() as u64;

        match send_result {
            Ok(()) => {
                log::info!(
                    "[TCP] 数据发送成功: connection_id={}, bytes={}",
                    connection_id,
                    framed_data.len()
                );

                // 8. 可选接收响应
                // 如果 framing_config.mode 不是 Manual 且有 receive 超时配置，则尝试接收
                // 这里简单地尝试一次 receive，如果超时则 received 为 null
                let receive_timeout_ms = match &config.framing_mode {
                    flow_engine_core::traits::TcpFramingMode::Timeout { timeout_ms } => {
                        *timeout_ms
                    }
                    _ => 0, // 不自动接收
                };

                let (received_output, receive_duration_ms) = if receive_timeout_ms > 0 {
                    let recv_start = Instant::now();
                    let recv_result = self
                        .tcp_manager
                        .receive(&connection_id, receive_timeout_ms)
                        .await;
                    let recv_ms = recv_start.elapsed().as_millis() as u64;

                    match recv_result {
                        Ok(Some(data)) => {
                            log::info!(
                                "[TCP] 接收响应成功: connection_id={}, bytes={}",
                                connection_id,
                                data.len()
                            );
                            (Some(build_binary_output(&data)), Some(recv_ms))
                        }
                        Ok(None) => {
                            // 超时无数据，非错误
                            log::debug!(
                                "[TCP] 接收超时无数据: connection_id={}",
                                connection_id
                            );
                            (None, Some(recv_ms))
                        }
                        Err(e) => {
                            log::warn!(
                                "[TCP] 接收响应失败（继续执行）: connection_id={}, error={}",
                                connection_id,
                                e
                            );
                            (None, None)
                        }
                    }
                } else {
                    (None, None)
                };

                // 9. 构建输出
                let total_ms = start_time.elapsed().as_millis() as u64;
                let sent_output = Self::build_sent_output(&raw_data, &framed_data, &framing_desc);

                let mut output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "sent": sent_output,
                    "received": received_output,
                    "timing": {
                        "send_ms": send_duration_ms,
                        "receive_ms": receive_duration_ms,
                        "total_ms": total_ms,
                    },
                });

                // 如果接收到数据，添加 text_preview
                if let Some(received) = &received_output {
                    if let Some(data_b64) = received.get("data").and_then(|d| d.as_str()) {
                        if let Ok(decoded) = crate::encoding::decode_from_base64(data_b64) {
                            output_data["received_text_preview"] =
                                try_decode_text(&decoded)
                                    .map(serde_json::Value::String)
                                    .unwrap_or(serde_json::Value::Null);
                        }
                    }
                }

                // 10. 日志：完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "TCP Send 完成 (发送: {}ms, 总计: {}ms)",
                            send_duration_ms, total_ms
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 返回 true 端口
                let next_node_ids = route_to_port(node, connections, "true");
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[TCP] 数据发送失败: connection_id={}, error={}",
                    connection_id,
                    e
                );

                let total_ms = start_time.elapsed().as_millis() as u64;

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("TCP 发送失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output =
                    Self::build_error_output(&connection_id, &e, Some(send_duration_ms), total_ms);

                route_to_false_port(node, connections, Some(error_output))
            }
        }
    }
}

use flow_engine_core::traits::FramingConfig;

/// 路由到指定端口
fn route_to_port(
    node: &Node,
    connections: &[Connection],
    port: &str,
) -> Vec<String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == port)
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    if filtered.is_empty() {
        vec![port.to_string()]
    } else {
        filtered
    }
}

/// 路由到 false 端口（辅助函数）
fn route_to_false_port(
    node: &Node,
    connections: &[Connection],
    output_data: Option<serde_json::Value>,
) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
    let next_node_ids = route_to_port(node, connections, "false");
    Ok((next_node_ids, output_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::TcpSendConfig;
    use flow_engine_core::traits::{FramingConfig, TcpFramingMode};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    // ========================================================================
    // 辅助函数
    // ========================================================================

    /// 创建测试节点
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        data: &str,
        data_encoding: &str,
        framing_mode: TcpFramingMode,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "tcp_send".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::TcpSend(TcpSendConfig {
                connection_id: connection_id.to_string(),
                data: data.to_string(),
                data_encoding: data_encoding.to_string(),
                framing_mode,
                framing_config: FramingConfig::default(),
                send_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            grpc_manager: None,
            providers: HashMap::new(),
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
        }
    }

    // ========================================================================
    // MockTcpProvider
    // ========================================================================

    /// Mock TCP 提供者，记录调用并返回可配置结果
    struct MockTcpProvider {
        send_result: std::sync::Mutex<Result<(), String>>,
        receive_result: std::sync::Mutex<Result<Option<Vec<u8>>, String>>,
        send_count: AtomicUsize,
        receive_count: AtomicUsize,
        last_sent_data: std::sync::Mutex<Vec<u8>>,
    }

    impl MockTcpProvider {
        fn new_send_ok() -> Self {
            Self {
                send_result: std::sync::Mutex::new(Ok(())),
                receive_result: std::sync::Mutex::new(Ok(None)),
                send_count: AtomicUsize::new(0),
                receive_count: AtomicUsize::new(0),
                last_sent_data: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn new_send_error(error: &str) -> Self {
            Self {
                send_result: std::sync::Mutex::new(Err(error.to_string())),
                receive_result: std::sync::Mutex::new(Ok(None)),
                send_count: AtomicUsize::new(0),
                receive_count: AtomicUsize::new(0),
                last_sent_data: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn with_receive_data(data: Vec<u8>) -> Self {
            Self {
                send_result: std::sync::Mutex::new(Ok(())),
                receive_result: std::sync::Mutex::new(Ok(Some(data))),
                send_count: AtomicUsize::new(0),
                receive_count: AtomicUsize::new(0),
                last_sent_data: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl TcpProvider for MockTcpProvider {
        async fn connect(
            &self, _uuid: String, _name: String, _host: String, _port: u16,
            _use_tls: bool, _tls_skip_verify: bool, _connect_timeout_ms: u64,
            _framing_config: FramingConfig,
        ) -> Result<(), String> {
            Err("MockTcpProvider::connect 不应被调用".to_string())
        }

        async fn send(
            &self, _uuid: &str, data: Vec<u8>, _send_timeout_ms: u64,
        ) -> Result<(), String> {
            self.send_count.fetch_add(1, AtomicOrdering::SeqCst);
            *self.last_sent_data.lock().expect("锁不应失败") = data;
            self.send_result.lock().expect("锁不应失败").clone()
        }

        async fn receive(
            &self, _uuid: &str, _receive_timeout_ms: u64,
        ) -> Result<Option<Vec<u8>>, String> {
            self.receive_count.fetch_add(1, AtomicOrdering::SeqCst);
            self.receive_result.lock().expect("锁不应失败").clone()
        }

        async fn close(&self, _uuid: &str, _close_timeout_ms: u64) -> Result<(), String> {
            Err("MockTcpProvider::close 不应被调用".to_string())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("Connected".to_string())
        }

        fn list_connections(&self) -> Vec<flow_engine_core::traits::TcpConnectionInfo> {
            Vec::new()
        }
    }

    fn create_context_with_tcp(flow_id: &str, tcp_manager: Arc<dyn TcpProvider>) -> ExecutionContext {
        let mut ctx = create_context(flow_id);
        ctx.tcp_manager = Some(tcp_manager);
        ctx
    }

    // ========================================================================
    // 测试：文本 + 长度前缀
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_text_length_prefix() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-1",
            "文本发送测试",
            "conn-1",
            "Hello, TCP!",
            "text",
            TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
        );

        let mut context = create_context_with_tcp("flow-1", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "conn-1");

        // 验证 sent 结构
        let sent = &data["sent"];
        assert_eq!(sent["encoding"], "base64");
        assert_eq!(sent["framing"], "LengthPrefix(4)");
        assert!(sent["size"].as_u64().unwrap() > 0);
        assert!(sent["raw_size"].as_u64().unwrap() > 0);

        // 验证 timing 结构
        let timing = &data["timing"];
        assert!(timing["send_ms"].as_u64().is_some());
        assert!(timing["total_ms"].as_u64().is_some());

        // 验证实际发送的数据包含 4 字节前缀 + "Hello, TCP!" (11 字节) = 15 字节
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(sent_data.len(), 15); // 4 + 11
        assert_eq!(&sent_data[4..], b"Hello, TCP!");
    }

    // ========================================================================
    // 测试：Hex + 分隔符
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_hex_delimiter() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-2",
            "Hex 发送测试",
            "conn-2",
            "48656c6c6f", // "Hello" 的 hex
            "hex",
            TcpFramingMode::Delimiter {
                delimiter: "\r\n".to_string(),
            },
        );

        let mut context = create_context_with_tcp("flow-2", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);

        // 验证 framing 描述包含 Delimiter
        let framing = data["sent"]["framing"].as_str().expect("framing 应为字符串");
        assert!(framing.starts_with("Delimiter("));

        // 验证发送的数据是 "Hello\r\n" (5 + 2 = 7 字节)
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(sent_data.len(), 7);
        assert_eq!(&sent_data[..5], b"Hello");
        assert_eq!(&sent_data[5..], b"\r\n");
    }

    // ========================================================================
    // 测试：Base64 + Manual
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_base64_manual() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-3",
            "Base64 发送测试",
            "conn-3",
            "SGVsbG8gV29ybGQ=", // "Hello World" 的 base64
            "base64",
            TcpFramingMode::Manual,
        );

        let mut context = create_context_with_tcp("flow-3", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["sent"]["framing"], "Manual");

        // Manual 模式不添加帧，直接发送 "Hello World" (11 字节)
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(sent_data.len(), 11);
        assert_eq!(&sent_data[..], b"Hello World");
    }

    // ========================================================================
    // 测试：空载荷
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_empty_payload() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-4",
            "空载荷测试",
            "conn-4",
            "",
            "text",
            TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
        );

        let mut context = create_context_with_tcp("flow-4", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        // 空数据 + 4 字节前缀 = 4 字节
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(sent_data.len(), 4); // 只有长度前缀 (0)
    }

    // ========================================================================
    // 测试：连接不存在/发送失败
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_closed_connection() {
        let mock = Arc::new(MockTcpProvider::new_send_error("TCP 连接 'conn-closed' 不存在"));
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-5",
            "关闭连接测试",
            "conn-closed",
            "Hello",
            "text",
            TcpFramingMode::Manual,
        );

        let mut context = create_context_with_tcp("flow-5", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().expect("应有错误信息").contains("不存在"));
    }

    // ========================================================================
    // 测试：变量替换
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_variable_replacement() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-var".to_string(),
            node_type: "tcp_send".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::TcpSend(TcpSendConfig {
                connection_id: "{{conn_id}}".to_string(),
                data: "Message: {{msg}}".to_string(),
                data_encoding: "text".to_string(),
                framing_mode: TcpFramingMode::Manual,
                framing_config: FramingConfig::default(),
                send_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context_with_tcp("flow-var", mock.clone());
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("tcp-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "msg".to_string(),
            json!("Hello"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "tcp-connection-123");

        // 验证发送的数据包含替换后的内容
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(&sent_data[..], b"Message: Hello");
    }

    // ========================================================================
    // 测试：无效编码
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_invalid_encoding() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-invalid-enc".to_string(),
            node_type: "tcp_send".to_string(),
            name: "无效编码测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::TcpSend(TcpSendConfig {
                connection_id: "conn-1".to_string(),
                data: "test".to_string(),
                data_encoding: "unknown".to_string(),
                framing_mode: TcpFramingMode::Manual,
                framing_config: FramingConfig::default(),
                send_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context_with_tcp("flow-enc", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().expect("应有错误").contains("不支持"));
    }

    // ========================================================================
    // 测试：无效 hex 数据
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_invalid_hex() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-bad-hex",
            "无效Hex测试",
            "conn-1",
            "ZZZZ", // 无效 hex
            "hex",
            TcpFramingMode::Manual,
        );

        let mut context = create_context_with_tcp("flow-hex", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().expect("应有错误").contains("Hex"));
    }

    // ========================================================================
    // 测试：端口路由
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_routes_to_true_port_with_connections() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-port-ok",
            "成功端口路由",
            "conn-1",
            "Hello",
            "text",
            TcpFramingMode::Manual,
        );

        let mut context = create_context_with_tcp("flow-port", mock.clone());
        let connections = vec![Connection {
            id: "c1".to_string(),
            source_node_uuid: "node-port-ok".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["next-node"]);
    }

    #[tokio::test]
    async fn test_tcp_send_routes_to_false_port_with_connections() {
        let mock = Arc::new(MockTcpProvider::new_send_error("发送失败"));
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-port-fail",
            "失败端口路由",
            "conn-1",
            "Hello",
            "text",
            TcpFramingMode::Manual,
        );

        let mut context = create_context_with_tcp("flow-port", mock.clone());
        let connections = vec![Connection {
            id: "c2".to_string(),
            source_node_uuid: "node-port-fail".to_string(),
            target_node_uuid: "error-handler".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["error-handler"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
    }

    // ========================================================================
    // 测试：错误节点数据类型
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_wrong_node_data_type() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context_with_tcp("flow-wrong", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TcpSendConfig"));
    }

    // ========================================================================
    // 测试：Timeout 模式（自动接收）
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_timeout_mode_with_receive() {
        let mock = Arc::new(MockTcpProvider::with_receive_data(b"Response data".to_vec()));
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-timeout",
            "超时模式测试",
            "conn-t",
            "Ping",
            "text",
            TcpFramingMode::Timeout { timeout_ms: 1000 },
        );

        let mut context = create_context_with_tcp("flow-timeout", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        // 验证接收到响应数据
        assert!(data["received"].is_object());
        assert!(data["received"]["size"].as_u64().unwrap() > 0);
        assert!(data["timing"]["receive_ms"].as_u64().is_some());

        // 验证 receive 被调用
        assert_eq!(mock.receive_count.load(AtomicOrdering::SeqCst), 1);
    }

    // ========================================================================
    // 测试：decode_user_data 单元测试
    // ========================================================================

    #[test]
    fn test_decode_user_data_text() {
        let result = TcpSendExecutor::decode_user_data("Hello", "text").expect("应成功");
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_decode_user_data_hex() {
        let result = TcpSendExecutor::decode_user_data("48656c6c6f", "hex").expect("应成功");
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_decode_user_data_base64() {
        let result = TcpSendExecutor::decode_user_data("SGVsbG8=", "base64").expect("应成功");
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_decode_user_data_invalid_encoding() {
        let result = TcpSendExecutor::decode_user_data("test", "json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持"));
    }

    #[test]
    fn test_decode_user_data_invalid_hex() {
        let result = TcpSendExecutor::decode_user_data("ZZZZ", "hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_user_data_invalid_base64() {
        let result = TcpSendExecutor::decode_user_data("!!!invalid!!!", "base64");
        assert!(result.is_err());
    }

    // ========================================================================
    // 测试：framing_description 单元测试
    // ========================================================================

    #[test]
    fn test_framing_description_length_prefix() {
        let desc = TcpSendExecutor::framing_description(&TcpFramingMode::LengthPrefix { prefix_bytes: 4 });
        assert_eq!(desc, "LengthPrefix(4)");
    }

    #[test]
    fn test_framing_description_delimiter() {
        let desc = TcpSendExecutor::framing_description(&TcpFramingMode::Delimiter {
            delimiter: "\r\n".to_string(),
        });
        assert!(desc.contains("Delimiter"));
    }

    #[test]
    fn test_framing_description_fixed_length() {
        let desc = TcpSendExecutor::framing_description(&TcpFramingMode::FixedLength { length: 128 });
        assert_eq!(desc, "FixedLength(128)");
    }

    #[test]
    fn test_framing_description_timeout() {
        let desc = TcpSendExecutor::framing_description(&TcpFramingMode::Timeout { timeout_ms: 5000 });
        assert_eq!(desc, "Timeout(5000ms)");
    }

    #[test]
    fn test_framing_description_manual() {
        let desc = TcpSendExecutor::framing_description(&TcpFramingMode::Manual);
        assert_eq!(desc, "Manual");
    }

    // ========================================================================
    // 测试：FixedLength 截断
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_send_fixed_length_exact() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        let node = create_test_node(
            "node-fixed",
            "固定长度测试",
            "conn-f",
            "12345678",
            "text",
            TcpFramingMode::FixedLength { length: 8 },
        );

        let mut context = create_context_with_tcp("flow-fixed", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);

        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(sent_data.len(), 8);
        assert_eq!(&sent_data[..], b"12345678");
    }

    #[tokio::test]
    async fn test_tcp_send_fixed_length_too_short() {
        let mock = Arc::new(MockTcpProvider::new_send_ok());
        let executor = TcpSendExecutor::new(mock.clone());

        // 数据只有 3 字节，但固定长度要求 10
        let node = create_test_node(
            "node-fixed-short",
            "固定长度不足测试",
            "conn-f",
            "abc",
            "text",
            TcpFramingMode::FixedLength { length: 10 },
        );

        let mut context = create_context_with_tcp("flow-fixed", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
    }
}
