//! `flow:loop` 执行语义的集成测试
//!
//! 全部通过真实的 [`FlowExecutor::execute_flow`] 跑小流程（`entry → flow:loop → end`），
//! body 只使用 `flow:delay`（确定性、无网络）或 `script`（变量作用域隔离）。

use flow_engine_core::connection::Connection;
use flow_engine_core::flow::{Flow, FlowSubGraph};
use flow_engine_core::node::{Node, NodePosition};
use flow_engine_core::node_config::{LoopConfig, LoopReportConfig, LoopSource, NodeData};
use flow_engine_core::report::{ExecutionStatus, NodeExecutionReport};
use serde_json::{json, Value};
use std::sync::Arc;

use flow_engine_executor::executor::{ExecutionResult, FlowExecutor};

fn node(uuid: &str, node_type: &str, data: NodeData) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: node_type.to_string(),
        name: uuid.to_string(),
        position: NodePosition::default(),
        data,
        ..Default::default()
    }
}

fn conn(id: &str, from: &str, port: &str, to: &str) -> Connection {
    Connection {
        id: id.to_string(),
        source_node_uuid: from.to_string(),
        target_node_uuid: to.to_string(),
        source_port_id: port.to_string(),
        target_port_id: "in".to_string(),
    }
}

/// 只有 body 的循环配置
fn loop_config(source: LoopSource) -> LoopConfig {
    LoopConfig {
        source,
        max_iterations: 100,
        report: LoopReportConfig {
            detail_iterations: 20,
            always_keep_failed: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// 一个 body：单个 `flow:delay` 节点（0 毫秒）
fn delay_body() -> FlowSubGraph {
    FlowSubGraph::new(
        vec![node(
            "body-delay",
            flow_engine_core::node_types::DELAY,
            NodeData::delay("0"),
        )],
        vec![],
    )
}

/// 构造 `entry → loop → (done|false) → end` 的主流程
fn build_flow(loop_cfg: LoopConfig) -> Flow {
    let entry = node(
        "entry",
        flow_engine_core::node_types::ENTRY,
        NodeData::entry(),
    );
    let loop_node = node(
        "loop",
        flow_engine_core::node_types::LOOP,
        NodeData::loop_node(loop_cfg),
    );
    let end = node("end", flow_engine_core::node_types::END, NodeData::end());
    Flow {
        uuid: "flow-loop-test".to_string(),
        name: "loop test".to_string(),
        nodes: vec![entry.clone(), loop_node.clone(), end.clone()],
        connections: vec![
            conn("c1", &entry.uuid, "out", &loop_node.uuid),
            conn("c2", &loop_node.uuid, "done", &end.uuid),
            conn("c3", &loop_node.uuid, "false", &end.uuid),
        ],
        ..Default::default()
    }
}

async fn run(flow: &Flow) -> ExecutionResult {
    FlowExecutor::new()
        .execute_flow(flow, None, None, None)
        .await
}

fn loop_report(result: &ExecutionResult) -> NodeExecutionReport {
    result
        .node_reports
        .iter()
        .find(|r| r.node_type == flow_engine_core::node_types::LOOP)
        .cloned()
        .expect("loop node report")
}

fn output_of(report: &NodeExecutionReport) -> Value {
    report
        .output_data
        .clone()
        .expect("loop node should produce output")
}

#[tokio::test]
async fn loop_count_runs_body_n_times() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(3);
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let report = loop_report(&result);
    let output = output_of(&report);
    assert_eq!(output["iterations"], json!(3));
    assert_eq!(output["break_reason"], json!("exhausted"));
    assert_eq!(report.used_port_id.as_deref(), Some("done"));

    let body = report
        .sub_flow_report
        .expect("loop should carry body report");
    assert_eq!(body.node_reports.len(), 3);
    for (i, inner) in body.node_reports.iter().enumerate() {
        assert_eq!(inner.iteration_index, Some(i as u32));
        assert_eq!(inner.iteration_of.as_deref(), Some("loop"));
    }
    let summaries = report.iteration_summary.expect("iteration summaries");
    assert_eq!(summaries.len(), 3);
    assert_eq!(report.truncated_iterations, Some(0));
}

#[tokio::test]
async fn loop_items_until_hits_early() {
    let mut cfg = loop_config(LoopSource::Items);
    cfg.items = Some(r#"["a","b","c"]"#.to_string());
    cfg.until = Some(r#"loop.item == "b""#.to_string());
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let report = loop_report(&result);
    let output = output_of(&report);
    assert_eq!(output["iterations"], json!(2));
    assert_eq!(output["break_reason"], json!("until"));
    assert_eq!(report.used_port_id.as_deref(), Some("done"));
}

#[tokio::test]
async fn loop_items_empty_is_noop() {
    let mut cfg = loop_config(LoopSource::Items);
    cfg.items = Some("[]".to_string());
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let report = loop_report(&result);
    let output = output_of(&report);
    assert_eq!(output["iterations"], json!(0));
    assert_eq!(output["break_reason"], json!("exhausted"));
    let body = report.sub_flow_report.expect("body report");
    assert!(body.node_reports.is_empty());
}

#[tokio::test]
async fn loop_local_vars_do_not_leak() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(3);
    cfg.body = FlowSubGraph::new(
        vec![node(
            "body-script",
            flow_engine_core::node_types::SCRIPT,
            NodeData::script(
                "const p = fn.variables.has('probe') ? Number(fn.variables.get('probe')) + 1 : 1;\n\
                 fn.variables.set('probe', p);",
            ),
        )],
        vec![],
    );
    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    assert_eq!(body.node_reports.len(), 3);
    for inner in &body.node_reports {
        let changes = inner
            .variable_changes
            .as_ref()
            .expect("script writes probe each round");
        let probe = changes
            .iter()
            .find(|c| c.key == "probe")
            .expect("probe change");
        assert!(
            probe.old_value.is_none(),
            "iteration {:?} saw a leaked probe value: {:?}",
            inner.iteration_index,
            probe.old_value
        );
        assert_eq!(probe.new_value, json!(1));
    }
}

#[tokio::test]
async fn loop_body_without_entry_runs_all_roots() {
    // 循环体没有 entry：入度为 0 的节点各自作为起点，两轮 × 2 个节点
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(2);
    cfg.body = FlowSubGraph::new(
        vec![
            node("root-a", flow_engine_core::node_types::DELAY, NodeData::delay("0")),
            node("root-b", flow_engine_core::node_types::DELAY, NodeData::delay("0")),
            // root-a 的后继：根节点的后续节点照常推进
            node("leaf-a", flow_engine_core::node_types::DELAY, NodeData::delay("0")),
        ],
        vec![conn("body-c1", "root-a", "out", "leaf-a")],
    );
    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    assert_eq!(body.node_reports.len(), 6, "3 body nodes x 2 iterations");
    let per_iteration: Vec<usize> = (0..2u32)
        .map(|i| {
            body.node_reports
                .iter()
                .filter(|r| r.iteration_index == Some(i))
                .count()
        })
        .collect();
    assert_eq!(per_iteration, vec![3, 3]);
}

#[tokio::test]
async fn loop_body_entry_runs_first_and_roots_follow_position() {
    // entry 存在时总是最先执行；两个未连线的根节点按画布位置（上→下）排序
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(1);
    let mut entry_node = node(
        "body-entry",
        flow_engine_core::node_types::ENTRY,
        NodeData::entry(),
    );
    entry_node.position = NodePosition { x: 0.0, y: 100.0 };
    let mut lower = node(
        "root-lower",
        flow_engine_core::node_types::DELAY,
        NodeData::delay("0"),
    );
    lower.position = NodePosition { x: 0.0, y: 200.0 };
    let mut upper = node(
        "root-upper",
        flow_engine_core::node_types::DELAY,
        NodeData::delay("0"),
    );
    upper.position = NodePosition { x: 0.0, y: 0.0 };
    cfg.body = FlowSubGraph::new(vec![lower, entry_node, upper], vec![]);

    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    let order: Vec<&str> = body
        .node_reports
        .iter()
        .map(|r| r.node_uuid.as_str())
        .collect();
    assert_eq!(
        order,
        vec!["body-entry", "root-upper", "root-lower"],
        "entry first, then roots by canvas position"
    );
}

#[tokio::test]
async fn slice_flow_of_body_node_runs_without_entry() {
    // 「测试此节点」会把 loop body 内的节点切成只含该节点的流程（没有 entry）：
    // 执行器必须回退到首个节点，而不是返回 "Flow missing start node"
    let flow = Flow {
        uuid: "slice-body-node".to_string(),
        name: "Test: body delay".to_string(),
        nodes: vec![node(
            "body-node",
            flow_engine_core::node_types::DELAY,
            NodeData::delay("0"),
        )],
        connections: vec![],
        ..Default::default()
    };

    let result = FlowExecutor::new()
        .execute_flow(
            &flow,
            Some(json!({
                "loop": {"index": 0, "count": 1, "first": true, "last": true, "item": null}
            })),
            None,
            None,
        )
        .await;

    assert!(result.success, "unexpected error: {:?}", result.error);
    assert_eq!(result.node_reports.len(), 1);
    assert_eq!(
        result.node_reports[0].node_uuid.as_str(),
        "body-node",
        "start node should fall back to the single node"
    );
}

#[tokio::test]
async fn loop_output_mappings_export_last() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(1);
    cfg.body = delay_body();
    cfg.output_mappings = vec![flow_engine_core::variable::OutputMapping {
        source: "$.last.requested_ms".to_string(),
        target: "final_delay".to_string(),
        description: None,
    }];
    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let flow_report = result.flow_report.expect("flow report");
    let final_vars = flow_report.final_variables.expect("final variables");
    assert_eq!(final_vars["final_delay"], json!(0));
}

#[tokio::test]
async fn loop_max_iterations_budget_exit() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(5);
    cfg.max_iterations = 2;
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;

    let report = loop_report(&result);
    let output = output_of(&report);
    assert_eq!(report.used_port_id.as_deref(), Some("false"));
    assert_eq!(output["break_reason"], json!("budget"));
    assert_eq!(report.status, ExecutionStatus::Failed);
}

#[tokio::test]
async fn loop_global_budget_spans_two_loops() {
    // 前两个 loop 各跑 600 轮会耗尽 1000 次全局额度
    let mut first = loop_config(LoopSource::Count);
    first.count = Some(600);
    first.max_iterations = 600;
    first.body = delay_body();
    let second = first.clone();

    let entry = node(
        "entry",
        flow_engine_core::node_types::ENTRY,
        NodeData::entry(),
    );
    let first_node = node(
        "loop1",
        flow_engine_core::node_types::LOOP,
        NodeData::loop_node(first),
    );
    let second_node = node(
        "loop2",
        flow_engine_core::node_types::LOOP,
        NodeData::loop_node(second),
    );
    let end = node("end", flow_engine_core::node_types::END, NodeData::end());
    let flow = Flow {
        uuid: "flow-loop-budget".to_string(),
        name: "loop budget".to_string(),
        nodes: vec![
            entry.clone(),
            first_node.clone(),
            second_node.clone(),
            end.clone(),
        ],
        connections: vec![
            conn("c1", &entry.uuid, "out", &first_node.uuid),
            conn("c2", &first_node.uuid, "done", &second_node.uuid),
            conn("c3", &first_node.uuid, "false", &second_node.uuid),
            conn("c4", &second_node.uuid, "done", &end.uuid),
            conn("c5", &second_node.uuid, "false", &end.uuid),
        ],
        ..Default::default()
    };

    let result = run(&flow).await;
    let reports: Vec<NodeExecutionReport> = result
        .node_reports
        .iter()
        .filter(|r| r.node_type == flow_engine_core::node_types::LOOP)
        .cloned()
        .collect();
    assert_eq!(reports.len(), 2);
    assert_eq!(output_of(&reports[0])["break_reason"], json!("exhausted"));
    assert_eq!(output_of(&reports[1])["break_reason"], json!("budget"));
}

#[tokio::test]
async fn loop_nesting_depth_rejected() {
    // 四层嵌套：外层 loop 的 body 内再套 3 层
    fn nested(depth: u32) -> FlowSubGraph {
        if depth == 0 {
            return delay_body();
        }
        let mut inner = loop_config(LoopSource::Count);
        inner.count = Some(1);
        inner.body = nested(depth - 1);
        FlowSubGraph::new(
            vec![node(
                &format!("nested-{depth}"),
                flow_engine_core::node_types::LOOP,
                NodeData::loop_node(inner),
            )],
            vec![],
        )
    }
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(1);
    cfg.body = nested(3);

    let flow = build_flow(cfg.clone());
    let registry = flow_engine_executor::default_registry::default_registry();
    let validation = flow_engine_executor::validator::FlowValidator::new()
        .validate_with_registry(&flow, &registry);
    assert!(
        validation
            .errors
            .iter()
            .any(|e| e.message.contains("nesting")),
        "expected nesting error, got {:?}",
        validation.errors
    );

    // 执行期同样以 depth 终止而不是栈溢出：最内层报 depth，外层因 body 失败而 error
    let result = run(&build_flow(cfg)).await;
    let report = loop_report(&result);
    assert_eq!(output_of(&report)["break_reason"], json!("error"));
    assert_eq!(report.used_port_id.as_deref(), Some("false"));
}

#[tokio::test]
async fn loop_break_on_error_true_vs_false() {
    fn failing_body() -> FlowSubGraph {
        // `fn.flow.stop()` 之外的确定性失败：script 语法错误
        FlowSubGraph::new(
            vec![node(
                "body-script",
                flow_engine_core::node_types::SCRIPT,
                NodeData::script("this is not valid javascript {{{"),
            )],
            vec![],
        )
    }

    let mut breaking = loop_config(LoopSource::Count);
    breaking.count = Some(3);
    breaking.body = failing_body();
    let result = run(&build_flow(breaking)).await;
    let report = loop_report(&result);
    assert_eq!(output_of(&report)["break_reason"], json!("error"));
    assert_eq!(report.used_port_id.as_deref(), Some("false"));

    let mut ignoring = loop_config(LoopSource::Count);
    ignoring.count = Some(3);
    ignoring.break_on_error = false;
    ignoring.body = failing_body();
    let result = run(&build_flow(ignoring)).await;
    let report = loop_report(&result);
    assert_eq!(output_of(&report)["break_reason"], json!("exhausted"));
    assert_eq!(report.used_port_id.as_deref(), Some("done"));
}

#[tokio::test]
async fn loop_report_trimming() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(5);
    cfg.report = LoopReportConfig {
        detail_iterations: 2,
        always_keep_failed: true,
        ..Default::default()
    };
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;

    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    assert_eq!(body.node_reports.len(), 2, "有界桶只保留 2 轮详情");
    let kept_indexes: Vec<u32> = body
        .node_reports
        .iter()
        .filter_map(|r| r.iteration_index)
        .collect();
    assert_eq!(
        kept_indexes,
        vec![3, 4],
        "满桶后丢弃最旧的轮次，保留最近两轮"
    );
    assert_eq!(report.truncated_iterations, Some(3));
    assert_eq!(
        report.iteration_summary.expect("summaries").len(),
        5,
        "summaries are never trimmed"
    );
}

