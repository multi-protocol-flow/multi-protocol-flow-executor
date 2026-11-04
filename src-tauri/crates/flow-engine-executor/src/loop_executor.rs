//! 循环节点执行器
//!
//! `flow:loop` 把内联子图（`LoopConfig::body`）当作循环体重复执行，支持三种来源：
//!
//! - `count`：固定次数
//! - `items`：遍历渲染后为 JSON 数组的表达式
//! - `until`：只按 `until` 表达式决定是否继续
//!
//! 每次迭代都从「父上下文变量的快照」重建 body 变量，因此 body 内的变量变更
//! **不会**泄漏到下一轮，也不会泄漏到主流程；需要带出循环的变量用
//! `output_mappings`（路径相对 `{"last": <最后一轮 body 末节点输出>}`）。
//!
//! 迭代预算、嵌套层数、超时与取消都集中在这里：
//! - 单次流程执行的累计迭代上限由 [`crate::loop_scope::LoopScope`] 共享
//! - 嵌套层数上限 [`crate::loop_scope::MAX_LOOP_NESTING`]
//! - `timeout_ms` 覆盖 body 执行 + 首轮等待 + 迭代间隔
//!
//! 所有退出路径都返回 `Ok`：`break_reason` 写在输出里，端口由
//! `LoopPlugin::resolve_port` 决定（`exhausted`/`until` → `done`，其余 → `false`）。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{LoopSource, NodeData};
use flow_engine_core::registry::NodeRegistry;
use flow_engine_core::report::{
    ExecutionStatus, FlowExecutionReport, IterationSummary, LoopReportSummary, NodeExecutionReport,
};
use flow_engine_core::statistics::FlowStatistics;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::executor::FlowExecutor;
use crate::loop_scope::{LoopScope, LOOP_SCOPE_PROVIDER, MAX_LOOP_NESTING};

/// 循环默认整体超时（毫秒）
const DEFAULT_TIMEOUT_MS: u64 = 600_000;

/// 循环节点执行器
#[derive(Debug, Default, Clone, Copy)]
pub struct LoopNodeExecutor;

impl LoopNodeExecutor {
    /// 创建执行器
    pub fn new() -> Self {
        Self
    }
}

/// 终止原因
///
/// 取值集合刻意不含 `break`：`flow:break` / `flow:continue`（文档 P2）尚未实现，
/// 不得为它预留分支。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BreakReason {
    /// 次数/集合耗尽
    Exhausted,
    /// `until` 命中
    Until,
    /// 迭代额度或 `max_iterations` 用尽
    Budget,
    /// 整体超时
    Timeout,
    /// 嵌套层数超限
    Depth,
    /// body 失败且 `break_on_error`
    Error,
    /// 被用户取消
    Cancelled,
}

impl BreakReason {
    fn as_str(self) -> &'static str {
        match self {
            BreakReason::Exhausted => "exhausted",
            BreakReason::Until => "until",
            BreakReason::Budget => "budget",
            BreakReason::Timeout => "timeout",
            BreakReason::Depth => "depth",
            BreakReason::Error => "error",
            BreakReason::Cancelled => "cancelled",
        }
    }

    /// 是否属于成功终止（走 `done` 端口）
    fn is_success(self) -> bool {
        matches!(self, BreakReason::Exhausted | BreakReason::Until)
    }
}

/// 距离 deadline 的剩余毫秒（已过期返回 0）
fn remaining_ms(deadline: Instant) -> u64 {
    deadline
        .saturating_duration_since(Instant::now())
        .as_millis() as u64
}

/// 依据来源判断是否还能再跑一轮
///
/// 返回 `None` 表示还能继续；`Some(reason)` 表示本轮结束后即可终止。
fn stop_reason(
    next_index: u32,
    source: &LoopSource,
    count: u32,
    items_len: usize,
    max_iterations: u32,
) -> Option<BreakReason> {
    // 先判来源耗尽：`count == max_iterations` 时应报 exhausted 而不是 budget
    match source {
        LoopSource::Count if next_index >= count => return Some(BreakReason::Exhausted),
        LoopSource::Items if (next_index as usize) >= items_len => {
            return Some(BreakReason::Exhausted)
        }
        _ => {}
    }
    if next_index >= max_iterations {
        return Some(BreakReason::Budget);
    }
    None
}

