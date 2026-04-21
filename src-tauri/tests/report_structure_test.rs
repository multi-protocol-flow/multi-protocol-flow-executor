// 报告结构综合测试 - 测试所有报告结构的序列化/反序列化和向后兼容性
#![allow(missing_docs)]

use flow_engine::error::{ErrorDetail, ErrorKind};
use flow_engine::report::{
    ExecutionMode, ExecutionStatus, FlowExecutionReport, NodeExecutionReport, SkippedNodeInfo,
    VariableChange,
};
use flow_engine::statistics::FlowStatistics;
use serde_json::json;

// ============================================================================
// VariableChange 测试
// ============================================================================

/// 测试 VariableChange 所有字段序列化/反序列化
#[test]
fn test_report_structure_variable_change() {
    // 测试新创建的变量（old_value 为 None）
    let new_var = VariableChange {
        key: "new_var".to_string(),
        old_value: None,
        new_value: json!({"value": 123}),
    };

    // 序列化
    let json_str = serde_json::to_string(&new_var).expect("VariableChange 序列化失败");

    // 验证 old_value 不在 JSON 中（skip_serializing_if）
    assert!(!json_str.contains("old_value"));

    // 反序列化
    let deserialized: VariableChange =
        serde_json::from_str(&json_str).expect("VariableChange 反序列化失败");
    assert_eq!(deserialized.key, "new_var");
    assert!(deserialized.old_value.is_none());
    assert_eq!(deserialized.new_value, json!({"value": 123}));

    // 测试修改的变量（old_value 有值）
    let modified_var = VariableChange {
        key: "modified_var".to_string(),
        old_value: Some(json!("old")),
        new_value: json!("new"),
    };

    let json_str = serde_json::to_string(&modified_var).expect("序列化失败");
    assert!(json_str.contains("old_value"));
    assert!(json_str.contains("modified_var"));

    let deserialized: VariableChange = serde_json::from_str(&json_str).expect("反序列化失败");
    assert_eq!(deserialized.old_value, Some(json!("old")));
    assert_eq!(deserialized.new_value, json!("new"));
}

/// 测试 VariableChange 不同类型的值
#[test]
fn test_report_structure_variable_change_value_types() {
    // 字符串值
    let str_var = VariableChange {
        key: "str_key".to_string(),
        old_value: None,
        new_value: json!("string value"),
    };
    let json_str = serde_json::to_string(&str_var).unwrap();
    let deserialized: VariableChange = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.new_value, json!("string value"));

    // 数字值
    let num_var = VariableChange {
        key: "num_key".to_string(),
        old_value: Some(json!(42)),
        new_value: json!(100),
    };
    let json_str = serde_json::to_string(&num_var).unwrap();
    let deserialized: VariableChange = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.new_value, json!(100));

    // 数组值
    let arr_var = VariableChange {
        key: "arr_key".to_string(),
        old_value: None,
        new_value: json!([1, 2, 3]),
    };
    let json_str = serde_json::to_string(&arr_var).unwrap();
    let deserialized: VariableChange = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.new_value, json!([1, 2, 3]));

    // 嵌套对象
    let obj_var = VariableChange {
        key: "obj_key".to_string(),
        old_value: None,
        new_value: json!({"nested": {"deep": "value"}}),
    };
    let json_str = serde_json::to_string(&obj_var).unwrap();
    let deserialized: VariableChange = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.new_value, json!({"nested": {"deep": "value"}}));
}

// ============================================================================
// SkippedNodeInfo 测试
// ============================================================================

/// 测试 SkippedNodeInfo 序列化/反序列化
#[test]
fn test_report_structure_skipped_node_info() {
    let info = SkippedNodeInfo {
        node_uuid: "skipped-uuid-123".to_string(),
        node_name: "被跳过的节点".to_string(),
        reason: "上游条件不满足".to_string(),
    };

    // 序列化
    let json_str = serde_json::to_string(&info).expect("SkippedNodeInfo 序列化失败");
    assert!(json_str.contains("skipped-uuid-123"));
    assert!(json_str.contains("被跳过的节点"));
    assert!(json_str.contains("上游条件不满足"));

    // 反序列化
    let deserialized: SkippedNodeInfo =
        serde_json::from_str(&json_str).expect("SkippedNodeInfo 反序列化失败");
    assert_eq!(deserialized.node_uuid, "skipped-uuid-123");
    assert_eq!(deserialized.node_name, "被跳过的节点");
    assert_eq!(deserialized.reason, "上游条件不满足");

    // 从 JSON 值反序列化
    let json_value = json!({
        "node_uuid": "node-456",
        "node_name": "Test Node",
        "reason": "Conditional branch not taken"
    });
    let from_value: SkippedNodeInfo =
        serde_json::from_value(json_value).expect("从 JSON 值反序列化失败");
    assert_eq!(from_value.node_uuid, "node-456");
}

