use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorCode;
use flow_engine_core::flow::Flow;
use flow_engine_core::node::{ExecutionErrorStrategy, Node, NodePosition, Port, PortType};
use flow_engine_core::node_config::{NodeData, ScriptNodeConfig, SubFlowConfig, VariableMapping};
use flow_engine_executor::executor::FlowExecutor;
use flow_engine_executor::storage::FlowFile;
use flow_engine_executor::validator::FlowValidator;
use serde_json::json;
use std::path::Path;
use std::sync::Mutex;
use tempfile::tempdir;

/// 流程目录覆盖是进程级状态：涉及该状态的用例串行执行，避免互相踩踏
static FLOWS_DIR_OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

/// RAII：登记流程目录覆盖，作用域结束（含 panic 展开）时复位为默认目录
struct FlowsDirOverride(std::path::PathBuf);

impl FlowsDirOverride {
    fn set(dir: std::path::PathBuf) -> Self {
        flow_engine_executor::storage::set_flows_dir_override(Some(dir.clone()));
        Self(dir)
    }
}

impl Drop for FlowsDirOverride {
    fn drop(&mut self) {
        flow_engine_executor::storage::set_flows_dir_override(None);
    }
}

fn write_flow_file(path: &Path, flow: &Flow) {
    let flow_file = FlowFile {
        _version: 1,
        flow: flow.clone(),
        _meta: None,
    };
    std::fs::write(path, serde_json::to_string_pretty(&flow_file).unwrap()).unwrap();
}

fn create_entry_node(uuid: &str, name: &str) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: "entry".to_string(),
        name: name.to_string(),
        position: NodePosition::default(),
        data: NodeData::entry(),
        ports: Some(vec![Port {
            id: "out".to_string(),
            port_type: PortType::Output,
            label: Some("Output".to_string()),
        }]),
        has_breakpoint: None,
        on_error: None,
    }
}
fn create_end_node(uuid: &str, name: &str) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: "end".to_string(),
        name: name.to_string(),
        position: NodePosition::default(),
        data: NodeData::end(),
        ports: Some(vec![Port {
            id: "in".to_string(),
            port_type: PortType::Input,
            label: Some("Input".to_string()),
        }]),
        has_breakpoint: None,
        on_error: None,
    }
}

fn create_script_node(uuid: &str, name: &str, script: &str) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: "script".to_string(),
        name: name.to_string(),
        position: NodePosition::default(),
        data: NodeData::Script(ScriptNodeConfig {
            script: script.to_string(),
            timeout_ms: 5000,
            ..Default::default()
        }),
        ports: Some(vec![
            Port {
                id: "in".to_string(),
                port_type: PortType::Input,
                label: Some("Input".to_string()),
            },
            Port {
                id: "out".to_string(),
                port_type: PortType::Output,
                label: Some("Output".to_string()),
            },
        ]),
        has_breakpoint: None,
        on_error: None,
    }
}

