//! 可调试的流程执行器
//!
//! 支持断点、单步执行、变量查看等调试功能

use crate::debug::{
    DebugCallFrame, DebugCommand, DebugEvent, DebugSessionState, DebugStatus, EndReason,
    ExecutionStackEntry, PauseReason,
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

/// 通过执行前后快照差异补充变量变更记录（新增、修改与删除）
///
/// 仅在节点未产生 pending 变更（版本号未变）时使用：部分执行器直接改写合并视图中的
/// 变量而不走版本追踪，此时按快照差异兜底，避免时间线漏记。
/// 删除的变量以 `new_value = Null` 表达，与 pending 变更路径的表示保持一致。
fn diff_variable_changes(
    before: &HashMap<String, serde_json::Value>,
    after: &HashMap<String, serde_json::Value>,
    changes: &mut Vec<flow_engine_core::report::VariableChange>,
) {
    for (k, new_val) in after {
        match before.get(k) {
            Some(old_val) if old_val != new_val => {
                changes.push(flow_engine_core::report::VariableChange {
                    key: k.clone(),
                    old_value: Some(old_val.clone()),
                    new_value: new_val.clone(),
                });
            }
            None => changes.push(flow_engine_core::report::VariableChange {
                key: k.clone(),
                old_value: None,
                new_value: new_val.clone(),
            }),
            Some(_) => {}
        }
    }

    for (k, old_val) in before {
        if !after.contains_key(k) {
            changes.push(flow_engine_core::report::VariableChange {
                key: k.clone(),
                old_value: Some(old_val.clone()),
                new_value: serde_json::Value::Null,
            });
        }
    }
}

/// 调试单步动作类型
#[derive(Debug, Clone, PartialEq)]
enum StepCommandResult {
    Continue,
    StepOver,
    StepInto,
    StepOut,
    RunToEnd,
    SkipNode(Option<String>, Option<serde_json::Value>),
    RetryCurrentNode,
    RunFromSnapshot(String),
}

/// 上下文快照，用于断点重试与历史快照恢复
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ContextSnapshot {
    /// 目标节点 UUID
    pub node_uuid: String,
    /// 变量存储快照
    pub variables: flow_engine_core::context::VariableStorage,
    /// 堆栈长度快照
    pub stack_len: usize,
    /// 已访问节点集合
    pub visited: std::collections::HashSet<String>,
    /// 创建时间戳（毫秒）
    pub timestamp_ms: u64,
}

/// 调试调用栈帧状态（用于多层子流程嵌套调试）
struct DebugFrameState {
    frame_id: usize,
    flow: Flow,
    node_map: HashMap<String, Node>,
    connection_map: HashMap<String, Vec<String>>,
    current_node_id: String,
    parent_subflow_node_uuid: Option<String>,
    context: ExecutionContext,
    subflow_config: Option<flow_engine_core::node_config::SubFlowConfig>,
    visited: std::collections::HashSet<String>,
    subflow_entry_index: Option<usize>,
    snapshots: HashMap<String, ContextSnapshot>,
}

fn build_call_frames(call_frames: &[DebugFrameState]) -> Vec<DebugCallFrame> {
    call_frames
        .iter()
        .map(|frame| DebugCallFrame {
            frame_id: frame.frame_id,
            flow_uuid: frame.flow.uuid.clone(),
            flow_name: if !frame.flow.name.trim().is_empty() {
                frame.flow.name.clone()
            } else {
                "SubFlow".to_string()
            },
            node_uuid: frame.current_node_id.clone(),
            node_name: frame
                .node_map
                .get(&frame.current_node_id)
                .map(|n| n.name.clone())
                .unwrap_or_default(),
            parent_subflow_node_uuid: frame.parent_subflow_node_uuid.clone(),
        })
        .collect()
}

/// 可调试的流程执行器
pub struct DebuggableFlowExecutor {
    /// 流程文件路径（用于 CLI 相对路径解析）
    flow_file_path: Option<PathBuf>,
    /// 节点插件注册表（通过 registry 查找替代 match 分发）
    registry: Arc<NodeRegistry>,
    /// 附加的自定义 Provider（例如 Ingress Mock 服务的响应回传通道）
    extra_providers: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
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
            extra_providers: HashMap::new(),
        }
    }

    /// 注入自定义 Provider
    ///
    /// 与 [`crate::executor::FlowExecutor::with_provider`] 语义一致：注入的 provider
    /// 会写入调试根上下文，并随调用栈下传到子流程帧，供节点通过
    /// `context.get_provider::<T>(name)` 取用（例如 Ingress Mock 的响应回传通道）。
    pub fn with_provider<T: 'static + Send + Sync>(mut self, name: &str, provider: Arc<T>) -> Self {
        self.extra_providers.insert(name.to_string(), provider);
        self
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
            // 每次执行生成独立 execution_id（插件 per-execution 资源隔离的 key）
            execution_id: uuid::Uuid::new_v4().to_string(),
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
            call_stack: Vec::new(),
            current_sub_flow_report: None,
        }
        .with_default_providers();

        // 注入调用方提供的自定义 provider（如 Ingress Mock 响应回传通道）
        for (name, provider) in &self.extra_providers {
            context.set_provider(name.clone(), provider.clone());
        }

        // 记录调试开始时间（用于报告生成）
        let debug_start_time_ms = context.flow_trigger_time;

        // 查找开始节点
        // 查找开始节点：与 FlowExecutor/校验器共用同一判定（ENTRY 优先，其次 HTTP_INGRESS）
        let start_node = flow
            .nodes
            .iter()
            .find(|node| node.node_type == node_types::ENTRY)
            .or_else(|| {
                flow.nodes
                    .iter()
                    .find(|node| node_types::is_entry_node_type(&node.node_type))
            });
        let start_node = match start_node {
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

        // 执行调试主循环
        let result = self
            .execute_debug_loop(
                &mut debug_state,
                flow,
                start_node,
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
        root_flow: &Flow,
        start_node: &Node,
        context: &mut ExecutionContext,
        event_sender: mpsc::UnboundedSender<DebugEvent>,
        mut command_receiver: mpsc::UnboundedReceiver<DebugCommand>,
        shared_state: &Arc<RwLock<DebugSessionState>>,
    ) -> DebugSessionState {
        let update_shared_state = |state: &DebugSessionState| {
            if let Ok(mut shared) = shared_state.write() {
                *shared = state.clone();
            }
        };

        let mut root_node_map: HashMap<String, Node> = HashMap::new();
        for node in &root_flow.nodes {
            root_node_map.insert(node.uuid.clone(), node.clone());
        }
        let mut root_connection_map: HashMap<String, Vec<String>> = HashMap::new();
        for conn in &root_flow.connections {
            root_connection_map
                .entry(conn.source_node_uuid.clone())
                .or_default()
                .push(conn.target_node_uuid.clone());
        }

        let mut root_snapshots = HashMap::new();
        root_snapshots.insert(
            start_node.uuid.clone(),
            ContextSnapshot {
                node_uuid: start_node.uuid.clone(),
                variables: context.variables.clone(),
                stack_len: 0,
                visited: std::collections::HashSet::new(),
                timestamp_ms: flow_engine_core::utils::unix_timestamp_ms(),
            },
        );

        let root_frame = DebugFrameState {
            frame_id: 0,
            flow: root_flow.clone(),
            node_map: root_node_map,
            connection_map: root_connection_map,
            current_node_id: start_node.uuid.clone(),
            parent_subflow_node_uuid: None,
            context: context.clone(),
            subflow_config: None,
            visited: std::collections::HashSet::new(),
            subflow_entry_index: None,
            snapshots: root_snapshots,
        };

        let mut call_frames: Vec<DebugFrameState> = vec![root_frame];
        let mut step_out_active = false;
        let mut step_out_target_depth = 0;

        while !call_frames.is_empty() {
            let active_idx = call_frames.len() - 1;
            let current_node_id = call_frames[active_idx].current_node_id.clone();

            // 如果当前节点 ID 为空，或者在 node_map 中找不到，说明该流程执行到底了
            let Some(node) = call_frames[active_idx]
                .node_map
                .get(&current_node_id)
                .cloned()
            else {
                if call_frames.len() > 1 {
                    // 子流程执行完毕，弹出并回到父流程
                    let child_frame = match call_frames.pop() {
                        Some(f) => f,
                        None => break,
                    };
                    let parent_frame = match call_frames.last_mut() {
                        Some(f) => f,
                        None => break,
                    };

                    // 1. 将子流程输出映射写回父流程变量
                    if let Some(cfg) = &child_frame.subflow_config {
                        if cfg.export_all_variables {
                            for (k, v) in child_frame.context.variables.get_merged_variables() {
                                parent_frame
                                    .context
                                    .variables
                                    .set_variable(k, v, String::new());
                            }
                        }
                        for mapping in &cfg.output_mappings {
                            if mapping.source.is_empty() || mapping.target.is_empty() {
                                continue;
                            }
                            if let Some(val) =
                                child_frame.context.variables.get_variable(&mapping.source)
                            {
                                parent_frame.context.variables.set_variable(
                                    mapping.target.clone(),
                                    val.clone(),
                                    String::new(),
                                );
                            }
                        }
                    }

                    // 2. 将子流程执行报告挂载到父帧对应的堆栈项
                    let child_report = Self::generate_debug_report(
                        debug_state,
                        &child_frame.flow,
                        debug_state.start_time_ms,
                        &child_frame.context,
                    )
                    .await;

                    if let Some(entry_idx) = child_frame.subflow_entry_index {
                        if entry_idx < debug_state.execution_stack.len() {
                            let now = flow_engine_core::utils::unix_timestamp_ms();
                            debug_state.execution_stack[entry_idx].ended_at = Some(now);
                            debug_state.execution_stack[entry_idx].success = true;
                            debug_state.execution_stack[entry_idx].sub_flow_report =
                                Some(Box::new(child_report));
                        }
                    }

                    // 3. 推进父帧游标到该 flow:sub 节点的后继节点
                    let parent_next = parent_frame
                        .connection_map
                        .get(&parent_frame.current_node_id)
                        .and_then(|targets| targets.first().cloned());

                    if let Some(next_id) = parent_next {
                        parent_frame.current_node_id = next_id;
                        if step_out_active || debug_state.is_stepping {
                            debug_state.is_stepping = true;
                            step_out_active = false;
                        }
                    } else {
                        parent_frame.current_node_id = String::new();
                    }

                    debug_state.call_frames = build_call_frames(&call_frames);
                    update_shared_state(debug_state);
                    continue;
                } else {
                    // 根流程无更多节点，主调试会话结束
                    break;
                }
            };

            // 检查循环
            if call_frames[active_idx].visited.contains(&current_node_id) {
                debug_state.status = DebugStatus::Ended;
                update_shared_state(debug_state);
                let report = Self::generate_debug_report(
                    debug_state,
                    root_flow,
                    debug_state.start_time_ms,
                    &call_frames[active_idx].context,
                )
                .await;
                let _ = event_sender.send(DebugEvent::SessionEnded {
                    reason: EndReason::Error {
                        message: format!("Cycle detected: node {} already visited", node.uuid),
                    },
                    report: Some(report),
                    execution_stack: debug_state.execution_stack.clone(),
                });
                return debug_state.clone();
            }

            call_frames[active_idx]
                .visited
                .insert(current_node_id.clone());
            call_frames[active_idx].context.current_node_uuid = Some(current_node_id.clone());
            debug_state.current_node_uuid = Some(current_node_id.clone());
            debug_state.call_frames = build_call_frames(&call_frames);
            update_shared_state(debug_state);

            // 检查是否暂停
            let has_bp = debug_state.has_breakpoint(&current_node_id);
            let is_stepping_pause = debug_state.is_stepping
                && (!step_out_active || call_frames.len() <= step_out_target_depth);
            let should_pause = !debug_state.skip_breakpoints && (has_bp || is_stepping_pause);
            if !should_pause {
                let now = flow_engine_core::utils::unix_timestamp_ms();
                let _ = event_sender.send(DebugEvent::Heartbeat {
                    status: DebugStatus::Running,
                    current_node_uuid: Some(current_node_id.clone()),
                    timestamp: now,
                });
            }

            if should_pause {
                let pause_reason = if is_stepping_pause && !debug_state.skip_breakpoints {
                    debug_state.is_stepping = false;
                    step_out_active = false;
                    PauseReason::StepComplete {
                        node_uuid: current_node_id.clone(),
                    }
                } else {
                    PauseReason::Breakpoint {
                        node_uuid: current_node_id.clone(),
                    }
                };

                debug_state.status = DebugStatus::Paused;
                debug_state.pause_reason = Some(pause_reason.clone());
                debug_state.variables = call_frames[active_idx]
                    .context
                    .variables
                    .get_merged_variables();
                debug_state.call_frames = build_call_frames(&call_frames);
                update_shared_state(debug_state);

                let _ = event_sender.send(DebugEvent::PausedAtNode {
                    node_uuid: current_node_id.clone(),
                    reason: pause_reason,
                    variables: debug_state.variables.clone(),
                    execution_stack: debug_state.execution_stack.clone(),
                    call_frames: debug_state.call_frames.clone(),
                });

                let active_frame = &mut call_frames[active_idx];
                let node_refs: HashMap<String, &Node> = active_frame
                    .node_map
                    .iter()
                    .map(|(k, v)| (k.clone(), v))
                    .collect();
                let flow_clone = active_frame.flow.clone();

                let command_result = self
                    .wait_for_command(
                        debug_state,
                        &node,
                        &mut active_frame.context,
                        &node_refs,
                        &flow_clone.connections,
                        &event_sender,
                        &mut command_receiver,
                        &flow_clone,
                    )
                    .await;

                match command_result {
                    Ok(StepCommandResult::SkipNode(next_node_uuid_opt, simulated_output)) => {
                        let node_end_ms = flow_engine_core::utils::unix_timestamp_ms();
                        let resolved_config = serde_json::to_value(&node.data).ok();
                        let stack_entry = ExecutionStackEntry {
                            node_uuid: node.uuid.clone(),
                            node_name: node.name.clone(),
                            node_type: node.node_type.clone(),
                            started_at: node_end_ms,
                            ended_at: Some(node_end_ms),
                            input: Some(json!(call_frames[active_idx]
                                .context
                                .variables
                                .get_merged_variables())),
                            output: simulated_output.clone(),
                            success: true,
                            error: Some("Node skipped".to_string()),
                            used_port_id: Some("skipped".to_string()),
                            resolved_config,
                            variable_changes: None,
                            skip_reason: Some("User skipped".to_string()),
                            sub_flow_report: None,
                        };
                        debug_state.push_stack_entry(stack_entry.clone());
                        let _ = event_sender.send(DebugEvent::NodeExecuted { entry: stack_entry });

                        if let Some(next_node_uuid) = next_node_uuid_opt {
                            debug_state.status = DebugStatus::Running;
                            update_shared_state(debug_state);
                            call_frames[active_idx].current_node_id = next_node_uuid;
                            continue;
                        } else {
                            call_frames[active_idx].current_node_id = String::new();
                            continue;
                        }
                    }
                    Ok(StepCommandResult::Continue) => {
                        debug_state.status = DebugStatus::Running;
                        update_shared_state(debug_state);
                    }
                    Ok(StepCommandResult::StepOver) => {
                        debug_state.status = DebugStatus::Running;
                        update_shared_state(debug_state);
                    }
                    Ok(StepCommandResult::StepInto) => {
                        let is_subflow =
                            node.node_type == node_types::SUB_FLOW || node.node_type == "flow:sub";
                        if is_subflow {
                            let subflow_config = match &node.data {
                                NodeData::SubFlow(cfg) => Some(cfg.clone()),
                                NodeData::Plugin(val) => serde_json::from_value(val.clone()).ok(),
                                _ => None,
                            };
                            if let Some(cfg) = subflow_config {
                                match crate::sub_flow_executor::SubFlowNodeExecutor::resolve_flow(
                                    &cfg,
                                    &call_frames[active_idx].context,
                                ) {
                                    Ok((child_flow, child_flow_path)) => {
                                        if (!child_flow.uuid.is_empty()
                                            && call_frames[active_idx]
                                                .context
                                                .call_stack
                                                .contains(&child_flow.uuid))
                                            || call_frames.len() >= 16
                                        {
                                            let err_msg = if call_frames.len() >= 16 {
                                                "Subflow call stack exceeded maximum depth (16)"
                                                    .to_string()
                                            } else {
                                                format!(
                                                    "Cycle detected in subflow call chain: {}",
                                                    child_flow.uuid
                                                )
                                            };
                                            debug_state.status = DebugStatus::Ended;
                                            update_shared_state(debug_state);
                                            let report = Self::generate_debug_report(
                                                debug_state,
                                                root_flow,
                                                debug_state.start_time_ms,
                                                &call_frames[active_idx].context,
                                            )
                                            .await;
                                            let _ = event_sender.send(DebugEvent::SessionEnded {
                                                reason: EndReason::Error { message: err_msg },
                                                report: Some(report),
                                                execution_stack: debug_state
                                                    .execution_stack
                                                    .clone(),
                                            });
                                            return debug_state.clone();
                                        }

                                        let node_start_ms =
                                            flow_engine_core::utils::unix_timestamp_ms();
                                        let resolved_config = serde_json::to_value(&node.data).ok();
                                        let stack_entry = ExecutionStackEntry {
                                            node_uuid: node.uuid.clone(),
                                            node_name: node.name.clone(),
                                            node_type: node.node_type.clone(),
                                            started_at: node_start_ms,
                                            ended_at: None,
                                            input: Some(json!(call_frames[active_idx]
                                                .context
                                                .variables
                                                .get_merged_variables())),
                                            output: None,
                                            success: false,
                                            error: None,
                                            used_port_id: None,
                                            resolved_config,
                                            variable_changes: None,
                                            skip_reason: None,
                                            sub_flow_report: None,
                                        };
                                        debug_state.push_stack_entry(stack_entry);
                                        let subflow_entry_idx =
                                            debug_state.execution_stack.len() - 1;

                                        let mut child_vars: HashMap<String, serde_json::Value> =
                                            if cfg.inherit_all_variables {
                                                call_frames[active_idx]
                                                    .context
                                                    .variables
                                                    .get_merged_variables()
                                            } else {
                                                HashMap::new()
                                            };
                                        for mapping in &cfg.variable_mappings {
                                            if mapping.source.is_empty()
                                                || mapping.target.is_empty()
                                            {
                                                continue;
                                            }
                                            let val = if let Some(var_val) = call_frames[active_idx]
                                                .context
                                                .variables
                                                .get_variable(&mapping.source)
                                            {
                                                var_val.clone()
                                            } else if mapping.source.contains("{{") {
                                                match flow_engine_core::executor::replace_variables(
                                                    &mapping.source,
                                                    &call_frames[active_idx].context,
                                                ) {
                                                    Ok(s) => serde_json::Value::String(s),
                                                    Err(_) => serde_json::Value::Null,
                                                }
                                            } else if mapping.source.starts_with('$') {
                                                flow_engine_core::utils::extract_value_by_path(
                                                    &call_frames[active_idx]
                                                        .context
                                                        .last_node_output
                                                        .clone()
                                                        .unwrap_or(serde_json::Value::Null),
                                                    &mapping.source,
                                                )
                                                .unwrap_or(serde_json::Value::Null)
                                            } else if let Some(v) = call_frames[active_idx]
                                                .context
                                                .variables
                                                .get_merged_variables()
                                                .get(&mapping.source)
                                            {
                                                v.clone()
                                            } else {
                                                serde_json::Value::String(mapping.source.clone())
                                            };
                                            child_vars.insert(mapping.target.clone(), val);
                                        }

                                        let mut child_context = ExecutionContext::new(
                                            child_flow.uuid.clone(),
                                            call_frames[active_idx].context.flow_trigger_time,
                                        );
                                        child_context.flow_file_path = child_flow_path;
                                        child_context.log_sender =
                                            call_frames[active_idx].context.log_sender.clone();
                                        child_context.call_stack =
                                            call_frames[active_idx].context.call_stack.clone();
                                        // Provider 必须随调用栈下传：否则子流程内的长连接探针
                                        // （SendStreamFrame）与 Ingress 响应节点都取不到共享实例。
                                        child_context.providers =
                                            call_frames[active_idx].context.providers.clone();
                                        if !child_flow.uuid.is_empty() {
                                            child_context.call_stack.push(child_flow.uuid.clone());
                                        }
                                        for (k, v) in child_vars {
                                            child_context.variables.set_variable(
                                                k,
                                                v,
                                                String::new(),
                                            );
                                        }

                                        let entry_node = child_flow
                                            .nodes
                                            .iter()
                                            .find(|n| n.node_type == node_types::ENTRY);
                                        let first_node_id = if let Some(entry) = entry_node {
                                            let entry_target = child_flow
                                                .connections
                                                .iter()
                                                .find(|c| c.source_node_uuid == entry.uuid)
                                                .map(|c| c.target_node_uuid.clone());
                                            if let Some(target_id) = entry_target {
                                                let now =
                                                    flow_engine_core::utils::unix_timestamp_ms();
                                                let entry_entry = ExecutionStackEntry {
                                                    node_uuid: entry.uuid.clone(),
                                                    node_name: entry.name.clone(),
                                                    node_type: entry.node_type.clone(),
                                                    started_at: now,
                                                    ended_at: Some(now),
                                                    input: Some(json!(child_context
                                                        .variables
                                                        .get_merged_variables())),
                                                    output: None,
                                                    success: true,
                                                    error: None,
                                                    used_port_id: Some("out".to_string()),
                                                    resolved_config: serde_json::to_value(
                                                        &entry.data,
                                                    )
                                                    .ok(),
                                                    variable_changes: None,
                                                    skip_reason: None,
                                                    sub_flow_report: None,
                                                };
                                                debug_state.push_stack_entry(entry_entry);
                                                target_id
                                            } else {
                                                entry.uuid.clone()
                                            }
                                        } else {
                                            child_flow
                                                .nodes
                                                .first()
                                                .map(|n| n.uuid.clone())
                                                .unwrap_or_default()
                                        };

                                        let mut child_node_map = HashMap::new();
                                        for n in &child_flow.nodes {
                                            child_node_map.insert(n.uuid.clone(), n.clone());
                                        }
                                        let mut child_connection_map: HashMap<String, Vec<String>> =
                                            HashMap::new();
                                        for c in &child_flow.connections {
                                            child_connection_map
                                                .entry(c.source_node_uuid.clone())
                                                .or_default()
                                                .push(c.target_node_uuid.clone());
                                        }

                                        let child_frame = DebugFrameState {
                                            frame_id: call_frames.len(),
                                            flow: child_flow,
                                            node_map: child_node_map,
                                            connection_map: child_connection_map,
                                            current_node_id: first_node_id,
                                            parent_subflow_node_uuid: Some(node.uuid.clone()),
                                            context: child_context,
                                            subflow_config: Some(cfg),
                                            visited: std::collections::HashSet::new(),
                                            subflow_entry_index: Some(subflow_entry_idx),
                                            snapshots: HashMap::new(),
                                        };
                                        call_frames.push(child_frame);

                                        debug_state.is_stepping = true;
                                        debug_state.status = DebugStatus::Running;
                                        debug_state.call_frames = build_call_frames(&call_frames);
                                        update_shared_state(debug_state);
                                        continue;
                                    }
                                    Err(e) => {
                                        log::warn!("[Debug] StepInto resolve_flow failed, falling back to StepOver: {}", e);
                                    }
                                }
                            }
                        }
                        debug_state.status = DebugStatus::Running;
                        update_shared_state(debug_state);
                    }
                    Ok(StepCommandResult::StepOut) => {
                        if call_frames.len() > 1 {
                            step_out_active = true;
                            step_out_target_depth = call_frames.len() - 1;
                            debug_state.is_stepping = false;
                        }
                        debug_state.status = DebugStatus::Running;
                        update_shared_state(debug_state);
                    }
                    Ok(StepCommandResult::RunToEnd) => {
                        debug_state.status = DebugStatus::Running;
                        update_shared_state(debug_state);
                    }
                    Ok(StepCommandResult::RetryCurrentNode) => {
                        let active_frame = &mut call_frames[active_idx];
                        if let Some(snapshot) = active_frame.snapshots.get(&current_node_id) {
                            active_frame.context.variables = snapshot.variables.clone();
                            debug_state.execution_stack.truncate(snapshot.stack_len);
                            active_frame.visited.remove(&current_node_id);
                            debug_state.status = DebugStatus::Running;
                            debug_state.pause_reason = None;
                            debug_state.variables = active_frame.context.variables.get_merged_variables();
                            update_shared_state(debug_state);
                            continue;
                        } else {
                            active_frame.visited.remove(&current_node_id);
                            debug_state.status = DebugStatus::Running;
                            update_shared_state(debug_state);
                            continue;
                        }
                    }
                    Ok(StepCommandResult::RunFromSnapshot(target_node_uuid)) => {
                        let active_frame = &mut call_frames[active_idx];
                        if let Some(snapshot) = active_frame.snapshots.get(&target_node_uuid).cloned() {
                            active_frame.context.variables = snapshot.variables.clone();
                            debug_state.execution_stack.truncate(snapshot.stack_len);
                            active_frame.visited = snapshot.visited.clone();
                            active_frame.visited.remove(&target_node_uuid);
                            active_frame.current_node_id = target_node_uuid.clone();
                            debug_state.current_node_uuid = Some(target_node_uuid.clone());
                            debug_state.is_stepping = true;
                            debug_state.status = DebugStatus::Paused;
                            let pause_reason = PauseReason::StepComplete {
                                node_uuid: target_node_uuid.clone(),
                            };
                            debug_state.pause_reason = Some(pause_reason.clone());
                            debug_state.variables = active_frame.context.variables.get_merged_variables();
                            debug_state.call_frames = build_call_frames(&call_frames);
                            update_shared_state(debug_state);

                            let _ = event_sender.send(DebugEvent::SnapshotRestored {
                                node_uuid: target_node_uuid.clone(),
                                restored_variables: debug_state.variables.clone(),
                                stack_len: snapshot.stack_len,
                            });
                            let _ = event_sender.send(DebugEvent::PausedAtNode {
                                node_uuid: target_node_uuid,
                                reason: pause_reason,
                                variables: debug_state.variables.clone(),
                                execution_stack: debug_state.execution_stack.clone(),
                                call_frames: debug_state.call_frames.clone(),
                            });
                            continue;
                        } else {
                            let _ = event_sender.send(DebugEvent::Error {
                                message: format!("Snapshot not found for node {}", target_node_uuid),
                            });
                            continue;
                        }
                    }
                    Err(error) => {
                        let reason = if error == "User aborted debugging" {
                            EndReason::UserAbort
                        } else {
                            EndReason::Error { message: error }
                        };
                        debug_state.status = DebugStatus::Ended;
                        update_shared_state(debug_state);
                        let report = Self::generate_debug_report(
                            debug_state,
                            root_flow,
                            debug_state.start_time_ms,
                            &call_frames[active_idx].context,
                        )
                        .await;
                        let _ = event_sender.send(DebugEvent::SessionEnded {
                            reason,
                            report: Some(report),
                            execution_stack: debug_state.execution_stack.clone(),
                        });
                        return debug_state.clone();
                    }
                }
            }

            // 执行当前节点
            let active_frame = match call_frames.last_mut() {
                Some(f) => f,
                None => break,
            };
            let node_start_ms = flow_engine_core::utils::unix_timestamp_ms();
            let resolved_config = serde_json::to_value(&node.data).ok();

            // 记录节点执行前快照（最多保留 32 个快照）
            if active_frame.snapshots.len() >= 32 {
                if let Some(oldest_key) = active_frame
                    .snapshots
                    .iter()
                    .min_by_key(|(_, s)| s.timestamp_ms)
                    .map(|(k, _)| k.clone())
                {
                    active_frame.snapshots.remove(&oldest_key);
                }
            }
            active_frame.snapshots.insert(
                current_node_id.clone(),
                ContextSnapshot {
                    node_uuid: current_node_id.clone(),
                    variables: active_frame.context.variables.clone(),
                    stack_len: debug_state.execution_stack.len(),
                    visited: active_frame.visited.clone(),
                    timestamp_ms: flow_engine_core::utils::unix_timestamp_ms(),
                },
            );
            let stack_entry = ExecutionStackEntry {
                node_uuid: node.uuid.clone(),
                node_name: node.name.clone(),
                node_type: node.node_type.clone(),
                started_at: node_start_ms,
                ended_at: None,
                input: Some(json!(active_frame.context.variables.get_merged_variables())),
                output: None,
                success: false,
                error: None,
                used_port_id: None,
                resolved_config: resolved_config.clone(),
                variable_changes: None,
                skip_reason: None,
                sub_flow_report: None,
            };
            debug_state.push_stack_entry(stack_entry);

            let effective_node =
                if let Some(temp_config) = debug_state.get_temp_node_config(&node.uuid) {
                    match serde_json::from_value::<NodeData>(temp_config.clone()) {
                        Ok(new_data) => Node {
                            data: new_data,
                            ..node.clone()
                        },
                        Err(_) => node.clone(),
                    }
                } else {
                    node.clone()
                };

            active_frame.context.node_reports = debug_state
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
                        duration_ms: entry
                            .ended_at
                            .map(|end| end.saturating_sub(entry.started_at))
                            .unwrap_or(0),
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
                        sub_flow_report: entry.sub_flow_report.clone(),
                    }
                })
                .collect();

            active_frame.context.variables.clear_pending_changes();
            let var_version_before = active_frame.context.variables.version();
            let vars_before = active_frame.context.variables.get_merged_variables();
            let node_refs: HashMap<String, &Node> = active_frame
                .node_map
                .iter()
                .map(|(k, v)| (k.clone(), v))
                .collect();

            match self
                .execute_node(
                    &effective_node,
                    &mut active_frame.context,
                    &active_frame.flow.connections,
                    &node_refs,
                )
                .await
            {
                Ok((next_node_ids, output_data)) => {
                    let mut variable_changes =
                        if active_frame.context.variables.version() == var_version_before {
                            Vec::new()
                        } else {
                            active_frame.context.variables.take_pending_changes()
                        };
                    if variable_changes.is_empty() {
                        let vars_after = active_frame.context.variables.get_merged_variables();
                        diff_variable_changes(&vars_before, &vars_after, &mut variable_changes);
                    }

                    let node_end_ms = flow_engine_core::utils::unix_timestamp_ms();
                    let type_id = node.data.type_name();
                    let used_port_id = self
                        .registry
                        .get(&type_id)
                        .map(|plugin| Some(plugin.resolve_port(&output_data)))
                        .unwrap_or(None);

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
                            Some(variable_changes)
                        },
                        None,
                    );

                    debug_state.set_last_stack_entry_sub_flow_report(
                        active_frame.context.current_sub_flow_report.take(),
                    );

                    let completed_entry = debug_state
                        .execution_stack
                        .last()
                        .cloned()
                        .unwrap_or_else(|| unreachable!("stack entry just pushed"));
                    let _ = event_sender.send(DebugEvent::NodeExecuted {
                        entry: completed_entry,
                    });

                    if let Some(ref data) = output_data {
                        active_frame.context.last_node_output = Some(data.clone());
                    }

                    let next_node_id = if !next_node_ids.is_empty() {
                        Some(next_node_ids[0].clone())
                    } else {
                        active_frame
                            .connection_map
                            .get(&current_node_id)
                            .and_then(|targets| targets.first().cloned())
                    };

                    if let Some(next_id) = next_node_id {
                        active_frame.current_node_id = next_id;
                    } else {
                        active_frame.current_node_id = String::new();
                    }
                }
                Err(error) => {
                    let mut variable_changes =
                        if active_frame.context.variables.version() == var_version_before {
                            Vec::new()
                        } else {
                            active_frame.context.variables.take_pending_changes()
                        };
                    if variable_changes.is_empty() {
                        let vars_after = active_frame.context.variables.get_merged_variables();
                        diff_variable_changes(&vars_before, &vars_after, &mut variable_changes);
                    }

                    let node_end_ms = flow_engine_core::utils::unix_timestamp_ms();
                    let route_to_false = node
                        .on_error
                        .as_ref()
                        .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse);
                    let is_ignore = node
                        .on_error
                        .as_ref()
                        .is_some_and(|s| *s == ExecutionErrorStrategy::Ignore);

                    let false_port_next_node = if route_to_false {
                        active_frame
                            .flow
                            .connections
                            .iter()
                            .find(|conn| {
                                conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                            })
                            .map(|conn| conn.target_node_uuid.clone())
                    } else {
                        None
                    };

                    let used_port_id = if false_port_next_node.is_some() {
                        Some("false".to_string())
                    } else {
                        None
                    };

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
                            Some(variable_changes)
                        },
                        None,
                    );

                    debug_state.set_last_stack_entry_sub_flow_report(
                        active_frame.context.current_sub_flow_report.take(),
                    );

                    let failed_entry = debug_state
                        .execution_stack
                        .last()
                        .cloned()
                        .unwrap_or_else(|| unreachable!("stack entry just pushed"));
                    let _ = event_sender.send(DebugEvent::NodeExecuted {
                        entry: failed_entry,
                    });

                    if route_to_false {
                        if let Some(next_node_uuid) = false_port_next_node {
                            active_frame.current_node_id = next_node_uuid;
                            continue;
                        }
                    }

                    if is_ignore {
                        let next_node_id = active_frame
                            .flow
                            .connections
                            .iter()
                            .find(|conn| {
                                conn.source_node_uuid == node.uuid
                                    && (conn.source_port_id == "true"
                                        || conn.source_port_id == "out")
                            })
                            .map(|conn| conn.target_node_uuid.clone());

                        if let Some(next_uuid) = next_node_id {
                            active_frame.current_node_id = next_uuid;
                            continue;
                        }
                        active_frame.current_node_id = String::new();
                        continue;
                    }

                    // 调试模式下发生未被错误策略捕获的异常时，暂停在当前节点以供原地修改配置/变量并重试
                    debug_state.status = DebugStatus::Paused;
                    let pause_reason = PauseReason::Error {
                        message: error.message.clone(),
                    };
                    debug_state.pause_reason = Some(pause_reason.clone());
                    debug_state.variables = active_frame.context.variables.get_merged_variables();
                    debug_state.call_frames = build_call_frames(&call_frames);
                    update_shared_state(debug_state);

                    let _ = event_sender.send(DebugEvent::Error {
                        message: error.message.clone(),
                    });
                    let _ = event_sender.send(DebugEvent::PausedAtNode {
                        node_uuid: current_node_id.clone(),
                        reason: pause_reason,
                        variables: debug_state.variables.clone(),
                        execution_stack: debug_state.execution_stack.clone(),
                        call_frames: debug_state.call_frames.clone(),
                    });

                    let active_frame = &mut call_frames[active_idx];
                    let node_refs: HashMap<String, &Node> = active_frame
                        .node_map
                        .iter()
                        .map(|(k, v)| (k.clone(), v))
                        .collect();
                    let flow_clone = active_frame.flow.clone();

                    let command_result = self
                        .wait_for_command(
                            debug_state,
                            &node,
                            &mut active_frame.context,
                            &node_refs,
                            &flow_clone.connections,
                            &event_sender,
                            &mut command_receiver,
                            &flow_clone,
                        )
                        .await;

                    match command_result {
                        // 出错暂停下，继续/单步/执行到底与“原地重试”同义：
                        // 都用快照回滚当前节点后重新执行，避免用户按 F5/F10/F11 直接结束会话。
                        Ok(StepCommandResult::RetryCurrentNode)
                        | Ok(StepCommandResult::Continue)
                        | Ok(StepCommandResult::StepOver)
                        | Ok(StepCommandResult::StepInto)
                        | Ok(StepCommandResult::StepOut)
                        | Ok(StepCommandResult::RunToEnd) => {
                            let active_frame = &mut call_frames[active_idx];
                            if let Some(snapshot) = active_frame.snapshots.get(&current_node_id) {
                                active_frame.context.variables = snapshot.variables.clone();
                                debug_state.execution_stack.truncate(snapshot.stack_len);
                                active_frame.visited.remove(&current_node_id);
                                debug_state.status = DebugStatus::Running;
                                debug_state.pause_reason = None;
                                debug_state.variables = active_frame.context.variables.get_merged_variables();
                                update_shared_state(debug_state);
                                continue;
                            } else {
                                active_frame.visited.remove(&current_node_id);
                                debug_state.status = DebugStatus::Running;
                                update_shared_state(debug_state);
                                continue;
                            }
                        }
                        Ok(StepCommandResult::RunFromSnapshot(target_node_uuid)) => {
                            let active_frame = &mut call_frames[active_idx];
                            if let Some(snapshot) = active_frame.snapshots.get(&target_node_uuid).cloned() {
                                active_frame.context.variables = snapshot.variables.clone();
                                debug_state.execution_stack.truncate(snapshot.stack_len);
                                active_frame.visited = snapshot.visited.clone();
                                active_frame.visited.remove(&target_node_uuid);
                                active_frame.current_node_id = target_node_uuid.clone();
                                debug_state.current_node_uuid = Some(target_node_uuid.clone());
                                debug_state.is_stepping = true;
                                debug_state.status = DebugStatus::Paused;
                                let pause_reason = PauseReason::StepComplete {
                                    node_uuid: target_node_uuid.clone(),
                                };
                                debug_state.pause_reason = Some(pause_reason.clone());
                                debug_state.variables = active_frame.context.variables.get_merged_variables();
                                debug_state.call_frames = build_call_frames(&call_frames);
                                update_shared_state(debug_state);

                                let _ = event_sender.send(DebugEvent::SnapshotRestored {
                                    node_uuid: target_node_uuid.clone(),
                                    restored_variables: debug_state.variables.clone(),
                                    stack_len: snapshot.stack_len,
                                });
                                let _ = event_sender.send(DebugEvent::PausedAtNode {
                                    node_uuid: target_node_uuid,
                                    reason: pause_reason,
                                    variables: debug_state.variables.clone(),
                                    execution_stack: debug_state.execution_stack.clone(),
                                    call_frames: debug_state.call_frames.clone(),
                                });
                                continue;
                            } else {
                                // 快照缺失（可能已被 32 条上限淘汰）：保持暂停并提示，不能顺势结束会话
                                let _ = event_sender.send(DebugEvent::Error {
                                    message: format!("Snapshot not found for node {}", target_node_uuid),
                                });
                                continue;
                            }
                        }
                        Ok(StepCommandResult::SkipNode(next_node_uuid_opt, _)) => {
                            if let Some(next_node_uuid) = next_node_uuid_opt {
                                debug_state.status = DebugStatus::Running;
                                update_shared_state(debug_state);
                                call_frames[active_idx].current_node_id = next_node_uuid;
                                continue;
                            } else {
                                call_frames[active_idx].current_node_id = String::new();
                                continue;
                            }
                        }
                        Err(abort) if abort == "User aborted debugging" => {
                            // EndDebug 已在 wait_for_command 内发出 SessionEnded(UserAbort) 并落盘报告，
                            // 此处直接返回，避免重复生成报告并把结束原因改写成 Error。
                            return debug_state.clone();
                        }
                        Err(error) => {
                            debug_state.status = DebugStatus::Ended;
                            update_shared_state(debug_state);
                            let report = Self::generate_debug_report(
                                debug_state,
                                root_flow,
                                debug_state.start_time_ms,
                                &call_frames[active_idx].context,
                            )
                            .await;
                            let _ = event_sender.send(DebugEvent::SessionEnded {
                                reason: EndReason::Error {
                                    message: error,
                                },
                                report: Some(report),
                                execution_stack: debug_state.execution_stack.clone(),
                            });
                            return debug_state.clone();
                        }
                    }
                }
            }
        }

        if let Some(first_frame) = call_frames.first() {
            *context = first_frame.context.clone();
        }

        debug_state.status = DebugStatus::Ended;
        debug_state.current_node_uuid = None;
        debug_state.call_frames.clear();
        debug_state.variables = context.variables.get_merged_variables();
        update_shared_state(debug_state);
        let report =
            Self::generate_debug_report(debug_state, root_flow, debug_state.start_time_ms, context)
                .await;
        let _ = event_sender.send(DebugEvent::SessionEnded {
            reason: EndReason::Completed,
            report: Some(report),
            execution_stack: debug_state.execution_stack.clone(),
        });

        debug_state.clone()
    }

    /// 等待调试命令
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
    ) -> Result<StepCommandResult, String> {
        while let Some(command) = command_receiver.recv().await {
            match command {
                DebugCommand::Continue => {
                    debug_state.is_stepping = false;
                    debug_state.skip_breakpoints = false;
                    return Ok(StepCommandResult::Continue);
                }
                DebugCommand::StepOver => {
                    debug_state.is_stepping = true;
                    debug_state.skip_breakpoints = false;
                    return Ok(StepCommandResult::StepOver);
                }
                DebugCommand::StepInto => {
                    debug_state.is_stepping = true;
                    debug_state.skip_breakpoints = false;
                    return Ok(StepCommandResult::StepInto);
                }
                DebugCommand::StepOut => {
                    debug_state.is_stepping = false;
                    debug_state.skip_breakpoints = false;
                    return Ok(StepCommandResult::StepOut);
                }
                DebugCommand::RunToEnd => {
                    debug_state.is_stepping = false;
                    debug_state.skip_breakpoints = true;
                    return Ok(StepCommandResult::RunToEnd);
                }
                DebugCommand::SkipNode {
                    selected_port,
                    simulated_output,
                } => {
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
                    return Ok(StepCommandResult::SkipNode(
                        result.next_node_uuid,
                        result.simulated_output,
                    ));
                }
                DebugCommand::EndDebug => {
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
                    context
                        .variables
                        .set_variable(name.clone(), value.clone(), String::new());
                    debug_state.set_variable(name.clone(), value.clone());
                    if let Err(e) = event_sender.send(DebugEvent::VariableModified { name, value })
                    {
                        log::error!("[Debug] Failed to send VariableModified event: {}", e);
                    }
                }
                DebugCommand::ModifyNodeConfig { node_uuid, config } => {
                    debug_state.set_temp_node_config(node_uuid.clone(), config.clone());
                    if let Err(e) =
                        event_sender.send(DebugEvent::NodeConfigModified { node_uuid, config })
                    {
                        log::error!("[Debug] Failed to send NodeConfigModified event: {}", e);
                    }
                }
                DebugCommand::SetBreakpoint { node_uuid, enabled } => {
                    if enabled {
                        debug_state.add_breakpoint(node_uuid.clone());
                        log::info!("[Debug] Added breakpoint: {}", node_uuid);
                    } else {
                        debug_state.remove_breakpoint(&node_uuid);
                        log::info!("[Debug] Removed breakpoint: {}", node_uuid);
                    }
                    if let Err(e) =
                        event_sender.send(DebugEvent::BreakpointModified { node_uuid, enabled })
                    {
                        log::error!("[Debug] Failed to send BreakpointModified event: {}", e);
                    }
                }
                DebugCommand::RetryCurrentNode => {
                    return Ok(StepCommandResult::RetryCurrentNode);
                }
                DebugCommand::RunFromSnapshot { node_uuid } => {
                    return Ok(StepCommandResult::RunFromSnapshot(node_uuid));
                }
                DebugCommand::SendStreamFrame {
                    connection_id,
                    payload,
                    is_binary,
                } => {
                    #[cfg(feature = "protocol-ws")]
                    {
                        if let Some(ws_manager) = context
                            .get_provider::<flow_engine_ws::websocket_manager::WebSocketConnectionManager>("ws")
                        {
                            match ws_manager.send(&connection_id, payload.clone(), is_binary, 30_000).await {
                                Ok(()) => {
                                    let now = flow_engine_core::utils::unix_timestamp_ms();
                                    let sent_content = if is_binary {
                                        flow_engine_core::ws_stream::MessageContent::Binary {
                                            content: payload.as_bytes().to_vec(),
                                        }
                                    } else {
                                        flow_engine_core::ws_stream::MessageContent::Text {
                                            content: payload.clone(),
                                        }
                                    };
                                    let sent_msg = flow_engine_core::ws_stream::WsMessage::new(
                                        flow_engine_core::ws_stream::MessageDirection::Sent,
                                        sent_content,
                                    );
                                    let _ = context.push_ws_message(&connection_id, sent_msg.clone()).await;
                                    let _ = event_sender.send(DebugEvent::WsMessageSent {
                                        connection_id: connection_id.clone(),
                                        owner_node_uuid: current_node.uuid.clone(),
                                        message: sent_msg,
                                    });
                                    let _ = event_sender.send(DebugEvent::StreamFrameSent {
                                        connection_id,
                                        timestamp_ms: now,
                                        payload,
                                    });
                                }
                                Err(err) => {
                                    let _ = event_sender.send(DebugEvent::Error {
                                        message: format!("Failed to send stream frame: {err}"),
                                    });
                                }
                            }
                        } else {
                            let _ = event_sender.send(DebugEvent::Error {
                                message: "WebSocket manager provider not available".to_string(),
                            });
                        }
                    }
                    #[cfg(not(feature = "protocol-ws"))]
                    {
                        let _ = is_binary;
                        let _ = connection_id;
                        let _ = payload;
                        let _ = event_sender.send(DebugEvent::Error {
                            message: "WebSocket protocol feature not enabled".to_string(),
                        });
                    }
                }
                DebugCommand::StartDebug { .. } => {
                    log::warn!("Debug session already started, ignoring StartDebug command");
                }
            }
        }
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
                    "[Debug {}] Executing {} node: {} ({})",
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

        // 将 execution_stack 转换为 NodeExecutionReport（按当前 flow 节点筛选）
        let flow_node_uuids: std::collections::HashSet<&str> =
            flow.nodes.iter().map(|n| n.uuid.as_str()).collect();
        let node_reports: Vec<NodeExecutionReport> = debug_state
            .execution_stack
            .iter()
            .filter(|entry| {
                flow_node_uuids.is_empty() || flow_node_uuids.contains(entry.node_uuid.as_str())
            })
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
                    sub_flow_report: entry.sub_flow_report.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::FlowFile;
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

    #[tokio::test]
    async fn test_debug_step_into_subflow() {
        use tempfile::tempdir;

        let dir = tempdir().expect("create tempdir");
        let child_path = dir.path().join("child.json");

        let child_flow = Flow {
            uuid: "child-flow-uuid".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Child Flow".to_string(),
            description: None,
            nodes: vec![
                Node {
                    uuid: "c-entry".to_string(),
                    node_type: "entry".to_string(),
                    name: "Child Entry".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::entry(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "c-script".to_string(),
                    node_type: "script".to_string(),
                    name: "Child Script".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::Script(flow_engine_core::node_config::ScriptNodeConfig {
                        script: "return { a: 1 };".to_string(),
                        timeout_ms: 5000,
                        ..Default::default()
                    }),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "c-end".to_string(),
                    node_type: "end".to_string(),
                    name: "Child End".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::end(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
            ],
            connections: vec![
                Connection {
                    id: "c-c1".to_string(),
                    source_node_uuid: "c-entry".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "c-script".to_string(),
                    target_port_id: "in".to_string(),
                },
                Connection {
                    id: "c-c2".to_string(),
                    source_node_uuid: "c-script".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "c-end".to_string(),
                    target_port_id: "in".to_string(),
                },
            ],
            hooks: None,
        };

        let child_file = FlowFile {
            _version: 1,
            flow: child_flow.clone(),
            _meta: None,
        };
        std::fs::write(
            &child_path,
            serde_json::to_string_pretty(&child_file).unwrap(),
        )
        .unwrap();

        let parent_flow = Flow {
            uuid: "parent-flow-uuid".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Parent Flow".to_string(),
            description: None,
            nodes: vec![
                Node {
                    uuid: "p-entry".to_string(),
                    node_type: "entry".to_string(),
                    name: "Parent Entry".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::entry(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "p-subflow".to_string(),
                    node_type: "flow:sub".to_string(),
                    name: "Call Child".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::sub_flow(flow_engine_core::node_config::SubFlowConfig {
                        target_flow_id: None,
                        target_flow_path: Some(child_path.to_string_lossy().to_string()),
                        inherit_all_variables: true,
                        variable_mappings: vec![],
                        export_all_variables: true,
                        output_mappings: vec![],
                    }),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "p-end".to_string(),
                    node_type: "end".to_string(),
                    name: "Parent End".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::end(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
            ],
            connections: vec![
                Connection {
                    id: "p-c1".to_string(),
                    source_node_uuid: "p-entry".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "p-subflow".to_string(),
                    target_port_id: "in".to_string(),
                },
                Connection {
                    id: "p-c2".to_string(),
                    source_node_uuid: "p-subflow".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "p-end".to_string(),
                    target_port_id: "in".to_string(),
                },
            ],
            hooks: None,
        };

        let executor = DebuggableFlowExecutor::new();
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let parent_clone = parent_flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &parent_clone,
                    None,
                    vec!["p-subflow".to_string()],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        let mut stepped_into = false;
        let mut verified_child_paused = false;

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, call_frames, .. } => {
                            if node_uuid == "p-subflow" && !stepped_into {
                                assert_eq!(call_frames.len(), 1);
                                assert_eq!(call_frames[0].flow_uuid, "parent-flow-uuid");
                                stepped_into = true;
                                let _ = command_sender.send(DebugCommand::StepInto);
                            } else if node_uuid == "c-script" {
                                assert_eq!(call_frames.len(), 2, "Call frames depth should be 2 after StepInto");
                                assert_eq!(call_frames[0].flow_uuid, "parent-flow-uuid");
                                assert_eq!(call_frames[1].flow_uuid, "child-flow-uuid");
                                assert_eq!(call_frames[1].node_uuid, "c-script");
                                verified_child_paused = true;
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
                    panic!("test_debug_step_into_subflow timeout");
                }
            }
        }

        let _ = handle.await;
        assert!(stepped_into, "Should have stepped into subflow");
        assert!(
            verified_child_paused,
            "Should have verified child script paused with 2 call frames"
        );
    }

    #[tokio::test]
    async fn test_debug_step_out_subflow() {
        use tempfile::tempdir;

        let dir = tempdir().expect("create tempdir");
        let child_path = dir.path().join("child.json");

        let child_flow = Flow {
            uuid: "child-flow-out".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Child Flow".to_string(),
            description: None,
            nodes: vec![
                Node {
                    uuid: "c-entry".to_string(),
                    node_type: "entry".to_string(),
                    name: "Child Entry".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::entry(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "c-script".to_string(),
                    node_type: "script".to_string(),
                    name: "Child Script".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::Script(flow_engine_core::node_config::ScriptNodeConfig {
                        script: "return { a: 1 };".to_string(),
                        timeout_ms: 5000,
                        ..Default::default()
                    }),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "c-end".to_string(),
                    node_type: "end".to_string(),
                    name: "Child End".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::end(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
            ],
            connections: vec![
                Connection {
                    id: "c-c1".to_string(),
                    source_node_uuid: "c-entry".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "c-script".to_string(),
                    target_port_id: "in".to_string(),
                },
                Connection {
                    id: "c-c2".to_string(),
                    source_node_uuid: "c-script".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "c-end".to_string(),
                    target_port_id: "in".to_string(),
                },
            ],
            hooks: None,
        };

        let child_file = FlowFile {
            _version: 1,
            flow: child_flow.clone(),
            _meta: None,
        };
        std::fs::write(
            &child_path,
            serde_json::to_string_pretty(&child_file).unwrap(),
        )
        .unwrap();

        let parent_flow = Flow {
            uuid: "parent-flow-out".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Parent Flow".to_string(),
            description: None,
            nodes: vec![
                Node {
                    uuid: "p-entry".to_string(),
                    node_type: "entry".to_string(),
                    name: "Parent Entry".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::entry(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "p-subflow".to_string(),
                    node_type: "flow:sub".to_string(),
                    name: "Call Child".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::sub_flow(flow_engine_core::node_config::SubFlowConfig {
                        target_flow_id: None,
                        target_flow_path: Some(child_path.to_string_lossy().to_string()),
                        inherit_all_variables: true,
                        variable_mappings: vec![],
                        export_all_variables: true,
                        output_mappings: vec![],
                    }),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "p-end".to_string(),
                    node_type: "end".to_string(),
                    name: "Parent End".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::end(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
            ],
            connections: vec![
                Connection {
                    id: "p-c1".to_string(),
                    source_node_uuid: "p-entry".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "p-subflow".to_string(),
                    target_port_id: "in".to_string(),
                },
                Connection {
                    id: "p-c2".to_string(),
                    source_node_uuid: "p-subflow".to_string(),
                    source_port_id: "out".to_string(),
                    target_node_uuid: "p-end".to_string(),
                    target_port_id: "in".to_string(),
                },
            ],
            hooks: None,
        };

        let executor = DebuggableFlowExecutor::new();
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let parent_clone = parent_flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &parent_clone,
                    None,
                    vec!["p-subflow".to_string()],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        let mut stepped_into = false;
        let mut stepped_out = false;
        let mut verified_parent_resumed = false;

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, call_frames, .. } => {
                            if node_uuid == "p-subflow" && !stepped_into {
                                stepped_into = true;
                                let _ = command_sender.send(DebugCommand::StepInto);
                            } else if node_uuid == "c-script" && !stepped_out {
                                assert_eq!(call_frames.len(), 2);
                                stepped_out = true;
                                let _ = command_sender.send(DebugCommand::StepOut);
                            } else if node_uuid == "p-end" {
                                assert_eq!(call_frames.len(), 1, "Call frames should pop back to depth 1 after StepOut");
                                assert_eq!(call_frames[0].flow_uuid, "parent-flow-out");
                                verified_parent_resumed = true;
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
                    panic!("test_debug_step_out_subflow timeout");
                }
            }
        }

        let _ = handle.await;
        assert!(stepped_into, "Should have stepped into subflow");
        assert!(stepped_out, "Should have triggered step out");
        assert!(
            verified_parent_resumed,
            "Should have resumed and paused at parent p-end node"
        );
    }

    #[tokio::test]
    async fn test_debug_subflow_memory_isolation() {
        use tempfile::tempdir;

        let dir = tempdir().expect("create tempdir");
        let child_path = dir.path().join("child_isolated.json");

        let child_flow = Flow {
            uuid: "child-flow-iso".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Child Flow Isolated".to_string(),
            description: None,
            nodes: vec![
                Node {
                    uuid: "c-entry".to_string(),
                    node_type: "entry".to_string(),
                    name: "Child Entry".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::entry(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "c-script".to_string(),
                    node_type: "script".to_string(),
                    name: "Child Script".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::Script(flow_engine_core::node_config::ScriptNodeConfig {
                        script: "return { x: 100 };".to_string(),
                        timeout_ms: 5000,
                        ..Default::default()
                    }),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
            ],
            connections: vec![Connection {
                id: "c-c1".to_string(),
                source_node_uuid: "c-entry".to_string(),
                source_port_id: "out".to_string(),
                target_node_uuid: "c-script".to_string(),
                target_port_id: "in".to_string(),
            }],
            hooks: None,
        };

        let child_file = FlowFile {
            _version: 1,
            flow: child_flow.clone(),
            _meta: None,
        };
        let original_content = serde_json::to_string_pretty(&child_file).unwrap();
        std::fs::write(&child_path, &original_content).unwrap();

        let parent_flow = Flow {
            uuid: "parent-flow-iso".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            name: "Parent Flow".to_string(),
            description: None,
            nodes: vec![
                Node {
                    uuid: "p-entry".to_string(),
                    node_type: "entry".to_string(),
                    name: "Parent Entry".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::entry(),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
                Node {
                    uuid: "p-subflow".to_string(),
                    node_type: "flow:sub".to_string(),
                    name: "Call Child".to_string(),
                    position: NodePosition::default(),
                    data: NodeData::sub_flow(flow_engine_core::node_config::SubFlowConfig {
                        target_flow_id: None,
                        target_flow_path: Some(child_path.to_string_lossy().to_string()),
                        inherit_all_variables: true,
                        variable_mappings: vec![],
                        export_all_variables: true,
                        output_mappings: vec![],
                    }),
                    ports: None,
                    has_breakpoint: None,
                    on_error: None,
                },
            ],
            connections: vec![Connection {
                id: "p-c1".to_string(),
                source_node_uuid: "p-entry".to_string(),
                source_port_id: "out".to_string(),
                target_node_uuid: "p-subflow".to_string(),
                target_port_id: "in".to_string(),
            }],
            hooks: None,
        };

        let executor = DebuggableFlowExecutor::new();
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let parent_clone = parent_flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &parent_clone,
                    None,
                    vec!["p-subflow".to_string()],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, .. } => {
                            if node_uuid == "p-subflow" {
                                let _ = command_sender.send(DebugCommand::StepInto);
                            } else if node_uuid == "c-script" {
                                // 修改变量并修改配置
                                let _ = command_sender.send(DebugCommand::ModifyVariable {
                                    name: "x".to_string(),
                                    value: json!(999),
                                });
                                let _ = command_sender.send(DebugCommand::ModifyNodeConfig {
                                    node_uuid: "c-script".to_string(),
                                    config: json!({"script": "return { x: 999 };"}),
                                });
                                let _ = command_sender.send(DebugCommand::EndDebug);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("test_debug_subflow_memory_isolation timeout");
                }
            }
        }

        let _ = handle.await;

        // 验证文件在磁盘上完全未被篡改
        let current_content = std::fs::read_to_string(&child_path).unwrap();
        assert_eq!(
            original_content, current_content,
            "Subflow file on disk must NOT be modified by debug session"
        );
    }

    #[tokio::test]
    async fn test_debug_snapshot_and_retry_current_node() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition::default(),
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let script_node = Node {
            uuid: "script-fail".to_string(),
            node_type: "script".to_string(),
            name: "脚本".to_string(),
            position: NodePosition::default(),
            data: NodeData::Script(flow_engine_core::node_config::ScriptNodeConfig {
                script: "throw new Error('Intentional Failure');".to_string(),
                timeout_ms: 5000,
                on_error: flow_engine_core::node_config::ScriptErrorStrategy::Fatal,
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let conn = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "start".to_string(),
            target_node_uuid: "script-fail".to_string(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id,
            name: "测试快照原地重试".to_string(),
            description: None,
            nodes: vec![start_node, script_node],
            connections: vec![conn],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec![],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        let mut got_error_pause = false;
        let mut retried = false;

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, reason, .. } => {
                            if let PauseReason::Error { message } = reason {
                                assert_eq!(node_uuid, "script-fail");
                                assert!(!message.is_empty());
                                got_error_pause = true;

                                // 修改节点配置为正确脚本并原地重试
                                let _ = command_sender.send(DebugCommand::ModifyNodeConfig {
                                    node_uuid: "script-fail".to_string(),
                                    config: serde_json::to_value(NodeData::Script(
                                        flow_engine_core::node_config::ScriptNodeConfig {
                                            script: "fn.variables.set('fixed', '42');".to_string(),
                                            timeout_ms: 5000,
                                            on_error: flow_engine_core::node_config::ScriptErrorStrategy::Recoverable,
                                            ..Default::default()
                                        },
                                    ))
                                    .unwrap(),
                                });
                                let _ = command_sender.send(DebugCommand::RetryCurrentNode);
                                retried = true;
                            }
                        }
                        DebugEvent::SessionEnded { report, .. } => {
                            let rep = report.expect("report should exist");
                            assert_eq!(rep.status, ExecutionStatus::Success);
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("test_debug_snapshot_and_retry_current_node timed out");
                }
            }
        }

        assert!(got_error_pause, "应捕获节点错误暂停");
        assert!(retried, "应成功触发原地重试");
        let result_state = handle.await.unwrap();
        assert_eq!(result_state.variables.get("fixed"), Some(&json!("42")));
    }

    #[tokio::test]
    async fn test_debug_run_from_snapshot() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition::default(),
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let script1 = Node {
            uuid: "script-1".to_string(),
            node_type: "script".to_string(),
            name: "脚本1".to_string(),
            position: NodePosition::default(),
            data: NodeData::Script(flow_engine_core::node_config::ScriptNodeConfig {
                script: "fn.variables.set('val', 100); return {};".to_string(),
                timeout_ms: 5000,
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let script2 = Node {
            uuid: "script-2".to_string(),
            node_type: "script".to_string(),
            name: "脚本2".to_string(),
            position: NodePosition::default(),
            data: NodeData::Script(flow_engine_core::node_config::ScriptNodeConfig {
                script: "fn.variables.set('val', 200); return {};".to_string(),
                timeout_ms: 5000,
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id,
            name: "测试从快照恢复".to_string(),
            description: None,
            nodes: vec![start_node, script1, script2],
            connections: vec![
                Connection {
                    id: "c1".to_string(),
                    source_node_uuid: "start".to_string(),
                    target_node_uuid: "script-1".to_string(),
                    source_port_id: "output".to_string(),
                    target_port_id: "input".to_string(),
                },
                Connection {
                    id: "c2".to_string(),
                    source_node_uuid: "script-1".to_string(),
                    target_node_uuid: "script-2".to_string(),
                    source_port_id: "output".to_string(),
                    target_port_id: "input".to_string(),
                },
            ],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec!["script-2".to_string()],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        let mut restored_snapshot = false;
        let mut paused_at_restored = false;

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { node_uuid, .. } => {
                            if node_uuid == "script-2" && !restored_snapshot {
                                // 在 script-2 暂停时从 script-1 快照恢复
                                let _ = command_sender.send(DebugCommand::RunFromSnapshot {
                                    node_uuid: "script-1".to_string(),
                                });
                            } else if node_uuid == "script-1" && restored_snapshot {
                                paused_at_restored = true;
                                // 恢复暂停后直接结束调试
                                let _ = command_sender.send(DebugCommand::EndDebug);
                            }
                        }
                        DebugEvent::SnapshotRestored { node_uuid, restored_variables, .. } => {
                            assert_eq!(node_uuid, "script-1");
                            assert!(restored_variables.get("val").is_none()); // script-1 执行前尚无 val 变量
                            restored_snapshot = true;
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("test_debug_run_from_snapshot timed out");
                }
            }
        }

        assert!(restored_snapshot, "应收到 SnapshotRestored 事件");
        assert!(paused_at_restored, "快照恢复后应在目标节点暂停");
        let _ = handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_debug_send_stream_frame() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition::default(),
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id,
            name: "测试长连接探针".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(
                    &flow_clone,
                    None,
                    vec!["start".to_string()],
                    None,
                    event_sender,
                    command_receiver,
                )
                .await
        });

        let mut error_received = false;

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { .. } => {
                            // 暂停时发送帧到不存在的连接
                            let _ = command_sender.send(DebugCommand::SendStreamFrame {
                                connection_id: "nonexistent-ws".to_string(),
                                payload: "hello probe".to_string(),
                                is_binary: false,
                            });
                        }
                        DebugEvent::Error { message } => {
                            if message.contains("nonexistent-ws") {
                                error_received = true;
                                let _ = command_sender.send(DebugCommand::EndDebug);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    panic!("test_debug_send_stream_frame timed out");
                }
            }
        }

        assert!(error_received, "应收到无法连接流的错误提示");
        let _ = handle.await.unwrap();
    }

    /// 快照差异必须同时覆盖新增、修改与删除三类变更
    #[test]
    fn test_diff_variable_changes_covers_add_update_delete() {
        let before: HashMap<String, serde_json::Value> = HashMap::from([
            ("kept".to_string(), json!("same")),
            ("changed".to_string(), json!("old")),
            ("removed".to_string(), json!(123)),
        ]);
        let after: HashMap<String, serde_json::Value> = HashMap::from([
            ("kept".to_string(), json!("same")),
            ("changed".to_string(), json!("new")),
            ("added".to_string(), json!(true)),
        ]);

        let mut changes = Vec::new();
        diff_variable_changes(&before, &after, &mut changes);

        let by_key: HashMap<&str, &flow_engine_core::report::VariableChange> =
            changes.iter().map(|c| (c.key.as_str(), c)).collect();

        assert_eq!(changes.len(), 3, "应为 新增/修改/删除 各一条: {changes:?}");
        assert_eq!(by_key["added"].old_value, None);
        assert_eq!(by_key["added"].new_value, json!(true));
        assert_eq!(by_key["changed"].old_value, Some(json!("old")));
        assert_eq!(by_key["changed"].new_value, json!("new"));
        assert_eq!(by_key["removed"].old_value, Some(json!(123)));
        assert_eq!(by_key["removed"].new_value, serde_json::Value::Null);
    }

    /// 构造 [entry] -> [script(必然报错)] 的流程，用于错误暂停路径的回归测试
    fn fatal_error_flow() -> Flow {
        let start_node = Node {
            uuid: "start".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition::default(),
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let script_node = Node {
            uuid: "script-fail".to_string(),
            node_type: "script".to_string(),
            name: "脚本".to_string(),
            position: NodePosition::default(),
            data: NodeData::Script(flow_engine_core::node_config::ScriptNodeConfig {
                script: "throw new Error('boom');".to_string(),
                timeout_ms: 5000,
                on_error: flow_engine_core::node_config::ScriptErrorStrategy::Fatal,
                ..Default::default()
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "错误暂停回归流程".to_string(),
            description: None,
            nodes: vec![start_node, script_node],
            connections: vec![Connection {
                id: "c1".to_string(),
                source_node_uuid: "start".to_string(),
                target_node_uuid: "script-fail".to_string(),
                source_port_id: "output".to_string(),
                target_port_id: "input".to_string(),
            }],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        }
    }

    /// 错误暂停时按“结束调试”必须只结束一次会话，且原因保持 UserAbort
    #[tokio::test]
    async fn test_error_pause_end_debug_ends_session_once() {
        let flow = fatal_error_flow();
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(&flow_clone, None, vec![], None, event_sender, command_receiver)
                .await
        });

        let mut session_ended_reasons: Vec<EndReason> = Vec::new();
        let mut sent_end = false;

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { reason, .. } => {
                            if matches!(reason, PauseReason::Error { .. }) && !sent_end {
                                sent_end = true;
                                let _ = command_sender.send(DebugCommand::EndDebug);
                            }
                        }
                        DebugEvent::SessionEnded { reason, .. } => {
                            session_ended_reasons.push(reason);
                            if session_ended_reasons.len() > 1 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                    if !session_ended_reasons.is_empty() {
                        break;
                    }
                }
            }
        }

        assert_eq!(
            session_ended_reasons.len(),
            1,
            "错误暂停下结束调试必须只发出一次 SessionEnded，实际: {:?}",
            session_ended_reasons
        );
        assert_eq!(
            session_ended_reasons[0],
            EndReason::UserAbort,
            "结束原因必须保持 UserAbort，不能被改写成 Error"
        );
        let _ = handle.await;
    }

    /// 错误暂停时请求不存在的快照必须保持暂停，不能顺势结束整个会话
    #[tokio::test]
    async fn test_error_pause_run_from_missing_snapshot_keeps_session() {
        let flow = fatal_error_flow();
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(&flow_clone, None, vec![], None, event_sender, command_receiver)
                .await
        });

        let mut got_missing_snapshot_error = false;
        let mut session_ended_before_abort = false;
        let mut sent_restore = false;

        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::PausedAtNode { reason, .. } => {
                            if matches!(reason, PauseReason::Error { .. }) && !sent_restore {
                                sent_restore = true;
                                let _ = command_sender.send(DebugCommand::RunFromSnapshot {
                                    node_uuid: "does-not-exist".to_string(),
                                });
                            }
                        }
                        DebugEvent::Error { message } => {
                            if message.contains("Snapshot not found") {
                                got_missing_snapshot_error = true;
                                // 会话必须仍在：此时结束调试应得到 UserAbort 而不是早就结束
                                let _ = command_sender.send(DebugCommand::EndDebug);
                            }
                        }
                        DebugEvent::SessionEnded { .. } => {
                            session_ended_before_abort = !got_missing_snapshot_error;
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => break,
            }
        }

        assert!(
            got_missing_snapshot_error,
            "缺失快照应收到 Snapshot not found 提示"
        );
        assert!(
            !session_ended_before_abort,
            "缺失快照不得终止会话，会话应在 EndDebug 后才结束"
        );
        let _ = handle.await;
    }

    /// http:ingress 作为唯一入口的流程必须可以启动调试会话（与校验器/执行器口径一致）
    #[tokio::test]
    async fn test_debug_accepts_http_ingress_as_entry() {
        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "Ingress 入口流程".to_string(),
            description: None,
            nodes: vec![Node {
                uuid: "ingress-1".to_string(),
                node_type: "http:ingress".to_string(),
                name: "Ingress".to_string(),
                position: NodePosition::default(),
                data: NodeData::HttpIngress(flow_engine_core::node_config::HttpIngressConfig {
                    port: 9527,
                    path_pattern: "/*".to_string(),
                    methods: vec!["POST".to_string()],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            }],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (_command_sender, command_receiver) = mpsc::unbounded_channel();

        let executor = DebuggableFlowExecutor::new();
        let flow_clone = flow.clone();
        let handle = tokio::spawn(async move {
            executor
                .start_debug(&flow_clone, None, vec![], None, event_sender, command_receiver)
                .await
        });

        let mut missing_start_node = false;
        let mut executed_ingress = false;
        loop {
            tokio::select! {
                Some(event) = event_receiver.recv() => {
                    match event {
                        DebugEvent::NodeExecuted { entry } => {
                            if entry.node_uuid == "ingress-1" && entry.success {
                                executed_ingress = true;
                            }
                        }
                        DebugEvent::SessionEnded { reason, .. } => {
                            if let EndReason::Error { message } = reason {
                                if message.contains("missing a start node") {
                                    missing_start_node = true;
                                }
                            }
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => break,
            }
        }

        assert!(
            !missing_start_node,
            "http:ingress 入口流程不应报 Flow is missing a start node"
        );
        assert!(executed_ingress, "http:ingress 节点应作为起点被调试执行");
        let _ = handle.await;
    }
}