/// `detail_iterations = 0` 且无失败轮 ⇒ 一轮详情都不保留
#[tokio::test]
async fn loop_report_trimming_zero_keeps_no_detail() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(3);
    cfg.report = LoopReportConfig {
        detail_iterations: 0,
        always_keep_failed: true,
        ..Default::default()
    };
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;

    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    assert!(body.node_reports.is_empty(), "不应保留任何轮次详情");
    assert_eq!(report.truncated_iterations, Some(3));
    assert_eq!(report.iteration_summary.expect("summaries").len(), 3);
}

/// 钉住的失败轮不会被桶挤出；未钉住时同样按 FIFO 淘汰
#[tokio::test]
async fn loop_failed_rounds_survive_bucket_eviction() {
    fn failing_body() -> FlowSubGraph {
        // script 语法错误：确定性失败，无需网络
        FlowSubGraph::new(
            vec![node(
                "body-script",
                flow_engine_core::node_types::SCRIPT,
                NodeData::script("this is not valid javascript {{{"),
            )],
            vec![],
        )
    }

    let mut pinned = loop_config(LoopSource::Count);
    pinned.count = Some(3);
    pinned.break_on_error = false;
    pinned.report = LoopReportConfig {
        detail_iterations: 1,
        always_keep_failed: true,
        ..Default::default()
    };
    pinned.body = failing_body();
    let result = run(&build_flow(pinned)).await;
    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    let kept: Vec<u32> = body
        .node_reports
        .iter()
        .filter_map(|r| r.iteration_index)
        .collect();
    assert_eq!(kept, vec![0, 1, 2], "钉住的失败轮不受桶容量限制");
    assert_eq!(report.truncated_iterations, Some(0));

    let mut evicting = loop_config(LoopSource::Count);
    evicting.count = Some(3);
    evicting.break_on_error = false;
    evicting.report = LoopReportConfig {
        detail_iterations: 1,
        always_keep_failed: false,
        ..Default::default()
    };
    evicting.body = failing_body();
    let result = run(&build_flow(evicting)).await;
    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    let kept: Vec<u32> = body
        .node_reports
        .iter()
        .filter_map(|r| r.iteration_index)
        .collect();
    assert_eq!(
        kept,
        vec![2],
        "未钉住的失败轮同样按 FIFO 淘汰，只留最近一轮"
    );
    assert_eq!(report.truncated_iterations, Some(2));
}

