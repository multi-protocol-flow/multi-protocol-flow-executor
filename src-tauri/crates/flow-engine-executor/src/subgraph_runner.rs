//! 流程子图执行模块 (Setup / Teardown 阶段执行器)
//!
//! 提供针对 `FlowSubGraph` 的拓扑排序与执行能力，
//! 用于执行流程的前置准备阶段 (Setup) 与后置收尾阶段 (Teardown)。
use crate::executor::{build_input_data_from_node_config, extract_error_from_output, FlowExecutor};
use crate::provider_builder::ProviderBuilderExt;
use flow_engine_core::context::{ExecutionContext, VariableStorage};
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::execution_event::ExecutionEvent;
use flow_engine_core::flow::FlowSubGraph;
use flow_engine_core::node::{ExecutionErrorStrategy, Node};
use flow_engine_core::report::{ExecutionStatus, NodeExecutionReport, StageExecutionReport};
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use uuid::Uuid;

/// 执行流程子图
///
/// # 参数
/// - `subgraph`: 要执行的流程子图
/// - `initial_vars`: 阶段初始变量
/// - `executor`: 主流程执行器引用
/// - `cancel_rx`: 可选的取消信号通道
///
/// # 返回
/// - `Ok((updated_variables, stage_report))`: 阶段执行报告与更新后的变量上下文
/// - `Err(error_message)`: 执行系统级错误
pub async fn execute_subgraph(
    subgraph: &FlowSubGraph,
    initial_vars: HashMap<String, serde_json::Value>,
    executor: &FlowExecutor,
    cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<(HashMap<String, serde_json::Value>, StageExecutionReport), String> {
    let start_time = std::time::Instant::now();

    if subgraph.is_empty() {
        return Ok((
            initial_vars,
            StageExecutionReport {
                status: ExecutionStatus::Success,
                duration_ms: 0,
                node_reports: Vec::new(),
                error: None,
            },
        ));
    }

    // 1. 初始化变量存储与执行上下文
    let mut var_storage = VariableStorage::new();
    for (k, v) in &initial_vars {
        var_storage.set_variable(k.clone(), v.clone(), String::new());
    }

    let mut context = ExecutionContext {
        flow_uuid: String::new(),
        execution_id: Uuid::new_v4().to_string(),
        current_node_uuid: None,
        variables: var_storage,
        node_reports: vec![],
        flow_trigger_time: flow_engine_core::utils::unix_timestamp_ms(),
        log_sender: None,
        flow_file_path: executor.flow_file_path().cloned(),
        ws_connection_urls: HashMap::new(),
        ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        last_node_output: None,
        debug_event_sender: executor.debug_event_sender().cloned(),
        execution_event_sender: executor.execution_event_sender().cloned(),
        cancel_receiver: cancel_rx.or_else(|| executor.cancel_receiver().cloned()),
        providers: HashMap::new(),
        current_node_plugin_data: None,
        call_stack: executor.call_stack().clone(),
        current_sub_flow_report: None,
    }
    .with_default_providers();

    context.set_provider("node_registry", executor.registry().clone());
    for (name, provider) in executor.extra_providers() {
        context.set_provider(name, provider.clone());
    }

    // 2. 构建节点索引与拓扑出入度映射
    let node_map: HashMap<String, &Node> = subgraph
        .nodes
        .iter()
        .map(|node| (node.uuid.clone(), node))
        .collect();

    let mut connection_map: HashMap<String, Vec<String>> = HashMap::new();
    for conn in &subgraph.connections {
        connection_map
            .entry(conn.source_node_uuid.clone())
            .or_default()
            .push(conn.target_node_uuid.clone());
    }

    let mut in_degrees: HashMap<String, usize> = HashMap::new();
    for node in &subgraph.nodes {
        in_degrees.insert(node.uuid.clone(), 0);
    }
    for conn in &subgraph.connections {
        if let Some(deg) = in_degrees.get_mut(&conn.target_node_uuid) {
            *deg += 1;
        }
    }

    // 3. 寻找入度为 0 的起始节点依次推进
    // 规则：
    // - 若存在显式 entry 节点，entry 节点强制最先执行；
    // - 其它入度为 0 的根节点（如多个未连线孤立节点），按画布视觉位置自上而下 (y 升序)、自左而右 (x 升序) 确定性执行。
    let mut root_nodes: Vec<&Node> = subgraph
        .nodes
        .iter()
        .filter(|n| in_degrees.get(&n.uuid).copied() == Some(0))
        .collect();

    root_nodes.sort_by(|a, b| {
        let is_a_entry = a.node_type == "entry";
        let is_b_entry = b.node_type == "entry";
        if is_a_entry != is_b_entry {
            return is_b_entry.cmp(&is_a_entry);
        }
        let y_diff = (a.position.y - b.position.y).abs();
        if y_diff < 15.0 {
            a.position
                .x
                .partial_cmp(&b.position.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.position
                .y
                .partial_cmp(&b.position.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let mut ready_nodes: VecDeque<String> =
        root_nodes.into_iter().map(|n| n.uuid.clone()).collect();
    if ready_nodes.is_empty() && !subgraph.nodes.is_empty() {
        ready_nodes.push_back(subgraph.nodes[0].uuid.clone());
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut node_reports: Vec<NodeExecutionReport> = Vec::new();
    let mut stage_status = ExecutionStatus::Success;
    let mut stage_error: Option<ErrorDetail> = None;

    // 4. 依次执行节点
    while let Some(current_node_id) = ready_nodes.pop_front() {
        if visited.contains(&current_node_id) {
            continue;
        }

        if context.is_cancelled() {
            log::info!("[Subgraph] Execution cancelled on node {}", current_node_id);
            stage_status = ExecutionStatus::Cancelled;
            break;
        }

        let node = match node_map.get(&current_node_id) {
            Some(n) => *n,
            None => continue,
        };

        visited.insert(current_node_id.clone());
        context.current_node_uuid = Some(current_node_id.clone());

        context
            .emit_execution_event(ExecutionEvent::NodeStarted {
                node_uuid: node.uuid.clone(),
                node_name: node.name.clone(),
                node_type: node.node_type.clone(),
                parent_node_uuid: executor.parent_node_uuid().clone(),
            })
            .await;

        let node_start_time = tokio::time::Instant::now();
        let node_start_ms = flow_engine_core::utils::unix_timestamp_ms();

        context.variables.clear_pending_changes();
        let var_version_before = context.variables.version();

        let node_timeout_ms = node
            .data
            .execution_timeout_ms()
            .map(|ms| std::cmp::min(ms, 3600000));

        let node_result = match (context.cancel_receiver.is_some(), node_timeout_ms) {
            (true, Some(ms)) => {
                let Some(mut rx) = context.cancel_receiver.clone() else {
                    unreachable!()
                };
                let timeout_dur = std::time::Duration::from_millis(ms);
                let execute =
                    executor.execute_node(node, &mut context, &subgraph.connections, &node_map);
                let wrapped = tokio::time::timeout(timeout_dur, execute);
                tokio::select! {
                    res = wrapped => {
                        match res {
                            Ok(inner) => Some(inner),
                            Err(_) => {
                                Some(Err(ErrorDetail::new(ErrorKind::Timeout, format!("Node execution timeout ({}ms)", ms))
                                    .with_error_code_typed(ErrorCode::E_TIMEOUT)
                                    .with_i18n_arg("ms", ms.to_string())
                                    .with_i18n_arg("operation", node.name.clone())))
                            }
                        }
                    }
                    _ = rx.changed() => None,
                }
            }
            (true, None) => {
                let Some(mut rx) = context.cancel_receiver.clone() else {
                    unreachable!()
                };
                tokio::select! {
                    res = executor.execute_node(node, &mut context, &subgraph.connections, &node_map) => Some(res),
                    _ = rx.changed() => None,
                }
            }
            (false, Some(ms)) => {
                let timeout_dur = std::time::Duration::from_millis(ms);
                match tokio::time::timeout(
                    timeout_dur,
                    executor.execute_node(node, &mut context, &subgraph.connections, &node_map),
                )
                .await
                {
                    Ok(res) => Some(res),
                    Err(_) => Some(Err(ErrorDetail::new(
                        ErrorKind::Timeout,
                        format!("Node execution timeout ({}ms)", ms),
                    )
                    .with_error_code_typed(ErrorCode::E_TIMEOUT)
                    .with_i18n_arg("ms", ms.to_string())
                    .with_i18n_arg("operation", node.name.clone()))),
                }
            }
            (false, None) => Some(
                executor
                    .execute_node(node, &mut context, &subgraph.connections, &node_map)
                    .await,
            ),
        };

        let node_result = match node_result {
            Some(res) => res,
            None => {
                stage_status = ExecutionStatus::Cancelled;
                break;
            }
        };

        let node_end_ms = flow_engine_core::utils::unix_timestamp_ms();
        let node_duration = node_start_time.elapsed().as_millis() as u64;

        match node_result {
            Ok((next_node_ids, output_data)) => {
                let type_id = node.data.type_name();
                let plugin = executor.registry().get(&type_id);
                let used_port_id = plugin.and_then(|p| {
                    let port = p.resolve_port(&output_data);
                    (!port.is_empty()).then_some(port)
                });

                let input_data = build_input_data_from_node_config(executor.registry(), node);
                if let Some(data) = &output_data {
                    context.last_node_output = Some(data.clone());
                }

                let variable_changes = if context.variables.version() == var_version_before {
                    None
                } else {
                    Some(context.variables.take_pending_changes())
                };

                let raw_config = serde_json::to_value(&node.data).unwrap_or(json!(null));
                let resolved_config = Some(flow_engine_core::executor::resolve_variables_in_json(
                    &raw_config,
                    &context,
                ));

                // 与主流程执行器保持同一约定：协议执行器在 Ok 路径下通过
                // `output_data["error"]` 传递可恢复错误（端口 "false"），
                // 此时节点必须记为 Failed，否则 Setup/Teardown 的失败会被静默吞掉。
                let node_failed = used_port_id.as_deref() == Some("false");
                let node_error = extract_error_from_output(&output_data);

                let report = NodeExecutionReport {
                    node_uuid: node.uuid.clone(),
                    node_type: node.node_type.clone(),
                    node_name: node.name.clone(),
                    start_time: node_start_ms,
                    end_time: node_end_ms,
                    duration_ms: node_duration,
                    status: if node_failed {
                        ExecutionStatus::Failed
                    } else {
                        ExecutionStatus::Success
                    },
                    input_data,
                    output_data,
                    used_port_id,
                    error: node_error.clone(),
                    resolved_config,
                    variable_changes,
                    skip_reason: None,
                    wait_duration_ms: None,
                    plugin_data: context.current_node_plugin_data.take(),
                    sub_flow_report: context.current_sub_flow_report.take(),
                    is_detail_stripped: None,
                };

                context
                    .emit_execution_event(ExecutionEvent::NodeCompleted {
                        report: Box::new(report.clone()),
                        parent_node_uuid: executor.parent_node_uuid().clone(),
                    })
                    .await;

                node_reports.push(report);

                // 推进后续节点
                let is_port_routing = plugin.map(|p| p.is_port_routing()).unwrap_or(true);

                if node_failed {
                    let on_error = node.on_error.as_ref();
                    if on_error == Some(&ExecutionErrorStrategy::Ignore) {
                        // 忽略错误：继续走 true 端口分支
                        continue;
                    }
                    if on_error == Some(&ExecutionErrorStrategy::AbortFlow) {
                        stage_status = ExecutionStatus::Failed;
                        stage_error = node_error;
                        break;
                    }
                    // RouteToFalse（默认）：false 端口无连接则阶段失败
                    let false_targets: Vec<String> = subgraph
                        .connections
                        .iter()
                        .filter(|c| {
                            c.source_node_uuid == current_node_id && c.source_port_id == "false"
                        })
                        .map(|c| c.target_node_uuid.clone())
                        .collect();
                    if false_targets.is_empty() {
                        stage_status = ExecutionStatus::Failed;
                        stage_error = node_error;
                        break;
                    }
                    for target in false_targets.into_iter().rev() {
                        ready_nodes.push_front(target);
                    }
                    continue;
                }

                if !next_node_ids.is_empty() {
                    for next_id in next_node_ids.into_iter().rev() {
                        ready_nodes.push_front(next_id);
                    }
                } else if !is_port_routing {
                    if let Some(targets) = connection_map.get(&current_node_id) {
                        for target in targets.iter().rev() {
                            ready_nodes.push_front(target.clone());
                        }
                    }
                }
            }
            Err(error) => {
                let raw_config = serde_json::to_value(&node.data).unwrap_or(json!(null));
                let resolved_config = Some(flow_engine_core::executor::resolve_variables_in_json(
                    &raw_config,
                    &context,
                ));

                let report = NodeExecutionReport {
                    node_uuid: node.uuid.clone(),
                    node_type: node.node_type.clone(),
                    node_name: node.name.clone(),
                    start_time: node_start_ms,
                    end_time: node_end_ms,
                    duration_ms: node_duration,
                    status: ExecutionStatus::Failed,
                    input_data: build_input_data_from_node_config(executor.registry(), node),
                    output_data: None,
                    used_port_id: Some("false".to_string()),
                    error: Some(error.clone()),
                    resolved_config,
                    variable_changes: None,
                    skip_reason: None,
                    wait_duration_ms: None,
                    plugin_data: context.current_node_plugin_data.take(),
                    sub_flow_report: context.current_sub_flow_report.take(),
                    is_detail_stripped: None,
                };

                context
                    .emit_execution_event(ExecutionEvent::NodeCompleted {
                        report: Box::new(report.clone()),
                        parent_node_uuid: executor.parent_node_uuid().clone(),
                    })
                    .await;

                node_reports.push(report);

                let on_error = node.on_error.as_ref();
                if on_error == Some(&ExecutionErrorStrategy::Ignore) {
                    continue;
                } else if on_error == Some(&ExecutionErrorStrategy::AbortFlow) {
                    stage_status = ExecutionStatus::Failed;
                    stage_error = Some(error);
                    break;
                } else {
                    // RouteToFalse
                    let false_targets: Vec<String> = subgraph
                        .connections
                        .iter()
                        .filter(|c| {
                            c.source_node_uuid == current_node_id && c.source_port_id == "false"
                        })
                        .map(|c| c.target_node_uuid.clone())
                        .collect();

                    if false_targets.is_empty() {
                        stage_status = ExecutionStatus::Failed;
                        stage_error = Some(error);
                        break;
                    } else {
                        for target in false_targets.into_iter().rev() {
                            ready_nodes.push_front(target);
                        }
                    }
                }
            }
        }
    }

    let duration_ms = start_time.elapsed().as_millis() as u64;
    let final_vars = context.variables.get_merged_variables();

    Ok((
        final_vars,
        StageExecutionReport {
            status: stage_status,
            duration_ms,
            node_reports,
            error: stage_error,
        },
    ))
}
