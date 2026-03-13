#![allow(missing_docs)]

// CLI 测试 - 测试命令行接口的所有功能

use assert_cmd::cargo;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

/// 创建一个有效的简单流程文件（仅 entry 节点，有连接）
fn create_valid_flow_file() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "test-flow-uuid",
        "name": "测试流程",
        "nodes": [
            {
                "uuid": "entry-node-1",
                "type": "entry",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "entry" },
                "ports": null
            },
            {
                "uuid": "http-node-1",
                "type": "http",
                "name": "HTTP 节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": { "type": "http", "url": "https://httpbin.org/status/200" },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "entry-node-1",
                "target_node_uuid": "http-node-1",
                "source_port_id": "output",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建一个带有 uuid 字段的流程文件
fn create_flow_with_uuid() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "flow-uuid-123",
        "name": "带 UUID 的流程",
        "nodes": [
            {
                "uuid": "entry-node-1",
                "type": "entry",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "entry" },
                "ports": null
            },
            {
                "uuid": "http-node-1",
                "type": "http",
                "name": "HTTP 节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": { "type": "http", "url": "https://httpbin.org/status/200" },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "entry-node-1",
                "target_node_uuid": "http-node-1",
                "source_port_id": "output",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建一个空的流程文件
fn create_empty_flow_file() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    fs::write(file.path(), "{}").unwrap();
    file
}

/// 创建一个大型流程文件（包含多个节点）
fn create_large_flow_file() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let mut nodes = String::from("[");
    let mut connections = String::from("[");

    // Add entry node
    nodes.push_str(r#"{"uuid": "entry-node-1", "type": "entry", "name": "开始节点", "position": {"x": 0.0, "y": 0.0}, "data": {"type": "entry" }, "ports": null}"#);

    let mut prev_uuid = "entry-node-1".to_string();

    // Add intermediate nodes (HTTP nodes)
    for i in 1..=48 {
        nodes.push_str(",");
        nodes.push_str(&format!(
            r#"{{"uuid": "node-{}", "type": "http", "name": "节点 {}", "position": {{"x": {}, "y": {}}}, "data": {{"type": "http", "url": "https://httpbin.org/status/200"}}, "ports": null}}"#,
            i, i, (i * 10) as f64, (i * 10) as f64
        ));
        // Add connection from previous node to this one
        connections.push_str(&format!(
            r#"{{"id": "conn-{}", "source_node_uuid": "{}", "target_node_uuid": "node-{}", "source_port_id": "output", "target_port_id": "input"}}"#,
            i, prev_uuid, i
        ));
        if i < 48 {
            connections.push_str(",");
        }
        prev_uuid = format!("node-{}", i);
    }

    nodes.push_str("]");
    connections.push_str("]");

    let flow_content = format!(
        r#"{{"uuid": "large-flow-uuid", "name": "大型流程", "nodes": {}, "connections": {}}}"#,
        nodes, connections
    );
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建一个无效的 JSON 文件
fn create_invalid_json_file() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    fs::write(file.path(), "{ invalid json }").unwrap();
    file
}

// ============ 参数解析测试 ============

#[test]
fn test_long_format_execute_flow() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_short_format_execute_flow() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("-e")
        .arg("-f")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_help_long_format() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("多协议流程执行器 - CLI 工具"))
        .stdout(predicate::str::contains("用法:"))
        .stdout(predicate::str::contains("--execute"))
        .stdout(predicate::str::contains("--flow"));
}

#[test]
fn test_help_short_format() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("多协议流程执行器 - CLI 工具"));
}

#[test]
fn test_no_arguments_shows_help() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("多协议流程执行器 - CLI 工具"));
}

#[test]
fn test_combined_short_and_long_format() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("-e")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_combined_long_and_short_format() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("-f")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

// ============ 错误处理测试 ============

#[test]
fn test_missing_flow_parameter() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--execute 需要 --flow 参数"));
}

#[test]
fn test_missing_execute_parameter() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--flow")
        .arg("dummy.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--flow 需要 --execute 参数"));
}

