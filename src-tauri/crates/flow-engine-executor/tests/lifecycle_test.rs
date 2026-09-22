//! 流程生命周期阶段 (Setup / Main / Teardown) 集成测试

use flow_engine_core::connection::Connection;
use flow_engine_core::flow::{Flow, FlowSubGraph};
use flow_engine_core::node::{ExecutionErrorStrategy, Node, NodePosition};
use flow_engine_core::node_config::{EntryConfig, NodeData, ScriptErrorStrategy, ScriptNodeConfig};
use flow_engine_core::report::ExecutionStatus;
use flow_engine_executor::executor::FlowExecutor;
use serde_json::json;

fn make_entry_node(uuid: &str, name: &str) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: "entry".to_string(),
        name: name.to_string(),
        position: NodePosition::default(),
        data: NodeData::Entry(EntryConfig::default()),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    }
}

fn make_script_node(uuid: &str, name: &str, code: &str) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: "script".to_string(),
        name: name.to_string(),
        position: NodePosition::default(),
        data: NodeData::Script(ScriptNodeConfig {
            script: code.to_string(),
            timeout_ms: 5000,
            ..Default::default()
        }),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    }
}

fn make_fatal_script_node(uuid: &str, name: &str, code: &str) -> Node {
    let mut node = make_script_node(uuid, name, code);
    node.data = NodeData::Script(ScriptNodeConfig {
        script: code.to_string(),
        timeout_ms: 5000,
        on_error: ScriptErrorStrategy::Fatal,
        ..Default::default()
    });
    node.on_error = Some(ExecutionErrorStrategy::AbortFlow);
    node
}