#[tokio::test]
async fn loop_interval_backoff_sequence() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(4);
    cfg.interval_ms = Some(50);
    cfg.interval_backoff = Some(2.0);
    cfg.interval_max_ms = Some(150);
    cfg.body = delay_body();

    let started = std::time::Instant::now();
    let result = run(&build_flow(cfg)).await;
    let elapsed = started.elapsed().as_millis();
    assert!(result.success);
    // 只校验下界（退避总时长）：上界受并行测试的调度抖动影响，墙钟断言会偶发跑飞，
    // 真正的退避契约由下面的 `waits` 序列断言。
    assert!(elapsed >= 300, "backoff too fast: {elapsed}ms");
    assert!(elapsed < 10_000, "backoff runaway: {elapsed}ms");

    let report = loop_report(&result);
    let summaries = report.iteration_summary.expect("summaries");
    let waits: Vec<u64> = summaries.iter().map(|s| s.wait_ms).collect();
    assert_eq!(waits, vec![100, 150, 150, 0], "backoff capped at 150ms");
}

#[tokio::test]
async fn loop_timeout_includes_wait() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(5);
    cfg.interval_ms = Some(1000);
    cfg.timeout_ms = Some(200);
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;

    let report = loop_report(&result);
    let output = output_of(&report);
    assert_eq!(output["break_reason"], json!("timeout"));
    // 超时必须在本轮等待边界处截断，远小于配置的 5 轮即可证明生效；
    // 允许 2 轮是因为 deadline 判定与睡眠唤醒在负载下可能相差一个调度周期。
    assert!(
        output["iterations"].as_u64().unwrap_or(0) <= 2,
        "unexpected iterations: {}",
        output["iterations"]
    );
}

