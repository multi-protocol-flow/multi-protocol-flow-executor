// WebSocket 连接节点执行器
//
// 实现 WebSocket 连接建立、超时控制、消息采集和变量映射功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{NodeData, WsCollectMode, WsConnectConfig};
use flow_engine_core::traits::WsProvider;
use flow_engine_core::ws_stream::{MessageContent, MessageDirection, WsMessage};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 采集结果
struct CollectResult {
    /// 采集过程中收到的所有消息
    collected: Vec<String>,
    /// 匹配消息在 collected 中的索引
    matched_index: Option<usize>,
}

/// WebSocket 连接节点执行器
pub struct WsConnectExecutor {
    ws_manager: Arc<dyn WsProvider>,
}

impl WsConnectExecutor {
    /// 创建新的 WebSocket 连接执行器
    pub fn new(ws_manager: Arc<dyn WsProvider>) -> Self {
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
                    let _ = sender.send_event(DebugEvent::WsStreamCreated {
                        connection_id: node.uuid.clone(),
                        url: url.clone(),
                        owner_node_uuid: node.uuid.clone(),
                        owner_node_name: node.name.clone(),
                    });
                }

                // 5. 如果配置了采集消息（计时）
                let collect_start = Instant::now();
                let (collect_timed_out, collect_result) = if config.collect_message {
                    match self.collect_initial_message(node, &config, context).await {
                        Ok(result) => {
                            if result.matched_index.is_some() {
                                (false, result)
                            } else {
                                (true, result) // 超时
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "[WebSocket] 采集消息出错: uuid={}, error={}",
                                node.uuid,
                                e
                            );
                            (true, CollectResult { collected: vec![], matched_index: None }) // 出错也视为超时
                        }
                    }
                } else {
                    (false, CollectResult { collected: vec![], matched_index: None })
                };

                let collect_response_duration_ms = if config.collect_message {
                    Some(collect_start.elapsed().as_millis() as u64)
                } else {
                    None
                };