#[tokio::test]
async fn test_flow_lifecycle_single_run() {
    // Setup：单次准备阶段，导出 token 变量
    let setup_node = make_script_node(
        "setup-1",
        "Setup Script",
        "fn.variables.set('token', 'secret');",
    );
    let setup = FlowSubGraph::new(vec![setup_node], vec![]);

    // Main：entry -> script（引用 Setup 导出的 token）
    let entry = make_entry_node("entry-1", "Main Entry");
    let main_script = make_script_node(
        "main-1",
        "Main Script",
        "let tok = fn.variables.get('token'); fn.variables.set('main_done', tok === 'secret');",
    );
    let conn = Connection {
        id: "c1".to_string(),
        source_node_uuid: "entry-1".to_string(),
        source_port_id: "out".to_string(),
        target_node_uuid: "main-1".to_string(),
        target_port_id: "in".to_string(),
    };

    // Teardown：读取 $report.status 并断言为 success
    let teardown_node = make_script_node(
        "teardown-1",
        "Teardown Script",
        "let status = fn.variables.replaceIn('{{$report.status}}');\n\
         fn.variables.set('report_status', status);\n\
         if (status !== 'success') { throw new Error('unexpected report status: ' + status); }",
    );
    let teardown = FlowSubGraph::new(vec![teardown_node], vec![]);

    let flow = Flow {
        uuid: "flow-lifecycle-1".to_string(),
        name: "Lifecycle Test".to_string(),
        setup: Some(setup),
        nodes: vec![entry, main_script],
        connections: vec![conn],
        teardown: Some(teardown),
        ..Default::default()
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(result.success, "Flow should succeed: {:?}", result.error);
    let flow_report = result
        .flow_report
        .unwrap_or_else(|| panic!("Should have flow report"));

    // 三阶段顺序执行且全部成功
    let setup_report = flow_report
        .stages
        .setup
        .as_ref()
        .unwrap_or_else(|| panic!("setup stage"));
    assert_eq!(setup_report.status, ExecutionStatus::Success);
    assert_eq!(setup_report.node_reports.len(), 1);
    assert_eq!(flow_report.stages.main.status, ExecutionStatus::Success);
    assert_eq!(flow_report.stages.main.node_reports.len(), 2);
    assert_eq!(
        flow_report.main_duration_ms,
        flow_report.stages.main.duration_ms
    );

    let teardown_report = flow_report
        .stages
        .teardown
        .as_ref()
        .unwrap_or_else(|| panic!("teardown stage"));
    assert_eq!(teardown_report.status, ExecutionStatus::Success);

    // 变量跨阶段透传 + $report 对 Teardown 可见
    let final_vars = flow_report
        .final_variables
        .unwrap_or_else(|| panic!("Should have final variables"));
    assert_eq!(final_vars.get("token"), Some(&json!("secret")));
    assert_eq!(final_vars.get("main_done"), Some(&json!(true)));
    assert_eq!(final_vars.get("report_status"), Some(&json!("success")));

    // 扁平节点报告合并三阶段
    assert_eq!(flow_report.node_reports.len(), 4);
}

#[tokio::test]
async fn test_flow_lifecycle_setup_failure() {
    // Setup 阶段报错（Fatal → 中止）
    let setup_node = make_fatal_script_node(
        "setup-fail",
        "Failing Setup",
        "throw new Error('setup exploded');",
    );
    let setup = FlowSubGraph::new(vec![setup_node], vec![]);

    // Main：应被跳过
    let entry = make_entry_node("entry-1", "Main Entry");
    let main_script = make_script_node(
        "main-1",
        "Main Script",
        "fn.variables.set('main_executed', true);",
    );
    let conn = Connection {
        id: "c1".to_string(),
        source_node_uuid: "entry-1".to_string(),
        source_port_id: "out".to_string(),
        target_node_uuid: "main-1".to_string(),
        target_port_id: "in".to_string(),
    };

    // Teardown：接收 $report（main 阶段被跳过 → status 为 skipped）
    let teardown_node = make_script_node(
        "teardown-1",
        "Teardown Script",
        "let status = fn.variables.replaceIn('{{$report.status}}');\n\
         fn.variables.set('report_status', status);",
    );
    let teardown = FlowSubGraph::new(vec![teardown_node], vec![]);

    let flow = Flow {
        uuid: "flow-lifecycle-fail".to_string(),
        name: "Setup Failure Test".to_string(),
        setup: Some(setup),
        nodes: vec![entry, main_script],
        connections: vec![conn],
        teardown: Some(teardown),
        ..Default::default()
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(!result.success, "Flow should fail due to setup failure");
    let flow_report = result
        .flow_report
        .unwrap_or_else(|| panic!("Should have flow report"));

    assert_eq!(
        flow_report
            .stages
            .setup
            .as_ref()
            .unwrap_or_else(|| panic!("setup stage"))
            .status,
        ExecutionStatus::Failed
    );
    assert_eq!(flow_report.stages.main.status, ExecutionStatus::Skipped);
    assert_eq!(flow_report.stages.main.node_reports.len(), 0);
    assert_eq!(flow_report.main_duration_ms, 0);
    assert_eq!(flow_report.status, ExecutionStatus::Failed);

    // Main 未执行，Teardown 仍然运行并看到 skipped 状态
    let final_vars = flow_report
        .final_variables
        .unwrap_or_else(|| panic!("Should have final variables"));
    assert_eq!(final_vars.get("main_executed"), None);
    assert_eq!(final_vars.get("report_status"), Some(&json!("skipped")));
}

#[tokio::test]
async fn test_flow_lifecycle_teardown_failure_keeps_main_success() {
    // Main 成功，Teardown 报错 → 整体 Failed，但 stages.main 仍为 Success
    let entry = make_entry_node("entry-1", "Main Entry");
    let conn = Connection {
        id: "c1".to_string(),
        source_node_uuid: "entry-1".to_string(),
        source_port_id: "out".to_string(),
        target_node_uuid: "main-1".to_string(),
        target_port_id: "in".to_string(),
    };
    let main_script = make_script_node("main-1", "Main Script", "fn.variables.set('ok', true);");

    let teardown_node = make_fatal_script_node(
        "teardown-fail",
        "Failing Teardown",
        "throw new Error('notify failed');",
    );
    let teardown = FlowSubGraph::new(vec![teardown_node], vec![]);

    let flow = Flow {
        uuid: "flow-lifecycle-teardown-fail".to_string(),
        name: "Teardown Failure Test".to_string(),
        setup: None,
        nodes: vec![entry, main_script],
        connections: vec![conn],
        teardown: Some(teardown),
        ..Default::default()
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    let flow_report = result
        .flow_report
        .unwrap_or_else(|| panic!("Should have flow report"));
    assert_eq!(flow_report.status, ExecutionStatus::Failed);
    assert_eq!(flow_report.stages.main.status, ExecutionStatus::Success);
    assert!(flow_report.stages.setup.is_none());
    assert_eq!(
        flow_report
            .stages
            .teardown
            .as_ref()
            .unwrap_or_else(|| panic!("teardown"))
            .status,
        ExecutionStatus::Failed
    );
}

#[tokio::test]
async fn test_flow_without_lifecycle_stages_unaffected() {
    // 未配置 setup/teardown：零开销跳过，stages 仅含 main
    let entry = make_entry_node("entry-1", "Main Entry");
    let conn = Connection {
        id: "c1".to_string(),
        source_node_uuid: "entry-1".to_string(),
        source_port_id: "out".to_string(),
        target_node_uuid: "main-1".to_string(),
        target_port_id: "in".to_string(),
    };
    let main_script = make_script_node("main-1", "Main Script", "fn.variables.set('ok', true);");

    let flow = Flow {
        uuid: "flow-no-lifecycle".to_string(),
        name: "Plain Flow".to_string(),
        setup: None,
        nodes: vec![entry, main_script],
        connections: vec![conn],
        teardown: None,
        ..Default::default()
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(result.success);
    let flow_report = result
        .flow_report
        .unwrap_or_else(|| panic!("Should have flow report"));
    assert!(flow_report.stages.setup.is_none());
    assert!(flow_report.stages.teardown.is_none());
    assert_eq!(flow_report.stages.main.status, ExecutionStatus::Success);
    assert_eq!(flow_report.stages.main.node_reports.len(), 2);
    assert_eq!(flow_report.node_reports.len(), 2);
}

#[tokio::test]
async fn test_stress_test_lifecycle_isolation() {
    use flow_engine_executor::stress_test::{
        StressTestConfig, StressTestEvent, StressTestExecutor,
    };

    // Setup：单次准备工作，导出 token
    let setup_node = make_script_node(
        "setup-1",
        "Setup Script",
        "fn.variables.set('token', 'from-setup');",
    );
    let setup = FlowSubGraph::new(vec![setup_node], vec![]);

    // Main：entry -> script（校验 Setup 变量可见；若不可见则节点失败）
    let entry = make_entry_node("entry-1", "Main Entry");
    let main_script = make_script_node(
        "main-1",
        "Main Script",
        "let tok = fn.variables.get('token');\n\
         if (tok !== 'from-setup') { throw new Error('setup token missing'); }",
    );
    let conn = Connection {
        id: "c1".to_string(),
        source_node_uuid: "entry-1".to_string(),
        source_port_id: "out".to_string(),
        target_node_uuid: "main-1".to_string(),
        target_port_id: "in".to_string(),
    };

    // Teardown：单次收尾，消费 $report 中的压测统计
    let teardown_node = make_script_node(
        "teardown-1",
        "Teardown Script",
        "let total = fn.variables.replaceIn('{{$report.statistics.totalIterations}}');\n\
         fn.variables.set('teardown_total', total);",
    );
    let teardown = FlowSubGraph::new(vec![teardown_node], vec![]);

    let flow = Flow {
        uuid: "flow-stress-lifecycle".to_string(),
        name: "Stress Lifecycle".to_string(),
        setup: Some(setup),
        nodes: vec![entry, main_script],
        connections: vec![conn],
        teardown: Some(teardown),
        ..Default::default()
    };

    let config = StressTestConfig {
        concurrent_users: 10,
        ramp_up_seconds: 0,
        duration_seconds: 2,
        flow_uuid: flow.uuid.clone(),
        dataset_path: None,
        dataset_encoding: None,
        dataset_strategy: None,
    };

    let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel::<StressTestEvent>();
    let registry = std::sync::Arc::new(flow_engine_executor::default_registry::default_registry());
    let stress = StressTestExecutor::new(flow, config, event_tx, registry);

    stress
        .start()
        .await
        .unwrap_or_else(|e| panic!("stress test should start: {e}"));

    // 等待压测完成（报告生成即代表 Teardown 已执行完毕）
    let report = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if let Some(report) = stress.get_report().await {
                break report;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("stress test should finish within 30s"));

    // Main 并发循环：10 用户 × 2 秒，迭代数远大于 1
    assert!(
        report.statistics.total_iterations > 10,
        "main should loop many times, got {}",
        report.statistics.total_iterations
    );
    assert_eq!(
        report.statistics.failed, 0,
        "main iterations should all succeed (setup variable must be visible)"
    );
    assert!(
        report.statistics.rps > 0.0,
        "QPS must be computed from main iterations"
    );

    // 指标纯净性：节点级统计只包含 Main 节点，不含 Setup/Teardown 节点
    let node_uuids: Vec<&str> = report
        .node_statistics
        .iter()
        .map(|s| s.node_uuid.as_str())
        .collect();
    assert!(
        node_uuids.contains(&"entry-1") && node_uuids.contains(&"main-1"),
        "main nodes must appear in metrics: {node_uuids:?}"
    );
    assert!(
        !node_uuids.contains(&"setup-1") && !node_uuids.contains(&"teardown-1"),
        "setup/teardown must not pollute stress metrics: {node_uuids:?}"
    );

    // 生命周期记录：Setup 与 Teardown 各执行 1 次（各 1 个节点报告）
    let lifecycle = report
        .lifecycle
        .unwrap_or_else(|| panic!("lifecycle report must be present"));
    let setup_stage = lifecycle
        .setup
        .unwrap_or_else(|| panic!("setup stage report"));
    assert_eq!(setup_stage.node_reports.len(), 1);
    assert_eq!(setup_stage.status, ExecutionStatus::Success);
    let teardown_stage = lifecycle
        .teardown
        .unwrap_or_else(|| panic!("teardown stage report"));
    assert_eq!(teardown_stage.node_reports.len(), 1);
    assert_eq!(teardown_stage.status, ExecutionStatus::Success);
    assert_eq!(lifecycle.main.status, ExecutionStatus::Success);
}

#[tokio::test]
async fn test_debug_lifecycle_stages_execution() {
    use flow_engine_executor::debug::{DebugCommand, DebugEvent, DebuggableFlowExecutor};

    let flow = Flow {
        uuid: "flow-debug-lifecycle".to_string(),
        name: "Debug 生命周期测试".to_string(),
        description: None,
        setup: Some(FlowSubGraph {
            nodes: vec![make_script_node(
                "setup-1",
                "Setup Script",
                "fn.variables.set('token', 'secret-debug');",
            )],
            connections: vec![],
        }),
        nodes: vec![
            make_entry_node("entry-1", "Entry"),
            make_script_node(
                "main-1",
                "Main Script",
                "const t = fn.variables.get('token'); fn.variables.set('main_done', true); fn.variables.set('got_token', t);",
            ),
        ],
        connections: vec![Connection {
            id: "c-1".to_string(),
            source_node_uuid: "entry-1".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "main-1".to_string(),
            target_port_id: "in".to_string(),
        }],
        teardown: Some(FlowSubGraph {
            nodes: vec![make_script_node(
                "teardown-1",
                "Teardown Script",
                "let status = fn.variables.replaceIn('{{$report.status}}'); fn.variables.set('report_status', status);",
            )],
            connections: vec![],
        }),
        initial_variables: None,
    };

    let executor = DebuggableFlowExecutor::new();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<DebugEvent>();
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<DebugCommand>();

    // 在 setup-1 和 main-1 上打断点
    let breakpoints = vec!["setup-1".to_string(), "main-1".to_string()];

    let debug_handle = tokio::spawn(async move {
        executor
            .start_debug(&flow, None, breakpoints, None, event_tx, cmd_rx)
            .await
    });

    // 1. 等待命中断点 setup-1 与 main-1
    let mut saw_setup_breakpoint = false;
    let mut saw_main_breakpoint = false;

    while let Some(event) = event_rx.recv().await {
        match event {
            DebugEvent::PausedAtNode { node_uuid, .. } => {
                if node_uuid == "setup-1" {
                    saw_setup_breakpoint = true;
                    cmd_tx
                        .send(DebugCommand::Continue)
                        .unwrap_or_else(|e| panic!("failed to send continue: {e}"));
                } else if node_uuid == "main-1" {
                    saw_main_breakpoint = true;
                    cmd_tx
                        .send(DebugCommand::Continue)
                        .unwrap_or_else(|e| panic!("failed to send continue: {e}"));
                }
            }
            DebugEvent::SessionEnded { report, .. } => {
                let rep = report.unwrap_or_else(|| panic!("report must exist"));
                assert!(saw_setup_breakpoint, "Must pause at setup breakpoint");
                assert!(saw_main_breakpoint, "Must pause at main breakpoint");
                assert!(
                    rep.stages.setup.is_some(),
                    "Setup stage must exist in debug report"
                );
                assert!(
                    rep.stages.teardown.is_some(),
                    "Teardown stage must exist in debug report"
                );
                assert_eq!(
                    rep.stages
                        .setup
                        .as_ref()
                        .unwrap_or_else(|| panic!("setup"))
                        .status,
                    ExecutionStatus::Success
                );
                assert_eq!(rep.stages.main.status, ExecutionStatus::Success);
                assert_eq!(
                    rep.stages
                        .teardown
                        .as_ref()
                        .unwrap_or_else(|| panic!("teardown"))
                        .status,
                    ExecutionStatus::Success
                );
                break;
            }
            _ => {}
        }
    }

    let debug_state = debug_handle
        .await
        .unwrap_or_else(|e| panic!("debug task panicked: {e}"));
    assert_eq!(
        debug_state.variables.get("got_token"),
        Some(&json!("secret-debug")),
        "Setup variables must pass to Main in debug mode"
    );
    assert_eq!(
        debug_state.variables.get("report_status"),
        Some(&json!("success")),
        "Teardown must receive $report in debug mode"
    );
}

#[tokio::test]
async fn test_subgraph_disconnected_nodes_visual_order() {
    let mut top_node = make_script_node(
        "node-top",
        "Top Node",
        "fn.variables.set('step1', 'done_first');",
    );
    top_node.position = NodePosition { x: 0.0, y: 10.0 };

    let mut bottom_node = make_script_node(
        "node-bottom",
        "Bottom Node",
        "let first = fn.variables.get('step1'); fn.variables.set('step2', first === 'done_first' ? 'done_second' : 'wrong');",
    );
    bottom_node.position = NodePosition { x: 0.0, y: 200.0 };

    // 即使在 nodes 数组中把 bottom 放前面，引擎仍应根据 y 坐标先执行 top_node
    let setup = FlowSubGraph::new(vec![bottom_node, top_node], vec![]);

    let entry = make_entry_node("entry-1", "Main Entry");
    let flow = Flow {
        uuid: "flow-visual-order".to_string(),
        name: "Visual Order Test".to_string(),
        setup: Some(setup),
        nodes: vec![entry],
        connections: vec![],
        ..Default::default()
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;
    assert!(result.success);
    let vars = result
        .flow_report
        .unwrap_or_else(|| panic!("flow report"))
        .final_variables
        .unwrap_or_else(|| panic!("final variables"));
    assert_eq!(vars.get("step1"), Some(&json!("done_first")));
    assert_eq!(vars.get("step2"), Some(&json!("done_second")));
}
