//! IMAP select mailbox节点执行器

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::ImapProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// IMAP select mailbox节点执行器
pub struct ImapSelectExecutor {
    imap_manager: Arc<dyn ImapProvider>,
}

impl ImapSelectExecutor {
    /// 创建新的 IMAP select mailbox执行器
    pub fn new(imap_manager: Arc<dyn ImapProvider>) -> Self {
        Self { imap_manager }
    }

    fn resolve_next_nodes(node_uuid: &str, port: &str, connections: &[Connection]) -> Vec<String> {
        let filtered: Vec<String> = connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node_uuid && conn.source_port_id == port)
            .map(|conn| conn.target_node_uuid.clone())
            .collect();
        if filtered.is_empty() {
            vec![port.to_string()]
        } else {
            filtered
        }
    }

    fn current_timestamp() -> u64 {
        flow_engine_core::utils::unix_timestamp_ms()
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ImapSelectExecutor {
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
            NodeData::ImapSelect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "[E_IMAP_FETCH] Node data is not ImapSelectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. variable substitution
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID variable substitutionfailed: {}", e);
            log::error!(
                "[Flow {}] [Error] nodeID: {}, nodeName: {}, error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;
        let mailbox = replace_variables(&config.mailbox, context).map_err(|e| {
            let error_msg = format!(
                "[E_IMAP_FETCH] mailbox name variable substitutionfailed: {}",
                e
            );
            log::error!(
                "[Flow {}] [Error] nodeID: {}, nodeName: {}, error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志
        log::info!(
            "[Flow {}] [Start] nodeID: {}, nodeName: {}, type: imap_select, mailbox: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            mailbox
        );
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("IMAP Select 节点 {} started execution", node.name),
                timestamp: Self::current_timestamp(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 调用 select
        let result = self
            .imap_manager
            .select(&connection_id, &mailbox, config.readonly)
            .await;

        let duration = start_time.elapsed();

        match result {
            Ok(output) => {
                let success = output.success;
                let port = if success { "true" } else { "false" };

                let output_data = json!({
                    "success": success,
                    "mailbox": output.mailbox,
                    "exists": output.exists,
                    "recent": output.recent,
                    "unseen": output.unseen,
                    "uid_validity": output.uid_validity,
                    "uid_next": output.uid_next,
                    "request": {
                        "connection_id": connection_id,
                        "mailbox": mailbox,
                        "readonly": config.readonly,
                    },
                    "timing": {
                        "select_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                let log_level = if success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if success {
                    format!(
                        "IMAP select mailbox {} succeeded ({}ms)",
                        mailbox,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "IMAP select mailbox {} failed: {} ({}ms)",
                        mailbox,
                        output.error.as_deref().unwrap_or("unknown error"),
                        duration.as_millis()
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_message,
                        timestamp: Self::current_timestamp(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let next_node_ids = Self::resolve_next_nodes(&node.uuid, port, connections);
                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[IMAP] Select exception: connection_id={}, mailbox={}, error={}",
                    connection_id,
                    mailbox,
                    e
                );
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("IMAP select mailbox exception: {}", e),
                        timestamp: Self::current_timestamp(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "mailbox": mailbox,
                    "exists": 0,
                    "recent": 0,
                    "request": {
                        "connection_id": connection_id,
                        "mailbox": mailbox,
                        "readonly": config.readonly,
                    },
                    "timing": {
                        "select_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });
                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "IMAP select failed: {}",
                        e
                    )));
                }

                let next_node_ids = Self::resolve_next_nodes(&node.uuid, "false", connections);
                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::ImapSelectConfig;
    use flow_engine_core::traits::ImapSelectOutput;

    /// 模拟 IMAP Provider（select Success）
    struct MockImapProvider {
        output: ImapSelectOutput,
    }

    impl MockImapProvider {
        fn new_success() -> Self {
            Self {
                output: ImapSelectOutput {
                    success: true,
                    mailbox: "INBOX".to_string(),
                    exists: 42,
                    recent: 3,
                    unseen: Some(5),
                    uid_validity: Some(1234567890),
                    uid_next: Some(43),
                    error: None,
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl ImapProvider for MockImapProvider {
        async fn connect(
            &self,
            _connection_id: &str,
            _config: &flow_engine_core::email_types::ImapConfig,
        ) -> Result<flow_engine_core::traits::ImapConnectOutput, String> {
            Err("not implemented".to_string())
        }
        async fn select(
            &self,
            _connection_id: &str,
            _mailbox: &str,
            _readonly: bool,
        ) -> Result<ImapSelectOutput, String> {
            Ok(self.output.clone())
        }
        async fn search(
            &self,
            _connection_id: &str,
            _mailbox: Option<&str>,
            _criteria: &str,
            _max_results: Option<usize>,
        ) -> Result<flow_engine_core::traits::ImapSearchOutput, String> {
            Err("not implemented".to_string())
        }
        async fn fetch(
            &self,
            _connection_id: &str,
            _uids: &str,
            _fetch_items: &str,
            _max_fetch_count: usize,
            _max_size_bytes: usize,
        ) -> Result<flow_engine_core::traits::ImapFetchOutput, String> {
            Err("not implemented".to_string())
        }
        async fn store_flags(
            &self,
            _connection_id: &str,
            _uids: &str,
            _operation: &str,
            _flags: &[String],
        ) -> Result<flow_engine_core::traits::ImapStoreFlagsOutput, String> {
            Err("not implemented".to_string())
        }
        async fn copy(
            &self,
            _connection_id: &str,
            _uids: &str,
            _target_mailbox: &str,
        ) -> Result<flow_engine_core::traits::ImapCopyOutput, String> {
            Err("not implemented".to_string())
        }
        async fn r#move(
            &self,
            _connection_id: &str,
            _uids: &str,
            _target_mailbox: &str,
        ) -> Result<flow_engine_core::traits::ImapMoveOutput, String> {
            Err("not implemented".to_string())
        }
        async fn expunge(
            &self,
            _connection_id: &str,
        ) -> Result<flow_engine_core::traits::ImapExpungeOutput, String> {
            Err("not implemented".to_string())
        }
        async fn list(
            &self,
            _connection_id: &str,
            _reference_name: &str,
            _pattern: &str,
        ) -> Result<flow_engine_core::traits::ImapListOutput, String> {
            Err("not implemented".to_string())
        }
        async fn create_folder(
            &self,
            _connection_id: &str,
            _folder_name: &str,
        ) -> Result<flow_engine_core::traits::ImapFolderCreateOutput, String> {
            Err("not implemented".to_string())
        }
        async fn delete_folder(
            &self,
            _connection_id: &str,
            _folder_name: &str,
        ) -> Result<flow_engine_core::traits::ImapFolderDeleteOutput, String> {
            Err("not implemented".to_string())
        }
        async fn idle(
            &self,
            _connection_id: &str,
            _mailbox: Option<&str>,
            _timeout_seconds: u64,
        ) -> Result<flow_engine_core::traits::ImapIdleOutput, String> {
            Err("not implemented".to_string())
        }
        async fn close(
            &self,
            _connection_id: &str,
        ) -> Result<flow_engine_core::traits::ImapCloseOutput, String> {
            Ok(flow_engine_core::traits::ImapCloseOutput {
                success: true,
                error: None,
            })
        }
        async fn test_connection(
            &self,
            _config: &flow_engine_core::email_types::ImapConfig,
        ) -> Result<bool, String> {
            Ok(true)
        }
    }

    fn create_test_node(uuid: &str, name: &str, connection_id: &str, mailbox: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "imap_select".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::ImapSelect(ImapSelectConfig {
                connection_id: connection_id.to_string(),
                mailbox: mailbox.to_string(),
                readonly: false,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
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

    #[tokio::test]
    async fn test_select_success() {
        let imap_manager = Arc::new(MockImapProvider::new_success());
        let executor = ImapSelectExecutor::new(imap_manager);
        let node = create_test_node("test-1", "select-mailbox", "conn-1", "INBOX");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], true);
        assert_eq!(data["exists"], 42);
    }

    #[tokio::test]
    async fn test_select_wrong_node_data_type() {
        let imap_manager = Arc::new(MockImapProvider::new_success());
        let executor = ImapSelectExecutor::new(imap_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "wrong-type".to_string(),
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
            .contains("ImapSelectConfig"));
    }
}
