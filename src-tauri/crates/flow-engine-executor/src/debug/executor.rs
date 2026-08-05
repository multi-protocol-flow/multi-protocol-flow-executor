//! 可调试的流程执行器
//!
//! 支持断点、单步执行、变量查看等调试功能

use crate::debug::{
    DebugCommand, DebugEvent, DebugSessionState, DebugStatus, EndReason, ExecutionStackEntry,
    PauseReason,
};
use flow_engine_core::connection::Connection;
use flow_engine_core::context::{ExecutionContext, VariableStorage};
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::flow::Flow;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::{ExecutionErrorStrategy, Node};
use flow_engine_core::node_config::NodeData;
use flow_engine_core::node_types;
use flow_engine_core::registry::NodeRegistry;
use flow_engine_core::report::{ExecutionStatus, NodeExecutionReport};
use flow_engine_core::traits::LogSender;
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

    /// 设置节点插件注册表（默认包含全部内置节点）
    ///
    /// 与 [`crate::executor::FlowExecutor::with_registry`] 相同：注入共享注册表后，
    /// 调试执行器可执行注入的插件节点。
    pub fn with_registry(mut self, registry: Arc<NodeRegistry>) -> Self {
        self.registry = registry;
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
        debug_state.start_time_ms = flow_engine_core::utils::unix_timestamp_ms();

        // 创建共享状态（用于心跳任务读取当前状态）
        let shared_state = Arc::new(RwLock::new(debug_state.clone()));

        // 创建心跳任务停止信号
        let (stop_tx, stop_rx) = broadcast::channel::<()>(1);

        // 启动心跳任务
        let heartbeat_state = Arc::clone(&shared_state);
        let heartbeat_sender = event_sender.clone();
        let flow_uuid_for_heartbeat = flow.uuid.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(HEARTBEAT_INTERVAL_MS));
            let mut stop_rx = stop_rx;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // 读取当前状态并发送心跳
                        if let Ok(state) = heartbeat_state.read() {
                            let timestamp = flow_engine_core::utils::unix_timestamp_ms();

                            let _ = heartbeat_sender.send(DebugEvent::Heartbeat {
                                status: state.status.clone(),
                                current_node_uuid: state.current_node_uuid.clone(),
                                timestamp,
                            });
                        }
                    }
                    _ = stop_rx.recv() => {
                        log::info!("[Debug {}] Heartbeat task stopped", flow_uuid_for_heartbeat);
                        break;
                    }
                }
            }
        });

        // 发送会话启动事件
        if let Err(e) = event_sender.send(DebugEvent::SessionStarted {
            flow_uuid: flow.uuid.clone(),
        }) {
            log::error!(
                "[Debug {}] Failed to send SessionStarted event: {}",
                flow.uuid,
                e
            );
        }

        // 初始化执行上下文
        // 优先使用调用方传入的变量，否则从流程的激活环境中解析
        let initial_variables = variables
            .or_else(|| flow.get_active_variables().map(|v| json!(v)))
            .unwrap_or_else(|| json!({}));
        let mut var_storage = VariableStorage::new();
        if let Some(obj) = initial_variables.as_object() {
            for (key, value) in obj {
                var_storage.set_variable(key.clone(), value.clone(), String::new());
            }
        }

        use crate::provider_builder::ProviderBuilderExt;

        let mut context = ExecutionContext {
            flow_uuid: flow.uuid.clone(),
            current_node_uuid: None,
            variables: var_storage,
            node_reports: vec![],
            flow_trigger_time: flow_engine_core::utils::unix_timestamp_ms(),
            log_sender,
            flow_file_path: self.flow_file_path.clone(),
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            last_node_output: None,
            debug_event_sender: Some(Arc::new(
                crate::debug_event_adapter::MpscDebugEventSender::new(event_sender.clone()),
            )),
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
            current_node_plugin_data: None,
        }
        .with_default_providers();

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
                // 生成报告（即使是Error也要生成）
                let report =
                    Self::generate_debug_report(&debug_state, flow, debug_start_time_ms, &context)
                        .await;
                if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                    reason: EndReason::Error {
                        message: "Flow is missing a start node".to_string(),
                    },
                    report: Some(report),
                    execution_stack: debug_state.execution_stack.clone(),
                }) {
                    log::error!(
                        "[Debug {}] Failed to send SessionEnded event: {}",
                        flow.uuid,
                        e
                    );
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
        let result = self
            .execute_debug_loop(
                &mut debug_state,
                flow,
                start_node,
                &node_map,
                &connection_map,
                &mut context,
                event_sender,
                command_receiver,
                &shared_state,
            )
            .await;

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
                let report = Self::generate_debug_report(
                    debug_state,
                    flow,
                    debug_state.start_time_ms,
                    context,
                )
                .await;
                if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                    reason: EndReason::Error {
                        message: format!("Cycle detected: node {} already visited", node.uuid),
                    },
                    report: Some(report),
                    execution_stack: debug_state.execution_stack.clone(),
                }) {
                    log::error!(
                        "[Debug {}] Failed to send SessionEnded event: {}",
                        flow.uuid,
                        e
                    );
                }
                return debug_state.clone();
            }

            visited.insert(current_node_id.clone());
            context.current_node_uuid = Some(current_node_id.clone());
            debug_state.current_node_uuid = Some(current_node_id.clone());
            // 同步到共享状态，使心跳任务能读取到当前执行节点
            update_shared_state(debug_state);

            // 检查是否需要暂停（断点或单步模式）
            // 当 skip_breakpoints 为 true 时，不暂停（执行到底模式）
            let should_pause = !debug_state.skip_breakpoints
                && (debug_state.has_breakpoint(&current_node_id) || debug_state.is_stepping);

            // 非暂停模式（RunToEnd）：立即发送心跳，让前端知道当前执行的节点
            if !should_pause {
                let now = flow_engine_core::utils::unix_timestamp_ms();
                let _ = event_sender.send(DebugEvent::Heartbeat {
                    status: DebugStatus::Running,
                    current_node_uuid: Some(current_node_id.clone()),
                    timestamp: now,
                });
            }

            if should_pause {
                // 确定暂停原因：单步模式优先于断点
                let pause_reason = if debug_state.is_stepping && !debug_state.skip_breakpoints {
                    // 单步execution complete，在此节点暂停
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
                    log::error!(
                        "[Debug {}] Failed to send PausedAtNode event (node {}): {}",
                        context.flow_uuid,
                        current_node_id,
                        e
                    );
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
                        let node_end_ms = flow_engine_core::utils::unix_timestamp_ms();

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
                            error: Some("Node skipped".to_string()),
                            used_port_id: Some("skipped".to_string()),
                            resolved_config,
                            variable_changes: None,
                            skip_reason: Some("User skipped".to_string()),
                        };
                        let skip_entry = stack_entry.clone();
                        debug_state.push_stack_entry(stack_entry);

                        // 发送节点execution complete事件（包含完整的执行信息）
                        if let Err(e) =
                            event_sender.send(DebugEvent::NodeExecuted { entry: skip_entry })
                        {
                            log::error!("[Debug {}] Failed to send NodeExecuted event (skipped node {}): {}", context.flow_uuid, node.uuid, e);
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
                            let report = Self::generate_debug_report(
                                debug_state,
                                flow,
                                debug_state.start_time_ms,
                                context,
                            )
                            .await;
                            if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                                reason: EndReason::Completed,
                                report: Some(report),
                                execution_stack: debug_state.execution_stack.clone(),
                            }) {
                                log::error!(
                                    "[Debug {}] Failed to send SessionEnded event: {}",
                                    flow.uuid,
                                    e
                                );
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
                        // 用户终止或Error
                        if error == "User aborted debugging" {
                            debug_state.status = DebugStatus::Ended;
                            let report = Self::generate_debug_report(
                                debug_state,
                                flow,
                                debug_state.start_time_ms,
                                context,
                            )
                            .await;
                            if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                                reason: EndReason::Error { message: error },
                                report: Some(report),
                                execution_stack: debug_state.execution_stack.clone(),
                            }) {
                                log::error!(
                                    "[Debug {}] Failed to send SessionEnded event: {}",
                                    flow.uuid,
                                    e
                                );
                            }
                            return debug_state.clone();
                        }
                        debug_state.status = DebugStatus::Ended;
                        update_shared_state(debug_state);
                        let report = Self::generate_debug_report(
                            debug_state,
                            flow,
                            debug_state.start_time_ms,
                            context,
                        )
                        .await;
                        if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                            reason: EndReason::Error { message: error },
                            report: Some(report),
                            execution_stack: debug_state.execution_stack.clone(),
                        }) {
                            log::error!(
                                "[Debug {}] Failed to send SessionEnded event: {}",
                                flow.uuid,
                                e
                            );
                        }
                        return debug_state.clone();
                    }
                }
            }

            // 执行当前节点
            let node_start_ms = flow_engine_core::utils::unix_timestamp_ms();

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
            let effective_node = if let Some(temp_config) =
                debug_state.get_temp_node_config(&node.uuid)
            {
                // 尝试反序列化临时配置
                match serde_json::from_value::<NodeData>(temp_config.clone()) {
                    Ok(new_data) => {
                        log::info!(
                            "[Debug {}] Applying temporary config to node: {} ({})",
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
                            "[Debug {}] Temp config deserialization failed, using original config: {}",
                            context.flow_uuid,
                            e
                        );
                        (*node).clone()
                    }
                }
            } else {
                (*node).clone()
            };

            // 同步 execution_stack 到 context.node_reports（供 VariableExtractor 等节点查找上游输出）
            context.node_reports = debug_state
                .execution_stack
                .iter()
                .filter(|entry| entry.ended_at.is_some())
                .map(|entry| {
                    let status = if entry.success {
                        ExecutionStatus::Success
                    } else {
                        ExecutionStatus::Failed
                    };
                    let error_detail = entry.error.as_ref().map(|e| {
                        flow_engine_core::error::ErrorDetail::new(
                            flow_engine_core::error::ErrorKind::Execution,
                            e.clone(),
                        )
                    });
                    NodeExecutionReport {
                        node_uuid: entry.node_uuid.clone(),
                        node_type: entry.node_type.clone(),
                        node_name: entry.node_name.clone(),
                        start_time: entry.started_at,
                        end_time: entry.ended_at.unwrap_or(entry.started_at),
                        duration_ms: entry.ended_at.map(|e| e - entry.started_at).unwrap_or(0),
                        status,
                        input_data: entry.input.clone(),
                        output_data: entry.output.clone(),
                        used_port_id: entry.used_port_id.clone(),
                        error: error_detail,
                        resolved_config: entry.resolved_config.clone(),
                        variable_changes: entry.variable_changes.clone(),
                        skip_reason: entry.skip_reason.clone(),
                        wait_duration_ms: None,
                        plugin_data: None,
                    }
                })
                .collect();

            // 快照执行前的变量状态，用于计算节点执行期间的变量变更
            let variables_before = context.variables.get_merged_variables();

            // 执行节点
            match self
                .execute_node(&effective_node, context, &flow.connections, node_map)
                .await
            {
                Ok((next_node_ids, output_data)) => {
                    // 计算变量变更
                    let variables_after = context.variables.get_merged_variables();
                    let variable_changes =
                        compute_variable_changes(&variables_before, &variables_after);

                    // 更新堆栈条目
                    let node_end_ms = flow_engine_core::utils::unix_timestamp_ms();

                    // 根据节点类型和输出数据确定 used_port_id（通过 registry 查找）
                    let type_id = node.data.type_name();
                    let used_port_id = self
                        .registry
                        .get(&type_id)
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
                        if variable_changes.is_empty() {
                            None
                        } else {
                            Some(variable_changes.clone())
                        },
                        None, // skip_reason 仅跳过节点时设置
                    );

                    // 发送节点execution complete事件（包含完整的执行信息）
                    let completed_entry = debug_state
                        .execution_stack
                        .last()
                        .cloned()
                        .unwrap_or_else(|| unreachable!("stack entry just pushed"));
                    if let Err(e) = event_sender.send(DebugEvent::NodeExecuted {
                        entry: completed_entry,
                    }) {
                        log::error!(
                            "[Debug {}] Failed to send NodeExecuted event (node {}): {}",
                            context.flow_uuid,
                            node.uuid,
                            e
                        );
                    }

                    // 更新 last_node_output（所有节点均更新，供下游节点通过 jsonpath 访问）
                    if let Some(ref data) = output_data {
                        context.last_node_output = Some(data.clone());
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
                        // execution complete
                        break;
                    }
                }
                Err(error) => {
                    // 计算变量变更（节点可能部分修改了变量才出错）
                    let variables_after = context.variables.get_merged_variables();
                    let variable_changes =
                        compute_variable_changes(&variables_before, &variables_after);

                    // 更新堆栈条目
                    let node_end_ms = flow_engine_core::utils::unix_timestamp_ms();

                    // 根据节点 on_error 策略决定执行行为
                    let route_to_false = node
                        .on_error
                        .as_ref()
                        .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse);
                    let is_ignore = node
                        .on_error
                        .as_ref()
                        .is_some_and(|s| *s == ExecutionErrorStrategy::Ignore);

                    let false_port_next_node = if route_to_false {
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
                        Some(error.message.clone()),
                        used_port_id,
                        effective_resolved_config,
                        if variable_changes.is_empty() {
                            None
                        } else {
                            Some(variable_changes.clone())
                        },
                        None, // skip_reason
                    );

                    // 发送节点execution complete事件（包含完整的执行信息）
                    let failed_entry = debug_state
                        .execution_stack
                        .last()
                        .cloned()
                        .unwrap_or_else(|| unreachable!("stack entry just pushed"));
                    if let Err(e) = event_sender.send(DebugEvent::NodeExecuted {
                        entry: failed_entry,
                    }) {
                        log::error!(
                            "[Debug {}] Failed to send NodeExecuted event (node {} failed): {}",
                            context.flow_uuid,
                            node.uuid,
                            e
                        );
                    }

                    // RouteToFalse（默认）: 如果有 false 端口连接，继续执行 false 分支
                    if route_to_false {
                        if let Some(next_node_uuid) = false_port_next_node {
                            log::info!(
                                "[Debug {}] Node {} execution failed, but false port connection exists, continuing to node {}",
                                context.flow_uuid,
                                node.name,
                                next_node_uuid
                            );

                            current_node_id = next_node_uuid;
                            continue;
                        }
                    }

                    // Ignore 策略: 忽略Error，继续 true/out 端口执行（如同Success）
                    if is_ignore {
                        let next_node_id = flow
                            .connections
                            .iter()
                            .find(|conn| {
                                conn.source_node_uuid == node.uuid
                                    && (conn.source_port_id == "true"
                                        || conn.source_port_id == "out")
                            })
                            .map(|conn| conn.target_node_uuid.clone());

                        if let Some(next_uuid) = next_node_id {
                            log::info!(
                                "[调试 {}] Node {} execution failed, on_error=Ignore, continuing to node {}",
                                context.flow_uuid,
                                node.name,
                                next_uuid
                            );
                            current_node_id = next_uuid;
                            continue;
                        }
                        // 没有后续连接，正常终止调试会话
                        break;
                    }

                    // 没有 false 端口连接或 on_error=AbortFlow，结束调试
                    // 发送Error事件
                    if let Err(e) = event_sender.send(DebugEvent::Error {
                        message: error.message.clone(),
                    }) {
                        log::error!(
                            "[Debug {}] Failed to send Error event: {}",
                            context.flow_uuid,
                            e
                        );
                    }

                    // 结束调试
                    debug_state.status = DebugStatus::Ended;
                    update_shared_state(debug_state);
                    let report = Self::generate_debug_report(
                        debug_state,
                        flow,
                        debug_state.start_time_ms,
                        context,
                    )
                    .await;
                    if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                        reason: EndReason::Error {
                            message: error.message,
                        },
                        report: Some(report),
                        execution_stack: debug_state.execution_stack.clone(),
                    }) {
                        log::error!(
                            "[Debug {}] Failed to send SessionEnded event: {}",
                            flow.uuid,
                            e
                        );
                    }
                    return debug_state.clone();
                }
            }
        }

        // 正常完成
        debug_state.status = DebugStatus::Ended;
        debug_state.current_node_uuid = None;
        update_shared_state(debug_state);
        let report =
            Self::generate_debug_report(debug_state, flow, debug_state.start_time_ms, context)
                .await;
        if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
            reason: EndReason::Completed,
            report: Some(report),
            execution_stack: debug_state.execution_stack.clone(),
        }) {
            log::error!(
                "[Debug {}] Failed to send SessionEnded event: {}",
                flow.uuid,
                e
            );
        }

        debug_state.clone()
    }

    /// 等待调试命令
    ///
    /// 返回值：
    /// - `Ok(Some((Some(next_uuid), output)))`：跳过节点，跳转到下一个节点
    /// - `Ok(Some((None, output)))`：跳过节点，没有下一个节点，结束流程
    /// - `Ok(None)`：继续执行当前节点
    /// - `Err(msg)`：Error或用户终止
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
                    let report = Self::generate_debug_report(
                        debug_state,
                        flow,
                        debug_state.start_time_ms,
                        context,
                    )
                    .await;
                    if let Err(e) = event_sender.send(DebugEvent::SessionEnded {
                        reason: EndReason::UserAbort,
                        report: Some(report),
                        execution_stack: debug_state.execution_stack.clone(),
                    }) {
                        log::error!("[Debug] Failed to send SessionEnded event: {}", e);
                    }
                    return Err("User aborted debugging".to_string());
                }
                DebugCommand::ModifyVariable { name, value } => {
                    // 修改变量
                    context
                        .variables
                        .set_variable(name.clone(), value.clone(), String::new());
                    debug_state.set_variable(name.clone(), value.clone());

                    // 发送变量修改事件
                    if let Err(e) = event_sender.send(DebugEvent::VariableModified { name, value })
                    {
                        log::error!("[Debug] Failed to send VariableModified event: {}", e);
                    }
                }
                DebugCommand::ModifyNodeConfig { node_uuid, config } => {
                    // 修改节点配置（临时）
                    debug_state.set_temp_node_config(node_uuid.clone(), config.clone());

                    // 发送节点配置修改事件
                    if let Err(e) =
                        event_sender.send(DebugEvent::NodeConfigModified { node_uuid, config })
                    {
                        log::error!("[Debug] Failed to send NodeConfigModified event: {}", e);
                    }
                }
                DebugCommand::SetBreakpoint { node_uuid, enabled } => {
                    // 动态设置断点
                    if enabled {
                        debug_state.add_breakpoint(node_uuid.clone());
                        log::info!("[Debug] Added breakpoint: {}", node_uuid);
                    } else {
                        debug_state.remove_breakpoint(&node_uuid);
                        log::info!("[Debug] Removed breakpoint: {}", node_uuid);
                    }

                    // 发送断点修改事件
                    if let Err(e) =
                        event_sender.send(DebugEvent::BreakpointModified { node_uuid, enabled })
                    {
                        log::error!("[Debug] Failed to send BreakpointModified event: {}", e);
                    }
                }
                DebugCommand::StartDebug { .. } => {
                    // 调试已启动，忽略此命令
                    log::warn!("Debug session already started, ignoring StartDebug command");
                }
            }
        }

        // 命令通道关闭
        Err("Debug command channel closed".to_string())
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
            "[Debug {}] Handling skip node: {} ({}), selected_port: {:?}",
            context.flow_uuid,
            current_node.name,
            current_node.uuid,
            selected_port
        );

        // 如果有模拟输出，注入到变量池
        if let Some(ref output) = simulated_output {
            if let Some(obj) = output.as_object() {
                for (key, value) in obj {
                    context.variables.set_variable(
                        key.clone(),
                        value.clone(),
                        current_node.uuid.clone(),
                    );
                    debug_state.set_variable(key.clone(), value.clone());
                }
            }
        }

        // 根据 selected_port 查找下一个节点
        let next_node_uuid = if let Some(port) = selected_port {
            // 根据指定的输出端口查找连接
            log::info!(
                "[Debug {}] Looking up connection: source_node_uuid={}, source_port_id={}",
                context.flow_uuid,
                current_node.uuid,
                port
            );

            // 打印所有连接用于调试
            for conn in connections {
                log::info!(
                    "[Debug {}] Connection: {} -> {} (port: {} -> {})",
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
                        "[Debug {}] Found matching connection, target node: {}",
                        context.flow_uuid,
                        conn.target_node_uuid
                    );
                    conn.target_node_uuid.clone()
                });

            if found.is_none() {
                log::warn!(
                    "[Debug {}] No matching connection found: source_node_uuid={}, source_port_id={}",
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
            "[Debug {}] Skip node result: next_node_uuid={:?}",
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
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        // 发送节点开始日志
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Executing node {} ({})", node.name, node.node_type),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        let type_id = node.data.type_name();
        match self.registry.get(&type_id) {
            Some(plugin) => {
                log::info!(
                    "[调试 {}] Executing {} node: {} ({})",
                    context.flow_uuid,
                    plugin.display_name(),
                    node.name,
                    node.uuid
                );
                // 调试模式不应用超时，用户可能在断点处暂停、修改变量、查看数据
                plugin.execute(node, context, connections, node_map).await
            }
            None => Err(ErrorDetail::execution_error(format!(
                "Unregistered node type: {}",
                type_id
            ))),
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

        let end_time_ms = flow_engine_core::utils::unix_timestamp_ms();

        // 将临时节点配置合并到 flow 副本中，使 flow_snapshot 反映运行时实际配置
        let mut flow_snapshot = flow.clone();
        if !debug_state.temp_node_configs.is_empty() {
            for node in &mut flow_snapshot.nodes {
                if let Some(temp_config) = debug_state.get_temp_node_config(&node.uuid) {
                    match serde_json::from_value::<flow_engine_core::node_config::NodeData>(
                        temp_config.clone(),
                    ) {
                        Ok(new_data) => {
                            log::info!(
                                "[Debug] flow_snapshot: Merging temp config into node {} ({})",
                                node.name,
                                node.uuid
                            );
                            node.data = new_data;
                        }
                        Err(e) => {
                            log::warn!(
                                "[Debug] flow_snapshot: Temp config deserialization failed, skipping node {} ({})",
                                node.name,
                                e
                            );
                        }
                    }
                }
            }
        }

        // 将 execution_stack 转换为 NodeExecutionReport
        let node_reports: Vec<NodeExecutionReport> = debug_state
            .execution_stack
            .iter()
            .map(|entry| {
                // 判断节点状态：如果 error 包含"跳过"则为 Skipped
                let status = if entry
                    .error
                    .as_ref()
                    .map(|e| e.contains("Node skipped"))
                    .unwrap_or(false)
                {
                    ExecutionStatus::Skipped
                } else if entry.success {
                    ExecutionStatus::Success
                } else {
                    ExecutionStatus::Failed
                };

                // 使用 entry 中的 used_port_id
                let used_port_id = entry.used_port_id.clone();

                // 处理Error信息
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
                    plugin_data: None,
                }
            })
            .collect();

        // 计算整体状态
        let flow_status = if node_reports
            .iter()
            .any(|r| r.status == ExecutionStatus::Failed)
        {
            ExecutionStatus::Failed
        } else if node_reports
            .iter()
            .all(|r| r.status == ExecutionStatus::Skipped)
        {
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
            flow_snapshot: Some(flow_snapshot),
            end_node_reached: None,
            flow_warning: None,
            plugin_streams: vec![],
        }
    }
}