// ============================================================================
// NodeExecutionReport 测试
// ============================================================================

/// 测试 NodeExecutionReport 所有字段序列化/反序列化
#[test]
fn test_report_structure_node_execution_report() {
    // 创建包含所有字段的报告
    let report = NodeExecutionReport {
        node_uuid: "node-uuid-full".to_string(),
        node_type: "http".to_string(),
        node_name: "完整测试节点".to_string(),
        start_time: 1700000000000,
        end_time: 1700000001000,
        duration_ms: 1000,
        status: ExecutionStatus::Success,
        input_data: Some(json!({"request": "data"})),
        output_data: Some(json!({"response": "ok", "status": 200})),
        used_port_id: Some("out".to_string()),
        error: None,
        // 新字段
        resolved_config: Some(json!({
            "url": "https://api.example.com/users",
            "method": "GET",
            "timeout_ms": 30000
        })),
        variable_changes: Some(vec![
            VariableChange {
                key: "response".to_string(),
                old_value: None,
                new_value: json!({"data": []}),
            },
            VariableChange {
                key: "status_code".to_string(),
                old_value: Some(json!(null)),
                new_value: json!(200),
            },
        ]),
        skip_reason: None,
        wait_duration_ms: Some(150),
    };

    // 序列化
    let json_str = serde_json::to_string(&report).expect("NodeExecutionReport 序列化失败");

    // 验证所有字段都在 JSON 中
    assert!(json_str.contains("node_uuid"));
    assert!(json_str.contains("node_type"));
    assert!(json_str.contains("node_name"));
    assert!(json_str.contains("start_time"));
    assert!(json_str.contains("end_time"));
    assert!(json_str.contains("duration_ms"));
    assert!(json_str.contains("status"));
    assert!(json_str.contains("input_data"));
    assert!(json_str.contains("output_data"));
    assert!(json_str.contains("used_port_id"));
    assert!(json_str.contains("resolved_config"));
    assert!(json_str.contains("variable_changes"));
    assert!(json_str.contains("wait_duration_ms"));

    // skip_reason 为 None，不应出现
    assert!(!json_str.contains("skip_reason"));

    // 反序列化
    let deserialized: NodeExecutionReport =
        serde_json::from_str(&json_str).expect("NodeExecutionReport 反序列化失败");

    // 验证所有字段
    assert_eq!(deserialized.node_uuid, "node-uuid-full");
    assert_eq!(deserialized.node_type, "http");
    assert_eq!(deserialized.node_name, "完整测试节点");
    assert_eq!(deserialized.start_time, 1700000000000);
    assert_eq!(deserialized.end_time, 1700000001000);
    assert_eq!(deserialized.duration_ms, 1000);
    assert_eq!(deserialized.status, ExecutionStatus::Success);
    assert!(deserialized.input_data.is_some());
    assert!(deserialized.output_data.is_some());
    assert_eq!(deserialized.used_port_id, Some("out".to_string()));
    assert!(deserialized.error.is_none());

    // 验证新字段
    assert!(deserialized.resolved_config.is_some());
    assert!(deserialized.variable_changes.is_some());
    assert_eq!(deserialized.variable_changes.as_ref().unwrap().len(), 2);
    assert!(deserialized.skip_reason.is_none());
    assert_eq!(deserialized.wait_duration_ms, Some(150));
}

