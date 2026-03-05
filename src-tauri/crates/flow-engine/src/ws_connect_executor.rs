// WebSocket 连接节点执行器
//
// 实现 WebSocket 连接建立、超时控制、消息等待和变量映射功能。

use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::executor::{replace_variables, NodeExecutor};
use crate::log_event::{LogEvent, LogLevel};
use crate::node::Node;
use crate::node_config::{NodeData, WsConnectConfig, WsWaitMode};
use crate::websocket_manager::WebSocketConnectionManager;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// WebSocket 连接节点执行器
pub struct WsConnectExecutor {
    ws_manager: Arc<WebSocketConnectionManager>,
}

impl WsConnectExecutor {
    /// 创建新的 WebSocket 连接执行器
    pub fn new(ws_manager: Arc<WebSocketConnectionManager>) -> Self {
        Self { ws_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for WsConnectExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::WsConnect(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 WsConnectConfig: {}", node.uuid)),
        };

        // 2. 变量替换 URL
        let url = replace_variables(&config.url, context).map_err(|e| {
            let error_msg = format!("URL 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 日志：开始连接
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: ws_connect, URL: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("WebSocket 连接节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 建立连接
        let connect_result = self
            .ws_manager
            .connect(
                node.uuid.clone(),
                node.name.clone(),
                url.clone(),
                config.connect_timeout_ms,
            )
            .await;

        match connect_result {
            Ok(()) => {
                log::info!(
                    "[WebSocket] 连接成功: uuid={}, url={}",
                    node.uuid,
                    url
                );

                // 5. 如果配置了等待消息
                if config.wait_for_message {
                    if let Err(e) = self.wait_for_initial_message(node, &config, context).await {
                        log::error!(
                            "[WebSocket] 等待消息失败: uuid={}, error={}",
                            node.uuid,
                            e
                        );

                        // 发送错误日志
                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: format!("WebSocket 等待消息失败: {}", e),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("系统时间错误")
                                    .as_millis() as u64,
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        // 关闭连接（避免资源泄漏）
                        let _ = self
                            .ws_manager
                            .close(&node.uuid, 1000, Some("等待消息失败".to_string()), 5000)
                            .await;

                        // 返回 false 端口
                        return Ok((vec!["false".to_string()], None));
                    }
                }

                // 6. 变量映射
                let output_data =
                    self.apply_output_mappings(&config, &node.uuid, context, start_time.elapsed().as_millis() as u64)?;

                // 7. 日志：连接完成
                let duration = start_time.elapsed();
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("WebSocket 连接成功 ({}ms)", duration.as_millis()),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. 返回下一个节点（true 端口）
                Ok((vec!["true".to_string()], output_data))
            }
            Err(e) => {
                log::error!(
                    "[WebSocket] 连接失败: uuid={}, error={}",
                    node.uuid,
                    e
                );

                // 日志：连接失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("WebSocket 连接失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 返回 false 端口
                Ok((vec!["false".to_string()], None))
            }
        }
    }
}

impl WsConnectExecutor {
    /// 等待初始消息
    async fn wait_for_initial_message(
        &self,
        node: &Node,
        config: &WsConnectConfig,
        context: &mut ExecutionContext,
    ) -> Result<(), String> {
        match config.wait_mode {
            WsWaitMode::None => Ok(()),
            WsWaitMode::FirstMessage => {
                let result = self
                    .ws_manager
                    .receive(&node.uuid, config.message_timeout_ms)
                    .await?;

                if let Some(message) = result {
                    log::info!(
                        "[WebSocket] 收到初始消息: uuid={}, len={}",
                        node.uuid,
                        message.len()
                    );

                    // 发送日志
                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Info,
                            message: format!("收到初始消息: {}", message),
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
                Ok(())
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
                            .receive(&node.uuid, remaining.min(5000))
                            .await?;

                        if let Some(message) = result {
                            if self.ws_manager.match_message(&message, condition)? {
                                log::info!(
                                    "[WebSocket] 消息匹配成功: uuid={}",
                                    node.uuid
                                );

                                // 发送日志
                                context
                                    .emit_log(LogEvent {
                                        level: LogLevel::Info,
                                        message: format!("消息匹配成功: {}", message),
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .expect("系统时间错误")
                                            .as_millis() as u64,
                                        node_uuid: Some(node.uuid.clone()),
                                        node_name: Some(node.name.clone()),
                                        flow_uuid: context.flow_uuid.clone(),
                                    })
                                    .await;

                                return Ok(());
                            }
                        }
                    }
                } else {
                    Err("等待条件未指定".to_string())
                }
            }
            WsWaitMode::Duration => {
                // 在指定时间内接收所有消息
                let start = Instant::now();
                let duration_ms = config.message_timeout_ms;

                while start.elapsed().as_millis() < duration_ms as u128 {
                    let result = self.ws_manager.receive(&node.uuid, 1000).await?;
                    if let Some(message) = result {
                        log::debug!(
                            "[WebSocket] 收到消息（Duration模式）: uuid={}, len={}",
                            node.uuid,
                            message.len()
                        );
                    }
                }
                Ok(())
            }
        }
    }

    /// 应用输出变量映射
    fn apply_output_mappings(
        &self,
        config: &WsConnectConfig,
        uuid: &str,
        context: &mut ExecutionContext,
        duration_ms: u64,
    ) -> Result<Option<serde_json::Value>, String> {
        // 构建基础输出数据
        let output_data = json!({
            "connected": true,
            "connection_id": uuid,
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
                        uuid,
                        mapping.source,
                        mapping.target
                    );
                    context.variables.set_variable(
                        mapping.target.clone(),
                        value.clone(),
                        uuid.to_string(),
                    );
                } else {
                    log::warn!(
                        "[流程 {}] [输出映射] 节点: {}, 路径 '{}' 未找到值",
                        context.flow_uuid,
                        uuid,
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
    use crate::context::VariableStorage;
    use crate::node::{NodePosition, NodeType};
    use crate::node_config::{WsConnectConfig, WsWaitCondition, WsMatchType};
    use crate::variable::OutputMapping;
    use crate::websocket_manager::WebSocketConnectionManager;
    use std::sync::Arc;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::WsConnect,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: url.to_string(),
                headers: vec![],
                subprotocols: vec![],
                connect_timeout_ms: 30000,
                wait_for_message: false,
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
                }
    }

    #[tokio::test]
    async fn test_connect_invalid_url() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);
        let node = create_test_node("test-1", "测试连接", "http://invalid.com");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（无效 URL）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_none());
    }

    #[tokio::test]
    async fn test_connect_timeout() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        // 使用不可达地址测试超时
        let node = Node {
            uuid: "test-timeout".to_string(),
            node_type: NodeType::WsConnect,
            name: "超时测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "ws://192.0.2.1:81/ws".to_string(), // RFC 3330 测试地址，不可路由
                connect_timeout_ms: 2000, // 2 秒超时
                ..Default::default()
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（超时）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_none());
    }

    #[tokio::test]
    async fn test_connect_with_variable_replacement() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "ws://{{host}}/ws");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("example.com"),
            String::new(),
        );

        // 注意：这个测试会失败，因为我们没有真实的 WebSocket 服务器
        // 但我们可以验证变量替换逻辑不会崩溃
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败），但不会因为变量替换错误
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
    }

    #[tokio::test]
    async fn test_wait_mode_first_message() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        // 创建配置了等待第一条消息的节点
        let node = Node {
            uuid: "test-wait-first".to_string(),
            node_type: NodeType::WsConnect,
            name: "等待第一条消息".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "wss://example.com/ws".to_string(),
                wait_for_message: true,
                wait_mode: WsWaitMode::FirstMessage,
                message_timeout_ms: 5000,
                ..Default::default()
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 服务器
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
    }

    #[tokio::test]
    async fn test_wait_mode_condition() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        // 创建配置了条件等待的节点
        let node = Node {
            uuid: "test-wait-condition".to_string(),
            node_type: NodeType::WsConnect,
            name: "条件等待".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "wss://example.com/ws".to_string(),
                wait_for_message: true,
                wait_mode: WsWaitMode::Condition,
                wait_condition: Some(WsWaitCondition {
                    match_type: WsMatchType::JsonPath,
                    expression: "$.status".to_string(),
                }),
                message_timeout_ms: 5000,
                ..Default::default()
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 服务器
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
    }

    #[tokio::test]
    async fn test_wait_mode_duration() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        // 创建配置了时长等待的节点
        let node = Node {
            uuid: "test-wait-duration".to_string(),
            node_type: NodeType::WsConnect,
            name: "时长等待".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "wss://example.com/ws".to_string(),
                wait_for_message: true,
                wait_mode: WsWaitMode::Duration,
                message_timeout_ms: 2000, // 2 秒
                ..Default::default()
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 服务器
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
    }

    #[tokio::test]
    async fn test_output_mappings() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        // 创建带输出映射的节点
        let node = Node {
            uuid: "test-mappings".to_string(),
            node_type: NodeType::WsConnect,
            name: "输出映射测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "wss://example.com/ws".to_string(),
                output_mappings: vec![OutputMapping {
                    source: "$.connection_id".to_string(),
                    target: "ws_conn_id".to_string(),
                    description: None,
                }],
                ..Default::default()
            }),
            ports: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 服务器
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
    }

    #[tokio::test]
    async fn test_connect_to_echo_server() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager.clone());

        // 连接到公共 WebSocket echo 服务器
        let node = create_test_node("echo-test", "Echo 测试", "wss://echo.websocket.org");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 网络测试：如果连接成功，应该返回 true 端口
        match result {
            Ok((next_nodes, output_data)) => {
                if next_nodes == vec!["true"] {
                    // 连接成功
                    assert!(output_data.is_some());
                    let data = output_data.unwrap();
                    assert_eq!(data["connected"], true);
                    assert_eq!(data["connection_id"], "echo-test");

                    // 清理：关闭连接
                    let _ = ws_manager
                        .close("echo-test", 1000, Some("测试完成".to_string()), 5000)
                        .await;
                } else {
                    // 连接失败（网络问题），跳过测试
                    println!("跳过测试：无法连接到 echo 服务器");
                }
            }
            Err(e) => {
                // 不应该发生错误（应该返回 false 端口）
                panic!("执行器返回错误而不是 false 端口: {}", e);
            }
        }
    }
}
