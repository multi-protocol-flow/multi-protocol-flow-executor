//! 可调试的流程执行器
//!
//! 支持断点、单步执行、变量查看等调试功能

use flow_engine_core::connection::Connection;
use flow_engine_core::context::{ExecutionContext, VariableStorage};
use crate::debug::{
    DebugCommand, DebugEvent, DebugSessionState, DebugStatus, EndReason, ExecutionStackEntry,
    PauseReason,
};
use flow_engine_core::flow::Flow;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_ws::websocket_manager::WebSocketConnectionManager;
use flow_engine_core::node_types;
use flow_engine_core::traits::LogSender;
use flow_engine_db::db::DatabaseConnectionManager;
use flow_engine_core::registry::NodeRegistry;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use uuid::Uuid;

/// 心跳发送间隔（毫秒）
const HEARTBEAT_INTERVAL_MS: u64 = 5000;

/// 跳过节点的结果
struct SkipNodeResult {
    /// 下一个节点 UUID
    next_node_uuid: Option<String>,
    /// 模拟输出（用于堆栈记录）
    simulated_output: Option<serde_json::Value>,
}

/// 可调试的流程执行器
pub struct DebuggableFlowExecutor {
    /// 流程文件路径（用于 CLI 相对路径解析）
    flow_file_path: Option<PathBuf>,
    /// 节点插件注册表（通过 registry 查找替代 match 分发）
    registry: Arc<NodeRegistry>,
}

impl Default for DebuggableFlowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl DebuggableFlowExecutor {
    /// 创建新的可调试执行器
    pub fn new() -> Self {
        DebuggableFlowExecutor {
            flow_file_path: None,
            registry: Arc::new(crate::default_registry::default_registry()),
        }
    }

    /// 设置流程文件路径（用于 CLI 相对路径解析）
    pub fn with_flow_file_path(mut self, path: PathBuf) -> Self {
        self.flow_file_path = Some(path);
        self
    }