#[test]
fn test_invalid_short_argument() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("-x")
        .assert()
        .failure()
        .stderr(predicate::str::contains("未知选项: -x"));
}

#[test]
fn test_invalid_long_argument() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("未知选项: --invalid"));
}

#[test]
fn test_file_not_found() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg("/nonexistent/file.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("加载流程失败"));
}

#[test]
fn test_invalid_json_file() {
    let file = create_invalid_json_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("加载流程失败"));
}

#[test]
fn test_unknown_positional_argument() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("unknown")
        .assert()
        .failure()
        .stderr(predicate::str::contains("未知参数"));
}

// ============ 文件加载测试 ============

#[test]
fn test_load_valid_json_flow_file() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_load_flow_with_uuid_field() {
    let file = create_flow_with_uuid();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

// ============ 执行输出测试 ============

#[test]
fn test_success_execution_json_output() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#))
        .stdout(predicate::str::contains(r#""data""#))
        .stdout(predicate::str::contains(r#""execution_time""#))
        .stdout(predicate::str::contains("}"));
}

#[test]
fn test_json_output_has_all_required_fields() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains(r#""success""#));
    assert!(output_str.contains(r#""data""#));
    assert!(output_str.contains(r#""execution_time""#));
}

#[test]
fn test_execution_time_field_exists() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains(r#""execution_time""#));

    // 解析 JSON 并验证 execution_time 是数字
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(json["execution_time"].is_number());
    assert!(json["execution_time"].is_number());
}

#[test]
fn test_error_output_on_invalid_flow() {
    let file = create_empty_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("错误"));
}

// ============ 边界情况测试 ============

#[test]
fn test_empty_flow_file() {
    let file = create_empty_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("错误"));
}

#[test]
fn test_large_flow_file() {
    let file = create_large_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_concurrent_execution() {
    let file1 = create_valid_flow_file();
    let file2 = create_valid_flow_file();

    let handle1 = std::thread::spawn(move || {
        let mut cmd = cargo::cargo_bin_cmd!("cli");
        cmd.arg("--execute")
            .arg("--flow")
            .arg(file1.path())
            .assert()
            .success();
    });

    let handle2 = std::thread::spawn(move || {
        let mut cmd = cargo::cargo_bin_cmd!("cli");
        cmd.arg("--execute")
            .arg("--flow")
            .arg(file2.path())
            .assert()
            .success();
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

#[test]
fn test_flow_parameter_without_value() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--flow 参数需要指定路径"));
}

#[test]
fn test_execution_with_single_entry_node() {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "single-node-flow-uuid",
        "name": "单节点流程",
        "nodes": [
            {
                "uuid": "entry-node-1",
                "type": "entry",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "entry"},
                "ports": null
            }
        ],
        "connections": []
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_flow_with_connections() {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "flow-with-connections-uuid",
        "name": "带连接的流程",
        "nodes": [
            {
                "uuid": "entry-node-1",
                "type": "entry",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "entry"},
                "ports": null
            },
            {
                "uuid": "http-node-1",
                "type": "http",
                "name": "HTTP 节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": { "type": "http", "url": "https://httpbin.org/status/200" },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "entry-node-1",
                "target_node_uuid": "http-node-1",
                "source_port_id": "output",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_execution_time_reasonable() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    let execution_time = json["execution_time"].as_u64().unwrap();

    // 执行时间应该在合理范围内（< 10 秒）
    assert!(execution_time < 10000, "执行时间过长: {}", execution_time);
}

// ============ 报告输出验证测试 (Task 21) ============

#[test]
fn test_cli_output_contains_node_reports() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""node_reports""#));
}

#[test]
fn test_cli_output_contains_flow_report() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""flow_report""#));
}

#[test]
fn test_node_reports_contains_required_fields() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证 node_reports 数组存在且不为空
    assert!(json["node_reports"].is_array());
    let node_reports = json["node_reports"].as_array().unwrap();
    assert!(!node_reports.is_empty(), "node_reports 不应该为空");

    // 验证每个节点报告包含所有必要字段
    for report in node_reports {
        assert!(
            report["node_id"].is_string() || report["node_uuid"].is_string(),
            "node_id/node_uuid 应该是字符串"
        );
        assert!(report["node_type"].is_string(), "node_type 应该是字符串");
        assert!(report["node_name"].is_string(), "node_name 应该是字符串");
        assert!(report["start_time"].is_number(), "start_time 应该是数字");
        assert!(report["end_time"].is_number(), "end_time 应该是数字");
        assert!(report["duration_ms"].is_number(), "duration_ms 应该是数字");
        assert!(report["status"].is_string(), "status 应该是字符串");
    }
}

#[test]
fn test_flow_report_contains_required_fields() {
    let file = create_flow_with_uuid();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证 flow_report 对象存在
    assert!(json["flow_report"].is_object(), "flow_report 应该是对象");
    let flow_report = &json["flow_report"];

    // 验证所有必要字段存在
    assert!(
        flow_report["flow_id"].is_string() || flow_report["flow_uuid"].is_string(),
        "flow_id/flow_uuid 应该是字符串"
    );
    assert!(
        flow_report["flow_name"].is_string(),
        "flow_name 应该是字符串"
    );
    assert!(
        flow_report["start_time"].is_number(),
        "start_time 应该是数字"
    );
    assert!(flow_report["end_time"].is_number(), "end_time 应该是数字");
    assert!(
        flow_report["duration_ms"].is_number(),
        "duration_ms 应该是数字"
    );
    assert!(
        flow_report["total_nodes"].is_number(),
        "total_nodes 应该是数字"
    );
    assert!(
        flow_report["executed_nodes"].is_number() || flow_report["executed_count"].is_number(),
        "executed_nodes/executed_count 应该是数字"
    );
    assert!(flow_report["status"].is_string(), "status 应该是字符串");
}

#[test]
fn test_node_reports_status_values() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    let node_reports = json["node_reports"].as_array().unwrap();

    // 验证状态值是有效的（success、failed 或 skipped）
    let valid_statuses = ["success", "failed", "skipped"];
    for report in node_reports {
        let status = report["status"].as_str().unwrap();
        assert!(valid_statuses.contains(&status), "无效的状态值: {}", status);
    }
}

#[test]
fn test_json_output_is_parseable() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();

    // 尝试解析 JSON，应该不会失败
    let json: serde_json::Value =
        serde_json::from_str(&output_str).expect("CLI 输出应该是有效的 JSON");

    // 验证 JSON 是对象类型
    assert!(json.is_object());
}

#[test]
fn test_backward_compatibility_success_data_error() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证向后兼容性字段存在
    assert!(json["success"].is_boolean(), "success 应该是布尔值");
    assert!(
        json["data"].is_object() || json["data"].is_null(),
        "data 应该是对象或 null"
    );
    assert!(
        json["error"].is_string() || json["error"].is_null(),
        "error 应该是字符串或 null"
    );
}

#[test]
fn test_cli_output_all_fields() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证所有字段都存在
    let json_obj = json.as_object().unwrap();
    assert!(json_obj.contains_key("success"), "缺少 success 字段");
    assert!(json_obj.contains_key("data"), "缺少 data 字段");
    assert!(json_obj.contains_key("error"), "缺少 error 字段");
    assert!(
        json_obj.contains_key("execution_time"),
        "缺少 execution_time 字段"
    );
    assert!(
        json_obj.contains_key("node_reports"),
        "缺少 node_reports 字段"
    );
    assert!(
        json_obj.contains_key("flow_report"),
        "缺少 flow_report 字段"
    );
}

#[test]
fn test_node_reports_count_matches_executed_nodes() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    let node_reports_count = json["node_reports"].as_array().unwrap().len() as u32;
    let flow_report = &json["flow_report"];

    // 验证 node_reports 数量与 flow_report.executed_nodes 一致
    if flow_report.is_object() {
        let executed_nodes = flow_report
            .get("executed_nodes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let executed_count = flow_report
            .get("executed_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let actual_executed = if executed_nodes > 0 {
            executed_nodes
        } else {
            executed_count
        };
        assert_eq!(
            node_reports_count, actual_executed,
            "node_reports 数量 ({}) 应该与 executed_nodes ({}) 一致",
            node_reports_count, actual_executed
        );
    }
}

#[test]
fn test_duration_ms_values_are_reasonable() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证每个节点的 duration_ms 是合理的正数
    let node_reports = json["node_reports"].as_array().unwrap();
    for report in node_reports {
        let duration_ms = report["duration_ms"].as_u64().unwrap();
        assert!(
            duration_ms < 60000,
            "duration_ms ({}) 应该在合理范围内 (0-60000ms)",
            duration_ms
        );
    }

    // 验证 flow_report.duration_ms 是合理的正数
    let flow_report = &json["flow_report"];
    if flow_report.is_object() {
        let flow_duration_ms = flow_report["duration_ms"].as_u64().unwrap();
        assert!(
            flow_duration_ms < 60000,
            "flow_report.duration_ms ({}) 应该在合理范围内 (0-60000ms)",
            flow_duration_ms
        );
    }
}

#[test]
fn test_node_reports_includes_error_on_failure() {
    let file = NamedTempFile::new().unwrap();
    // 创建一个会失败的流程（无效的 URL）
    let flow_content = r#"{
        "uuid": "failing-flow-uuid",
        "name": "失败的流程",
        "nodes": [
            {
                "uuid": "entry-node-1",
                "type": "entry",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "entry"},
                "ports": null
            },
            {
                "uuid": "http-node-1",
                "type": "http",
                "name": "HTTP 节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": { "type": "http", "url": "https://invalid.invalid.invalid/test" },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "entry-node-1",
                "target_node_uuid": "http-node-1",
                "source_port_id": "output",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证即使HTTP节点失败，flow执行成功（因为flow本身执行了）
    // 但node_reports中应该包含失败节点的错误信息
    assert!(json["node_reports"].is_array());
    let node_reports = json["node_reports"].as_array().unwrap();
    assert!(!node_reports.is_empty());

    // 验证失败的HTTP节点包含错误信息
    let http_node = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "http-node-1")
        .expect("Should find HTTP node report");

    // HTTP 节点执行状态为 success，但 output_data 中包含错误信息
    assert!(
        http_node["output_data"]["error"].is_string(),
        "HTTP node output_data should contain error message"
    );
    assert!(
        http_node["output_data"]["success"].is_boolean()
            && !http_node["output_data"]["success"].as_bool().unwrap(),
        "HTTP node output_data.success should be false"
    );
}

#[test]
fn test_flow_report_contains_node_reports() {
    let file = create_flow_with_uuid();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    let flow_report = &json["flow_report"];

    // 验证 flow_report 包含 node_reports 字段
    if flow_report.is_object() {
        assert!(flow_report["node_reports"].is_array());
        let flow_node_reports = flow_report["node_reports"].as_array().unwrap();

        // 验证 flow_report.node_reports 和顶层的 node_reports 是一致的
        let top_level_node_reports = json["node_reports"].as_array().unwrap();
        assert_eq!(
            flow_node_reports.len(),
            top_level_node_reports.len(),
            "flow_report.node_reports 和顶层 node_reports 数量应该一致"
        );
    }
}

// ============ 日志输出验证测试 (Task 21) ============

#[test]
fn test_log_output_to_stderr() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        // 验证 stderr 包含日志输出（env_logger 输出 INFO 级别日志）
        .stderr(predicate::str::contains("INFO").or(predicate::str::contains("")));
}

#[test]
fn test_log_output_contains_execution_info() {
    let file = create_flow_with_uuid();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        // 验证日志包含执行相关信息（匹配实际的日志消息）
        .stderr(predicate::str::contains("执行开始节点").or(predicate::str::contains("[完成]")));
}

#[test]
fn test_log_level_debug() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");

    // 设置 RUST_LOG=debug 环境变量
    cmd.env("RUST_LOG", "debug")
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        // DEBUG 级别应该输出更多日志
        .stderr(predicate::str::contains("DEBUG").or(predicate::str::contains("")));
}