/// 测试 NodeExecutionReport 失败状态
#[test]
fn test_report_structure_node_execution_report_failed() {
    let report = NodeExecutionReport {
        node_uuid: "failed-node".to_string(),
        node_type: "http".to_string(),
        node_name: "失败的 HTTP 请求".to_string(),
        start_time: 1000,
        end_time: 5000,
        duration_ms: 4000,
        status: ExecutionStatus::Failed,
        input_data: Some(json!({"url": "https://invalid.example.com"})),
        output_data: None,
        used_port_id: None,
        error: Some(ErrorDetail::new(
            ErrorKind::Connection,
            "连接超时".to_string(),
        )),
        resolved_config: Some(json!({"url": "https://invalid.example.com"})),
        variable_changes: None,
        skip_reason: None,
        wait_duration_ms: None,
    };

    let json_str = serde_json::to_string(&report).expect("序列化失败");
    assert!(json_str.contains("failed"));
    assert!(json_str.contains("连接超时"));

    let deserialized: NodeExecutionReport = serde_json::from_str(&json_str).expect("反序列化失败");
    assert_eq!(deserialized.status, ExecutionStatus::Failed);
    assert!(deserialized.error.is_some());
    assert_eq!(deserialized.error.as_ref().unwrap().message, "连接超时");
}

/// 测试 NodeExecutionReport 跳过状态
#[test]
fn test_report_structure_node_execution_report_skipped() {
    let report = NodeExecutionReport {
        node_uuid: "skipped-node".to_string(),
        node_type: "http".to_string(),
        node_name: "跳过的节点".to_string(),
        start_time: 1000,
        end_time: 1000,
        duration_ms: 0,
        status: ExecutionStatus::Skipped,
        input_data: None,
        output_data: None,
        used_port_id: None,
        error: None,
        resolved_config: None,
        variable_changes: None,
        skip_reason: Some("条件表达式评估为 false".to_string()),
        wait_duration_ms: None,
    };

    let json_str = serde_json::to_string(&report).expect("序列化失败");
    assert!(json_str.contains("skipped"));
    assert!(json_str.contains("skip_reason"));
    assert!(json_str.contains("条件表达式评估为 false"));

    let deserialized: NodeExecutionReport = serde_json::from_str(&json_str).expect("反序列化失败");
    assert_eq!(deserialized.status, ExecutionStatus::Skipped);
    assert_eq!(
        deserialized.skip_reason,
        Some("条件表达式评估为 false".to_string())
    );
}

// ============================================================================
// FlowExecutionReport 测试
// ============================================================================