    /// 启动调试会话
    ///
    /// # 参数
    /// - `flow`: 要调试的流程
    /// - `variables`: 初始变量
    /// - `breakpoints`: 断点列表（节点 UUID）
    /// - `log_sender`: 日志发送器
    /// - `event_sender`: 调试事件发送器
    /// - `command_receiver`: 调试命令接收器
    ///
    /// # 返回
    /// 调试会话的最终状态
    pub async fn start_debug(
        &self,
        flow: &Flow,
        variables: Option<serde_json::Value>,
        breakpoints: Vec<String>,
        log_sender: Option<Arc<dyn LogSender>>,
        event_sender: mpsc::UnboundedSender<DebugEvent>,
        command_receiver: mpsc::UnboundedReceiver<DebugCommand>,
    ) -> DebugSessionState {
        // 初始化调试会话状态
        let session_id = Uuid::new_v4().to_string();
        let mut debug_state = DebugSessionState::new(session_id, flow.uuid.clone());
        debug_state.set_breakpoints(breakpoints);
        debug_state.status = DebugStatus::Running;
        // 记录调试开始时间
        debug_state.start_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 创建共享状态（用于心跳任务读取当前状态）
        let shared_state = Arc::new(RwLock::new(debug_state.clone()));
        
        // 创建心跳任务停止信号
        let (stop_tx, stop_rx) = broadcast::channel::<()>(1);
        
        // 启动心跳任务
        let heartbeat_state = Arc::clone(&shared_state);
        let heartbeat_sender = event_sender.clone();
        let flow_uuid_for_heartbeat = flow.uuid.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_millis(HEARTBEAT_INTERVAL_MS)
            );
            let mut stop_rx = stop_rx;
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // 读取当前状态并发送心跳
                        if let Ok(state) = heartbeat_state.read() {
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;
                            
                            let _ = heartbeat_sender.send(DebugEvent::Heartbeat {
                                status: state.status.clone(),
                                current_node_uuid: state.current_node_uuid.clone(),
                                timestamp,
                            });
                        }
                    }
                    _ = stop_rx.recv() => {
                        log::info!("[调试 {}] 心跳任务已停止", flow_uuid_for_heartbeat);
                        break;
                    }
                }
            }
        });

        // 发送会话启动事件
        if let Err(e) = event_sender.send(DebugEvent::SessionStarted {
            flow_uuid: flow.uuid.clone(),
        }) {
            log::error!("[调试 {}] 发送 SessionStarted 事件失败: {}", flow.uuid, e);
        }

        // 初始化执行上下文
        let initial_variables = variables.unwrap_or_else(|| json!({}));
        let mut var_storage = VariableStorage::new();
        if let Some(obj) = initial_variables.as_object() {
            for (key, value) in obj {
                var_storage.set_variable(key.clone(), value.clone(), String::new());
            }
        }

        // 初始化 WebSocket 连接管理器
        let ws_manager = Arc::new(WebSocketConnectionManager::new());

        // 初始化数据库连接池管理器
        let db_manager = Arc::new(DatabaseConnectionManager::new());

        let mut context = ExecutionContext {
            flow_uuid: flow.uuid.clone(),
            current_node_uuid: None,
            variables: var_storage,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            log_sender,
            flow_file_path: self.flow_file_path.clone(),
            ws_manager: Some(ws_manager),
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: Some(db_manager),
            grpc_manager: None,
            last_node_output: None,
            debug_event_sender: Some(Arc::new(crate::debug_event_adapter::MpscDebugEventSender::new(event_sender.clone()))),
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        };

        // 记录调试开始时间（用于报告生成）
        let debug_start_time_ms = context.flow_trigger_time;

        // 查找开始节点
        let start_node = match flow
            .nodes
            .iter()
            .find(|node| node.node_type == node_types::ENTRY)
        {
            Some(node) => node,
            None => {
                // 停止心跳任务
                let _ = stop_tx.send(());
                debug_state.status = DebugStatus::Ended;
                // 生成报告（即使是错误也要生成）
                let report = Self::generate_debug_report(&debug_state, flow, debug_start_time_ms, &context).await;
                if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                    reason: EndReason::Error {
                        message: "流程缺少开始节点".to_string(),
                    },
                    report: Some(report),
                    execution_stack: debug_state.execution_stack.clone(),
                }) {
                    log::error!("[调试 {}] 发送 SessionEnded 事件失败: {}", flow.uuid, e);
                }
                return debug_state;
            }
        };

        // 构建节点查找映射
        let node_map: HashMap<String, &Node> = flow
            .nodes
            .iter()
            .map(|node| (node.uuid.clone(), node))
            .collect();

        // 构建连接映射
        let mut connection_map: HashMap<String, Vec<String>> = HashMap::new();
        for conn in &flow.connections {
            connection_map
                .entry(conn.source_node_uuid.clone())
                .or_default()
                .push(conn.target_node_uuid.clone());
        }

        // 执行调试主循环
        let result = self.execute_debug_loop(
            &mut debug_state,
            flow,
            start_node,
            &node_map,
            &connection_map,
            &mut context,
            event_sender,
            command_receiver,
            &shared_state,
        ).await;
        
        // 停止心跳任务
        let _ = stop_tx.send(());
        
        // 更新最终状态到共享状态
        if let Ok(mut state) = shared_state.write() {
            *state = result.clone();
        }
        
        result
    }

    /// 执行调试主循环
    #[allow(clippy::too_many_arguments)]
    async fn execute_debug_loop(
        &self,
        debug_state: &mut DebugSessionState,
        flow: &Flow,
        start_node: &Node,
        node_map: &HashMap<String, &Node>,
        connection_map: &HashMap<String, Vec<String>>,
        context: &mut ExecutionContext,
        event_sender: mpsc::UnboundedSender<DebugEvent>,
        mut command_receiver: mpsc::UnboundedReceiver<DebugCommand>,
        shared_state: &Arc<RwLock<DebugSessionState>>,
    ) -> DebugSessionState {
        // 辅助函数：更新共享状态
        let update_shared_state = |state: &DebugSessionState| {
            if let Ok(mut shared) = shared_state.write() {
                *shared = state.clone();
            }
        };
        
        let mut current_node_id = start_node.uuid.clone();
        let mut visited = std::collections::HashSet::new();

        // 主循环
        while let Some(node) = node_map.get(&current_node_id) {
            // 检查循环
            if visited.contains(&current_node_id) {
                debug_state.status = DebugStatus::Ended;
                update_shared_state(debug_state);
                let report = Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context).await;
                if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                    reason: EndReason::Error {
                        message: format!("检测到循环：节点 {} 已被访问", node.uuid),
                    },
                    report: Some(report),
                    execution_stack: debug_state.execution_stack.clone(),
                }) {
                    log::error!("[调试 {}] 发送 SessionEnded 事件失败: {}", flow.uuid, e);
                }
                return debug_state.clone();
            }

            visited.insert(current_node_id.clone());
            context.current_node_uuid = Some(current_node_id.clone());
            debug_state.current_node_uuid = Some(current_node_id.clone());

            // 检查是否需要暂停（断点或单步模式）
            // 当 skip_breakpoints 为 true 时，不暂停（执行到底模式）
            let should_pause = !debug_state.skip_breakpoints
                && (debug_state.has_breakpoint(&current_node_id) || debug_state.is_stepping);

            if should_pause {
                // 确定暂停原因：单步模式优先于断点
                let pause_reason = if debug_state.is_stepping && !debug_state.skip_breakpoints {
                    // 单步执行完成，在此节点暂停
                    // 重置单步模式标志（在暂停时重置，而不是在执行前）
                    debug_state.is_stepping = false;
                    PauseReason::StepComplete {
                        node_uuid: current_node_id.clone(),
                    }
                } else {
                    // 断点触发暂停
                    PauseReason::Breakpoint {
                        node_uuid: current_node_id.clone(),
                    }
                };

                // 在节点执行前暂停
                debug_state.status = DebugStatus::Paused;
                debug_state.pause_reason = Some(pause_reason.clone());

                // 同步变量到调试状态
                debug_state.variables = context.variables.get_merged_variables();
                
                // 更新共享状态
                update_shared_state(debug_state);

                // 发送暂停事件
                if let Err(e) = event_sender.send(DebugEvent::PausedAtNode {
                    node_uuid: current_node_id.clone(),
                    reason: pause_reason,
                    variables: debug_state.variables.clone(),
                    execution_stack: debug_state.execution_stack.clone(),
                }) {
                    log::error!("[调试 {}] 发送 PausedAtNode 事件失败 (节点 {}): {}", context.flow_uuid, current_node_id, e);
                }

                // 等待调试命令
                let command_result = self
                    .wait_for_command(
                        debug_state,
                        node,
                        context,
                        node_map,
                        &flow.connections,
                        &event_sender,
                        &mut command_receiver,
                        flow,
                    )
                    .await;

                match command_result {
                    Ok(Some((next_node_uuid_opt, simulated_output))) => {
                        // 跳过节点，创建堆栈条目并发送事件
                        let node_end_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;

                        // 解析节点配置为 JSON（用于 resolved_config 字段）
                        let resolved_config = serde_json::to_value(&node.data).ok();

                        // 创建跳过节点的堆栈条目
                        let stack_entry = ExecutionStackEntry {
                            node_uuid: node.uuid.clone(),
                            node_name: node.name.clone(),
                            node_type: node.node_type.clone(),
                            started_at: node_end_ms,
                            ended_at: Some(node_end_ms),
                            input: Some(json!(context.variables.get_merged_variables())),
                            output: simulated_output.clone(),
                            success: true,
                            error: Some("节点已跳过".to_string()),
                            used_port_id: Some("skipped".to_string()),
                            resolved_config,
                            variable_changes: None,
                            skip_reason: Some("用户跳过".to_string()),
                        };
                        debug_state.push_stack_entry(stack_entry);

                        // 发送节点执行完成事件（包含完整的执行信息）
                        let skip_entry = debug_state.execution_stack.last().expect("stack entry just pushed").clone();
                        if let Err(e) = event_sender.send(DebugEvent::NodeExecuted {
                            entry: skip_entry,
                        }) {
                            log::error!("[调试 {}] 发送 NodeExecuted 事件失败 (跳过节点 {}): {}", context.flow_uuid, node.uuid, e);
                        }

                        // 检查是否有下一个节点
                        if let Some(next_node_uuid) = next_node_uuid_opt {
                            // 有下一个节点，跳转并继续
                            debug_state.status = DebugStatus::Running;
                            update_shared_state(debug_state);
                            current_node_id = next_node_uuid;
                            // 不执行当前节点，直接进入下一次循环
                            continue;
                        } else {
                            // 没有下一个节点，结束流程
                            debug_state.status = DebugStatus::Ended;
                            debug_state.current_node_uuid = None;
                            update_shared_state(debug_state);
                            let report = Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context).await;
                            if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                                reason: EndReason::Completed,
                                report: Some(report),
                                execution_stack: debug_state.execution_stack.clone(),
                            }) {
                                log::error!("[调试 {}] 发送 SessionEnded 事件失败: {}", flow.uuid, e);
                            }
                            return debug_state.clone();
                        }
                    }
                    Ok(None) => {
                        // 继续执行当前节点
                        // 注意：不在这里重置 is_stepping，让它在下一个节点检查时生效
                        // is_stepping 会在下一个节点暂停时重置（见 should_pause 分支）
                        debug_state.status = DebugStatus::Running;
                        update_shared_state(debug_state);
                    }
                    Err(error) => {
                        // 用户终止或错误
                        if error == "用户终止调试" {
                            debug_state.status = DebugStatus::Ended;
                            let report = Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context).await;
                            if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                                reason: EndReason::Error {
                                    message: error,
                                },
                                report: Some(report),
                                execution_stack: debug_state.execution_stack.clone(),
                            }) {
                                log::error!("[调试 {}] 发送 SessionEnded 事件失败: {}", flow.uuid, e);
                            }
                            return debug_state.clone();
                        }
                        debug_state.status = DebugStatus::Ended;
                        update_shared_state(debug_state);
                        let report = Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context).await;
                        if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                            reason: EndReason::Error {
                                message: error,
                            },
                            report: Some(report),
                            execution_stack: debug_state.execution_stack.clone(),
                        }) {
                            log::error!("[调试 {}] 发送 SessionEnded 事件失败: {}", flow.uuid, e);
                        }
                        return debug_state.clone();
                    }
                }
            }

            // 执行当前节点
            let node_start_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            // 解析节点配置为 JSON（用于 resolved_config 字段）
            let resolved_config = serde_json::to_value(&node.data).ok();

            // 创建执行堆栈条目
            let stack_entry = ExecutionStackEntry {
                node_uuid: node.uuid.clone(),
                node_name: node.name.clone(),
                node_type: node.node_type.clone(),
                started_at: node_start_ms,
                ended_at: None,
                input: Some(json!(context.variables.get_merged_variables())),
                output: None,
                success: false,
                error: None,
                used_port_id: None,
                resolved_config: resolved_config.clone(),
                variable_changes: None,
                skip_reason: None,
            };
            debug_state.push_stack_entry(stack_entry);

            // 检查是否有临时节点配置
            let effective_node = if let Some(temp_config) = debug_state.get_temp_node_config(&node.uuid) {
                // 尝试反序列化临时配置
                match serde_json::from_value::<NodeData>(temp_config.clone()) {
                    Ok(new_data) => {
                        log::info!(
                            "[调试 {}] 应用临时配置到节点: {} ({})",
                            context.flow_uuid,
                            node.name,
                            node.uuid
                        );
                        // 创建使用临时配置的节点副本
                        // node 是 &&Node 类型，需要解引用
                        Node {
                            data: new_data,
                            ..(*node).clone()
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "[调试 {}] 临时配置反序列化失败，使用原始配置: {}",
                            context.flow_uuid,
                            e
                        );
                        (*node).clone()
                    }
                }
            } else {
                (*node).clone()
            };

            // 执行节点
            match self
                .execute_node(&effective_node, context, &flow.connections, node_map)
                .await
            {
                Ok((next_node_ids, output_data)) => {
                    // 更新堆栈条目
                    let node_end_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;

                    // 根据节点类型和输出数据确定 used_port_id（通过 registry 查找）
                    let type_id = node.data.type_name();
                    let used_port_id = self.registry.get(&type_id)
                        .map(|plugin| Some(plugin.resolve_port(&output_data)))
                        .unwrap_or(None);

                    // 从实际使用的节点配置获取 resolved_config
                    let effective_resolved_config = serde_json::to_value(&effective_node.data).ok();

                    debug_state.update_last_stack_entry(
                        node_end_ms,
                        output_data.clone(),
                        true,
                        None,
                        used_port_id,
                        effective_resolved_config,
                        None, // variable_changes 在 update 时更新
                        None, // skip_reason 仅跳过节点时设置
                    );

                    // 发送节点执行完成事件（包含完整的执行信息）
                    let completed_entry = debug_state.execution_stack.last().expect("stack entry just pushed").clone();
                    if let Err(e) = event_sender.send(DebugEvent::NodeExecuted {
                        entry: completed_entry,
                    }) {
                        log::error!("[调试 {}] 发送 NodeExecuted 事件失败 (节点 {}): {}", context.flow_uuid, node.uuid, e);
                    }

                    // 协议节点：Http, WsConnect, WsSendCollect
                    // 工具节点：Entry, Conditional, WsClose（不更新，延续上游）
                    let type_id = node.data.type_name();
                    let is_protocol_node = self.registry.get(&type_id)
                        .map(|p| p.is_protocol_node())
                        .unwrap_or(false);
                    if is_protocol_node {
                        context.last_node_output = output_data.clone();
                        log::debug!(
                            "[调试 {}] 更新 last_node_output（协议节点 {}）",
                            context.flow_uuid,
                            node.name
                        );
                    }

                    // 查找下一个节点
                    let next_node_id = if !next_node_ids.is_empty() {
                        Some(next_node_ids[0].clone())
                    } else {
                        connection_map
                            .get(&current_node_id)
                            .and_then(|targets| targets.first().cloned())
                    };

                    if let Some(next_id) = next_node_id {
                        current_node_id = next_id;
                    } else {
                        // 执行完成
                        break;
                    }
                }
                Err(error) => {
                    // 更新堆栈条目
                    let node_end_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;

                    // 检查节点是否支持 false 端口，以及是否有 false 端口的连接
                    let type_id = node.data.type_name();
                    let supports_false_port = self.registry.get(&type_id)
                        .map(|p| p.supports_false_port())
                        .unwrap_or(false);

                    let false_port_next_node = if supports_false_port {
                        // 查找 false 端口的连接
                        flow.connections
                            .iter()
                            .find(|conn| {
                                conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                            })
                            .map(|conn| conn.target_node_uuid.clone())
                    } else {
                        None
                    };

                    // 更新堆栈条目，根据是否有 false 端口连接设置 used_port_id
                    let used_port_id = if false_port_next_node.is_some() {
                        Some("false".to_string())
                    } else {
                        None
                    };

                    // 从实际使用的节点配置获取 resolved_config
                    let effective_resolved_config = serde_json::to_value(&effective_node.data).ok();

                    debug_state.update_last_stack_entry(
                        node_end_ms,
                        None,
                        false,
                        Some(error.clone()),
                        used_port_id,
                        effective_resolved_config,
                        None, // variable_changes
                        None, // skip_reason
                    );

                    // 发送节点执行完成事件（包含完整的执行信息）
                    let failed_entry = debug_state.execution_stack.last().expect("stack entry just pushed").clone();
                    if let Err(e) = event_sender.send(DebugEvent::NodeExecuted {
                        entry: failed_entry,
                    }) {
                        log::error!("[调试 {}] 发送 NodeExecuted 事件失败 (节点 {} 失败): {}", context.flow_uuid, node.uuid, e);
                    }

                    if let Some(next_node_uuid) = false_port_next_node {
                        // 有 false 端口连接，继续执行 false 分支
                        log::info!(
                            "[调试 {}] 节点 {} 执行失败，但有 false 端口连接，继续执行节点 {}",
                            context.flow_uuid,
                            node.name,
                            next_node_uuid
                        );

                        current_node_id = next_node_uuid;
                        continue;
                    }

                    // 没有 false 端口连接或节点不支持，结束调试
                    // 发送错误事件
                    if let Err(e) = event_sender.send(DebugEvent::Error {
                        message: error.clone(),
                    }) {
                        log::error!("[调试 {}] 发送 Error 事件失败: {}", context.flow_uuid, e);
                    }

                    // 结束调试
                    debug_state.status = DebugStatus::Ended;
                    update_shared_state(debug_state);
                    let report = Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context).await;
                    if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                        reason: EndReason::Error {
                            message: error,
                        },
                        report: Some(report),
                        execution_stack: debug_state.execution_stack.clone(),
                    }) {
                        log::error!("[调试 {}] 发送 SessionEnded 事件失败: {}", flow.uuid, e);
                    }
                    return debug_state.clone();
                }
            }

        }

        
        // 正常完成
        debug_state.status = DebugStatus::Ended;
        debug_state.current_node_uuid = None;
        update_shared_state(debug_state);
        let report = Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context).await;
        if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
            reason: EndReason::Completed,
            report: Some(report),
            execution_stack: debug_state.execution_stack.clone(),
        }) {
            log::error!("[调试 {}] 发送 SessionEnded 事件失败: {}", flow.uuid, e);
        }

        debug_state.clone()
    }

    /// 等待调试命令
    /// 
    /// 返回值：
    /// - `Ok(Some((Some(next_uuid), output)))`：跳过节点，跳转到下一个节点
    /// - `Ok(Some((None, output)))`：跳过节点，没有下一个节点，结束流程
    /// - `Ok(None)`：继续执行当前节点
    /// - `Err(msg)`：错误或用户终止
    #[allow(clippy::too_many_arguments)]
    async fn wait_for_command(
        &self,
        debug_state: &mut DebugSessionState,
        current_node: &Node,
        context: &mut ExecutionContext,
        _node_map: &HashMap<String, &Node>,
        connections: &[Connection],
        event_sender: &mpsc::UnboundedSender<DebugEvent>,
        command_receiver: &mut mpsc::UnboundedReceiver<DebugCommand>,
        flow: &Flow,
    ) -> Result<Option<(Option<String>, Option<serde_json::Value>)>, String> {
        while let Some(command) = command_receiver.recv().await {
            match command {
                DebugCommand::Continue => {
                    // 继续执行到下一个断点
                    debug_state.is_stepping = false;
                    debug_state.skip_breakpoints = false;
                    return Ok(None);
                }
                DebugCommand::StepOver => {
                    // 单步执行
                    debug_state.is_stepping = true;
                    debug_state.skip_breakpoints = false;
                    return Ok(None);
                }
                DebugCommand::SkipNode {
                    selected_port,
                    simulated_output,
                } => {
                    // 跳过节点（增强版本，支持分支选择）
                    // 重置单步模式，跳过后继续执行到下一个断点
                    debug_state.is_stepping = false;
                    debug_state.skip_breakpoints = false;
                    let result = self
                        .handle_skip_node(
                            debug_state,
                            current_node,
                            context,
                            connections,
                            selected_port,
                            simulated_output,
                        )
                        .await?;
                    // 如果有下一个节点，返回 Some；否则返回 None（表示跳过节点并结束流程）
                    return Ok(Some((result.next_node_uuid, result.simulated_output)));
                }
                DebugCommand::RunToEnd => {
                    // 执行到底
                    debug_state.is_stepping = false;
                    debug_state.skip_breakpoints = true;
                    return Ok(None);
                }
                DebugCommand::EndDebug => {
                    // 结束调试
                    debug_state.status = DebugStatus::Ended;
                    let report = Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context).await;
                    if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                        reason: EndReason::UserAbort,
                        report: Some(report),
                        execution_stack: debug_state.execution_stack.clone(),
                    }) {
                        log::error!("[调试] 发送 SessionEnded 事件失败: {}", e);
                    }
                    return Err("用户终止调试".to_string());
                }
                DebugCommand::ModifyVariable { name, value } => {
                    // 修改变量
                    context
                        .variables
                        .set_variable(name.clone(), value.clone(), String::new());
                    debug_state.set_variable(name.clone(), value.clone());

                    // 发送变量修改事件
                    if let Err(e) = event_sender.send(DebugEvent::VariableModified { name, value }) {
                        log::error!("[调试] 发送 VariableModified 事件失败: {}", e);
                    }
                }
                DebugCommand::ModifyNodeConfig { node_uuid, config } => {
                    // 修改节点配置（临时）
                    debug_state.set_temp_node_config(node_uuid.clone(), config.clone());

                    // 发送节点配置修改事件
                    if let Err(e) = event_sender.send(DebugEvent::NodeConfigModified { node_uuid, config }) {
                        log::error!("[调试] 发送 NodeConfigModified 事件失败: {}", e);
                    }
                }
                DebugCommand::SetBreakpoint { node_uuid, enabled } => {
                    // 动态设置断点
                    if enabled {
                        debug_state.add_breakpoint(node_uuid.clone());
                        log::info!("[调试] 添加断点: {}", node_uuid);
                    } else {
                        debug_state.remove_breakpoint(&node_uuid);
                        log::info!("[调试] 移除断点: {}", node_uuid);
                    }

                    // 发送断点修改事件
                    if let Err(e) = event_sender.send(DebugEvent::BreakpointModified {
                        node_uuid,
                        enabled,
                    }) {
                        log::error!("[调试] 发送 BreakpointModified 事件失败: {}", e);
                    }
                }
                DebugCommand::StartDebug { .. } => {
                    // 调试已启动，忽略此命令
                    log::warn!("调试会话已启动，忽略 StartDebug 命令");
                }
            }
        }

        // 命令通道关闭
        Err("调试命令通道已关闭".to_string())
    }

    /// 处理跳过节点
    async fn handle_skip_node(
        &self,
        debug_state: &mut DebugSessionState,
        current_node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        selected_port: Option<String>,
        simulated_output: Option<serde_json::Value>,
    ) -> Result<SkipNodeResult, String> {
        log::info!(
            "[调试 {}] 处理跳过节点: {} ({}), selected_port: {:?}",
            context.flow_uuid,
            current_node.name,
            current_node.uuid,
            selected_port
        );

        // 如果有模拟输出，注入到变量池
        if let Some(ref output) = simulated_output {
            if let Some(obj) = output.as_object() {
                for (key, value) in obj {
                    context
                        .variables
                        .set_variable(key.clone(), value.clone(), current_node.uuid.clone());
                    debug_state.set_variable(key.clone(), value.clone());
                }
            }
        }

        // 根据 selected_port 查找下一个节点
        let next_node_uuid = if let Some(port) = selected_port {
            // 根据指定的输出端口查找连接
            log::info!(
                "[调试 {}] 查找连接: source_node_uuid={}, source_port_id={}",
                context.flow_uuid,
                current_node.uuid,
                port
            );
            
            // 打印所有连接用于调试
            for conn in connections {
                log::info!(
                    "[调试 {}] 连接: {} -> {} (port: {} -> {})",
                    context.flow_uuid,
                    conn.source_node_uuid,
                    conn.target_node_uuid,
                    conn.source_port_id,
                    conn.target_port_id
                );
            }
            
            let found = connections
                .iter()
                .find(|conn| {
                    conn.source_node_uuid == current_node.uuid && conn.source_port_id == port
                })
                .map(|conn| {
                    log::info!(
                        "[调试 {}] 找到匹配的连接，目标节点: {}",
                        context.flow_uuid,
                        conn.target_node_uuid
                    );
                    conn.target_node_uuid.clone()
                });
            
            if found.is_none() {
                log::warn!(
                    "[调试 {}] 未找到匹配的连接: source_node_uuid={}, source_port_id={}",
                    context.flow_uuid,
                    current_node.uuid,
                    port
                );
            }
            
            found
        } else {
            // 没有指定端口，使用第一个连接（默认行为）
            connections
                .iter()
                .find(|conn| conn.source_node_uuid == current_node.uuid)
                .map(|conn| conn.target_node_uuid.clone())
        };

        log::info!(
            "[调试 {}] 跳过节点结果: next_node_uuid={:?}",
            context.flow_uuid,
            next_node_uuid
        );

        Ok(SkipNodeResult {
            next_node_uuid,
            simulated_output,
        })
    }

    /// 执行单个节点
    async fn execute_node(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 发送节点开始日志
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("执行节点 {} ({})", node.name, node.node_type),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        let type_id = node.data.type_name();
        match self.registry.get(&type_id) {
            Some(plugin) => {
                log::info!(
                    "[调试 {}] 执行{}节点: {} ({})",
                    context.flow_uuid,
                    plugin.display_name(),
                    node.name,
                    node.uuid
                );
                plugin.execute(node, context, connections, node_map).await
            }
            None => Err(format!("未注册的节点类型: {}", type_id)),
        }
    }

    /// 从执行堆栈生成流程执行报告
    ///
    /// # 参数
    /// - `debug_state`: 调试会话状态
    /// - `flow`: 流程定义
    /// - `start_time_ms`: 调试开始时间（毫秒）
    /// - `context`: 执行上下文（用于收集 WebSocket 消息流）
    ///
    /// # 返回
    /// - `FlowExecutionReport`: 流程执行报告
    async fn generate_debug_report(
        debug_state: &DebugSessionState,
        flow: &Flow,
        start_time_ms: u64,
        context: &flow_engine_core::context::ExecutionContext,
    ) -> flow_engine_core::report::FlowExecutionReport {
        use flow_engine_core::report::{ExecutionMode, ExecutionStatus, NodeExecutionReport};

        let end_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 将 execution_stack 转换为 NodeExecutionReport
        let node_reports: Vec<NodeExecutionReport> = debug_state
            .execution_stack
            .iter()
            .map(|entry| {
                // 判断节点状态：如果 error 包含"跳过"则为 Skipped
                let status = if entry.error.as_ref().map(|e| e.contains("跳过")).unwrap_or(false) {
                    ExecutionStatus::Skipped
                } else if entry.success {
                    ExecutionStatus::Success
                } else {
                    ExecutionStatus::Failed
                };

                // 使用 entry 中的 used_port_id
                let used_port_id = entry.used_port_id.clone();

                // 处理错误信息
                let error_detail = if status == ExecutionStatus::Skipped {
                    None
                } else {
                    entry.error.as_ref().map(|e| {
                        flow_engine_core::error::ErrorDetail::new(
                            flow_engine_core::error::ErrorKind::Execution,
                            e.clone(),
                        )
                    })
                };

                NodeExecutionReport {
                    node_uuid: entry.node_uuid.clone(),
                    node_type: entry.node_type.clone(),
                    node_name: entry.node_name.clone(),
                    start_time: entry.started_at,
                    end_time: entry.ended_at.unwrap_or(end_time_ms),
                    duration_ms: entry.ended_at.map(|e| e - entry.started_at).unwrap_or(0),
                    status,
                    input_data: entry.input.clone(),
                    output_data: entry.output.clone(),
                    used_port_id,
                    error: error_detail,
                    // 新增字段（Task 3, Task 11 映射）
                    resolved_config: entry.resolved_config.clone(),
                    variable_changes: entry.variable_changes.clone(),
                    skip_reason: entry.skip_reason.clone(),
                    wait_duration_ms: None,
                }
            })
            .collect();

        // 计算整体状态
        let flow_status = if node_reports.iter().any(|r| r.status == ExecutionStatus::Failed) {
            ExecutionStatus::Failed
        } else if node_reports.iter().all(|r| r.status == ExecutionStatus::Skipped) {
            ExecutionStatus::Skipped
        } else {
            ExecutionStatus::Success
        };

        // 计算统计信息（在移动 node_reports 之前）
        let statistics = flow_engine_core::statistics::FlowStatistics::from_reports(
            &node_reports,
            flow.nodes.len() as u32,
        );
        let executed_count = node_reports.len() as u32;

        flow_engine_core::report::FlowExecutionReport {
            uuid: Uuid::new_v4().to_string(),
            flow_uuid: flow.uuid.clone(),
            flow_name: flow.name.clone(),
            start_time: start_time_ms,
            end_time: end_time_ms,
            duration_ms: end_time_ms.saturating_sub(start_time_ms),
            total_nodes: flow.nodes.len() as u32,
            executed_count,
            status: flow_status,
            node_reports,
            statistics,
            flow_error: None,
            execution_mode: ExecutionMode::Debug,
            // 新增字段（Task 4）
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            // 收集 WebSocket 消息流（完整数据）
            ws_streams: {
                let streams = context.ws_streams.read().await;
                streams.values().cloned().collect()
            },
            flow_snapshot: Some(flow.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::flow::Flow;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::NodeData;

    #[tokio::test]
    async fn test_debuggable_executor_creation() {
        let executor = DebuggableFlowExecutor::new();
        assert!(executor.flow_file_path.is_none());

        let executor_with_path =
            DebuggableFlowExecutor::new().with_flow_file_path(PathBuf::from("/test/path"));
        assert!(executor_with_path.flow_file_path.is_some());
    }

    #[tokio::test]
    async fn test_debug_session_initialization() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let (event_sender, _event_receiver) = mpsc::unbounded_channel();
        let (_command_sender, command_receiver) = mpsc::unbounded_channel();

        let executor = DebuggableFlowExecutor::new();
        let debug_state = executor
            .start_debug(&flow, None, vec![], None, event_sender, command_receiver)
            .await;

        assert_eq!(debug_state.flow_uuid, flow_id);
        assert_eq!(debug_state.status, DebugStatus::Ended);
    }

    #[tokio::test]
    async fn test_debug_session_with_breakpoint() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node.clone()],
            connections: vec![],
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话
        let executor = DebuggableFlowExecutor::new();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow,
                    None,
                    vec![start_node.uuid.clone()],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 等待暂停事件
        if let Some(event) = event_receiver.recv().await {
            match event {
                DebugEvent::SessionStarted { flow_uuid } => {
                    assert_eq!(flow_uuid, flow_id);
                }
                _ => {}
            }
        }

        // 发送继续命令
        let _ = command_sender.send(DebugCommand::Continue);

        // 等待会话结束
        let result = handle.await;
        assert!(result.is_ok());
    }

    /// 测试单步执行功能：StepOver 应该在下一个节点暂停，无论是否有断点
    #[tokio::test]
    async fn test_step_over_pauses_at_next_node() {
        // 创建一个有两个节点的流程：Entry -> Entry（第二个节点没有断点）
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true), // 第一个节点有断点
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None, // 第二个节点没有断点！
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: second_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试单步执行".to_string(),
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话（只在第一个节点设置断点）
        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let start_node_uuid = start_node.uuid.clone();
        let _second_node_uuid = second_node.uuid.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![start_node_uuid], // 只在第一个节点有断点
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 收集事件
        let mut paused_count = 0;
        let mut pause_reasons: Vec<PauseReason> = vec![];
        let mut paused_nodes: Vec<String> = vec![];

        // 处理事件循环
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::SessionStarted { .. } => {
                            // 会话启动，什么都不做
                        }
                        DebugEvent::PausedAtNode { node_uuid, reason, .. } => {
                            paused_count += 1;
                            pause_reasons.push(reason.clone());
                            paused_nodes.push(node_uuid.clone());

                            // 如果是第一个节点（断点暂停），发送 StepOver 命令
                            if node_uuid == start_node.uuid {
                                let _ = command_sender.send(DebugCommand::StepOver);
                            } else if node_uuid == second_node.uuid {
                                // 如果是第二个节点（单步暂停），发送继续命令结束测试
                                let _ = command_sender.send(DebugCommand::Continue);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            // 会话结束
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    // 超时，结束测试
                    panic!("测试超时");
                }
            }
        }

        // 等待调试会话完成
        let result = handle.await;
        assert!(result.is_ok());

        // 验证：应该暂停两次
        // 1. 第一次在 start_node（断点暂停）
        // 2. 第二次在 second_node（单步执行暂停，即使没有断点）
        assert_eq!(paused_count, 2, "应该暂停两次：断点和单步");
        assert_eq!(paused_nodes.len(), 2);
        assert_eq!(paused_nodes[0], start_node.uuid, "第一次暂停应该在第一个节点");
        assert_eq!(paused_nodes[1], second_node.uuid, "第二次暂停应该在第二个节点（即使没有断点）");

        // 验证暂停原因
        assert!(
            matches!(pause_reasons[0], PauseReason::Breakpoint { .. }),
            "第一次暂停应该是断点原因"
        );
        assert!(
            matches!(pause_reasons[1], PauseReason::StepComplete { .. }),
            "第二次暂停应该是单步完成原因，而不是断点"
        );
    }

    /// 测试跳过节点时堆栈条目唯一性
    ///
    /// Bug 场景：入口节点设置断点，点击"跳过"后，堆栈中应该只有一个入口节点条目
    #[tokio::test]
    async fn test_skip_node_single_stack_entry() {
        // 创建一个有两个节点的流程：Entry -> Entry
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true), // 第一个节点有断点
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: second_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试跳过节点堆栈".to_string(),
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话
        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let start_node_uuid = start_node.uuid.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![start_node_uuid],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 处理事件循环
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, .. } => {
                            // 如果是第一个节点（断点暂停），发送 SkipNode 命令
                            if node_uuid == start_node.uuid {
                                let _ = command_sender.send(DebugCommand::SkipNode {
                                    selected_port: Some("output".to_string()),
                                    simulated_output: Some(json!({"test": "value"})),
                                });
                            } else if node_uuid == second_node.uuid {
                                // 如果是第二个节点，发送继续命令
                                let _ = command_sender.send(DebugCommand::Continue);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("测试超时");
                }
            }
        }

        // 等待调试会话完成并获取最终状态
        let result = handle.await;
        assert!(result.is_ok());
        let final_state = result.unwrap();

        // 【关键断言】验证堆栈中只有一个 start_node 条目
        let start_node_entries: Vec<_> = final_state.execution_stack
            .iter()
            .filter(|entry| entry.node_uuid == start_node.uuid)
            .collect();

        assert_eq!(
            start_node_entries.len(),
            1,
            "堆栈中应该只有一个入口节点条目，但实际有 {} 个: {:?}",
            start_node_entries.len(),
            start_node_entries
        );
    }

    /// 测试正常执行入口节点时初始变量被注入
    ///
    /// 对比测试：正常执行入口节点时，初始变量应该被注入
    #[tokio::test]
    async fn test_execute_entry_node_with_initial_variables() {
        use flow_engine_core::node_config::EntryConfig;
        use flow_engine_core::variable::InitialVariable;

        // 创建一个有初始变量的入口节点
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(EntryConfig {
                initial_variables: vec![
                    InitialVariable {
                        name: "test_var".to_string(),
                        value: json!("initial_value"),
                        description: None,
                    },
                ],
            }),
            ports: None,
            has_breakpoint: Some(true), // 设置断点，以便暂停
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true), // 第二个节点也设置断点
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: second_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试执行入口节点".to_string(),
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话（两个节点都设置断点）
        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let start_node_uuid = start_node.uuid.clone();
        let second_node_uuid = second_node.uuid.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![start_node_uuid, second_node_uuid], // 两个节点都有断点
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 处理事件循环
        let mut paused_count = 0;
        let mut second_node_variables: Option<HashMap<String, serde_json::Value>> = None;
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, variables, .. } => {
                            paused_count += 1;
                            // 第一个节点暂停时，发送继续命令（正常执行）
                            if node_uuid == start_node.uuid {
                                let _ = command_sender.send(DebugCommand::Continue);
                            } else if node_uuid == second_node.uuid {
                                // 第二个节点暂停时，记录变量并发送继续命令
                                second_node_variables = Some(variables);
                                let _ = command_sender.send(DebugCommand::Continue);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("测试超时");
                }
            }
        }

        // 等待调试会话完成并获取最终状态
        let result = handle.await;
        assert!(result.is_ok());
        let final_state = result.unwrap();

        // 【关键断言1】验证暂停了两次（入口节点和第二个节点）
        assert_eq!(paused_count, 2, "应该暂停两次");

        // 【关键断言2】验证在第二个节点暂停时，test_var 变量已存在（因为正常执行了入口节点）
        if let Some(vars) = second_node_variables {
            assert!(
                vars.contains_key("test_var"),
                "在第二个节点暂停时，初始变量 'test_var' 应该已存在（入口节点已执行），但变量池为: {:?}",
                vars
            );
        }

        // 【关键断言3】验证最终状态中 test_var 变量存在
        assert!(
            final_state.variables.contains_key("test_var"),
            "正常执行入口节点后，初始变量 'test_var' 应该存在，但变量池为: {:?}",
            final_state.variables
        );

        // 【关键断言4】验证变量值正确
        assert_eq!(
            final_state.variables.get("test_var"),
            Some(&json!("initial_value")),
            "变量值应该是 'initial_value'"
        );
    }

    /// 测试跳过入口节点时初始变量不被注入
    ///
    /// Bug 场景：入口节点有 initial_variables，用户点击"跳过"后，初始变量不应该被注入到上下文中
    #[tokio::test]
    async fn test_skip_entry_node_no_initial_variables() {
        use flow_engine_core::node_config::EntryConfig;
        use flow_engine_core::variable::InitialVariable;

        // 创建一个有初始变量的入口节点
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(EntryConfig {
                initial_variables: vec![
                    InitialVariable {
                        name: "test_var".to_string(),
                        value: json!("initial_value"),
                        description: None,
                    },
                ],
            }),
            ports: None,
            has_breakpoint: Some(true), // 设置断点，以便暂停
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: second_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试跳过入口节点".to_string(),
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话
        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let start_node_uuid = start_node.uuid.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![start_node_uuid],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 处理事件循环
        let mut paused_variables: Option<HashMap<String, serde_json::Value>> = None;
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, variables, .. } => {
                            // 如果是第一个节点（断点暂停），记录变量并发送 SkipNode 命令
                            if node_uuid == start_node.uuid {
                                paused_variables = Some(variables);
                                // 【关键】不提供 simulated_output，模拟真实场景
                                let _ = command_sender.send(DebugCommand::SkipNode {
                                    selected_port: Some("output".to_string()),
                                    simulated_output: None, // 不提供模拟输出
                                });
                            } else if node_uuid == second_node.uuid {
                                // 如果是第二个节点，发送继续命令
                                let _ = command_sender.send(DebugCommand::Continue);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("测试超时");
                }
            }
        }

        // 等待调试会话完成并获取最终状态
        let result = handle.await;
        assert!(result.is_ok());
        let final_state = result.unwrap();

        // 【关键断言1】验证入口节点在暂停时变量为空（因为还没执行）
        if let Some(vars) = paused_variables {
            assert!(
                !vars.contains_key("test_var"),
                "入口节点暂停时，初始变量不应该已经被注入，但发现了: {:?}",
                vars
            );
        }

        // 【关键断言2】验证最终状态中 test_var 变量不存在（因为跳过了节点）
        assert!(
            !final_state.variables.contains_key("test_var"),
            "跳过入口节点后，初始变量 'test_var' 不应该存在，但发现了: {:?}",
            final_state.variables
        );

        // 【关键断言3】验证堆栈中入口节点被标记为"已跳过"
        let start_node_entry = final_state.execution_stack
            .iter()
            .find(|entry| entry.node_uuid == start_node.uuid);
        
        assert!(
            start_node_entry.is_some(),
            "堆栈中应该有入口节点条目"
        );
        
        let entry = start_node_entry.unwrap();
        assert_eq!(
            entry.error,
            Some("节点已跳过".to_string()),
            "入口节点应该被标记为'已跳过'，但实际错误信息为: {:?}",
            entry.error
        );
    }

    /// 测试跳过没有后续节点的入口节点
    ///
    /// Bug 场景：入口节点没有后续连接，用户点击"跳过"后，应该结束流程而不是执行入口节点
    #[tokio::test]
    async fn test_skip_entry_node_no_next_node() {
        use flow_engine_core::node_config::EntryConfig;
        use flow_engine_core::variable::InitialVariable;

        // 创建一个有初始变量但没有后续连接的入口节点
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(EntryConfig {
                initial_variables: vec![
                    InitialVariable {
                        name: "test_var".to_string(),
                        value: json!("initial_value"),
                        description: None,
                    },
                ],
            }),
            ports: None,
            has_breakpoint: Some(true), // 设置断点，以便暂停
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试跳过没有后续节点的入口节点".to_string(),
            nodes: vec![start_node.clone()],
            connections: vec![], // 没有连接
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话
        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let start_node_uuid = start_node.uuid.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![start_node_uuid],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 处理事件循环
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, .. } => {
                            // 入口节点暂停时，发送 SkipNode 命令
                            if node_uuid == start_node.uuid {
                                let _ = command_sender.send(DebugCommand::SkipNode {
                                    selected_port: Some("output".to_string()),
                                    simulated_output: None,
                                });
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("测试超时");
                }
            }
        }

        // 等待调试会话完成并获取最终状态
        let result = handle.await;
        assert!(result.is_ok());
        let final_state = result.unwrap();

        // 【关键断言1】验证最终状态中 test_var 变量不存在（因为跳过了节点，没有执行）
        assert!(
            !final_state.variables.contains_key("test_var"),
            "跳过入口节点后，初始变量 'test_var' 不应该存在，但发现了: {:?}",
            final_state.variables
        );

        // 【关键断言2】验证堆栈中只有一个入口节点条目
        assert_eq!(
            final_state.execution_stack.len(),
            1,
            "堆栈中应该只有一个入口节点条目，但实际有 {} 个",
            final_state.execution_stack.len()
        );

        // 【关键断言3】验证入口节点被标记为"已跳过"
        let entry = &final_state.execution_stack[0];
        assert_eq!(
            entry.error,
            Some("节点已跳过".to_string()),
            "入口节点应该被标记为'已跳过'，但实际错误信息为: {:?}",
            entry.error
        );
    }

    /// 测试动态设置断点
    ///
    /// 场景：调试启动时第二个节点没有断点，在第一个节点暂停时动态添加第二个节点的断点
    /// 预期：第二个节点应该因为新添加的断点而暂停
    #[tokio::test]
    async fn test_dynamic_breakpoint_add() {
        // 创建一个有三个节点的流程：Entry -> Entry -> Entry
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true), // 第一个节点有断点
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None, // 第二个节点初始没有断点
        };

        let third_node = Node {
            uuid: format!("third-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第三个节点".to_string(),
            position: NodePosition { x: 400.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        };

        let connection1 = Connection {
            id: format!("conn1-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: second_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let connection2 = Connection {
            id: format!("conn2-{}", flow_id),
            source_node_uuid: second_node.uuid.clone(),
            target_node_uuid: third_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试动态断点".to_string(),
            nodes: vec![start_node.clone(), second_node.clone(), third_node.clone()],
            connections: vec![connection1, connection2],
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话（只在第一个节点有断点）
        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let start_node_uuid = start_node.uuid.clone();
        let second_node_uuid = second_node.uuid.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![start_node_uuid], // 只在第一个节点有断点
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 收集事件
        let mut paused_nodes: Vec<String> = vec![];
        let mut breakpoint_modified_events: Vec<(String, bool)> = vec![];

        // 处理事件循环
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, .. } => {
                            paused_nodes.push(node_uuid.clone());

                            if node_uuid == start_node.uuid {
                                // 在第一个节点暂停时，动态添加第二个节点的断点
                                let _ = command_sender.send(DebugCommand::SetBreakpoint {
                                    node_uuid: second_node_uuid.clone(),
                                    enabled: true,
                                });
                                // 然后继续执行
                                let _ = command_sender.send(DebugCommand::Continue);
                            } else if node_uuid == second_node.uuid {
                                // 第二个节点暂停（因为动态添加的断点），继续执行
                                let _ = command_sender.send(DebugCommand::Continue);
                            }
                        }
                        DebugEvent::BreakpointModified { node_uuid, enabled } => {
                            breakpoint_modified_events.push((node_uuid, enabled));
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("测试超时");
                }
            }
        }

        // 等待调试会话完成
        let result = handle.await;
        assert!(result.is_ok());

        // 【关键断言1】应该暂停两次：第一个节点（初始断点）和第二个节点（动态断点）
        assert_eq!(paused_nodes.len(), 2, "应该暂停两次");
        assert_eq!(paused_nodes[0], start_node.uuid, "第一次暂停应该在第一个节点");
        assert_eq!(paused_nodes[1], second_node.uuid, "第二次暂停应该在第二个节点（动态断点）");

        // 【关键断言2】应该收到断点修改事件
        assert_eq!(breakpoint_modified_events.len(), 1, "应该收到一个断点修改事件");
        assert_eq!(breakpoint_modified_events[0].0, second_node.uuid);
        assert!(breakpoint_modified_events[0].1, "断点应该是启用状态");
    }

    /// 测试动态移除断点
    ///
    /// 场景：调试启动时两个节点都有断点，在第一个节点暂停时动态移除第二个节点的断点
    /// 预期：第二个节点应该不会暂停（直接执行完成）
    #[tokio::test]
    async fn test_dynamic_breakpoint_remove() {
        // 创建一个有两个节点的流程：Entry -> Entry
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true), // 初始有断点
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: second_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试动态移除断点".to_string(),
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        // 启动调试会话（两个节点都有断点）
        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let start_node_uuid = start_node.uuid.clone();
        let second_node_uuid_for_spawn = second_node.uuid.clone();
        let second_node_uuid_for_event = second_node.uuid.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![start_node_uuid.clone(), second_node_uuid_for_spawn.clone()], // 两个节点都有断点
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        // 收集暂停事件
        let mut paused_nodes: Vec<String> = vec![];

        // 处理事件循环
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, .. } => {
                            paused_nodes.push(node_uuid.clone());

                            if node_uuid == start_node.uuid {
                                // 在第一个节点暂停时，动态移除第二个节点的断点
                                let _ = command_sender.send(DebugCommand::SetBreakpoint {
                                    node_uuid: second_node_uuid_for_event.clone(),
                                    enabled: false,
                                });
                                // 然后继续执行
                                let _ = command_sender.send(DebugCommand::Continue);
                            } else if node_uuid == second_node.uuid {
                                // 如果第二个节点暂停，继续执行
                                let _ = command_sender.send(DebugCommand::Continue);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("测试超时");
                }
            }
        }

        // 等待调试会话完成
        let result = handle.await;
        assert!(result.is_ok());

        // 【关键断言】应该只暂停一次（第一个节点），第二个节点因为断点被移除而不暂停
        assert_eq!(paused_nodes.len(), 1, "应该只暂停一次（第二个节点断点被移除）");
        assert_eq!(paused_nodes[0], start_node.uuid, "暂停应该在第一个节点");
    }
}