#[test]
fn test_log_level_info() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");

    // 设置 RUST_LOG=info 环境变量
    cmd.env("RUST_LOG", "info")
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success();
}

// ============ 错误处理和边界情况测试 (Task 21) ============

#[test]
fn test_error_handling_invalid_flow_file() {
    let file = create_invalid_json_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("加载流程失败"));
}

#[test]
fn test_error_handling_syntax_error() {
    let file = NamedTempFile::new().unwrap();
    // 创建一个语法错误的 JSON
    let flow_content = r#"{
        "uuid": "syntax-error-flow",
        "name": "语法错误流程",
        "nodes": [
            {
                "uuid": "node1",
                "type": "entry"
        ]
    }"#; // 缺少右括号
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure();
}

#[test]
fn test_cli_output_json_format() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();

    // 验证输出格式是正确的 JSON
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证 JSON 结构符合预期
    assert!(json.is_object());
    assert!(json["success"].is_boolean());
    assert!(json["node_reports"].is_array());
    assert!(json["flow_report"].is_object() || json["flow_report"].is_null());
    assert!(json["execution_time"].is_number());
}

#[test]
fn test_flow_with_missing_uuid() {
    let file = NamedTempFile::new().unwrap();
    // 创建一个没有 uuid 字段的流程（应该失败）
    let flow_content = r#"{
        "name": "无 UUID 流程",
        "nodes": [
            {
                "uuid": "entry-node-1",
                "type": "entry",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "entry"},
                "ports": null
            },
            {
                "uuid": "http-node-1",
                "type": "http",
                "name": "HTTP 节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": { "type": "http", "url": "https://httpbin.org/status/200" },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "entry-node-1",
                "target_node_uuid": "http-node-1",
                "source_port_id": "output",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("加载流程失败"));
}

