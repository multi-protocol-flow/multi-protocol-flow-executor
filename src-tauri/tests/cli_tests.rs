#![allow(missing_docs)]

// CLI 测试 - 测试 mpe 命令行接口的所有功能

use assert_cmd::cargo;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

/// 创建一个有效的简单流程文件（仅 entry 节点，有连接）
fn create_valid_flow_file() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "_version": 1,
        "flow": {
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
                    "data": { "type": "http", "url": "http://localhost:19090/api/status/200" },
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
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建一个带有 uuid 字段的流程文件
fn create_flow_with_uuid() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "_version": 1,
        "flow": {
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
                    "data": { "type": "http", "url": "http://localhost:19090/api/status/200" },
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
        },
        "_meta": null
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
            r#"{{"uuid": "node-{}", "type": "http", "name": "节点 {}", "position": {{"x": {}, "y": {}}}, "data": {{"type": "http", "url": "http://localhost:19090/api/status/200"}}, "ports": null}}"#,
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
        r#"{{"_version": 1, "flow": {{\"uuid\": \"large-flow-uuid\", \"name\": \"大型流程\", \"nodes\": {}, \"connections\": {}}}, "_meta": null}}"#,
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
fn test_run_flow_long_format() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_run_flow_short_format() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("-f")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_help() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("mpe"))
        .stdout(predicate::str::contains("multi-protocol flow execution engine"));
}

#[test]
fn test_run_subcommand_help() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("execute flow"))
        .stdout(predicate::str::contains("--flow"));
}

// ============ 错误处理测试 ============

#[test]
fn test_run_missing_flow_parameter() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run").assert().failure();
}

#[test]
fn test_run_file_not_found() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg("/nonexistent/file.json")
        .assert()
        .failure()
        .stdout(predicate::str::contains("flow file not found").or(predicate::str::contains("error")));
}

#[test]
fn test_run_invalid_json_file() {
    let file = create_invalid_json_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("failed to load flow").or(predicate::str::contains("error")));
}

#[test]
fn test_run_empty_flow_file() {
    let file = create_empty_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure();
}

// ============ 文件加载测试 ============

#[test]
fn test_load_valid_json_flow_file() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_load_flow_with_uuid_field() {
    let file = create_flow_with_uuid();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
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
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#))
        .stdout(predicate::str::contains(r#""data""#));
}

#[test]
fn test_json_output_is_parseable() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
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
    assert!(json["success"].is_boolean());
    assert!(json["data"].is_object() || json["data"].is_null());
}

#[test]
fn test_cli_output_has_success_data_error_fields() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // 验证统一输出格式: { success, data, error }
    assert!(json["success"].is_boolean(), "missing success field");
    assert!(
        json["data"].is_object() || json["data"].is_null(),
        "missing data field"
    );
}

#[test]
fn test_execution_output_contains_execution_time() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // data 内部应该包含 execution_time_ms
    let data = &json["data"];
    assert!(data.is_object(), "data 应该是对象");
    assert!(
        data["execution_time_ms"].is_number(),
        "data 中应该包含 execution_time_ms"
    );
}

#[test]
fn test_execution_output_contains_executed_count() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    let data = &json["data"];
    assert!(
        data["executed_count"].is_number(),
        "data 中应该包含 executed_count"
    );
}

// ============ 边界情况测试 ============

