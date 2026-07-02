// Kafka 批量消费节点执行器
//
// 从指定 Topic 批量消费消息，支持 3 种批量策略：
// - count_timeout: 达到数量上限或超时即停止
// - count_only: 仅按数量上限消费
// - timeout_only: 仅按时间窗口消费
// 输出端口: "true"（成功）/ "false"（失败）

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{KafkaConsumeBatchConfig, NodeData};
use flow_engine_core::traits::KafkaProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Kafka 批量消费节点执行器
pub struct KafkaConsumeBatchExecutor {
    kafka_manager: Arc<dyn KafkaProvider>,
}

impl KafkaConsumeBatchExecutor {
    /// 创建新的 Kafka 批量消费执行器
    pub fn new(kafka_manager: Arc<dyn KafkaProvider>) -> Self {
        Self { kafka_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for KafkaConsumeBatchExecutor {
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
            NodeData::KafkaConsumeBatch(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not KafkaConsumeBatchConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 检查取消信号
        if context.is_cancelled() {
            log::warn!(
                "[流程 {}] Kafka 批量消费节点 {} 被取消",
                context.flow_uuid,
                node.uuid
            );
            return Ok((vec!["false".to_string()], Some(json!({
                "success": false,
                "error": "Execution cancelled",
            }))));
        }

        // 3. 变量替换: connection_id、topic、group_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("Connection ID variable substitution failed: {}", e);
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

        let group_id = match &config.group_id {
            Some(gid) => Some(replace_variables(gid, context).map_err(|e| {
                let error_msg = format!("Group ID variable substitution failed: {}", e);
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

        // 4. 日志：开始批量消费
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: kafka_consume_batch, 连接ID: {}, topic: {}",
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
                    "Kafka 批量消费节点 {} 开始执行, topic: {}",
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

        // 5. 构建批量消费配置（使用替换后的字段）
        let resolved_config = KafkaConsumeBatchConfig {
            r#type: config.r#type,
            connection_id: connection_id.clone(),
            topic: topic.clone(),
            partition: config.partition,
            offset: config.offset,
            offset_reset: config.offset_reset.clone(),
            group_id,
            enable_auto_commit: config.enable_auto_commit,
            max_messages: config.max_messages,
            poll_timeout_ms: config.poll_timeout_ms,
            deserialization_format: config.deserialization_format.clone(),
            schema_subject: config.schema_subject.clone(),
            batch_strategy: config.batch_strategy.clone(),
        };

        // 6. 执行批量消费
        let result = self
            .kafka_manager
            .consume_batch(&connection_id, &resolved_config)
            .await;

        let duration = start_time.elapsed();

        match result {
            Ok(output) => {
                let success = output.success;
                let batch_size = output.batch_size.unwrap_or(0);

                if success {
                    log::info!(
                        "[Kafka] 批量消费成功: connection_id={}, topic={}, batch_size={}, 耗时={}ms",
                        connection_id,
                        topic,
                        batch_size,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[Kafka] 批量消费返回失败: connection_id={}, topic={}, error={:?}",
                        connection_id,
                        topic,
                        output.error
                    );
                }

                // 7. 构建输出数据
                let output_data = serde_json::to_value(&output).unwrap_or_else(|_| {
                    json!({
                        "success": false,
                        "error": "Failed to serialize batch consume output",
                    })
                });

                // 日志：批量消费完成
                let log_level = if success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: format!(
                            "Kafka 批量消费{} ({}条, {}ms)",
                            if success { "成功" } else { "失败" },
                            batch_size,
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

                // 8. 路由端口: success → "true", failure → "false"
                if !success
                    && !node.on_error.as_ref().map_or(true, |s| {
                        *s == ExecutionErrorStrategy::RouteToFalse
                    })
                {
                    let error_msg = output_data
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Kafka 批量消费失败")
                        .to_string();
                    return Err(ErrorDetail::execution_error(error_msg));
                }
                let port = if success { "true" } else { "false" };
                let next_node_ids = resolve_port_connections(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[Kafka] 批量消费异常: connection_id={}, topic={}, error={}",
                    connection_id,
                    topic,
                    e
                );

                // 日志：批量消费异常
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Kafka 批量消费异常: {}", e),
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
                    "messages": [],
                    "batch_size": 0,
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| {
                    *s == ExecutionErrorStrategy::RouteToFalse
                }) {
                    return Err(ErrorDetail::execution_error(e));
                }

                let next_node_ids = resolve_port_connections(connections, &node.uuid, "false");

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

/// 根据端口名称解析下游连接
fn resolve_port_connections(
    connections: &[Connection],
    node_uuid: &str,
    port: &str,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kafka_connection_manager::KafkaConnectionManager;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, topic: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "kafka_consume_batch".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::KafkaConsumeBatch(KafkaConsumeBatchConfig {
                r#type: "kafka_consume_batch".to_string(),
                connection_id: connection_id.to_string(),
                topic: topic.to_string(),
                partition: None,
                offset: None,
                offset_reset: Some("latest".to_string()),
                group_id: None,
                enable_auto_commit: Some(false),
                max_messages: Some(10),
                poll_timeout_ms: Some(1000),
                deserialization_format: None,
                schema_subject: None,
                batch_strategy: Some("count_timeout".to_string()),
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
                .expect("System time error")
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

    /// 测试：消费不存在的连接，路由到 "false" 端口
    #[tokio::test]
    async fn test_consume_batch_nonexistent_connection_routes_to_false() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaConsumeBatchExecutor::new(kafka_manager);
        let node = create_test_node("test-1", "测试批量消费", "nonexistent-conn", "test-topic");
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
    }

    /// 测试：NodeData 类型不匹配时返回错误
    #[tokio::test]
    async fn test_consume_batch_wrong_node_data_type() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaConsumeBatchExecutor::new(kafka_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
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
        assert!(result.unwrap_err().message.contains("KafkaConsumeBatchConfig"));
    }

    /// 测试：变量替换 — connection_id 和 topic 中的变量应被正确替换
    #[tokio::test]
    async fn test_consume_batch_with_variable_replacement() {
        let kafka_manager = Arc::new(KafkaConnectionManager::new());
        let executor = KafkaConsumeBatchExecutor::new(kafka_manager);

        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}", "{{topic_name}}");

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("kafka-connection-789"),
            String::new(),
        );
        context.variables.set_variable(
            "topic_name".to_string(),
            json!("my-topic"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // 连接不存在，走 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
    }
}