/// 测试 FlowExecutionReport 所有字段序列化/反序列化
#[test]
fn test_report_structure_flow_execution_report() {
    // 创建节点报告
    let node_reports = vec![
        NodeExecutionReport {
            node_uuid: "entry-1".to_string(),
            node_type: "entry".to_string(),
            node_name: "入口".to_string(),
            start_time: 1000,
            end_time: 1001,
            duration_ms: 1,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(json!({"started": true})),
            used_port_id: Some("out".to_string()),
            error: None,
            resolved_config: None,
            variable_changes: Some(vec![VariableChange {
                key: "started".to_string(),
                old_value: None,
                new_value: json!(true),
            }]),
            skip_reason: None,
            wait_duration_ms: None,
        },
        NodeExecutionReport {
            node_uuid: "http-1".to_string(),
            node_type: "http".to_string(),
            node_name: "API 请求".to_string(),
            start_time: 1001,
            end_time: 2500,
            duration_ms: 1499,
            status: ExecutionStatus::Success,
            input_data: Some(json!({"started": true})),
            output_data: Some(json!({"users": []})),
            used_port_id: Some("out".to_string()),
            error: None,
            resolved_config: Some(json!({"url": "https://api.example.com/users"})),
            variable_changes: Some(vec![VariableChange {
                key: "users".to_string(),
                old_value: None,
                new_value: json!([]),
            }]),
            skip_reason: None,
            wait_duration_ms: Some(0),
        },
    ];

    // 创建跳过节点信息
    let skipped_nodes = vec![SkippedNodeInfo {
        node_uuid: "http-2".to_string(),
        node_name: "备用 API".to_string(),
        reason: "条件分支未命中".to_string(),
    }];

    // 创建完整报告
    let report = FlowExecutionReport {
        uuid: "report-uuid-12345".to_string(),
        flow_uuid: "flow-uuid-abc".to_string(),
        flow_name: "完整测试流程".to_string(),
        start_time: 1000,
        end_time: 3000,
        duration_ms: 2000,
        total_nodes: 3,
        executed_count: 2,
        status: ExecutionStatus::Success,
        node_reports,
        statistics: FlowStatistics {
            success_count: 2,
            failed_count: 0,
            avg_node_duration_ms: Some(750),
            max_node_duration_ms: Some(1499),
            min_node_duration_ms: Some(1),
            total_input_size_bytes: Some(20),
            total_output_size_bytes: Some(50),
            error_rate: Some(0.0),
            completion_rate: Some(0.6666666666666666),
        },
        flow_error: None,
        execution_mode: ExecutionMode::Normal,
        // 新字段
        initial_variables: Some(json!({
            "api_base": "https://api.example.com",
            "timeout": 30000
        })),
        final_variables: Some(json!({
            "api_base": "https://api.example.com",
            "timeout": 30000,
            "started": true,
            "users": []
        })),
        execution_path: vec!["entry-1".to_string(), "http-1".to_string()],
        skipped_nodes,
        ws_streams: vec![],
        flow_snapshot: None,
    };

    // 序列化
    let json_str = serde_json::to_string(&report).expect("FlowExecutionReport 序列化失败");

    // 验证所有字段
    assert!(json_str.contains("uuid"));
    assert!(json_str.contains("flow_uuid"));
    assert!(json_str.contains("flow_name"));
    assert!(json_str.contains("start_time"));
    assert!(json_str.contains("end_time"));
    assert!(json_str.contains("duration_ms"));
    assert!(json_str.contains("total_nodes"));
    assert!(json_str.contains("executed_count"));
    assert!(json_str.contains("status"));
    assert!(json_str.contains("node_reports"));
    assert!(json_str.contains("statistics"));
    assert!(json_str.contains("execution_mode"));

    // 验证新字段
    assert!(json_str.contains("initial_variables"));
    assert!(json_str.contains("final_variables"));
    assert!(json_str.contains("execution_path"));
    assert!(json_str.contains("skipped_nodes"));

    // 反序列化
    let deserialized: FlowExecutionReport =
        serde_json::from_str(&json_str).expect("FlowExecutionReport 反序列化失败");

    assert_eq!(deserialized.uuid, "report-uuid-12345");
    assert_eq!(deserialized.flow_uuid, "flow-uuid-abc");
    assert_eq!(deserialized.flow_name, "完整测试流程");
    assert_eq!(deserialized.total_nodes, 3);
    assert_eq!(deserialized.executed_count, 2);
    assert_eq!(deserialized.status, ExecutionStatus::Success);
    assert_eq!(deserialized.node_reports.len(), 2);
    assert_eq!(deserialized.execution_mode, ExecutionMode::Normal);

    // 验证新字段
    assert!(deserialized.initial_variables.is_some());
    assert!(deserialized.final_variables.is_some());
    assert_eq!(deserialized.execution_path.len(), 2);
    assert_eq!(deserialized.skipped_nodes.len(), 1);
    assert_eq!(deserialized.skipped_nodes[0].node_uuid, "http-2");
}

/// 测试 FlowExecutionReport 失败状态
#[test]
fn test_report_structure_flow_execution_report_failed() {
    let report = FlowExecutionReport {
        uuid: "failed-report".to_string(),
        flow_uuid: "flow-failed".to_string(),
        flow_name: "失败的流程".to_string(),
        start_time: 1000,
        end_time: 5000,
        duration_ms: 4000,
        total_nodes: 3,
        executed_count: 1,
        status: ExecutionStatus::Failed,
        node_reports: vec![],
        statistics: FlowStatistics {
            success_count: 0,
            failed_count: 1,
            avg_node_duration_ms: Some(4000),
            max_node_duration_ms: Some(4000),
            min_node_duration_ms: Some(4000),
            total_input_size_bytes: None,
            total_output_size_bytes: None,
            error_rate: Some(1.0),
            completion_rate: Some(0.3333333333333333),
        },
        flow_error: Some(ErrorDetail::new(
            ErrorKind::Execution,
            "流程执行失败".to_string(),
        )),
        execution_mode: ExecutionMode::Normal,
        initial_variables: Some(json!({})),
        final_variables: None,
        execution_path: vec!["entry-1".to_string()],
        skipped_nodes: vec![],
        ws_streams: vec![],
        flow_snapshot: None,
    };

    let json_str = serde_json::to_string(&report).expect("序列化失败");
    assert!(json_str.contains("failed"));
    assert!(json_str.contains("flow_error"));

    let deserialized: FlowExecutionReport = serde_json::from_str(&json_str).expect("反序列化失败");
    assert_eq!(deserialized.status, ExecutionStatus::Failed);
    assert!(deserialized.flow_error.is_some());
}