/// 计算变量变更列表
///
/// 对比变量快照前后的差异，生成 `VariableChange` 列表
fn compute_variable_changes(
    variables_before: &std::collections::HashMap<String, serde_json::Value>,
    variables_after: &std::collections::HashMap<String, serde_json::Value>,
) -> Vec<flow_engine_core::report::VariableChange> {
    let mut changes = Vec::new();

    // 检测新增和修改的变量
    for (key, new_value) in variables_after {
        let old_value = variables_before.get(key).cloned();
        // 只有值真正变化时才记录
        if old_value.as_ref() != Some(new_value) {
            changes.push(flow_engine_core::report::VariableChange {
                key: key.clone(),
                old_value,
                new_value: new_value.clone(),
            });
        }
    }

    changes
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
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            hooks: None,
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
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![start_node.clone()],
            connections: vec![],
            hooks: None,
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
        if let Some(DebugEvent::SessionStarted { flow_uuid }) = event_receiver.recv().await {
            assert_eq!(flow_uuid, flow_id);
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
            on_error: None,
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None, // 第二个节点没有断点！
            on_error: None,
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
            environments: vec![],
            active_environment: None,
            name: "测试单步执行".to_string(),
            description: None,
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
            hooks: None,
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
        assert_eq!(
            paused_nodes[0], start_node.uuid,
            "第一次暂停应该在第一个节点"
        );
        assert_eq!(
            paused_nodes[1], second_node.uuid,
            "第二次暂停应该在第二个节点（即使没有断点）"
        );

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
            on_error: None,
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
            environments: vec![],
            active_environment: None,
            name: "测试跳过节点堆栈".to_string(),
            description: None,
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
            hooks: None,
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
        let final_state = result.unwrap_or_else(|e| panic!("{e}"));

        // 【关键断言】验证堆栈中只有一个 start_node 条目
        let start_node_entries: Vec<_> = final_state
            .execution_stack
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

    /// Test Flow级默认变量被正确注入
    ///
    /// 验证流程级 `initial_variables` 在执行开始时已被注入到上下文中
    #[tokio::test]
    async fn test_execute_entry_node_with_initial_variables() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
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
            initial_variables: Some({
                let mut m = std::collections::HashMap::new();
                m.insert("test_var".to_string(), json!("initial_value"));
                m
            }),
            environments: vec![],
            active_environment: None,
            name: "测试执行入口节点".to_string(),
            description: None,
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
            hooks: None,
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
                    vec![start_node_uuid, second_node_uuid],
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
        let final_state = result.unwrap_or_else(|e| panic!("{e}"));

        // 【关键断言1】验证暂停了两次（入口节点和第二个节点）
        assert_eq!(paused_count, 2, "应该暂停两次");

        // 【关键断言2】验证在第二个节点暂停时，test_var 变量已存在（流程级变量在执行开始时注入）
        if let Some(vars) = second_node_variables {
            assert!(
                vars.contains_key("test_var"),
                "在第二个节点暂停时，流程级默认变量 'test_var' 应该已存在，但变量池为: {:?}",
                vars
            );
        }

        // 【关键断言3】验证最终状态中 test_var 变量存在
        assert!(
            final_state.variables.contains_key("test_var"),
            "流程级默认变量 'test_var' 应该存在，但变量池为: {:?}",
            final_state.variables
        );

        // 【关键断言4】验证变量值正确
        assert_eq!(
            final_state.variables.get("test_var"),
            Some(&json!("initial_value")),
            "变量值应该是 'initial_value'"
        );
    }

    /// 测试跳过入口节点时流程级默认变量仍然存在
    ///
    /// 流程级 `initial_variables` 在执行开始时已注入上下文，即使跳过 Entry node也依然存在
    #[tokio::test]
    async fn test_skip_entry_node_no_initial_variables() {
        let flow_id = Uuid::new_v4().to_string();

        // 创建入口节点（流程级变量通过 Flow.initial_variables 设置）
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
            initial_variables: Some({
                let mut m = std::collections::HashMap::new();
                m.insert("test_var".to_string(), json!("initial_value"));
                m
            }),
            environments: vec![],
            active_environment: None,
            name: "测试跳过入口节点".to_string(),
            description: None,
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
            hooks: None,
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
                            if node_uuid == start_node.uuid {
                                paused_variables = Some(variables);
                                // 发送 SkipNode 命令
                                let _ = command_sender.send(DebugCommand::SkipNode {
                                    selected_port: Some("output".to_string()),
                                    simulated_output: None,
                                });
                            } else if node_uuid == second_node.uuid {
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
        let final_state = result.unwrap_or_else(|e| panic!("{e}"));

        // 【关键断言1】验证入口节点在暂停时 test_var 已经存在（流程级变量在执行开始时注入）
        if let Some(vars) = paused_variables {
            assert!(
                vars.contains_key("test_var"),
                "入口节点暂停时，流程级默认变量 'test_var' 应该已存在，但变量池为: {:?}",
                vars
            );
        }

        // 【关键断言2】验证最终状态中 test_var 变量仍然存在（流程级变量不依赖 Entry node执行）
        assert!(
            final_state.variables.contains_key("test_var"),
            "流程级默认变量 'test_var' 应该始终存在，但变量池为: {:?}",
            final_state.variables
        );

        // 【关键断言3】验证堆栈中入口节点被标记为"已跳过"
        let start_node_entry = final_state
            .execution_stack
            .iter()
            .find(|entry| entry.node_uuid == start_node.uuid);

        assert!(start_node_entry.is_some(), "堆栈中应该有入口节点条目");

        let entry = start_node_entry.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(
            entry.error,
            Some("Node skipped".to_string()),
            "Entry node should have been marked as 'skipped', but actual error: {:?}",
            entry.error
        );
    }

    /// 测试跳过没有后续节点的入口节点
    ///
    /// Bug 场景：入口节点没有后续连接，用户点击"跳过"后，应该结束流程而不是执行入口节点
    #[tokio::test]
    async fn test_skip_entry_node_no_next_node() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试跳过没有后续节点的入口节点".to_string(),
            description: None,
            nodes: vec![start_node.clone()],
            connections: vec![],
            environments: vec![],
            active_environment: None,
            hooks: None,
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
        let final_state = result.unwrap_or_else(|e| panic!("{e}"));

        // 【关键断言1】验证最终状态中没有来自 Entry 的额外变量（flow.initial_variables 为 None）
        // 注意：流程级 initial_variables 不依赖 Entry node执行，但本例中未设置任何 flow.initial_variables
        // 跳过入口节点不应产生任何副作用

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
            Some("Node skipped".to_string()),
            "Entry node should have been marked as 'skipped', but actual error: {:?}",
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
            on_error: None,
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None, // 第二个节点初始没有断点
            on_error: None,
        };

        let third_node = Node {
            uuid: format!("third-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第三个节点".to_string(),
            position: NodePosition { x: 400.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
            environments: vec![],
            active_environment: None,
            name: "测试动态断点".to_string(),
            description: None,
            nodes: vec![start_node.clone(), second_node.clone(), third_node.clone()],
            connections: vec![connection1, connection2],
            hooks: None,
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
        assert_eq!(
            paused_nodes[0], start_node.uuid,
            "第一次暂停应该在第一个节点"
        );
        assert_eq!(
            paused_nodes[1], second_node.uuid,
            "第二次暂停应该在第二个节点（动态断点）"
        );

        // 【关键断言2】应该收到断点修改事件
        assert_eq!(
            breakpoint_modified_events.len(),
            1,
            "应该收到一个断点修改事件"
        );
        assert_eq!(breakpoint_modified_events[0].0, second_node.uuid);
        assert!(breakpoint_modified_events[0].1, "断点应该是启用状态");
    }

    /// 测试动态移除断点
    ///
    /// 场景：调试启动时两个节点都有断点，在第一个节点暂停时动态移除第二个节点的断点
    /// 预期：第二个节点应该不会暂停（直接execution complete）
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
            on_error: None,
        };

        let second_node = Node {
            uuid: format!("second-{}", flow_id),
            node_type: "entry".to_string(),
            name: "第二个节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true), // 初始有断点
            on_error: None,
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
            environments: vec![],
            active_environment: None,
            name: "测试动态移除断点".to_string(),
            description: None,
            nodes: vec![start_node.clone(), second_node.clone()],
            connections: vec![connection],
            hooks: None,
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
        assert_eq!(
            paused_nodes.len(),
            1,
            "应该只暂停一次（第二个节点断点被移除）"
        );
        assert_eq!(paused_nodes[0], start_node.uuid, "暂停应该在第一个节点");
    }
}
