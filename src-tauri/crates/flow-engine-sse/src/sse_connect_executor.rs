//! SSE 连接节点执行器
//!
//! 实现 SSE 连接建立、变量替换和输出构建功能。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{NodeData, SseConnectConfig};
use flow_engine_core::traits::SseProvider;
use serde_json::json;

/// SSE 连接节点执行器
pub struct SseConnectExecutor {
    sse_manager: Arc<dyn SseProvider>,
}

impl SseConnectExecutor {
    /// 创建新的 SSE 连接执行器
    pub fn new(sse_manager: Arc<dyn SseProvider>) -> Self {
        Self { sse_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for SseConnectExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::SseConnect(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 SseConnectConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换（url, headers, last_event_id）
        let url = replace_variables(&config.url, context).map_err(|e| {
            let error_msg = format!("URL 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        let last_event_id = match &config.last_event_id {
            Some(id) => Some(replace_variables(id, context).map_err(|e| {
                format!("last_event_id 变量替换失败: {}", e)
            })?),
            None => None,
        };

        // 替换 headers 中的变量
        let headers = match &config.headers {
            Some(h) => {
                let mut replaced = HashMap::new();
                for (k, v) in h {
                    let rv = replace_variables(v, context)
                        .map_err(|e| format!("Header '{}' 变量替换失败: {}", k, e))?;
                    replaced.insert(k.clone(), rv);
                }
                Some(replaced)
            }
            None => None,
        };

        // 3. 日志：开始连接
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: sse_connect, URL: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("SSE 连接节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建连接配置（使用替换后的值）
        let connect_config = SseConnectConfig {
            url,
            headers,
            last_event_id,
            ..config
        };

        // 5. 建立连接
        let connect_result = self
            .sse_manager
            .connect(&node.uuid, &connect_config)
            .await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[SSE] 连接成功: uuid={}, url={}, connection_id={}",
                        node.uuid,
                        connect_config.url,
                        output.connection_id
                    );
                } else {
                    log::warn!(
                        "[SSE] 连接失败: uuid={}, url={}, error={:?}",
                        node.uuid,
                        connect_config.url,
                        output.error
                    );
                }

                // 6. 构建输出数据
                let server_headers_json = match &output.server_headers {
                    Some(h) => serde_json::to_value(h).unwrap_or(serde_json::Value::Null),
                    None => serde_json::Value::Null,
                };

                let output_data = json!({
                    "success": output.success,
                    "connection_id": output.connection_id,
                    "server_headers": server_headers_json,
                    "last_event_id": output.last_event_id,
                    "request": {
                        "url": connect_config.url,
                        "timeout_ms": connect_config.timeout_ms,
                        "auto_reconnect": connect_config.auto_reconnect,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 7. 日志：连接完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!("SSE 连接成功 ({}ms)", duration.as_millis())
                } else {
                    format!(
                        "SSE 连接失败: {} ({}ms)",
                        output.error.as_deref().unwrap_or("未知错误"),
                        duration.as_millis()
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_message,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. 路由：成功走 true 端口，失败走 false 端口
                let port = if output.success { "true" } else { "false" };
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == port
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec![port.to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[SSE] 连接异常: uuid={}, url={}, error={}",
                    node.uuid,
                    connect_config.url,
                    e
                );

                // 日志：连接异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("SSE 连接异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "server_headers": serde_json::Value::Null,
                    "last_event_id": serde_json::Value::Null,
                    "request": {
                        "url": connect_config.url,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 返回 false 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid
                                && conn.source_port_id == "false"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec!["false".to_string()]
                    } else {
                        filtered
                    }
                };

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
    use flow_engine_core::node_config::SseConnectConfig;
    use flow_engine_core::traits::{SseConnectOutput, SseConnectionInfo};

    /// 模拟 SSE Provider（连接成功）
    struct MockSseProvider {
        output: SseConnectOutput,
        /// 是否在 connect 中回传 config.last_event_id
        echo_last_event_id: bool,
    }

    impl MockSseProvider {
        fn new_success() -> Self {
            let mut server_headers = HashMap::new();
            server_headers.insert("content-type".to_string(), "text/event-stream".to_string());
            server_headers.insert("cache-control".to_string(), "no-cache".to_string());

            Self {
                output: SseConnectOutput {
                    success: true,
                    connection_id: "test-conn-1".to_string(),
                    server_headers: Some(server_headers),
                    last_event_id: None,
                    error: None,
                },
                echo_last_event_id: false,
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: SseConnectOutput {
                    success: false,
                    connection_id: String::new(),
                    server_headers: None,
                    last_event_id: None,
                    error: Some(error.to_string()),
                },
                echo_last_event_id: false,
            }
        }

        /// 创建会回传 config.last_event_id 的成功 mock
        fn new_success_echo() -> Self {
            let mut server_headers = HashMap::new();
            server_headers.insert("content-type".to_string(), "text/event-stream".to_string());

            Self {
                output: SseConnectOutput {
                    success: true,
                    connection_id: "test-conn-1".to_string(),
                    server_headers: Some(server_headers),
                    last_event_id: None,
                    error: None,
                },
                echo_last_event_id: true,
            }
        }
    }

    #[async_trait::async_trait]
    impl SseProvider for MockSseProvider {
        async fn connect(
            &self,
            _uuid: &str,
            config: &SseConnectConfig,
        ) -> Result<SseConnectOutput, String> {
            if self.echo_last_event_id {
                // 模拟真实行为：回传 config 中的 last_event_id
                let mut out = self.output.clone();
                out.last_event_id = config.last_event_id.clone();
                return Ok(out);
            }
            Ok(self.output.clone())
        }

        async fn listen(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::SseListenConfig,
        ) -> Result<flow_engine_core::traits::SseListenOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::SseDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<SseConnectionInfo> {
            vec![]
        }
    }

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "sse_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseConnect(SseConnectConfig {
                url: url.to_string(),
                ..SseConnectConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
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
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            grpc_manager: None,
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：连接成功时输出 JSON 结构正确
    #[tokio::test]
    async fn test_sse_connect_success_output_structure() {
        let sse_manager = Arc::new(MockSseProvider::new_success());
        let executor = SseConnectExecutor::new(sse_manager);
        let node = create_test_node("test-1", "测试连接", "https://example.com/sse");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 成功应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_id"], "test-conn-1");
        assert!(data["server_headers"].is_object());
        assert_eq!(data["server_headers"]["content-type"], "text/event-stream");
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert_eq!(data["request"]["url"], "https://example.com/sse");
        // error 应为 null（成功时无错误）
        assert!(data["error"].is_null());
    }

    /// 测试：连接失败时路由到 false 端口
    #[tokio::test]
    async fn test_sse_connect_failure_routes_to_false_port() {
        let sse_manager = Arc::new(MockSseProvider::new_failure("Connection refused"));
        let executor = SseConnectExecutor::new(sse_manager);
        let node = create_test_node("test-2", "失败测试", "https://unreachable.example/sse");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 失败应路由到 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Connection refused");
        assert!(data["server_headers"].is_null());
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_sse_connect_wrong_node_data_type() {
        let sse_manager = Arc::new(MockSseProvider::new_success());
        let executor = SseConnectExecutor::new(sse_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SseConnectConfig"));
    }

    /// 测试：变量替换在 URL 中生效
    #[tokio::test]
    async fn test_sse_connect_with_variable_replacement() {
        let sse_manager = Arc::new(MockSseProvider::new_success_echo());
        let executor = SseConnectExecutor::new(sse_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "sse_connect".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::SseConnect(SseConnectConfig {
                url: "https://{{host}}/sse".to_string(),
                headers: None,
                last_event_id: Some("evt-{{ver}}".to_string()),
                ..SseConnectConfig::default()
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("api.example.com"),
            String::new(),
        );
        context.variables.set_variable(
            "ver".to_string(),
            json!("123"),
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
        assert_eq!(data["request"]["url"], "https://api.example.com/sse");
        assert_eq!(data["last_event_id"], "evt-123");
    }

    /// 测试：连接端口路由
    #[tokio::test]
    async fn test_sse_connect_with_connections_routing() {
        let sse_manager = Arc::new(MockSseProvider::new_success());
        let executor = SseConnectExecutor::new(sse_manager);
        let node = create_test_node("test-route", "路由测试", "https://example.com/sse");
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

    /// 测试：provider connect 方法返回 Err 时走 false 端口
    struct MockSseProviderError;

    #[async_trait::async_trait]
    impl SseProvider for MockSseProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &SseConnectConfig,
        ) -> Result<SseConnectOutput, String> {
            Err("网络超时".to_string())
        }

        async fn listen(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::SseListenConfig,
        ) -> Result<flow_engine_core::traits::SseListenOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::SseDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }

        fn list_connections(&self) -> Vec<SseConnectionInfo> {
            vec![]
        }
    }

    #[tokio::test]
    async fn test_sse_connect_provider_error_routes_to_false() {
        let sse_manager = Arc::new(MockSseProviderError);
        let executor = SseConnectExecutor::new(sse_manager);
        let node = create_test_node("test-err", "异常测试", "https://example.com/sse");
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
        assert_eq!(data["error"], "网络超时");
    }
}
