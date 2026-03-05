// WebSocket Send+Wait 节点执行器
//
// 实现 WebSocket 消息发送、等待响应和变量映射功能。

use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::executor::{replace_variables, NodeExecutor};
use crate::log_event::{LogEvent, LogLevel};
use crate::node::Node;
use crate::node_config::{NodeData, WsMessageType, WsSendWaitConfig, WsWaitMode};
use crate::websocket_manager::WebSocketConnectionManager;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// WebSocket Send+Wait 节点执行器
pub struct WsSendWaitExecutor {
    ws_manager: Arc<WebSocketConnectionManager>,
}

impl WsSendWaitExecutor {
    /// 创建新的 WebSocket Send+Wait 执行器
    pub fn new(ws_manager: Arc<WebSocketConnectionManager>) -> Self {
        Self { ws_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for WsSendWaitExecutor {
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
            NodeData::WsSendWait(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 WsSendWaitConfig: {}", node.uuid)),
        };

        // 2. 变量替换 connection_id 和 message
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

        let message = replace_variables(&config.message, context).map_err(|e| {
            let error_msg = format!("消息变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 日志：开始发送
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: ws_send_wait, 连接ID: {}, 消息长度: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            message.len()
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("WebSocket Send+Wait 节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 检查连接是否存在且状态为 Connected
        let connection_status = self.ws_manager.get_status(&connection_id);
        match connection_status {
            Some(crate::websocket_manager::ConnectionStatus::Connected) => {
                // 连接正常，继续执行
            }
            Some(status) => {
                let error_msg = format!(
                    "连接状态不是 Connected（当前状态: {:?}）: {}",
                    status, connection_id
                );
                log::error!(
                    "[WebSocket] 连接状态错误: uuid={}, status={:?}",
                    connection_id,
                    status
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: error_msg.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                return Err(error_msg);
            }
            None => {
                let error_msg = format!("连接不存在: {}", connection_id);
                log::error!("[WebSocket] 连接不存在: uuid={}", connection_id);

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: error_msg.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                return Err(error_msg);
            }
        }

        // 5. 发送消息
        let is_binary = config.message_type == WsMessageType::Binary;
        let send_result = self
            .ws_manager
            .send(&connection_id, message.clone(), is_binary, config.send_timeout_ms)
            .await;

        match send_result {
            Ok(()) => {
                log::info!(
                    "[WebSocket] 消息发送成功: connection_id={}, len={}",
                    connection_id,
                    message.len()
                );

                // 6. 如果配置了等待响应
                let response_data = if config.wait_for_response {
                    match self.wait_for_response(&connection_id, &config, context, node).await {
                        Ok(response) => response,
                        Err(e) => {
                            log::error!(
                                "[WebSocket] 等待响应失败: connection_id={}, error={}",
                                connection_id,
                                e
                            );

                            context
                                .emit_log(LogEvent {
                                    level: LogLevel::Error,
                                    message: format!("等待响应失败: {}", e),
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .expect("系统时间错误")
                                        .as_millis() as u64,
                                    node_uuid: Some(node.uuid.clone()),
                                    node_name: Some(node.name.clone()),
                                    flow_uuid: context.flow_uuid.clone(),
                                })
                                .await;

                            // 等待失败返回错误
                            return Err(e);
                        }
                    }
                } else {
                    None
                };

                // 7. 变量映射
                let output_data = self.apply_output_mappings(
                    &config,
                    &connection_id,
                    &message,
                    response_data.as_deref(),
                    context,
                    start_time.elapsed().as_millis() as u64,
                )?;

                // 8. 日志：执行完成
                let duration = start_time.elapsed();
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("WebSocket Send+Wait 完成 ({}ms)", duration.as_millis()),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9. 返回下一个节点（默认 out 端口）
                // 根据 "out" 端口过滤连接，找到下一个节点的 UUID
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| {
                        conn.source_node_uuid == node.uuid && conn.source_port_id == "out"
                    })
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();

                Ok((next_node_ids, output_data))
            }
            Err(e) => {
                log::error!(
                    "[WebSocket] 消息发送失败: connection_id={}, error={}",
                    connection_id,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("消息发送失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                Err(e)
            }
        }
    }
}

impl WsSendWaitExecutor {
    /// 等待响应
    async fn wait_for_response(
        &self,
        connection_id: &str,
        config: &WsSendWaitConfig,
        context: &mut ExecutionContext,
        node: &Node,
    ) -> Result<Option<String>, String> {
        match config.wait_mode {
            WsWaitMode::None => Ok(None),
            WsWaitMode::FirstMessage => {
                let result = self
                    .ws_manager
                    .receive(connection_id, config.message_timeout_ms)
                    .await?;

                if let Some(message) = result {
                    log::info!(
                        "[WebSocket] 收到响应消息: connection_id={}, len={}",
                        connection_id,
                        message.len()
                    );

                    // 发送日志
                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Info,
                            message: format!("收到响应消息: {}", message),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("系统时间错误")
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;

                    Ok(Some(message))
                } else {
                    Err("等待响应超时".to_string())
                }
            }
            WsWaitMode::Condition => {
                if let Some(ref condition) = config.wait_condition {
                    let start = Instant::now();
                    let timeout_ms = config.message_timeout_ms;

                    loop {
                        let elapsed = start.elapsed().as_millis() as u64;
                        if elapsed >= timeout_ms {
                            return Err("等待条件超时".to_string());
                        }

                        let remaining = timeout_ms - elapsed;
                        let result = self
                            .ws_manager
                            .receive(connection_id, remaining.min(5000))
                            .await?;

                        if let Some(message) = result {
                            if self.ws_manager.match_message(&message, condition)? {
                                log::info!(
                                    "[WebSocket] 响应匹配成功: connection_id={}",
                                    connection_id
                                );

                                // 发送日志
                                context
                                    .emit_log(LogEvent {
                                        level: LogLevel::Info,
                                        message: format!("响应匹配成功: {}", message),
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .expect("系统时间错误")
                                            .as_millis() as u64,
                                        node_uuid: Some(node.uuid.clone()),
                                        node_name: Some(node.name.clone()),
                                        flow_uuid: context.flow_uuid.clone(),
                                    })
                                    .await;

                                return Ok(Some(message));
                            }
                        }
                    }
                } else {
                    Err("等待条件未指定".to_string())
                }
            }
            WsWaitMode::Duration => {
                // 在指定时间内接收所有消息，返回最后一条
                let start = Instant::now();
                let duration_ms = config.message_timeout_ms;
                let mut last_message: Option<String> = None;

                while start.elapsed().as_millis() < duration_ms as u128 {
                    let result = self.ws_manager.receive(connection_id, 1000).await?;
                    if let Some(message) = result {
                        log::debug!(
                            "[WebSocket] 收到消息（Duration模式）: connection_id={}, len={}",
                            connection_id,
                            message.len()
                        );
                        last_message = Some(message);
                    }
                }

                // 发送最后一条消息的日志
                if let Some(ref msg) = last_message {
                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Info,
                            message: format!("Duration 模式最后消息: {}", msg),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("系统时间错误")
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;
                }

                Ok(last_message)
            }
        }
    }

    /// 应用输出变量映射
    fn apply_output_mappings(
        &self,
        config: &WsSendWaitConfig,
        connection_id: &str,
        sent_message: &str,
        response_message: Option<&str>,
        context: &mut ExecutionContext,
        duration_ms: u64,
    ) -> Result<Option<serde_json::Value>, String> {
        // 构建基础输出数据
        let output_data = json!({
            "connection_id": connection_id,
            "sent_message": sent_message,
            "response_message": response_message,
            "duration_ms": duration_ms,
        });

        // 应用变量映射
        if !config.output_mappings.is_empty() {
            for mapping in &config.output_mappings {
                if let Some(value) =
                    crate::http_executor::extract_value_by_path(&output_data, &mapping.source)
                {
                    log::info!(
                        "[流程 {}] [输出映射] 节点: {}, 路径: {} -> 变量: {}",
                        context.flow_uuid,
                        connection_id,
                        mapping.source,
                        mapping.target
                    );
                    context.variables.set_variable(
                        mapping.target.clone(),
                        value.clone(),
                        connection_id.to_string(),
                    );
                } else {
                    log::warn!(
                        "[流程 {}] [输出映射] 节点: {}, 路径 '{}' 未找到值",
                        context.flow_uuid,
                        connection_id,
                        mapping.source
                    );
                }
            }
        }

        Ok(Some(output_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ExecutionContext;
    use crate::node::{NodePosition, NodeType};
    use crate::node_config::{WsMatchType, WsSendWaitConfig, WsWaitCondition, WsWaitMode};
    use crate::variable::OutputMapping;
    use crate::websocket_manager::WebSocketConnectionManager;
    use serde_json::json;
    use std::sync::Arc;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, message: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::WsSendWait,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendWait(WsSendWaitConfig {
                connection_id: connection_id.to_string(),
                message: message.to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                wait_for_response: false,
                wait_mode: WsWaitMode::None,
                wait_condition: None,
                message_timeout_ms: 10000,
                output_mappings: vec![],
            }),
            ports: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext::new(flow_id.to_string(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间错误")
            .as_millis() as u64)
    }

    #[tokio::test]
    async fn test_send_wait_connection_not_found() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager);
        let node = create_test_node("test-1", "测试发送", "nonexistent-conn", "Hello");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误（连接不存在）
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("连接不存在"));
    }

    #[tokio::test]
    async fn test_send_wait_with_variable_replacement() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "{{conn_id}}", "Message: {{msg}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("ws-connection-123"),
            String::new(),
        );
        context
            .variables
            .set_variable("msg".to_string(), json!("Hello"), String::new());

        // 注意：这个测试会失败，因为我们没有真实的 WebSocket 连接
        // 但我们可以验证变量替换逻辑不会崩溃
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误（连接不存在），但不会因为变量替换错误
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("连接不存在"));
    }

    #[tokio::test]
    async fn test_send_wait_wait_mode_first_message() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager);

        // 创建配置了等待第一条消息的节点
        let node = Node {
            uuid: "test-wait-first".to_string(),
            node_type: NodeType::WsSendWait,
            name: "等待第一条消息".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendWait(WsSendWaitConfig {
                connection_id: "test-conn".to_string(),
                message: "Hello".to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                wait_for_response: true,
                wait_mode: WsWaitMode::FirstMessage,
                wait_condition: None,
                message_timeout_ms: 5000,
                output_mappings: vec![],
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误（连接不存在）
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_wait_wait_mode_condition() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager);

        // 创建配置了条件等待的节点
        let node = Node {
            uuid: "test-wait-condition".to_string(),
            node_type: NodeType::WsSendWait,
            name: "条件等待".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendWait(WsSendWaitConfig {
                connection_id: "test-conn".to_string(),
                message: r#"{"type":"ping"}"#.to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                wait_for_response: true,
                wait_mode: WsWaitMode::Condition,
                wait_condition: Some(WsWaitCondition {
                    match_type: WsMatchType::JsonPath,
                    expression: "$.type".to_string(),
                }),
                message_timeout_ms: 5000,
                output_mappings: vec![],
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误（连接不存在）
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_wait_wait_mode_duration() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager);

        // 创建配置了时长等待的节点
        let node = Node {
            uuid: "test-wait-duration".to_string(),
            node_type: NodeType::WsSendWait,
            name: "时长等待".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendWait(WsSendWaitConfig {
                connection_id: "test-conn".to_string(),
                message: "Test".to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                wait_for_response: true,
                wait_mode: WsWaitMode::Duration,
                wait_condition: None,
                message_timeout_ms: 2000, // 2 秒
                output_mappings: vec![],
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误（连接不存在）
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_wait_output_mappings() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager);

        // 创建带输出映射的节点
        let node = Node {
            uuid: "test-mappings".to_string(),
            node_type: NodeType::WsSendWait,
            name: "输出映射测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendWait(WsSendWaitConfig {
                connection_id: "test-conn".to_string(),
                message: "Test".to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                wait_for_response: false,
                wait_mode: WsWaitMode::None,
                wait_condition: None,
                message_timeout_ms: 10000,
                output_mappings: vec![OutputMapping {
                    source: "$.connection_id".to_string(),
                    target: "ws_conn_id".to_string(),
                    description: None,
                }],
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误（连接不存在）
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_binary_message() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager);

        // 创建二进制消息节点
        let node = Node {
            uuid: "test-binary".to_string(),
            node_type: NodeType::WsSendWait,
            name: "二进制消息".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendWait(WsSendWaitConfig {
                connection_id: "test-conn".to_string(),
                message: "0102030405".to_string(), // 十六进制字符串
                message_type: WsMessageType::Binary,
                send_timeout_ms: 10000,
                wait_for_response: false,
                wait_mode: WsWaitMode::None,
                wait_condition: None,
                message_timeout_ms: 10000,
                output_mappings: vec![],
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误（连接不存在）
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_wait_to_echo_server() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager.clone());

        // 先建立连接
        let connect_result = ws_manager
            .connect(
                "echo-conn".to_string(),
                "Echo 连接".to_string(),
                "wss://echo.websocket.org".to_string(),
                30000,
            )
            .await;

        // 网络测试：如果连接成功，继续测试
        if connect_result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = ws_manager.receive("echo-conn", 3000).await;

            // 创建发送节点
            let node = create_test_node("echo-test", "Echo 测试", "echo-conn", "Hello, WebSocket!");
            let mut context = create_context("test-flow");

            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            // 验证发送成功
            match result {
                Ok((next_nodes, output_data)) => {
                    assert_eq!(next_nodes, vec!["out"]);
                    assert!(output_data.is_some());
                    let data = output_data.unwrap();
                    assert_eq!(data["connection_id"], "echo-conn");
                    assert_eq!(data["sent_message"], "Hello, WebSocket!");

                    // 清理：关闭连接
                    let _ = ws_manager
                        .close("echo-conn", 1000, Some("测试完成".to_string()), 5000)
                        .await;
                }
                Err(e) => {
                    // 不应该发生错误
                    panic!("执行器返回错误: {}", e);
                }
            }
        } else {
            println!("跳过测试：无法连接到 echo 服务器");
        }
    }

    #[tokio::test]
    async fn test_send_wait_with_response_to_echo_server() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager.clone());

        // 先建立连接
        let connect_result = ws_manager
            .connect(
                "echo-conn-2".to_string(),
                "Echo 连接 2".to_string(),
                "wss://echo.websocket.org".to_string(),
                30000,
            )
            .await;

        // 网络测试：如果连接成功，继续测试
        if connect_result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = ws_manager.receive("echo-conn-2", 3000).await;

            // 创建带等待响应的节点
            let node = Node {
                uuid: "echo-test-2".to_string(),
                node_type: NodeType::WsSendWait,
                name: "Echo 测试 2".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::WsSendWait(WsSendWaitConfig {
                    connection_id: "echo-conn-2".to_string(),
                    message: "Test Message".to_string(),
                    message_type: WsMessageType::Text,
                    send_timeout_ms: 10000,
                    wait_for_response: true,
                    wait_mode: WsWaitMode::FirstMessage,
                    wait_condition: None,
                    message_timeout_ms: 10000,
                    output_mappings: vec![],
                }),
                ports: None,
            };
            let mut context = create_context("test-flow");

            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            // 验证发送和接收成功
            match result {
                Ok((next_nodes, output_data)) => {
                    assert_eq!(next_nodes, vec!["out"]);
                    assert!(output_data.is_some());
                    let data = output_data.unwrap();
                    assert_eq!(data["connection_id"], "echo-conn-2");
                    assert_eq!(data["sent_message"], "Test Message");
                    assert!(data["response_message"].is_string());
                    assert_eq!(
                        data["response_message"].as_str().unwrap(),
                        "Test Message"
                    );

                    // 清理：关闭连接
                    let _ = ws_manager
                        .close("echo-conn-2", 1000, Some("测试完成".to_string()), 5000)
                        .await;
                }
                Err(e) => {
                    // 不应该发生错误
                    panic!("执行器返回错误: {}", e);
                }
            }
        } else {
            println!("跳过测试：无法连接到 echo 服务器");
        }
    }

    #[tokio::test]
    async fn test_send_wait_timeout() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendWaitExecutor::new(ws_manager.clone());

        // 先建立连接
        let connect_result = ws_manager
            .connect(
                "timeout-conn".to_string(),
                "超时连接".to_string(),
                "wss://echo.websocket.org".to_string(),
                30000,
            )
            .await;

        // 网络测试：如果连接成功，继续测试
        if connect_result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = ws_manager.receive("timeout-conn", 3000).await;

            // 创建带等待响应的节点，但设置较短超时
            let node = Node {
                uuid: "timeout-test".to_string(),
                node_type: NodeType::WsSendWait,
                name: "超时测试".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::WsSendWait(WsSendWaitConfig {
                    connection_id: "timeout-conn".to_string(),
                    message: "Test".to_string(),
                    message_type: WsMessageType::Text,
                    send_timeout_ms: 10000,
                    wait_for_response: true,
                    wait_mode: WsWaitMode::FirstMessage,
                    wait_condition: None,
                    message_timeout_ms: 1000, // 1 秒超时
                    output_mappings: vec![],
                }),
                ports: None,
            };
            let mut context = create_context("test-flow");

            // 注意：这个测试可能会超时，因为 echo 服务器可能不会立即响应
            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            // 可能成功（收到响应）或失败（超时）
            if let Err(e) = result {
                assert!(e.contains("超时") || e.contains("timeout"));
            }

            // 清理：关闭连接
            let _ = ws_manager
                .close("timeout-conn", 1000, Some("测试完成".to_string()), 5000)
                .await;
        } else {
            println!("跳过测试：无法连接到 echo 服务器");
        }
    }
}
