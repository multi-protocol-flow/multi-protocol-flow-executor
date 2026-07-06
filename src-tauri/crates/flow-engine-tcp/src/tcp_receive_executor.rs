// TCP Receive 独立节点执行器
//
// 实现 TCP 数据独立接收功能，不依赖 tcp_send 的超时模式。
// 从已有的 tcp_connect 连接中接收数据，支持超时配置和帧定界。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::TcpProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::encoding::{build_binary_output, try_decode_text};

/// TCP Receive 独立节点执行器
pub struct TcpReceiveExecutor {
    tcp_manager: Arc<dyn TcpProvider>,
}

impl TcpReceiveExecutor {
    /// 创建新的 TCP Receive 执行器
    pub fn new(tcp_manager: Arc<dyn TcpProvider>) -> Self {
        Self { tcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for TcpReceiveExecutor {
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
            NodeData::TcpReceive(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not TcpReceiveConfig: {}",
                    node.uuid
                )))
            }
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

        // 3. 日志：start execution
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: tcp_receive, ConnectionID: {}, Timeout: {}ms",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            config.receive_timeout_ms
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "TCP Receive node {} started execution (connection: {}, timeout: {}ms)",
                    node.name, connection_id, config.receive_timeout_ms
                ),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 接收数据
        let receive_result = self
            .tcp_manager
            .receive(&connection_id, config.receive_timeout_ms)
            .await;

        let total_ms = start_time.elapsed().as_millis() as u64;

        match receive_result {
            Ok(Some(data)) => {
                log::info!(
                    "[TCP] Data received successfully: connection_id={}, bytes={}",
                    connection_id,
                    data.len()
                );

                // 5. 构建Success输出
                let received_output = build_binary_output(&data);

                let mut output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "received": received_output,
                    "timing": {
                        "receive_ms": total_ms,
                        "total_ms": total_ms,
                    },
                });

                // 添加 text_preview
                output_data["received_text_preview"] = try_decode_text(&data)
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null);

                // 6. 日志：完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "TCP Receive completed (received: {} bytes, total: {}ms)",
                            data.len(),
                            total_ms
                        ),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 返回 true 端口
                let next_node_ids = route_to_port(node, connections, "true");
                Ok((next_node_ids, Some(output_data)))
            }
            Ok(None) => {
                // 超时无数据，仍算Success但标记
                log::debug!(
                    "[TCP] Receive timeout, no data: connection_id={}",
                    connection_id
                );

                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "received": null,
                    "received_text_preview": "No data received (timeout)",
                    "timing": {
                        "receive_ms": total_ms,
                        "total_ms": total_ms,
                    },
                });

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("TCP Receive timeout, no data (total: {}ms)", total_ms),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let next_node_ids = route_to_port(node, connections, "true");
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                log::error!(
                    "[TCP] Receive data failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                let output_data = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "received": null,
                    "timing": {
                        "receive_ms": total_ms,
                        "total_ms": total_ms,
                    },
                    "error": e,
                });

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("TCP Receive failed: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // Failed走 false 端口
                route_to_false_port(node, connections, Some(output_data))
            }
        }
    }
}