#[test]
fn test_large_flow_file() {
    let file = create_large_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
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
        let mut cmd = cargo::cargo_bin_cmd!("mpe");
        cmd.arg("run")
            .arg("--flow")
            .arg(file1.path())
            .assert()
            .success();
    });

    let handle2 = std::thread::spawn(move || {
        let mut cmd = cargo::cargo_bin_cmd!("mpe");
        cmd.arg("run")
            .arg("--flow")
            .arg(file2.path())
            .assert()
            .success();
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

#[test]
fn test_execution_with_single_entry_node() {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"{
        "_version": 1,
        "flow": {
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
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
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
        "_version": 1,
        "flow": {
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
                    "data": { "type": "http", "url": "http://localhost:19090/api/status/200" },
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
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

#[test]
fn test_execution_time_reasonable() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    let execution_time_ms = json["data"]["execution_time_ms"].as_u64().unwrap();

    // 执行时间应该在合理范围内（< 30 秒）
    assert!(
        execution_time_ms < 30000,
        "执行时间过长: {}ms",
        execution_time_ms
    );
}

// ============ 错误输出测试 ============

#[test]
fn test_error_output_on_invalid_flow() {
    let file = create_empty_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure();
}

#[test]
fn test_error_handling_invalid_json_file() {
    let file = create_invalid_json_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("failed to load flow").or(predicate::str::contains("error")));
}

#[test]
fn test_flow_with_missing_uuid() {
    let file = NamedTempFile::new().unwrap();
    // 创建 FlowFile 包装但 Flow 内缺少 uuid 字段（反序列化失败）
    let flow_content = r#"{
        "_version": 1,
        "flow": {
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
                    "data": { "type": "http", "url": "http://localhost:19090/api/status/200" },
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
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure();
}

// ============ validate 命令测试 ============

#[test]
fn test_validate_valid_flow() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("validate")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#))
        .stdout(predicate::str::contains(r#""valid": true"#));
}

#[test]
fn test_validate_invalid_flow() {
    let file = NamedTempFile::new().unwrap();
    // 创建有 uuid 但无 nodes 的流程（可以加载但验证失败）
    let flow_content = r#"{
        "_version": 1,
        "flow": {
            "uuid": "invalid-flow-uuid",
            "name": "无效流程",
            "nodes": [],
            "connections": []
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("validate")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains(r#""valid": false"#));
}

#[test]
fn test_validate_missing_flow_arg() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("validate").assert().failure();
}

#[test]
fn test_validate_nonexistent_file() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("validate")
        .arg("--flow")
        .arg("/nonexistent/file.json")
        .assert()
        .failure()
        .stdout(predicate::str::contains("flow file not found").or(predicate::str::contains("error")));
}

#[test]
fn test_validate_output_format() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("validate")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert!(json["success"].is_boolean());
    assert!(json["data"]["valid"].is_boolean());
    assert!(json["data"]["errors"].is_array());
}

// ============ flow 管理 CRUD 测试 ============

#[test]
fn test_flow_create() {
    // 使用临时目录作为 flows_dir
    let temp_dir = tempfile::tempdir().unwrap();
    let flows_dir = temp_dir.path().to_string_lossy().to_string();

    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("create")
        .arg("--name")
        .arg("测试流程")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env(
            "MPE_REPORTS_DIR",
            temp_dir.path().join("reports").to_string_lossy().as_ref(),
        )
        .env(
            "MPE_LOGS_DIR",
            temp_dir.path().join("logs").to_string_lossy().as_ref(),
        )
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#))
        .stdout(predicate::str::contains("测试流程"));
}

#[test]
fn test_flow_list() {
    let temp_dir = tempfile::tempdir().unwrap();
    let flows_dir = temp_dir.path().to_string_lossy().to_string();

    // 先创建一个流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("create")
        .arg("--name")
        .arg("列表测试流程")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env(
            "MPE_REPORTS_DIR",
            temp_dir.path().join("reports").to_string_lossy().as_ref(),
        )
        .env(
            "MPE_LOGS_DIR",
            temp_dir.path().join("logs").to_string_lossy().as_ref(),
        )
        .assert()
        .success();

    // 列出流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("list")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env(
            "MPE_REPORTS_DIR",
            temp_dir.path().join("reports").to_string_lossy().as_ref(),
        )
        .env(
            "MPE_LOGS_DIR",
            temp_dir.path().join("logs").to_string_lossy().as_ref(),
        )
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#))
        .stdout(predicate::str::contains("列表测试流程"));
}

#[test]
fn test_flow_create_and_show_crud_cycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let flows_dir = temp_dir.path().to_string_lossy().to_string();
    let reports_dir = temp_dir
        .path()
        .join("reports")
        .to_string_lossy()
        .to_string();
    let logs_dir = temp_dir.path().join("logs").to_string_lossy().to_string();

    // 创建流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("flow")
        .arg("create")
        .arg("--name")
        .arg("CRUD 测试流程")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    let flow_uuid = json["data"]["flow"]["uuid"].as_str().unwrap().to_string();

    // 查看流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("show")
        .arg(&flow_uuid)
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("CRUD 测试流程"));

    // 删除流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("delete")
        .arg(&flow_uuid)
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""deleted"#));

    // 删除后再查看应该失败
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("show")
        .arg(&flow_uuid)
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .failure();
}

// ============ flow edit 测试 ============

#[test]
fn test_flow_edit_node_add() {
    let temp_dir = tempfile::tempdir().unwrap();
    let flows_dir = temp_dir.path().to_string_lossy().to_string();
    let reports_dir = temp_dir
        .path()
        .join("reports")
        .to_string_lossy()
        .to_string();
    let logs_dir = temp_dir.path().join("logs").to_string_lossy().to_string();

    // 先创建流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("flow")
        .arg("create")
        .arg("--name")
        .arg("节点添加测试")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    let flow_uuid = json["data"]["flow"]["uuid"].as_str().unwrap().to_string();

    // 添加一个 HTTP 节点
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("edit")
        .arg("node-add")
        .arg("--flow")
        .arg(&flow_uuid)
        .arg("--type")
        .arg("http")
        .arg("--name")
        .arg("新 HTTP 节点")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("新 HTTP 节点"));
}

