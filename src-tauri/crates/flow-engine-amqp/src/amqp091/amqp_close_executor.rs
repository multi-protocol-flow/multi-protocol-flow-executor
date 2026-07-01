//! AMQP 关闭连接节点执行器
//!
//! 实现 AMQP 0-9-1 连接关闭功能。
//! 关闭始终走 "out" 端口（失败不阻塞流程）。
//! 共享模式下 close 为 no-op。

use std::collections::HashMap;
use flow_engine_core::error::ErrorDetail;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{AmqpCloseConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 关闭连接节点执行器
pub struct AmqpCloseExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpCloseExecutor {
    /// 创建新的 AMQP 关闭连接执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpCloseExecutor {
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
            NodeData::AmqpClose(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "节点数据类型不是 AmqpCloseConfig: {}",
                    node.uuid
                )))
            }
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
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. 日志：开始关闭
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: amqp_close, 连接ID: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP Close 节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建关闭配置（使用替换后的 connection_id）
        let close_config = AmqpCloseConfig {
            connection_id: connection_id.clone(),
            ..config
        };

        // 5. 关闭连接
        let close_result = self
            .amqp_manager
            .close(&connection_id, &close_config)
            .await;

        // AMQP Close 始终走 "out" 端口，不阻塞流程
        let next_node_ids: Vec<String> = {
            let filtered: Vec<String> = connections
                .iter()
                .filter(|conn| {
                    conn.source_node_uuid == node.uuid && conn.source_port_id == "out"
                })
                .map(|conn| conn.target_node_uuid.clone())
                .collect();

            if filtered.is_empty() {
                vec!["out".to_string()]
            } else {
                filtered
            }
        };

        let duration = start_time.elapsed();

        match close_result {
            Ok(()) => {
                log::info!(
                    "[AMQP] 连接关闭成功: connection_id={}",
                    connection_id
                );

                // 6. 构建输出数据
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                        "reply_code": close_config.reply_code,
                        "reply_text": close_config.reply_text,
                    },
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 日志：关闭完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("AMQP 连接关闭成功 ({}ms)", duration.as_millis()),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 连接关闭失败: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：关闭失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 连接关闭失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据（仍然走 out 端口）
                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "request": {
                        "connection_id": connection_id,
                        "reply_code": close_config.reply_code,
                        "reply_text": close_config.reply_text,
                    },
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amqp091::amqp_connection_manager::AmqpConnectionManager;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpClose(AmqpCloseConfig {
                connection_id: connection_id.to_string(),
                ..AmqpCloseConfig::default()
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

    /// 测试：关闭不存在的连接，仍然路由到 "out" 端口（失败不阻塞流程）
    #[tokio::test]
    async fn test_amqp_close_nonexistent_connection_routes_to_out() {
        let amqp_manager = Arc::new(AmqpConnectionManager::new());
        let executor = AmqpCloseExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试关闭", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口（关闭失败不阻塞）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "nonexistent-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    /// 测试：NodeData 类型不匹配时返回错误
    #[tokio::test]
    async fn test_amqp_close_wrong_node_data_type() {
        let amqp_manager = Arc::new(AmqpConnectionManager::new());
        let executor = AmqpCloseExecutor::new(amqp_manager);

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
        assert!(result.unwrap_err().contains("AmqpCloseConfig"));
    }

    /// 测试：变量替换 — connection_id 中的变量应被正确替换
    #[tokio::test]
    async fn test_amqp_close_with_variable_replacement() {
        let amqp_manager = Arc::new(AmqpConnectionManager::new());
        let executor = AmqpCloseExecutor::new(amqp_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("amqp-connection-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 out 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "amqp-connection-456");
        // 连接不存在所以 success=false
        assert_eq!(data["success"], false);
    }

    /// 测试：共享模式下关闭不存在的连接应成功（no-op）
    #[tokio::test]
    async fn test_amqp_close_shared_mode_noop() {
        let amqp_manager = Arc::new(AmqpConnectionManager::new_shared());
        let executor = AmqpCloseExecutor::new(amqp_manager);

        let node = create_test_node("test-shared", "共享模式关闭", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 共享模式下 close 是 no-op，应返回 Ok + success=true
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
    }

    /// 测试：始终路由到 "out" 端口
    #[tokio::test]
    async fn test_amqp_close_always_routes_to_out_port() {
        let amqp_manager = Arc::new(AmqpConnectionManager::new());
        let executor = AmqpCloseExecutor::new(amqp_manager);

        let node = create_test_node("test-out", "端口测试", "some-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        // 无连接时默认返回 ["out"]
        assert_eq!(next_nodes, vec!["out"]);
    }

    /// 测试：配置提取 — 从 NodeData::AmqpClose 正确提取
    #[tokio::test]
    async fn test_amqp_close_config_extraction() {
        let amqp_manager = Arc::new(AmqpConnectionManager::new());
        let executor = AmqpCloseExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-cfg".to_string(),
            node_type: "amqp_close".to_string(),
            name: "配置提取测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpClose(AmqpCloseConfig {
                connection_id: "my-amqp-conn".to_string(),
                reply_code: 200,
                reply_text: Some("normal shutdown".to_string()),
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
        assert_eq!(data["connection_id"], "my-amqp-conn");
        assert_eq!(data["request"]["connection_id"], "my-amqp-conn");
        assert_eq!(data["request"]["reply_code"], 200);
        assert_eq!(data["request"]["reply_text"], "normal shutdown");
    }

    /// 测试：连接端口路由到下游节点
    #[tokio::test]
    async fn test_amqp_close_with_connections_routing() {
        let amqp_manager = Arc::new(AmqpConnectionManager::new_shared());
        let executor = AmqpCloseExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", "some-conn");
        let mut context = create_context("test-flow");

        // 创建 out 端口连接到下一个节点
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-route".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "out".to_string(),
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
