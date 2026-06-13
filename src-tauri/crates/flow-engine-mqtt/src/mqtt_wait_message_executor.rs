//! MQTT 等待消息执行器
//!
//! 在已订阅的连接上等待匹配的消息。
//! 支持 match_condition：json_path / regex / expression 三种匹配模式。
//! 超时或无匹配消息时路由到 false 端口。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{MqttWaitMessageConfig, NodeData};
use flow_engine_core::traits::MqttProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// MQTT 等待消息节点执行器
pub struct MqttWaitMessageExecutor {
    mqtt_manager: Arc<dyn MqttProvider>,
}

impl MqttWaitMessageExecutor {
    /// 创建新的 MQTT 等待消息执行器
    pub fn new(mqtt_manager: Arc<dyn MqttProvider>) -> Self {
        Self { mqtt_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for MqttWaitMessageExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取 MqttWaitMessageConfig
        let config = match &node.data {
            NodeData::MqttWaitMessage(cfg) => cfg.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 MqttWaitMessageConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换：connection_id 和 topic
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            format!("connection_id 变量替换失败: {}", e)
        })?;

        let topic = replace_variables(&config.topic, context).map_err(|e| {
            format!("topic 变量替换失败: {}", e)
        })?;

        // 3. 日志：开始等待
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mqtt_wait_message, topic: {}, connection_id: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            topic,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "MQTT 等待消息节点 {} 开始执行: topic={}",
                    node.name, topic
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建等待配置（使用替换后的值）
        let wait_config = MqttWaitMessageConfig {
            r#type: config.r#type.clone(),
            connection_id: connection_id.clone(),
            topic: topic.clone(),
            match_condition: config.match_condition.clone(),
            timeout_ms: config.timeout_ms,
            max_messages: config.max_messages,
        };

        // 5. 调用 provider.wait_message()
        let wait_result = self.mqtt_manager.wait_message(&node.uuid, &wait_config).await;

        match wait_result {
            Ok(output) => {
                let duration = start_time.elapsed();
                let matched_count = output.messages.len();

                // 6. 构建输出 JSON
                let output_data = json!({
                    "success": output.success,
                    "messages": output.messages.iter().map(|msg| {
                        json!({
                            "topic": msg.topic,
                            "payload": msg.payload.to_vec(),
                            "payload_text": String::from_utf8_lossy(&msg.payload).to_string(),
                            "qos": msg.qos,
                            "retain": msg.retain,
                            "payload_format": msg.payload_format,
                            "timestamp_ms": msg.timestamp_ms,
                        })
                    }).collect::<Vec<_>>(),
                    "matched_count": matched_count,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                        "first_message_ms": output.timing.as_ref().and_then(|t| t.first_message_ms),
                    },
                    "request": {
                        "topic": topic,
                        "connection_id": connection_id,
                        "match_condition": config.match_condition,
                        "timeout_ms": config.timeout_ms,
                        "max_messages": config.max_messages,
                    },
                    "error": output.error,
                });

                if output.success && matched_count > 0 {
                    log::info!(
                        "[MQTT] 等待消息成功: topic={}, 匹配 {} 条消息, 耗时={}ms",
                        wait_config.topic,
                        matched_count,
                        duration.as_millis()
                    );

                    // 7. 日志：等待成功
                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Info,
                            message: format!(
                                "MQTT 等待消息成功: topic={}, 匹配 {} 条消息 ({}ms)",
                                wait_config.topic,
                                matched_count,
                                duration.as_millis()
                            ),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0),
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;

                    // 8. 匹配到消息 → 走 true 端口
                    let next_node_ids: Vec<String> = {
                        let filtered: Vec<String> = connections
                            .iter()
                            .filter(|conn| {
                                conn.source_node_uuid == node.uuid
                                    && conn.source_port_id == "true"
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
                } else {
                    // 成功但无匹配消息（超时）→ 走 false 端口
                    log::warn!(
                        "[MQTT] 等待消息超时或无匹配: topic={}, 匹配 {} 条消息, 耗时={}ms",
                        wait_config.topic,
                        matched_count,
                        duration.as_millis()
                    );

                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Warn,
                            message: format!(
                                "MQTT 等待消息超时或无匹配: topic={}, 耗时={}ms",
                                wait_config.topic,
                                duration.as_millis()
                            ),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0),
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;

                    // 检查 on_error 策略
                    if !node.on_error.as_ref().map_or(true, |s| {
                        *s == ExecutionErrorStrategy::RouteToFalse
                    }) {
                        let error_msg = output_data
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("等待消息超时或无匹配")
                            .to_string();
                        return Err(error_msg);
                    }

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

                    Ok((next_node_ids, Some(output_data)))
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();

                log::error!(
                    "[MQTT] 等待消息异常: uuid={}, topic={}, error={}",
                    node.uuid,
                    topic,
                    e
                );

                // 日志：等待异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MQTT 等待消息异常: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出
                let error_output = json!({
                    "success": false,
                    "messages": [],
                    "matched_count": 0,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "request": {
                        "topic": topic,
                        "connection_id": connection_id,
                        "match_condition": config.match_condition,
                        "timeout_ms": config.timeout_ms,
                        "max_messages": config.max_messages,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| {
                    *s == ExecutionErrorStrategy::RouteToFalse
                }) {
                    return Err(e);
                }

                // 失败时走 false 端口
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
    use flow_engine_core::node_config::MqttWaitMessageConfig;
    use flow_engine_core::traits::{
        MqttConnectOutput, MqttConnectionInfo, MqttMessageOutput, MqttPublishOutput,
        MqttReceivedMessage, MqttSubscribeOutput, MqttTimingInfo,
    };

    // ---------------------------------------------------------------------------
    // 辅助工具
    // ---------------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, topic: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mqtt_wait_message".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttWaitMessage(MqttWaitMessageConfig {
                r#type: "mqtt_wait_message".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
                match_condition: None,
                timeout_ms: Some(5000),
                max_messages: None,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建带匹配条件的测试节点
    fn create_test_node_with_condition(
        uuid: &str,
        name: &str,
        topic: &str,
        connection_id: &str,
        match_type: &str,
        expression: &str,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mqtt_wait_message".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttWaitMessage(MqttWaitMessageConfig {
                r#type: "mqtt_wait_message".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
                match_condition: Some(json!({
                    "match_type": match_type,
                    "expression": expression,
                })),
                timeout_ms: Some(5000),
                max_messages: None,
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
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
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
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None,
        }
    }

    // ---------------------------------------------------------------------------
    // Mock MqttProvider
    // ---------------------------------------------------------------------------

    /// 模拟 MqttProvider（控制 wait_message 返回值）
    struct MockMqttProvider {
        wait_output: MqttMessageOutput,
        wait_error: Option<String>,
    }

    impl MockMqttProvider {
        fn new_success(messages: Vec<MqttReceivedMessage>) -> Self {
            Self {
                wait_output: MqttMessageOutput {
                    success: true,
                    messages,
                    timing: Some(MqttTimingInfo {
                        total_ms: 100,
                        first_message_ms: Some(50),
                    }),
                    error: None,
                },
                wait_error: None,
            }
        }

        fn new_timeout() -> Self {
            Self {
                wait_output: MqttMessageOutput {
                    success: true,
                    messages: vec![],
                    timing: Some(MqttTimingInfo {
                        total_ms: 5000,
                        first_message_ms: None,
                    }),
                    error: Some("等待超时".to_string()),
                },
                wait_error: None,
            }
        }

        fn new_error(error: &str) -> Self {
            Self {
                wait_output: MqttMessageOutput {
                    success: false,
                    messages: vec![],
                    timing: None,
                    error: Some(error.to_string()),
                },
                wait_error: Some(error.to_string()),
            }
        }
    }

    #[async_trait::async_trait]
    impl MqttProvider for MockMqttProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttConnectConfig,
        ) -> Result<MqttConnectOutput, String> {
            Ok(MqttConnectOutput {
                success: true,
                broker_info: None,
                reason_code: Some(0),
                error: None,
            })
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttPublishConfig,
        ) -> Result<MqttPublishOutput, String> {
            Ok(MqttPublishOutput {
                success: true,
                reason_code: Some(0),
                error: None,
            })
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttSubscribeConfig,
        ) -> Result<MqttSubscribeOutput, String> {
            Ok(MqttSubscribeOutput {
                success: true,
                reason_code: Some(0),
                messages: vec![],
                error: None,
            })
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
            match &self.wait_error {
                Some(e) => Err(e.clone()),
                None => Ok(self.wait_output.clone()),
            }
        }

        async fn request(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttRequestConfig,
        ) -> Result<MqttMessageOutput, String> {
            Err("not implemented".to_string())
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

    // ---------------------------------------------------------------------------
    // 测试用例
    // ---------------------------------------------------------------------------

    /// 测试：等待消息成功，匹配到消息时路由到 true 端口
    #[tokio::test]
    async fn test_wait_message_success_routes_to_true() {
        let messages = vec![MqttReceivedMessage {
            topic: "test/response".to_string(),
            payload: b"hello".to_vec(),
            qos: 1,
            retain: false,
            payload_format: None,
            timestamp_ms: 1700000000000,
        }];
        let provider = Arc::new(MockMqttProvider::new_success(messages));
        let executor = MqttWaitMessageExecutor::new(provider);
        let node = create_test_node("wait-1", "等待消息", "test/response", "conn-1");
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
        assert_eq!(data["matched_count"], 1);
        assert_eq!(data["request"]["topic"], "test/response");
        assert_eq!(data["request"]["connection_id"], "conn-1");

        // 验证消息内容
        let msgs = data["messages"].as_array().expect("messages 应为数组");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["topic"], "test/response");
        assert_eq!(msgs[0]["payload_text"], "hello");
        assert_eq!(msgs[0]["qos"], 1);

        // 验证 timing
        assert!(data["timing"]["total_ms"].is_number());
        assert_eq!(data["timing"]["first_message_ms"], 50);
    }

    /// 测试：等待消息超时/无匹配时路由到 false 端口
    #[tokio::test]
    async fn test_wait_message_timeout_routes_to_false() {
        let provider = Arc::new(MockMqttProvider::new_timeout());
        let executor = MqttWaitMessageExecutor::new(provider);
        let node = create_test_node("wait-2", "超时测试", "test/timeout", "conn-1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        // 超时时 success=true（provider 调用本身没出错），但 matched_count=0
        assert_eq!(data["success"], true);
        assert_eq!(data["matched_count"], 0);
        assert_eq!(data["messages"].as_array().map(|a| a.len()), Some(0));
    }

    /// 测试：provider 返回错误时路由到 false 端口
    #[tokio::test]
    async fn test_wait_message_error_routes_to_false() {
        let provider = Arc::new(MockMqttProvider::new_error("连接 'conn-missing' 不存在"));
        let executor = MqttWaitMessageExecutor::new(provider);
        let node = create_test_node("wait-3", "错误测试", "test/topic", "conn-missing");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "连接 'conn-missing' 不存在");
        assert_eq!(data["matched_count"], 0);
    }

    /// 测试：节点数据类型不匹配时返回错误
    #[tokio::test]
    async fn test_wait_message_wrong_node_data_type() {
        let provider = Arc::new(MockMqttProvider::new_success(vec![]));
        let executor = MqttWaitMessageExecutor::new(provider);

        let node = Node {
            uuid: "wrong-type".to_string(),
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
        assert!(result.unwrap_err().contains("MqttWaitMessageConfig"));
    }

    /// 测试：变量替换在 topic 和 connection_id 中生效
    #[tokio::test]
    async fn test_wait_message_variable_replacement() {
        let messages = vec![MqttReceivedMessage {
            topic: "sensors/temperature".to_string(),
            payload: b"25.3".to_vec(),
            qos: 0,
            retain: false,
            payload_format: None,
            timestamp_ms: 1700000000000,
        }];
        let provider = Arc::new(MockMqttProvider::new_success(messages));
        let executor = MqttWaitMessageExecutor::new(provider);

        let node = Node {
            uuid: "wait-var".to_string(),
            node_type: "mqtt_wait_message".to_string(),
            name: "变量替换".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttWaitMessage(MqttWaitMessageConfig {
                r#type: "mqtt_wait_message".to_string(),
                connection_id: "{{conn_id}}".to_string(),
                topic: "sensors/{{sensor_type}}".to_string(),
                match_condition: None,
                timeout_ms: Some(3000),
                max_messages: None,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("my-connection"),
            String::new(),
        );
        context.variables.set_variable(
            "sensor_type".to_string(),
            json!("temperature"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);

        let data = output_data.unwrap();
        assert_eq!(data["request"]["topic"], "sensors/temperature");
        assert_eq!(data["request"]["connection_id"], "my-connection");
    }

    /// 测试：带 match_condition 的等待消息
    #[tokio::test]
    async fn test_wait_message_with_match_condition() {
        let messages = vec![MqttReceivedMessage {
            topic: "response/data".to_string(),
            payload: br#"{"status":"ok","value":42}"#.to_vec(),
            qos: 1,
            retain: false,
            payload_format: None,
            timestamp_ms: 1700000000000,
        }];
        let provider = Arc::new(MockMqttProvider::new_success(messages));
        let executor = MqttWaitMessageExecutor::new(provider);

        let node = create_test_node_with_condition(
            "wait-cond",
            "条件匹配",
            "response/data",
            "conn-1",
            "json_path",
            "$.status == 'ok'",
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        assert_eq!(data["success"], true);
        assert_eq!(data["matched_count"], 1);
        // 验证 match_condition 被传递到输出中
        let mc = &data["request"]["match_condition"];
        assert_eq!(mc["match_type"], "json_path");
        assert_eq!(mc["expression"], "$.status == 'ok'");
    }

    /// 测试：连接端口路由
    #[tokio::test]
    async fn test_wait_message_connection_routing() {
        let messages = vec![MqttReceivedMessage {
            topic: "test/topic".to_string(),
            payload: b"data".to_vec(),
            qos: 0,
            retain: false,
            payload_format: None,
            timestamp_ms: 1700000000000,
        }];
        let provider = Arc::new(MockMqttProvider::new_success(messages));
        let executor = MqttWaitMessageExecutor::new(provider);
        let node = create_test_node("wait-route", "路由测试", "test/topic", "conn-1");
        let mut context = create_context("test-flow");

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "wait-route".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-node"]);
    }
}