/// 从父上下文构建 body 子执行器
fn build_host(
    context: &ExecutionContext,
    node: &Node,
    registry: Option<Arc<NodeRegistry>>,
) -> FlowExecutor {
    let mut host = FlowExecutor::new()
        .with_parent_node_uuid(node.uuid.clone())
        .with_call_stack(context.call_stack.clone())
        .with_extra_providers(context.providers.clone());
    if let Some(registry) = registry {
        host = host.with_registry(registry);
    }
    if let Some(rx) = context.cancel_receiver.clone() {
        host = host.with_cancel_receiver(rx);
    }
    if let Some(sender) = context.debug_event_sender.clone() {
        host = host.with_debug_event_sender(sender);
    }
    if let Some(sender) = context.execution_event_sender.clone() {
        host = host.with_execution_event_sender(sender);
    }
    if let Some(path) = context.flow_file_path.clone() {
        host = host.with_flow_file_path(path);
    }
    host
}

#[async_trait::async_trait]
impl NodeExecutor for LoopNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<Value>), ErrorDetail> {
        let NodeData::Loop(cfg) = &node.data else {
            return Err(ErrorDetail::new(
                ErrorKind::Configuration,
                format!("flow:loop node '{}' missing LoopConfig", node.name),
            )
            .with_error_code_typed(ErrorCode::E_CONFIG_ERROR));
        };

        let started = Instant::now();
        let start_ms = flow_engine_core::utils::unix_timestamp_ms();

        // ---- 1. 作用域与嵌套层数 ----
        let parent = context.get_provider::<LoopScope>(LOOP_SCOPE_PROVIDER);
        let parent_depth = parent.as_ref().map(|s| s.depth).unwrap_or(0);

        // ---- 2. 解析来源 ----
        let (mut items, mut count, mut until_expr) = (Vec::new(), 0u32, None);
        let mut planned_total = cfg.max_iterations;
        match cfg.source {
            LoopSource::Items => {
                let Some(raw) = cfg
                    .items
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                else {
                    return Err(ErrorDetail::new(
                        ErrorKind::Configuration,
                        format!(
                            "flow:loop node '{}' requires a non-empty items expression",
                            node.name
                        ),
                    )
                    .with_error_code_typed(ErrorCode::E_CONFIG_ERROR));
                };
                let rendered = flow_engine_core::executor::replace_variables(raw, context)
                    .map_err(|e| {
                        ErrorDetail::new(
                            ErrorKind::Configuration,
                            format!("Loop items '{}' render failed: {}", raw, e),
                        )
                        .with_error_code_typed(ErrorCode::E_CONFIG_ERROR)
                    })?;
                let parsed: Value = serde_json::from_str(&rendered).map_err(|e| {
                    ErrorDetail::new(
                        ErrorKind::Configuration,
                        format!("Loop items '{}' is not a valid JSON array: {}", rendered, e),
                    )
                    .with_error_code_typed(ErrorCode::E_CONFIG_ERROR)
                })?;
                let Value::Array(arr) = parsed else {
                    return Err(ErrorDetail::new(
                        ErrorKind::Configuration,
                        format!("Loop items '{}' is not a JSON array", rendered),
                    )
                    .with_error_code_typed(ErrorCode::E_CONFIG_ERROR));
                };
                planned_total = arr.len() as u32;
                items = arr;
            }
            LoopSource::Count => {
                count = cfg.count.unwrap_or(0);
                planned_total = count;
            }
            LoopSource::Until => {
                let Some(expr) = cfg
                    .until
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                else {
                    return Err(ErrorDetail::new(
                        ErrorKind::Configuration,
                        format!(
                            "flow:loop node '{}' requires a non-empty until expression",
                            node.name
                        ),
                    )
                    .with_error_code_typed(ErrorCode::E_CONFIG_ERROR));
                };
                until_expr = Some(expr.to_string());
            }
        }
        if cfg.until.is_some() && until_expr.is_none() {
            until_expr = cfg
                .until
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
        }

