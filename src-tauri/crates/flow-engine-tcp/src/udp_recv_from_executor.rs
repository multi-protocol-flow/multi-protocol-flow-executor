// UDP RecvFrom 节点执行器
//
// 实现 UDP 数据接收和统一输出构建功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::UdpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::encoding::build_binary_output;

/// UDP `RecvFrom` 节点执行器
pub struct UdpRecvFromExecutor {
    udp_manager: Arc<dyn UdpProvider>,
}

impl UdpRecvFromExecutor {
    /// 创建新的 UDP `RecvFrom` 执行器
    pub fn new(udp_manager: Arc<dyn UdpProvider>) -> Self {
        Self { udp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for UdpRecvFromExecutor {
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
            NodeData::UdpRecvFrom(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 UdpRecvFromConfig: {}",
                    node.uuid
                ))
            }
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

        // 3. 日志：开始接收
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: udp_recv_from, 连接ID: {}, 超时: {}ms",
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
                    "UDP RecvFrom 节点 {} 开始执行 (连接: {}, 超时: {}ms)",
                    node.name, connection_id, config.receive_timeout_ms
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

        // 4. 接收数据（计时）
        let recv_start = Instant::now();
        let recv_result = self
            .udp_manager
            .recv_from(&connection_id, config.receive_timeout_ms)
            .await;
        let recv_duration_ms = recv_start.elapsed().as_millis() as u64;

        match recv_result {
            Ok(Some((data, source_addr, source_port))) => {
                log::info!(
                    "[UDP] 接收成功: connection_id={}, from={}:{}, bytes={}",
                    connection_id,
                    source_addr,
                    source_port,
                    data.len()
                );

                // 5. 构建成功输出
                let total_ms = start_time.elapsed().as_millis() as u64;
                let received_output = build_binary_output(&data);

                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "received": {
                        "data": received_output["data"],
                        "encoding": received_output["encoding"],
                        "size": received_output["size"],
                        "source_addr": source_addr,
                        "source_port": source_port,
                    },
                    "timing": {
                        "recv_ms": recv_duration_ms,
                        "total_ms": total_ms,
                    },
                });

                // 6. 日志：完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "UDP RecvFrom 完成 (接收: {} 字节, {}ms)",
                            data.len(),
                            total_ms
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
            Ok(None) => {
                // 超时无数据，走 false 端口
                log::debug!(
                    "[UDP] 接收超时: connection_id={}, timeout={}ms",
                    connection_id,
                    config.receive_timeout_ms
                );

                let total_ms = start_time.elapsed().as_millis() as u64;

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Warn,
                        message: format!(
                            "UDP 接收超时 ({}ms)",
                            config.receive_timeout_ms
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

                let output_data = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "received": null,
                    "timing": {
                        "recv_ms": recv_duration_ms,
                        "total_ms": total_ms,
                    },
                    "error": format!("接收超时 ({}ms)", config.receive_timeout_ms),
                });

                let next_node_ids = route_to_port(node, connections, "false");
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[UDP] 接收失败: connection_id={}, error={}",
                    connection_id,
                    e
                );

