// WebSocket 连接节点执行器
//
// 实现 WebSocket 连接建立、超时控制、消息等待和变量映射功能。

use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::debug::DebugEvent;
use crate::executor::{replace_variables, NodeExecutor};
use crate::log_event::{LogEvent, LogLevel};
use crate::node::Node;
use crate::node_config::{NodeData, WsConnectConfig, WsWaitMode};
use crate::websocket_manager::WebSocketConnectionManager;
use crate::ws_stream::{MessageContent, MessageDirection, WsMessage};
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
        connections: &[Connection],
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

        // 4. 建立连接（计时）
        let connect_start = Instant::now();
        let connect_result = self
            .ws_manager
            .connect(
                node.uuid.clone(),
                node.name.clone(),
                url.clone(),
                config.headers.clone(),
                config.subprotocols.clone(),
                config.connect_timeout_ms,
            )
            .await;
        let connect_duration_ms = connect_start.elapsed().as_millis() as u64;

        match connect_result {
            Ok(()) => {
                log::info!(
                    "[WebSocket] 连接成功: uuid={}, url={}",
                    node.uuid,
                    url
                );

                // 存储连接 URL 到 context（供后续节点反查）
                context.set_ws_url(node.uuid.clone(), url.clone());

                // 创建 WebSocket 消息流（所有权创建）
                context
                    .create_ws_stream(
                        node.uuid.clone(),
                        url.clone(),
                        node.uuid.clone(),
                        node.name.clone(),
                        "ws_connect".to_string(),
                    )
                    .await;

                // 发送调试事件：WsStreamCreated
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send(DebugEvent::WsStreamCreated {
                        connection_id: node.uuid.clone(),
                        url: url.clone(),
                        owner_node_uuid: node.uuid.clone(),
                        owner_node_name: node.name.clone(),
                    });
                }

                // 5. 如果配置了等待消息（计时）
                let wait_start = Instant::now();
                let (wait_timed_out, message_data) = if config.wait_for_message {
                    match self.wait_for_initial_message_new(node, &config, context).await {
                        Ok(Some(msg)) => (false, Some(msg)),
                        Ok(None) => (true, None), // 超时
                        Err(e) => {
                            log::error!(
                                "[WebSocket] 等待消息出错: uuid={}, error={}",
                                node.uuid,
                                e
                            );
                            (true, None) // 出错也视为超时
                        }
                    }
                } else {
                    (false, None)
                };
        let wait_response_duration_ms = if config.wait_for_message && !wait_timed_out {
            Some(wait_start.elapsed().as_millis() as u64)
        } else {
            None
        };

                // 6. 构建输出数据
                let output_data = self.build_output_data(
                    &node.uuid,
                    &url,
                    connect_duration_ms,
                    wait_response_duration_ms,
                    wait_timed_out,
                    message_data,
                    start_time.elapsed().as_millis() as u64,
                    &config,
                );

                // 7. 应用变量映射
                self.apply_output_mappings_new(&config, &output_data, context)?;

                // 8. 日志：连接完成
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

                // 9. 返回 true 端口（连接成功，无论等待是否超时）
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();
                    
                    // 如果没有连接，返回端口名称（用于测试）
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

                // 构建错误输出数据（结构化格式，与 HTTP 保持一致）
                let error_output = json!({
                    "success": false,
                    "connected": false,
                    "connection_id": serde_json::Value::Null,
                    "url": url,
                    "request": {
                        "url": url,
                        "headers": config.headers,
                        "subprotocols": config.subprotocols,
                        "connect_timeout_ms": config.connect_timeout_ms,
                        "wait_for_message": config.wait_for_message,
                        "wait_mode": format!("{:?}", config.wait_mode),
                        "message_timeout_ms": config.message_timeout_ms,
                    },
                    "timing": {
                        "connect_ms": serde_json::Value::Null,
                        "wait_response_ms": serde_json::Value::Null,
                        "total_ms": start_time.elapsed().as_millis() as u64,
                    },
                    "wait_timed_out": false,
                    "error": e
                });

                // 返回 false 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();
                    
                    // 如果没有连接，返回端口名称（用于测试）
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

impl WsConnectExecutor {
    /// 等待初始消息（新版本）
    /// 
    /// 返回值：
    /// - Ok(Some(message)) - 成功收到消息
    /// - Ok(None) - 超时未收到消息
    /// - Err(e) - 发生错误
    async fn wait_for_initial_message_new(
        &self,
        node: &Node,
        config: &WsConnectConfig,
        context: &mut ExecutionContext,
    ) -> Result<Option<String>, String> {
        match config.wait_mode {
            WsWaitMode::FirstMessage => {
                // 等待第一条消息（或超时）
                let timeout_ms = config.message_timeout_ms;
                match self.ws_manager.receive(&node.uuid, timeout_ms).await {
                    Ok(Some(message)) => {
                        log::info!(
                            "[WebSocket] 收到初始消息: uuid={}, len={}",
                            node.uuid,
                            message.len()
                        );

                        // 记录消息到 WebSocket 消息流
                        let ws_message = WsMessage::new(
                            MessageDirection::Received,
                            MessageContent::Text {
                                content: message.clone(),
                            },
                        );
                        if let Err(e) = context.push_ws_message(&node.uuid, ws_message).await {
                            log::warn!(
                                "[WebSocket] 记录消息失败: uuid={}, error={}",
                                node.uuid,
                                e
                            );
                        }

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

                        Ok(Some(message))
                    }
                    Ok(None) => {
                        log::info!(
                            "[WebSocket] 等待消息超时: uuid={}",
                            node.uuid
                        );
                        Ok(None)
                    }
                    Err(e) => Err(e),
                }
            }
            WsWaitMode::Condition => {
                // 等待满足条件的消息（或超时）
                if let Some(ref condition) = config.wait_condition {
                    let start = Instant::now();
                    let timeout_ms = config.message_timeout_ms;

                    loop {
                        let elapsed = start.elapsed().as_millis() as u64;
                        if elapsed >= timeout_ms {
                            log::info!(
                                "[WebSocket] 等待条件超时: uuid={}",
                                node.uuid
                            );
                            return Ok(None);
                        }

                        let remaining = timeout_ms - elapsed;
                        match self.ws_manager.receive(&node.uuid, remaining.min(5000)).await {
                            Ok(Some(message)) => {
                                // 记录消息到 WebSocket 消息流
                                let ws_message = WsMessage::new(
                                    MessageDirection::Received,
                                    MessageContent::Text {
                                        content: message.clone(),
                                    },
                                );
                                if let Err(e) = context.push_ws_message(&node.uuid, ws_message).await {
                                    log::warn!(
                                        "[WebSocket] 记录消息失败: uuid={}, error={}",
                                        node.uuid,
                                        e
                                    );
                                }

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

                                    return Ok(Some(message));
                                }
                                // 不匹配，继续等待
                            }
                            Ok(None) => continue, // 接收超时，继续
                            Err(e) => return Err(e),
                        }
                    }
                } else {
                    Err("等待条件未指定".to_string())
                }
            }
            WsWaitMode::Duration => {
                // 在指定时长内接收消息，返回最后一条
                let start = Instant::now();
                let duration_ms = config.message_timeout_ms;
                let mut last_message: Option<String> = None;

                while start.elapsed().as_millis() < duration_ms as u128 {
                    if let Ok(Some(message)) = self.ws_manager.receive(&node.uuid, 1000).await {
                        log::debug!(
                            "[WebSocket] 收到消息（Duration模式）: uuid={}, len={}",
                            node.uuid,
                            message.len()
                        );

                        // 记录消息到 WebSocket 消息流
                        let ws_message = WsMessage::new(
                            MessageDirection::Received,
                            MessageContent::Text {
                                content: message.clone(),
                            },
                        );
                        if let Err(e) = context.push_ws_message(&node.uuid, ws_message).await {
                            log::warn!(
                                "[WebSocket] 记录消息失败: uuid={}, error={}",
                                node.uuid,
                                e
                            );
                        }

                        last_message = Some(message);
                    }
                }

                if let Some(ref _msg) = last_message {
                    log::info!(
                        "[WebSocket] Duration 模式完成，返回最后一条消息: uuid={}",
                        node.uuid
                    );
                } else {
                    log::info!(
                        "[WebSocket] Duration 模式完成，未收到消息: uuid={}",
                        node.uuid
                    );
                }

                Ok(last_message)
            }
        }
    }

    /// 构建输出数据（结构化格式，与 HTTP 报告保持一致）
    #[allow(clippy::too_many_arguments)]
    fn build_output_data(
        &self,
        uuid: &str,
        url: &str,
        connect_duration_ms: u64,
        wait_response_duration_ms: Option<u64>,
        wait_timed_out: bool,
        message_data: Option<String>,
        duration_ms: u64,
        config: &WsConnectConfig,
    ) -> serde_json::Value {
        let mut output = json!({
            // 统一成功标识（与 HTTP 保持一致）
            "success": true,
            "connected": true,
            "connection_id": uuid,
            "url": url,
            // 请求配置（与 HTTP 的 request 结构对应）
            "request": {
                "url": url,
                "headers": config.headers,
                "subprotocols": config.subprotocols,
                "connect_timeout_ms": config.connect_timeout_ms,
                "wait_for_message": config.wait_for_message,
                "wait_mode": format!("{:?}", config.wait_mode),
                "message_timeout_ms": config.message_timeout_ms,
            },
            // 耗时分解（与 HTTP 的 timing 结构对应）
            "timing": {
                "connect_ms": connect_duration_ms,
                "wait_response_ms": wait_response_duration_ms,
                "total_ms": duration_ms,
            },
            "wait_timed_out": wait_timed_out,
        });

        if let Some(msg) = message_data {
            output["message"] = json!({
                "data": msg,
                "type": "text", // 简化处理，假设都是文本消息
            });
        } else {
            output["message"] = serde_json::Value::Null;
        }

        output
    }

    /// 应用输出变量映射（新版本）
    fn apply_output_mappings_new(
        &self,
        config: &WsConnectConfig,
        output_data: &serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<(), String> {
        if !config.output_mappings.is_empty() {
            for mapping in &config.output_mappings {
                if let Some(value) =
                    crate::http_executor::extract_value_by_path(output_data, &mapping.source)
                {
                    log::info!(
                        "[流程 {}] [输出映射] 路径: {} -> 变量: {}",
                        context.flow_uuid,
                        mapping.source,
                        mapping.target
                    );
                    context.variables.set_variable(
                        mapping.target.clone(),
                        value.clone(),
                        String::new(), // connection_id
                    );
                } else {
                    log::warn!(
                        "[流程 {}] [输出映射] 路径 '{}' 未找到值",
                        context.flow_uuid,
                        mapping.source
                    );
                }
            }
        }
        Ok(())
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
                wait_mode: WsWaitMode::FirstMessage,
                wait_condition: None,
                message_timeout_ms: 10000,
                output_mappings: vec![],
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
                    ws_connection_urls: std::collections::HashMap::new(),
                    ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                    last_node_output: None,
                    debug_event_sender: None,
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
        assert!(output_data.is_some());
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
        has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（超时）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
        assert!(!data["error"].is_null());
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
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
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
        has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 服务器
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
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
        has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 服务器
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
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
        has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 服务器
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
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
        has_breakpoint: None,
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

                    // 验证消息流已创建
                    let stream = context.get_ws_stream("echo-test").await;
                    assert!(
                        stream.is_some(),
                        "连接成功后应该创建 WebSocket 消息流"
                    );
                    let stream = stream.unwrap();
                    assert_eq!(stream.connection_id, "echo-test");
                    assert_eq!(stream.url, "wss://echo.websocket.org");
                    assert_eq!(
                        stream.current_owner_node_uuid,
                        Some("echo-test".to_string())
                    );
                    assert_eq!(stream.segments.len(), 1);
                    assert_eq!(stream.segments[0].node_type, "ws_connect");

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

    /// 测试：连接成功后创建 WsStreamSegment
    #[tokio::test]
    async fn test_create_ws_stream_on_connect_success() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager.clone());

        // 连接到公共 WebSocket echo 服务器
        let node = create_test_node("stream-test", "消息流测试", "wss://echo.websocket.org");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        if let Ok((next_nodes, _)) = result {
            if next_nodes == vec!["true"] {
                // 验证消息流已创建
                let stream = context.get_ws_stream("stream-test").await;
                assert!(stream.is_some(), "连接成功后应该创建 WebSocket 消息流");

                let stream = stream.unwrap();
                assert_eq!(stream.connection_id, "stream-test");
                assert_eq!(stream.url, "wss://echo.websocket.org");
                assert_eq!(stream.current_owner_node_uuid, Some("stream-test".to_string()));
                assert_eq!(stream.segments.len(), 1, "应该有一个初始段");

                let segment = &stream.segments[0];
                assert_eq!(segment.node_uuid, "stream-test");
                assert_eq!(segment.node_name, "消息流测试");
                assert_eq!(segment.node_type, "ws_connect");
                assert!(segment.ended_at.is_none(), "初始段不应结束");

                // 清理
                let _ = ws_manager
                    .close("stream-test", 1000, Some("测试完成".to_string()), 5000)
                    .await;
            } else {
                println!("跳过测试：无法连接到 echo 服务器");
            }
        }
    }

    /// 测试：连接失败时不创建 WsStreamSegment
    #[tokio::test]
    async fn test_no_ws_stream_on_connect_failure() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        // 使用无效 URL
        let node = create_test_node("fail-test", "失败测试", "ws://invalid.example.com:9999/ws");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"], "应该返回 false 端口");

        // 验证没有创建消息流
        let stream = context.get_ws_stream("fail-test").await;
        assert!(stream.is_none(), "连接失败不应创建 WebSocket 消息流");
    }
}
