//! IMAP fetchmessages节点执行器

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
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

/// IMAP fetchmessages节点执行器
pub struct ImapFetchExecutor {
    imap_manager: Arc<dyn ImapProvider>,
}

impl ImapFetchExecutor {
    /// 创建新的 IMAP fetchmessages执行器
    pub fn new(imap_manager: Arc<dyn ImapProvider>) -> Self {
        Self { imap_manager }
    }

    fn resolve_next_nodes(node_uuid: &str, port: &str, connections: &[Connection]) -> Vec<String> {
        let filtered: Vec<String> = connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node_uuid && conn.source_port_id == port)
            .map(|conn| conn.target_node_uuid.clone())
            .collect();
        if filtered.is_empty() { vec![port.to_string()] } else { filtered }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("system time went backwards").as_millis() as u64
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ImapFetchExecutor {
    async fn execute(
        &self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        let config = match &node.data {
            NodeData::ImapFetch(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("[E_IMAP_FETCH] Node data is not ImapFetchConfig: {}", node.uuid))),
        };

        let connection_id = replace_variables(&config.connection_id, context).map_err(ErrorDetail::execution_error)?;
        let uids = replace_variables(&config.uids, context).map_err(ErrorDetail::execution_error)?;
        let fetch_items = replace_variables(&config.fetch_items, context).map_err(ErrorDetail::execution_error)?;

        log::info!(
            "[Flow {}] [Start] nodeID: {}, nodeName: {}, type: imap_fetch",
            context.flow_uuid, node.uuid, node.name
        );
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("IMAP Fetch 节点 {} started execution", node.name),
            timestamp: Self::current_timestamp(),
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        let result = self.imap_manager.fetch(
            &connection_id, &uids, &fetch_items,
            config.max_fetch_count, config.max_size_bytes,
        ).await;

        let duration = start_time.elapsed();

        match result {
            Ok(output) => {
                let port = if output.success { "true" } else { "false" };
                let output_data = json!({
                    "success": output.success,
                    "messages": output.messages,
                    "skipped_count": output.skipped_count,
                    "request": {
                        "connection_id": connection_id,
                        "uids": uids,
                        "fetch_items": fetch_items,
                        "max_fetch_count": config.max_fetch_count,
                        "max_size_bytes": config.max_size_bytes,
                    },
                    "timing": { "fetch_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": output.error,
                });

                let log_level = if output.success { LogLevel::Info } else { LogLevel::Error };
                let log_msg = if output.success {
                    format!("IMAP fetch succeeded, got {} messages ({}ms)", output.messages.len(), duration.as_millis())
                } else {
                    format!("IMAP fetch failed: {} ({}ms)", output.error.as_deref().unwrap_or("unknown error"), duration.as_millis())
                };
                context.emit_log(LogEvent {
                    level: log_level, message: log_msg, timestamp: Self::current_timestamp(),
                    node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                let next = Self::resolve_next_nodes(&node.uuid, port, connections);
                Ok((next, Some(output_data)))
            }
            Err(e) => {
                context.emit_log(LogEvent {
                    level: LogLevel::Error, message: format!("IMAP fetchexception: {}", e),
                    timestamp: Self::current_timestamp(), node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()), flow_uuid: context.flow_uuid.clone(),
                }).await;
                let error_output = json!({
                    "success": false, "messages": [], "skipped_count": 0,
                    "request": { "connection_id": connection_id, "uids": uids },
                    "timing": { "fetch_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": e,
                });
                // 检查 on_error 策略：AbortFlow 时返回错误而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("IMAP fetch failed: {}", e)));
                }

                let next = Self::resolve_next_nodes(&node.uuid, "false", connections);
                Ok((next, Some(error_output)))
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
    use flow_engine_core::node_config::ImapFetchConfig;

    fn create_test_node(uuid: &str) -> Node {
        Node {
            uuid: uuid.to_string(), node_type: "imap_fetch".to_string(),
            name: "fetch".to_string(), position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::ImapFetch(ImapFetchConfig {
                connection_id: "conn-1".to_string(), uids: "1:*".to_string(),
                fetch_items: "(ENVELOPE)".to_string(), max_fetch_count: 50, max_size_bytes: 5242880,
            }),
            ports: None, has_breakpoint: None,
        }
    }

    fn create_context() -> ExecutionContext {
        ExecutionContext {
            flow_uuid: "test-flow".to_string(), current_node_uuid: None,
            variables: VariableStorage::new(), node_reports: vec![],
            flow_trigger_time: 0, log_sender: None, flow_file_path: None,
            ws_manager: None, ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None, grpc_manager: None, mcp_manager: None,
            tcp_manager: None, udp_manager: None, mqtt_manager: None,
            kafka_manager: None, sse_manager: None, graphql_manager: None,
            amqp_manager: None, smtp_manager: None, imap_manager: None, redis_manager: None,
            last_node_output: None, debug_event_sender: None,
            execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_fetch_provider_error_routes_to_false() {
        let imap_manager = Arc::new(ImapConnectionManager::new());
        let executor = ImapFetchExecutor::new(imap_manager);
        let node = create_test_node("test-1");
        let mut context = create_context();
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_nodes, data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert_eq!(data.unwrap()["success"], false);
    }

    #[tokio::test]
    async fn test_fetch_wrong_node_data_type() {
        let imap_manager = Arc::new(ImapConnectionManager::new());
        let executor = ImapFetchExecutor::new(imap_manager);
        let node = Node {
            uuid: "test-wrong".to_string(), node_type: "http".to_string(),
            name: "wrong-type".to_string(), position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None, has_breakpoint: None,
        };
        let mut context = create_context();
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ImapFetchConfig"));
    }
}
