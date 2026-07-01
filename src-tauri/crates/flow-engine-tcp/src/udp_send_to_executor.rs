// UDP SendTo 节点执行器
//
// 实现 UDP 数据发送、编码解码和统一输出构建功能。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::traits::UdpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::encoding::build_binary_output;

/// UDP `SendTo` 节点执行器
pub struct UdpSendToExecutor {
    udp_manager: Arc<dyn UdpProvider>,
}

impl UdpSendToExecutor {
    /// 创建新的 UDP `SendTo` 执行器
    pub fn new(udp_manager: Arc<dyn UdpProvider>) -> Self {
        Self { udp_manager }
    }

    /// 解码用户输入数据为原始字节
    fn decode_user_data(data: &str, encoding: &str) -> Result<Vec<u8>, String> {
        match encoding {
            "text" => Ok(data.as_bytes().to_vec()),
            "hex" => hex::decode(data).map_err(|e| format!("Hex decoding failed: {}", e)),
            "base64" => BASE64
                .decode(data)
                .map_err(|e| format!("Base64 decoding failed: {}", e)),
            other => Err(format!(
                "Unsupported data encoding: '{}' (supported: text/hex/base64)",
                other
            )),
        }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for UdpSendToExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::UdpSendTo(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not UdpSendToConfig: {}", node.uuid))),
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("Connection ID variable replacement failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 变量替换 data
        let data_str = replace_variables(&config.data, context).map_err(|e| {
            let error_msg = format!("Data variable replacement failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 4. 变量替换 target_addr
        let target_addr = replace_variables(&config.target_addr, context).map_err(|e| {
            let error_msg = format!("Target address variable replacement failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 5. 日志：开始执行
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: udp_send_to, ConnectionID: {}, Target: {}:{}, DataEncoding: {}, DataLength: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            target_addr,
            config.target_port,
            config.data_encoding,
            data_str.len()
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "UDP SendTo node {} started execution (connection: {}, target: {}:{}, encoding: {})",
                    node.name, connection_id, target_addr, config.target_port, config.data_encoding
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

        // 6. 解码用户数据
        let raw_data = match Self::decode_user_data(&data_str, &config.data_encoding) {
            Ok(data) => data,
            Err(e) => {
                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                let total_ms = start_time.elapsed().as_millis() as u64;
                let error_output = build_error_output(&connection_id, &e, None, total_ms);

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Data decoding failed: {}", e),
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

        // 7. 发送数据
        let send_start = Instant::now();
        let send_result = self
            .udp_manager
            .send_to(
                &connection_id,
                raw_data.clone(),
                target_addr.clone(),
                config.target_port,
                config.send_timeout_ms,
            )
            .await;
        let send_duration_ms = send_start.elapsed().as_millis() as u64;

        match send_result {
            Ok(()) => {
                log::info!(
                    "[UDP] Data sent successfully: connection_id={}, target={}:{}, bytes={}",
                    connection_id,
                    target_addr,
                    config.target_port,
                    raw_data.len()
                );

                // 8. 构建成功输出
                let total_ms = start_time.elapsed().as_millis() as u64;
                let sent_output = build_binary_output(&raw_data);

                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "sent": {
                        "data": sent_output["data"],
                        "encoding": sent_output["encoding"],
                        "size": sent_output["size"],
                        "target": format!("{}:{}", target_addr, config.target_port),
                    },
                    "timing": {
                        "send_ms": send_duration_ms,
                        "total_ms": total_ms,
                    },
                });

                // 9. 日志：完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "UDP SendTo completed (send: {}ms, total: {}ms)",
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
                    "[UDP] Data send failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                let total_ms = start_time.elapsed().as_millis() as u64;

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("UDP send failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                let error_output =
                    build_error_output(&connection_id, &e, Some(send_duration_ms), total_ms);

                route_to_false_port(node, connections, Some(error_output))
            }
        }
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
        "timing": {
            "send_ms": send_duration_ms,
            "total_ms": total_duration_ms,
        },
        "error": error_msg,
    })
}

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
) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
    let next_node_ids = route_to_port(node, connections, "false");
    Ok((next_node_ids, output_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::UdpSendToConfig;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    // ========================================================================
    // Mock UdpProvider
    // ========================================================================

    /// Mock UDP 提供者，记录调用并返回可配置结果
    struct MockUdpProvider {
        send_result: std::sync::Mutex<Result<(), String>>,
        send_count: AtomicUsize,
        last_sent_data: std::sync::Mutex<Vec<u8>>,
    }

    impl MockUdpProvider {
        fn new_send_ok() -> Self {
            Self {
                send_result: std::sync::Mutex::new(Ok(())),
                send_count: AtomicUsize::new(0),
                last_sent_data: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn new_send_error(error: &str) -> Self {
            Self {
                send_result: std::sync::Mutex::new(Err(error.to_string())),
                send_count: AtomicUsize::new(0),
                last_sent_data: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl UdpProvider for MockUdpProvider {
        async fn bind(
            &self,
            _uuid: String,
            _name: String,
            _local_addr: String,
            _local_port: u16,
            _read_timeout_ms: u64,
        ) -> Result<(), String> {
            Err("MockUdpProvider::bind 不应被调用".to_string())
        }

        async fn send_to(
            &self,
            _uuid: &str,
            data: Vec<u8>,
            _target_addr: String,
            _target_port: u16,
            _send_timeout_ms: u64,
        ) -> Result<(), String> {
            self.send_count.fetch_add(1, AtomicOrdering::SeqCst);
            *self.last_sent_data.lock().expect("锁不应失败") = data;
            self.send_result.lock().expect("锁不应失败").clone()
        }

        async fn recv_from(
            &self,
            _uuid: &str,
            _receive_timeout_ms: u64,
        ) -> Result<Option<(Vec<u8>, String, u16)>, String> {
            Ok(None)
        }

        async fn close(&self, _uuid: &str) -> Result<(), String> {
            Err("MockUdpProvider::close 不应被调用".to_string())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("Bound".to_string())
        }
    }

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
        target_addr: &str,
        target_port: u16,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "udp_send_to".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpSendTo(UdpSendToConfig {
                connection_id: connection_id.to_string(),
                data: data.to_string(),
                data_encoding: data_encoding.to_string(),
                target_addr: target_addr.to_string(),
                target_port,
                send_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext::new(flow_id.to_string(), 0)
    }

    fn create_context_with_udp<T: 'static + Send + Sync>(
        flow_id: &str,
        udp_manager: Arc<T>,
    ) -> ExecutionContext {
        let mut ctx = create_context(flow_id);
        ctx.set_provider("udp", udp_manager);
        ctx
    }

    // ========================================================================
    // 测试：文本发送
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_text_success() {
        let mock = Arc::new(MockUdpProvider::new_send_ok());
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = create_test_node(
            "node-1",
            "文本发送测试",
            "conn-1",
            "Hello, UDP!",
            "text",
            "127.0.0.1",
            9090,
        );

        let mut context = create_context_with_udp("flow-1", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "conn-1");
        assert_eq!(data["sent"]["target"], "127.0.0.1:9090");

        // 验证 timing 结构
        let timing = &data["timing"];
        assert!(timing["send_ms"].as_u64().is_some());
        assert!(timing["total_ms"].as_u64().is_some());

        // 验证实际发送的数据
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(&sent_data[..], b"Hello, UDP!");
    }

    // ========================================================================
    // 测试：Hex 发送
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_hex_success() {
        let mock = Arc::new(MockUdpProvider::new_send_ok());
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = create_test_node(
            "node-2",
            "Hex 发送测试",
            "conn-2",
            "48656c6c6f", // "Hello" 的 hex
            "hex",
            "192.168.1.1",
            8080,
        );

        let mut context = create_context_with_udp("flow-2", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);

        // 验证发送的数据是 "Hello" (5 字节)
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(&sent_data[..], b"Hello");
    }

    // ========================================================================
    // 测试：Base64 发送
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_base64_success() {
        let mock = Arc::new(MockUdpProvider::new_send_ok());
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = create_test_node(
            "node-3",
            "Base64 发送测试",
            "conn-3",
            "SGVsbG8gV29ybGQ=", // "Hello World" 的 base64
            "base64",
            "10.0.0.1",
            1234,
        );

        let mut context = create_context_with_udp("flow-3", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);

        // 验证发送的数据是 "Hello World" (11 字节)
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(&sent_data[..], b"Hello World");
    }

    // ========================================================================
    // 测试：发送失败
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_failure() {
        let mock = Arc::new(MockUdpProvider::new_send_error("UDP Socket 'conn-closed' 不存在"));
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = create_test_node(
            "node-5",
            "失败测试",
            "conn-closed",
            "Hello",
            "text",
            "127.0.0.1",
            9999,
        );

        let mut context = create_context_with_udp("flow-5", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().expect("should have error message").contains("not found"));
    }

    // ========================================================================
    // 测试：变量替换
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_variable_replacement() {
        let mock = Arc::new(MockUdpProvider::new_send_ok());
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-var".to_string(),
            node_type: "udp_send_to".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpSendTo(UdpSendToConfig {
                connection_id: "{{conn_id}}".to_string(),
                data: "Message: {{msg}}".to_string(),
                data_encoding: "text".to_string(),
                target_addr: "{{target}}".to_string(),
                target_port: 9090,
                send_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context_with_udp("flow-var", mock.clone());
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("udp-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "msg".to_string(),
            json!("Hello"),
            String::new(),
        );
        context.variables.set_variable(
            "target".to_string(),
            json!("10.0.0.1"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "udp-connection-123");
        assert_eq!(data["sent"]["target"], "10.0.0.1:9090");

        // 验证发送的数据包含替换后的内容
        let sent_data = mock.last_sent_data.lock().expect("锁不应失败");
        assert_eq!(&sent_data[..], b"Message: Hello");
    }

    // ========================================================================
    // 测试：无效编码
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_invalid_encoding() {
        let mock = Arc::new(MockUdpProvider::new_send_ok());
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-invalid-enc".to_string(),
            node_type: "udp_send_to".to_string(),
            name: "无效编码测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpSendTo(UdpSendToConfig {
                connection_id: "conn-1".to_string(),
                data: "test".to_string(),
                data_encoding: "unknown".to_string(),
                target_addr: "127.0.0.1".to_string(),
                target_port: 9090,
                send_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context_with_udp("flow-enc", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().expect("should have error").contains("Unsupported"));
    }

    // ========================================================================
    // 测试：端口路由
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_routes_to_true_port_with_connections() {
        let mock = Arc::new(MockUdpProvider::new_send_ok());
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = create_test_node(
            "node-port-ok",
            "成功端口路由",
            "conn-1",
            "Hello",
            "text",
            "127.0.0.1",
            9090,
        );

        let mut context = create_context_with_udp("flow-port", mock.clone());
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

    // ========================================================================
    // 测试：错误节点数据类型
    // ========================================================================

    #[tokio::test]
    async fn test_udp_send_to_wrong_node_data_type() {
        let mock = Arc::new(MockUdpProvider::new_send_ok());
        let executor = UdpSendToExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context_with_udp("flow-wrong", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("UdpSendToConfig"));
    }

    // ========================================================================
    // 测试：decode_user_data 单元测试
    // ========================================================================

    #[test]
    fn test_decode_user_data_text() {
        let result = UdpSendToExecutor::decode_user_data("Hello", "text").expect("应成功");
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_decode_user_data_hex() {
        let result =
            UdpSendToExecutor::decode_user_data("48656c6c6f", "hex").expect("应成功");
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_decode_user_data_base64() {
        let result =
            UdpSendToExecutor::decode_user_data("SGVsbG8=", "base64").expect("应成功");
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_decode_user_data_invalid_encoding() {
        let result = UdpSendToExecutor::decode_user_data("test", "json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported"));
    }

    #[test]
    fn test_decode_user_data_invalid_hex() {
        let result = UdpSendToExecutor::decode_user_data("ZZZZ", "hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_user_data_invalid_base64() {
        let result = UdpSendToExecutor::decode_user_data("!!!invalid!!!", "base64");
        assert!(result.is_err());
    }
}
