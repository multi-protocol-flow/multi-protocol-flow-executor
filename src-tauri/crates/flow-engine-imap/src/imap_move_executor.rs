//! IMAP moved messages节点执行器

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::ImapProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// IMAP moved messages节点执行器
pub struct ImapMoveExecutor {
    imap_manager: Arc<dyn ImapProvider>,
}

impl ImapMoveExecutor {
    /// 创建新的 IMAP moved messages执行器
    pub fn new(imap_manager: Arc<dyn ImapProvider>) -> Self { Self { imap_manager } }

    fn resolve_next_nodes(node_uuid: &str, port: &str, connections: &[Connection]) -> Vec<String> {
        let f: Vec<String> = connections.iter()
            .filter(|c| c.source_node_uuid == node_uuid && c.source_port_id == port)
            .map(|c| c.target_node_uuid.clone()).collect();
        if f.is_empty() { vec![port.to_string()] } else { f }
    }

    fn ts() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("system time went backwards").as_millis() as u64
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ImapMoveExecutor {
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();
        let config = match &node.data {
            NodeData::ImapMove(c) => c.clone(),
            _ => return Err(format!("[E_IMAP_FETCH] Node data is not ImapMoveConfig: {}", node.uuid)),
        };
        let connection_id = replace_variables(&config.connection_id, context)?;
        let uids = replace_variables(&config.uids, context)?;
        let target_mailbox = replace_variables(&config.target_mailbox, context)?;

        log::info!("[Flow {}] [Start] nodeID: {}, type: imap_move", context.flow_uuid, node.uuid);
        context.emit_log(LogEvent { level: LogLevel::Info, message: format!("IMAP Move 节点 {} started execution", node.name),
            timestamp: Self::ts(), node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone() }).await;

        let result = self.imap_manager.r#move(&connection_id, &uids, &target_mailbox).await;
        let duration = start_time.elapsed();

        match result {
            Ok(output) => {
                let port = if output.success { "true" } else { "false" };
                let output_data = json!({
                    "success": output.success, "source_mailbox": output.source_mailbox,
                    "target_mailbox": output.target_mailbox, "moved_count": output.moved_count,
                    "request": { "connection_id": connection_id, "uids": uids, "target_mailbox": target_mailbox },
                    "timing": { "move_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": output.error,
                });
                let ll = if output.success { LogLevel::Info } else { LogLevel::Error };
                let msg = if output.success {
                    format!("IMAP move succeeded，moved  {} items ({}ms)", output.moved_count, duration.as_millis())
                } else { format!("IMAP move failed: {} ({}ms)", output.error.as_deref().unwrap_or("unknown error"), duration.as_millis()) };
                context.emit_log(LogEvent { level: ll, message: msg, timestamp: Self::ts(),
                    node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()), flow_uuid: context.flow_uuid.clone() }).await;
                Ok((Self::resolve_next_nodes(&node.uuid, port, connections), Some(output_data)))
            }
            Err(e) => {
                context.emit_log(LogEvent { level: LogLevel::Error, message: format!("IMAP move exception: {}", e),
                    timestamp: Self::ts(), node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone() }).await;
                let err_out = json!({ "success": false, "moved_count": 0,
                    "request": { "connection_id": connection_id, "uids": uids, "target_mailbox": target_mailbox },
                    "timing": { "move_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 }, "error": e });
                // 检查 on_error 策略：AbortFlow 时返回错误而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(format!("IMAP move failed: {}", e));
                }

                Ok((Self::resolve_next_nodes(&node.uuid, "false", connections), Some(err_out)))
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
    use flow_engine_core::node_config::ImapMoveConfig;

    fn create_node(uuid: &str) -> Node {
        Node { uuid: uuid.to_string(), node_type: "imap_move".to_string(), name: "move".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::ImapMove(ImapMoveConfig { connection_id: "c1".to_string(), uids: "1".to_string(), target_mailbox: "Trash".to_string() }),
            ports: None, has_breakpoint: None }
    }
    fn create_ctx() -> ExecutionContext {
        ExecutionContext { flow_uuid: "f".to_string(), current_node_uuid: None, variables: VariableStorage::new(),
            node_reports: vec![], flow_trigger_time: 0, log_sender: None, flow_file_path: None, ws_manager: None,
            ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None,
            mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None,
            smtp_manager: None, imap_manager: None, redis_manager: None, last_node_output: None, debug_event_sender: None,
            execution_event_sender: None, cancel_receiver: None, providers: HashMap::new() }
    }

    #[tokio::test]
    async fn test_move_provider_error() {
        let m = Arc::new(ImapConnectionManager::new()); let e = ImapMoveExecutor::new(m);
        let n = create_node("t1"); let mut c = create_ctx();
        let r = e.execute(&n, &mut c, &[], &HashMap::new()).await;
        assert!(r.is_ok()); let (nn, d) = r.unwrap(); assert_eq!(nn, vec!["false"]); assert_eq!(d.unwrap()["success"], false);
    }

    #[tokio::test]
    async fn test_move_wrong_type() {
        let m = Arc::new(ImapConnectionManager::new()); let e = ImapMoveExecutor::new(m);
        let n = Node { uuid: "w".to_string(), node_type: "http".to_string(), name: "x".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 }, data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None, has_breakpoint: None };
        let mut c = create_ctx();
        let r = e.execute(&n, &mut c, &[], &HashMap::new()).await;
        assert!(r.is_err()); assert!(r.unwrap_err().contains("ImapMoveConfig"));
    }
}
