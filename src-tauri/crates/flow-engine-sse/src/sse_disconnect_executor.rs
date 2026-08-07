// SSE 断开连接节点执行器
//
// 实现 SSE 连接断开功能。
// 断开始终走 "out" 端口（Failed不阻塞流程）。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::SseProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// SSE 断开连接节点执行器
pub struct SseDisconnectExecutor {
    sse_manager: Arc<dyn SseProvider>,
}

impl SseDisconnectExecutor {
    /// 创建新的 SSE 断开连接执行器
    pub fn new(sse_manager: Arc<dyn SseProvider>) -> Self {
        Self { sse_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for SseDisconnectExecutor {
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
            NodeData::SseDisconnect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not SseDisconnectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始断开
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: sse_disconnect, Connection ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("SSE Disconnect 节点 {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建断开配置（使用替换后的 connection_id）
        let disconnect_config = flow_engine_core::node_config::SseDisconnectConfig {
            connection_id: connection_id.clone(),
        };

        // 5. 断开连接
        let disconnect_result = self
            .sse_manager
            .disconnect(&connection_id, &disconnect_config)
            .await;

        // SSE Disconnect 始终走 "out" 端口，不阻塞流程
        let next_node_ids: Vec<String> = {
            let filtered: Vec<String> = connections
                .iter()
                .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "out")
                .map(|conn| conn.target_node_uuid.clone())
                .collect();

            if filtered.is_empty() {
                vec!["out".to_string()]
            } else {
                filtered
            }
        };

        let duration = start_time.elapsed();

        match disconnect_result {
            Ok(()) => {
                log::info!("[SSE] 连接断开Success: connection_id={}", connection_id);

                // 6. 构建输出数据
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                    },
                    "timing": {
                        "disconnect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 日志：断开完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("SSE 连接断开Success ({}ms)", duration.as_millis()),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[SSE] 连接断开Failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：断开Failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("SSE 连接断开Failed: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据（仍然走 out 端口）
                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                    },
                    "timing": {
                        "disconnect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sse_connection_manager::SseConnectionManager;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::SseDisconnectConfig;

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "sse_disconnect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseDisconnect(SseDisconnectConfig {
                connection_id: connection_id.to_string(),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            execution_id: String::new(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: flow_engine_core::utils::unix_timestamp_ms(),
            log_sender: None,
            flow_file_path: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
            current_node_plugin_data: None,
        }
    }

    /// 测试：断开不存在的连接，仍然路由到 "out" 端口（Failed不阻塞流程）
    #[tokio::test]
    async fn test_disconnect_nonexistent_connection_routes_to_out() {
        let sse_manager = Arc::new(SseConnectionManager::new());
        let executor = SseDisconnectExecutor::new(sse_manager);
        let node = create_test_node("test-1", "测试断开", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口（断开Failed不阻塞）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["connection_id"], "nonexistent-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    /// 测试：NodeData 类型不匹配时返回Error
    #[tokio::test]
    async fn test_disconnect_wrong_node_data_type() {
        let sse_manager = Arc::new(SseConnectionManager::new());
        let executor = SseDisconnectExecutor::new(sse_manager);

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
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .message
            .contains("SseDisconnectConfig"));
    }

    /// 测试：变量替换 — `connection_id` 中的变量应被正确替换
    #[tokio::test]
    async fn test_disconnect_with_variable_replacement() {
        let sse_manager = Arc::new(SseConnectionManager::new());
        let executor = SseDisconnectExecutor::new(sse_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("sse-connection-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["connection_id"], "sse-connection-456");
        // 连接不存在所以 success=false
        assert_eq!(data["success"], false);
    }

    /// 测试：共享模式下断开不存在的连接应Success（no-op）
    #[tokio::test]
    async fn test_disconnect_shared_mode_noop() {
        let sse_manager = Arc::new(SseConnectionManager::new_shared());
        let executor = SseDisconnectExecutor::new(sse_manager);

        let node = create_test_node("test-shared", "共享模式断开", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 共享模式下 disconnect 是 no-op，应返回 Ok + success=true
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], true);
    }

    /// `测试：resolve_port` 始终返回 "out"
    #[tokio::test]
    async fn test_disconnect_always_routes_to_out_port() {
        let sse_manager = Arc::new(SseConnectionManager::new());
        let executor = SseDisconnectExecutor::new(sse_manager);

        let node = create_test_node("test-out", "端口测试", "some-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap_or_else(|e| panic!("{e}"));
        // 无连接时默认返回 ["out"]
        assert_eq!(next_nodes, vec!["out"]);
    }

    /// 测试：配置提取 — 从 `NodeData::SseDisconnect` 正确提取
    #[tokio::test]
    async fn test_disconnect_config_extraction() {
        let sse_manager = Arc::new(SseConnectionManager::new());
        let executor = SseDisconnectExecutor::new(sse_manager);

        // 验证 SseDisconnectConfig 能正确从 NodeData 提取
        let node = create_test_node("test-cfg", "配置提取", "my-sse-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["connection_id"], "my-sse-conn");
        assert_eq!(data["request"]["connection_id"], "my-sse-conn");
    }
}