                let total_ms = start_time.elapsed().as_millis() as u64;

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("UDP 接收失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "received": null,
                    "timing": {
                        "recv_ms": recv_duration_ms,
                        "total_ms": total_ms,
                    },
                    "error": e,
                });

                let next_node_ids = route_to_port(node, connections, "false");
                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::UdpRecvFromConfig;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    // ========================================================================
    // Mock UdpProvider
    // ========================================================================

    /// Mock UDP 提供者，记录调用并返回可配置结果
    struct MockUdpProvider {
        recv_result: std::sync::Mutex<Result<Option<(Vec<u8>, String, u16)>, String>>,
        recv_count: AtomicUsize,
    }

    impl MockUdpProvider {
        fn new_recv_with_data(data: Vec<u8>, source_addr: &str, source_port: u16) -> Self {
            Self {
                recv_result: std::sync::Mutex::new(Ok(Some((
                    data,
                    source_addr.to_string(),
                    source_port,
                )))),
                recv_count: AtomicUsize::new(0),
            }
        }

        fn new_recv_timeout() -> Self {
            Self {
                recv_result: std::sync::Mutex::new(Ok(None)),
                recv_count: AtomicUsize::new(0),
            }
        }

        fn new_recv_error(error: &str) -> Self {
            Self {
                recv_result: std::sync::Mutex::new(Err(error.to_string())),
                recv_count: AtomicUsize::new(0),
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
            _data: Vec<u8>,
            _target_addr: String,
            _target_port: u16,
            _send_timeout_ms: u64,
        ) -> Result<(), String> {
            Err("MockUdpProvider::send_to 不应被调用".to_string())
        }

        async fn recv_from(
            &self,
            _uuid: &str,
            _receive_timeout_ms: u64,
        ) -> Result<Option<(Vec<u8>, String, u16)>, String> {
            self.recv_count.fetch_add(1, AtomicOrdering::SeqCst);
            self.recv_result.lock().expect("锁不应失败").clone()
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
        receive_timeout_ms: u64,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "udp_recv_from".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpRecvFrom(UdpRecvFromConfig {
                connection_id: connection_id.to_string(),
                receive_timeout_ms,
                max_size: 65536,
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
    // 测试：接收成功
    // ========================================================================

    #[tokio::test]
    async fn test_udp_recv_from_success() {
        let mock = Arc::new(MockUdpProvider::new_recv_with_data(
            b"Hello, UDP!".to_vec(),
            "192.168.1.100",
            12345,
        ));
        let executor = UdpRecvFromExecutor::new(mock.clone());

        let node = create_test_node("node-1", "接收测试", "conn-1", 5000);
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

        // 验证 received 结构
        let received = &data["received"];
        assert_eq!(received["encoding"], "base64");
        assert_eq!(received["size"], 11);
        assert_eq!(received["source_addr"], "192.168.1.100");
        assert_eq!(received["source_port"], 12345);

        // 验证 timing 结构
        let timing = &data["timing"];
        assert!(timing["recv_ms"].as_u64().is_some());
        assert!(timing["total_ms"].as_u64().is_some());

        // 验证 recv 被调用
        assert_eq!(mock.recv_count.load(AtomicOrdering::SeqCst), 1);
    }

    // ========================================================================
    // 测试：接收超时
    // ========================================================================

    #[tokio::test]
    async fn test_udp_recv_from_timeout() {
        let mock = Arc::new(MockUdpProvider::new_recv_timeout());
        let executor = UdpRecvFromExecutor::new(mock.clone());

        let node = create_test_node("node-2", "超时测试", "conn-2", 1000);
        let mut context = create_context_with_udp("flow-2", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["received"].is_null());
        assert!(data["error"].as_str().expect("应有错误").contains("超时"));
    }

    // ========================================================================
    // 测试：接收失败
    // ========================================================================

    #[tokio::test]
    async fn test_udp_recv_from_error() {
        let mock = Arc::new(MockUdpProvider::new_recv_error("UDP Socket 'conn-err' 不存在"));
        let executor = UdpRecvFromExecutor::new(mock.clone());

        let node = create_test_node("node-3", "失败测试", "conn-err", 5000);
        let mut context = create_context_with_udp("flow-3", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().expect("应有错误").contains("不存在"));
    }

    // ========================================================================
    // 测试：变量替换
    // ========================================================================

    #[tokio::test]
    async fn test_udp_recv_from_variable_replacement() {
        let mock = Arc::new(MockUdpProvider::new_recv_with_data(
            b"data".to_vec(),
            "10.0.0.1",
            8080,
        ));
        let executor = UdpRecvFromExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-var".to_string(),
            node_type: "udp_recv_from".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpRecvFrom(UdpRecvFromConfig {
                connection_id: "{{conn_id}}".to_string(),
                receive_timeout_ms: 3000,
                max_size: 65536,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context_with_udp("flow-var", mock.clone());
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("udp-connection-789"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "udp-connection-789");
    }

    // ========================================================================
    // 测试：端口路由
    // ========================================================================

    #[tokio::test]
    async fn test_udp_recv_from_routes_with_connections() {
        let mock = Arc::new(MockUdpProvider::new_recv_with_data(
            b"response".to_vec(),
            "127.0.0.1",
            9999,
        ));
        let executor = UdpRecvFromExecutor::new(mock.clone());

        let node = create_test_node("node-port", "端口路由测试", "conn-1", 5000);
        let mut context = create_context_with_udp("flow-port", mock.clone());
        let connections = vec![Connection {
            id: "c1".to_string(),
            source_node_uuid: "node-port".to_string(),
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
    async fn test_udp_recv_from_wrong_node_data_type() {
        let mock = Arc::new(MockUdpProvider::new_recv_timeout());
        let executor = UdpRecvFromExecutor::new(mock.clone());

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
        assert!(result.unwrap_err().contains("UdpRecvFromConfig"));
    }
}
