// TCP Close 节点执行器
//
// 实现 TCP 连接关闭功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::traits::TcpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// TCP Close 节点执行器
pub struct TcpCloseExecutor {
    tcp_manager: Arc<dyn TcpProvider>,
}

impl TcpCloseExecutor {
    /// 创建新的 TCP Close 执行器
    pub fn new(tcp_manager: Arc<dyn TcpProvider>) -> Self {
        Self { tcp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for TcpCloseExecutor {
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
            NodeData::TcpClose(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not TcpCloseConfig: {}", node.uuid))),
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

        // 3. 日志：开始关闭
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: tcp_close, ConnectionID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("TCP Close node {} started execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 关闭连接
        let close_result = self
            .tcp_manager
            .close(&connection_id, config.close_timeout_ms)
            .await;

        match close_result {
            Ok(()) => {
                log::info!("[TCP] Connection closed successfully: connection_id={}", connection_id);

                // 5. 构建输出数据
                let duration = start_time.elapsed();
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                        "close_timeout_ms": config.close_timeout_ms,
                    },
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 6. 日志：关闭完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "TCP connection closed successfully ({}ms)",
                            duration.as_millis()
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

                // 7. 成功时走 true 端口
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| {
                        conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
                    })
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[TCP] Connection close failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：关闭失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("TCP connection close failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                        "close_timeout_ms": config.close_timeout_ms,
                    },
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                // 关闭失败走 false 端口
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| {
                        conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                    })
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::TcpCloseConfig;
    use flow_engine_core::traits::TcpConnectionInfo;

    // -----------------------------------------------------------------------
    // Mock TcpProvider
    // -----------------------------------------------------------------------

    struct MockTcpProvider {
        close_result: Result<(), String>,
    }

    impl MockTcpProvider {
        fn new_ok() -> Self {
            Self {
                close_result: Ok(()),
            }
        }

        fn new_err(msg: &str) -> Self {
            Self {
                close_result: Err(msg.to_string()),
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
            _framing_config: flow_engine_core::traits::FramingConfig,
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
            Err("MockTcpProvider::receive 不应被调用".to_string())
        }

        async fn close(&self, _uuid: &str, _close_timeout_ms: u64) -> Result<(), String> {
            // 简单返回预设结果，忽略参数
            self.close_result.clone()
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }

        fn list_connections(&self) -> Vec<TcpConnectionInfo> {
            vec![]
        }
    }

    // -----------------------------------------------------------------------
    // 辅助函数
    // -----------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "tcp_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::TcpClose(TcpCloseConfig {
                connection_id: connection_id.to_string(),
                close_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文（带 TCP 管理器）
    fn create_context<T: 'static + Send + Sync>(flow_id: &str, tcp_manager: Arc<T>) -> ExecutionContext {
        let mut ctx = ExecutionContext::new(flow_id.to_string(), 0);
        ctx.set_provider("tcp", tcp_manager);
        ctx
    }

    // -----------------------------------------------------------------------
    // 测试
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_tcp_close_success() {
        let mock = Arc::new(MockTcpProvider::new_ok());
        let executor = TcpCloseExecutor::new(mock.clone());
        let node = create_test_node("test-1", "测试关闭", "conn-123");
        let mut context = create_context("test-flow", mock);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 无连接，所以 next_nodes 为空
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "conn-123");
        assert!(data["timing"]["total_ms"].is_number());
    }

    #[tokio::test]
    async fn test_tcp_close_not_found() {
        let mock = Arc::new(MockTcpProvider::new_err("连接不存在"));
        let executor = TcpCloseExecutor::new(mock.clone());
        let node = create_test_node("test-2", "关闭不存在连接", "nonexistent-conn");
        let mut context = create_context("test-flow", mock);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "nonexistent-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_tcp_close_with_variable_replacement() {
        let mock = Arc::new(MockTcpProvider::new_err("连接不存在"));
        let executor = TcpCloseExecutor::new(mock.clone());

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow", mock);
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("tcp-connection-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 变量应被替换
        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "tcp-connection-456");
    }

    #[tokio::test]
    async fn test_tcp_close_wrong_node_data_type() {
        let mock = Arc::new(MockTcpProvider::new_ok());
        let executor = TcpCloseExecutor::new(mock);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = ExecutionContext::new("test-flow".to_string(), 0);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TcpCloseConfig"));
    }
}
