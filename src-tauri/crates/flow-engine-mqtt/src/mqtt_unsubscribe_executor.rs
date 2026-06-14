// MQTT 取消订阅节点执行器
//
// 实现 MQTT 主题取消订阅功能。
// 成功路由到 "true" 端口，失败路由到 "false" 端口。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::MqttProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// MQTT 取消订阅节点执行器
pub struct MqttUnsubscribeExecutor {
    mqtt_manager: Arc<dyn MqttProvider>,
}

impl MqttUnsubscribeExecutor {
    /// 创建新的 MQTT 取消订阅执行器
    pub fn new(mqtt_manager: Arc<dyn MqttProvider>) -> Self {
        Self { mqtt_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for MqttUnsubscribeExecutor {
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
            NodeData::MqttUnsubscribe(config) => config.clone(),
            _ => {
                return Err(format!(
                    "Node data type is not MqttUnsubscribeConfig: {}",
                    node.uuid
                ))
            }
        };

        // 2. 变量替换 connection_id 和 topic
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

        let topic = replace_variables(&config.topic, context).map_err(|e| {
            let error_msg = format!("Topic variable substitution failed: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 日志：开始取消订阅
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: mqtt_unsubscribe, 连接ID: {}, 主题: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            topic
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("MQTT Unsubscribe 节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("System time error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 取消订阅
        let unsubscribe_result = self
            .mqtt_manager
            .unsubscribe(&connection_id, &node.uuid, &topic)
            .await;

        let duration = start_time.elapsed();

        match unsubscribe_result {
            Ok(()) => {
                log::info!(
                    "[MQTT] 取消订阅成功: connection_id={}, topic={}",
                    connection_id,
                    topic
                );

                // 5. 构建输出数据
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "topic": topic,
                    "request": {
                        "connection_id": connection_id,
                        "topic": topic,
                    },
                    "timing": {
                        "unsubscribe_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 日志：取消订阅完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "MQTT 取消订阅成功 ({}ms)",
                            duration.as_millis()
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

                // 成功时走 true 端口
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
                    "[MQTT] 取消订阅失败: connection_id={}, topic={}, error={}",
                    connection_id,
                    topic,
                    e
                );

                // 日志：取消订阅失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MQTT unsubscribe failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("System time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据
                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "topic": topic,
                    "request": {
                        "connection_id": connection_id,
                        "topic": topic,
                    },
                    "timing": {
                        "unsubscribe_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
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
    use crate::mqtt_connection_manager::MqttConnectionManager;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::MqttUnsubscribeConfig;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, topic: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mqtt_unsubscribe".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttUnsubscribe(MqttUnsubscribeConfig {
                r#type: "mqtt_unsubscribe".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
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
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None,
            tcp_manager: None,
            udp_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 测试：取消订阅不存在的连接，路由到 "false" 端口
    #[tokio::test]
    async fn test_unsubscribe_nonexistent_connection_routes_to_false() {
        let mqtt_manager = Arc::new(MqttConnectionManager::new());
        let executor = MqttUnsubscribeExecutor::new(mqtt_manager);
        let node = create_test_node("test-1", "测试取消订阅", "nonexistent-conn", "test/topic");
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
        assert_eq!(data["topic"], "test/topic");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    /// 测试：NodeData 类型不匹配时返回错误
    #[tokio::test]
    async fn test_unsubscribe_wrong_node_data_type() {
        let mqtt_manager = Arc::new(MqttConnectionManager::new());
        let executor = MqttUnsubscribeExecutor::new(mqtt_manager);

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
        assert!(result.unwrap_err().contains("MqttUnsubscribeConfig"));
    }

    /// 测试：变量替换 — connection_id 和 topic 中的变量应被正确替换
    #[tokio::test]
    async fn test_unsubscribe_with_variable_replacement() {
        let mqtt_manager = Arc::new(MqttConnectionManager::new());
        let executor = MqttUnsubscribeExecutor::new(mqtt_manager);

        // 创建带变量的节点
        let node = create_test_node(
            "test-var",
            "变量替换测试",
            "{{conn_id}}",
            "{{topic_name}}",
        );

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("mqtt-conn-789"),
            String::new(),
        );
        context.variables.set_variable(
            "topic_name".to_string(),
            json!("sensor/temperature"),
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
        // 验证变量替换生效
        assert_eq!(data["connection_id"], "mqtt-conn-789");
        assert_eq!(data["topic"], "sensor/temperature");
        assert_eq!(data["success"], false);
    }

    /// 测试：输出 JSON 结构完整性检查
    #[tokio::test]
    async fn test_unsubscribe_output_json_structure() {
        let mqtt_manager = Arc::new(MqttConnectionManager::new());
        let executor = MqttUnsubscribeExecutor::new(mqtt_manager);
        let node = create_test_node("test-json", "JSON结构测试", "conn-1", "test/topic");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        // 验证输出 JSON 包含所有必要字段
        assert!(data.get("success").is_some(), "缺少 success 字段");
        assert!(data.get("connection_id").is_some(), "缺少 connection_id 字段");
        assert!(data.get("topic").is_some(), "缺少 topic 字段");
        assert!(data.get("request").is_some(), "缺少 request 字段");
        assert!(data.get("timing").is_some(), "缺少 timing 字段");

        // 验证 request 子对象
        let request = &data["request"];
        assert_eq!(request["connection_id"], "conn-1");
        assert_eq!(request["topic"], "test/topic");

        // 验证 timing 子对象
        let timing = &data["timing"];
        assert!(timing.get("unsubscribe_ms").is_some());
        assert!(timing.get("total_ms").is_some());
    }
}
