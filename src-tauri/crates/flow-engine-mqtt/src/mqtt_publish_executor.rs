//! MQTT 发布执行器
//!
//! 实现 MQTT 消息发布功能。
//! 参照 `mcp_call_tool_executor.rs` 模式实现。
//! 支持 payload 编码（text/json/hex/base64 → Vec<u8>）。

use std::collections::HashMap;
use flow_engine_core::error::ErrorDetail;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{MqttPublishConfig, NodeData};
use flow_engine_core::traits::MqttProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;

/// MQTT 发布节点执行器
pub struct MqttPublishExecutor {
    mqtt_manager: Arc<dyn MqttProvider>,
}

impl MqttPublishExecutor {
    /// 创建新的 MQTT 发布执行器
    pub fn new(mqtt_manager: Arc<dyn MqttProvider>) -> Self {
        Self { mqtt_manager }
    }
}

// ---------------------------------------------------------------------------
// Payload 编码辅助函数
// ---------------------------------------------------------------------------

/// 将 payload 字符串按指定格式编码为字节
///
/// 支持的格式:
/// - "text" / None → UTF-8 原始字节
/// - "json" → 校验 JSON 合法性后取 UTF-8 字节
/// - "hex" → 十六进制解码（自动去除 0x 前缀）
/// - "base64" → Base64 解码
fn encode_payload(payload: &str, format: Option<&str>) -> Result<Vec<u8>, ErrorDetail> {
    let fmt = format.unwrap_or("text");
    match fmt {
        "text" => Ok(payload.as_bytes().to_vec()),
        "json" => {
            // 校验 JSON 合法性
            let _: serde_json::Value =
                serde_json::from_str(payload).map_err(|e| ErrorDetail::execution_error(format!("JSON 格式无效: {}", e)))?;
            Ok(payload.as_bytes().to_vec())
        }
        "hex" => {
            let cleaned = payload.trim().trim_start_matches("0x").trim_start_matches("0X");
            hex_to_bytes(cleaned)
        }
        "base64" => base64_to_bytes(payload),
        _ => Err(ErrorDetail::execution_error(format!("不支持的 payload_format: '{}'", fmt))),
    }
}

/// 十六进制字符串 → 字节
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, ErrorDetail> {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }
    if !cleaned.len().is_multiple_of(2) {
        return Err(ErrorDetail::execution_error(format!(
            "十六进制字符串长度必须为偶数，当前长度: {}",
            cleaned.len()
        )));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| ErrorDetail::execution_error(format!("十六进制解码失败（位置 {}）: {}", i, e)))
        })
        .collect()
}

/// Base64 字符串 → 字节
fn base64_to_bytes(input: &str) -> Result<Vec<u8>, ErrorDetail> {
    // 手动实现 Base64 解码，避免引入额外依赖
    let trimmed = input.trim();
    let trimmed = trimmed.trim_end_matches('=');

    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut result = Vec::with_capacity(trimmed.len() * 3 / 4);

    let mut byte_acc: u32 = 0;
    let mut bits: u32 = 0;

    for ch in trimmed.chars() {
        let val = if ch.is_ascii_uppercase() {
            ch as u32 - 'A' as u32
        } else if ch.is_ascii_lowercase() {
            ch as u32 - 'a' as u32 + 26
        } else if ch.is_ascii_digit() {
            ch as u32 - '0' as u32 + 52
        } else if ch == '+' {
            62
        } else if ch == '/' {
            63
        } else {
            return Err(ErrorDetail::execution_error(format!("Base64 解码失败: 非法字符 '{}'", ch)));
        };

        byte_acc = (byte_acc << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            result.push((byte_acc >> bits) as u8);
            byte_acc &= (1 << bits) - 1;
        }
    }

    Ok(result)
}

