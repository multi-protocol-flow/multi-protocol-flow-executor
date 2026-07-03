// gRPC Close 节点执行器
//
// 实现 gRPC 连接关闭功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::GrpcProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// gRPC Close 节点执行器
pub struct GrpcCloseExecutor {
    grpc_manager: Arc<dyn GrpcProvider>,
}

impl GrpcCloseExecutor {
    /// 创建新的 gRPC Close 执行器
    pub fn new(grpc_manager: Arc<dyn GrpcProvider>) -> Self {
        Self { grpc_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for GrpcCloseExecutor {
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
            NodeData::GrpcClose(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not GrpcCloseConfig: {}", node.uuid))),
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

        // 3. 日志：开始关闭
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: grpc_close, Connection ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("gRPC Close 节点 {} start execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间Error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 关闭连接
        let close_result = self.grpc_manager.close(&connection_id).await;

        match close_result {
            Ok(()) => {
                log::info!(
                    "[gRPC] 连接关闭Success: connection_id={}",
                    connection_id
                );

                // 5. 构建输出数据
                let duration = start_time.elapsed();
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
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
                            "gRPC 连接关闭Success ({}ms)",
                            duration.as_millis()
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间Error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 6.5 发送调试事件：GrpcChannelClosed
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::GrpcChannelClosed {
                        node_uuid: node.uuid.clone(),
                        connection_id: connection_id.clone(),
                        url: String::new(),
                    });
                }

                // 7. Success时走 true 端口
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
                    "[gRPC] 连接关闭Failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：关闭Failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("gRPC 连接关闭Failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间Error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
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

                // 关闭Failed走 false 端口
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
    use flow_engine_core::node_config::GrpcCloseConfig;
    use crate::grpc_connection_manager::GrpcConnectionManager;

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "grpc_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcClose(GrpcCloseConfig {
                connection_id: connection_id.to_string(),
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
                .expect("系统时间Error")
                .as_millis() as u64,
        )
    }

    #[tokio::test]
    async fn test_close_connection_not_found() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCloseExecutor::new(grpc_manager);
        let node = create_test_node("test-1", "测试关闭", "nonexistent-conn");
        let mut context = create_context("test-flow");

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
    async fn test_close_with_variable_replacement() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCloseExecutor::new(grpc_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("grpc-connection-123"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "grpc-connection-123");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_close_wrong_node_data_type() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCloseExecutor::new(grpc_manager);

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
        assert!(result.unwrap_err().message.contains("GrpcCloseConfig"));
    }
}
