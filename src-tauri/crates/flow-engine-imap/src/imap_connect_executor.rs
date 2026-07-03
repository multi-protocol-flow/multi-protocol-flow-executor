//! IMAP connect node执行器
//!
//! 实现 IMAP 连接建立、variable substitution和输出构建功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::email_types::{EmailAuthConfig, ImapConfig, TlsMode};
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

/// IMAP connect node执行器
pub struct ImapConnectExecutor {
    imap_manager: Arc<dyn ImapProvider>,
}

impl ImapConnectExecutor {
    /// 创建新的 IMAP 连接执行器
    pub fn new(imap_manager: Arc<dyn ImapProvider>) -> Self {
        Self { imap_manager }
    }

    /// 将字符串 `tls_mode` 解析为 `TlsMode` 枚举
    fn parse_tls_mode(tls_mode: &str) -> TlsMode {
        match tls_mode {
            "starttls" => TlsMode::StartTls,
            "none" => TlsMode::None,
            _ => TlsMode::Implicit,
        }
    }

    /// 将 JSON Value 格式的 auth 配置解析为 `EmailAuthConfig`
    fn parse_auth(auth_value: &serde_json::Value) -> Result<Option<EmailAuthConfig>, ErrorDetail> {
        if auth_value.is_null() {
            return Ok(None);
        }
        let auth: EmailAuthConfig =
            serde_json::from_value(auth_value.clone())
                .map_err(|e| ErrorDetail::execution_error(format!("[E_IMAP_CONNECTION] auth config parsefailed: {}", e)))?;
        Ok(Some(auth))
    }

    /// 根据连接查找目标节点 ID 列表
    fn resolve_next_nodes(
        node_uuid: &str,
        port: &str,
        connections: &[Connection],
    ) -> Vec<String> {
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

    /// 构建当前时间戳（毫秒）
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time went backwards")
            .as_millis() as u64
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ImapConnectExecutor {
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
            NodeData::ImapConnect(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("[E_IMAP_CONNECTION] Node data is not ImapConnectConfig: {}", node.uuid))),
        };