#[tokio::test]
async fn loop_initial_delay_applied() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(1);
    cfg.initial_delay_ms = Some(200);
    cfg.body = delay_body();

    let started = std::time::Instant::now();
    let result = run(&build_flow(cfg)).await;
    assert!(result.success);
    assert!(started.elapsed().as_millis() >= 200);
}

#[tokio::test]
async fn loop_cancel_during_wait_is_fast() {
    use flow_engine_core::execution_event::ExecutionEvent;
    use flow_engine_core::traits::ExecutionEventSender;

    /// 在 loop 节点开始执行时放行取消信号，避免依赖墙钟时序（负载下会偶发跑飞）
    struct LoopStartSignal {
        tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    }

    impl ExecutionEventSender for LoopStartSignal {
        fn send_event(&self, event: ExecutionEvent) -> Result<(), String> {
            if let ExecutionEvent::NodeStarted { node_uuid, .. } = &event {
                if node_uuid == "loop" {
                    if let Ok(mut guard) = self.tx.lock() {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(());
                        }
                    }
                }
            }
            Ok(())
        }
    }

    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(5);
    cfg.interval_ms = Some(60_000);
    cfg.body = delay_body();

    let flow = build_flow(cfg);
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let (start_tx, start_rx) = tokio::sync::oneshot::channel();
    let executor = FlowExecutor::new()
        .with_cancel_receiver(cancel_rx)
        .with_execution_event_sender(Arc::new(LoopStartSignal {
            tx: std::sync::Mutex::new(Some(start_tx)),
        }));

    let started = std::time::Instant::now();
    let handle = tokio::spawn(async move {
        // 兜底超时：即使事件没到也放行，避免测试挂死
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), start_rx).await;
        let _ = cancel_tx.send(true);
    });
    let result = executor.execute_flow(&flow, None, None, None).await;
    handle.await.unwrap_or_else(|e| panic!("{e}"));
    assert!(
        started.elapsed().as_millis() < 3000,
        "cancellation should interrupt the wait"
    );

    // 取消信号与节点返回在 executor 的 select 中竞争：可能直接中止流程（无 loop 报告），
    // 也可能先拿到 loop 节点的 cancelled 输出。两条路径都必须快速返回且整体标记为取消。
    let flow_report = result.flow_report.clone().expect("flow report");
    assert_eq!(
        flow_report.status,
        flow_engine_core::report::ExecutionStatus::Cancelled,
        "flow should be marked cancelled"
    );
    if let Some(report) = result
        .node_reports
        .iter()
        .find(|r| r.node_type == flow_engine_core::node_types::LOOP)
    {
        assert_eq!(output_of(report)["break_reason"], json!("cancelled"));
        assert_eq!(report.used_port_id.as_deref(), Some("false"));
    }
}

