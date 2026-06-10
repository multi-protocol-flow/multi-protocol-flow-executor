//! IMAP 关闭连接节点执行器
//!
//! 断开 IMAP 连接，始终走 "out" 端口（失败不阻塞流程）。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::ImapProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// IMAP 关闭连接节点执行器
pub struct ImapCloseExecutor {
    imap_manager: Arc<dyn ImapProvider>,
}

impl ImapCloseExecutor {
    /// 创建新的 IMAP 关闭连接执行器
    pub fn new(imap_manager: Arc<dyn ImapProvider>) -> Self { Self { imap_manager } }

    fn ts() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("系统时间错误").as_millis() as u64
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ImapCloseExecutor {
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();
        let config = match &node.data {
            NodeData::ImapClose(c) => c.clone(),
            _ => return Err(format!("节点数据类型不是 ImapCloseConfig: {}", node.uuid)),
        };
        let connection_id = replace_variables(&config.connection_id, context)?;

        log::info!("[流程 {}] [开始] 节点ID: {}, 类型: imap_close", context.flow_uuid, node.uuid);
        context.emit_log(LogEvent { level: LogLevel::Info, message: format!("IMAP Close 节点 {} 开始执行", node.name),
            timestamp: Self::ts(), node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone() }).await;

        let result = self.imap_manager.close(&connection_id).await;

        // Close 始终走 "out" 端口
        let next_node_ids: Vec<String> = {
            let filtered: Vec<String> = connections.iter()
                .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "out")
                .map(|conn| conn.target_node_uuid.clone()).collect();
            if filtered.is_empty() { vec!["out".to_string()] } else { filtered }
        };

        let duration = start_time.elapsed();

        match result {
            Ok(output) => {
                log::info!("[IMAP] 连接关闭成功: connection_id={}", connection_id);
                let output_data = json!({
                    "success": output.success, "connection_id": connection_id,
                    "request": { "connection_id": connection_id },
                    "timing": { "close_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                });
                context.emit_log(LogEvent { level: LogLevel::Info,
                    message: format!("IMAP 连接关闭成功 ({}ms)", duration.as_millis()),
                    timestamp: Self::ts(), node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone() }).await;
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!("[IMAP] 连接关闭失败: connection_id={}, error={}", connection_id, e);
                context.emit_log(LogEvent { level: LogLevel::Error, message: format!("IMAP 连接关闭失败: {}", e),
                    timestamp: Self::ts(), node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone() }).await;
                let error_output = json!({
                    "success": false, "connection_id": connection_id,
                    "request": { "connection_id": connection_id },
                    "timing": { "close_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
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
    use crate::imap_connection_manager::ImapConnectionManager;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::ImapCloseConfig;

    fn create_test_node(uuid: &str, connection_id: &str) -> Node {
        Node { uuid: uuid.to_string(), node_type: "imap_close".to_string(), name: "关闭".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::ImapClose(ImapCloseConfig { connection_id: connection_id.to_string() }),
            ports: None, has_breakpoint: None }
    }

    fn create_context() -> ExecutionContext {
        ExecutionContext { flow_uuid: "test-flow".to_string(), current_node_uuid: None, variables: VariableStorage::new(),
            node_reports: vec![], flow_trigger_time: 0, log_sender: None, flow_file_path: None, ws_manager: None,
            ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None,
            mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None,
            smtp_manager: None, imap_manager: None, redis_manager: None, last_node_output: None, debug_event_sender: None,
            execution_event_sender: None, cancel_receiver: None, providers: HashMap::new() }
    }

    #[tokio::test]
    async fn test_close_nonexistent_routes_to_out() {
        let m = Arc::new(ImapConnectionManager::new()); let e = ImapCloseExecutor::new(m);
        let n = create_test_node("test-1", "nonexistent-conn");
        let mut c = create_context();
        let r = e.execute(&n, &mut c, &[], &HashMap::new()).await;
        assert!(r.is_ok());
        let (nn, d) = r.unwrap();
        assert_eq!(nn, vec!["out"]);
        assert_eq!(d.unwrap()["success"], false);
    }

    #[tokio::test]
    async fn test_close_wrong_type() {
        let m = Arc::new(ImapConnectionManager::new()); let e = ImapCloseExecutor::new(m);
        let n = Node { uuid: "w".to_string(), node_type: "http".to_string(), name: "x".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 }, data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None, has_breakpoint: None };
        let mut c = create_context();
        let r = e.execute(&n, &mut c, &[], &HashMap::new()).await;
        assert!(r.is_err()); assert!(r.unwrap_err().contains("ImapCloseConfig"));
    }
}