/// 测试 FlowExecutionReport 调试模式
#[test]
fn test_report_structure_flow_execution_report_debug_mode() {
    let report = FlowExecutionReport {
        uuid: "debug-report".to_string(),
        flow_uuid: "flow-debug".to_string(),
        flow_name: "调试流程".to_string(),
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        total_nodes: 1,
        executed_count: 1,
        status: ExecutionStatus::Success,
        node_reports: vec![],
        statistics: FlowStatistics::default(),
        flow_error: None,
        execution_mode: ExecutionMode::Debug,
        initial_variables: None,
        final_variables: None,
        execution_path: vec![],
        skipped_nodes: vec![],
        ws_streams: vec![],
        flow_snapshot: None,
    };

    let json_str = serde_json::to_string(&report).expect("序列化失败");

    let deserialized: FlowExecutionReport = serde_json::from_str(&json_str).expect("反序列化失败");
    assert_eq!(deserialized.execution_mode, ExecutionMode::Debug);
}

// ============================================================================
// 向后兼容性测试
// ============================================================================

/// 测试向后兼容性：旧版 NodeExecutionReport JSON 可反序列化
#[test]
fn test_report_structure_backward_compat_node_report() {
    // 模拟旧版 JSON（不包含新字段）
    let old_json = json!({
        "node_uuid": "old-node-uuid",
        "node_type": "entry",
        "node_name": "旧版入口节点",
        "start_time": 1000,
        "end_time": 2000,
        "duration_ms": 1000,
        "status": "success",
        "input_data": null,
        "output_data": {"started": true},
        "used_port_id": "out",
        "error": null
    });

    // 反序列化应该成功
    let report: NodeExecutionReport =
        serde_json::from_value(old_json).expect("旧版 NodeExecutionReport JSON 反序列化失败");

    // 验证基本字段
    assert_eq!(report.node_uuid, "old-node-uuid");
    assert_eq!(report.node_type, "entry");
    assert_eq!(report.node_name, "旧版入口节点");
    assert_eq!(report.status, ExecutionStatus::Success);

    // 验证新字段使用默认值
    assert!(report.resolved_config.is_none());
    assert!(report.variable_changes.is_none());
    assert!(report.skip_reason.is_none());
    assert!(report.wait_duration_ms.is_none());
}

/// 测试向后兼容性：旧版 FlowExecutionReport JSON 可反序列化
#[test]
fn test_report_structure_backward_compat_flow_report() {
    // 模拟旧版 JSON（不包含新字段）
    let old_json = json!({
        "uuid": "old-report-uuid",
        "flow_uuid": "old-flow-uuid",
        "flow_name": "旧版流程",
        "start_time": 1000,
        "end_time": 3000,
        "duration_ms": 2000,
        "total_nodes": 2,
        "executed_count": 2,
        "status": "success",
        "node_reports": [
            {
                "node_uuid": "node-1",
                "node_type": "entry",
                "node_name": "入口",
                "start_time": 1000,
                "end_time": 1001,
                "duration_ms": 1,
                "status": "success",
                "input_data": null,
                "output_data": {"started": true},
                "used_port_id": "out",
                "error": null
            }
        ],
        "statistics": {
            "success_count": 1,
            "failed_count": 0,
            "avg_node_duration_ms": 1,
            "max_node_duration_ms": 1,
            "min_node_duration_ms": 1,
            "total_input_size_bytes": 0,
            "total_output_size_bytes": 10,
            "error_rate": 0.0,
            "completion_rate": 0.5
        },
        "flow_error": null,
        "execution_mode": "normal"
    });

    // 反序列化应该成功
    let report: FlowExecutionReport =
        serde_json::from_value(old_json).expect("旧版 FlowExecutionReport JSON 反序列化失败");

    // 验证基本字段
    assert_eq!(report.uuid, "old-report-uuid");
    assert_eq!(report.flow_uuid, "old-flow-uuid");
    assert_eq!(report.flow_name, "旧版流程");
    assert_eq!(report.status, ExecutionStatus::Success);
    assert_eq!(report.node_reports.len(), 1);

    // 验证新字段使用默认值
    assert!(report.initial_variables.is_none());
    assert!(report.final_variables.is_none());
    assert_eq!(report.execution_path.len(), 0);
    assert_eq!(report.skipped_nodes.len(), 0);
}