#[tokio::test]
async fn loop_until_expression_error() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(2);
    cfg.until = Some("last.requested_ms ==".to_string());
    cfg.body = delay_body();
    let result = run(&build_flow(cfg)).await;

    let report = loop_report(&result);
    let output = output_of(&report);
    assert_eq!(report.used_port_id.as_deref(), Some("false"));
    assert_eq!(output["break_reason"], json!("error"));
    let message = output["error"].as_str().unwrap_or_default().to_string();
    assert!(
        message.contains("last.requested_ms =="),
        "unexpected message: {message}"
    );
}

#[tokio::test]
async fn loop_config_errors_surface_as_configuration() {
    use flow_engine_core::context::ExecutionContext;
    use flow_engine_core::executor::NodeExecutor;

    // items 不是数组 → 配置错误（直接跑节点，避免 false 端口把错误吞进流程状态）
    let mut cfg = loop_config(LoopSource::Items);
    cfg.items = Some(r#"{"a":1}"#.to_string());
    cfg.body = delay_body();
    let loop_node = node(
        "loop",
        flow_engine_core::node_types::LOOP,
        NodeData::loop_node(cfg),
    );
    let mut context = ExecutionContext::new("flow-1".to_string(), 0);
    let err = flow_engine_executor::loop_executor::LoopNodeExecutor::new()
        .execute(
            &loop_node,
            &mut context,
            &[],
            &std::collections::HashMap::new(),
        )
        .await
        .err()
        .expect("expected configuration error");
    assert!(
        err.message.contains("not a JSON array"),
        "unexpected message: {}",
        err.message
    );
}

// ============================================================================
// 运行中实时事件 + 全量详情保留
// ============================================================================

/// 记录所有执行事件，供断言使用
#[derive(Default)]
struct LoopEventRecorder {
    events: std::sync::Mutex<Vec<flow_engine_core::execution_event::ExecutionEvent>>,
}

impl LoopEventRecorder {
    fn loop_iterations(&self) -> Vec<(u32, ExecutionStatus, usize, bool)> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter_map(|event| match event {
                flow_engine_core::execution_event::ExecutionEvent::LoopIteration {
                    iteration,
                    reports,
                    truncated,
                    ..
                } => Some((
                    iteration.index,
                    iteration.status.clone(),
                    reports.len(),
                    *truncated,
                )),
                _ => None,
            })
            .collect()
    }
}

