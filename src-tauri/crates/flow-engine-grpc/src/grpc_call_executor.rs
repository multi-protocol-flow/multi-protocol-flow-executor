// gRPC 调用节点执行器
//
// 实现 gRPC 调用（Unary/Server Streaming/Client Streaming/Bidi Streaming）、
// 变量替换和统一输出映射功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::{GrpcPattern, GrpcProvider};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// gRPC 调用节点执行器
pub struct GrpcCallExecutor {
    grpc_manager: Arc<dyn GrpcProvider>,
}

impl GrpcCallExecutor {
    /// 创建新的 gRPC 调用执行器
    pub fn new(grpc_manager: Arc<dyn GrpcProvider>) -> Self {
        Self { grpc_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for GrpcCallExecutor {
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
            NodeData::GrpcCall(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 GrpcCallConfig: {}", node.uuid)),
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 变量替换 request_json
        let request_json = replace_variables(&config.request_json, context).map_err(|e| {
            let error_msg = format!("请求 JSON 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 4. 变量替换 metadata 值
        let metadata: Vec<(String, String)> = config
            .metadata
            .iter()
            .map(|m| {
                let value = replace_variables(&m.value, context).unwrap_or_else(|_| m.value.clone());
                (m.key.clone(), value)
            })
            .collect();

        // 5. 检测流式通信模式
        let pattern = {
            let services = self.grpc_manager.list_services(&connection_id);
            let method_info = services.as_ref().and_then(|svcs| {
                svcs.iter()
                    .find(|s| s.service_name == config.service_name)
                    .and_then(|s| {
                        s.methods
                            .iter()
                            .find(|m| m.method_name == config.method_name)
                    })
            });
            match method_info {
                Some(info) => {
                    GrpcPattern::from_method_info(info.is_client_streaming, info.is_server_streaming)
                }
                None => GrpcPattern::Unary, // 无方法信息时默认 Unary
            }
        };

        let pattern_str = format!("{:?}", pattern).to_lowercase(); // "unary", "server_streaming" 等

        // 6. 日志：开始调用
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: grpc_call, 模式: {}, 连接ID: {}, 服务: {}/{}",
            context.flow_uuid,
            node.uuid,
            node.name,
            pattern_str,
            connection_id,
            config.service_name,
            config.method_name
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "gRPC 调用节点 {} 开始执行: {}/{} ({})",
                    node.name, config.service_name, config.method_name, pattern_str
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 7. 根据模式路由到正确的 gRPC 调用方法
        let call_start = Instant::now();
        let call_result: Result<
            (
                &str,
                serde_json::Value,
                Vec<serde_json::Value>,
                usize,
                usize,
                std::collections::HashMap<String, String>,
                Option<flow_engine_core::traits::GrpcTimingInfo>,
            ),
            String,
        > = match pattern {
            GrpcPattern::Unary => self
                .grpc_manager
                .call_unary(
                    &connection_id,
                    &config.service_name,
                    &config.method_name,
                    &request_json,
                    metadata,
                    config.timeout_ms,
                )
                .await
                .map(|v| {
                    (
                        "unary",
                        v,
                        vec![] as Vec<serde_json::Value>,
                        0usize,
                        0usize,
                        std::collections::HashMap::new(),
                        None as Option<flow_engine_core::traits::GrpcTimingInfo>,
                    )
                }),
            GrpcPattern::ServerStreaming => self
                .grpc_manager
                .call_server_streaming(
                    &connection_id,
                    &config.service_name,
                    &config.method_name,
                    &request_json,
                    metadata,
                    config.timeout_ms,
                )
                .await
                .map(|responses| {
                    let received = responses.len();
                    (
                        "server_streaming",
                        serde_json::Value::Null,
                        responses,
                        1usize,
                        received,
                        std::collections::HashMap::new(),
                        None as Option<flow_engine_core::traits::GrpcTimingInfo>,
                    )
                }),
            GrpcPattern::ClientStreaming => {
                // 过滤启用的消息并提取内容
                let messages: Vec<String> = config
                    .request_messages
                    .iter()
                    .filter(|m| m.enabled)
                    .map(|m| m.content.clone())
                    .collect();
                let sent = messages.len();
                self.grpc_manager
                    .call_client_streaming(
                        &connection_id,
                        &config.service_name,
                        &config.method_name,
                        messages,
                        metadata,
                        config.timeout_ms,
                    )
                    .await
                    .map(|response| {
                        (
                            "client_streaming",
                            response,
                            vec![] as Vec<serde_json::Value>,
                            sent,
                            1usize,
                            std::collections::HashMap::new(),
                            None as Option<flow_engine_core::traits::GrpcTimingInfo>,
                        )
                    })
            }
            GrpcPattern::BidiStreaming => {
                let messages: Vec<String> = config
                    .request_messages
                    .iter()
                    .filter(|m| m.enabled)
                    .map(|m| m.content.clone())
                    .collect();
                let collect_timeout = config
                    .collect_config
                    .as_ref()
                    .map(|c| c.timeout_ms)
                    .unwrap_or(30000);
                let sent = messages.len();
                self.grpc_manager
                    .call_bidi_streaming(
                        &connection_id,
                        &config.service_name,
                        &config.method_name,
                        messages,
                        metadata,
                        config.timeout_ms,
                        collect_timeout,
                    )
                    .await
                    .map(|result| {
                        let received = result.received_count;
                        let timing = Some(result.timing);
                        (
                            "bidi_streaming",
                            serde_json::Value::Null,
                            result.responses,
                            sent,
                            received,
                            result.trailers,
                            timing,
                        )
                    })
            }
        };
        let call_duration_ms = call_start.elapsed().as_millis() as u64;

        match call_result {
            Ok((pattern_str, response_data, responses, sent_count, received_count, trailers, streaming_timing)) => {
                log::info!(
                    "[gRPC] 调用成功: connection_id={}, service={}, method={}, 模式={}, 耗时={}ms",
                    connection_id,
                    config.service_name,
                    config.method_name,
                    pattern_str,
                    call_duration_ms
                );

                // 8. 构建统一成功输出数据
                let duration = start_time.elapsed();
                let mut output_data = json!({
                    "success": true,
                    "pattern": pattern_str,
                    "connection_id": connection_id,
                    "service_name": config.service_name,
                    "method_name": config.method_name,
                    "status": "ok",
                    "status_message": "",
                    "request": {
                        "connection_id": connection_id,
                        "service_name": config.service_name,
                        "method_name": config.method_name,
                        "request_json": config.request_json,
                        "timeout_ms": config.timeout_ms,
                        "metadata_count": config.metadata.len(),
                    },
                    "timing": {
                        "call_ms": call_duration_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 添加流式统计字段
                if sent_count > 0 || received_count > 0 {
                    output_data["sent_count"] = json!(sent_count);
                    output_data["received_count"] = json!(received_count);
                }
                if !responses.is_empty() {
                    output_data["responses"] = json!(responses);
                }
                if !trailers.is_empty() {
                    output_data["trailers"] = json!(trailers);
                }
                if let Some(ref t) = streaming_timing {
                    output_data["streaming_timing"] = json!(t);
                }

                // Unary/Client: 单个响应在 response_data
                // Server/Bidi: 响应在 responses 数组
                if response_data != serde_json::Value::Null {
                    output_data["data"] = json!(response_data);
                }

                // 9. 日志：调用完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "gRPC 调用成功: {}/{} ({}) ({}ms)",
                            config.service_name,
                            config.method_name,
                            pattern_str,
                            duration.as_millis()
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 10. 返回 true 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec!["true".to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[gRPC] 调用失败: connection_id={}, service={}, method={}, 模式={}, error={}",
                    connection_id,
                    config.service_name,
                    config.method_name,
                    pattern_str,
                    e
                );

                // 日志：调用失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!(
                            "gRPC 调用失败: {}/{} ({}) - {}",
                            config.service_name, config.method_name, pattern_str, e
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据（包含 pattern 字段）
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "pattern": pattern_str,
                    "connection_id": connection_id,
                    "service_name": config.service_name,
                    "method_name": config.method_name,
                    "request": {
                        "connection_id": connection_id,
                        "service_name": config.service_name,
                        "method_name": config.method_name,
                        "request_json": config.request_json,
                        "timeout_ms": config.timeout_ms,
                        "metadata_count": config.metadata.len(),
                    },
                    "timing": {
                        "call_ms": call_duration_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "data": serde_json::Value::Null,
                    "status": "error",
                    "status_message": &e,
                    "error": &e,
                });

                // 返回 false 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
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
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::{GrpcCallConfig, GrpcMetadataEntry};
    use crate::grpc_connection_manager::GrpcConnectionManager;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "grpc_call".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: connection_id.to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: r#"{"user_id": "1"}"#.to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![],
                collect_config: None,
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
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            grpc_manager: None,
            providers: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_call_connection_not_found() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);
        let node = create_test_node("test-1", "测试调用", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "nonexistent-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_call_with_variable_replacement() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);

        // 创建带变量的节点
        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "grpc_call".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "{{conn_id}}".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: r#"{"user_id": "{{uid}}"}"#.to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![],
                collect_config: None,
            }),
            ports: None,
            has_breakpoint: None,
        };

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("grpc-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "uid".to_string(),
            json!("user-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "grpc-connection-123");
        assert_eq!(data["success"], false);
    }

    #[tokio::test]
    async fn test_call_with_metadata_variable_replacement() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);

        let node = Node {
            uuid: "test-meta".to_string(),
            node_type: "grpc_call".to_string(),
            name: "Metadata 替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "test-conn".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![GrpcMetadataEntry {
                    key: "authorization".to_string(),
                    value: "Bearer {{token}}".to_string(),
                }],
                request_messages: vec![],
                collect_config: None,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "token".to_string(),
            json!("my-secret-token"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
    }

    #[tokio::test]
    async fn test_call_wrong_node_data_type() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);

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
        assert!(result.unwrap_err().contains("GrpcCallConfig"));
    }

    // ========================================================================
    // MockGrpcProvider — 用于测试路由逻辑，无需网络连接
    // ========================================================================

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use flow_engine_core::traits::{GrpcMethodInfo, GrpcServiceInfo, GrpcStreamingResult, GrpcTimingInfo};

    /// 记录 MockGrpcProvider 各方法的调用次数和参数
    struct MockCallLog {
        call_unary_count: AtomicUsize,
        call_server_streaming_count: AtomicUsize,
        call_client_streaming_count: AtomicUsize,
        call_bidi_streaming_count: AtomicUsize,
        last_client_messages: std::sync::Mutex<Vec<String>>,
        last_bidi_messages: std::sync::Mutex<Vec<String>>,
        last_bidi_collect_timeout_ms: std::sync::Mutex<u64>,
    }

    impl MockCallLog {
        fn new() -> Self {
            Self {
                call_unary_count: AtomicUsize::new(0),
                call_server_streaming_count: AtomicUsize::new(0),
                call_client_streaming_count: AtomicUsize::new(0),
                call_bidi_streaming_count: AtomicUsize::new(0),
                last_client_messages: std::sync::Mutex::new(Vec::new()),
                last_bidi_messages: std::sync::Mutex::new(Vec::new()),
                last_bidi_collect_timeout_ms: std::sync::Mutex::new(0),
            }
        }
    }

    /// Mock gRPC Provider，支持配置方法信息和返回值
    struct MockGrpcProvider {
        /// 提供的服务列表（用于模式检测）
        services: Vec<GrpcServiceInfo>,
        /// unary 返回值
        unary_response: Result<serde_json::Value, String>,
        /// server streaming 返回值
        server_streaming_response: Result<Vec<serde_json::Value>, String>,
        /// client streaming 返回值
        client_streaming_response: Result<serde_json::Value, String>,
        /// bidi streaming 返回值
        bidi_streaming_response: Result<GrpcStreamingResult, String>,
        /// 调用日志
        call_log: MockCallLog,
    }

    impl MockGrpcProvider {
        /// 创建返回成功结果的 Mock
        fn new_success(services: Vec<GrpcServiceInfo>) -> Self {
            Self {
                services,
                unary_response: Ok(serde_json::json!({"result": "ok"})),
                server_streaming_response: Ok(vec![
                    serde_json::json!({"chunk": 1}),
                    serde_json::json!({"chunk": 2}),
                ]),
                client_streaming_response: Ok(serde_json::json!({"aggregated": true})),
                bidi_streaming_response: Ok(GrpcStreamingResult {
                    status: 0,
                    responses: vec![serde_json::json!({"resp": "bidi1"})],
                    sent_count: 2,
                    received_count: 1,
                    trailers: std::collections::HashMap::new(),
                    timing: GrpcTimingInfo {
                        total_ms: 100,
                        first_response_ms: Some(50),
                    },
                }),
                call_log: MockCallLog::new(),
            }
        }

        /// 创建返回错误的 Mock
        fn new_error(services: Vec<GrpcServiceInfo>) -> Self {
            Self {
                services,
                unary_response: Err("模拟调用失败".to_string()),
                server_streaming_response: Err("模拟流式调用失败".to_string()),
                client_streaming_response: Err("模拟客户端流式调用失败".to_string()),
                bidi_streaming_response: Err("模拟双向流式调用失败".to_string()),
                call_log: MockCallLog::new(),
            }
        }
    }

    #[async_trait::async_trait]
    impl GrpcProvider for MockGrpcProvider {
        async fn connect(
            &self, _uuid: String, _url: String, _use_tls: bool, _tls_skip_verify: bool,
            _enable_reflection: bool, _proto_files: Vec<(String, String)>,
            _metadata: Vec<(String, String)>, _connect_timeout_ms: u64,
        ) -> Result<Vec<GrpcServiceInfo>, String> {
            Err("MockGrpcProvider::connect 不应被调用".to_string())
        }

        async fn call_unary(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
        ) -> Result<serde_json::Value, String> {
            self.call_log.call_unary_count.fetch_add(1, AtomicOrdering::SeqCst);
            self.unary_response.clone()
        }

        async fn call_server_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
        ) -> Result<Vec<serde_json::Value>, String> {
            self.call_log.call_server_streaming_count.fetch_add(1, AtomicOrdering::SeqCst);
            self.server_streaming_response.clone()
        }

        async fn call_client_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            request_messages: Vec<String>, _metadata: Vec<(String, String)>, _timeout_ms: u64,
        ) -> Result<serde_json::Value, String> {
            self.call_log.call_client_streaming_count.fetch_add(1, AtomicOrdering::SeqCst);
            *self.call_log.last_client_messages.lock().expect("锁不应失败") = request_messages;
            self.client_streaming_response.clone()
        }

        async fn call_bidi_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            request_messages: Vec<String>, _metadata: Vec<(String, String)>,
            _timeout_ms: u64, collect_timeout_ms: u64,
        ) -> Result<GrpcStreamingResult, String> {
            self.call_log.call_bidi_streaming_count.fetch_add(1, AtomicOrdering::SeqCst);
            *self.call_log.last_bidi_messages.lock().expect("锁不应失败") = request_messages;
            *self.call_log.last_bidi_collect_timeout_ms.lock().expect("锁不应失败") = collect_timeout_ms;
            self.bidi_streaming_response.clone()
        }

        async fn close(&self, _connection_id: &str) -> Result<(), String> {
            Err("MockGrpcProvider::close 不应被调用".to_string())
        }

        fn get_status(&self, _connection_id: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_services(&self, _connection_id: &str) -> Option<Vec<GrpcServiceInfo>> {
            Some(self.services.clone())
        }
    }

    /// 创建包含指定 streaming 配置的服务列表
    fn make_services(
        service_name: &str,
        method_name: &str,
        is_client_streaming: bool,
        is_server_streaming: bool,
    ) -> Vec<GrpcServiceInfo> {
        vec![GrpcServiceInfo {
            service_name: service_name.to_string(),
            methods: vec![GrpcMethodInfo {
                method_name: method_name.to_string(),
                input_type: "test.Request".to_string(),
                output_type: "test.Response".to_string(),
                is_server_streaming,
                is_client_streaming,
            }],
        }]
    }

    /// 创建带 gRPC manager 的执行上下文
    fn create_context_with_grpc(flow_id: &str, grpc_manager: Arc<dyn GrpcProvider>) -> ExecutionContext {
        let mut ctx = create_context(flow_id);
        ctx.grpc_manager = Some(grpc_manager);
        ctx
    }

    // ========================================================================
    // 路由测试 — Unary 模式
    // ========================================================================

    #[tokio::test]
    async fn test_unary_pattern_routes_to_call_unary() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());
        let node = create_test_node("node-1", "Unary 调用", "conn-1");
        let mut context = create_context_with_grpc("flow-1", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["pattern"], "unary");
        assert_eq!(data["data"]["result"], "ok");

        // 验证调用了 call_unary 且未调用其他方法
        assert_eq!(
            mock.call_log.call_unary_count.load(AtomicOrdering::SeqCst),
            1
        );
        assert_eq!(
            mock.call_log.call_server_streaming_count.load(AtomicOrdering::SeqCst),
            0
        );
    }

    // ========================================================================
    // 路由测试 — Server Streaming 模式
    // ========================================================================

    #[tokio::test]
    async fn test_server_streaming_pattern_routes_correctly() {
        let services = make_services("test.TestService", "ListItems", false, true);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        // 节点的 service_name/method_name 必须与 mock 提供的服务匹配
        let node = Node {
            uuid: "node-2".to_string(),
            node_type: "grpc_call".to_string(),
            name: "Server Streaming 调用".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "ListItems".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![],
                collect_config: None,
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-2", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["pattern"], "server_streaming");
        // server streaming 的 data 应为 null，响应在 responses 数组中
        assert_eq!(data["data"], serde_json::Value::Null);
        let responses = data["responses"].as_array().expect("responses 应为数组");
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0]["chunk"], 1);
        assert_eq!(responses[1]["chunk"], 2);
        assert_eq!(data["sent_count"], 1);
        assert_eq!(data["received_count"], 2);

        assert_eq!(
            mock.call_log.call_server_streaming_count.load(AtomicOrdering::SeqCst),
            1
        );
    }

