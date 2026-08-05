// WebSocket Send+Collect 节点执行器
//
// 实现 WebSocket 消息发送、采集响应和变量映射功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{NodeData, WsCollectMode, WsMessageType, WsSendCollectConfig};
use flow_engine_core::traits::WsProvider;
use flow_engine_core::ws_stream::{MessageContent, MessageDirection, WsMessage};
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Instant;

/// 采集结果
struct CollectResult {
    /// 采集过程中收到的所有消息
    collected: Vec<String>,
    /// 匹配消息在 collected 中的索引
    matched_index: Option<usize>,
}

/// WebSocket Send+Collect 节点执行器
pub struct WsSendCollectExecutor {
    ws_manager: Arc<dyn WsProvider>,
}

impl WsSendCollectExecutor {
    /// 创建新的 WebSocket Send+Collect 执行器
    pub fn new(ws_manager: Arc<dyn WsProvider>) -> Self {
        Self { ws_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for WsSendCollectExecutor {
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
            NodeData::WsSendCollect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not WsSendCollectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换 connection_id 和 message
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("Connection ID variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let message = replace_variables(&config.message, context).map_err(|e| {
            let error_msg = format!("Message variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. Log: start sending
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: ws_send_collect, ConnectionID: {}, MessageLength: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            message.len()
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("WebSocket Send+Collect node {} started", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 检查连接是否存在且状态为 Connected
        let connection_status = self.ws_manager.get_status(&connection_id);
        let error_data = match connection_status {
            Some(status) if status == "Connected" => {
                // 连接正常，继续执行
                None
            }
            Some(status) => {
                let error_msg = format!(
                    "Connection status is not Connected (current: {}): {}",
                    status, connection_id
                );

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "Flow aborted: {}",
                        error_msg
                    )));
                }

                log::error!(
                    "[WebSocket] Connection status error: uuid={}, status={}",
                    connection_id,
                    status
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: error_msg.clone(),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 返回 false 端口 + Error数据
                let url = context.get_ws_url(&connection_id).map(|s| s.as_str());
                Some(self.build_error_output_data(&connection_id, &error_msg, url))
            }
            None => {
                let error_msg = format!("Connection not found: {}", connection_id);

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "Flow aborted: {}",
                        error_msg
                    )));
                }

                log::error!("[WebSocket] Connection not found: uuid={}", connection_id);

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: error_msg.clone(),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 返回 false 端口 + Error数据
                let url = context.get_ws_url(&connection_id).map(|s| s.as_str());
                Some(self.build_error_output_data(&connection_id, &error_msg, url))
            }
        };

        // 如果有Error数据，直接返回 false 端口
        if let Some(output_data) = error_data {
            // 返回 false 端口
            let filtered: Vec<String> = connections
                .iter()
                .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "false")
                .map(|conn| conn.target_node_uuid.clone())
                .collect();

            let next_node_ids = if filtered.is_empty() {
                vec!["false".to_string()]
            } else {
                filtered
            };

            return Ok((next_node_ids, output_data));
        }

        // 4.5 检查并转移 WebSocket 连接所有权
        {
            let current_owner = context
                .get_ws_stream(&connection_id)
                .await
                .and_then(|s| s.current_owner_node_uuid);

            if current_owner.as_ref() != Some(&node.uuid) {
                log::info!(
                    "[WebSocket] Transferring connection ownership: connection_id={}, from={:?}, to={}",
                    connection_id,
                    current_owner,
                    node.uuid
                );

                context
                    .transfer_ws_ownership(
                        &connection_id,
                        node.uuid.clone(),
                        node.name.clone(),
                        "ws_send_collect".to_string(),
                    )
                    .await;
            }
        }

        // 5. Drain pending messages before sending (must be before send to avoid swallowing fast server responses)
        match self.ws_manager.drain_pending_messages(&connection_id).await {
            Ok(drained_count) => {
                if drained_count > 0 {
                    log::info!(
                        "[WebSocket] Discarded {} pre-send pending message(s): connection_id={}",
                        drained_count,
                        connection_id
                    );
                }
            }
            Err(e) => {
                log::warn!(
                    "[WebSocket] Failed to drain pending messages (continuing): connection_id={}, error={}",
                    connection_id,
                    e
                );
            }
        }

        // 6. 发送消息
        let is_binary = config.message_type == WsMessageType::Binary;
        let send_start = Instant::now();
        let send_result = self
            .ws_manager
            .send(
                &connection_id,
                message.clone(),
                is_binary,
                config.send_timeout_ms,
            )
            .await;
        let send_duration_ms = send_start.elapsed().as_millis() as u64;

        // 处理发送结果
        let (port, output_data) = match send_result {
            Ok(()) => {
                log::info!(
                    "[WebSocket] Message sent successfully: connection_id={}, len={}",
                    connection_id,
                    message.len()
                );

                // Record sent message to ws_streams
                {
                    let sent_content = if is_binary {
                        MessageContent::Binary {
                            content: message.as_bytes().to_vec(),
                        }
                    } else {
                        MessageContent::Text {
                            content: message.clone(),
                        }
                    };
                    let sent_msg = WsMessage::new(MessageDirection::Sent, sent_content.clone());
                    if let Err(e) = context
                        .push_ws_message(&connection_id, sent_msg.clone())
                        .await
                    {
                        log::warn!(
                            "[WebSocket] Failed to record sent message (continuing): connection_id={}, error={}",
                            connection_id,
                            e
                        );
                    }

                    // Send debug event: WsMessageSent
                    if let Some(sender) = &context.debug_event_sender {
                        let _ = sender.send_event(DebugEvent::WsMessageSent {
                            connection_id: connection_id.clone(),
                            owner_node_uuid: node.uuid.clone(),
                            message: sent_msg,
                        });
                    }
                }

                // 7. 如果配置了采集响应
                let collect_start = Instant::now();
                let (collect_timed_out, collect_result) = if config.collect_response {
                    match self
                        .collect_response(&connection_id, &config, context, node)
                        .await
                    {
                        Ok(result) => {
                            if result.matched_index.is_some() {
                                log::info!(
                                    "[WebSocket] Response collection succeeded: connection_id={}, message_count={}",
                                    connection_id,
                                    result.collected.len()
                                );
                                (false, result)
                            } else {
                                log::warn!(
                                    "[WebSocket] Response collection timeout: connection_id={}",
                                    connection_id
                                );
                                (true, result)
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "[WebSocket] Response collection failed: connection_id={}, error={}",
                                connection_id,
                                e.message
                            );

                            context
                                .emit_log(LogEvent {
                                    level: LogLevel::Error,
                                    message: format!("Collect response failed: {}", e.message),
                                    timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                                    node_uuid: Some(node.uuid.clone()),
                                    node_name: Some(node.name.clone()),
                                    flow_uuid: context.flow_uuid.clone(),
                                })
                                .await;

                            // Collect failure is also treated as timeout, but still returns true port
                            (
                                true,
                                CollectResult {
                                    collected: vec![],
                                    matched_index: None,
                                },
                            )
                        }
                    }
                } else {
                    (
                        false,
                        CollectResult {
                            collected: vec![],
                            matched_index: None,
                        },
                    )
                };

                // 计算采集响应耗时
                let collect_response_duration_ms = if config.collect_response {
                    Some(collect_start.elapsed().as_millis() as u64)
                } else {
                    None
                };

                // 反查 URL
                let url = context.get_ws_url(&connection_id).map(|s| s.as_str());

                // 7. 构建输出数据
                let output_data = self.build_output_data(
                    &connection_id,
                    &message,
                    is_binary,
                    &collect_result.collected,
                    collect_result.matched_index,
                    collect_timed_out,
                    url,
                    send_duration_ms,
                    collect_response_duration_ms,
                )?;

                // 8. 日志：execution complete
                let duration = start_time.elapsed();
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "WebSocket Send+Collect completed ({}ms)",
                            duration.as_millis()
                        ),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                ("true", output_data)
            }
            Err(e) => {
                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                log::error!(
                    "[WebSocket] Message send failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Message send failed: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 返回 false 端口 + Error数据
                let url = context.get_ws_url(&connection_id).map(|s| s.as_str());
                (
                    "false",
                    self.build_error_output_data(&connection_id, &e, url),
                )
            }
        };

        // 9. 返回下一个节点（根据端口过滤）
        let filtered: Vec<String> = connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == port)
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        // 如果没有连接，返回端口名称（用于测试）
        let next_node_ids = if filtered.is_empty() {
            vec![port.to_string()]
        } else {
            filtered
        };

        Ok((next_node_ids, output_data))
    }
}