/// 路由到指定端口
fn route_to_port(node: &Node, connections: &[Connection], port: &str) -> Vec<String> {
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
#[allow(clippy::result_large_err)]
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
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::TcpReceiveConfig;
    use flow_engine_core::traits::{FramingConfig, TcpFramingMode};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    // ========================================================================
    // 辅助函数
    // ========================================================================

    /// 创建Test Node
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        receive_timeout_ms: u64,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "tcp_receive".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::TcpReceive(TcpReceiveConfig {
                connection_id: connection_id.to_string(),
                receive_timeout_ms,
                framing_mode: TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
                framing_config: FramingConfig::default(),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext::new(
            flow_id.to_string(),
            flow_engine_core::utils::unix_timestamp_ms(),
        )
    }

    // ========================================================================
    // MockTcpProvider
    // ========================================================================

    /// Mock TCP 提供者
    struct MockTcpProvider {
        receive_result: std::sync::Mutex<Result<Option<Vec<u8>>, String>>,
        receive_count: AtomicUsize,
    }

    impl MockTcpProvider {
        fn new_with_data(data: Vec<u8>) -> Self {
            Self {
                receive_result: std::sync::Mutex::new(Ok(Some(data))),
                receive_count: AtomicUsize::new(0),
            }
        }

        fn new_with_timeout() -> Self {
            Self {
                receive_result: std::sync::Mutex::new(Ok(None)),
                receive_count: AtomicUsize::new(0),
            }
        }

        fn new_with_error(error: &str) -> Self {
            Self {
                receive_result: std::sync::Mutex::new(Err(error.to_string())),
                receive_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl TcpProvider for MockTcpProvider {
        async fn connect(
            &self,
            _uuid: String,
            _name: String,
            _host: String,
            _port: u16,
            _use_tls: bool,
            _tls_skip_verify: bool,
            _connect_timeout_ms: u64,
            _framing_config: FramingConfig,
        ) -> Result<(), String> {
            Err("MockTcpProvider::connect 不应被调用".to_string())
        }

        async fn send(
            &self,
            _uuid: &str,
            _data: Vec<u8>,
            _send_timeout_ms: u64,
        ) -> Result<(), String> {
            Err("MockTcpProvider::send 不应被调用".to_string())
        }

        async fn receive(
            &self,
            _uuid: &str,
            _receive_timeout_ms: u64,
        ) -> Result<Option<Vec<u8>>, String> {
            self.receive_count.fetch_add(1, AtomicOrdering::SeqCst);
            self.receive_result.lock().expect("锁不应Failed").clone()
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

    fn create_context_with_tcp<T: 'static + Send + Sync>(
        flow_id: &str,
        tcp_manager: Arc<T>,
    ) -> ExecutionContext {
        let mut ctx = create_context(flow_id);
        ctx.set_provider("tcp", tcp_manager);
        ctx
    }

    // ========================================================================
    // 测试：Success接收数据
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_success() {
        let mock = Arc::new(MockTcpProvider::new_with_data(b"Hello, TCP!".to_vec()));
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = create_test_node("node-1", "接收测试", "conn-1", 5000);
        let mut context = create_context_with_tcp("flow-1", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "conn-1");

        // 验证 received 结构
        let received = &data["received"];
        assert_eq!(received["encoding"], "base64");
        assert!(received["size"].as_u64().unwrap() > 0);

        // 验证 text_preview
        assert_eq!(data["received_text_preview"], "Hello, TCP!");

        // 验证 timing 结构
        let timing = &data["timing"];
        assert!(timing["receive_ms"].as_u64().is_some());
        assert!(timing["total_ms"].as_u64().is_some());

        // 验证 receive 被调用一次
        assert_eq!(mock.receive_count.load(AtomicOrdering::SeqCst), 1);
    }

    // ========================================================================
    // 测试：超时无数据
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_timeout() {
        let mock = Arc::new(MockTcpProvider::new_with_timeout());
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = create_test_node("node-2", "超时测试", "conn-2", 1000);
        let mut context = create_context_with_tcp("flow-2", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert!(data["received"].is_null());
        assert_eq!(data["received_text_preview"], "No data received (timeout)");
    }

    // ========================================================================
    // 测试：接收Failed
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_error() {
        let mock = Arc::new(MockTcpProvider::new_with_error("连接不存在"));
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = create_test_node("node-3", "Failed测试", "conn-3", 5000);
        let mut context = create_context_with_tcp("flow-3", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["error"]
            .as_str()
            .expect("should have error message")
            .contains("不存在"));
    }

    // ========================================================================
    // 测试：变量替换
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_variable_replacement() {
        let mock = Arc::new(MockTcpProvider::new_with_data(b"Data".to_vec()));
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-var".to_string(),
            node_type: "tcp_receive".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::TcpReceive(TcpReceiveConfig {
                connection_id: "{{conn_id}}".to_string(),
                receive_timeout_ms: 5000,
                framing_mode: TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
                framing_config: FramingConfig::default(),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context_with_tcp("flow-var", mock.clone());
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("tcp-connection-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应Success");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "tcp-connection-456");
    }

    // ========================================================================
    // 测试：端口路由
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_routes_to_true_port_with_connections() {
        let mock = Arc::new(MockTcpProvider::new_with_data(b"OK".to_vec()));
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = create_test_node("node-port-ok", "Success端口路由", "conn-1", 5000);
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
        let (next_nodes, _) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["next-node"]);
    }

    #[tokio::test]
    async fn test_tcp_receive_routes_to_false_port_with_connections() {
        let mock = Arc::new(MockTcpProvider::new_with_error("connection failed"));
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = create_test_node("node-port-fail", "Failed端口路由", "conn-1", 5000);
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
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["error-handler"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
    }

    // ========================================================================
    // 测试：Error节点数据类型
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_wrong_node_data_type() {
        let mock = Arc::new(MockTcpProvider::new_with_timeout());
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-wrong".to_string(),
            node_type: "http".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context_with_tcp("flow-wrong", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("TcpReceiveConfig"));
    }

    // ========================================================================
    // 测试：空数据接收
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_empty_data() {
        let mock = Arc::new(MockTcpProvider::new_with_data(Vec::new()));
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = create_test_node("node-empty", "空数据测试", "conn-e", 5000);
        let mut context = create_context_with_tcp("flow-empty", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["received"]["size"], 0);
    }

    // ========================================================================
    // 测试：大二进制数据接收
    // ========================================================================

    #[tokio::test]
    async fn test_tcp_receive_large_binary() {
        let large_data: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();
        let mock = Arc::new(MockTcpProvider::new_with_data(large_data.clone()));
        let executor = TcpReceiveExecutor::new(mock.clone());

        let node = create_test_node("node-large", "大数据测试", "conn-l", 10000);
        let mut context = create_context_with_tcp("flow-large", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应Success");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["received"]["size"], 65536);
        // 不可打印字节，text_preview 应为 null
        assert!(data["received_text_preview"].is_null());
    }
}