        // ---- 3. 循环状态 ----
        let deadline =
            Instant::now() + Duration::from_millis(cfg.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
        let mut break_reason = BreakReason::Exhausted;
        let mut error_message: Option<String> = None;
        let mut summaries: Vec<IterationSummary> = Vec::new();
        let mut kept_reports: Vec<NodeExecutionReport> = Vec::new();
        let mut truncated: u32 = 0;
        let mut index: u32 = 0;
        let mut last_output: Option<Value> = None;

        // body 变量基座：父上下文快照（每轮以它为基础重建，实现迭代隔离）
        let shared = context.variables.get_merged_variables();

        let mut skip_iterations = false;
        if parent_depth >= MAX_LOOP_NESTING {
            break_reason = BreakReason::Depth;
            error_message = Some(format!(
                "Loop '{}' nesting depth {} exceeds {} levels",
                node.name, parent_depth, MAX_LOOP_NESTING
            ));
            skip_iterations = true;
        }

        // ---- 4. 首轮等待 ----
        let initial_delay = cfg.initial_delay_ms.unwrap_or(0);
        if !skip_iterations && initial_delay > 0 {
            let wait = initial_delay.min(remaining_ms(deadline));
            if wait > 0 && !context.wait_or_cancelled(wait).await {
                break_reason = BreakReason::Cancelled;
                error_message = Some("Loop cancelled by user".to_string());
                skip_iterations = true;
            } else if Instant::now() >= deadline {
                break_reason = BreakReason::Timeout;
                error_message = Some("Loop timed out before first iteration".to_string());
                skip_iterations = true;
            }
        }

        // ---- 5. 迭代 ----
        if !skip_iterations {
            'iterations: loop {
                if context.is_cancelled() {
                    break_reason = BreakReason::Cancelled;
                    error_message = Some("Loop cancelled by user".to_string());
                    break 'iterations;
                }
                if Instant::now() >= deadline {
                    break_reason = BreakReason::Timeout;
                    error_message = Some("Loop timed out".to_string());
                    break 'iterations;
                }
                if let Some(reason) =
                    stop_reason(index, &cfg.source, count, items.len(), cfg.max_iterations)
                {
                    break_reason = reason;
                    if reason == BreakReason::Budget {
                        error_message = Some(format!(
                            "Loop iteration limit ({}) reached",
                            cfg.max_iterations
                        ));
                    }
                    break 'iterations;
                }

                let item = items.get(index as usize).cloned().unwrap_or(Value::Null);
                let loop_json = json!({
                    "index": index,
                    "count": planned_total,
                    "first": index == 0,
                    "last": index + 1 == planned_total,
                    "item": item.clone(),
                });

                let mut vars = shared.clone();
                vars.insert("loop".to_string(), loop_json.clone());

                let child = parent
                    .as_ref()
                    .map(|scope| scope.child())
                    .unwrap_or_else(|| LoopScope::root().child());
                if !child.try_acquire() {
                    break_reason = BreakReason::Budget;
                    error_message = Some("Global loop iteration budget exhausted".to_string());
                    break 'iterations;
                }

                let host = build_host(
                    context,
                    node,
                    context.get_provider::<NodeRegistry>("node_registry"),
                );
                let result = crate::subgraph_runner::execute_subgraph(
                    &cfg.body,
                    vars,
                    &host,
                    context.cancel_receiver.clone(),
                    Some(Arc::new(child)),
                )
                .await;

                let stage_report = match result {
                    Ok((_, report)) => report,
                    Err(e) => {
                        return Err(ErrorDetail::execution_error(format!(
                            "Loop '{}' body execution failed: {}",
                            node.name, e
                        )));
                    }
                };

                let mut reports = stage_report.node_reports;
                for report in &mut reports {
                    report.iteration_index = Some(index);
                    report.iteration_of = Some(node.uuid.clone());
                }
                let iteration_output = reports.iter().rev().find_map(|r| r.output_data.clone());
                if iteration_output.is_some() {
                    last_output = iteration_output;
                }

                let failed = stage_report.status != ExecutionStatus::Success;
                summaries.push(IterationSummary {
                    index,
                    status: stage_report.status.clone(),
                    duration_ms: stage_report.duration_ms,
                    wait_ms: 0,
                    error: stage_report.error.clone(),
                });

                let keep_detail = index < cfg.report.detail_iterations
                    || (failed && cfg.report.always_keep_failed);
                if keep_detail {
                    kept_reports.extend(reports);
                } else {
                    truncated += 1;
                }

                if failed && cfg.break_on_error {
                    break_reason = BreakReason::Error;
                    error_message = Some(
                        stage_report
                            .error
                            .map(|e| e.message)
                            .unwrap_or_else(|| format!("Loop '{}' body failed", node.name)),
                    );
                    index += 1;
                    break 'iterations;
                }

                index += 1;

                // ---- until 求值 ----
                if let Some(expr) = &until_expr {
                    let mut eval_vars = shared.clone();
                    eval_vars.insert("loop".to_string(), loop_json);
                    eval_vars.insert(
                        "last".to_string(),
                        last_output.clone().unwrap_or(Value::Null),
                    );
                    let eval_input = Value::Object(eval_vars.into_iter().collect());
                    match crate::expression::evaluate_boolean_expression(expr, &eval_input, None) {
                        Ok(true) => {
                            break_reason = BreakReason::Until;
                            break 'iterations;
                        }
                        Ok(false) => {}
                        Err(msg) => {
                            break_reason = BreakReason::Error;
                            error_message = Some(format!(
                                "Loop until expression failed: {} (expression: {})",
                                msg, expr
                            ));
                            break 'iterations;
                        }
                    }
                }

                // ---- 是否还需要下一轮 ----
                if let Some(reason) =
                    stop_reason(index, &cfg.source, count, items.len(), cfg.max_iterations)
                {
                    break_reason = reason;
                    if reason == BreakReason::Budget {
                        error_message = Some(format!(
                            "Loop iteration limit ({}) reached",
                            cfg.max_iterations
                        ));
                    }
                    break 'iterations;
                }

                // ---- 迭代间隔（含退避）----
                let base = cfg.interval_ms.unwrap_or(0) as f64;
                if base > 0.0 {
                    let backoff = cfg.interval_backoff.unwrap_or(1.0).max(1.0);
                    let cap = cfg.interval_max_ms.unwrap_or(10_000) as f64;
                    let mut wait = (base * backoff.powi(index as i32)).min(cap) as u64;
                    let left = remaining_ms(deadline);
                    if left == 0 {
                        break_reason = BreakReason::Timeout;
                        error_message = Some("Loop timed out".to_string());
                        break 'iterations;
                    }
                    wait = wait.min(left);
                    if !context.wait_or_cancelled(wait).await {
                        break_reason = BreakReason::Cancelled;
                        error_message = Some("Loop cancelled by user".to_string());
                        break 'iterations;
                    }
                    if let Some(summary) = summaries.last_mut() {
                        summary.wait_ms = wait;
                    }
                    if Instant::now() >= deadline {
                        break_reason = BreakReason::Timeout;
                        error_message = Some("Loop timed out".to_string());
                        break 'iterations;
                    }
                }
            }
        }