/// 测试向后兼容性：部分缺失字段的 JSON
#[test]
fn test_report_structure_backward_compat_partial_fields() {
    // 只有部分新字段的 JSON
    let partial_json = json!({
        "uuid": "partial-report",
        "flow_uuid": "flow-uuid",
        "flow_name": "部分字段流程",
        "start_time": 1000,
        "end_time": 2000,
        "duration_ms": 1000,
        "total_nodes": 1,
        "executed_count": 1,
        "status": "success",
        "node_reports": [],
        "statistics": {
            "success_count": 1,
            "failed_count": 0
        },
        "flow_error": null,
        "execution_mode": "normal",
        // 只有 initial_variables，没有其他新字段
        "initial_variables": {"var1": "value1"}
    });

    let report: FlowExecutionReport =
        serde_json::from_value(partial_json).expect("部分字段 JSON 反序列化失败");

    assert!(report.initial_variables.is_some());
    assert!(report.final_variables.is_none());
    assert_eq!(report.execution_path.len(), 0);
    assert_eq!(report.skipped_nodes.len(), 0);
}

// ============================================================================
// ExecutionStatus 和 ExecutionMode 枚举测试
// ============================================================================

/// 测试 ExecutionStatus 序列化格式
#[test]
fn test_report_structure_execution_status() {
    // 测试小写序列化
    assert_eq!(
        serde_json::to_string(&ExecutionStatus::Success).unwrap(),
        "\"success\""
    );
    assert_eq!(
        serde_json::to_string(&ExecutionStatus::Failed).unwrap(),
        "\"failed\""
    );
    assert_eq!(
        serde_json::to_string(&ExecutionStatus::Skipped).unwrap(),
        "\"skipped\""
    );

    // 测试反序列化
    let success: ExecutionStatus = serde_json::from_str("\"success\"").unwrap();
    assert_eq!(success, ExecutionStatus::Success);

    let failed: ExecutionStatus = serde_json::from_str("\"failed\"").unwrap();
    assert_eq!(failed, ExecutionStatus::Failed);

    let skipped: ExecutionStatus = serde_json::from_str("\"skipped\"").unwrap();
    assert_eq!(skipped, ExecutionStatus::Skipped);
}

/// 测试 ExecutionMode 序列化格式
#[test]
fn test_report_structure_execution_mode() {
    // 测试小写序列化
    assert_eq!(
        serde_json::to_string(&ExecutionMode::Normal).unwrap(),
        "\"normal\""
    );
    assert_eq!(
        serde_json::to_string(&ExecutionMode::Debug).unwrap(),
        "\"debug\""
    );

    // 测试反序列化
    let normal: ExecutionMode = serde_json::from_str("\"normal\"").unwrap();
    assert_eq!(normal, ExecutionMode::Normal);

    let debug: ExecutionMode = serde_json::from_str("\"debug\"").unwrap();
    assert_eq!(debug, ExecutionMode::Debug);

    // 测试默认值
    assert_eq!(ExecutionMode::default(), ExecutionMode::Normal);
}

// ============================================================================
// 复杂场景测试
// ============================================================================