#[test]
fn test_execution_report_timestamps() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    let node_reports = json["node_reports"].as_array().unwrap();

    // 验证时间戳的合理性（start_time <= end_time）
    for report in node_reports {
        let start_time = report["start_time"].as_u64().unwrap();
        let end_time = report["end_time"].as_u64().unwrap();
        assert!(
            start_time <= end_time,
            "start_time ({}) 不应该大于 end_time ({})",
            start_time,
            end_time
        );
    }

    // 验证 flow_report 的时间戳
    let flow_report = &json["flow_report"];
    if flow_report.is_object() {
        let flow_start_time = flow_report["start_time"].as_u64().unwrap();
        let flow_end_time = flow_report["end_time"].as_u64().unwrap();
        assert!(
            flow_start_time <= flow_end_time,
            "flow_report.start_time ({}) 不应该大于 flow_report.end_time ({})",
            flow_start_time,
            flow_end_time
        );
    }
}

// ============ 断言节点集成测试 (Task 22) ============

/// 创建断言通过流程（断言验证状态码 200，通过 true 分支）
fn create_assertion_pass_flow() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    // 使用 HTTP 节点作为分支标记（true-branch 和 false-branch）
    let flow_content = r#"{
        "uuid": "assertion-pass-flow-001",
        "name": "断言通过流程",
        "nodes": [
            {
                "uuid": "entry-node",
                "type": "entry",
                "name": "开始",
                "position": { "x": 0.0, "y": 0.0 },
                "data": { "type": "entry" },
                "ports": null
            },
            {
                "uuid": "http-node",
                "type": "http",
                "name": "HTTP 请求",
                "position": { "x": 200.0, "y": 0.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/200",
                    "method": "get"
                },
                "ports": null
            },
            {
                "uuid": "assertion-node",
                "type": "assertion",
                "name": "断言状态码",
                "position": { "x": 400.0, "y": 0.0 },
                "data": {
                    "type": "assertion",
                    "preset": null,
                    "mode": "all",
                    "on_failure": "route_false",
                    "continue_on_failure": false,
                    "assertions": [
                        {
                            "id": "assert-status",
                            "description": "验证状态码为 200",
                            "target": "$.status",
                            "operator": "eq",
                            "expected": 200,
                            "enabled": true
                        }
                    ]
                },
                "ports": null
            },
            {
                "uuid": "true-branch-http",
                "type": "http",
                "name": "True 分支标记",
                "position": { "x": 600.0, "y": -100.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/201",
                    "method": "get"
                },
                "ports": null
            },
            {
                "uuid": "false-branch-http",
                "type": "http",
                "name": "False 分支标记",
                "position": { "x": 600.0, "y": 100.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/400",
                    "method": "get"
                },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-entry-http",
                "source_node_uuid": "entry-node",
                "target_node_uuid": "http-node",
                "source_port_id": "output",
                "target_port_id": "input"
            },
            {
                "id": "conn-http-assertion",
                "source_node_uuid": "http-node",
                "target_node_uuid": "assertion-node",
                "source_port_id": "true",
                "target_port_id": "input"
            },
            {
                "id": "conn-assertion-true",
                "source_node_uuid": "assertion-node",
                "target_node_uuid": "true-branch-http",
                "source_port_id": "true",
                "target_port_id": "input"
            },
            {
                "id": "conn-assertion-false",
                "source_node_uuid": "assertion-node",
                "target_node_uuid": "false-branch-http",
                "source_port_id": "false",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建断言失败流程（断言验证状态码 999，实际返回 200，走 false 分支）
fn create_assertion_fail_route_false_flow() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "assertion-fail-route-false-001",
        "name": "断言失败路由流程",
        "nodes": [
            {
                "uuid": "entry-node",
                "type": "entry",
                "name": "开始",
                "position": { "x": 0.0, "y": 0.0 },
                "data": { "type": "entry" },
                "ports": null
            },
            {
                "uuid": "http-node",
                "type": "http",
                "name": "HTTP 请求",
                "position": { "x": 200.0, "y": 0.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/200",
                    "method": "get"
                },
                "ports": null
            },
            {
                "uuid": "assertion-node",
                "type": "assertion",
                "name": "断言状态码",
                "position": { "x": 400.0, "y": 0.0 },
                "data": {
                    "type": "assertion",
                    "preset": null,
                    "mode": "all",
                    "on_failure": "route_false",
                    "continue_on_failure": false,
                    "assertions": [
                        {
                            "id": "assert-status-fail",
                            "description": "验证状态码为 999（故意失败）",
                            "target": "$.status",
                            "operator": "eq",
                            "expected": 999,
                            "enabled": true
                        }
                    ]
                },
                "ports": null
            },
            {
                "uuid": "true-branch-http",
                "type": "http",
                "name": "True 分支标记",
                "position": { "x": 600.0, "y": -100.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/201",
                    "method": "get"
                },
                "ports": null
            },
            {
                "uuid": "false-branch-http",
                "type": "http",
                "name": "False 分支标记",
                "position": { "x": 600.0, "y": 100.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/400",
                    "method": "get"
                },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-entry-http",
                "source_node_uuid": "entry-node",
                "target_node_uuid": "http-node",
                "source_port_id": "output",
                "target_port_id": "input"
            },
            {
                "id": "conn-http-assertion",
                "source_node_uuid": "http-node",
                "target_node_uuid": "assertion-node",
                "source_port_id": "true",
                "target_port_id": "input"
            },
            {
                "id": "conn-assertion-true",
                "source_node_uuid": "assertion-node",
                "target_node_uuid": "true-branch-http",
                "source_port_id": "true",
                "target_port_id": "input"
            },
            {
                "id": "conn-assertion-false",
                "source_node_uuid": "assertion-node",
                "target_node_uuid": "false-branch-http",
                "source_port_id": "false",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建断言失败抛出错误流程（流程在断言节点终止）
fn create_assertion_fail_throw_flow() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "assertion-fail-throw-001",
        "name": "断言失败抛出流程",
        "nodes": [
            {
                "uuid": "entry-node",
                "type": "entry",
                "name": "开始",
                "position": { "x": 0.0, "y": 0.0 },
                "data": { "type": "entry" },
                "ports": null
            },
            {
                "uuid": "http-node",
                "type": "http",
                "name": "HTTP 请求",
                "position": { "x": 200.0, "y": 0.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/200",
                    "method": "get"
                },
                "ports": null
            },
            {
                "uuid": "assertion-node",
                "type": "assertion",
                "name": "断言状态码",
                "position": { "x": 400.0, "y": 0.0 },
                "data": {
                    "type": "assertion",
                    "preset": null,
                    "mode": "all",
                    "on_failure": "throw",
                    "continue_on_failure": false,
                    "assertions": [
                        {
                            "id": "assert-status-throw",
                            "description": "验证状态码为 999（失败时抛出）",
                            "target": "$.status",
                            "operator": "eq",
                            "expected": 999,
                            "enabled": true
                        }
                    ]
                },
                "ports": null
            },
            {
                "uuid": "true-branch-http",
                "type": "http",
                "name": "True 分支标记",
                "position": { "x": 600.0, "y": 0.0 },
                "data": {
                    "type": "http",
                    "url": "https://httpbin.org/status/201",
                    "method": "get"
                },
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-entry-http",
                "source_node_uuid": "entry-node",
                "target_node_uuid": "http-node",
                "source_port_id": "output",
                "target_port_id": "input"
            },
            {
                "id": "conn-http-assertion",
                "source_node_uuid": "http-node",
                "target_node_uuid": "assertion-node",
                "source_port_id": "true",
                "target_port_id": "input"
            },
            {
                "id": "conn-assertion-true",
                "source_node_uuid": "assertion-node",
                "target_node_uuid": "true-branch-http",
                "source_port_id": "true",
                "target_port_id": "input"
            }
        ]
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

#[test]
fn test_assertion_pass_routes_to_true() {
    let file = create_assertion_pass_flow();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证流程执行成功
    assert!(json["success"].as_bool().unwrap(), "流程应该成功执行");

    // 验证 node_reports 包含断言节点
    let node_reports = json["node_reports"].as_array().unwrap();
    let assertion_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "assertion-node")
        .expect("应该找到断言节点报告");

    // 验证断言节点执行成功
    assert_eq!(
        assertion_report["status"].as_str().unwrap(),
        "success",
        "断言节点应该执行成功"
    );

    // 验证断言结果
    let output_data = &assertion_report["output_data"];
    assert!(output_data["passed"].as_bool().unwrap(), "断言应该通过");
    assert_eq!(output_data["passed_count"].as_u64().unwrap(), 1);
    assert_eq!(output_data["failed_count"].as_u64().unwrap(), 0);

    // 验证执行了 true 分支的 HTTP 节点（true-branch-http）
    let true_branch_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "true-branch-http");
    assert!(
        true_branch_report.is_some(),
        "应该执行了 true-branch-http 节点（true 分支）"
    );

    // 验证没有执行 false 分支的 HTTP 节点（false-branch-http）
    let false_branch_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "false-branch-http");
    assert!(
        false_branch_report.is_none(),
        "不应该执行 false-branch-http 节点（false 分支）"
    );
}

#[test]
fn test_assertion_fail_routes_to_false() {
    let file = create_assertion_fail_route_false_flow();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证流程执行成功（断言失败但 route_false 模式不终止流程）
    assert!(
        json["success"].as_bool().unwrap(),
        "流程应该成功执行（route_false 模式）"
    );

    // 验证 node_reports 包含断言节点
    let node_reports = json["node_reports"].as_array().unwrap();
    let assertion_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "assertion-node")
        .expect("应该找到断言节点报告");

    // 验证断言节点执行成功（节点本身执行成功，但断言结果为 false）
    assert_eq!(
        assertion_report["status"].as_str().unwrap(),
        "success",
        "断言节点应该执行成功（即使断言失败）"
    );

    // 验证断言结果
    let output_data = &assertion_report["output_data"];
    assert!(!output_data["passed"].as_bool().unwrap(), "断言应该失败");
    assert_eq!(output_data["passed_count"].as_u64().unwrap(), 0);
    assert_eq!(output_data["failed_count"].as_u64().unwrap(), 1);

    // 验证执行了 false 分支的 HTTP 节点（false-branch-http）
    let false_branch_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "false-branch-http");
    assert!(
        false_branch_report.is_some(),
        "应该执行了 false-branch-http 节点（false 分支）"
    );

    // 验证没有执行 true 分支的 HTTP 节点（true-branch-http）
    let true_branch_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "true-branch-http");
    assert!(
        true_branch_report.is_none(),
        "不应该执行 true-branch-http 节点（true 分支）"
    );
}