impl flow_engine_core::traits::ExecutionEventSender for LoopEventRecorder {
    fn send_event(
        &self,
        event: flow_engine_core::execution_event::ExecutionEvent,
    ) -> Result<(), String> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(event);
        Ok(())
    }
}

#[tokio::test]
async fn loop_emits_iteration_events_while_running() {
    use flow_engine_core::traits::ExecutionEventSender;

    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(3);
    cfg.report.detail_iterations = 1;
    cfg.body = delay_body();

    let recorder = Arc::new(LoopEventRecorder::default());
    let sender: Arc<dyn ExecutionEventSender> = recorder.clone();
    let flow = build_flow(cfg);
    let result = FlowExecutor::new()
        .with_execution_event_sender(sender)
        .execute_flow(&flow, None, None, None)
        .await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let iterations = recorder.loop_iterations();
    assert_eq!(iterations.len(), 3, "每轮都应推送一条 LoopIteration");

    // 桶容量 > 0 时，每一轮在推送时都是"最新一轮"，因此都带详情、都不是 truncated
    for (index, status, report_count, truncated) in &iterations {
        assert_eq!(*status, ExecutionStatus::Success);
        assert_eq!(*report_count, 1, "第 {index} 轮应携带 body 节点报告");
        assert!(!*truncated, "第 {index} 轮在推送时尚未被挤出桶");
    }

    // 但最终报告只保留最近 1 轮（桶容量 = 1）
    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    let kept: Vec<u32> = body
        .node_reports
        .iter()
        .filter_map(|r| r.iteration_index)
        .collect();
    assert_eq!(kept, vec![2], "最终报告只保留最近一轮详情");
    assert_eq!(report.truncated_iterations, Some(2));
}

#[tokio::test]
async fn loop_keep_all_details_preserves_every_iteration() {
    let mut cfg = loop_config(LoopSource::Count);
    cfg.count = Some(3);
    cfg.report.detail_iterations = 1;
    cfg.report.keep_all_details = true;
    cfg.body = delay_body();

    let result = run(&build_flow(cfg)).await;
    assert!(result.success, "flow should succeed: {:?}", result.error);

    let report = loop_report(&result);
    let body = report.sub_flow_report.expect("body report");
    assert_eq!(
        body.node_reports.len(),
        3,
        "keep_all_details 应保留每一轮的完整详情"
    );
    assert_eq!(report.truncated_iterations, Some(0));
    assert_eq!(report.iteration_summary.expect("summaries").len(), 3);
}
