// gRPC 调用节点执行器
//
// 实现 gRPC 调用（Unary/Server Streaming/Client Streaming/Bidi Streaming）、
// 变量替换、统一输出映射和重试功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::{GrpcCallConfig, NodeData};
use flow_engine_core::traits::{GrpcPattern, GrpcProvider};
use rand::Rng;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 最大退避时间上限（30 秒）
const MAX_BACKOFF_CAP_MS: u64 = 30_000;

/// 检查 gRPC Error是否可重试
///
/// 可重试的状态码：UNAVAILABLE、DEADLINE_EXCEEDED
/// 这些 code 出现在 tonic Status 的字符串表示中，格式为 "message (CODE)"
fn is_retryable_error(error: &str) -> bool {
    // Extract the gRPC status code from the parenthesized suffix in tonic error strings
    // Format: "description (CODE)" - parse the CODE segment deterministically
    let last_paren = match error.rfind('(') {
        Some(pos) => pos,
        None => return false,
    };
    let code = &error[last_paren + 1..];
    let code = match code.strip_suffix(')') {
        Some(c) => c.trim(),
        None => return false,
    };
    code == "UNAVAILABLE" || code == "DEADLINE_EXCEEDED"
}

/// 计算指数退避时间（带随机抖动）
///
/// - `initial_backoff_ms` — 初始退避时间（毫秒）
/// - `attempt` — 当前重试次数（从 0 开始）
/// - 返回等待时间，上限 30 秒，加 0-50% 随机抖动
fn calculate_backoff_ms(initial_backoff_ms: u64, attempt: u32) -> u64 {
    let base_ms = initial_backoff_ms.saturating_mul(2u64.saturating_pow(attempt));
    let capped_ms = base_ms.min(MAX_BACKOFF_CAP_MS);
    // 添加 0-50% 随机抖动
    let jitter_ms = if capped_ms > 0 {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..=capped_ms / 2)
    } else {
        0
    };
    capped_ms + jitter_ms
}

