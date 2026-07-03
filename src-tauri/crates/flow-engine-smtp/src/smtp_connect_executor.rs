//! SMTP 连接节点执行器
//!
//! 实现 SMTP 连接建立、变量替换和输出构建功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::email_types::{EmailAuthConfig, SmtpConfig, TlsMode};
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::SmtpProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// SMTP 连接节点执行器
pub struct SmtpConnectExecutor {
    smtp_manager: Arc<dyn SmtpProvider>,
}

impl SmtpConnectExecutor {
    /// 创建新的 SMTP 连接执行器
    pub fn new(smtp_manager: Arc<dyn SmtpProvider>) -> Self {
        Self { smtp_manager }
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
                .map_err(|e| ErrorDetail::execution_error(format!("解析认证配置Failed: {}", e)))?;
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
            .expect("系统时间Error")
            .as_millis() as u64
    }
}

#[async_trait::async_trait]
impl NodeExecutor for SmtpConnectExecutor {
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
            NodeData::SmtpConnect(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not SmtpConnectConfig: {}", node.uuid))),
        };

        // 2. 变量替换（host）
        let host = replace_variables(&config.host, context).map_err(|e| {
            let error_msg = format!("host variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 解析 TLS 模式
        let tls_mode = Self::parse_tls_mode(&config.tls_mode);

        // 4. 解析认证配置
        let auth = match &config.auth {
            Some(auth_value) => Self::parse_auth(auth_value)?,
            None => None,
        };

        // 5. 构建 SmtpConfig
        let smtp_config = SmtpConfig {
            host: host.clone(),
            port: config.port,
            tls_mode,
            auth,
        };

        // 6. 日志：开始连接
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: smtp_connect, host: {}:{}",
            context.flow_uuid,
            node.uuid,
            node.name,
            smtp_config.host,
            smtp_config.port
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("SMTP 连接节点 {} start execution", node.name),
                timestamp: Self::current_timestamp(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 7. 建立连接
        let connect_result = self
            .smtp_manager
            .connect(&node.uuid, &smtp_config)
            .await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[SMTP] 连接Success: uuid={}, host={}:{}, connection_id={}",
                        node.uuid,
                        smtp_config.host,
                        smtp_config.port,
                        output.connection_id.as_deref().unwrap_or("unknown")
                    );
                } else {
                    log::warn!(
                        "[SMTP] connection failed: uuid={}, host={}:{}, error={}",
                        node.uuid,
                        smtp_config.host,
                        smtp_config.port,
                        output.error.as_deref().unwrap_or("未知Error")
                    );
                }

                // 8. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "connection_id": output.connection_id,
                    "server_greeting": output.server_greeting,
                    "request": {
                        "host": smtp_config.host,
                        "port": smtp_config.port,
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
                    format!(
                        "SMTP 连接Success ({}ms)",
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "SMTP connection failed: {} ({}ms)",
                        output.error.as_deref().unwrap_or("未知Error"),
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
                    "[SMTP] connection error: uuid={}, host={}:{}, error={}",
                    node.uuid,
                    smtp_config.host,
                    smtp_config.port,
                    e
                );

                // 日志：connection error
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("SMTP connection error: {}", e),
                        timestamp: Self::current_timestamp(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "server_greeting": serde_json::Value::Null,
                    "request": {
                        "host": smtp_config.host,
                        "port": smtp_config.port,
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
                    return Err(ErrorDetail::execution_error(format!("SMTP connect Failed: {}", e)));
                }

                // 返回 false 端口
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
    use flow_engine_core::node_config::SmtpConnectConfig;
    use flow_engine_core::traits::SmtpConnectOutput;

    /// 模拟 SMTP Provider（连接Success）
    struct MockSmtpProvider {
        output: SmtpConnectOutput,
    }

    impl MockSmtpProvider {
        fn new_success() -> Self {
            Self {
                output: SmtpConnectOutput {
                    success: true,
                    connection_id: Some("conn-1".to_string()),
                    server_greeting: Some("220 smtp.example.com ESMTP".to_string()),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: SmtpConnectOutput {
                    success: false,
                    connection_id: None,
                    server_greeting: None,
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl SmtpProvider for MockSmtpProvider {
        async fn connect(
            &self,
            _connection_id: &str,
            _config: &SmtpConfig,
        ) -> Result<SmtpConnectOutput, String> {
            Ok(self.output.clone())
        }

        async fn send(
            &self,
            _connection_id: &str,
            _from: &str,
            _to: &[String],
            _cc: &[String],
            _bcc: &[String],
            _subject: &str,
            _body_type: &str,
            _body: &str,
            _html_body: &str,
            _attachments: &[serde_json::Value],
        ) -> Result<flow_engine_core::traits::SmtpSendOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _connection_id: &str,
        ) -> Result<flow_engine_core::traits::SmtpDisconnectOutput, String> {
            Ok(flow_engine_core::traits::SmtpDisconnectOutput {
                success: true,
                error: None,
            })
        }

        async fn test_connection(&self, _config: &SmtpConfig) -> Result<bool, String> {
            Ok(true)
        }
    }

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, host: &str, port: u16) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "smtp_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SmtpConnect(SmtpConnectConfig {
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
                .expect("系统时间Error")
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

    /// 测试：连接Success时输出 JSON 结构正确
    #[tokio::test]
    async fn test_connect_success_output_structure() {
        let smtp_manager = Arc::new(MockSmtpProvider::new_success());
        let executor = SmtpConnectExecutor::new(smtp_manager);
        let node = create_test_node("test-1", "测试连接", "smtp.example.com", 465);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // Success应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "conn-1");
        assert_eq!(data["server_greeting"], "220 smtp.example.com ESMTP");
        // timing 字段应存在
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        // request 字段应包含连接信息
        assert_eq!(data["request"]["host"], "smtp.example.com");
        assert_eq!(data["request"]["port"], 465);
    }

    /// 测试：connection failed时路由到 false 端口
    #[tokio::test]
    async fn test_connect_failure_routes_to_false_port() {
        let smtp_manager = Arc::new(MockSmtpProvider::new_failure("Connection refused"));
        let executor = SmtpConnectExecutor::new(smtp_manager);
        let node = create_test_node("test-2", "Failed测试", "unreachable.host", 465);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // Failed应路由到 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Connection refused");
        assert!(data["connection_id"].is_null());
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_connect_wrong_node_data_type() {
        let smtp_manager = Arc::new(MockSmtpProvider::new_success());
        let executor = SmtpConnectExecutor::new(smtp_manager);

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
        assert!(result.unwrap_err().message.contains("SmtpConnectConfig"));
    }

    /// 测试：变量替换在 host 中生效
    #[tokio::test]
    async fn test_connect_with_variable_replacement() {
        let smtp_manager = Arc::new(MockSmtpProvider::new_success());
        let executor = SmtpConnectExecutor::new(smtp_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "smtp_connect".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SmtpConnect(SmtpConnectConfig {
                host: "smtp.{{domain}}".to_string(),
                port: 465,
                tls_mode: "implicit".to_string(),
                auth: None,
                connect_timeout_ms: 30000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "domain".to_string(),
            json!("example.com"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(data["request"]["host"], "smtp.example.com");
    }

    /// 测试：连接配置中的连接端口路由
    #[tokio::test]
    async fn test_connect_with_connections_routing() {
        let smtp_manager = Arc::new(MockSmtpProvider::new_success());
        let executor = SmtpConnectExecutor::new(smtp_manager);
        let node = create_test_node("test-route", "路由测试", "smtp.example.com", 465);
        let mut context = create_context("test-flow");

        // 创建 true 端口连接到下一个节点
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-route".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        // 应路由到连接中指定的目标节点
        assert_eq!(next_nodes, vec!["next-node"]);
    }

    /// 测试：TLS 模式解析
    #[test]
    fn test_parse_tls_mode() {
        assert_eq!(SmtpConnectExecutor::parse_tls_mode("implicit"), TlsMode::Implicit);
        assert_eq!(SmtpConnectExecutor::parse_tls_mode("starttls"), TlsMode::StartTls);
        assert_eq!(SmtpConnectExecutor::parse_tls_mode("none"), TlsMode::None);
        // 默认值
        assert_eq!(SmtpConnectExecutor::parse_tls_mode(""), TlsMode::Implicit);
        assert_eq!(SmtpConnectExecutor::parse_tls_mode("unknown"), TlsMode::Implicit);
    }

    /// 测试：Provider 抛出异常时路由到 false 端口
    #[tokio::test]
    async fn test_connect_provider_error_routes_to_false() {
        struct ErrorSmtpProvider;

        #[async_trait::async_trait]
        impl SmtpProvider for ErrorSmtpProvider {
            async fn connect(
                &self,
                _connection_id: &str,
                _config: &SmtpConfig,
            ) -> Result<SmtpConnectOutput, String> {
                Err("网络不可达".to_string())
            }

            async fn send(
                &self,
                _connection_id: &str,
                _from: &str,
                _to: &[String],
                _cc: &[String],
                _bcc: &[String],
                _subject: &str,
                _body_type: &str,
                _body: &str,
                _html_body: &str,
                _attachments: &[serde_json::Value],
            ) -> Result<flow_engine_core::traits::SmtpSendOutput, String> {
                Err("not implemented".to_string())
            }

            async fn disconnect(
                &self,
                _connection_id: &str,
            ) -> Result<flow_engine_core::traits::SmtpDisconnectOutput, String> {
                Ok(flow_engine_core::traits::SmtpDisconnectOutput {
                    success: true,
                    error: None,
                })
            }

            async fn test_connection(&self, _config: &SmtpConfig) -> Result<bool, String> {
                Ok(false)
            }
        }

        let smtp_manager = Arc::new(ErrorSmtpProvider);
        let executor = SmtpConnectExecutor::new(smtp_manager);
        let node = create_test_node("test-err", "异常测试", "bad.host", 465);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "网络不可达");
    }
}