        // 2. variable substitution（host）
        let host = replace_variables(&config.host, context).map_err(|e| {
            let error_msg = format!("host variable substitutionfailed: {}", e);
            log::error!(
                "[Flow {}] [Error] nodeID: {}, nodeName: {}, error: {}",
                context.flow_uuid, node.uuid, node.name, error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 解析 TLS 模式
        let tls_mode = Self::parse_tls_mode(&config.tls_mode);

        // 4. [E_IMAP_CONNECTION] auth config parse
        let auth = match &config.auth {
            Some(auth_value) => Self::parse_auth(auth_value)?,
            None => None,
        };

        // 5. 构建 ImapConfig
        let imap_config = ImapConfig {
            host: host.clone(),
            port: config.port,
            tls_mode,
            auth,
        };

        // 6. 日志：开始连接
        log::info!(
            "[Flow {}] [Start] nodeID: {}, nodeName: {}, type: imap_connect, host: {}:{}",
            context.flow_uuid, node.uuid, node.name, imap_config.host, imap_config.port
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("IMAP connect node {} started execution", node.name),
                timestamp: Self::current_timestamp(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 7. 建立连接
        let connect_result = self
            .imap_manager
            .connect(&node.uuid, &imap_config)
            .await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[IMAP] connect succeeded: uuid={}, host={}:{}, connection_id={}",
                        node.uuid, imap_config.host, imap_config.port,
                        output.connection_id.as_deref().unwrap_or("unknown")
                    );
                } else {
                    log::warn!(
                        "[IMAP] connect failed: uuid={}, host={}:{}, error={}",
                        node.uuid, imap_config.host, imap_config.port,
                        output.error.as_deref().unwrap_or("unknown error")
                    );
                }

                // 8. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "connection_id": output.connection_id,
                    "server_greeting": output.server_greeting,
                    "capabilities": output.capabilities,
                    "request": {
                        "host": imap_config.host,
                        "port": imap_config.port,
                        "tls_mode": config.tls_mode,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 9. 日志：连接完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!("IMAP connect succeeded ({}ms)", duration.as_millis())
                } else {
                    format!(
                        "IMAP connect failed: {} ({}ms)",
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

                // 10. 路由：Success走 true 端口，Failed走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = Self::resolve_next_nodes(&node.uuid, port, connections);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[IMAP] connection exception: uuid={}, host={}:{}, error={}",
                    node.uuid, imap_config.host, imap_config.port, e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("IMAP connect exception: {}", e),
                        timestamp: Self::current_timestamp(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "server_greeting": serde_json::Value::Null,
                    "capabilities": [],
                    "request": {
                        "host": imap_config.host,
                        "port": imap_config.port,
                        "tls_mode": config.tls_mode,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("IMAP connect failed: {}", e)));
                }

                let next_node_ids =
                    Self::resolve_next_nodes(&node.uuid, "false", connections);

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
    use flow_engine_core::node_config::ImapConnectConfig;
    use flow_engine_core::traits::ImapConnectOutput;

    /// 模拟 IMAP Provider（连接Success）
    struct MockImapProvider {
        output: ImapConnectOutput,
    }

    impl MockImapProvider {
        fn new_success() -> Self {
            Self {
                output: ImapConnectOutput {
                    success: true,
                    connection_id: Some("imap-conn-1".to_string()),
                    server_greeting: Some("* OK [CAPABILITY IMAP4rev1]".to_string()),
                    capabilities: vec!["IMAP4rev1".to_string(), "IDLE".to_string()],
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: ImapConnectOutput {
                    success: false,
                    connection_id: None,
                    server_greeting: None,
                    capabilities: vec![],
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl ImapProvider for MockImapProvider {
        async fn connect(
            &self,
            _connection_id: &str,
            _config: &ImapConfig,
        ) -> Result<ImapConnectOutput, String> {
            Ok(self.output.clone())
        }

        async fn select(
            &self, _connection_id: &str, _mailbox: &str, _readonly: bool,
        ) -> Result<flow_engine_core::traits::ImapSelectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn search(
            &self, _connection_id: &str, _mailbox: Option<&str>, _criteria: &str,
            _max_results: Option<usize>,
        ) -> Result<flow_engine_core::traits::ImapSearchOutput, String> {
            Err("not implemented".to_string())
        }

        async fn fetch(
            &self, _connection_id: &str, _uids: &str, _fetch_items: &str,
            _max_fetch_count: usize, _max_size_bytes: usize,
        ) -> Result<flow_engine_core::traits::ImapFetchOutput, String> {
            Err("not implemented".to_string())
        }

        async fn store_flags(
            &self, _connection_id: &str, _uids: &str, _operation: &str,
            _flags: &[String],
        ) -> Result<flow_engine_core::traits::ImapStoreFlagsOutput, String> {
            Err("not implemented".to_string())
        }

        async fn copy(
            &self, _connection_id: &str, _uids: &str, _target_mailbox: &str,
        ) -> Result<flow_engine_core::traits::ImapCopyOutput, String> {
            Err("not implemented".to_string())
        }

        async fn r#move(
            &self, _connection_id: &str, _uids: &str, _target_mailbox: &str,
        ) -> Result<flow_engine_core::traits::ImapMoveOutput, String> {
            Err("not implemented".to_string())
        }

        async fn expunge(
            &self, _connection_id: &str,
        ) -> Result<flow_engine_core::traits::ImapExpungeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn list(
            &self, _connection_id: &str, _reference_name: &str, _pattern: &str,
        ) -> Result<flow_engine_core::traits::ImapListOutput, String> {
            Err("not implemented".to_string())
        }

        async fn create_folder(
            &self, _connection_id: &str, _folder_name: &str,
        ) -> Result<flow_engine_core::traits::ImapFolderCreateOutput, String> {
            Err("not implemented".to_string())
        }

        async fn delete_folder(
            &self, _connection_id: &str, _folder_name: &str,
        ) -> Result<flow_engine_core::traits::ImapFolderDeleteOutput, String> {
            Err("not implemented".to_string())
        }

        async fn idle(
            &self, _connection_id: &str, _mailbox: Option<&str>, _timeout_seconds: u64,
        ) -> Result<flow_engine_core::traits::ImapIdleOutput, String> {
            Err("not implemented".to_string())
        }

        async fn close(
            &self, _connection_id: &str,
        ) -> Result<flow_engine_core::traits::ImapCloseOutput, String> {
            Ok(flow_engine_core::traits::ImapCloseOutput {
                success: true,
                error: None,
            })
        }

        async fn test_connection(&self, _config: &ImapConfig) -> Result<bool, String> {
            Ok(true)
        }
    }

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, host: &str, port: u16) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "imap_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::ImapConnect(ImapConnectConfig {
                host: host.to_string(),
                port,
                tls_mode: "implicit".to_string(),
                auth: None,
                connect_timeout_ms: 30000,
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
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：连接Success时路由到 true 端口
    #[tokio::test]
    async fn test_connect_success_routes_to_true() {
        let imap_manager = Arc::new(MockImapProvider::new_success());
        let executor = ImapConnectExecutor::new(imap_manager);
        let node = create_test_node("test-1", "test-connect", "imap.gmail.com", 993);
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
        assert_eq!(data["connection_id"], "imap-conn-1");
        assert!(data["timing"]["connect_ms"].is_number());
    }

    /// 测试：connection failed时路由到 false 端口
    #[tokio::test]
    async fn test_connect_failure_routes_to_false() {
        let imap_manager = Arc::new(MockImapProvider::new_failure("Connection refused"));
        let executor = ImapConnectExecutor::new(imap_manager);
        let node = create_test_node("test-2", "failure-test", "unreachable.host", 993);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Connection refused");
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_connect_wrong_node_data_type() {
        let imap_manager = Arc::new(MockImapProvider::new_success());
        let executor = ImapConnectExecutor::new(imap_manager);

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
        assert!(result.unwrap_err().message.contains("ImapConnectConfig"));
    }

    /// 测试：TLS 模式解析
    #[test]
    fn test_parse_tls_mode() {
        assert_eq!(
            ImapConnectExecutor::parse_tls_mode("implicit"),
            TlsMode::Implicit
        );
        assert_eq!(
            ImapConnectExecutor::parse_tls_mode("starttls"),
            TlsMode::StartTls
        );
        assert_eq!(ImapConnectExecutor::parse_tls_mode("none"), TlsMode::None);
        assert_eq!(ImapConnectExecutor::parse_tls_mode(""), TlsMode::Implicit);
    }
}