/// 从 call config 和 connect config 解析有效的重试参数
///
/// Call 级配置覆盖 connect 级默认值
fn resolve_retry_config(
    call_config: &GrpcCallConfig,
    node_map: &HashMap<String, &Node>,
) -> (u32, u64) {
    // 尝试从连接节点获取默认值
    let (default_max_retries, default_initial_backoff_ms) = node_map
        .get(&call_config.connection_id)
        .and_then(|connect_node| {
            if let NodeData::GrpcConnect(connect_config) = &connect_node.data {
                Some((
                    connect_config.max_retries.unwrap_or(0),
                    connect_config.initial_backoff_ms.unwrap_or(1000),
                ))
            } else {
                None
            }
        })
        .unwrap_or((0, 1000));

    // Call 级覆盖 Connect 级
    let max_retries = call_config.max_retries.unwrap_or(default_max_retries);
    let initial_backoff_ms = call_config.initial_backoff_ms.unwrap_or(default_initial_backoff_ms);

    (max_retries, initial_backoff_ms)
}

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
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::GrpcCall(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not GrpcCallConfig: {}", node.uuid))),
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 变量替换 request_json
        let request_json = replace_variables(&config.request_json, context).map_err(|e| {
            let error_msg = format!("请求 JSON variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
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

        // 6. 生成调用 ID（用于区分同一连接的多次调用）
        let call_id = format!("{}-{}", node.uuid, start_time.elapsed().as_millis());

        // 6.5 获取调试事件发送器（如果可用）
        let debug_sender = context.debug_event_sender.clone();

        // 7. 日志：开始调用
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: grpc_call, 模式: {}, Connection ID: {}, 服务: {}/{}",
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
                    "gRPC 调用节点 {} start execution: {}/{} ({})",
                    node.name, config.service_name, config.method_name, pattern_str
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间Error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 7. 解析重试配置
        let (max_retries, initial_backoff_ms) = resolve_retry_config(&config, node_map);

        // 8. 根据模式路由到正确的 gRPC 调用方法
        let call_start = Instant::now();

        // 辅助类型别名
        type CallOutput = (
            &'static str,
            serde_json::Value,
            Vec<serde_json::Value>,
            usize,
            usize,
            std::collections::HashMap<String, String>,
            Option<flow_engine_core::traits::GrpcTimingInfo>,
            Vec<(String, String)>,
            bool, // cancelled
        );

        let call_result: Result<CallOutput, String> = match pattern {
            GrpcPattern::Unary => {
                let mut attempt: u32 = 0;

                loop {
                    let result = self
                        .grpc_manager
                        .call_unary(
                            &connection_id,
                            &config.service_name,
                            &config.method_name,
                            &request_json,
                            metadata.clone(),
                            config.timeout_ms,
                            config.compression_encoding.clone(),
                            debug_sender.clone(),
                            &node.uuid,
                            &call_id,
                        )
                        .await
                        .map(|(v, meta)| {
                            (
                                "unary",
                                v,
                                vec![] as Vec<serde_json::Value>,
                                0usize,
                                0usize,
                                std::collections::HashMap::new(),
                                None as Option<flow_engine_core::traits::GrpcTimingInfo>,
                                meta,
                                false, // unary 不支持取消
                            )
                        });

                    match result {
                        Ok(output) => break Ok(output),
                        Err(e) => {
                            // 检查是否可重试
                            if attempt < max_retries && is_retryable_error(&e) {
                                let backoff_ms = calculate_backoff_ms(initial_backoff_ms, attempt);
                                log::warn!(
                                    "[gRPC] Unary 调用Failed（可重试），第 {} 次重试，等待 {}ms: {}",
                                    attempt + 1,
                                    backoff_ms,
                                    e
                                );
                                context
                                    .emit_log(LogEvent {
                                        level: LogLevel::Warn,
                                        message: format!(
                                            "gRPC 调用Failed，第 {} 次重试（等待 {}ms）",
                                            attempt + 1,
                                            backoff_ms
                                        ),
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .expect("系统时间Error")
                                            .as_millis() as u64,
                                        node_uuid: Some(node.uuid.clone()),
                                        node_name: Some(node.name.clone()),
                                        flow_uuid: context.flow_uuid.clone(),
                                    })
                                    .await;
                                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                                attempt += 1;
                            } else {
                                break Err(e);
                            }
                        }
                    }
                }
            }
            GrpcPattern::ServerStreaming => {
                let mut attempt: u32 = 0;

                loop {
                    let result = self
                        .grpc_manager
                        .call_server_streaming(
                            &connection_id,
                            &config.service_name,
                            &config.method_name,
                            &request_json,
                            metadata.clone(),
                            config.timeout_ms,
                            config.compression_encoding.clone(),
                            debug_sender.clone(),
                            &node.uuid,
                            &call_id,
                        )
                        .await
                        .map(|(responses, meta)| {
                            let received = responses.len();
                            (
                                "server_streaming",
                                serde_json::Value::Null,
                                responses,
                                1usize,
                                received,
                                std::collections::HashMap::new(),
                                None as Option<flow_engine_core::traits::GrpcTimingInfo>,
                                meta,
                                false, // server_streaming 取消状态由前端跟踪
                            )
                        });

                    match result {
                        Ok(output) => break Ok(output),
                        Err(e) => {
                            // 检查是否可重试
                            if attempt < max_retries && is_retryable_error(&e) {
                                let backoff_ms = calculate_backoff_ms(initial_backoff_ms, attempt);
                                log::warn!(
                                    "[gRPC] Server Streaming 调用Failed（可重试），第 {} 次重试，等待 {}ms: {}",
                                    attempt + 1,
                                    backoff_ms,
                                    e
                                );
                                context
                                    .emit_log(LogEvent {
                                        level: LogLevel::Warn,
                                        message: format!(
                                            "gRPC 调用Failed，第 {} 次重试（等待 {}ms）",
                                            attempt + 1,
                                            backoff_ms
                                        ),
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .expect("系统时间Error")
                                            .as_millis() as u64,
                                        node_uuid: Some(node.uuid.clone()),
                                        node_name: Some(node.name.clone()),
                                        flow_uuid: context.flow_uuid.clone(),
                                    })
                                    .await;
                                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                                attempt += 1;
                            } else {
                                break Err(e);
                            }
                        }
                    }
                }
            }
            GrpcPattern::ClientStreaming => {
                // Client Streaming 不支持重试（复杂状态）
                if max_retries > 0 {
                    log::warn!(
                        "[gRPC] Client Streaming 模式不支持重试，忽略重试配置 (max_retries={})",
                        max_retries
                    );
                }
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
                        config.compression_encoding.clone(),
                        debug_sender.clone(),
                        &node.uuid,
                        &call_id,
                    )
                    .await
                    .map(|(response, meta)| {
                        (
                            "client_streaming",
                            response,
                            vec![] as Vec<serde_json::Value>,
                            sent,
                            1usize,
                            std::collections::HashMap::new(),
                            None as Option<flow_engine_core::traits::GrpcTimingInfo>,
                            meta,
                            false, // client_streaming 不支持取消
                        )
                    })
            }
            GrpcPattern::BidiStreaming => {
                // Bidi Streaming 不支持重试（复杂状态）
                if max_retries > 0 {
                    log::warn!(
                        "[gRPC] Bidi Streaming 模式不支持重试，忽略重试配置 (max_retries={})",
                        max_retries
                    );
                }
                let messages: Vec<String> = config
                    .request_messages
                    .iter()
                    .filter(|m| m.enabled)
                    .map(|m| m.content.clone())
                    .collect();
                let sent = messages.len();
                self.grpc_manager
                    .call_bidi_streaming(
                        &connection_id,
                        &config.service_name,
                        &config.method_name,
                        messages,
                        metadata,
                        config.timeout_ms,
                        config.compression_encoding.clone(),
                        debug_sender.clone(),
                        &node.uuid,
                        &call_id,
                    )
                    .await
                    .map(|result| {
                        let received = result.received_count;
                        let timing = Some(result.timing);
                        let meta = result.response_metadata;
                        let cancelled = result.cancelled;
                        (
                            "bidi_streaming",
                            serde_json::Value::Null,
                            result.responses,
                            sent,
                            received,
                            result.trailers,
                            timing,
                            meta,
                            cancelled,
                        )
                    })
            }
        };
        let call_duration_ms = call_start.elapsed().as_millis() as u64;

        match call_result {
            Ok((pattern_str, response_data, responses, sent_count, received_count, trailers, streaming_timing, response_metadata, cancelled)) => {
                log::info!(
                    "[gRPC] 调用Success: connection_id={}, service={}, method={}, 模式={}, 耗时={}ms",
                    connection_id,
                    config.service_name,
                    config.method_name,
                    pattern_str,
                    call_duration_ms
                );

                // 8. 构建统一Success输出数据
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
                // 添加响应 Initial Metadata
                let meta_map: std::collections::HashMap<String, String> = response_metadata.into_iter().collect();
                if !meta_map.is_empty() {
                    output_data["response_metadata"] = json!(meta_map);
                }
                if let Some(ref t) = streaming_timing {
                    output_data["streaming_timing"] = json!(t);
                }
                // 添加取消标记
                if cancelled {
                    output_data["cancelled"] = json!(true);
                    output_data["status"] = json!("cancelled");
                    output_data["status_message"] = json!("用户取消");
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
                            "gRPC 调用Success: {}/{} ({}) ({}ms)",
                            config.service_name,
                            config.method_name,
                            pattern_str,
                            duration.as_millis()
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间Error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9.5 发送调试事件：GrpcMethodCalled
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::GrpcMethodCalled {
                        node_uuid: node.uuid.clone(),
                        connection_id: connection_id.clone(),
                        service: config.service_name.clone(),
                        method: config.method_name.clone(),
                        pattern: pattern_str.to_string(),
                        duration_ms: call_duration_ms,
                    });
                }

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
                // 解析Error消息和结构化Error详情
                let (error_message, error_details) =
                    crate::error_detail::parse_error_details(&e);

                log::error!(
                    "[gRPC] 调用Failed: connection_id={}, service={}, method={}, 模式={}, error={}",
                    connection_id,
                    config.service_name,
                    config.method_name,
                    pattern_str,
                    error_message
                );

                // 日志：调用Failed
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!(
                            "gRPC 调用Failed: {}/{} ({}) - {}",
                            config.service_name, config.method_name, pattern_str, error_message
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间Error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据（包含 pattern 字段和结构化Error详情）
                let duration = start_time.elapsed();
                let mut error_output = json!({
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
                    "status_message": error_message,
                    "error": error_message,
                });

                // 添加结构化Error详情（来自 google.rpc.Status 的 google.protobuf.Any）
                if !error_details.is_empty() {
                    error_output["error_details"] = json!(error_details);
                }

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", error_message)));
                }

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

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "grpc_call".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig { connection_id: connection_id.to_string(),
            service_name: "test.TestService".to_string(),
            method_name: "GetUser".to_string(),
            request_json: r#"{"user_id": "1"}"#.to_string(),
            timeout_ms: 30000,
            metadata: vec![],
            request_messages: vec![], ..Default::default() }),
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
                .expect("系统时间Error")
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
            data: NodeData::GrpcCall(GrpcCallConfig { connection_id: "{{conn_id}}".to_string(),
            service_name: "test.TestService".to_string(),
            method_name: "GetUser".to_string(),
            request_json: r#"{"user_id": "{{uid}}"}"#.to_string(),
            timeout_ms: 30000,
            metadata: vec![],
            request_messages: vec![], ..Default::default() }),
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
            data: NodeData::GrpcCall(GrpcCallConfig { connection_id: "test-conn".to_string(),
            service_name: "test.TestService".to_string(),
            method_name: "GetUser".to_string(),
            request_json: "{}".to_string(),
            timeout_ms: 30000,
            metadata: vec![GrpcMetadataEntry {
                key: "authorization".to_string(),
                value: "Bearer {{token}}".to_string(),
            }],
            request_messages: vec![], ..Default::default() }),
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
            name: "Error类型".to_string(),
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
    use flow_engine_core::traits::{GrpcMethodInfo, GrpcServiceInfo, GrpcStreamingResult, GrpcTimingInfo, HealthCheckResponse};

    /// 记录 MockGrpcProvider 各方法的调用次数和参数
    struct MockCallLog {
        call_unary_count: AtomicUsize,
        call_server_streaming_count: AtomicUsize,
        call_client_streaming_count: AtomicUsize,
        call_bidi_streaming_count: AtomicUsize,
        last_client_messages: std::sync::Mutex<Vec<String>>,
        last_bidi_messages: std::sync::Mutex<Vec<String>>,
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
        /// 创建返回Success结果的 Mock
        fn new_success(services: Vec<GrpcServiceInfo>) -> Self {
            Self {
                services,
                unary_response: Ok(serde_json::json!({"result": "ok"})),
                server_streaming_response: Ok(vec![
                    serde_json::json!({"index": 1, "data": {"chunk": 1}, "received_at_ms": 0, "size_bytes": 0}),
                    serde_json::json!({"index": 2, "data": {"chunk": 2}, "received_at_ms": 0, "size_bytes": 0}),
                ]),
                client_streaming_response: Ok(serde_json::json!({"aggregated": true})),
                bidi_streaming_response: Ok(GrpcStreamingResult {
                    status: 0,
                    responses: vec![serde_json::json!({"index": 1, "data": {"resp": "bidi1"}, "received_at_ms": 50, "size_bytes": 10})],
                    sent_count: 2,
                    received_count: 1,
                    trailers: std::collections::HashMap::new(),
                    response_metadata: vec![],
                    timing: GrpcTimingInfo {
                        total_ms: 100,
                        first_response_ms: Some(50),
                    },
                    cancelled: false,
                }),
                call_log: MockCallLog::new(),
            }
        }

        /// 创建返回Error的 Mock
        fn new_error(services: Vec<GrpcServiceInfo>) -> Self {
            Self {
                services,
                unary_response: Err("模拟调用Failed".to_string()),
                server_streaming_response: Err("模拟流式调用Failed".to_string()),
                client_streaming_response: Err("模拟客户端流式调用Failed".to_string()),
                bidi_streaming_response: Err("模拟双向流式调用Failed".to_string()),
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
            _tls_ca_cert: Option<String>, _tls_client_cert: Option<String>,
            _tls_client_key: Option<String>, _tls_server_name_override: Option<String>,
            _reflection_metadata: Vec<(String, String)>,
            _compression_encoding: Option<String>,
            _keepalive_time_ms: Option<u64>,
            _keepalive_timeout_ms: Option<u64>,
            _keepalive_permit_without_streams: Option<bool>,
            _endpoints: Option<Vec<String>>,
        ) -> Result<Vec<GrpcServiceInfo>, String> {
            Err("MockGrpcProvider::connect 不应被调用".to_string())
        }

        async fn call_unary(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
            _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
            self.call_log.call_unary_count.fetch_add(1, AtomicOrdering::SeqCst);
            self.unary_response.clone().map(|v| (v, vec![]))
        }

        async fn call_server_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
            _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<(Vec<serde_json::Value>, Vec<(String, String)>), String> {
            self.call_log.call_server_streaming_count.fetch_add(1, AtomicOrdering::SeqCst);
            self.server_streaming_response.clone().map(|v| (v, vec![]))
        }

        async fn call_client_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            request_messages: Vec<String>, _metadata: Vec<(String, String)>, _timeout_ms: u64,
            _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
            self.call_log.call_client_streaming_count.fetch_add(1, AtomicOrdering::SeqCst);
            *self.call_log.last_client_messages.lock().expect("锁不应Failed") = request_messages;
            self.client_streaming_response.clone().map(|v| (v, vec![]))
        }

        async fn call_bidi_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            request_messages: Vec<String>, _metadata: Vec<(String, String)>,
            _timeout_ms: u64, _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<GrpcStreamingResult, String> {
            self.call_log.call_bidi_streaming_count.fetch_add(1, AtomicOrdering::SeqCst);
            *self.call_log.last_bidi_messages.lock().expect("锁不应Failed") = request_messages;
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

        async fn health_check(
            &self,
            _connection_id: &str,
            _service: &str,
        ) -> Result<HealthCheckResponse, String> {
            Ok(HealthCheckResponse {
                status: "SERVING".to_string(),
            })
        }

        fn cancel_stream(&self, _connection_id: &str, _call_id: &str) -> Result<(), String> {
            Ok(())
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
    fn create_context_with_grpc<T: 'static + Send + Sync>(flow_id: &str, grpc_manager: Arc<T>) -> ExecutionContext {
        let mut ctx = create_context(flow_id);
        ctx.set_provider("grpc", grpc_manager);
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
        let (next_nodes, output_data) = result.expect("执行应Success");
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
            data: NodeData::GrpcCall(GrpcCallConfig { connection_id: "conn-1".to_string(),
            service_name: "test.TestService".to_string(),
            method_name: "ListItems".to_string(),
            request_json: "{}".to_string(),
            timeout_ms: 30000,
            metadata: vec![],
            request_messages: vec![], ..Default::default() }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-2", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["pattern"], "server_streaming");
        // server streaming 的 data 应为 null，响应在 responses 数组中
        assert_eq!(data["data"], serde_json::Value::Null);
        let responses = data["responses"].as_array().expect("responses 应为数组");
        assert_eq!(responses.len(), 2);
        // 每条响应包装为 { index, data } 格式
        assert_eq!(responses[0]["index"], 1);
        assert_eq!(responses[0]["data"]["chunk"], 1);
        assert_eq!(responses[1]["index"], 2);
        assert_eq!(responses[1]["data"]["chunk"], 2);
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
            data: NodeData::GrpcCall(GrpcCallConfig { connection_id: "conn-1".to_string(),
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
            ], ..Default::default() }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-3", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["pattern"], "client_streaming");
        assert_eq!(data["data"]["aggregated"], true);
        assert_eq!(data["sent_count"], 2); // 只发送 2 条启用的消息
        assert_eq!(data["received_count"], 1);

        // 验证发送的消息确实被过滤了
        let msgs = mock.call_log.last_client_messages.lock().expect("锁不应Failed");
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].contains("item\": 1"));
        assert!(msgs[1].contains("item\": 3"));
    }

    // ========================================================================
    // 路由测试 — Bidi Streaming 模式
    // ========================================================================

    #[tokio::test]
    async fn test_bidi_streaming_pattern_routes_correctly() {
        let services = make_services("test.TestService", "Chat", true, true);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-4".to_string(),
            node_type: "grpc_call".to_string(),
            name: "Bidi Streaming 调用".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig { connection_id: "conn-1".to_string(),
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
            ], ..Default::default() }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-4", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["pattern"], "bidi_streaming");
        // bidi streaming 有 responses 数组，每条包装为 { index, data } 格式
        let responses = data["responses"].as_array().expect("responses 应为数组");
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["index"], 1);
        assert_eq!(responses[0]["data"]["resp"], "bidi1");
        assert_eq!(data["sent_count"], 1);
        assert_eq!(data["received_count"], 1);
        // 验证 streaming_timing 字段
        assert!(data.get("streaming_timing").is_some());
        assert_eq!(data["streaming_timing"]["total_ms"], 100);
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
            data: NodeData::GrpcCall(GrpcCallConfig { connection_id: "conn-1".to_string(),
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
            ], ..Default::default() }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-filter", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应Success");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["sent_count"], 1); // 只有 1 条启用的

        let msgs = mock.call_log.last_client_messages.lock().expect("锁不应Failed");
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

        let node = create_test_node("node-port-success", "Success端口测试", "conn-1");
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
        let (next_nodes, _) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["next-node"]); // 走 true 端口
    }

    #[tokio::test]
    async fn test_error_routes_to_false_port() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_error(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = create_test_node("node-port-error", "Failed端口测试", "conn-1");
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
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["error-handler"]); // 走 false 端口
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "模拟调用Failed");
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
        let (_, output_data) = result.expect("执行应Success");
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
        let (_, output_data) = result.expect("执行应Success");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["pattern"], "unary");
    }

    // ========================================================================
    // 调试事件发射测试
    // ========================================================================

    use flow_engine_core::debug_types::DebugEvent;
    use flow_engine_core::traits::DebugEventSender;

    /// Mock DebugEventSender，记录发送的事件
    struct MockDebugEventSender {
        events: std::sync::Mutex<Vec<DebugEvent>>,
    }

    impl MockDebugEventSender {
        fn new() -> Self {
            Self {
                events: std::sync::Mutex::new(Vec::new()),
            }
        }

        /// 获取已发送的事件
        fn events(&self) -> Vec<DebugEvent> {
            self.events.lock().expect("锁不应Failed").clone()
        }
    }

    impl DebugEventSender for MockDebugEventSender {
        fn send_event(&self, event: DebugEvent) -> Result<(), String> {
            self.events.lock().expect("锁不应Failed").push(event);
            Ok(())
        }
    }

    /// 创建带 debug_event_sender 的执行上下文
    fn create_context_with_debug(flow_id: &str, sender: Arc<dyn DebugEventSender>) -> ExecutionContext {
        let mut ctx = create_context(flow_id);
        ctx.debug_event_sender = Some(sender);
        ctx
    }

    #[tokio::test]
    async fn test_debug_event_grpc_method_called_on_success() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_success(services));
        let executor = GrpcCallExecutor::new(mock.clone());
        let debug_sender = Arc::new(MockDebugEventSender::new());

        let node = create_test_node("node-debug", "调试事件测试", "conn-1");
        let mut context = create_context_with_debug("flow-debug", debug_sender.clone());
        context.set_provider("grpc", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.expect("执行应Success");
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);

        // 验证 debug 事件已发送
        let events = debug_sender.events();
        assert_eq!(events.len(), 1, "应发送 1 个调试事件");

        match &events[0] {
            DebugEvent::GrpcMethodCalled {
                node_uuid,
                connection_id,
                service,
                method,
                pattern,
                duration_ms,
            } => {
                assert_eq!(node_uuid, "node-debug");
                assert_eq!(connection_id, "conn-1");
                assert_eq!(service, "test.TestService");
                assert_eq!(method, "GetUser");
                assert_eq!(pattern, "unary");
                assert!(duration_ms > &0 || duration_ms == &0, "duration_ms 应为非负数");
            }
            other => panic!("期望 GrpcMethodCalled 事件，实际为: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_no_debug_event_on_call_failure() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_error(services));
        let executor = GrpcCallExecutor::new(mock.clone());
        let debug_sender = Arc::new(MockDebugEventSender::new());

        let node = create_test_node("node-fail", "Failed无事件测试", "conn-1");
        let mut context = create_context_with_debug("flow-fail", debug_sender.clone());
        context.set_provider("grpc", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        // Failed时不应发送 GrpcMethodCalled 事件
        let events = debug_sender.events();
        assert_eq!(events.len(), 0, "Failed时不应发送调试事件");
    }

    // ========================================================================
    // 重试逻辑测试
    // ========================================================================

    #[test]
    fn test_is_retryable_error_unavailable() {
        assert!(is_retryable_error("gRPC Unary 调用Failed: service unavailable (UNAVAILABLE)"));
    }

    #[test]
    fn test_is_retryable_error_deadline_exceeded() {
        assert!(is_retryable_error("gRPC Unary 调用Failed: timeout (DEADLINE_EXCEEDED)"));
    }

    #[test]
    fn test_is_retryable_error_not_retryable() {
        assert!(!is_retryable_error("gRPC Unary 调用Failed: not found (NOT_FOUND)"));
        assert!(!is_retryable_error("gRPC Unary 调用Failed: permission denied (PERMISSION_DENIED)"));
        assert!(!is_retryable_error("连接 'xxx' 不存在"));
    }

    #[test]
    fn test_calculate_backoff_exponential() {
        // initial_backoff_ms=1000, attempt=0 → 1000 + jitter(0-500)
        let backoff = calculate_backoff_ms(1000, 0);
        assert!(backoff >= 1000 && backoff <= 1500, "attempt 0: {} should be in [1000, 1500]", backoff);

        // attempt=1 → 2000 + jitter(0-1000)
        let backoff = calculate_backoff_ms(1000, 1);
        assert!(backoff >= 2000 && backoff <= 3000, "attempt 1: {} should be in [2000, 3000]", backoff);

        // attempt=2 → 4000 + jitter(0-2000)
        let backoff = calculate_backoff_ms(1000, 2);
        assert!(backoff >= 4000 && backoff <= 6000, "attempt 2: {} should be in [4000, 6000]", backoff);
    }

    #[test]
    fn test_calculate_backoff_capped_at_30s() {
        // initial_backoff_ms=1000, attempt=10 → 1000*1024 > 30000, cap at 30000
        let backoff = calculate_backoff_ms(1000, 10);
        assert!(backoff >= 30000 && backoff <= 45000, "should be capped at 30000 + jitter");
    }

    #[test]
    fn test_resolve_retry_config_call_overrides_connect() {
        let call_config = GrpcCallConfig {
            connection_id: "conn-1".to_string(),
            service_name: "test.Service".to_string(),
            method_name: "Get".to_string(),
            request_json: "{}".to_string(),
            timeout_ms: 30000,
            metadata: vec![],
            request_messages: vec![],
            compression_encoding: None,
            max_retries: Some(5),
            initial_backoff_ms: Some(2000),
            max_receive_message_size: None,
            max_send_message_size: None,
        };

        let connect_node = Node {
            uuid: "conn-1".to_string(),
            node_type: "grpc_connect".to_string(),
            name: "gRPC 连接".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcConnect(flow_engine_core::node_config::GrpcConnectConfig {
                url: "http://localhost:50051".to_string(),
                max_retries: Some(3),
                initial_backoff_ms: Some(500),
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut node_map = HashMap::new();
        node_map.insert("conn-1".to_string(), &connect_node);

        let (max_retries, initial_backoff_ms) = resolve_retry_config(&call_config, &node_map);
        assert_eq!(max_retries, 5, "call config should override");
        assert_eq!(initial_backoff_ms, 2000, "call config should override");
    }

    #[test]
    fn test_resolve_retry_config_fallback_to_connect() {
        let call_config = GrpcCallConfig {
            connection_id: "conn-1".to_string(),
            service_name: "test.Service".to_string(),
            method_name: "Get".to_string(),
            request_json: "{}".to_string(),
            timeout_ms: 30000,
            metadata: vec![],
            request_messages: vec![],
            compression_encoding: None,
            max_retries: None,
            initial_backoff_ms: None,
            max_receive_message_size: None,
            max_send_message_size: None,
        };

        let connect_node = Node {
            uuid: "conn-1".to_string(),
            node_type: "grpc_connect".to_string(),
            name: "gRPC 连接".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcConnect(flow_engine_core::node_config::GrpcConnectConfig {
                url: "http://localhost:50051".to_string(),
                max_retries: Some(3),
                initial_backoff_ms: Some(500),
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut node_map = HashMap::new();
        node_map.insert("conn-1".to_string(), &connect_node);

        let (max_retries, initial_backoff_ms) = resolve_retry_config(&call_config, &node_map);
        assert_eq!(max_retries, 3, "should use connect default");
        assert_eq!(initial_backoff_ms, 500, "should use connect default");
    }

    #[test]
    fn test_resolve_retry_config_no_connect_defaults() {
        let call_config = GrpcCallConfig {
            connection_id: "conn-1".to_string(),
            service_name: "test.Service".to_string(),
            method_name: "Get".to_string(),
            request_json: "{}".to_string(),
            timeout_ms: 30000,
            metadata: vec![],
            request_messages: vec![],
            compression_encoding: None,
            max_retries: None,
            initial_backoff_ms: None,
            max_receive_message_size: None,
            max_send_message_size: None,
        };

        // 连接节点不在 node_map 中
        let node_map = HashMap::new();
        let (max_retries, initial_backoff_ms) = resolve_retry_config(&call_config, &node_map);
        assert_eq!(max_retries, 0, "no connect → default 0");
        assert_eq!(initial_backoff_ms, 1000, "no connect → default 1000ms");
    }

    // ========================================================================
    // 重试集成测试（使用 Mock）
    // ========================================================================

    /// Mock: 第一次返回 UNAVAILABLE Error，第二次Success
    struct RetryThenSuccessMock {
        fail_count: AtomicUsize,
        services: Vec<GrpcServiceInfo>,
    }

    #[async_trait::async_trait]
    impl GrpcProvider for RetryThenSuccessMock {
        async fn connect(
            &self, _uuid: String, _url: String, _use_tls: bool, _tls_skip_verify: bool,
            _enable_reflection: bool, _proto_files: Vec<(String, String)>,
            _metadata: Vec<(String, String)>, _connect_timeout_ms: u64,
            _tls_ca_cert: Option<String>, _tls_client_cert: Option<String>,
            _tls_client_key: Option<String>, _tls_server_name_override: Option<String>,
            _reflection_metadata: Vec<(String, String)>,
            _compression_encoding: Option<String>,
            _keepalive_time_ms: Option<u64>,
            _keepalive_timeout_ms: Option<u64>,
            _keepalive_permit_without_streams: Option<bool>,
            _endpoints: Option<Vec<String>>,
        ) -> Result<Vec<GrpcServiceInfo>, String> {
            Err("不应被调用".to_string())
        }

        async fn call_unary(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
            _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
            let count = self.fail_count.fetch_add(1, AtomicOrdering::SeqCst);
            if count == 0 {
                Err("gRPC Unary 调用Failed: service unavailable (UNAVAILABLE)".to_string())
            } else {
                Ok((serde_json::json!({"success": true}), vec![]))
            }
        }

        async fn call_server_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
            _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<(Vec<serde_json::Value>, Vec<(String, String)>), String> {
            Err("不应被调用".to_string())
        }

        async fn call_client_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_messages: Vec<String>, _metadata: Vec<(String, String)>, _timeout_ms: u64,
            _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
            Err("不应被调用".to_string())
        }

        async fn call_bidi_streaming(
            &self, _connection_id: &str, _service_name: &str, _method_name: &str,
            _request_messages: Vec<String>, _metadata: Vec<(String, String)>,
            _timeout_ms: u64, _compression_encoding: Option<String>,
            _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
            _node_uuid: &str,
            _call_id: &str,
        ) -> Result<GrpcStreamingResult, String> {
            Err("不应被调用".to_string())
        }

        async fn close(&self, _connection_id: &str) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _connection_id: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_services(&self, _connection_id: &str) -> Option<Vec<GrpcServiceInfo>> {
            Some(self.services.clone())
        }

        async fn health_check(
            &self, _connection_id: &str, _service: &str,
        ) -> Result<HealthCheckResponse, String> {
            Ok(HealthCheckResponse { status: "SERVING".to_string() })
        }

        fn cancel_stream(&self, _connection_id: &str, _call_id: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_unary_retries_on_unavailable_then_succeeds() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(RetryThenSuccessMock {
            fail_count: AtomicUsize::new(0),
            services,
        });
        let executor = GrpcCallExecutor::new(mock.clone());

        // 创建节点，配置 max_retries=3
        let node = Node {
            uuid: "node-retry".to_string(),
            node_type: "grpc_call".to_string(),
            name: "重试测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![],
                max_retries: Some(3),
                initial_backoff_ms: Some(100), // 快速重试
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-retry", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["true"]); // Success走 true 端口
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], true);
        assert_eq!(data["data"]["success"], true);
        // 验证重试了 1 次后Success（第一次Failed，第二次Success）
        assert_eq!(mock.fail_count.load(AtomicOrdering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_unary_no_retry_on_non_retryable_error() {
        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(MockGrpcProvider::new_error(services));
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-no-retry".to_string(),
            node_type: "grpc_call".to_string(),
            name: "不可重试Error".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![],
                max_retries: Some(3),
                initial_backoff_ms: Some(100),
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-no-retry", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["false"]); // Failed走 false 端口
        let data = output_data.expect("应有输出数据");
        assert_eq!(data["success"], false);
        // 验证只调用了一次（没有重试）
        assert_eq!(
            mock.call_log.call_unary_count.load(AtomicOrdering::SeqCst),
            1,
            "不可重试Error应只调用一次"
        );
    }

    #[tokio::test]
    async fn test_unary_retries_exhausted() {
        // Mock: 始终返回 UNAVAILABLE
        struct AlwaysUnavailableMock {
            services: Vec<GrpcServiceInfo>,
            call_count: AtomicUsize,
        }

        #[async_trait::async_trait]
        impl GrpcProvider for AlwaysUnavailableMock {
            async fn connect(
                &self, _uuid: String, _url: String, _use_tls: bool, _tls_skip_verify: bool,
                _enable_reflection: bool, _proto_files: Vec<(String, String)>,
                _metadata: Vec<(String, String)>, _connect_timeout_ms: u64,
                _tls_ca_cert: Option<String>, _tls_client_cert: Option<String>,
                _tls_client_key: Option<String>, _tls_server_name_override: Option<String>,
                _reflection_metadata: Vec<(String, String)>,
                _compression_encoding: Option<String>,
                _keepalive_time_ms: Option<u64>,
                _keepalive_timeout_ms: Option<u64>,
                _keepalive_permit_without_streams: Option<bool>,
                _endpoints: Option<Vec<String>>,
            ) -> Result<Vec<GrpcServiceInfo>, String> {
                Err("不应被调用".to_string())
            }

            async fn call_unary(
                &self, _connection_id: &str, _service_name: &str, _method_name: &str,
                _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
                _compression_encoding: Option<String>,
                _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
                _node_uuid: &str,
                _call_id: &str,
            ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
                self.call_count.fetch_add(1, AtomicOrdering::SeqCst);
                Err("gRPC Unary 调用Failed: service unavailable (UNAVAILABLE)".to_string())
            }

            async fn call_server_streaming(
                &self, _connection_id: &str, _service_name: &str, _method_name: &str,
                _request_json: &str, _metadata: Vec<(String, String)>, _timeout_ms: u64,
                _compression_encoding: Option<String>,
                _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
                _node_uuid: &str,
                _call_id: &str,
            ) -> Result<(Vec<serde_json::Value>, Vec<(String, String)>), String> {
                Err("不应被调用".to_string())
            }

            async fn call_client_streaming(
                &self, _connection_id: &str, _service_name: &str, _method_name: &str,
                _request_messages: Vec<String>, _metadata: Vec<(String, String)>, _timeout_ms: u64,
                _compression_encoding: Option<String>,
                _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
                _node_uuid: &str,
                _call_id: &str,
            ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
                Err("不应被调用".to_string())
            }

            async fn call_bidi_streaming(
                &self, _connection_id: &str, _service_name: &str, _method_name: &str,
                _request_messages: Vec<String>, _metadata: Vec<(String, String)>,
                _timeout_ms: u64, _compression_encoding: Option<String>,
                _debug_sender: Option<std::sync::Arc<dyn flow_engine_core::traits::DebugEventSender>>,
                _node_uuid: &str,
                _call_id: &str,
            ) -> Result<GrpcStreamingResult, String> {
                Err("不应被调用".to_string())
            }

            async fn close(&self, _connection_id: &str) -> Result<(), String> {
                Ok(())
            }

            fn get_status(&self, _connection_id: &str) -> Option<String> {
                Some("connected".to_string())
            }

            fn list_services(&self, _connection_id: &str) -> Option<Vec<GrpcServiceInfo>> {
                Some(self.services.clone())
            }

            async fn health_check(
                &self,
                _connection_id: &str,
                _service: &str,
            ) -> Result<HealthCheckResponse, String> {
                Ok(HealthCheckResponse { status: "SERVING".to_string() })
            }

            fn cancel_stream(&self, _connection_id: &str, _call_id: &str) -> Result<(), String> {
                Ok(())
            }
        }

        let services = make_services("test.TestService", "GetUser", false, false);
        let mock = Arc::new(AlwaysUnavailableMock {
            services,
            call_count: AtomicUsize::new(0),
        });
        let executor = GrpcCallExecutor::new(mock.clone());

        let node = Node {
            uuid: "node-exhaust".to_string(),
            node_type: "grpc_call".to_string(),
            name: "重试耗尽".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "conn-1".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![],
                request_messages: vec![],
                max_retries: Some(2),
                initial_backoff_ms: Some(50), // 快速重试
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context_with_grpc("flow-exhaust", mock.clone());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.expect("执行应Success");
        assert_eq!(next_nodes, vec!["false"]); // 全部重试Failed，走 false 端口
        // 验证调用了 3 次（初始 1 次 + 2 次重试）
        assert_eq!(
            mock.call_count.load(AtomicOrdering::SeqCst),
            3,
            "应调用 3 次（1 初始 + 2 重试）"
        );
    }
}

