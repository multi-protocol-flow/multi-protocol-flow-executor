// UDP Close 节点执行器
//
// 实现 UDP Socket 关闭功能。

use flow_engine_core::connection::Connection;
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

/// UDP Close 节点执行器
pub struct UdpCloseExecutor {
    udp_manager: Arc<dyn UdpProvider>,
}

impl UdpCloseExecutor {
    /// 创建新的 UDP Close 执行器
    pub fn new(udp_manager: Arc<dyn UdpProvider>) -> Self {
        Self { udp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for UdpCloseExecutor {
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
            NodeData::UdpClose(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 UdpCloseConfig: {}", node.uuid)),
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

        // 3. 日志：开始关闭
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: udp_close, 连接ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("UDP Close 节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 关闭 Socket
        let close_result = self.udp_manager.close(&connection_id).await;

        match close_result {
            Ok(()) => {
                log::info!("[UDP] Socket 关闭成功: connection_id={}", connection_id);

                // 5. 构建输出数据
                let duration = start_time.elapsed();
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
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
                            "UDP Socket 关闭成功 ({}ms)",
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
                    "[UDP] Socket 关闭失败: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：关闭失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("UDP Socket 关闭失败: {}", e),
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
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(format!("Flow aborted: {}", e));
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
    use flow_engine_core::node_config::UdpCloseConfig;

    // -----------------------------------------------------------------------
    // Mock UdpProvider
    // -----------------------------------------------------------------------

    struct MockUdpProvider {
        close_result: Result<(), String>,
    }

    impl MockUdpProvider {
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
            Err("MockUdpProvider::recv_from 不应被调用".to_string())
        }

        async fn close(&self, _uuid: &str) -> Result<(), String> {
            self.close_result.clone()
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }
    }

    // -----------------------------------------------------------------------
    // 辅助函数
    // -----------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "udp_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpClose(UdpCloseConfig {
                connection_id: connection_id.to_string(),
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文（带 UDP 管理器）
    fn create_context<T: 'static + Send + Sync>(flow_id: &str, udp_manager: Arc<T>) -> ExecutionContext {
        let mut ctx = ExecutionContext::new(flow_id.to_string(), 0);
        ctx.set_provider("udp", udp_manager);
        ctx
    }

    // -----------------------------------------------------------------------
    // 测试
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_udp_close_success() {
        let mock = Arc::new(MockUdpProvider::new_ok());
        let executor = UdpCloseExecutor::new(mock.clone());
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
    async fn test_udp_close_not_found() {
        let mock = Arc::new(MockUdpProvider::new_err("连接不存在"));
        let executor = UdpCloseExecutor::new(mock.clone());
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
    async fn test_udp_close_with_variable_replacement() {
        let mock = Arc::new(MockUdpProvider::new_err("连接不存在"));
        let executor = UdpCloseExecutor::new(mock.clone());

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow", mock);
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("udp-connection-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 变量应被替换
        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "udp-connection-456");
    }

    #[tokio::test]
    async fn test_udp_close_wrong_node_data_type() {
        let mock = Arc::new(MockUdpProvider::new_ok());
        let executor = UdpCloseExecutor::new(mock);

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
        assert!(result.unwrap_err().contains("UdpCloseConfig"));
    }
}
