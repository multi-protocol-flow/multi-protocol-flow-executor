//! MQTT 订阅执行器
//!
//! 实现 MQTT 主题订阅功能，支持共享订阅和消息收集。
//! 订阅 topic → 等待消息收集（max_messages + collect_timeout_ms）→ 返回收集到的消息列表。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{MqttSubscribeConfig, NodeData};
use flow_engine_core::traits::MqttProvider;
use serde_json::json;

/// MQTT 订阅节点执行器
pub struct MqttSubscribeExecutor {
    mqtt_manager: Arc<dyn MqttProvider>,
}

impl MqttSubscribeExecutor {
    /// 创建新的 MQTT 订阅执行器
    pub fn new(mqtt_manager: Arc<dyn MqttProvider>) -> Self {
        Self { mqtt_manager }
    }
}

/// 构建共享订阅 topic
///
/// 如果配置了 `share_group`，将 topic 格式化为 `$share/{group}/{original_topic}`。
fn build_shared_topic(topic: &str, share_group: Option<&String>) -> String {
    match share_group {
        Some(group) if !group.is_empty() => format!("$share/{}/{}", group, topic),
        _ => topic.to_string(),
    }
}

/// 根据 output_mode 过滤消息列表
///
/// - "first": 仅返回第一条消息
/// - "all" 或其他: 返回全部消息
fn apply_output_mode(
    messages: Vec<flow_engine_core::traits::MqttReceivedMessage>,
    output_mode: Option<&str>,
) -> Vec<flow_engine_core::traits::MqttReceivedMessage> {
    match output_mode {
        Some("first") => {
            if messages.is_empty() {
                messages
            } else {
                messages.into_iter().take(1).collect()
            }
        }
        _ => messages,
    }
}