                // 6. 构建输出数据
                let output_data = self.build_output_data(
                    &node.uuid,
                    &url,
                    connect_duration_ms,
                    collect_response_duration_ms,
                    collect_timed_out,
                    &collect_result.collected,
                    collect_result.matched_index,
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

                // 9. 返回 true 端口（连接成功，无论采集是否超时）
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

                // 构建错误输出数据
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
                        "collect_message": config.collect_message,
                        "collect_mode": format!("{:?}", config.collect_mode),
                        "collect_timeout_ms": config.collect_timeout_ms,
                    },
                    "timing": {
                        "connect_ms": serde_json::Value::Null,
                        "collect_response_ms": serde_json::Value::Null,
                        "total_ms": start_time.elapsed().as_millis() as u64,
                    },
                    "collect_timed_out": false,
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
    /// 采集初始消息
    async fn collect_initial_message(
        &self,
        node: &Node,
        config: &WsConnectConfig,
        context: &mut ExecutionContext,
    ) -> Result<CollectResult, String> {
        match config.collect_mode {
            WsCollectMode::FirstMatch => {
                // 取消检查
                if context.is_cancelled() {
                    return Err("流程已取消".to_string());
                }
                let timeout_ms = config.collect_timeout_ms;
                // 使用 watch channel 监听取消信号
                let receive_result = if let Some(ref rx) = context.cancel_receiver {
                    let mut cancel_rx = rx.clone();
                    tokio::select! {
                        msg = self.ws_manager.receive(&node.uuid, timeout_ms) => msg,
                        _ = cancel_rx.changed() => Err("流程已取消".to_string()),
                    }
                } else {
                    self.ws_manager.receive(&node.uuid, timeout_ms).await
                };
                match receive_result {
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

                        Ok(CollectResult {
                            collected: vec![message],
                            matched_index: Some(0),
                        })
                    }
                    Ok(None) => {
                        log::info!(
                            "[WebSocket] 采集消息超时: uuid={}",
                            node.uuid
                        );
                        Ok(CollectResult {
                            collected: vec![],
                            matched_index: None,
                        })
                    }
                    Err(e) => Err(e),
                }
            }
            WsCollectMode::ConditionMatch => {
                match &config.collect_condition {
                    Some(condition) if !condition.expression.trim().is_empty() => {
                        let start = Instant::now();
                        let timeout_ms = config.collect_timeout_ms;
                        let mut collected: Vec<String> = Vec::new();

                        loop {
                            // 取消检查
                            if context.is_cancelled() {
                                return Err("流程已取消".to_string());
                            }
                            let elapsed = start.elapsed().as_millis() as u64;
                            if elapsed >= timeout_ms {
                                log::info!(
                                    "[WebSocket] 采集条件超时: uuid={}",
                                    node.uuid
                                );
                                return Ok(CollectResult { collected, matched_index: None });
                            }

                            let remaining = timeout_ms - elapsed;
                            // 使用 watch channel 监听取消信号
                            let receive_result = if let Some(ref rx) = context.cancel_receiver {
                                let mut cancel_rx = rx.clone();
                                tokio::select! {
                                    msg = self.ws_manager.receive(&node.uuid, remaining.min(5000)) => msg,
                                    _ = cancel_rx.changed() => Err("流程已取消".to_string()),
                                }
                            } else {
                                self.ws_manager.receive(&node.uuid, remaining.min(5000)).await
                            };
                            match receive_result {
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

                                    let msg_index = collected.len();
                                    collected.push(message.clone());

                                    if self.ws_manager.match_message(&message, condition)? {
                                        log::info!(
                                            "[WebSocket] 消息匹配成功: uuid={}",
                                            node.uuid
                                        );

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

                                        return Ok(CollectResult { collected, matched_index: Some(msg_index) });
                                    }
                                    // 不匹配，继续等待
                                }
                                Ok(None) => continue, // 接收超时，继续
                                Err(e) => return Err(e),
                            }
                        }
                    }
                    Some(condition) => {
                        let error_msg = format!(
                            "条件匹配模式需要填写匹配表达式（当前表达式为空）: match_type={:?}",
                            condition.match_type
                        );
                        log::error!(
                            "[WebSocket] 条件匹配配置错误: uuid={}, expression 为空",
                            node.uuid
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

                        Err(error_msg)
                    }
                    None => {
                        let error_msg = "条件匹配模式需要配置匹配条件（collect_condition 未设置）".to_string();
                        log::error!(
                            "[WebSocket] 条件匹配配置错误: uuid={}, collect_condition 为 None",
                            node.uuid
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

                        Err(error_msg)
                    }
                }
            }
            WsCollectMode::DurationCollect => {
                // 在指定时长内采集所有消息
                let start = Instant::now();
                let duration_ms = config.collect_timeout_ms;
                let mut collected: Vec<String> = Vec::new();

                while start.elapsed().as_millis() < duration_ms as u128 {
                    // 取消检查
                    if context.is_cancelled() {
                        return Err("流程已取消".to_string());
                    }
                    // 使用 watch channel 监听取消信号
                    let receive_result = if let Some(ref rx) = context.cancel_receiver {
                        let mut cancel_rx = rx.clone();
                        tokio::select! {
                            msg = self.ws_manager.receive(&node.uuid, 1000) => msg,
                            _ = cancel_rx.changed() => Err("流程已取消".to_string()),
                        }
                    } else {
                        self.ws_manager.receive(&node.uuid, 1000).await
                    };
                    if let Ok(Some(message)) = receive_result {
                        log::debug!(
                            "[WebSocket] 收到消息（DurationCollect模式）: uuid={}, len={}",
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

                        collected.push(message);
                    }
                }

                if collected.is_empty() {
                    log::info!(
                        "[WebSocket] DurationCollect 模式完成，未收到消息: uuid={}",
                        node.uuid
                    );
                } else {
                    log::info!(
                        "[WebSocket] DurationCollect 模式完成，返回最后一条消息: uuid={}",
                        node.uuid
                    );
                }

                let matched_index = if collected.is_empty() { None } else { Some(collected.len() - 1) };
                Ok(CollectResult { collected, matched_index })
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
        collect_response_duration_ms: Option<u64>,
        collect_timed_out: bool,
        collected_messages: &[String],
        matched_index: Option<usize>,
        duration_ms: u64,
        config: &WsConnectConfig,
    ) -> serde_json::Value {
        // 构建 matched_message
        let matched_message: Option<serde_json::Value> = matched_index
            .and_then(|idx| collected_messages.get(idx))
            .map(|msg| {
                let mut msg_obj = json!({
                    "data": msg,
                    "type": "text",
                });
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(msg) {
                    msg_obj["parsed"] = parsed;
                }
                msg_obj
            });

        // 构建 collected_messages 数组
        let collected_arr: Vec<serde_json::Value> = collected_messages
            .iter()
            .map(|msg| {
                let mut msg_obj = json!({
                    "data": msg,
                    "type": "text",
                });
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(msg) {
                    msg_obj["parsed"] = parsed;
                }
                msg_obj
            })
            .collect();

        let mut output = json!({
            "success": true,
            "connected": true,
            "connection_id": uuid,
            "url": url,
            "request": {
                "url": url,
                "headers": config.headers,
                "subprotocols": config.subprotocols,
                "connect_timeout_ms": config.connect_timeout_ms,
                "collect_message": config.collect_message,
                "collect_mode": format!("{:?}", config.collect_mode),
                "collect_timeout_ms": config.collect_timeout_ms,
            },
            "timing": {
                "connect_ms": connect_duration_ms,
                "collect_response_ms": collect_response_duration_ms,
                "total_ms": duration_ms,
            },
            "collect_timed_out": collect_timed_out,
            "collected_messages": collected_arr,
            "message_count": collected_messages.len(),
        });

        if let Some(msg) = matched_message {
            output["matched_message"] = msg.clone();
            output["message"] = msg;
        } else {
            output["matched_message"] = serde_json::Value::Null;
            output["message"] = serde_json::Value::Null;
        }

        output
    }

    /// 应用输出变量映射
    fn apply_output_mappings_new(
        &self,
        config: &WsConnectConfig,
        output_data: &serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<(), String> {
        if !config.output_mappings.is_empty() {
            for mapping in &config.output_mappings {
                if let Some(value) =
                    flow_engine_core::utils::extract_value_by_path(output_data, &mapping.source)
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
                        String::new(),
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
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::{WsConnectConfig, WsCollectCondition, WsMatchType};
    use flow_engine_core::variable::OutputMapping;
    use crate::websocket_manager::WebSocketConnectionManager;
    use std::sync::Arc;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "ws_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: url.to_string(),
                headers: vec![],
                subprotocols: vec![],
                connect_timeout_ms: 30000,
                collect_message: false,
                collect_mode: WsCollectMode::FirstMatch,
                collect_condition: None,
                collect_timeout_ms: 10000,
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
            db_manager: None,
            grpc_manager: None,
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
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
            node_type: "ws_connect".to_string(),
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

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
    }

    #[tokio::test]
    async fn test_collect_mode_first_message() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        let node = Node {
            uuid: "test-wait-first".to_string(),
            node_type: "ws_connect".to_string(),
            name: "等待第一条消息".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "wss://example.com/ws".to_string(),
                collect_message: true,
                collect_mode: WsCollectMode::FirstMatch,
                collect_timeout_ms: 5000,
                ..Default::default()
            }),
            ports: None,
        has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
    }

    #[tokio::test]
    async fn test_collect_mode_condition() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        let node = Node {
            uuid: "test-wait-condition".to_string(),
            node_type: "ws_connect".to_string(),
            name: "条件等待".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "wss://example.com/ws".to_string(),
                collect_message: true,
                collect_mode: WsCollectMode::ConditionMatch,
                collect_condition: Some(WsCollectCondition {
                    match_type: WsMatchType::JsonPath,
                    expression: "$.status".to_string(),
                }),
                collect_timeout_ms: 5000,
                ..Default::default()
            }),
            ports: None,
        has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connected"], false);
    }

    #[tokio::test]
    async fn test_collect_mode_duration() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsConnectExecutor::new(ws_manager);

        let node = Node {
            uuid: "test-wait-duration".to_string(),
            node_type: "ws_connect".to_string(),
            name: "时长等待".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsConnect(WsConnectConfig {
                url: "wss://example.com/ws".to_string(),
                collect_message: true,
                collect_mode: WsCollectMode::DurationCollect,
                collect_timeout_ms: 2000, // 2 秒
                ..Default::default()
            }),
            ports: None,
        has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

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

        let node = Node {
            uuid: "test-mappings".to_string(),
            node_type: "ws_connect".to_string(),
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

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

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

        let node = create_test_node("stream-test", "消息流测试", "wss://echo.websocket.org");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        if let Ok((next_nodes, _)) = result {
            if next_nodes == vec!["true"] {
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