/// 测试完整的端到端序列化/反序列化流程
#[test]
fn test_report_structure_full_roundtrip() {
    // 创建一个复杂的报告
    let original = FlowExecutionReport {
        uuid: "roundtrip-test".to_string(),
        flow_uuid: "flow-roundtrip".to_string(),
        flow_name: "完整往返测试".to_string(),
        start_time: 1700000000000,
        end_time: 1700000010000,
        duration_ms: 10000,
        total_nodes: 5,
        executed_count: 3,
        status: ExecutionStatus::Success,
        node_reports: vec![
            NodeExecutionReport {
                node_uuid: "entry".to_string(),
                node_type: "entry".to_string(),
                node_name: "入口".to_string(),
                start_time: 0,
                end_time: 1,
                duration_ms: 1,
                status: ExecutionStatus::Success,
                input_data: None,
                output_data: Some(json!({})),
                used_port_id: Some("out".to_string()),
                error: None,
                resolved_config: None,
                variable_changes: None,
                skip_reason: None,
                wait_duration_ms: None,
            },
            NodeExecutionReport {
                node_uuid: "http-1".to_string(),
                node_type: "http".to_string(),
                node_name: "获取用户".to_string(),
                start_time: 1,
                end_time: 500,
                duration_ms: 499,
                status: ExecutionStatus::Success,
                input_data: Some(json!({})),
                output_data: Some(json!({"users": [{"id": 1}]})),
                used_port_id: Some("out".to_string()),
                error: None,
                resolved_config: Some(json!({"url": "https://api.example.com/users"})),
                variable_changes: Some(vec![VariableChange {
                    key: "users".to_string(),
                    old_value: None,
                    new_value: json!([{"id": 1}]),
                }]),
                skip_reason: None,
                wait_duration_ms: Some(0),
            },
            NodeExecutionReport {
                node_uuid: "cond-1".to_string(),
                node_type: "conditional".to_string(),
                node_name: "检查用户数".to_string(),
                start_time: 500,
                end_time: 501,
                duration_ms: 1,
                status: ExecutionStatus::Success,
                input_data: Some(json!({"users": [{"id": 1}]})),
                output_data: None,
                used_port_id: Some("true".to_string()),
                error: None,
                resolved_config: Some(json!({"condition": "users.length > 0"})),
                variable_changes: None,
                skip_reason: None,
                wait_duration_ms: Some(0),
            },
        ],
        statistics: FlowStatistics {
            success_count: 3,
            failed_count: 0,
            avg_node_duration_ms: Some(167),
            max_node_duration_ms: Some(499),
            min_node_duration_ms: Some(1),
            total_input_size_bytes: Some(30),
            total_output_size_bytes: Some(50),
            error_rate: Some(0.0),
            completion_rate: Some(0.6),
        },
        flow_error: None,
        execution_mode: ExecutionMode::Normal,
        initial_variables: Some(json!({"env": "production"})),
        final_variables: Some(json!({"env": "production", "users": [{"id": 1}]})),
        execution_path: vec![
            "entry".to_string(),
            "http-1".to_string(),
            "cond-1".to_string(),
        ],
        skipped_nodes: vec![
            SkippedNodeInfo {
                node_uuid: "http-2".to_string(),
                node_name: "备用 API".to_string(),
                reason: "条件分支未命中".to_string(),
            },
            SkippedNodeInfo {
                node_uuid: "http-3".to_string(),
                node_name: "其他 API".to_string(),
                reason: "上游节点未执行".to_string(),
            },
        ],
        ws_streams: vec![],
        flow_snapshot: None,
    };

    // 序列化
    let json_str = serde_json::to_string(&original).expect("序列化失败");

    // 反序列化
    let restored: FlowExecutionReport = serde_json::from_str(&json_str).expect("反序列化失败");

    // 验证关键字段
    assert_eq!(restored.uuid, original.uuid);
    assert_eq!(restored.flow_uuid, original.flow_uuid);
    assert_eq!(restored.flow_name, original.flow_name);
    assert_eq!(restored.status, original.status);
    assert_eq!(restored.total_nodes, original.total_nodes);
    assert_eq!(restored.executed_count, original.executed_count);
    assert_eq!(restored.node_reports.len(), original.node_reports.len());
    assert_eq!(restored.execution_path, original.execution_path);
    assert_eq!(restored.skipped_nodes.len(), original.skipped_nodes.len());

    // 验证嵌套的 variable_changes
    let http_report = &restored.node_reports[1];
    assert!(http_report.variable_changes.is_some());
    let changes = http_report.variable_changes.as_ref().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].key, "users");
}
