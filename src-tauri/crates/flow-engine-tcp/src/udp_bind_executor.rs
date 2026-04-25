// UDP Bind 节点执行器
//
// 实现 UDP Socket 绑定和变量替换功能。

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

/// UDP Bind 节点执行器
pub struct UdpBindExecutor {
    udp_manager: Arc<dyn UdpProvider>,
}

impl UdpBindExecutor {
    /// 创建新的 UDP Bind 执行器
    pub fn new(udp_manager: Arc<dyn UdpProvider>) -> Self {
        Self { udp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for UdpBindExecutor {
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
            NodeData::UdpBind(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 UdpBindConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换 local_addr
        let local_addr = replace_variables(&config.local_addr, context).map_err(|e| {
            let error_msg = format!("本地地址变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 日志：开始绑定
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: udp_bind, 目标: {}:{}",
            context.flow_uuid,
            node.uuid,
            node.name,
            local_addr,
            config.local_port
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("UDP Bind 节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 绑定 Socket（计时）
        let bind_start = Instant::now();
        let bind_result = self
            .udp_manager
            .bind(
                node.uuid.clone(),
                node.name.clone(),
                local_addr.clone(),
                config.local_port,
                0, // read_timeout_ms 使用默认值
            )
            .await;
        let bind_duration_ms = bind_start.elapsed().as_millis() as u64;

        match bind_result {
            Ok(()) => {
                log::info!(
                    "[UDP] 绑定成功: uuid={}, addr={}:{}",
                    node.uuid,
                    local_addr,
                    config.local_port
                );

                // 5. 构建成功输出数据
                let total_ms = start_time.elapsed().as_millis() as u64;
                let output_data = json!({
                    "success": true,
                    "connection_id": node.uuid,
                    "local_addr": local_addr,
                    "local_port": config.local_port,
                    "timing": {
                        "bind_ms": bind_duration_ms,
                        "total_ms": total_ms,
                    },
                });

                // 6. 日志：绑定完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "UDP 绑定成功 {}:{} ({}ms)",
                            local_addr,
                            config.local_port,
                            total_ms
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

                // 7. 返回 true 端口（绑定成功）
                let next_node_ids = resolve_port_nodes(connections, &node.uuid, "true");

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[UDP] 绑定失败: uuid={}, addr={}:{}, error={}",
                    node.uuid,
                    local_addr,
                    config.local_port,
                    e
                );

                // 日志：绑定失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("UDP 绑定失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据
                let total_ms = start_time.elapsed().as_millis() as u64;
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "local_addr": local_addr,
                    "local_port": config.local_port,
                    "timing": {
                        "bind_ms": bind_duration_ms,
                        "total_ms": total_ms,
                    },
                    "error": e,
                });

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
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::UdpBindConfig;
    use std::sync::atomic::{AtomicBool, Ordering};

    // -----------------------------------------------------------------------
    // Mock UdpProvider
    // -----------------------------------------------------------------------

    /// Mock UDP 提供者（可配置绑定成功/失败）
    struct MockUdpProvider {
        should_succeed: AtomicBool,
    }

    impl MockUdpProvider {
        fn new(should_succeed: bool) -> Self {
            Self {
                should_succeed: AtomicBool::new(should_succeed),
            }
        }
    }

    #[async_trait::async_trait]
    impl UdpProvider for MockUdpProvider {
        async fn bind(
            &self,
            uuid: String,
            _name: String,
            local_addr: String,
            local_port: u16,
            _read_timeout_ms: u64,
        ) -> Result<(), String> {
            if self.should_succeed.load(Ordering::Relaxed) {
                log::info!(
                    "[MockUdp] 绑定成功: uuid={}, {}:{}",
                    uuid,
                    local_addr,
                    local_port
                );
                Ok(())
            } else {
                Err(format!(
                    "MockUdp 绑定失败: {}:{}",
                    local_addr, local_port
                ))
            }
        }

        async fn send_to(
            &self,
            _uuid: &str,
            _data: Vec<u8>,
            _target_addr: String,
            _target_port: u16,
            _send_timeout_ms: u64,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn recv_from(
            &self,
            _uuid: &str,
            _receive_timeout_ms: u64,
        ) -> Result<Option<(Vec<u8>, String, u16)>, String> {
            Ok(None)
        }

        async fn close(&self, _uuid: &str) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            if self.should_succeed.load(Ordering::Relaxed) {
                Some("Bound".to_string())
            } else {
                Some("Error".to_string())
            }
        }
    }

    // -----------------------------------------------------------------------
    // 辅助函数
    // -----------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, local_addr: &str, local_port: u16) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "udp_bind".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpBind(UdpBindConfig {
                local_addr: local_addr.to_string(),
                local_port,
                reuse_addr: true,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext::new(flow_id.to_string(), 0)
    }

    // -----------------------------------------------------------------------
    // 执行器测试
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_udp_bind_executor_success() {
        let mock_provider = Arc::new(MockUdpProvider::new(true));
        let executor = UdpBindExecutor::new(mock_provider);
        let node = create_test_node("test-1", "测试绑定", "0.0.0.0", 8080);
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
        assert_eq!(data["local_addr"], "0.0.0.0");
        assert_eq!(data["local_port"], 8080);
        assert!(data.get("timing").is_some());
        assert!(data["timing"]["total_ms"].is_number());
    }

    #[tokio::test]
    async fn test_udp_bind_executor_failure() {
        let mock_provider = Arc::new(MockUdpProvider::new(false));
        let executor = UdpBindExecutor::new(mock_provider);
        let node = create_test_node("test-2", "失败绑定", "192.168.1.1", 9999);
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
        assert_eq!(data["local_addr"], "192.168.1.1");
        assert_eq!(data["local_port"], 9999);
        assert!(data.get("error").is_some());
        let error = data["error"].as_str().expect("error 应为字符串");
        assert!(error.contains("绑定失败"));
    }

    #[tokio::test]
    async fn test_udp_bind_wrong_node_data_type() {
        let mock_provider = Arc::new(MockUdpProvider::new(true));
        let executor = UdpBindExecutor::new(mock_provider);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("UdpBindConfig"));
    }

    #[tokio::test]
    async fn test_udp_bind_with_variable_replacement() {
        let mock_provider = Arc::new(MockUdpProvider::new(true));
        let executor = UdpBindExecutor::new(mock_provider);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{addr}}", 9090);

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("addr".to_string(), json!("127.0.0.1"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_next_nodes, output_data) = result.unwrap();
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(data["local_addr"], "127.0.0.1");
    }
}