        // ---- 6. 变量导出 ----
        flow_engine_core::utils::apply_output_mappings(
            &json!({ "last": last_output.clone().unwrap_or(Value::Null) }),
            &cfg.output_mappings,
            context,
            &node.uuid,
        );

        // ---- 7. 报告回传 ----
        let total_body_nodes = cfg.body.nodes.len() as u32;
        let mut body_report = FlowExecutionReport {
            uuid: Uuid::new_v4().to_string(),
            flow_uuid: context.flow_uuid.clone(),
            flow_name: format!("{} body", node.name),
            start_time: start_ms,
            end_time: flow_engine_core::utils::unix_timestamp_ms(),
            duration_ms: started.elapsed().as_millis() as u64,
            total_nodes: total_body_nodes,
            executed_count: kept_reports.len() as u32,
            status: if break_reason.is_success() {
                ExecutionStatus::Success
            } else if break_reason == BreakReason::Cancelled {
                ExecutionStatus::Cancelled
            } else {
                ExecutionStatus::Failed
            },
            node_reports: kept_reports,
            ..Default::default()
        };
        body_report.statistics =
            FlowStatistics::from_reports(&body_report.node_reports, total_body_nodes);
        body_report.execution_path = body_report
            .node_reports
            .iter()
            .map(|r| r.node_uuid.clone())
            .collect();
        if !break_reason.is_success() {
            body_report.flow_error = error_message.as_ref().map(|msg| {
                ErrorDetail::new(ErrorKind::Execution, msg.clone())
                    .with_error_code_typed(ErrorCode::E_EXECUTION_ERROR)
            });
        }
        context.current_sub_flow_report = Some(Box::new(body_report));
        context.current_loop_report = Some(LoopReportSummary {
            summaries,
            truncated_iterations: truncated,
        });

        // ---- 8. 输出与端口 ----
        let port = if break_reason.is_success() {
            "done"
        } else {
            "false"
        };
        let mut output = json!({
            "iterations": index,
            "break_reason": break_reason.as_str(),
            "last": last_output,
        });
        if let Some(msg) = &error_message {
            output["error"] = json!(msg);
        }

        let targets: Vec<String> = connections
            .iter()
            .filter(|c| c.source_node_uuid == node.uuid && c.source_port_id == port)
            .map(|c| c.target_node_uuid.clone())
            .collect();

        Ok((targets, Some(output)))
    }
}
