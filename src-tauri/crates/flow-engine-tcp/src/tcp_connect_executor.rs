// TCP 连接节点执行器
//
// 实现 TCP 连接建立和变量替换功能。

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

/// TCP 连接节点执行器
pub struct TcpConnectExecutor {
    tcp_manager: Arc<dyn TcpProvider>,
}

impl TcpConnectExecutor {
    /// 创建新的 TCP 连接执行器
    pub fn new(tcp_manager: Arc<dyn TcpProvider>) -> Self {
        Self { tcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for TcpConnectExecutor {
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
            NodeData::TcpConnect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not TcpConnectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换 host
        let host = replace_variables(&config.host, context).map_err(|e| {
            let error_msg = format!("Host variable replacement failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始连接
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: tcp_connect, Target: {}:{}",
            context.flow_uuid,
            node.uuid,
            node.name,
            host,
            config.port
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("TCP Connect node {} started execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 建立连接（计时）
        // 注意：tls_ca_cert 配置已从 config 中获取，但 TcpProvider trait 的 connect 方法
        // 不支持传递自定义 CA 证书。使用 plugin.rs 中的 connect_with_options 方法可支持
        // 自定义 CA 证书。此处保留 tls_ca_cert 引用以便将来扩展。
        let _tls_ca_cert = config.tls_ca_cert.as_ref();
        let connect_start = Instant::now();
        let connect_result = self
            .tcp_manager
            .connect(
                node.uuid.clone(),
                node.name.clone(),
                host.clone(),
                config.port,
                config.use_tls,
                config.tls_skip_verify,
                config.connect_timeout_ms,
                config.framing_config.clone(),
            )
            .await;
        let connect_duration_ms = connect_start.elapsed().as_millis() as u64;

        match connect_result {
            Ok(()) => {
                log::info!(
                    "[TCP] Connection successful: uuid={}, host={}, port={}",
                    node.uuid,
                    host,
                    config.port
                );

                // 5. 构建Success输出数据
                let total_ms = start_time.elapsed().as_millis() as u64;
                let output_data = json!({
                    "success": true,
                    "connection_id": node.uuid,
                    "host": host,
                    "port": config.port,
                    "use_tls": config.use_tls,
                    "timing": {
                        "connect_ms": connect_duration_ms,
                        "total_ms": total_ms,
                    },
                });

                // 6. 日志：连接完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "TCP connection successful {}:{} ({}ms)",
                            host, config.port, total_ms
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 7. 返回 true 端口（连接Success）
                let next_node_ids = resolve_port_nodes(connections, &node.uuid, "true");

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[TCP] Connection failed: uuid={}, host={}, port={}, error={}",
                    node.uuid,
                    host,
                    config.port,
                    e
                );

                // 日志：connection failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("TCP connection failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据
                let total_ms = start_time.elapsed().as_millis() as u64;
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "host": host,
                    "port": config.port,
                    "use_tls": config.use_tls,
                    "timing": {
                        "connect_ms": connect_duration_ms,
                        "total_ms": total_ms,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                // 返回 false 端口
                let next_node_ids = resolve_port_nodes(connections, &node.uuid, "false");

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

/// 解析指定端口的下游节点 ID 列表
///
/// 如果没有匹配的连接，返回端口名称本身（用于测试）。
fn resolve_port_nodes(connections: &[Connection], node_uuid: &str, port_id: &str) -> Vec<String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node_uuid && conn.source_port_id == port_id)
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    if filtered.is_empty() {
        vec![port_id.to_string()]
    } else {
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::TcpConnectConfig;
    use flow_engine_core::traits::FramingConfig;
    use std::sync::atomic::{AtomicBool, Ordering};

    // -----------------------------------------------------------------------
    // Mock TcpProvider
    // -----------------------------------------------------------------------

    /// Mock TCP 提供者（可配置连接Success/Failed）
    struct MockTcpProvider {
        should_succeed: AtomicBool,
    }

    impl MockTcpProvider {
        fn new(should_succeed: bool) -> Self {
            Self {
                should_succeed: AtomicBool::new(should_succeed),
            }
        }
    }

    #[async_trait::async_trait]
    impl TcpProvider for MockTcpProvider {
        async fn connect(
            &self,
            uuid: String,
            _name: String,
            host: String,
            port: u16,
            _use_tls: bool,
            _tls_skip_verify: bool,
            _connect_timeout_ms: u64,
            _framing_config: FramingConfig,
        ) -> Result<(), String> {
            if self.should_succeed.load(Ordering::Relaxed) {
                log::info!("[MockTcp] 连接Success: uuid={}, {}:{}", uuid, host, port);
                Ok(())
            } else {
                Err(format!("MockTcp connection failed: {}:{}", host, port))
            }
        }

        async fn send(
            &self,
            _uuid: &str,
            _data: Vec<u8>,
            _send_timeout_ms: u64,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn receive(
            &self,
            _uuid: &str,
            _receive_timeout_ms: u64,
        ) -> Result<Option<Vec<u8>>, String> {
            Ok(None)
        }

        async fn close(&self, _uuid: &str, _close_timeout_ms: u64) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            if self.should_succeed.load(Ordering::Relaxed) {
                Some("Connected".to_string())
            } else {
                Some("Error".to_string())
            }
        }

        fn list_connections(&self) -> Vec<flow_engine_core::traits::TcpConnectionInfo> {
            Vec::new()
        }
    }

    // -----------------------------------------------------------------------
    // 辅助函数
    // -----------------------------------------------------------------------

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, host: &str, port: u16) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "tcp_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::TcpConnect(TcpConnectConfig {
                host: host.to_string(),
                port,
                use_tls: false,
                tls_skip_verify: false,
                connect_timeout_ms: 30000,
                no_delay: false,
                keepalive: None,
                tls_ca_cert: None,
                framing_mode: flow_engine_core::traits::TcpFramingMode::LengthPrefix {
                    prefix_bytes: 4,
                },
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
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        )
    }

    // -----------------------------------------------------------------------
    // 执行器测试
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_tcp_connect_executor_success() {
        let mock_provider = Arc::new(MockTcpProvider::new(true));
        let executor = TcpConnectExecutor::new(mock_provider);
        let node = create_test_node("test-1", "测试连接", "127.0.0.1", 8080);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "test-1");
        assert_eq!(data["host"], "127.0.0.1");
        assert_eq!(data["port"], 8080);
        assert_eq!(data["use_tls"], false);
        assert!(data.get("timing").is_some());
        assert!(data["timing"]["total_ms"].is_number());
    }

    #[tokio::test]
    async fn test_tcp_connect_executor_failure() {
        let mock_provider = Arc::new(MockTcpProvider::new(false));
        let executor = TcpConnectExecutor::new(mock_provider);
        let node = create_test_node("test-2", "Failed连接", "192.168.1.1", 9999);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["connection_id"].is_null());
        assert_eq!(data["host"], "192.168.1.1");
        assert_eq!(data["port"], 9999);
        assert!(data.get("error").is_some());
        let error = data["error"].as_str().expect("error should be a string");
        assert!(error.find("connection failed").is_some());
    }

    #[tokio::test]
    async fn test_tcp_connect_wrong_node_data_type() {
        let mock_provider = Arc::new(MockTcpProvider::new(true));
        let executor = TcpConnectExecutor::new(mock_provider);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("TcpConnectConfig"));
    }

    #[tokio::test]
    async fn test_tcp_connect_with_variable_replacement() {
        let mock_provider = Arc::new(MockTcpProvider::new(false));
        let executor = TcpConnectExecutor::new(mock_provider);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{host}}", 9090);

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("host".to_string(), json!("example.com"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_next_nodes, output_data) = result.unwrap();
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(data["host"], "example.com");
    }
}