impl WsSendCollectExecutor {
    /// 采集响应
    async fn collect_response(
        &self,
        connection_id: &str,
        config: &WsSendCollectConfig,
        context: &mut ExecutionContext,
        node: &Node,
    ) -> Result<CollectResult, ErrorDetail> {
        match config.collect_mode {
            WsCollectMode::FirstMatch => {
                // 取消检查
                if context.is_cancelled() {
                    return Err(ErrorDetail::execution_error("Flow cancelled".to_string()));
                }

                // 使用 tokio::select! 监听取消信号
                let result = if let Some(mut rx) = context.cancel_receiver.clone() {
                    tokio::select! {
                        res = self.ws_manager.receive(connection_id, config.collect_timeout_ms) => res.map_err(ErrorDetail::execution_error)?,
                        _ = rx.changed() => {
                            return Err(ErrorDetail::execution_error("Flow cancelled".to_string()));
                        }
                    }
                } else {
                    self.ws_manager
                        .receive(connection_id, config.collect_timeout_ms)
                        .await
                        .map_err(ErrorDetail::execution_error)?
                };

                if let Some(message) = result {
                    log::info!(
                        "[WebSocket] Response message received: connection_id={}, len={}",
                        connection_id,
                        message.len()
                    );

                    // Record received message to ws_streams
                    {
                        let received_msg = WsMessage::new(
                            MessageDirection::Received,
                            MessageContent::Text {
                                content: message.clone(),
                            },
                        );
                        if let Err(e) = context
                            .push_ws_message(connection_id, received_msg.clone())
                            .await
                        {
                            log::warn!(
                                "[WebSocket] Failed to record received message (continuing): connection_id={}, error={}",
                                connection_id,
                                e
                            );
                        }

                        // Send debug event: WsMessageReceived
                        if let Some(sender) = &context.debug_event_sender {
                            let _ = sender.send_event(DebugEvent::WsMessageReceived {
                                connection_id: connection_id.to_string(),
                                owner_node_uuid: node.uuid.clone(),
                                message: received_msg,
                            });
                        }
                    }

                    // 发送日志
                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Info,
                            message: format!("Response message received: {}", message),
                            timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;

                    Ok(CollectResult {
                        collected: vec![message],
                        matched_index: Some(0),
                    })
                } else {
                    Ok(CollectResult {
                        collected: vec![],
                        matched_index: None,
                    })
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
                                return Err(ErrorDetail::execution_error(
                                    "Flow cancelled".to_string(),
                                ));
                            }

                            let elapsed = start.elapsed().as_millis() as u64;
                            if elapsed >= timeout_ms {
                                return Ok(CollectResult {
                                    collected,
                                    matched_index: None,
                                });
                            }

                            let remaining = timeout_ms - elapsed;
                            // 使用 tokio::select! 监听取消信号
                            let result = if let Some(mut rx) = context.cancel_receiver.clone() {
                                tokio::select! {
                                    res = self.ws_manager.receive(connection_id, remaining.min(5000)) => res.map_err(ErrorDetail::execution_error)?,
                                    _ = rx.changed() => {
                                        return Err(ErrorDetail::execution_error("Flow cancelled".to_string()));
                                    }
                                }
                            } else {
                                self.ws_manager
                                    .receive(connection_id, remaining.min(5000))
                                    .await
                                    .map_err(ErrorDetail::execution_error)?
                            };

                            if let Some(message) = result {
                                // 记录所有收到的消息
                                {
                                    let received_msg = WsMessage::new(
                                        MessageDirection::Received,
                                        MessageContent::Text {
                                            content: message.clone(),
                                        },
                                    );
                                    if let Err(e) = context
                                        .push_ws_message(connection_id, received_msg.clone())
                                        .await
                                    {
                                        log::warn!(
                                            "[WebSocket] Failed to record received message (continuing): connection_id={}, error={}",
                                            connection_id,
                                            e
                                        );
                                    }

                                    if let Some(sender) = &context.debug_event_sender {
                                        let _ = sender.send_event(DebugEvent::WsMessageReceived {
                                            connection_id: connection_id.to_string(),
                                            owner_node_uuid: node.uuid.clone(),
                                            message: received_msg,
                                        });
                                    }
                                }

                                let msg_index = collected.len();
                                collected.push(message.clone());

                                // 条件匹配：对 match_message Error做容错处理
                                // 表达式语法Error等不匹配问题不应中断整个采集过程
                                match self.ws_manager.match_message(&message, condition) {
                                    Ok(true) => {
                                        log::info!(
                                            "[WebSocket] Response matched: connection_id={}",
                                            connection_id
                                        );

                                        context
                                            .emit_log(LogEvent {
                                                level: LogLevel::Info,
                                                message: format!("Response matched: {}", message),
                                                timestamp:
                                                    flow_engine_core::utils::unix_timestamp_ms(),
                                                node_uuid: Some(node.uuid.clone()),
                                                node_name: Some(node.name.clone()),
                                                flow_uuid: context.flow_uuid.clone(),
                                            })
                                            .await;

                                        return Ok(CollectResult {
                                            collected,
                                            matched_index: Some(msg_index),
                                        });
                                    }
                                    Ok(false) => {
                                        // Message doesn't match condition, continue waiting
                                        log::debug!(
                                            "[WebSocket] Message does not match condition, continuing to wait: connection_id={}, message={}",
                                            connection_id,
                                            &message[..message.len().min(200)]
                                        );
                                    }
                                    Err(e) => {
                                        // Match error (e.g. expression syntax error), log but continue waiting
                                        log::warn!(
                                            "[WebSocket] Condition match error (continuing to wait): connection_id={}, error={}",
                                            connection_id,
                                            e
                                        );
                                        context
                                            .emit_log(LogEvent {
                                                level: LogLevel::Warn,
                                                message: format!("Condition match error (continuing to wait): {}", e),
                                                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                                                node_uuid: Some(node.uuid.clone()),
                                                node_name: Some(node.name.clone()),
                                                flow_uuid: context.flow_uuid.clone(),
                                            })
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                    Some(condition) => {
                        let error_msg = format!(
                        "Condition match mode requires a match expression (current expression is empty): match_type={:?}",
                        condition.match_type
                        );
                        log::error!(
                            "[WebSocket] Condition match config error: connection_id={}, expression is empty",
                            connection_id
                        );

                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: error_msg.clone(),
                                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        Err(ErrorDetail::execution_error(error_msg))
                    }
                    None => {
                        let error_msg =
                            "Condition match mode requires collect_condition to be configured"
                                .to_string();
                        log::error!(
                            "[WebSocket] Condition match config error: connection_id={}, collect_condition is None",
                            connection_id
                        );

                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: error_msg.clone(),
                                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        Err(ErrorDetail::execution_error(error_msg))
                    }
                }
            }
            WsCollectMode::DurationCollect => {
                // 在指定时间内接收所有消息
                let start = Instant::now();
                let duration_ms = config.collect_timeout_ms;
                let mut collected: Vec<String> = Vec::new();

                while start.elapsed().as_millis() < duration_ms as u128 {
                    // 取消检查
                    if context.is_cancelled() {
                        return Err(ErrorDetail::execution_error("Flow cancelled".to_string()));
                    }

                    // 使用 tokio::select! 监听取消信号
                    let result = if let Some(mut rx) = context.cancel_receiver.clone() {
                        tokio::select! {
                            res = self.ws_manager.receive(connection_id, 1000) => res.map_err(ErrorDetail::execution_error)?,
                            _ = rx.changed() => {
                                return Err(ErrorDetail::execution_error("Flow cancelled".to_string()));
                            }
                        }
                    } else {
                        self.ws_manager
                            .receive(connection_id, 1000)
                            .await
                            .map_err(ErrorDetail::execution_error)?
                    };
                    if let Some(message) = result {
                        log::debug!(
                            "[WebSocket] Message received (DurationCollect mode): connection_id={}, len={}",
                            connection_id,
                            message.len()
                        );

                        // Record received message to ws_streams
                        {
                            let received_msg = WsMessage::new(
                                MessageDirection::Received,
                                MessageContent::Text {
                                    content: message.clone(),
                                },
                            );
                            if let Some(sender) = &context.debug_event_sender {
                                let _ = sender.send_event(DebugEvent::WsMessageReceived {
                                    connection_id: connection_id.to_string(),
                                    owner_node_uuid: node.uuid.clone(),
                                    message: received_msg.clone(),
                                });
                            }
                            if let Err(e) =
                                context.push_ws_message(connection_id, received_msg).await
                            {
                                log::warn!(
                                    "[WebSocket] Failed to record received message (continuing): connection_id={}, error={}",
                                    connection_id,
                                    e
                                );
                            }
                        }

                        collected.push(message);
                    }
                }

                let matched_index = if collected.is_empty() {
                    None
                } else {
                    Some(collected.len() - 1)
                };

                // 发送最后一条消息的日志
                if let Some(idx) = matched_index {
                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Info,
                            message: format!("DurationCollect last message: {}", collected[idx]),
                            timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;
                }

                Ok(CollectResult {
                    collected,
                    matched_index,
                })
            }
        }
    }

    /// 构建输出数据（结构化格式，与 HTTP 报告保持一致）
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::result_large_err)]
    fn build_output_data(
        &self,
        connection_id: &str,
        sent_message: &str,
        is_binary: bool,
        collected_messages: &[String],
        matched_index: Option<usize>,
        collect_timed_out: bool,
        url: Option<&str>,
        send_duration_ms: u64,
        collect_response_duration_ms: Option<u64>,
    ) -> Result<Option<serde_json::Value>, ErrorDetail> {
        // 构建发送的消息数据
        let sent_data = json!({
            "data": sent_message,
            "type": if is_binary { "binary" } else { "text" },
        });

        // 构建 matched_message
        let matched_message: Option<serde_json::Value> = matched_index
            .and_then(|idx| collected_messages.get(idx))
            .map(|msg| {
                let mut msg_obj = json!({
                    "data": msg,
                    "type": if is_binary { "binary" } else { "text" },
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

        // 构建完整的输出数据（结构化格式）
        let mut output_data = json!({
            "success": true,
            "connection_id": connection_id,
            "url": url,
            "request": {
                "connection_id": connection_id,
                "message_type": if is_binary { "binary" } else { "text" },
            },
            "timing": {
                "send_ms": send_duration_ms,
                "collect_response_ms": collect_response_duration_ms,
                "total_ms": send_duration_ms + collect_response_duration_ms.unwrap_or(0),
            },
            "collect_timed_out": collect_timed_out,
            "sent": sent_data,
            "collected_messages": collected_arr,
            "message_count": collected_messages.len(),
        });

        if let Some(msg) = matched_message {
            output_data["matched_message"] = msg.clone();
            output_data["message"] = msg;
        } else {
            output_data["matched_message"] = serde_json::Value::Null;
            output_data["message"] = serde_json::Value::Null;
        }

        Ok(Some(output_data))
    }

    /// 构建Error输出数据（结构化格式，与 HTTP 报告保持一致）
    fn build_error_output_data(
        &self,
        connection_id: &str,
        error_msg: &str,
        url: Option<&str>,
    ) -> Option<serde_json::Value> {
        Some(json!({
            "success": false,
            "connection_id": connection_id,
            "url": url,
            "request": {
                "connection_id": connection_id,
            },
            "timing": {
                "send_ms": null,
                "collect_response_ms": null,
                "total_ms": null,
            },
            "collect_timed_out": false,
            "sent": null,
            "message": null,
            "error": error_msg,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket_manager::WebSocketConnectionManager;
    use flow_engine_core::context::ExecutionContext;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::{
        WsCollectCondition, WsCollectMode, WsMatchType, WsSendCollectConfig,
    };
    use serde_json::json;
    use std::sync::Arc;

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_id: &str, message: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "ws_send_collect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendCollect(WsSendCollectConfig {
                connection_id: connection_id.to_string(),
                message: message.to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                collect_response: false,
                collect_mode: WsCollectMode::FirstMatch,
                collect_condition: None,
                collect_timeout_ms: 10000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext::new(
            flow_id.to_string(),
            flow_engine_core::utils::unix_timestamp_ms(),
        )
    }

    #[tokio::test]
    async fn test_send_wait_connection_not_found() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);
        let node = create_test_node("test-1", "test send", "nonexistent-conn", "Hello");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，但包含Error数据（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false".to_string()]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert!(data["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection not found"));
    }
    #[tokio::test]
    async fn test_send_wait_with_variable_replacement() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 创建带变量的节点
        let node = create_test_node(
            "test-var",
            "variable substitution test",
            "{{conn_id}}",
            "Message: {{msg}}",
        );

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

        // 注意：这个测试会返回Error数据，因为我们没有真实的 WebSocket 连接
        // 但我们可以验证变量替换逻辑不会崩溃
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，但包含Error数据（连接不存在），变量替换正常工作
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false".to_string()]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert!(data["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection not found"));
    }
    #[tokio::test]
    async fn test_send_wait_collect_mode_first_message() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 创建配置了等待第一条消息的节点
        let node = Node {
            uuid: "test-wait-first".to_string(),
            node_type: "ws_send_collect".to_string(),
            name: "Wait for first message".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendCollect(WsSendCollectConfig {
                connection_id: "test-conn".to_string(),
                message: "Hello".to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                collect_response: true,
                collect_mode: WsCollectMode::FirstMatch,
                collect_condition: None,
                collect_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会返回Error数据，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，但包含Error数据（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false".to_string()]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert!(data["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection not found"));
    }
    #[tokio::test]
    async fn test_send_wait_collect_mode_condition() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 创建配置了条件等待的节点
        let node = Node {
            uuid: "test-wait-condition".to_string(),
            node_type: "ws_send_collect".to_string(),
            name: "Condition wait".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendCollect(WsSendCollectConfig {
                connection_id: "test-conn".to_string(),
                message: r#"{\"type\":\"ping\"}"#.to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                collect_response: true,
                collect_mode: WsCollectMode::ConditionMatch,
                collect_condition: Some(WsCollectCondition {
                    match_type: WsMatchType::JsonPath,
                    expression: "$.type".to_string(),
                }),
                collect_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会返回Error数据，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，但包含Error数据（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false".to_string()]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert!(data["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection not found"));
    }
    #[tokio::test]
    async fn test_send_wait_collect_mode_duration() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 创建配置了时长等待的节点
        let node = Node {
            uuid: "test-wait-duration".to_string(),
            node_type: "ws_send_collect".to_string(),
            name: "Duration wait".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendCollect(WsSendCollectConfig {
                connection_id: "test-conn".to_string(),
                message: "Test".to_string(),
                message_type: WsMessageType::Text,
                send_timeout_ms: 10000,
                collect_response: true,
                collect_mode: WsCollectMode::DurationCollect,
                collect_condition: None,
                collect_timeout_ms: 2000, // 2 秒
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会返回Error数据，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，但包含Error数据（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false".to_string()]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert!(data["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection not found"));
    }
    #[tokio::test]
    async fn test_send_binary_message() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 创建二进制消息节点
        let node = Node {
            uuid: "test-binary".to_string(),
            node_type: "ws_send_collect".to_string(),
            name: "Binary message".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsSendCollect(WsSendCollectConfig {
                connection_id: "test-conn".to_string(),
                message: "0102030405".to_string(), // 十六进制字符串
                message_type: WsMessageType::Binary,
                send_timeout_ms: 10000,
                collect_response: false,
                collect_mode: WsCollectMode::FirstMatch,
                collect_condition: None,
                collect_timeout_ms: 10000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会返回Error数据，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，但包含Error数据（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["false".to_string()]);
        assert!(output_data.is_some());
        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert!(data["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection not found"));
    }
    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_send_wait_to_echo_server() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager.clone());

        // 先建立连接
        let connect_result = ws_manager
            .connect(
                "echo-conn".to_string(),
                "Echo Connection".to_string(),
                "ws://localhost:19091".to_string(),
                vec![],
                vec![],
                30000,
                false, // use_tls
                false, // tls_skip_verify
                None,  // ping_interval_ms
                None,  // ping_timeout_ms
            )
            .await;

        // 网络测试：如果连接Success，继续测试
        if connect_result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = ws_manager.receive("echo-conn", 3000).await;

            // 创建发送节点
            let node = create_test_node("echo-test", "Echo Test", "echo-conn", "Hello, WebSocket!");
            let mut context = create_context("test-flow");

            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            // 验证发送Success
            match result {
                Ok((next_nodes, output_data)) => {
                    // 没有连接时返回端口名称
                    assert_eq!(next_nodes, vec!["true".to_string()]);
                    assert!(output_data.is_some());
                    let data = output_data.unwrap_or_else(|| panic!("expected Some"));
                    assert_eq!(data["connection_id"], "echo-conn");
                    assert_eq!(data["sent"]["data"], "Hello, WebSocket!");
                    assert_eq!(data["sent"]["type"], "text");

                    // 清理：关闭连接
                    let _ = ws_manager
                        .close("echo-conn", 1000, Some("test complete".to_string()), 5000)
                        .await;
                }
                Err(e) => {
                    // 不应该发生Error
                    panic!("Executor returned error: {}", e);
                }
            }
        } else {
            println!("Skipping test: unable to connect to echo server");
        }
    }

    #[tokio::test]
    #[ignore = "requires echo Docker service (cd echo && docker compose up -d)"]
    async fn test_send_wait_with_response_to_echo_server() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager.clone());

        // 先建立连接
        let connect_result = ws_manager
            .connect(
                "echo-conn-2".to_string(),
                "Echo Connection 2".to_string(),
                "ws://localhost:19091".to_string(),
                vec![],
                vec![],
                30000,
                false, // use_tls
                false, // tls_skip_verify
                None,  // ping_interval_ms
                None,  // ping_timeout_ms
            )
            .await;

        // 网络测试：如果连接Success，继续测试
        if connect_result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = ws_manager.receive("echo-conn-2", 3000).await;

            // 创建带等待响应的节点
            let node = Node {
                uuid: "echo-test-2".to_string(),
                node_type: "ws_send_collect".to_string(),
                name: "Echo Test 2".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::WsSendCollect(WsSendCollectConfig {
                    connection_id: "echo-conn-2".to_string(),
                    message: "Test Message".to_string(),
                    message_type: WsMessageType::Text,
                    send_timeout_ms: 10000,
                    collect_response: true,
                    collect_mode: WsCollectMode::FirstMatch,
                    collect_condition: None,
                    collect_timeout_ms: 10000,
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            };
            let mut context = create_context("test-flow");

            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            // 验证发送和接收Success
            match result {
                Ok((next_nodes, output_data)) => {
                    // 没有连接时返回端口名称
                    assert_eq!(next_nodes, vec!["true".to_string()]);
                    assert!(output_data.is_some());
                    let data = output_data.unwrap_or_else(|| panic!("expected Some"));
                    assert_eq!(data["connection_id"], "echo-conn-2");
                    assert_eq!(data["sent"]["data"], "Test Message");
                    assert!(data["message"]["data"].is_string());

                    // 清理：关闭连接
                    let _ = ws_manager
                        .close("echo-conn-2", 1000, Some("test complete".to_string()), 5000)
                        .await;
                }
                Err(e) => {
                    panic!("Executor returned error: {}", e);
                }
            }
        } else {
            println!("Skipping test: unable to connect to echo server");
        }
    }

    // ========================================================================
    // collected_messages 输出数据结构测试
    // ========================================================================

    /// 测试：FirstMatch 模式返回 1 条消息在 `collected_messages` 中
    #[test]
    fn test_collected_messages_first_match() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        let collected = vec!["hello world".to_string()];
        let result = executor
            .build_output_data(
                "conn-1",
                "ping",
                false,
                &collected,
                Some(0), // FirstMatch 匹配索引为 0
                false,
                Some("wss://example.com"),
                10,
                Some(5),
            )
            .unwrap_or_else(|e| panic!("Failed to build output data: {e}"))
            .unwrap_or_else(|| panic!("Should return Some"));

        // 验证 collected_messages 数组
        let collected_arr = result["collected_messages"]
            .as_array()
            .unwrap_or_else(|| panic!("Should be an array"));
        assert_eq!(collected_arr.len(), 1, "FirstMatch 应返回 1 条消息");
        assert_eq!(collected_arr[0]["data"], "hello world");
        assert_eq!(collected_arr[0]["type"], "text");

        // 验证 matched_message
        assert_eq!(result["matched_message"]["data"], "hello world");
        assert_eq!(result["message"]["data"], "hello world");

        // 验证 message_count
        assert_eq!(result["message_count"], 1);

        // 验证 success
        assert_eq!(result["success"], true);
    }

    /// 测试：ConditionMatch 模式返回所有消息（包含不匹配的）
    #[test]
    fn test_collected_messages_condition_match() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 模拟 ConditionMatch 场景：收到 3 条消息，第 3 条匹配
        let collected = vec![
            r#"{"type":"info","msg":"connecting"}"#.to_string(),
            r#"{"type":"info","msg":"connected"}"#.to_string(),
            r#"{"type":"ready","status":"ok"}"#.to_string(),
        ];
        let result = executor
            .build_output_data(
                "conn-2",
                "ping",
                false,
                &collected,
                Some(2), // 第 3 条消息匹配
                false,
                Some("wss://example.com"),
                10,
                Some(50),
            )
            .unwrap_or_else(|e| panic!("Failed to build output data: {e}"))
            .unwrap_or_else(|| panic!("Should return Some"));

        // 验证 collected_messages 包含所有消息
        let collected_arr = result["collected_messages"]
            .as_array()
            .unwrap_or_else(|| panic!("Should be an array"));
        assert_eq!(collected_arr.len(), 3, "ConditionMatch 应返回所有消息");
        assert_eq!(
            collected_arr[0]["data"],
            r#"{"type":"info","msg":"connecting"}"#
        );
        assert_eq!(
            collected_arr[1]["data"],
            r#"{"type":"info","msg":"connected"}"#
        );
        assert_eq!(
            collected_arr[2]["data"],
            r#"{"type":"ready","status":"ok"}"#
        );

        // 验证 parsed 字段（JSON 消息应自动解析）
        assert!(collected_arr[0]["parsed"].is_object());
        assert!(collected_arr[1]["parsed"].is_object());
        assert!(collected_arr[2]["parsed"].is_object());

        // 验证 matched_message 是最后一条
        assert_eq!(
            result["matched_message"]["data"],
            r#"{"type":"ready","status":"ok"}"#
        );
        assert_eq!(
            result["message"]["data"],
            r#"{"type":"ready","status":"ok"}"#
        );

        // 验证 message_count
        assert_eq!(result["message_count"], 3);
    }

    /// 测试：DurationCollect 模式返回所有消息
    #[test]
    fn test_collected_messages_duration_collect() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 模拟 DurationCollect 场景：收到 4 条消息，最后一条作为 matched
        let collected = vec![
            "msg1".to_string(),
            "msg2".to_string(),
            "msg3".to_string(),
            "msg4".to_string(),
        ];
        let result = executor
            .build_output_data(
                "conn-3",
                "start",
                false,
                &collected,
                Some(3), // 最后一条作为 matched
                false,
                Some("wss://example.com"),
                5,
                Some(2000),
            )
            .unwrap_or_else(|e| panic!("Failed to build output data: {e}"))
            .unwrap_or_else(|| panic!("Should return Some"));

        // 验证 collected_messages 包含所有 4 条消息
        let collected_arr = result["collected_messages"]
            .as_array()
            .unwrap_or_else(|| panic!("Should be an array"));
        assert_eq!(collected_arr.len(), 4, "DurationCollect 应返回所有消息");
        assert_eq!(collected_arr[0]["data"], "msg1");
        assert_eq!(collected_arr[1]["data"], "msg2");
        assert_eq!(collected_arr[2]["data"], "msg3");
        assert_eq!(collected_arr[3]["data"], "msg4");

        // 验证 matched_message 是最后一条
        assert_eq!(result["matched_message"]["data"], "msg4");
        assert_eq!(result["message_count"], 4);
    }

    /// 测试：超时无消息时 `collected_messages` 为空数组
    #[test]
    fn test_collected_messages_empty() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        // 模拟超时场景：没有收到任何消息
        let collected: Vec<String> = vec![];
        let result = executor
            .build_output_data(
                "conn-4",
                "ping",
                false,
                &collected,
                None, // 无匹配
                true, // 超时
                Some("wss://example.com"),
                10,
                Some(10000),
            )
            .unwrap_or_else(|e| panic!("Failed to build output data: {e}"))
            .unwrap_or_else(|| panic!("Should return Some"));

        // 验证 collected_messages 为空数组
        let collected_arr = result["collected_messages"]
            .as_array()
            .unwrap_or_else(|| panic!("Should be an array"));
        assert_eq!(collected_arr.len(), 0, "超时时应返回空数组");

        // 验证 matched_message 为 null
        assert!(result["matched_message"].is_null());
        assert!(result["message"].is_null());

        // 验证 message_count 为 0
        assert_eq!(result["message_count"], 0);

        // 验证超时标记
        assert_eq!(result["collect_timed_out"], true);
    }

    /// `测试：output_mappings` 能通过 $.`collected_messages`[0].data 提取值
    #[test]
    fn test_output_mapping_collected_messages() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsSendCollectExecutor::new(ws_manager);

        let collected = vec![
            r#"{"status":"ok"}"#.to_string(),
            r#"{"status":"done"}"#.to_string(),
        ];
        let output_data = executor
            .build_output_data(
                "conn-5",
                "ping",
                false,
                &collected,
                Some(1),
                false,
                Some("wss://example.com"),
                10,
                Some(5),
            )
            .unwrap_or_else(|e| panic!("Failed to build output data: {e}"))
            .unwrap_or_else(|| panic!("Should return Some"));

        // 验证 $.collected_messages[0].data 路径可以提取值
        let value = flow_engine_core::utils::extract_value_by_path(
            &output_data,
            "$.collected_messages[0].data",
        );
        assert!(
            value.is_some(),
            "$.collected_messages[0].data 应该能提取到值"
        );
        assert_eq!(
            value.unwrap_or_else(|| panic!("Value does not exist")),
            r#"{"status":"ok"}"#
        );

        // 验证 $.collected_messages[1].data 路径
        let value2 = flow_engine_core::utils::extract_value_by_path(
            &output_data,
            "$.collected_messages[1].data",
        );
        assert!(
            value2.is_some(),
            "$.collected_messages[1].data 应该能提取到值"
        );
        assert_eq!(
            value2.unwrap_or_else(|| panic!("Value does not exist")),
            r#"{"status":"done"}"#
        );

        // 验证 $.message_count 路径
        let count = flow_engine_core::utils::extract_value_by_path(&output_data, "$.message_count");
        assert!(count.is_some());
        assert_eq!(count.unwrap_or_else(|| panic!("Value does not exist")), 2);

        // 验证 $.matched_message.data 路径
        let matched =
            flow_engine_core::utils::extract_value_by_path(&output_data, "$.matched_message.data");
        assert!(matched.is_some());
        assert_eq!(
            matched.unwrap_or_else(|| panic!("Value does not exist")),
            r#"{"status":"done"}"#
        );
    }
}