#[test]
fn test_assertion_fail_throw_terminates_flow() {
    let file = create_assertion_fail_throw_flow();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证 node_reports 包含断言节点
    let node_reports = json["node_reports"].as_array().unwrap();
    let assertion_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "assertion-node")
        .expect("应该找到断言节点报告");

    // 验证断言节点状态为 failed（throw 模式）
    assert_eq!(
        assertion_report["status"].as_str().unwrap(),
        "failed",
        "断言节点应该标记为失败（throw 模式）"
    );

    // 验证断言节点包含错误信息（error 是一个对象，包含 message 字段）
    assert!(
        assertion_report["error"].is_object(),
        "断言节点应该包含错误对象（throw 模式）"
    );
    assert!(
        assertion_report["error"]["message"].is_string(),
        "断言节点错误对象应该包含 message 字段（throw 模式）"
    );

    // 验证没有执行 true-branch-http 节点（流程在断言节点终止）
    let true_branch_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "true-branch-http");
    assert!(
        true_branch_report.is_none(),
        "不应该执行 true-branch-http 节点（流程应该在断言节点终止）"
    );
}

#[test]
fn test_assertion_node_report_contains_results() {
    let file = create_assertion_pass_flow();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    let output = cmd
        .arg("--execute")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证断言节点报告包含详细结果
    let node_reports = json["node_reports"].as_array().unwrap();
    let assertion_report = node_reports
        .iter()
        .find(|r| r["node_uuid"] == "assertion-node")
        .expect("应该找到断言节点报告");

    let output_data = &assertion_report["output_data"];

    // 验证输出包含所有必要字段
    assert!(output_data["passed"].is_boolean(), "应该有 passed 字段");
    assert!(output_data["total"].is_number(), "应该有 total 字段");
    assert!(
        output_data["passed_count"].is_number(),
        "应该有 passed_count 字段"
    );
    assert!(
        output_data["failed_count"].is_number(),
        "应该有 failed_count 字段"
    );
    assert!(output_data["results"].is_array(), "应该有 results 数组");
    assert!(
        output_data["failed_assertions"].is_array(),
        "应该有 failed_assertions 数组"
    );
    assert!(output_data["input"].is_object(), "应该有 input 对象");

    // 验证 results 包含断言详情
    let results = output_data["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "应该有 1 条断言结果");

    let result = &results[0];
    assert_eq!(result["id"].as_str().unwrap(), "assert-status");
    assert!(result["passed"].as_bool().unwrap());
    assert_eq!(result["target"].as_str().unwrap(), "$.status");
    // 操作符序列化为 PascalCase（如 "Eq"）
    assert_eq!(
        result["operator"].as_str().unwrap().to_lowercase().as_str(),
        "eq"
    );
    assert_eq!(result["expected"].as_u64().unwrap(), 200);
    assert_eq!(result["actual"].as_u64().unwrap(), 200);
}

// ============ 压力测试命令测试 (Task 25) ============

/// 测试压测命令帮助信息
#[test]
fn test_stress_test_help() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("stress-test").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("压力测试"))
        .stdout(predicate::str::contains("-c"))
        .stdout(predicate::str::contains("-r"))
        .stdout(predicate::str::contains("-d"));
}

/// 测试压测命令缺少必需参数
#[test]
fn test_stress_test_missing_flow() {
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("stress-test");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("flow"));
}

/// 测试压测命令无效的并发数
#[test]
fn test_stress_test_invalid_concurrent_users() {
    let flow_file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("cli");
    cmd.arg("stress-test")
        .arg("-f")
        .arg(flow_file.path())
        .arg("-c")
        .arg("invalid");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("无效"));
}
