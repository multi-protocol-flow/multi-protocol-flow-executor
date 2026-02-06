// CLI 测试 - 测试命令行接口的所有功能

use assert_cmd::cargo;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

/// 创建一个有效的简单流程文件（start + end 节点，有连接）
fn create_valid_flow_file() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "test-flow-uuid",
        "name": "测试流程",
        "nodes": [
            {
                "uuid": "start-node",
                "type": "start",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "start"},
                "ports": null
            },
            {
                "uuid": "end-node",
                "type": "end",
                "name": "结束节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": {"type": "start"},
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "start-node",
                "target_node_uuid": "end-node",
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
                "uuid": "start-node",
                "type": "start",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "start"},
                "ports": null
            },
            {
                "uuid": "end-node",
                "type": "end",
                "name": "结束节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": {"type": "start"},
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "start-node",
                "target_node_uuid": "end-node",
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

    // Add start node
    nodes.push_str(r#"{"uuid": "start-node", "type": "start", "name": "开始节点", "position": {"x": 0.0, "y": 0.0}, "data": {"type": "start"}, "ports": null}"#);

    let mut prev_uuid = "start-node".to_string();

    // Add intermediate nodes
    for i in 1..=48 {
        nodes.push_str(",");
        nodes.push_str(&format!(
            r#"{{"uuid": "node-{}", "type": "end", "name": "节点 {}", "position": {{"x": {}, "y": {}}}, "data": {{"type": "end"}}, "ports": null}}"#,
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

    // Add final end node
    nodes.push_str(",");
    nodes.push_str(r#"{"uuid": "end-node", "type": "end", "name": "结束节点", "position": {"x": 500.0, "y": 500.0}, "data": {"type": "end"}, "ports": null}"#);

    // Add connection from last intermediate to end
    connections.push_str(",");
    connections.push_str(r#"{"id": "conn-final", "source_node_uuid": "node-48", "target_node_uuid": "end-node", "source_port_id": "output", "target_port_id": "input"}"#);

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
    assert!(json["execution_time"].as_u64().unwrap() >= 0);
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
fn test_execution_with_single_start_node() {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "single-node-flow-uuid",
        "name": "单节点流程",
        "nodes": [
            {
                "uuid": "start-node",
                "type": "start",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "start"},
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
        .failure()
        .stdout(predicate::str::contains(r#""success": false"#))
        .stdout(predicate::str::contains(r#""error""#));
}

#[test]
fn test_flow_with_connections() {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "uuid": "flow-with-connections-uuid",
        "name": "带连接的流程",
        "nodes": [
            {
                "uuid": "start-node",
                "type": "start",
                "name": "开始节点",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {"type": "start"},
                "ports": null
            },
            {
                "uuid": "http-node",
                "type": "http",
                "name": "HTTP 节点",
                "position": { "x": 100.0, "y": 0.0 },
                "data": { "type": "http", "url": "https://example.com" },
                "ports": null
            },
            {
                "uuid": "end-node",
                "type": "end",
                "name": "结束节点",
                "position": { "x": 200.0, "y": 0.0 },
                "data": {"type": "start"},
                "ports": null
            }
        ],
        "connections": [
            {
                "id": "conn-1",
                "source_node_uuid": "start-node",
                "target_node_uuid": "http-node",
                "source_port_id": "output",
                "target_port_id": "input"
            },
            {
                "id": "conn-2",
                "source_node_uuid": "http-node",
                "target_node_uuid": "end-node",
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
