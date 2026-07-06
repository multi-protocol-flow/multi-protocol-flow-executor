// UDP Bind 节点执行器
//
// 实现 UDP Socket 绑定和变量替换功能。
// 支持 reuse_addr/broadcast/multicast 等高级选项。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// UDP Bind 节点执行器
pub struct UdpBindExecutor {
    udp_manager: Arc<crate::udp_socket_manager::UdpSocketManager>,
}

impl UdpBindExecutor {
    /// 创建新的 UDP Bind 执行器
    pub fn new(udp_manager: Arc<crate::udp_socket_manager::UdpSocketManager>) -> Self {
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::UdpBind(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not UdpBindConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换 local_addr
        let local_addr = replace_variables(&config.local_addr, context).map_err(|e| {
            let error_msg = format!("Local address variable replacement failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始绑定
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: udp_bind, Target: {}:{}",
            context.flow_uuid,
            node.uuid,
            node.name,
            local_addr,
            config.local_port
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("UDP Bind node {} started execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 绑定 Socket（带完整配置选项）
        let bind_start = Instant::now();
        let bind_result = self
            .udp_manager
            .bind_with_options(
                node.uuid.clone(),
                node.name.clone(),
                local_addr.clone(),
                config.local_port,
                0, // read_timeout_ms 使用默认值
                config.reuse_addr,
                config.broadcast,
                config.multicast_group.clone(),
                config.multicast_ttl,
                config.multicast_loop,
                config.multicast_interface.clone(),
            )
            .await;
        let bind_duration_ms = bind_start.elapsed().as_millis() as u64;

        match bind_result {
            Ok(()) => {
                log::info!(
                    "[UDP] Bind successful: uuid={}, addr={}:{}",
                    node.uuid,
                    local_addr,
                    config.local_port
                );

                // 5. 构建Success输出数据
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
                            "UDP bind successful {}:{} ({}ms)",
                            local_addr, config.local_port, total_ms
                        ),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 7. 返回 true 端口（绑定Success）
                let next_node_ids = resolve_port_nodes(connections, &node.uuid, "true");

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[UDP] Bind failed: uuid={}, addr={}:{}, error={}",
                    node.uuid,
                    local_addr,
                    config.local_port,
                    e
                );

                // 日志：绑定Failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("UDP bind failed: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
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
                    "local_addr": local_addr,
                    "local_port": config.local_port,
                    "timing": {
                        "bind_ms": bind_duration_ms,
                        "total_ms": total_ms,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
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
    use flow_engine_core::node_config::UdpBindConfig;

    // -----------------------------------------------------------------------
    // 辅助函数
    // -----------------------------------------------------------------------

    /// 创建Test Node
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
                broadcast: false,
                multicast_group: None,
                multicast_ttl: None,
                multicast_loop: None,
                multicast_interface: None,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
        let manager = Arc::new(crate::udp_socket_manager::UdpSocketManager::new());
        let executor = UdpBindExecutor::new(manager);
        let node = create_test_node("test-1", "测试绑定", "0.0.0.0", 0);
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
        assert!(data.get("timing").is_some());
        assert!(data["timing"]["total_ms"].is_number());
    }

    #[tokio::test]
    async fn test_udp_bind_executor_failure() {
        let manager = Arc::new(crate::udp_socket_manager::UdpSocketManager::new());
        let executor = UdpBindExecutor::new(manager);
        // 使用无效地址触发绑定Failed
        let node = create_test_node("test-2", "Failed绑定", "256.256.256.256", 9999);
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
        assert!(data.get("error").is_some());
        let error = data["error"].as_str().expect("error 应为字符串");
        assert!(!error.is_empty());
    }

    #[tokio::test]
    async fn test_udp_bind_wrong_node_data_type() {
        let manager = Arc::new(crate::udp_socket_manager::UdpSocketManager::new());
        let executor = UdpBindExecutor::new(manager);

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
        assert!(result.unwrap_err().message.contains("UdpBindConfig"));
    }

    #[tokio::test]
    async fn test_udp_bind_with_variable_replacement() {
        let manager = Arc::new(crate::udp_socket_manager::UdpSocketManager::new());
        let executor = UdpBindExecutor::new(manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{addr}}", 0);

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

    #[tokio::test]
    async fn test_udp_bind_with_broadcast() {
        let manager = Arc::new(crate::udp_socket_manager::UdpSocketManager::new());
        let executor = UdpBindExecutor::new(manager);

        // 创建启用广播的节点
        let node = Node {
            uuid: "test-broadcast".to_string(),
            node_type: "udp_bind".to_string(),
            name: "广播测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::UdpBind(UdpBindConfig {
                local_addr: "0.0.0.0".to_string(),
                local_port: 0,
                reuse_addr: true,
                broadcast: true,
                multicast_group: None,
                multicast_ttl: None,
                multicast_loop: None,
                multicast_interface: None,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
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
    }
}