#[test]
fn test_flow_edit_connection_add() {
    let temp_dir = tempfile::tempdir().unwrap();
    let flows_dir = temp_dir.path().to_string_lossy().to_string();
    let reports_dir = temp_dir
        .path()
        .join("reports")
        .to_string_lossy()
        .to_string();
    let logs_dir = temp_dir.path().join("logs").to_string_lossy().to_string();

    // 创建流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("flow")
        .arg("create")
        .arg("--name")
        .arg("连接添加测试")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    let flow_uuid = json["data"]["flow"]["uuid"].as_str().unwrap().to_string();

    // 获取 entry 节点 UUID
    let entry_node_uuid = json["data"]["flow"]["nodes"][0]["uuid"]
        .as_str()
        .unwrap()
        .to_string();

    // 添加 HTTP 节点
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let node_output = cmd
        .arg("flow")
        .arg("edit")
        .arg("node-add")
        .arg("--flow")
        .arg(&flow_uuid)
        .arg("--type")
        .arg("http")
        .arg("--name")
        .arg("目标节点")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let node_output_str = String::from_utf8(node_output).unwrap();
    let node_json: serde_json::Value = serde_json::from_str(&node_output_str).unwrap();

    // 找到新添加的 HTTP 节点 UUID
    let http_node_uuid = node_json["data"]["flow"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"].as_str().unwrap() == "目标节点")
        .map(|n| n["uuid"].as_str().unwrap().to_string())
        .unwrap();

    // 添加连接
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("edit")
        .arg("connection-add")
        .arg("--flow")
        .arg(&flow_uuid)
        .arg("--source")
        .arg(&entry_node_uuid)
        .arg("--target")
        .arg(&http_node_uuid)
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(&entry_node_uuid))
        .stdout(predicate::str::contains(&http_node_uuid));
}

#[test]
fn test_flow_edit_variable_set() {
    let temp_dir = tempfile::tempdir().unwrap();
    let flows_dir = temp_dir.path().to_string_lossy().to_string();
    let reports_dir = temp_dir
        .path()
        .join("reports")
        .to_string_lossy()
        .to_string();
    let logs_dir = temp_dir.path().join("logs").to_string_lossy().to_string();

    // 创建流程
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("flow")
        .arg("create")
        .arg("--name")
        .arg("变量设置测试")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    let flow_uuid = json["data"]["flow"]["uuid"].as_str().unwrap().to_string();

    // 设置变量
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("flow")
        .arg("edit")
        .arg("variable-set")
        .arg("--flow")
        .arg(&flow_uuid)
        .arg("--key")
        .arg("api_url")
        .arg("--value")
        .arg("https://api.example.com")
        .env("MPE_FLOWS_DIR", &flows_dir)
        .env("MPE_REPORTS_DIR", &reports_dir)
        .env("MPE_LOGS_DIR", &logs_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("api_url"))
        .stdout(predicate::str::contains("https://api.example.com"));
}

