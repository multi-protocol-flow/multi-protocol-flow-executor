//! IMAP 搜索邮件节点执行器

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

/// IMAP 搜索邮件节点执行器
pub struct ImapSearchExecutor {
    imap_manager: Arc<dyn ImapProvider>,
}

impl ImapSearchExecutor {
    pub fn new(imap_manager: Arc<dyn ImapProvider>) -> Self {
        Self { imap_manager }
    }

    fn resolve_next_nodes(node_uuid: &str, port: &str, connections: &[Connection]) -> Vec<String> {
        let filtered: Vec<String> = connections.iter()
            .filter(|c| c.source_node_uuid == node_uuid && c.source_port_id == port)
            .map(|c| c.target_node_uuid.clone()).collect();
        if filtered.is_empty() { vec![port.to_string()] } else { filtered }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("系统时间错误").as_millis() as u64
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ImapSearchExecutor {
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();
        let config = match &node.data {
            NodeData::ImapSearch(c) => c.clone(),
            _ => return Err(format!("节点数据类型不是 ImapSearchConfig: {}", node.uuid)),
        };
        let connection_id = replace_variables(&config.connection_id, context)?;
        let criteria = replace_variables(&config.criteria, context)?;
        let mailbox = match &config.mailbox {
            Some(m) => Some(replace_variables(m, context)?),
            None => None,
        };
        log::info!("[流程 {}] [开始] 节点ID: {}, 类型: imap_search", context.flow_uuid, node.uuid);
        context.emit_log(LogEvent { level: LogLevel::Info, message: format!("IMAP Search 节点 {} 开始执行", node.name),
            timestamp: Self::current_timestamp(), node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()), flow_uuid: context.flow_uuid.clone() }).await;

        let result = self.imap_manager.search(&connection_id, mailbox.as_deref(), &criteria, config.max_results).await;
        let duration = start_time.elapsed();

        match result {
            Ok(output) => {
                let port = if output.success { "true" } else { "false" };
                let output_data = json!({
                    "success": output.success, "uids": output.uids, "messages": output.messages,
                    "request": { "connection_id": connection_id, "mailbox": mailbox, "criteria": criteria, "max_results": config.max_results },
                    "timing": { "search_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 },
                    "error": output.error,
                });
                let ll = if output.success { LogLevel::Info } else { LogLevel::Error };
                let msg = if output.success {
                    format!("IMAP 搜索成功，找到 {} 条 ({}ms)", output.uids.len(), duration.as_millis())
                } else {
                    format!("IMAP 搜索失败: {} ({}ms)", output.error.as_deref().unwrap_or("未知错误"), duration.as_millis())
                };
                context.emit_log(LogEvent { level: ll, message: msg, timestamp: Self::current_timestamp(),
                    node_uuid: Some(node.uuid.clone()), node_name: Some(node.name.clone()), flow_uuid: context.flow_uuid.clone() }).await;
                Ok((Self::resolve_next_nodes(&node.uuid, port, connections), Some(output_data)))
            }
            Err(e) => {
                context.emit_log(LogEvent { level: LogLevel::Error, message: format!("IMAP 搜索异常: {}", e),
                    timestamp: Self::current_timestamp(), node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()), flow_uuid: context.flow_uuid.clone() }).await;
                let err_out = json!({ "success": false, "uids": [], "messages": [],
                    "request": { "connection_id": connection_id, "criteria": criteria },
                    "timing": { "search_ms": duration.as_millis() as u64, "total_ms": duration.as_millis() as u64 }, "error": e });
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
    use flow_engine_core::node_config::ImapSearchConfig;

    fn create_test_node(uuid: &str) -> Node {
        Node { uuid: uuid.to_string(), node_type: "imap_search".to_string(), name: "搜索".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::ImapSearch(ImapSearchConfig { connection_id: "conn-1".to_string(), mailbox: None, criteria: "ALL".to_string(), max_results: None }),
            ports: None, has_breakpoint: None }
    }
    fn create_context() -> ExecutionContext {
        ExecutionContext { flow_uuid: "test-flow".to_string(), current_node_uuid: None, variables: VariableStorage::new(),
            node_reports: vec![], flow_trigger_time: 0, log_sender: None, flow_file_path: None, ws_manager: None,
            ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None,
            mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None,
            smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None,
            execution_event_sender: None, cancel_receiver: None, providers: HashMap::new() }
    }

    #[tokio::test]
    async fn test_search_provider_error() {
        let m = Arc::new(ImapConnectionManager::new()); let e = ImapSearchExecutor::new(m);
        let n = create_test_node("t1"); let mut c = create_context();
        let r = e.execute(&n, &mut c, &[], &HashMap::new()).await;
        assert!(r.is_ok()); let (nn, d) = r.unwrap(); assert_eq!(nn, vec!["false"]); assert_eq!(d.unwrap()["success"], false);
    }

    #[tokio::test]
    async fn test_search_wrong_type() {
        let m = Arc::new(ImapConnectionManager::new()); let e = ImapSearchExecutor::new(m);
        let n = Node { uuid: "w".to_string(), node_type: "http".to_string(), name: "x".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 }, data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None, has_breakpoint: None };
        let mut c = create_context();
        let r = e.execute(&n, &mut c, &[], &HashMap::new()).await;
        assert!(r.is_err()); assert!(r.unwrap_err().contains("ImapSearchConfig"));
    }
}