#[async_trait::async_trait]
impl NodeExecutor for MqttPublishExecutor {
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
            NodeData::MqttPublish(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not MqttPublishConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换: connection_id, topic, payload
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
                ErrorDetail::execution_error(error_msg)
            })?;

        let topic = replace_variables(&config.topic, context).map_err(|e| {
            let error_msg = format!("Topic variable substitution failed: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let resolved_payload = match &config.payload {
            Some(p) => Some(replace_variables(p, context).map_err(|e| {
                let error_msg = format!("Payload variable substitution failed: {}", e);
                log::error!(
                    "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        // 3. 日志：开始发布
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mqtt_publish, 连接ID: {}, 主题: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            topic
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "MQTT 发布节点 {} 开始执行 (主题: {})",
                    node.name, topic
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("System time error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. Payload 编码
        let payload_bytes = match encode_payload(
            resolved_payload.as_deref().unwrap_or(""),
            config.payload_format.as_deref(),
        ) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::error!(
                    "[MQTT] Payload 编码失败: connection_id={}, topic={}, error={}",
                    connection_id,
                    topic,
                    e.message
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MQTT Payload 编码失败: {}", e.message),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("System time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "topic": topic,
                    "qos": config.qos.unwrap_or(0),
                    "retain": config.retain.unwrap_or(false),
                    "payload_size": 0,
                    "reason_code": null,
                    "error": e,
                    "request": {
                        "connection_id": connection_id,
                        "topic": topic,
                        "payload_format": config.payload_format,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                return route_to_false(node, connections, error_output);
            }
        };

        // 5. 构建发布配置（使用变量替换后的值）
        let publish_config = MqttPublishConfig {
            topic: topic.clone(),
            payload: Some(
                String::from_utf8_lossy(&payload_bytes).to_string(),
            ),
            qos: config.qos,
            retain: config.retain,
            payload_format: config.payload_format.clone(),
            ..config.clone()
        };

        // 6. 调用 provider.publish
        let publish_result = self
            .mqtt_manager
            .publish(&connection_id, &publish_config)
            .await;

        match publish_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[MQTT] 发布成功: connection_id={}, topic={} ({}ms)",
                        connection_id,
                        topic,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[MQTT] 发布返回错误: connection_id={}, topic={}, error={:?}",
                        connection_id,
                        topic,
                        output.error
                    );
                }

                // 7. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "topic": topic,
                    "qos": config.qos.unwrap_or(0),
                    "retain": config.retain.unwrap_or(false),
                    "payload_size": payload_bytes.len(),
                    "reason_code": output.reason_code,
                    "error": output.error,
                    "request": {
                        "connection_id": connection_id,
                        "topic": topic,
                        "payload_format": config.payload_format,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 8. 日志：发布完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_msg = if output.success {
                    format!(
                        "MQTT 发布成功: {} ({}ms)",
                        topic,
                        duration.as_millis()
                    )
                } else {
                    let err = output.error.as_deref().unwrap_or("Unknown error");
                    format!("MQTT publish failed: {} - {}", topic, err)
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_msg,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("System time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9. 端口路由
                if output.success {
                    route_to_true(node, connections, output_data)
                } else {
                    route_to_false(node, connections, output_data)
                }
            }
            Err(e) => {
                let duration = start_time.elapsed();
                log::error!(
                    "[MQTT] 发布失败: connection_id={}, topic={}, error={}",
                    connection_id,
                    topic,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MQTT publish failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("System time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let error_output = json!({
                    "success": false,
                    "topic": topic,
                    "qos": config.qos.unwrap_or(0),
                    "retain": config.retain.unwrap_or(false),
                    "payload_size": 0,
                    "reason_code": null,
                    "error": e,
                    "request": {
                        "connection_id": connection_id,
                        "topic": topic,
                        "payload_format": config.payload_format,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                route_to_false(node, connections, error_output)
            }
        }
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
) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
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
) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
    // 检查 on_error 策略
    if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
        let error_msg = output_data
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("节点执行失败 (AbortFlow)")
            .to_string();
        return Err(ErrorDetail::execution_error(error_msg));
    }

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
    use flow_engine_core::node_config::MqttPublishConfig;
    use flow_engine_core::traits::{
        MqttConnectOutput, MqttConnectionInfo, MqttMessageOutput, MqttPublishOutput,
        MqttSubscribeOutput,
    };
    use crate::mqtt_connection_manager::MqttConnectionManager;

    /// 创建测试节点
    fn create_test_node(
        uuid: &str,
        name: &str,
        connection_id: &str,
        topic: &str,
        payload: Option<&str>,
        payload_format: Option<&str>,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mqtt_publish".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttPublish(MqttPublishConfig {
                r#type: "mqtt_publish".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
                payload: payload.map(|s| s.to_string()),
                qos: Some(0),
                retain: Some(false),
                payload_format: payload_format.map(|s| s.to_string()),
                content_type: None,
                message_expiry_interval: None,
                response_topic: None,
                correlation_data: None,
                topic_alias: None,
                user_properties: None,
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
                .expect("System time error")
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

    /// 测试：发布到不存在的连接 → 走 false 端口
    #[tokio::test]
    async fn test_publish_nonexistent_connection() {
        let mqtt_manager = Arc::new(MqttConnectionManager::new());
        let executor = MqttPublishExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-1",
            "测试发布",
            "nonexistent-conn",
            "test/topic",
            Some("hello"),
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

    /// 测试：变量替换
    #[tokio::test]
    async fn test_publish_with_variable_replacement() {
        let mqtt_manager = Arc::new(MqttConnectionManager::new());
        let executor = MqttPublishExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-var",
            "变量替换测试",
            "{{conn_id}}",
            "sensor/{{device_id}}/data",
            Some("{{msg}}"),
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
            .set_variable("msg".to_string(), json!("hello"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]); // 连接不存在，走 false
        let data = output_data.unwrap();
        // 验证变量替换生效：topic 应包含替换后的值
        assert_eq!(data["topic"], "sensor/sensor-001/data");
    }

    /// 测试：错误的节点数据类型 → Err
    #[tokio::test]
    async fn test_publish_wrong_node_data_type() {
        let mqtt_manager = Arc::new(MqttConnectionManager::new());
        let executor = MqttPublishExecutor::new(mqtt_manager);

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
        assert!(result.unwrap_err().contains("MqttPublishConfig"));
    }

    // ===== Payload 编码测试 =====

    #[test]
    fn test_encode_payload_text() {
        let result = encode_payload("hello world", None).expect("text 编码应成功");
        assert_eq!(result, b"hello world");

        let result = encode_payload("hello", Some("text")).expect("text 编码应成功");
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_encode_payload_json() {
        let json_str = r#"{"key": "value"}"#;
        let result = encode_payload(json_str, Some("json")).expect("json 编码应成功");
        assert_eq!(result, json_str.as_bytes());

        // 非法 JSON 应失败
        let bad_json = "{invalid}";
        let result = encode_payload(bad_json, Some("json"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON 格式无效"));
    }

    #[test]
    fn test_encode_payload_hex() {
        let result = encode_payload("48656c6c6f", Some("hex")).expect("hex 编码应成功");
        assert_eq!(result, b"Hello");

        // 带 0x 前缀
        let result = encode_payload("0x48656c6c6f", Some("hex")).expect("hex 编码应成功");
        assert_eq!(result, b"Hello");

        // 带空格
        let result = encode_payload("48 65 6c 6c 6f", Some("hex")).expect("hex 编码应成功");
        assert_eq!(result, b"Hello");

        // 奇数长度应失败
        let result = encode_payload("ABC", Some("hex"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("偶数"));

        // 空字符串
        let result = encode_payload("", Some("hex")).expect("空 hex 应成功");
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_payload_base64() {
        let result = encode_payload("SGVsbG8=", Some("base64")).expect("base64 编码应成功");
        assert_eq!(result, b"Hello");

        // 非法 base64 应失败
        let result = encode_payload("!!!invalid!!!", Some("base64"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Base64 解码失败"));
    }

    #[test]
    fn test_encode_payload_unsupported_format() {
        let result = encode_payload("data", Some("xml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持的 payload_format"));
    }

    // ===== 成功场景（使用 Mock） =====

    /// 模拟 MQTT Provider（publish 成功）
    struct MockMqttProvider;

    #[async_trait::async_trait]
    impl flow_engine_core::traits::MqttProvider for MockMqttProvider {
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
            _config: &MqttPublishConfig,
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
            None
        }

        fn list_connections(&self) -> Vec<MqttConnectionInfo> {
            vec![]
        }
    }

    /// 测试：Mock 成功发布 → 走 true 端口
    #[tokio::test]
    async fn test_publish_success_with_mock() {
        let mqtt_manager = Arc::new(MockMqttProvider);
        let executor = MqttPublishExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-ok",
            "成功发布",
            "mock-conn",
            "test/topic",
            Some("hello mqtt"),
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
        assert_eq!(data["topic"], "test/topic");
        assert_eq!(data["qos"], 0);
        assert_eq!(data["retain"], false);
        assert_eq!(data["payload_size"], 10); // "hello mqtt" = 10 bytes
        assert_eq!(data["reason_code"], 0);
        assert!(data["timing"]["total_ms"].is_number());
    }

    /// 测试：hex payload Mock 成功发布
    #[tokio::test]
    async fn test_publish_hex_payload_with_mock() {
        let mqtt_manager = Arc::new(MockMqttProvider);
        let executor = MqttPublishExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-hex",
            "Hex 发布",
            "mock-conn",
            "test/hex",
            Some("0x48656c6c6f"),
            Some("hex"),
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
        assert_eq!(data["payload_size"], 5); // "Hello" = 5 bytes
    }

    /// 测试：非法 hex payload → 走 false 端口
    #[tokio::test]
    async fn test_publish_invalid_hex_goes_to_false() {
        let mqtt_manager = Arc::new(MockMqttProvider);
        let executor = MqttPublishExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-bad-hex",
            "非法 Hex",
            "mock-conn",
            "test/bad",
            Some("ZZZZ"),
            Some("hex"),
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
        assert!(data["error"].is_string());
    }

    /// 测试：输出 JSON 结构完整性
    #[tokio::test]
    async fn test_publish_output_json_structure() {
        let mqtt_manager = Arc::new(MockMqttProvider);
        let executor = MqttPublishExecutor::new(mqtt_manager);
        let node = create_test_node(
            "test-structure",
            "结构测试",
            "mock-conn",
            "sensor/data",
            Some(r#"{"temp": 42}"#),
            Some("json"),
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
        assert!(data.get("topic").is_some(), "应有 topic 字段");
        assert!(data.get("qos").is_some(), "应有 qos 字段");
        assert!(data.get("retain").is_some(), "应有 retain 字段");
        assert!(data.get("payload_size").is_some(), "应有 payload_size 字段");
        assert!(data.get("reason_code").is_some(), "应有 reason_code 字段");
        assert!(data.get("timing").is_some(), "应有 timing 字段");
        assert!(data.get("request").is_some(), "应有 request 字段");
        assert!(
            data["request"]["connection_id"].is_string(),
            "request 中应有 connection_id"
        );
        assert!(
            data["request"]["payload_format"].is_string(),
            "request 中应有 payload_format"
        );
    }
}