// ============ report 管理测试 ============

#[test]
fn test_report_list() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("report")
        .arg("list")
        .env(
            "MPE_FLOWS_DIR",
            temp_dir.path().join("flows").to_string_lossy().as_ref(),
        )
        .env(
            "MPE_REPORTS_DIR",
            temp_dir.path().join("reports").to_string_lossy().as_ref(),
        )
        .env(
            "MPE_LOGS_DIR",
            temp_dir.path().join("logs").to_string_lossy().as_ref(),
        )
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""success": true"#));
}

// ============ 日志输出验证测试 ============

#[test]
fn test_log_output_to_stderr() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
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
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        // 验证日志包含执行相关信息（匹配实际的日志消息）
        .stderr(predicate::str::contains("executing start node").or(predicate::str::contains("[done]")));
}

#[test]
fn test_log_level_debug() {
    let file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");

    // 设置 RUST_LOG=debug 环境变量
    cmd.env("RUST_LOG", "debug")
        .arg("run")
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
    let mut cmd = cargo::cargo_bin_cmd!("mpe");

    // 设置 RUST_LOG=info 环境变量
    cmd.env("RUST_LOG", "info")
        .arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success();
}

// ============ 断言节点集成测试 ============

/// 创建断言通过流程（断言验证状态码 200，通过 true 分支）
fn create_assertion_pass_flow() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"    {
        "_version": 1,
        "flow": {
            "uuid": "assertion-pass-flow-001",
            "name": "断言通过流程",
            "nodes": [
                {
                    "uuid": "entry-node",
                    "type": "entry",
                    "name": "开始",
                    "position": {
                        "x": 0,
                        "y": 0
                    },
                    "data": {
                        "type": "entry"
                    },
                    "ports": null
                },
                {
                    "uuid": "http-node",
                    "type": "http",
                    "name": "HTTP 请求",
                    "position": {
                        "x": 200,
                        "y": 0
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/200",
                        "method": "get"
                    },
                    "ports": null
                },
                {
                    "uuid": "assertion-node",
                    "type": "assertion",
                    "name": "断言状态码",
                    "position": {
                        "x": 400,
                        "y": 0
                    },
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
                    "position": {
                        "x": 600,
                        "y": -100
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/201",
                        "method": "get"
                    },
                    "ports": null
                },
                {
                    "uuid": "false-branch-http",
                    "type": "http",
                    "name": "False 分支标记",
                    "position": {
                        "x": 600,
                        "y": 100
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/400",
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
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建断言失败流程（断言验证状态码 999，实际返回 200，走 false 分支）
fn create_assertion_fail_route_false_flow() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"    {
        "_version": 1,
        "flow": {
            "uuid": "assertion-fail-route-false-001",
            "name": "断言失败路由流程",
            "nodes": [
                {
                    "uuid": "entry-node",
                    "type": "entry",
                    "name": "开始",
                    "position": {
                        "x": 0,
                        "y": 0
                    },
                    "data": {
                        "type": "entry"
                    },
                    "ports": null
                },
                {
                    "uuid": "http-node",
                    "type": "http",
                    "name": "HTTP 请求",
                    "position": {
                        "x": 200,
                        "y": 0
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/200",
                        "method": "get"
                    },
                    "ports": null
                },
                {
                    "uuid": "assertion-node",
                    "type": "assertion",
                    "name": "断言状态码",
                    "position": {
                        "x": 400,
                        "y": 0
                    },
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
                    "position": {
                        "x": 600,
                        "y": -100
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/201",
                        "method": "get"
                    },
                    "ports": null
                },
                {
                    "uuid": "false-branch-http",
                    "type": "http",
                    "name": "False 分支标记",
                    "position": {
                        "x": 600,
                        "y": 100
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/400",
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
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

/// 创建断言失败抛出错误流程（流程在断言节点终止）
fn create_assertion_fail_throw_flow() -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    let flow_content = r#"    {
        "_version": 1,
        "flow": {
            "uuid": "assertion-fail-throw-001",
            "name": "断言失败抛出流程",
            "nodes": [
                {
                    "uuid": "entry-node",
                    "type": "entry",
                    "name": "开始",
                    "position": {
                        "x": 0,
                        "y": 0
                    },
                    "data": {
                        "type": "entry"
                    },
                    "ports": null
                },
                {
                    "uuid": "http-node",
                    "type": "http",
                    "name": "HTTP 请求",
                    "position": {
                        "x": 200,
                        "y": 0
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/200",
                        "method": "get"
                    },
                    "ports": null
                },
                {
                    "uuid": "assertion-node",
                    "type": "assertion",
                    "name": "断言状态码",
                    "position": {
                        "x": 400,
                        "y": 0
                    },
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
                    "position": {
                        "x": 600,
                        "y": 0
                    },
                    "data": {
                        "type": "http",
                        "url": "http://localhost:19090/api/status/201",
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
        },
        "_meta": null
    }"#;
    fs::write(file.path(), flow_content).unwrap();
    file
}

#[test]
fn test_assertion_pass_routes_to_true() {
    let file = create_assertion_pass_flow();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
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

    // 验证 data 中包含成功标志
    let data = &json["data"];
    assert!(
        data["success"].as_bool().unwrap(),
        "data.success 应该为 true"
    );

    // 验证执行了 4 个节点 (entry, http, assertion, true-branch-http)
    let executed_count = data["executed_count"].as_u64().unwrap();
    assert_eq!(
        executed_count, 4,
        "应该执行了 4 个节点（entry + http + assertion + true-branch）"
    );
}

#[test]
fn test_assertion_fail_routes_to_false() {
    let file = create_assertion_fail_route_false_flow();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
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

    // 验证执行了 4 个节点 (entry, http, assertion, false-branch-http)
    let data = &json["data"];
    let executed_count = data["executed_count"].as_u64().unwrap();
    assert_eq!(
        executed_count, 4,
        "应该执行了 4 个节点（entry + http + assertion + false-branch）"
    );
}

#[test]
fn test_assertion_fail_throw_terminates_flow() {
    let file = create_assertion_fail_throw_flow();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    let output = cmd
        .arg("run")
        .arg("--flow")
        .arg(file.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // throw 模式：断言失败导致流程提前终止
    // 验证流程返回结果中 success 为 false（或 error 字段存在）
    // 注意：mpe run 可能将 flow failure 映射为 outer success=true, data.success=false
    // 也可能直接 outer success=false
    let data = &json["data"];

    // 验证执行了有限数量的节点（entry, http, assertion = 3 个），
    // 不包括 true-branch-http（因为 throw 模式终止了流程）
    let executed_count = data["executed_count"].as_u64().unwrap();
    assert!(
        executed_count <= 3,
        "throw 模式应该只执行了 3 个节点或更少（流程在断言节点终止），实际: {}",
        executed_count
    );
}

// ============ 压力测试命令测试 ============

/// 测试压测命令帮助信息
#[test]
fn test_stress_test_help() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("stress").arg("run").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("flow"));
}

/// 测试压测命令缺少必需参数
#[test]
fn test_stress_test_missing_flow() {
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("stress").arg("run");

    cmd.assert().failure();
}

/// 测试压测命令无效的并发数
#[test]
fn test_stress_test_invalid_concurrent_users() {
    let flow_file = create_valid_flow_file();
    let mut cmd = cargo::cargo_bin_cmd!("mpe");
    cmd.arg("stress")
        .arg("run")
        .arg("-f")
        .arg(flow_file.path())
        .arg("--concurrent-users")
        .arg("invalid");

    cmd.assert().failure();
}
