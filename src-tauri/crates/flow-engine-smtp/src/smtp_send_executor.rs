//! SMTP 发送邮件节点执行器
//!
//! 实现 SMTP 邮件发送功能。
//! 参照 `mqtt_publish_executor.rs` 模式实现。
//! 从配置中提取收件人、正文等信息，执行变量替换，委托给 SmtpProvider::send()。

use std::collections::HashMap;
use flow_engine_core::error::ErrorDetail;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::SmtpProvider;
use serde_json::json;

/// SMTP 发送邮件节点执行器
pub struct SmtpSendExecutor {
    smtp_manager: Arc<dyn SmtpProvider>,
}

impl SmtpSendExecutor {
    /// 创建新的 SMTP 发送执行器
    pub fn new(smtp_manager: Arc<dyn SmtpProvider>) -> Self {
        Self { smtp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for SmtpSendExecutor {
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
            NodeData::SmtpSend(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not SmtpSendConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换: connection_id
        let connection_id =
            replace_variables(&config.connection_id, context).map_err(|e| {
                let error_msg = format!("Connection ID variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?;

        // 3. 变量替换: from
        let from = match &config.from {
            Some(f) => Some(replace_variables(f, context).map_err(|e| {
                let error_msg = format!("From address variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        // 4. 变量替换: to (数组)
        let to: Vec<String> = config
            .to
            .iter()
            .map(|addr| replace_variables(addr, context))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                let error_msg = format!("Recipient variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?;

        // 5. 变量替换: cc (数组)
        let cc: Vec<String> = match &config.cc {
            Some(addrs) => addrs
                .iter()
                .map(|addr| replace_variables(addr, context))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    let error_msg = format!("CC variable substitution failed: {}", e);
                    log::error!(
                        "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                        context.flow_uuid,
                        node.uuid,
                        node.name,
                        error_msg
                    );
                    ErrorDetail::execution_error(error_msg)
                })?,
            None => vec![],
        };

        // 6. 变量替换: bcc (数组)
        let bcc: Vec<String> = match &config.bcc {
            Some(addrs) => addrs
                .iter()
                .map(|addr| replace_variables(addr, context))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    let error_msg = format!("BCC variable substitution failed: {}", e);
                    log::error!(
                        "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                        context.flow_uuid,
                        node.uuid,
                        node.name,
                        error_msg
                    );
                    ErrorDetail::execution_error(error_msg)
                })?,
            None => vec![],
        };

        // 7. 变量替换: subject
        let subject = match &config.subject {
            Some(s) => replace_variables(s, context).map_err(|e| {
                let error_msg = format!("Subject variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?,
            None => String::new(),
        };

        // 8. 变量替换: body
        let body = match &config.body {
            Some(b) => replace_variables(b, context).map_err(|e| {
                let error_msg = format!("Body variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?,
            None => String::new(),
        };

        // 9. 变量替换: html_body
        let html_body = match &config.html_body {
            Some(h) => replace_variables(h, context).map_err(|e| {
                let error_msg = format!("HTML body variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?,
            None => String::new(),
        };

        // 10. 附件（不做变量替换，直接透传）
        let attachments: Vec<serde_json::Value> =
            config.attachments.clone().unwrap_or_default();

        // 11. 日志：开始发送
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: smtp_send, Connection ID: {}, recipients: {:?}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            to
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "SMTP send node {} start execution (recipients: {})",
                    node.name,
                    to.join(", ")
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 12. 基本参数校验
        if from.as_ref().is_none_or(|f| f.is_empty()) {
            let duration = start_time.elapsed();
            let error_msg = "Sender address is required";
            log::error!(
                "[SMTP] Send failed: connection_id={}, error={}",
                connection_id,
                error_msg
            );

            context
                .emit_log(LogEvent {
                    level: LogLevel::Error,
                    message: format!("SMTP send failed: {}", error_msg),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("system time error")
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            let error_output = json!({
                "success": false,
                "message_id": null,
                "response_code": null,
                "error": error_msg,
                "request": {
                    "connection_id": connection_id,
                    "from": from,
                    "to": to,
                    "cc": cc,
                    "bcc": bcc,
                    "subject": subject,
                    "body_type": config.body_type,
                },
                "timing": {
                    "total_ms": duration.as_millis() as u64,
                },
            });

            return route_to_false(node, connections, error_output);
        }

        if to.is_empty() {
            let duration = start_time.elapsed();
            let error_msg = "Recipient list cannot be empty";
            log::error!(
                "[SMTP] Send failed: connection_id={}, error={}",
                connection_id,
                error_msg
            );

            context
                .emit_log(LogEvent {
                    level: LogLevel::Error,
                    message: format!("SMTP send failed: {}", error_msg),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("system time error")
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            let error_output = json!({
                "success": false,
                "message_id": null,
                "response_code": null,
                "error": error_msg,
                "request": {
                    "connection_id": connection_id,
                    "from": from,
                    "to": to,
                    "cc": cc,
                    "bcc": bcc,
                    "subject": subject,
                    "body_type": config.body_type,
                },
                "timing": {
                    "total_ms": duration.as_millis() as u64,
                },
            });

            return route_to_false(node, connections, error_output);
        }

        // 13. 调用 SmtpProvider::send()
        let from_str = from.as_deref().unwrap_or("");
        let body_type = if config.body_type.is_empty() {
            "plain_text"
        } else {
            &config.body_type
        };

        let send_result = self
            .smtp_manager
            .send(
                &connection_id,
                from_str,
                &to,
                &cc,
                &bcc,
                &subject,
                body_type,
                &body,
                &html_body,
                &attachments,
            )
            .await;

        match send_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[SMTP] Email sent: connection_id={}, message_id={:?} ({}ms)",
                        connection_id,
                        output.message_id,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[SMTP] Email send returned error: connection_id={}, error={:?}",
                        connection_id,
                        output.error
                    );
                }

                // 14. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "message_id": output.message_id,
                    "response_code": output.response_code,
                    "error": output.error,
                    "request": {
                        "connection_id": connection_id,
                        "from": from,
                        "to": to,
                        "cc": cc,
                        "bcc": bcc,
                        "subject": subject,
                        "body_type": config.body_type,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 15. 日志：发送完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_msg = if output.success {
                    format!(
                        "SMTP email sent: {} → {:?} ({}ms)",
                        from_str,
                        to,
                        duration.as_millis()
                    )
                } else {
                    let err = output.error.as_deref().unwrap_or("Unknown error");
                    format!("SMTP email send failed: {} - {}", subject, err)
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_msg,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("system time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 16. 端口路由
                if output.success {
                    route_to_true(node, connections, output_data)
                } else {
                    route_to_false(node, connections, output_data)
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[SMTP] Email send failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("SMTP email send failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("system time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "message_id": null,
                    "response_code": null,
                    "error": e,
                    "request": {
                        "connection_id": connection_id,
                        "from": from,
                        "to": to,
                        "cc": cc,
                        "bcc": bcc,
                        "subject": subject,
                        "body_type": config.body_type,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("SMTP send Failed: {}", e)));
                }

                route_to_false(node, connections, error_output)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 端口路由辅助函数
// ---------------------------------------------------------------------------

/// Success路由到 true 端口
fn route_to_true(
    node: &Node,
    connections: &[Connection],
    output_data: serde_json::Value,
) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "true")
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    let next_node_ids = if filtered.is_empty() {
        vec!["true".to_string()]
    } else {
        filtered
    };

    Ok((next_node_ids, Some(output_data)))
}

/// Failed路由到 false 端口
fn route_to_false(
    node: &Node,
    connections: &[Connection],
    output_data: serde_json::Value,
) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "false")
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    let next_node_ids = if filtered.is_empty() {
        vec!["false".to_string()]
    } else {
        filtered
    };

    Ok((next_node_ids, Some(output_data)))
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::SmtpSendConfig;
    use flow_engine_core::traits::{
        SmtpConnectOutput, SmtpDisconnectOutput, SmtpSendOutput,
    };
    use flow_engine_core::email_types::SmtpConfig;
    use crate::smtp_connection_manager::SmtpConnectionManager;

    /// 创建Test Node
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        from: Option<&str>,
        to: Vec<&str>,
        cc: Option<Vec<&str>>,
        bcc: Option<Vec<&str>>,
        subject: Option<&str>,
        body_type: &str,
        body: Option<&str>,
        html_body: Option<&str>,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "smtp_send".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SmtpSend(SmtpSendConfig {
                connection_id: connection_id.to_string(),
                from: from.map(|s| s.to_string()),
                to: to.iter().map(|s| s.to_string()).collect(),
                cc: cc.map(|v| v.iter().map(|s| s.to_string()).collect()),
                bcc: bcc.map(|v| v.iter().map(|s| s.to_string()).collect()),
                subject: subject.map(|s| s.to_string()),
                body_type: body_type.to_string(),
                body: body.map(|s| s.to_string()),
                html_body: html_body.map(|s| s.to_string()),
                attachments: None,
                timeout_ms: 30000,
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
                .expect("system time error")
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

    /// 测试：发送到不存在的连接 → 走 false 端口
    #[tokio::test]
    async fn test_send_nonexistent_connection() {
        let smtp_manager = Arc::new(SmtpConnectionManager::new());
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-1",
            "测试发送",
            "nonexistent-conn",
            Some("from@test.com"),
            vec!["to@test.com"],
            None,
            None,
            Some("Test Subject"),
            "plain_text",
            Some("Hello"),
            None,
        );
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
        assert!(data["error"].is_string());
    }

    /// 测试：缺少发件人 → 走 false 端口
    #[tokio::test]
    async fn test_send_missing_from() {
        let smtp_manager = Arc::new(SmtpConnectionManager::new());
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-no-from",
            "无发件人",
            "conn-1",
            None,
            vec!["to@test.com"],
            None,
            None,
            Some("Test"),
            "plain_text",
            Some("Hello"),
            None,
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().unwrap().contains("Sender"));
    }

    /// 测试：空收件人列表 → 走 false 端口
    #[tokio::test]
    async fn test_send_empty_to() {
        let smtp_manager = Arc::new(SmtpConnectionManager::new());
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-empty-to",
            "空收件人",
            "conn-1",
            Some("from@test.com"),
            vec![],
            None,
            None,
            Some("Test"),
            "plain_text",
            Some("Hello"),
            None,
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().unwrap().contains("Recipient"));
    }

    /// 测试：变量替换
    #[tokio::test]
    async fn test_send_with_variable_replacement() {
        let smtp_manager = Arc::new(SmtpConnectionManager::new());
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-var",
            "变量替换测试",
            "{{conn_id}}",
            Some("{{sender}}@test.com"),
            vec!["{{recipient}}@test.com"],
            Some(vec!["{{cc_user}}@test.com"]),
            Some(vec!["{{bcc_user}}@test.com"]),
            Some("Report: {{report_name}}"),
            "plain_text",
            Some("Hello {{user_name}}"),
            None,
        );

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("smtp-conn-123"),
            String::new(),
        );
        context.variables.set_variable(
            "sender".to_string(),
            json!("alice"),
            String::new(),
        );
        context.variables.set_variable(
            "recipient".to_string(),
            json!("bob"),
            String::new(),
        );
        context.variables.set_variable(
            "cc_user".to_string(),
            json!("charlie"),
            String::new(),
        );
        context.variables.set_variable(
            "bcc_user".to_string(),
            json!("dave"),
            String::new(),
        );
        context.variables.set_variable(
            "report_name".to_string(),
            json!("Q4 财报"),
            String::new(),
        );
        context.variables.set_variable(
            "user_name".to_string(),
            json!("Bob"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]); // 连接不存在，走 false
        let data = output_data.unwrap();
        // 验证变量替换生效
        assert_eq!(data["request"]["connection_id"], "smtp-conn-123");
        assert_eq!(data["request"]["from"], "alice@test.com");
        assert_eq!(data["request"]["to"][0], "bob@test.com");
        assert_eq!(data["request"]["cc"][0], "charlie@test.com");
        assert_eq!(data["request"]["bcc"][0], "dave@test.com");
        assert_eq!(data["request"]["subject"], "Report: Q4 财报");
    }

    /// 测试：Error的节点数据类型 → Err
    #[tokio::test]
    async fn test_send_wrong_node_data_type() {
        let smtp_manager = Arc::new(SmtpConnectionManager::new());
        let executor = SmtpSendExecutor::new(smtp_manager);

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
        assert!(result.unwrap_err().message.contains("SmtpSendConfig"));
    }

    // ===== Mock Success场景 =====

    /// 模拟 SMTP Provider（send Success）
    struct MockSmtpProvider;

    #[async_trait::async_trait]
    impl SmtpProvider for MockSmtpProvider {
        async fn connect(
            &self,
            _connection_id: &str,
            _config: &SmtpConfig,
        ) -> Result<SmtpConnectOutput, String> {
            Err("not implemented".to_string())
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
        ) -> Result<SmtpSendOutput, String> {
            Ok(SmtpSendOutput {
                success: true,
                message_id: Some("<msg123@smtp.test>".to_string()),
                response_code: Some(250),
                error: None,
            })
        }

        async fn disconnect(
            &self,
            _connection_id: &str,
        ) -> Result<SmtpDisconnectOutput, String> {
            Ok(SmtpDisconnectOutput {
                success: true,
                error: None,
            })
        }

        async fn test_connection(&self, _config: &SmtpConfig) -> Result<bool, String> {
            Ok(true)
        }
    }

    /// 模拟 SMTP Provider（send Failed）
    struct MockSmtpProviderFail;

    #[async_trait::async_trait]
    impl SmtpProvider for MockSmtpProviderFail {
        async fn connect(
            &self,
            _connection_id: &str,
            _config: &SmtpConfig,
        ) -> Result<SmtpConnectOutput, String> {
            Err("not implemented".to_string())
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
        ) -> Result<SmtpSendOutput, String> {
            Ok(SmtpSendOutput {
                success: false,
                message_id: None,
                response_code: None,
                error: Some("模拟发送Failed".to_string()),
            })
        }

        async fn disconnect(
            &self,
            _connection_id: &str,
        ) -> Result<SmtpDisconnectOutput, String> {
            Ok(SmtpDisconnectOutput {
                success: true,
                error: None,
            })
        }

        async fn test_connection(&self, _config: &SmtpConfig) -> Result<bool, String> {
            Ok(true)
        }
    }

    /// 模拟 SMTP Provider（send 抛出Error）
    struct MockSmtpProviderError;

    #[async_trait::async_trait]
    impl SmtpProvider for MockSmtpProviderError {
        async fn connect(
            &self,
            _connection_id: &str,
            _config: &SmtpConfig,
        ) -> Result<SmtpConnectOutput, String> {
            Err("not implemented".to_string())
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
        ) -> Result<SmtpSendOutput, String> {
            Err("连接超时".to_string())
        }

        async fn disconnect(
            &self,
            _connection_id: &str,
        ) -> Result<SmtpDisconnectOutput, String> {
            Ok(SmtpDisconnectOutput {
                success: true,
                error: None,
            })
        }

        async fn test_connection(&self, _config: &SmtpConfig) -> Result<bool, String> {
            Ok(true)
        }
    }

    /// 测试：Mock Success发送 → 走 true 端口
    #[tokio::test]
    async fn test_send_success_with_mock() {
        let smtp_manager = Arc::new(MockSmtpProvider);
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-ok",
            "Success发送",
            "mock-conn",
            Some("from@test.com"),
            vec!["to@test.com"],
            None,
            None,
            Some("Test Subject"),
            "plain_text",
            Some("Hello World"),
            None,
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["message_id"], "<msg123@smtp.test>");
        assert_eq!(data["response_code"], 250);
        assert!(data["timing"]["total_ms"].is_number());
    }

    /// 测试：Mock send 返回Failed → 走 false 端口
    #[tokio::test]
    async fn test_send_failure_with_mock() {
        let smtp_manager = Arc::new(MockSmtpProviderFail);
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-fail",
            "发送Failed",
            "mock-conn",
            Some("from@test.com"),
            vec!["to@test.com"],
            None,
            None,
            Some("Test"),
            "plain_text",
            Some("Hello"),
            None,
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "模拟发送Failed");
    }

    /// 测试：Mock send 抛出 Err → 走 false 端口
    #[tokio::test]
    async fn test_send_error_with_mock() {
        let smtp_manager = Arc::new(MockSmtpProviderError);
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-error",
            "发送Error",
            "mock-conn",
            Some("from@test.com"),
            vec!["to@test.com"],
            None,
            None,
            Some("Test"),
            "plain_text",
            Some("Hello"),
            None,
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "连接超时");
    }

    /// 测试：输出 JSON 结构完整性
    #[tokio::test]
    async fn test_send_output_json_structure() {
        let smtp_manager = Arc::new(MockSmtpProvider);
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-structure",
            "结构测试",
            "mock-conn",
            Some("from@test.com"),
            vec!["to@test.com", "to2@test.com"],
            Some(vec!["cc@test.com"]),
            Some(vec!["bcc@test.com"]),
            Some("完整结构测试"),
            "html",
            None,
            Some("<p>Hello</p>"),
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        // 验证所有预期字段存在
        assert!(data.get("success").is_some(), "应有 success 字段");
        assert!(data.get("message_id").is_some(), "应有 message_id 字段");
        assert!(data.get("response_code").is_some(), "应有 response_code 字段");
        assert!(data.get("timing").is_some(), "应有 timing 字段");
        assert!(data.get("request").is_some(), "应有 request 字段");
        assert!(
            data["request"]["connection_id"].is_string(),
            "request 中应有 connection_id"
        );
        assert!(
            data["request"]["from"].is_string(),
            "request 中应有 from"
        );
        assert!(
            data["request"]["to"].is_array(),
            "request 中应有 to 数组"
        );
        assert_eq!(data["request"]["to"].as_array().unwrap().len(), 2);
        assert!(
            data["request"]["cc"].is_array(),
            "request 中应有 cc 数组"
        );
        assert!(
            data["request"]["bcc"].is_array(),
            "request 中应有 bcc 数组"
        );
        assert!(
            data["request"]["subject"].is_string(),
            "request 中应有 subject"
        );
        assert!(
            data["request"]["body_type"].is_string(),
            "request 中应有 body_type"
        );
    }

    /// 测试：默认 body_type 应为 "plain_text"
    #[tokio::test]
    async fn test_send_default_body_type() {
        let smtp_manager = Arc::new(MockSmtpProvider);
        let executor = SmtpSendExecutor::new(smtp_manager);
        let node = create_test_node(
            "test-default-body",
            "默认正文类型",
            "mock-conn",
            Some("from@test.com"),
            vec!["to@test.com"],
            None,
            None,
            Some("Test"),
            "", // 空 body_type
            Some("Hello"),
            None,
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
    }

    // ===== 端口路由测试 =====

    #[test]
    fn test_route_to_true_with_connection() {
        use flow_engine_core::connection::Connection;

        let node = Node {
            uuid: "node-1".to_string(),
            node_type: "smtp_send".to_string(),
            name: "test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SmtpSend(SmtpSendConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "node-1".to_string(),
            target_node_uuid: "node-2".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = route_to_true(&node, &connections, json!({"success": true}));
        assert!(result.is_ok());
        let (ids, _) = result.unwrap();
        assert_eq!(ids, vec!["node-2"]);
    }

    #[test]
    fn test_route_to_true_no_connection() {
        let node = Node {
            uuid: "node-1".to_string(),
            node_type: "smtp_send".to_string(),
            name: "test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SmtpSend(SmtpSendConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let result = route_to_true(&node, &[], json!({"success": true}));
        assert!(result.is_ok());
        let (ids, _) = result.unwrap();
        assert_eq!(ids, vec!["true"]);
    }

    #[test]
    fn test_route_to_false_with_connection() {
        use flow_engine_core::connection::Connection;

        let node = Node {
            uuid: "node-1".to_string(),
            node_type: "smtp_send".to_string(),
            name: "test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SmtpSend(SmtpSendConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "node-1".to_string(),
            target_node_uuid: "node-3".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = route_to_false(&node, &connections, json!({"success": false}));
        assert!(result.is_ok());
        let (ids, _) = result.unwrap();
        assert_eq!(ids, vec!["node-3"]);
    }

    #[test]
    fn test_route_to_false_no_connection() {
        let node = Node {
            uuid: "node-1".to_string(),
            node_type: "smtp_send".to_string(),
            name: "test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SmtpSend(SmtpSendConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let result = route_to_false(&node, &[], json!({"success": false}));
        assert!(result.is_ok());
        let (ids, _) = result.unwrap();
        assert_eq!(ids, vec!["false"]);
    }
}
