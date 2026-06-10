//! MQTT 请求/响应执行器
//!
//! 实现 MQTT 5.0 Request/Response 模式:
//! 自动生成 correlation_data → 自动订阅 response_topic → 发布请求 → 等待响应 → 取消订阅。
//!
//! 实际的请求/响应生命周期由 provider.request() 处理，
//! 本执行器负责配置解析、变量替换、correlation_data 自动生成及输出构建。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{MqttRequestConfig, NodeData};
use flow_engine_core::traits::{MqttProvider, MqttReceivedMessage};
use serde_json::json;

/// MQTT 请求/响应节点执行器
pub struct MqttRequestExecutor {
    mqtt_manager: Arc<dyn MqttProvider>,
}

impl MqttRequestExecutor {
    /// 创建新的 MQTT 请求/响应执行器
    pub fn new(mqtt_manager: Arc<dyn MqttProvider>) -> Self {
        Self { mqtt_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for MqttRequestExecutor {
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
            NodeData::MqttRequest(config) => config.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 MqttRequestConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换: connection_id, request_topic, response_topic, payload, correlation_data
        let connection_id =
            replace_variables(&config.connection_id, context).map_err(|e| {
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

        let request_topic =
            replace_variables(&config.request_topic, context).map_err(|e| {
                let error_msg = format!("请求主题变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?;

        let response_topic =
            replace_variables(&config.response_topic, context).map_err(|e| {
                let error_msg = format!("响应主题变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?;

        let resolved_payload = match &config.payload {
            Some(p) => Some(replace_variables(p, context).map_err(|e| {
                let error_msg = format!("载荷变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?),
            None => None,
        };

        let resolved_correlation_data = match &config.correlation_data {
            Some(cd) => Some(replace_variables(cd, context).map_err(|e| {
                let error_msg = format!("关联数据变量替换失败: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                error_msg
            })?),
            // 3. 自动生成 correlation_data（UUID）
            None => Some(uuid::Uuid::new_v4().to_string()),
        };

        // 4. 日志：开始请求
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mqtt_request, 连接ID: {}, 请求主题: {}, 响应主题: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            request_topic,
            response_topic
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "MQTT 请求/响应节点 {} 开始执行 (请求: {} → 响应: {})",
                    node.name, request_topic, response_topic
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

        // 5. 构建请求配置（使用变量替换后的值 + 自动生成的 correlation_data）
        let resolved_config = MqttRequestConfig {
            connection_id: connection_id.clone(),
            request_topic: request_topic.clone(),
            response_topic: response_topic.clone(),
            payload: resolved_payload,
            correlation_data: resolved_correlation_data.clone(),
            ..config
        };

        // 6. 调用 provider.request() — 处理完整的请求/响应生命周期
        let request_result = self
            .mqtt_manager
            .request(&connection_id, &resolved_config)
            .await;

        match request_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[MQTT] 请求/响应成功: connection_id={}, request_topic={}, response_topic={}, correlation_data={} ({}ms)",
                        connection_id,
                        request_topic,
                        response_topic,
                        resolved_correlation_data.as_deref().unwrap_or(""),
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[MQTT] 请求/响应返回错误: connection_id={}, request_topic={}, error={:?}",
                        connection_id,
                        request_topic,
                        output.error
                    );
                }

                // 7. 构建输出数据
                let response_messages = build_response_messages_json(&output.messages);
                let timing_json = build_timing_json(&output.timing, duration);

                let output_data = json!({
                    "success": output.success,
                    "request_topic": request_topic,
                    "response_topic": response_topic,
                    "correlation_data": resolved_correlation_data,
                    "qos": config.qos.unwrap_or(0),
                    "response_messages": response_messages,
                    "response_count": output.messages.len(),
                    "timing": timing_json,
                    "error": output.error,
                    "request": {
                        "connection_id": connection_id,
                        "request_topic": request_topic,
                        "response_topic": response_topic,
                        "correlation_data": resolved_correlation_data,
                        "qos": config.qos.unwrap_or(0),
                        "timeout_ms": config.timeout_ms,
                    },
                });

                // 8. 日志：请求完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_msg = if output.success {
                    format!(
                        "MQTT 请求/响应成功: {} → {} ({} 条响应, {}ms)",
                        request_topic,
                        response_topic,
                        output.messages.len(),
                        duration.as_millis()
                    )
                } else {
                    let err = output.error.as_deref().unwrap_or("未知错误");
                    format!(
                        "MQTT 请求/响应失败: {} → {} - {} ({}ms)",
                        request_topic, response_topic, err, duration.as_millis()
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_msg,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9. 端口路由：成功走 true，失败/超时走 false
                if output.success {
                    route_to_true(node, connections, output_data)
                } else {
                    route_to_false(node, connections, output_data)
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[MQTT] 请求/响应失败: connection_id={}, request_topic={}, error={}",
                    connection_id,
                    request_topic,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MQTT 请求/响应失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出（包含请求信息便于调试）
                let error_output = json!({
                    "success": false,
                    "request_topic": request_topic,
                    "response_topic": response_topic,
                    "correlation_data": resolved_correlation_data,
                    "qos": config.qos.unwrap_or(0),
                    "response_messages": [],
                    "response_count": 0,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                    "request": {
                        "connection_id": connection_id,
                        "request_topic": request_topic,
                        "response_topic": response_topic,
                        "correlation_data": resolved_correlation_data,
                        "qos": config.qos.unwrap_or(0),
                        "timeout_ms": config.timeout_ms,
                    },
                });

                route_to_false(node, connections, error_output)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 输出构建辅助函数
// ---------------------------------------------------------------------------

/// 将接收到的消息列表构建为 JSON 数组
fn build_response_messages_json(messages: &[MqttReceivedMessage]) -> serde_json::Value {
    let messages_json: Vec<serde_json::Value> = messages
        .iter()
        .map(|msg| {
            let payload_str = String::from_utf8_lossy(&msg.payload).to_string();
            json!({
                "topic": msg.topic,
                "payload": payload_str,
                "qos": msg.qos,
                "retain": msg.retain,
                "payload_format": msg.payload_format,
                "timestamp_ms": msg.timestamp_ms,
            })
        })
        .collect();
    json!(messages_json)
}

/// 构建时间信息 JSON
fn build_timing_json(
    timing: &Option<flow_engine_core::traits::MqttTimingInfo>,
    elapsed: std::time::Duration,
) -> serde_json::Value {
    match timing {
        Some(t) => json!({
            "total_ms": t.total_ms,
            "first_message_ms": t.first_message_ms,
        }),
        None => json!({
            "total_ms": elapsed.as_millis() as u64,
        }),
    }
}

// ---------------------------------------------------------------------------
// 端口路由辅助函数
// ---------------------------------------------------------------------------

/// 成功路由到 true 端口
fn route_to_true(
    node: &Node,
    connections: &[Connection],
    output_data: serde_json::Value,
) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| {
            conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
        })
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    let next_node_ids = if filtered.is_empty() {
        vec!["true".to_string()]
    } else {
        filtered
    };

    Ok((next_node_ids, Some(output_data)))
}

/// 失败路由到 false 端口
fn route_to_false(
    node: &Node,
    connections: &[Connection],
    output_data: serde_json::Value,
) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| {
            conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
        })
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
    use flow_engine_core::node_config::MqttRequestConfig;
    use flow_engine_core::traits::{
        MqttConnectOutput, MqttConnectionInfo, MqttMessageOutput, MqttSubscribeOutput,
        MqttTimingInfo,
    };

    // ===== Mock Provider =====

    /// 模拟 MQTT Provider（请求成功，返回一条响应消息）
    struct MockRequestSuccessProvider {
        response_messages: Vec<MqttReceivedMessage>,
    }

    impl MockRequestSuccessProvider {
        fn new() -> Self {
            Self {
                response_messages: vec![MqttReceivedMessage {
                    topic: "response/device-001".to_string(),
                    payload: b"ok".to_vec(),
                    qos: 0,
                    retain: false,
                    payload_format: None,
                    timestamp_ms: 1700000000000,
                }],
            }
        }
    }

    #[async_trait::async_trait]
    impl MqttProvider for MockRequestSuccessProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttConnectConfig,
        ) -> Result<MqttConnectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttPublishConfig,
        ) -> Result<flow_engine_core::traits::MqttPublishOutput, String> {
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttSubscribeConfig,
        ) -> Result<MqttSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn unsubscribe(
            &self,
            _uuid: &str,
            _connection_id: &str,
            _topic: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn wait_message(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttWaitMessageConfig,
        ) -> Result<MqttMessageOutput, String> {
            Err("not implemented".to_string())
        }

        async fn request(
            &self,
            _uuid: &str,
            _config: &MqttRequestConfig,
        ) -> Result<MqttMessageOutput, String> {
            Ok(MqttMessageOutput {
                success: true,
                messages: self.response_messages.clone(),
                timing: Some(MqttTimingInfo {
                    total_ms: 50,
                    first_message_ms: Some(30),
                }),
                error: None,
            })
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<MqttConnectionInfo> {
            vec![]
        }
    }

    /// 模拟 MQTT Provider（请求超时，返回空消息）
    struct MockRequestTimeoutProvider;

    #[async_trait::async_trait]
    impl MqttProvider for MockRequestTimeoutProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttConnectConfig,
        ) -> Result<MqttConnectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttPublishConfig,
        ) -> Result<flow_engine_core::traits::MqttPublishOutput, String> {
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttSubscribeConfig,
        ) -> Result<MqttSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn unsubscribe(
            &self,
            _uuid: &str,
            _connection_id: &str,
            _topic: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn wait_message(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttWaitMessageConfig,
        ) -> Result<MqttMessageOutput, String> {
            Err("not implemented".to_string())
        }

        async fn request(
            &self,
            _uuid: &str,
            _config: &MqttRequestConfig,
        ) -> Result<MqttMessageOutput, String> {
            // 模拟超时：success=false，空消息列表
            Ok(MqttMessageOutput {
                success: false,
                messages: vec![],
                timing: Some(MqttTimingInfo {
                    total_ms: 30000,
                    first_message_ms: None,
                }),
                error: Some("响应超时".to_string()),
            })
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<MqttConnectionInfo> {
            vec![]
        }
    }

    /// 模拟 MQTT Provider（请求异常）
    struct MockRequestErrorProvider;

    #[async_trait::async_trait]
    impl MqttProvider for MockRequestErrorProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttConnectConfig,
        ) -> Result<MqttConnectOutput, String> {
            Err("not implemented".to_string())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttPublishConfig,
        ) -> Result<flow_engine_core::traits::MqttPublishOutput, String> {
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttSubscribeConfig,
        ) -> Result<MqttSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn unsubscribe(
            &self,
            _uuid: &str,
            _connection_id: &str,
            _topic: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn wait_message(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttWaitMessageConfig,
        ) -> Result<MqttMessageOutput, String> {
            Err("not implemented".to_string())
        }

        async fn request(
            &self,
            _uuid: &str,
            _config: &MqttRequestConfig,
        ) -> Result<MqttMessageOutput, String> {
            Err("连接不存在".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }

        fn list_connections(&self) -> Vec<MqttConnectionInfo> {
            vec![]
        }
    }

    // ===== 辅助函数 =====

    /// 创建测试节点
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        request_topic: &str,
        response_topic: &str,
        payload: Option<&str>,
        correlation_data: Option<&str>,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mqtt_request".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttRequest(MqttRequestConfig {
                r#type: "mqtt_request".to_string(),
                connection_id: connection_id.to_string(),
                request_topic: request_topic.to_string(),
                response_topic: response_topic.to_string(),
                payload: payload.map(|s| s.to_string()),
                qos: Some(0),
                correlation_data: correlation_data.map(|s| s.to_string()),
                timeout_ms: Some(30000),
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
            imap_manager: None, redis_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    // ===== 测试用例 =====

    /// 测试：请求/响应成功时输出 JSON 结构正确，路由到 true 端口
    #[tokio::test]
    async fn test_request_success_output_structure() {
        let mqtt_manager = Arc::new(MockRequestSuccessProvider::new());
        let executor = MqttRequestExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-1",
            "测试请求",
            "conn-001",
            "request/device",
            "response/device",
            Some("ping"),
            None, // 不提供 correlation_data，由执行器自动生成
        );
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
        assert_eq!(data["request_topic"], "request/device");
        assert_eq!(data["response_topic"], "response/device");
        // correlation_data 应自动生成（UUID 格式）
        let cd = data["correlation_data"].as_str().expect("应有 correlation_data");
        assert!(!cd.is_empty(), "correlation_data 不应为空");
        assert!(
            uuid::Uuid::parse_str(cd).is_ok(),
            "correlation_data 应为有效的 UUID"
        );
        // 响应消息
        assert_eq!(data["response_count"], 1);
        let msgs = data["response_messages"].as_array().expect("应为数组");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["topic"], "response/device-001");
        assert_eq!(msgs[0]["payload"], "ok");
        // timing 字段
        assert!(data["timing"]["total_ms"].is_number());
        // request 字段
        assert_eq!(data["request"]["connection_id"], "conn-001");
        assert_eq!(data["request"]["request_topic"], "request/device");
    }

    /// 测试：超时时路由到 false 端口
    #[tokio::test]
    async fn test_request_timeout_routes_to_false_port() {
        let mqtt_manager = Arc::new(MockRequestTimeoutProvider);
        let executor = MqttRequestExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-2",
            "超时测试",
            "conn-001",
            "request/slow",
            "response/slow",
            Some("ping"),
            Some("corr-123"), // 显式提供 correlation_data
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 超时应路由到 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "响应超时");
        assert_eq!(data["response_count"], 0);
        assert_eq!(data["response_messages"].as_array().expect("应为数组").len(), 0);
        // 提供的 correlation_data 应保留
        assert_eq!(data["correlation_data"], "corr-123");
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_request_wrong_node_data_type() {
        let mqtt_manager = Arc::new(MockRequestSuccessProvider::new());
        let executor = MqttRequestExecutor::new(mqtt_manager);

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
        assert!(result.unwrap_err().contains("MqttRequestConfig"));
    }

    /// 测试：变量替换在 connection_id、request_topic、response_topic、payload 中生效
    #[tokio::test]
    async fn test_request_with_variable_replacement() {
        let mqtt_manager = Arc::new(MockRequestSuccessProvider::new());
        let executor = MqttRequestExecutor::new(mqtt_manager);

        let node = create_test_node(
            "test-var",
            "变量替换测试",
            "{{conn_id}}",
            "device/{{device_id}}/request",
            "device/{{device_id}}/response",
            Some("{{action}}"),
            None,
        );

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("mqtt-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "device_id".to_string(),
            json!("sensor-001"),
            String::new(),
        );
        context
            .variables
            .set_variable("action".to_string(), json!("read"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        let data = output_data.unwrap();

        // 验证变量替换生效
        assert_eq!(data["request_topic"], "device/sensor-001/request");
        assert_eq!(data["response_topic"], "device/sensor-001/response");
        assert_eq!(data["request"]["connection_id"], "mqtt-connection-123");
        assert_eq!(data["request"]["request_topic"], "device/sensor-001/request");
        assert_eq!(data["request"]["response_topic"], "device/sensor-001/response");
    }

    /// 测试：连接异常时路由到 false 端口并包含错误信息
    #[tokio::test]
    async fn test_request_error_routes_to_false() {
        let mqtt_manager = Arc::new(MockRequestErrorProvider);
        let executor = MqttRequestExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-error",
            "异常测试",
            "nonexistent-conn",
            "request/topic",
            "response/topic",
            Some("ping"),
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
        assert_eq!(data["error"], "连接不存在");
        assert_eq!(data["response_count"], 0);
    }

    /// 测试：显式提供 correlation_data 时保持原值不变
    #[tokio::test]
    async fn test_request_explicit_correlation_data_preserved() {
        let mqtt_manager = Arc::new(MockRequestSuccessProvider::new());
        let executor = MqttRequestExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-corr",
            "关联数据测试",
            "conn-001",
            "req/topic",
            "resp/topic",
            None,
            Some("my-custom-correlation-id"),
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        // 显式提供的 correlation_data 应保持原值
        assert_eq!(data["correlation_data"], "my-custom-correlation-id");
        assert_eq!(data["request"]["correlation_data"], "my-custom-correlation-id");
    }

    /// 测试：连接端口路由
    #[tokio::test]
    async fn test_request_with_connections_routing() {
        let mqtt_manager = Arc::new(MockRequestSuccessProvider::new());
        let executor = MqttRequestExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-route",
            "路由测试",
            "conn-001",
            "req/topic",
            "resp/topic",
            Some("ping"),
            None,
        );
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
}