    // ========================================================================
    // 路由测试 — Client Streaming 模式
    // ========================================================================

    #[tokio::test]
    async fn test_client_streaming_pattern_routes_with_filtered_messages() {
        let services = make_services("test.TestService", "UploadItems", true, false);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        // 创建带 3 条消息（2 启用，1 禁用）的节点，service_name/method_name 匹配 mock
        let node = Node {
            uuid: "node-3".to_string(),
            node_type: "grpc_call".to_string(),
            name: "Client Streaming 调用".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "UploadItems".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![
                    flow_engine_core::node_config::StreamMessage {
                        enabled: true,
                        content: r#"{"item": 1}"#.to_string(),
                    },
                    flow_engine_core::node_config::StreamMessage {
                        enabled: false,
                        content: r#"{"item": 2}"#.to_string(),
                    },
                    flow_engine_core::node_config::StreamMessage {
                        enabled: true,
                        content: r#"{"item": 3}"#.to_string(),
                    },
                ],
                collect_config: None,
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-3", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["pattern"], "client_streaming");
        assert_eq!(data["data"]["aggregated"], true);
        assert_eq!(data["sent_count"], 2); // 只发送 2 条启用的消息
        assert_eq!(data["received_count"], 1);

        // 验证发送的消息确实被过滤了
        let msgs = mock.call_log.last_client_messages.lock().expect("锁不应失败");
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].contains("item\": 1"));
        assert!(msgs[1].contains("item\": 3"));
    }

    // ========================================================================
    // 路由测试 — Bidi Streaming 模式
    // ========================================================================

    #[tokio::test]
    async fn test_bidi_streaming_pattern_routes_with_collect_timeout() {
        let services = make_services("test.TestService", "Chat", true, true);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-4".to_string(),
            node_type: "grpc_call".to_string(),
            name: "Bidi Streaming 调用".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "Chat".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 5000,
                metadata: vec![],
                request_messages: vec![
                    flow_engine_core::node_config::StreamMessage {
                        enabled: true,
                        content: r#"{"msg": "hello"}"#.to_string(),
                    },
                ],
                collect_config: Some(flow_engine_core::node_config::GrpcCollectConfig {
                    mode: flow_engine_core::node_config::GrpcCollectMode::All,
                    max_count: None,
                    duration_ms: None,
                    condition: None,
                    timeout_ms: 15000,
                }),
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-4", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["pattern"], "bidi_streaming");
        // bidi streaming 有 responses 数组
        let responses = data["responses"].as_array().expect("responses 应为数组");
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["resp"], "bidi1");
        assert_eq!(data["sent_count"], 1);
        assert_eq!(data["received_count"], 1);
        // 验证 streaming_timing 字段
        assert!(data.get("streaming_timing").is_some());
        assert_eq!(data["streaming_timing"]["total_ms"], 100);

        // 验证 collect_timeout 使用了 collect_config.timeout_ms
        let timeout = mock.call_log.last_bidi_collect_timeout_ms.lock().expect("锁不应失败");
        assert_eq!(*timeout, 15000);
    }

    // ========================================================================
    // 消息过滤测试
    // ========================================================================

    #[tokio::test]
    async fn test_disabled_messages_filtered_out() {
        let services = make_services("test.TestService", "Upload", true, false);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        // 3 条消息，其中 2 条禁用
        let node = Node {
            uuid: "node-filter".to_string(),
            node_type: "grpc_call".to_string(),
            name: "消息过滤测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "Upload".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![
                    flow_engine_core::node_config::StreamMessage {
                        enabled: false,
                        content: r#"{"skip": 1}"#.to_string(),
                    },
                    flow_engine_core::node_config::StreamMessage {
                        enabled: true,
                        content: r#"{"keep": 2}"#.to_string(),
                    },
                    flow_engine_core::node_config::StreamMessage {
                        enabled: false,
                        content: r#"{"skip": 3}"#.to_string(),
                    },
                ],
                collect_config: None,
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-filter", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["sent_count"], 1); // 只有 1 条启用的

        let msgs = mock.call_log.last_client_messages.lock().expect("锁不应失败");
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("keep"));
    }

    // ========================================================================
    // 端口路由测试
    // ========================================================================

    #[tokio::test]
    async fn test_success_routes_to_true_port() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = create_test_node("node-port-success", "成功端口测试", "conn-1");
        let mut context = create_context_with_grpc("flow-port", mock.clone());

        // 创建 true 端口连接
        let connections = vec![Connection {
            id: "c1".to_string(),
            source_node_uuid: "node-port-success".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["next-node"]); // 走 true 端口
    }

    #[tokio::test]
    async fn test_error_routes_to_false_port() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_error(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = create_test_node("node-port-error", "失败端口测试", "conn-1");
        let mut context = create_context_with_grpc("flow-port", mock.clone());

        // 创建 false 端口连接
        let connections = vec![Connection {
            id: "c2".to_string(),
            source_node_uuid: "node-port-error".to_string(),
            target_node_uuid: "error-handler".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应成功");
        assert_eq!(next_nodes, vec!["error-handler"]); // 走 false 端口
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "模拟调用失败");
    }

    // ========================================================================
    // 输出格式测试
    // ========================================================================

    #[tokio::test]
    async fn test_output_format_includes_required_fields() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = create_test_node("node-format", "输出格式测试", "conn-1");
        let mut context = create_context_with_grpc("flow-format", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");

        // 验证必需字段存在
        assert!(data.get("success").is_some(), "缺少 success 字段");
        assert!(data.get("pattern").is_some(), "缺少 pattern 字段");
        assert!(data.get("timing").is_some(), "缺少 timing 字段");
        assert!(data.get("request").is_some(), "缺少 request 字段");
        assert!(data.get("connection_id").is_some(), "缺少 connection_id 字段");
        assert!(data.get("service_name").is_some(), "缺少 service_name 字段");
        assert!(data.get("method_name").is_some(), "缺少 method_name 字段");

        // 验证 timing 结构
        let timing = data.get("timing").expect("timing 应存在");
        assert!(timing.get("call_ms").is_some(), "缺少 timing.call_ms");
        assert!(timing.get("total_ms").is_some(), "缺少 timing.total_ms");

        // 验证 request 结构
        let request = data.get("request").expect("request 应存在");
        assert!(request.get("connection_id").is_some());
        assert!(request.get("service_name").is_some());
        assert!(request.get("method_name").is_some());
        assert!(request.get("request_json").is_some());
        assert!(request.get("timeout_ms").is_some());
    }

    // ========================================================================
    // Bidi 无 collect_config 时使用默认 timeout
    // ========================================================================

    #[tokio::test]
    async fn test_bidi_default_collect_timeout_when_no_config() {
        let services = make_services("test.TestService", "Chat", true, true);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-bidi-default".to_string(),
            node_type: "grpc_call".to_string(),
            name: "Bidi 默认超时测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "Chat".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 5000,
                metadata: vec![],
                request_messages: vec![
                    flow_engine_core::node_config::StreamMessage {
                        enabled: true,
                        content: r#"{"msg": "hi"}"#.to_string(),
                    },
                ],
                collect_config: None, // 无配置，应使用默认 30000ms
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-bidi-default", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let timeout = mock.call_log.last_bidi_collect_timeout_ms.lock().expect("锁不应失败");
        assert_eq!(*timeout, 30000); // 默认 30s
    }

    // ========================================================================
    // 无方法信息时默认 Unary
    // ========================================================================

    #[tokio::test]
    async fn test_no_method_info_defaults_to_unary() {
        // 空服务列表 → 找不到方法 → 默认 Unary
        let mock = Arc::new(MockGrpcProvider::new_success(vec![]));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = create_test_node("node-default", "默认模式测试", "conn-1");
        let mut context = create_context_with_grpc("flow-default", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应成功");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["pattern"], "unary");
    }
}