#[tokio::test]
async fn test_subflow_basic_execution_and_variable_passing() {
    let dir = tempdir().expect("create tempdir");
    let child_path = dir.path().join("child_flow.json");

    // 1. 创建子流程：entry -> script (设置 token 变量)
    let child_flow = Flow {
        uuid: "child-flow-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Child Flow".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("child-entry", "Child Entry"),
            create_script_node(
                "child-script",
                "Set Token",
                "fn.variables.set('token', 'secret123'); fn.variables.set('passed', (parseInt(fn.variables.get('input_num'), 10) + 1).toString());",
            ),
        ],
        connections: vec![Connection {
            id: "conn-child-1".to_string(),
            source_node_uuid: "child-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "child-script".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    write_flow_file(&child_path, &child_flow);

    // 2. 创建父流程：entry -> sub_flow
    let subflow_config = SubFlowConfig {
        target_flow_id: None,
        target_flow_path: Some(child_path.to_string_lossy().to_string()),
        inherit_all_variables: true,
        variable_mappings: vec![VariableMapping {
            source: "num".to_string(),
            target: "input_num".to_string(),
        }],
        export_all_variables: true,
        output_mappings: vec![],
    };

    let parent_flow = Flow {
        uuid: "parent-flow-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Parent Flow".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("parent-entry", "Parent Entry"),
            Node {
                uuid: "parent-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Child Flow".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(subflow_config),
                ports: Some(vec![
                    Port {
                        id: "in".to_string(),
                        port_type: PortType::Input,
                        label: Some("Input".to_string()),
                    },
                    Port {
                        id: "out".to_string(),
                        port_type: PortType::Output,
                        label: Some("Success".to_string()),
                    },
                    Port {
                        id: "false".to_string(),
                        port_type: PortType::Output,
                        label: Some("Failed".to_string()),
                    },
                ]),
                has_breakpoint: None,
                on_error: None,
            },
        ],
        connections: vec![Connection {
            id: "conn-parent-1".to_string(),
            source_node_uuid: "parent-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "parent-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let initial_vars = json!({ "num": 41 });
    let result = executor
        .execute_flow(&parent_flow, Some(initial_vars), None, None)
        .await;
    if !result.success {
        for r in &result.node_reports {
            eprintln!(
                "Node {} ({}): status={:?}, error={:?}",
                r.node_name, r.node_type, r.status, r.error
            );
        }
    }
    assert!(
        result.success,
        "Parent execution failed: {:?}",
        result.error
    );
    let parent_report = result.flow_report.expect("Parent flow report present");

    // 验证子流程报告树存在
    let subflow_node_report = parent_report
        .node_reports
        .iter()
        .find(|r| r.node_uuid == "parent-subflow")
        .expect("SubFlow node report found");

    assert!(
        subflow_node_report.sub_flow_report.is_some(),
        "sub_flow_report should be stashed inside NodeExecutionReport"
    );

    let child_report = subflow_node_report.sub_flow_report.as_ref().unwrap();
    assert_eq!(child_report.flow_uuid, "child-flow-uuid");
    assert_eq!(child_report.executed_count, 2);

    // 验证变量导出到父流程
    let final_vars = parent_report
        .final_variables
        .expect("Final variables exist");
    assert_eq!(
        final_vars.get("token").and_then(|v| v.as_str()),
        Some("secret123")
    );
    assert_eq!(
        final_vars.get("passed").and_then(|v| v.as_str()),
        Some("42")
    );
}

#[tokio::test]
async fn test_subflow_direct_cycle_validation_and_runtime() {
    let self_referencing_flow = Flow {
        uuid: "flow-self".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Self Loop Flow".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("entry-1", "Entry"),
            Node {
                uuid: "subflow-1".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Self Call".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: Some("flow-self".to_string()),
                    target_flow_path: None,
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
            id: "c1".to_string(),
            source_node_uuid: "entry-1".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "subflow-1".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    // 1. 静态验证拦截自引用
    let validator = FlowValidator::new();
    let val_result = validator.validate(&self_referencing_flow);
    assert!(
        !val_result.valid,
        "Static validation should reject direct self reference"
    );
    let cycle_err = val_result
        .errors
        .iter()
        .find(|e| e.code == ErrorCode::E_VAL_CYCLE_DETECTED);
    assert!(cycle_err.is_some(), "Expected E_VAL_CYCLE_DETECTED error");

    // 2. 执行期拦截循环调用
    let executor = FlowExecutor::new();
    let result = executor
        .execute_flow(&self_referencing_flow, None, None, None)
        .await;
    assert!(!result.success);
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Flow validation failed")
            || result
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Cycle detected"),
        "Execution should fail on self-referencing flow: {:?}",
        result.error
    );
}

#[tokio::test]
async fn test_subflow_indirect_cycle_runtime_guard() {
    let dir = tempdir().expect("create tempdir");
    let flow_a_path = dir.path().join("flow_a.json");
    let flow_b_path = dir.path().join("flow_b.json");

    // Flow A calls Flow B
    let flow_a = Flow {
        uuid: "flow-a-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Flow A".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("a-entry", "Entry A"),
            Node {
                uuid: "a-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Flow B".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: None,
                    target_flow_path: Some(flow_b_path.to_string_lossy().to_string()),
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
            id: "ca".to_string(),
            source_node_uuid: "a-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "a-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    // Flow B calls Flow A (indirect cycle)
    let flow_b = Flow {
        uuid: "flow-b-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Flow B".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("b-entry", "Entry B"),
            Node {
                uuid: "b-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Flow A".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: None,
                    target_flow_path: Some(flow_a_path.to_string_lossy().to_string()),
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
            id: "cb".to_string(),
            source_node_uuid: "b-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "b-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    write_flow_file(&flow_a_path, &flow_a);
    write_flow_file(&flow_b_path, &flow_b);

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow_a, None, None, None).await;

    assert!(!result.success, "Execution should fail when cycle occurs");
    let has_cycle_error = result.error.as_deref().is_some_and(|e| {
        e.contains("Circular subflow dependency detected") || e.contains("Cycle detected")
    }) || result.node_reports.iter().any(|r| {
        r.error
            .as_ref()
            .is_some_and(|e| e.message.contains("Cycle detected in subflow call chain"))
    });
    assert!(
        has_cycle_error,
        "Execution should fail with cycle detection error: {:?}",
        result
    );

    // 验证运行期第二道防线：直接调用 SubFlowNodeExecutor 时，通过 context.call_stack 拦截
    let mut ctx = flow_engine_core::context::ExecutionContext::new(flow_a.uuid.clone(), 0);
    ctx.flow_file_path = Some(flow_a_path.clone());
    ctx.call_stack.push(flow_b.uuid.clone());
    let subflow_node = &flow_a.nodes[1];
    let subflow_executor = flow_engine_executor::sub_flow_executor::SubFlowNodeExecutor::new();
    let runtime_res = flow_engine_core::executor::NodeExecutor::execute(
        &subflow_executor,
        subflow_node,
        &mut ctx,
        &flow_a.connections,
        &std::collections::HashMap::new(),
    )
    .await;
    assert!(
        runtime_res.is_err(),
        "Runtime execution must reject cycle in call_stack"
    );
    let err_msg = runtime_res.unwrap_err().message;
    assert!(
        err_msg.contains("Cycle detected in subflow call chain"),
        "Error message should mention cycle in call chain: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_subflow_indirect_cycle_static_validator() {
    let dir = tempdir().expect("create tempdir");
    let flow_a_path = dir.path().join("flow_a.json");
    let flow_b_path = dir.path().join("flow_b.json");

    // Flow A calls Flow B
    let flow_a = Flow {
        uuid: "flow-a-static".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Flow A Static".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("a-entry", "Entry A"),
            Node {
                uuid: "a-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Flow B".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: None,
                    target_flow_path: Some(flow_b_path.to_string_lossy().to_string()),
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
            id: "ca".to_string(),
            source_node_uuid: "a-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "a-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    // Flow B calls Flow A (indirect cycle)
    let flow_b = Flow {
        uuid: "flow-b-static".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Flow B Static".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("b-entry", "Entry B"),
            Node {
                uuid: "b-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Flow A".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: None,
                    target_flow_path: Some(flow_a_path.to_string_lossy().to_string()),
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
            id: "cb".to_string(),
            source_node_uuid: "b-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "b-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    write_flow_file(&flow_a_path, &flow_a);
    write_flow_file(&flow_b_path, &flow_b);

    let validator = FlowValidator::new();
    let val_result = validator.validate_with_dirs(&flow_a, &[dir.path().to_path_buf()]);

    assert!(
        !val_result.valid,
        "Static validation must fail on indirect cycle"
    );
    let cycle_err = val_result
        .errors
        .iter()
        .find(|e| e.code == ErrorCode::E_VAL_CYCLE_DETECTED);
    assert!(cycle_err.is_some(), "Expected E_VAL_CYCLE_DETECTED error");
    let msg = &cycle_err.unwrap().message;
    assert!(
        msg.contains("Circular subflow dependency detected"),
        "Error message should mention circular dependency: {}",
        msg
    );
}

#[tokio::test]
async fn test_subflow_nested_3_levels_valid() {
    let dir = tempdir().expect("create tempdir");
    let flow_a_path = dir.path().join("flow_a.json");
    let flow_b_path = dir.path().join("flow_b.json");
    let flow_c_path = dir.path().join("flow_c.json");

    // Flow C: Entry -> End
    let flow_c = Flow {
        uuid: "flow-c-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Flow C".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("c-entry", "Entry C"),
            create_end_node("c-end", "End C"),
        ],
        connections: vec![Connection {
            id: "c-conn".to_string(),
            source_node_uuid: "c-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "c-end".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    // Flow B calls Flow C
    let flow_b = Flow {
        uuid: "flow-b-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Flow B".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("b-entry", "Entry B"),
            Node {
                uuid: "b-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Flow C".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: None,
                    target_flow_path: Some(flow_c_path.to_string_lossy().to_string()),
                    inherit_all_variables: true,
                    variable_mappings: vec![],
                    export_all_variables: true,
                    output_mappings: vec![],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            },
            create_end_node("b-end", "End B"),
        ],
        connections: vec![
            Connection {
                id: "b-c1".to_string(),
                source_node_uuid: "b-entry".to_string(),
                source_port_id: "out".to_string(),
                target_node_uuid: "b-subflow".to_string(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "b-c2".to_string(),
                source_node_uuid: "b-subflow".to_string(),
                source_port_id: "out".to_string(),
                target_node_uuid: "b-end".to_string(),
                target_port_id: "in".to_string(),
            },
        ],
        hooks: None,
    };

    // Flow A calls Flow B
    let flow_a = Flow {
        uuid: "flow-a-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Flow A".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("a-entry", "Entry A"),
            Node {
                uuid: "a-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Flow B".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: None,
                    target_flow_path: Some(flow_b_path.to_string_lossy().to_string()),
                    inherit_all_variables: true,
                    variable_mappings: vec![],
                    export_all_variables: true,
                    output_mappings: vec![],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            },
            create_end_node("a-end", "End A"),
        ],
        connections: vec![
            Connection {
                id: "a-c1".to_string(),
                source_node_uuid: "a-entry".to_string(),
                source_port_id: "out".to_string(),
                target_node_uuid: "a-subflow".to_string(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "a-c2".to_string(),
                source_node_uuid: "a-subflow".to_string(),
                source_port_id: "out".to_string(),
                target_node_uuid: "a-end".to_string(),
                target_port_id: "in".to_string(),
            },
        ],
        hooks: None,
    };

    write_flow_file(&flow_a_path, &flow_a);
    write_flow_file(&flow_b_path, &flow_b);
    write_flow_file(&flow_c_path, &flow_c);

    let validator = FlowValidator::new();
    let val_result = validator.validate_with_dirs(&flow_a, &[dir.path().to_path_buf()]);

    assert!(
        val_result.valid,
        "3-level nested flow should pass validation: {:?}",
        val_result.errors
    );
    let cycle_err = val_result
        .errors
        .iter()
        .find(|e| e.code == ErrorCode::E_VAL_CYCLE_DETECTED);
    assert!(cycle_err.is_none(), "Expected no cycle error");
}
#[tokio::test]
async fn test_subflow_failure_routing() {
    let dir = tempdir().expect("create tempdir");
    let child_path = dir.path().join("failing_child.json");

    // 子流程：包含一个抛出错误的脚本
    let child_flow = Flow {
        uuid: "failing-child-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Failing Child Flow".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("c-entry", "Entry"),
            create_script_node(
                "c-script",
                "Failing Script",
                "throw new Error('child failure');",
            ),
        ],
        connections: vec![Connection {
            id: "c1".to_string(),
            source_node_uuid: "c-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "c-script".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    write_flow_file(&child_path, &child_flow);

    // 父流程：entry -> sub_flow. false 端口连接到 recovery script
    let parent_flow = Flow {
        uuid: "parent-recovery-flow".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Parent Recovery Flow".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("p-entry", "Entry"),
            Node {
                uuid: "p-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "SubFlow Call".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: None,
                    target_flow_path: Some(child_path.to_string_lossy().to_string()),
                    inherit_all_variables: true,
                    variable_mappings: vec![],
                    export_all_variables: true,
                    output_mappings: vec![],
                }),
                ports: Some(vec![
                    Port {
                        id: "in".to_string(),
                        port_type: PortType::Input,
                        label: Some("Input".to_string()),
                    },
                    Port {
                        id: "out".to_string(),
                        port_type: PortType::Output,
                        label: Some("Success".to_string()),
                    },
                    Port {
                        id: "false".to_string(),
                        port_type: PortType::Output,
                        label: Some("Failed".to_string()),
                    },
                ]),
                has_breakpoint: None,
                on_error: Some(ExecutionErrorStrategy::RouteToFalse),
            },
            create_script_node(
                "p-recover",
                "Recovery Script",
                "fn.variables.set('recovered', 'true');",
            ),
        ],
        connections: vec![
            Connection {
                id: "c-p-1".to_string(),
                source_node_uuid: "p-entry".to_string(),
                source_port_id: "out".to_string(),
                target_node_uuid: "p-subflow".to_string(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "c-p-false".to_string(),
                source_node_uuid: "p-subflow".to_string(),
                source_port_id: "false".to_string(),
                target_node_uuid: "p-recover".to_string(),
                target_port_id: "in".to_string(),
            },
        ],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&parent_flow, None, None, None).await;
    let parent_report = result.flow_report.as_ref().expect("Parent flow report");
    assert!(
        parent_report
            .execution_path
            .contains(&"p-recover".to_string()),
        "Execution path should include recovery node"
    );
    let final_vars = parent_report
        .final_variables
        .clone()
        .expect("Final variables exist");
    assert_eq!(
        final_vars.get("recovered").and_then(|v| v.as_str()),
        Some("true")
    );
}

/// 回归：GUI 的流程目录可由设置指向任意工作区目录，按 ID 引用的子流程
/// 必须在该目录（而非默认数据目录）中解析，否则会出现「列表可见、运行时找不到」。
///
/// 注意：流程目录覆盖是进程级状态；本用例结束时复位，避免影响同进程内的其他用例。
#[tokio::test]
async fn test_subflow_resolved_by_id_in_configured_flows_dir() {
    let _guard = FLOWS_DIR_OVERRIDE_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let workspace = tempdir().expect("create tempdir");
    let nested_dir = workspace.path().join("xpack-common");
    std::fs::create_dir_all(&nested_dir).expect("create nested dir");

    let child_flow = Flow {
        uuid: "child-login-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Login".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("child-entry", "Child Entry"),
            create_script_node(
                "child-script",
                "Set Session",
                "fn.variables.set('session', 'session-cookie');",
            ),
        ],
        connections: vec![Connection {
            id: "c1".to_string(),
            source_node_uuid: "child-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "child-script".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };
    // 用生产同款存储写入，落在配置目录的子文件夹中（文件名由流程名生成，与 UUID 不同）
    flow_engine_executor::storage::save_flow_file(&child_flow, &nested_dir)
        .expect("save child flow into workspace");

    let _override_guard = FlowsDirOverride::set(workspace.path().to_path_buf());

    let parent_flow = Flow {
        uuid: "parent-flow-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Parent".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("parent-entry", "Parent Entry"),
            Node {
                uuid: "parent-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Login (sub_flow)".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: Some("child-login-uuid".to_string()),
                    target_flow_path: Some(String::new()),
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
            id: "p1".to_string(),
            source_node_uuid: "parent-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "parent-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    let result = FlowExecutor::new()
        .execute_flow(&parent_flow, None, None, None)
        .await;

    assert!(
        result.success,
        "subflow referenced by ID should resolve inside the configured flows dir: {:?}",
        result.error
    );
    let report = result.flow_report.expect("flow report");
    let node_report = report
        .node_reports
        .iter()
        .find(|r| r.node_uuid == "parent-subflow")
        .expect("subflow node report");
    let child_report = node_report
        .sub_flow_report
        .as_ref()
        .expect("nested child report");
    assert_eq!(child_report.flow_uuid, "child-login-uuid");
    assert_eq!(
        report
            .final_variables
            .as_ref()
            .and_then(|v| v.get("session"))
            .and_then(|v| v.as_str()),
        Some("session-cookie")
    );
}

/// 目标流程 ID 不存在时，错误信息需列出实际搜索过的目录，便于定位目录配置问题。
#[tokio::test]
async fn test_subflow_missing_id_reports_searched_dirs() {
    let _guard = FLOWS_DIR_OVERRIDE_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let workspace = tempdir().expect("create tempdir");
    let _override_guard = FlowsDirOverride::set(workspace.path().to_path_buf());

    let parent_flow = Flow {
        uuid: "parent-missing-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Parent Missing".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("parent-entry", "Parent Entry"),
            Node {
                uuid: "parent-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Missing (sub_flow)".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
                    target_flow_id: Some("f9762dd7-9857-44c1-b7bd-a64f91f2e74e".to_string()),
                    target_flow_path: Some(String::new()),
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
            id: "p1".to_string(),
            source_node_uuid: "parent-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "parent-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    let result = FlowExecutor::new()
        .execute_flow(&parent_flow, None, None, None)
        .await;

    flow_engine_executor::storage::set_flows_dir_override(None);

    let message = result
        .node_reports
        .iter()
        .find(|r| r.node_uuid == "parent-subflow")
        .and_then(|r| r.error.as_ref())
        .map(|e| e.message.clone())
        .expect("subflow error");
    assert!(
        message.contains("f9762dd7-9857-44c1-b7bd-a64f91f2e74e"),
        "error should name the missing flow: {}",
        message
    );
    assert!(
        message.contains(&workspace.path().display().to_string()),
        "error should list the searched directory: {}",
        message
    );
}

/// 收集执行事件的测试用发送器（验证事件通道的嵌套标记契约）
#[derive(Default)]
struct EventRecorder {
    events: Mutex<Vec<flow_engine_core::ExecutionEvent>>,
}

impl EventRecorder {
    fn take(&self) -> Vec<flow_engine_core::ExecutionEvent> {
        std::mem::take(&mut *self.events.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl flow_engine_core::ExecutionEventSender for EventRecorder {
    fn send_event(&self, event: flow_engine_core::ExecutionEvent) -> Result<(), String> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(event);
        Ok(())
    }
}

/// 子流程内部的事件必须带上 SubFlow 节点 UUID，顶层事件不带：
/// 前端据此把子流程视为**单个节点**（计数/高亮），而不会被子流程的
/// FlowStarted/FlowCompleted 重置顶层运行状态。
#[tokio::test]
async fn test_subflow_events_are_tagged_with_parent_node_uuid() {
    use flow_engine_core::ExecutionEvent;

    let dir = tempdir().expect("create tempdir");
    let child_path = dir.path().join("child_events.json");

    // 子流程：entry -> script
    let child_flow = Flow {
        uuid: "child-events-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Child Events".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("child-entry", "Child Entry"),
            create_script_node(
                "child-script",
                "Child Script",
                "fn.variables.set('ok', '1');",
            ),
        ],
        connections: vec![Connection {
            id: "child-c1".to_string(),
            source_node_uuid: "child-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "child-script".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };
    write_flow_file(&child_path, &child_flow);

    // 父流程：entry -> sub_flow
    let parent_flow = Flow {
        uuid: "parent-events-uuid".to_string(),
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        name: "Parent Events".to_string(),
        description: None,
        nodes: vec![
            create_entry_node("parent-entry", "Parent Entry"),
            Node {
                uuid: "parent-subflow".to_string(),
                node_type: "sub_flow".to_string(),
                name: "Call Child Events".to_string(),
                position: NodePosition::default(),
                data: NodeData::sub_flow(SubFlowConfig {
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
            id: "parent-c1".to_string(),
            source_node_uuid: "parent-entry".to_string(),
            source_port_id: "out".to_string(),
            target_node_uuid: "parent-subflow".to_string(),
            target_port_id: "in".to_string(),
        }],
        hooks: None,
    };

    let recorder = std::sync::Arc::new(EventRecorder::default());
    let result = FlowExecutor::new()
        .with_execution_event_sender(recorder.clone())
        .execute_flow(&parent_flow, None, None, None)
        .await;
    assert!(
        result.success,
        "parent execution failed: {:?}",
        result.error
    );

    let events = recorder.take();

    // --- FlowStarted：顶层 1 个（无标记），子流程 1 个（标记为 SubFlow 节点）---
    let flow_started: Vec<&ExecutionEvent> = events
        .iter()
        .filter(|e| matches!(e, ExecutionEvent::FlowStarted { .. }))
        .collect();
    assert_eq!(
        flow_started.len(),
        2,
        "expected parent + child FlowStarted, got {events:?}"
    );
    assert!(flow_started.iter().any(|e| matches!(
        e,
        ExecutionEvent::FlowStarted {
            parent_node_uuid: None,
            total_nodes: 2,
            ..
        }
    )));
    assert!(flow_started.iter().any(|e| matches!(
        e,
        ExecutionEvent::FlowStarted { parent_node_uuid: Some(p), total_nodes: 2, .. }
            if p == "parent-subflow"
    )));

    // --- NodeStarted：父节点无标记，子流程内部节点标记为 parent-subflow ---
    let started_parent = events.iter().filter(|e| {
        matches!(
            e,
            ExecutionEvent::NodeStarted { node_uuid, parent_node_uuid: None, .. }
                if node_uuid == "parent-entry" || node_uuid == "parent-subflow"
        )
    });
    assert_eq!(
        started_parent.count(),
        2,
        "parent nodes must emit untagged NodeStarted"
    );

    for child_uuid in ["child-entry", "child-script"] {
        assert!(
            events.iter().any(|e| matches!(
                e,
                ExecutionEvent::NodeStarted { node_uuid, parent_node_uuid: Some(p), .. }
                    if node_uuid == child_uuid && p == "parent-subflow"
            )),
            "child node {child_uuid} events must be tagged with the SubFlow node uuid"
        );
    }

    // --- NodeCompleted：子节点标记，父节点不标记 ---
    assert!(events.iter().any(|e| matches!(
        e,
        ExecutionEvent::NodeCompleted { report, parent_node_uuid: Some(p) }
            if report.node_uuid == "child-script" && p == "parent-subflow"
    )));
    let completed_untagged: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            ExecutionEvent::NodeCompleted {
                report,
                parent_node_uuid: None,
            } => Some(report.node_uuid.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        completed_untagged.len(),
        2,
        "only top-level nodes complete untagged, got {completed_untagged:?}"
    );
    assert!(completed_untagged.contains(&"parent-subflow".to_string()));

    // --- FlowCompleted：仅顶层 1 个，且其中子流程节点携带嵌套报告树 ---
    let completed: Vec<&ExecutionEvent> = events
        .iter()
        .filter(|e| matches!(e, ExecutionEvent::FlowCompleted { .. }))
        .collect();
    assert_eq!(completed.len(), 2, "parent + child flow completion events");
    assert!(completed
        .iter()
        .any(|e| matches!(e, ExecutionEvent::FlowCompleted { parent_node_uuid: Some(p), .. } if p == "parent-subflow")));

    let root_completed = completed
        .iter()
        .find_map(|e| match e {
            ExecutionEvent::FlowCompleted {
                report,
                parent_node_uuid: None,
            } => Some(report),
            _ => None,
        })
        .expect("untagged FlowCompleted");
    assert_eq!(
        root_completed.node_reports.len(),
        2,
        "subflow must count as a single node in the parent report"
    );
    let subflow_node = root_completed
        .node_reports
        .iter()
        .find(|r| r.node_uuid == "parent-subflow")
        .expect("subflow node report");
    assert!(
        subflow_node.sub_flow_report.is_some(),
        "subflow node report carries the nested report tree"
    );
}