#[async_trait::async_trait]
impl NodeExecutor for MqttSubscribeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取 MqttSubscribeConfig
        let config = match &node.data {
            NodeData::MqttSubscribe(cfg) => cfg.clone(),
            _ => {
                return Err(format!(
                    "节点数据类型不是 MqttSubscribeConfig: {}",
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

        // 3. 构建共享订阅 topic
        let effective_topic = build_shared_topic(&topic, config.share_group.as_ref());

        // 4. 日志：开始订阅
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mqtt_subscribe, topic: {}, connection_id: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            effective_topic,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "MQTT 订阅节点 {} 开始执行: topic={}",
                    node.name, effective_topic
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

        // 5. 构建订阅配置（使用替换后的值和共享 topic）
        let subscribe_config = MqttSubscribeConfig {
            r#type: config.r#type.clone(),
            connection_id: connection_id.clone(),
            topic: effective_topic.clone(),
            qos: config.qos,
            max_messages: config.max_messages,
            collect_timeout_ms: config.collect_timeout_ms,
            output_mode: config.output_mode.clone(),
            no_local: config.no_local,
            retain_as_published: config.retain_as_published,
            retain_handling: config.retain_handling,
            subscription_identifier: config.subscription_identifier,
            share_group: config.share_group.clone(),
        };

        // 6. 调用 provider.subscribe()
        let subscribe_result = self
            .mqtt_manager
            .subscribe(&node.uuid, &subscribe_config)
            .await;

        match subscribe_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                // 7. 根据 output_mode 过滤消息
                let filtered_messages =
                    apply_output_mode(output.messages, config.output_mode.as_deref());

                // 8. 构建输出 JSON
                let output_data = json!({
                    "success": output.success,
                    "reason_code": output.reason_code,
                    "messages": filtered_messages.iter().map(|msg| {
                        json!({
                            "topic": msg.topic,
                            "payload": msg.payload.iter().map(|b| *b as u8).collect::<Vec<_>>(),
                            "payload_text": String::from_utf8_lossy(&msg.payload).to_string(),
                            "qos": msg.qos,
                            "retain": msg.retain,
                            "payload_format": msg.payload_format,
                            "timestamp_ms": msg.timestamp_ms,
                        })
                    }).collect::<Vec<_>>(),
                    "message_count": filtered_messages.len(),
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "request": {
                        "topic": effective_topic,
                        "connection_id": connection_id,
                        "qos": config.qos.unwrap_or(0),
                        "max_messages": config.max_messages,
                        "collect_timeout_ms": config.collect_timeout_ms,
                        "output_mode": config.output_mode,
                    },
                });

                log::info!(
                    "[MQTT] 订阅成功: topic={}, 消息数={}, 耗时={}ms",
                    effective_topic,
                    filtered_messages.len(),
                    duration.as_millis()
                );

                // 9. 日志：订阅完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "MQTT 订阅成功: topic={}, 收到 {} 条消息 ({}ms)",
                            effective_topic,
                            filtered_messages.len(),
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

                // 10. 成功时走 true 端口
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
            }
            Err(e) => {
                let duration = start_time.elapsed();

                log::error!(
                    "[MQTT] 订阅失败: uuid={}, topic={}, error={}",
                    node.uuid,
                    effective_topic,
                    e
                );

                // 日志：订阅失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MQTT 订阅失败: {}", e),
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
                    "reason_code": serde_json::Value::Null,
                    "messages": [],
                    "message_count": 0,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "request": {
                        "topic": effective_topic,
                        "connection_id": connection_id,
                        "qos": config.qos.unwrap_or(0),
                    },
                    "error": e,
                });

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
    use flow_engine_core::traits::{
        MqttConnectOutput, MqttConnectionInfo, MqttMessageOutput,
        MqttPublishOutput, MqttReceivedMessage, MqttSubscribeOutput,
    };

    // ---------------------------------------------------------------------------
    // 辅助工具
    // ---------------------------------------------------------------------------

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, topic: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mqtt_subscribe".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttSubscribe(MqttSubscribeConfig {
                r#type: "mqtt_subscribe".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
                qos: Some(1),
                max_messages: Some(10),
                collect_timeout_ms: Some(5000),
                output_mode: Some("all".to_string()),
                no_local: None,
                retain_as_published: None,
                retain_handling: None,
                subscription_identifier: None,
                share_group: None,
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

    /// 模拟成功订阅的 MqttProvider
    struct MockMqttProvider {
        subscribe_output: MqttSubscribeOutput,
    }

    impl MockMqttProvider {
        fn new_success(messages: Vec<MqttReceivedMessage>) -> Self {
            Self {
                subscribe_output: MqttSubscribeOutput {
                    success: true,
                    reason_code: Some(0),
                    messages,
                    error: None,
                },
            }
        }

        fn new_failure(error: String) -> Self {
            Self {
                subscribe_output: MqttSubscribeOutput {
                    success: false,
                    reason_code: None,
                    messages: vec![],
                    error: Some(error),
                },
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
            _config: &MqttSubscribeConfig,
        ) -> Result<MqttSubscribeOutput, String> {
            if self.subscribe_output.success {
                Ok(self.subscribe_output.clone())
            } else {
                Err(self
                    .subscribe_output
                    .error
                    .clone()
                    .unwrap_or_else(|| "未知错误".to_string()))
            }
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

    /// 测试：NodeData::MqttSubscribe 提取
    #[tokio::test]
    async fn test_subscribe_node_data_extraction() {
        let provider = Arc::new(MockMqttProvider::new_success(vec![]));
        let executor = MqttSubscribeExecutor::new(provider);
        let node = create_test_node("sub-1", "测试订阅", "test/topic", "conn-1");
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
        assert_eq!(data["request"]["topic"], "test/topic");
        assert_eq!(data["request"]["connection_id"], "conn-1");
        assert_eq!(data["message_count"], 0);
    }

    /// 测试：共享订阅 topic 格式化
    #[tokio::test]
    async fn test_shared_subscription_topic_formatting() {
        let provider = Arc::new(MockMqttProvider::new_success(vec![]));
        let executor = MqttSubscribeExecutor::new(provider);

        let node = Node {
            uuid: "sub-share-1".to_string(),
            node_type: "mqtt_subscribe".to_string(),
            name: "共享订阅".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttSubscribe(MqttSubscribeConfig {
                r#type: "mqtt_subscribe".to_string(),
                connection_id: "conn-1".to_string(),
                topic: "sensors/temperature".to_string(),
                qos: Some(1),
                max_messages: None,
                collect_timeout_ms: None,
                output_mode: Some("first".to_string()),
                no_local: None,
                retain_as_published: None,
                retain_handling: None,
                subscription_identifier: None,
                share_group: Some("group-a".to_string()),
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // 共享订阅 topic 应为 $share/group-a/sensors/temperature
        assert_eq!(
            data["request"]["topic"],
            "$share/group-a/sensors/temperature"
        );
    }

    /// 测试：output_mode first 仅返回第一条消息
    #[tokio::test]
    async fn test_output_mode_first_returns_first_message() {
        let messages = vec![
            MqttReceivedMessage {
                topic: "test/topic".to_string(),
                payload: b"hello".to_vec(),
                qos: 1,
                retain: false,
                payload_format: None,
                timestamp_ms: 1000,
            },
            MqttReceivedMessage {
                topic: "test/topic".to_string(),
                payload: b"world".to_vec(),
                qos: 0,
                retain: false,
                payload_format: None,
                timestamp_ms: 2000,
            },
        ];

        let provider = Arc::new(MockMqttProvider::new_success(messages));
        let executor = MqttSubscribeExecutor::new(provider);

        let node = Node {
            uuid: "sub-first-1".to_string(),
            node_type: "mqtt_subscribe".to_string(),
            name: "首条消息".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttSubscribe(MqttSubscribeConfig {
                r#type: "mqtt_subscribe".to_string(),
                connection_id: "conn-1".to_string(),
                topic: "test/topic".to_string(),
                qos: Some(1),
                max_messages: Some(10),
                collect_timeout_ms: Some(5000),
                output_mode: Some("first".to_string()),
                no_local: None,
                retain_as_published: None,
                retain_handling: None,
                subscription_identifier: None,
                share_group: None,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // output_mode=first 应仅返回 1 条消息
        assert_eq!(data["message_count"], 1);
        let msgs = data["messages"].as_array().expect("messages 应为数组");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["payload_text"], "hello");
    }

    /// 测试：output_mode all 返回全部消息
    #[tokio::test]
    async fn test_output_mode_all_returns_all_messages() {
        let messages = vec![
            MqttReceivedMessage {
                topic: "test/topic".to_string(),
                payload: b"msg1".to_vec(),
                qos: 1,
                retain: false,
                payload_format: None,
                timestamp_ms: 1000,
            },
            MqttReceivedMessage {
                topic: "test/topic".to_string(),
                payload: b"msg2".to_vec(),
                qos: 0,
                retain: true,
                payload_format: None,
                timestamp_ms: 2000,
            },
        ];

        let provider = Arc::new(MockMqttProvider::new_success(messages));
        let executor = MqttSubscribeExecutor::new(provider);

        let node = Node {
            uuid: "sub-all-1".to_string(),
            node_type: "mqtt_subscribe".to_string(),
            name: "全部消息".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttSubscribe(MqttSubscribeConfig {
                r#type: "mqtt_subscribe".to_string(),
                connection_id: "conn-1".to_string(),
                topic: "test/topic".to_string(),
                qos: Some(1),
                max_messages: Some(10),
                collect_timeout_ms: Some(5000),
                output_mode: Some("all".to_string()),
                no_local: None,
                retain_as_published: None,
                retain_handling: None,
                subscription_identifier: None,
                share_group: None,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // output_mode=all 应返回 2 条消息
        assert_eq!(data["message_count"], 2);
        let msgs = data["messages"].as_array().expect("messages 应为数组");
        assert_eq!(msgs.len(), 2);
    }

    /// 测试：订阅失败时走 false 端口
    #[tokio::test]
    async fn test_subscribe_failure_routes_to_false_port() {
        let provider = Arc::new(MockMqttProvider::new_failure(
            "连接 'conn-missing' 不存在".to_string(),
        ));
        let executor = MqttSubscribeExecutor::new(provider);
        let node = create_test_node("sub-fail-1", "失败订阅", "test/topic", "conn-missing");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    /// 测试：错误的节点数据类型
    #[tokio::test]
    async fn test_wrong_node_data_type() {
        let provider = Arc::new(MockMqttProvider::new_success(vec![]));
        let executor = MqttSubscribeExecutor::new(provider);

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
        assert!(result.unwrap_err().contains("MqttSubscribeConfig"));
    }

    /// 测试：变量替换
    #[tokio::test]
    async fn test_variable_replacement() {
        let provider = Arc::new(MockMqttProvider::new_success(vec![]));
        let executor = MqttSubscribeExecutor::new(provider);

        let node = Node {
            uuid: "sub-var-1".to_string(),
            node_type: "mqtt_subscribe".to_string(),
            name: "变量替换".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttSubscribe(MqttSubscribeConfig {
                r#type: "mqtt_subscribe".to_string(),
                connection_id: "{{conn_id}}".to_string(),
                topic: "sensors/{{sensor_type}}".to_string(),
                qos: Some(0),
                max_messages: None,
                collect_timeout_ms: None,
                output_mode: Some("first".to_string()),
                no_local: None,
                retain_as_published: None,
                retain_handling: None,
                subscription_identifier: None,
                share_group: None,
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
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["request"]["topic"], "sensors/temperature");
        assert_eq!(data["request"]["connection_id"], "my-connection");
    }

    // ---------------------------------------------------------------------------
    // 单元测试（纯函数）
    // ---------------------------------------------------------------------------

    #[test]
    fn test_build_shared_topic_with_group() {
        let topic = "sensors/data";
        let group = "my-group".to_string();
        assert_eq!(
            build_shared_topic(topic, Some(&group)),
            "$share/my-group/sensors/data"
        );
    }

    #[test]
    fn test_build_shared_topic_without_group() {
        let topic = "sensors/data";
        assert_eq!(build_shared_topic(topic, None), "sensors/data");
    }

    #[test]
    fn test_build_shared_topic_empty_group() {
        let topic = "sensors/data";
        let empty_group = "".to_string();
        assert_eq!(
            build_shared_topic(topic, Some(&empty_group)),
            "sensors/data"
        );
    }

    #[test]
    fn test_apply_output_mode_first() {
        let messages = vec![
            MqttReceivedMessage {
                topic: "t1".to_string(),
                payload: vec![1],
                qos: 0,
                retain: false,
                payload_format: None,
                timestamp_ms: 100,
            },
            MqttReceivedMessage {
                topic: "t2".to_string(),
                payload: vec![2],
                qos: 0,
                retain: false,
                payload_format: None,
                timestamp_ms: 200,
            },
        ];

        let result = apply_output_mode(messages, Some("first"));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].topic, "t1");
    }

    #[test]
    fn test_apply_output_mode_all() {
        let messages = vec![
            MqttReceivedMessage {
                topic: "t1".to_string(),
                payload: vec![1],
                qos: 0,
                retain: false,
                payload_format: None,
                timestamp_ms: 100,
            },
            MqttReceivedMessage {
                topic: "t2".to_string(),
                payload: vec![2],
                qos: 0,
                retain: false,
                payload_format: None,
                timestamp_ms: 200,
            },
        ];

        let result = apply_output_mode(messages, Some("all"));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_apply_output_mode_first_empty() {
        let messages: Vec<MqttReceivedMessage> = vec![];
        let result = apply_output_mode(messages, Some("first"));
        assert!(result.is_empty());
    }
}
