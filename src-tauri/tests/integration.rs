#![allow(missing_docs)]

// Integration Tests for Multi-Protocol Flow Executor
// End-to-end testing with real flow fixtures

use std::fs;
use std::path::PathBuf;

// ============================================================================
// Test Setup Helper Functions
// ============================================================================

/// Get the path to the fixtures directory
fn fixtures_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR points to src-tauri, so we need to go up one level
    // to reach the project root where tests/fixtures/flows/ is located
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go up from src-tauri to project root
    path.push("tests");
    path.push("fixtures");
    path.push("flows");
    path
}

/// Load a flow from a fixture file
///
/// Fixture 文件使用 FlowFile 包装格式，此函数自动解包返回内部的 flow 数据。
fn load_flow_fixture(fixture_name: &str) -> serde_json::Value {
    let mut path = fixtures_dir();
    path.push(fixture_name);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read fixture: {:?}", path));
    let flow_file: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse fixture JSON: {:?}", path));
    // 解包 FlowFile 包装格式，返回内部的 flow 数据
    flow_file.get("flow").cloned().unwrap_or_else(|| {
        panic!(
            "Fixture must be in FlowFile format (with 'flow' key): {:?}",
            path
        )
    })
}

/// Verify UUID format (basic check)
/// Accepts standard UUIDs and prefixed UUIDs (e.g., "start-550e8400-e29b-41d4-a716-446655440000")
fn is_valid_uuid(uuid: &str) -> bool {
    // Simple check: not empty, contains some hex chars, reasonable length
    !uuid.is_empty() && uuid.len() >= 10 && uuid.chars().any(|c| c.is_ascii_hexdigit())
}

// ============================================================================
// Scenario 1: Valid Flow Execution
// ============================================================================

#[test]
fn test_valid_flow_execution() {
    let flow_json = load_flow_fixture("01_valid_start_with_connection.json");

    // Verify flow UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "550e8400-e29b-41d4-a716-446655440000");
    assert!(is_valid_uuid(flow_uuid), "Flow UUID should be valid");

    // Verify flow name
    assert_eq!(
        flow_json["name"].as_str().expect("flow name should be a string"),
        "Valid Flow - Start Node Connected to End Node"
    );

    // Verify nodes
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 2, "Flow should have 2 nodes");

    // Verify start node
    let start_node = &nodes[0];
    let start_uuid = start_node["uuid"]
        .as_str()
        .expect("Node must have uuid field");
    assert_eq!(start_uuid, "start-550e8400-e29b-41d4-a716-446655440000");
    assert!(is_valid_uuid(start_uuid), "Start node UUID should be valid");
    assert_eq!(start_node["type"].as_str().expect("start node type should be a string"), "entry");

    // Verify end node
    let end_node = &nodes[1];
    let end_uuid = end_node["uuid"]
        .as_str()
        .expect("Node must have uuid field");
    assert_eq!(end_uuid, "end-550e8400-e29b-41d4-a716-446655440000");
    assert!(is_valid_uuid(end_uuid), "End node UUID should be valid");
    assert_eq!(end_node["type"].as_str().expect("end node type should be a string"), "end");

    // Verify connections
    let connections = flow_json["connections"]
        .as_array()
        .expect("Flow must have connections array");
    assert_eq!(connections.len(), 1, "Flow should have 1 connection");

    let conn = &connections[0];
    let conn_id = conn["id"].as_str().expect("Connection must have id field");
    assert_eq!(conn_id, "conn-550e8400-e29b-41d4-a716-446655440000");
    assert!(is_valid_uuid(conn_id), "Connection ID should be valid");

    let source_uuid = conn["source_node_uuid"]
        .as_str()
        .expect("Connection must have source_node_uuid");
    assert_eq!(
        source_uuid, start_uuid,
        "Connection source should match start node UUID"
    );

    let target_uuid = conn["target_node_uuid"]
        .as_str()
        .expect("Connection must have target_node_uuid");
    assert_eq!(
        target_uuid, end_uuid,
        "Connection target should match end node UUID"
    );

    // Verify connection references valid nodes
    let all_node_uuids: Vec<&str> = nodes.iter().map(|n| n["uuid"].as_str().expect("node uuid should be a string")).collect();
    assert!(
        all_node_uuids.contains(&source_uuid),
        "Connection source UUID must reference an existing node"
    );
    assert!(
        all_node_uuids.contains(&target_uuid),
        "Connection target UUID must reference an existing node"
    );
}

#[test]
fn test_valid_flow_all_uuid_fields_present() {
    let flow_json = load_flow_fixture("01_valid_start_with_connection.json");

    // Verify all UUID fields exist
    assert!(flow_json["uuid"].is_string(), "Flow must have uuid field");
    assert!(
        !flow_json["uuid"].as_str().expect("flow uuid should be a string").is_empty(),
        "Flow uuid must not be empty"
    );

    for (i, node) in flow_json["nodes"].as_array().expect("value should be an array").iter().enumerate() {
        assert!(
            node["uuid"].is_string(),
            "Node {} must have uuid field",
            i + 1
        );
        let node_uuid = node["uuid"].as_str().expect("node uuid should be a string");
        assert!(
            !node_uuid.is_empty(),
            "Node {} uuid must not be empty",
            i + 1
        );
        assert!(
            is_valid_uuid(node_uuid),
            "Node {} UUID should be valid",
            i + 1
        );
    }

    for (i, conn) in flow_json["connections"]
        .as_array()
        .expect("connections should be an array")
        .iter()
        .enumerate()
    {
        assert!(
            conn["id"].is_string(),
            "Connection {} must have id field",
            i + 1
        );
        let conn_id = conn["id"].as_str().expect("connection id should be a string");
        assert!(
            !conn_id.is_empty(),
            "Connection {} id must not be empty",
            i + 1
        );

        assert!(
            conn["source_node_uuid"].is_string(),
            "Connection {} must have source_node_uuid",
            i + 1
        );
        let source_uuid = conn["source_node_uuid"].as_str().expect("source_node_uuid should be a string");
        assert!(
            !source_uuid.is_empty(),
            "Connection {} source_node_uuid must not be empty",
            i + 1
        );

        assert!(
            conn["target_node_uuid"].is_string(),
            "Connection {} must have target_node_uuid",
            i + 1
        );
        let target_uuid = conn["target_node_uuid"].as_str().expect("target_node_uuid should be a string");
        assert!(
            !target_uuid.is_empty(),
            "Connection {} target_node_uuid must not be empty",
            i + 1
        );
    }
}

// ============================================================================
// Scenario 2: No Start Node Error Handling
// ============================================================================

#[test]
fn test_no_start_node_error_handling() {
    let flow_json = load_flow_fixture("02_no_start_node.json");

    // Verify flow UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "6ba7b810-9dad-11d1-80b4-00c04fd430c8");
    assert!(is_valid_uuid(flow_uuid), "Flow UUID should be valid");

    // Verify flow name
    assert_eq!(
        flow_json["name"].as_str().expect("flow name should be a string"),
        "Invalid Flow - Missing Start Node"
    );

    // Verify nodes
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 1, "Flow should have 1 node");

    // Verify end node exists
    let end_node = &nodes[0];
    let end_uuid = end_node["uuid"]
        .as_str()
        .expect("Node must have uuid field");
    assert_eq!(end_uuid, "end-6ba7b810-9dad-11d1-80b4-00c04fd430c8");
    assert!(is_valid_uuid(end_uuid), "End node UUID should be valid");
    assert_eq!(end_node["type"].as_str().expect("end node type should be a string"), "end");

    // Verify no connections
    let connections = flow_json["connections"]
        .as_array()
        .expect("Flow must have connections array");
    assert_eq!(connections.len(), 0, "Flow should have 0 connections");

    // Verify no entry node
    let entry_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| n["type"].as_str().expect("node type should be a string") == "entry")
        .collect();
    assert_eq!(entry_nodes.len(), 0, "Flow should not have an entry node");
}

#[test]
fn test_no_start_node_uuid_validity() {
    let flow_json = load_flow_fixture("02_no_start_node.json");

    // Verify flow UUID format
    let flow_uuid = flow_json["uuid"].as_str().expect("flow uuid should be a string");
    assert!(is_valid_uuid(flow_uuid), "Flow UUID must be valid");

    // Verify node UUID format
    let nodes = flow_json["nodes"].as_array().expect("value should be an array");
    for (i, node) in nodes.iter().enumerate() {
        let node_uuid = node["uuid"].as_str().expect("node uuid should be a string");
        assert!(
            is_valid_uuid(node_uuid),
            "Node {} UUID must be valid",
            i + 1
        );
    }
}

// ============================================================================
// Scenario 3: Loop Detection
// ============================================================================

#[test]
fn test_loop_detection() {
    let flow_json = load_flow_fixture("04_loop_detection.json");

    // Verify flow UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "6ba7b812-9dad-11d1-80b4-00c04fd430c8");
    assert!(is_valid_uuid(flow_uuid), "Flow UUID should be valid");

    // Verify flow name
    assert_eq!(
        flow_json["name"].as_str().expect("flow name should be a string"),
        "Invalid Flow - Loop Detection"
    );

    // Verify nodes
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 1, "Flow should have 1 node");

    // Verify start node
    let start_node = &nodes[0];
    let start_uuid = start_node["uuid"]
        .as_str()
        .expect("Node must have uuid field");
    assert_eq!(start_uuid, "start-6ba7b812-9dad-11d1-80b4-00c04fd430c8");
    assert!(is_valid_uuid(start_uuid), "Start node UUID should be valid");
    assert_eq!(start_node["type"].as_str().expect("start node type should be a string"), "entry");

    // Verify connections
    let connections = flow_json["connections"]
        .as_array()
        .expect("Flow must have connections array");
    assert_eq!(connections.len(), 1, "Flow should have 1 connection");

    let conn = &connections[0];
    let conn_id = conn["id"].as_str().expect("Connection must have id field");
    assert_eq!(conn_id, "conn-6ba7b812-9dad-11d1-80b4-00c04fd430c8");
    assert!(is_valid_uuid(conn_id), "Connection ID should be valid");

    let source_uuid = conn["source_node_uuid"]
        .as_str()
        .expect("Connection must have source_node_uuid");
    let target_uuid = conn["target_node_uuid"]
        .as_str()
        .expect("Connection must have target_node_uuid");

    // Verify self-loop (connection points to same node)
    assert_eq!(source_uuid, start_uuid, "Source should be the start node");
    assert_eq!(
        target_uuid, start_uuid,
        "Target should be the start node (self-loop)"
    );
}

#[test]
fn test_loop_detection_uuid_consistency() {
    let flow_json = load_flow_fixture("04_loop_detection.json");

    let nodes = flow_json["nodes"].as_array().expect("value should be an array");
    let connections = flow_json["connections"].as_array().expect("value should be an array");

    // Verify all connection source/target UUIDs reference existing nodes
    let all_node_uuids: Vec<&str> = nodes.iter().map(|n| n["uuid"].as_str().expect("node uuid should be a string")).collect();

    for (i, conn) in connections.iter().enumerate() {
        let source_uuid = conn["source_node_uuid"].as_str().expect("source_node_uuid should be a string");
        let target_uuid = conn["target_node_uuid"].as_str().expect("target_node_uuid should be a string");

        assert!(
            all_node_uuids.contains(&source_uuid),
            "Connection {} source UUID must reference an existing node",
            i + 1
        );
        assert!(
            all_node_uuids.contains(&target_uuid),
            "Connection {} target UUID must reference an existing node",
            i + 1
        );
    }

    // In the loop detection case, the same node is used for both source and target
    let conn = &connections[0];
    assert_eq!(
        conn["source_node_uuid"].as_str().expect("source_node_uuid should be a string"),
        conn["target_node_uuid"].as_str().expect("target_node_uuid should be a string"),
        "Loop connection should have same source and target"
    );
}

// ============================================================================
// Additional UUID Validation Tests
// ============================================================================

#[test]
fn test_all_fixture_uuids_are_unique() {
    let fixture_files = vec![
        "01_valid_start_with_connection.json",
        "02_no_start_node.json",
        "04_loop_detection.json",
    ];

    let mut all_uuids = std::collections::HashSet::new();

    for fixture_name in fixture_files {
        let flow_json = load_flow_fixture(fixture_name);

        // Check flow UUID
        let flow_uuid = flow_json["uuid"].as_str().expect("flow uuid should be a string");
        assert!(
            !all_uuids.contains(flow_uuid),
            "Flow UUID {} in {} is not unique",
            flow_uuid,
            fixture_name
        );
        all_uuids.insert(flow_uuid.to_string());

        // Check node UUIDs
        for (i, node) in flow_json["nodes"].as_array().expect("value should be an array").iter().enumerate() {
            let node_uuid = node["uuid"].as_str().expect("node uuid should be a string");
            assert!(
                !all_uuids.contains(node_uuid),
                "Node UUID {} in {} (node {}) is not unique",
                node_uuid,
                fixture_name,
                i + 1
            );
            all_uuids.insert(node_uuid.to_string());
        }

        // Check connection UUIDs
        for (i, conn) in flow_json["connections"]
            .as_array()
            .expect("connections should be an array")
            .iter()
            .enumerate()
        {
            let conn_id = conn["id"].as_str().expect("connection id should be a string");
            assert!(
                !all_uuids.contains(conn_id),
                "Connection UUID {} in {} (connection {}) is not unique",
                conn_id,
                fixture_name,
                i + 1
            );
            all_uuids.insert(conn_id.to_string());
        }
    }
}

#[test]
fn test_fixture_files_exist() {
    let fixtures = fixtures_dir();

    let required_files = vec![
        "01_valid_start_with_connection.json",
        "02_no_start_node.json",
        "04_loop_detection.json",
    ];

    for file_name in required_files {
        let file_path = fixtures.join(file_name);
        assert!(
            file_path.exists(),
            "Fixture file {:?} does not exist",
            file_path
        );
    }
}

#[test]
fn test_all_uuids_are_strings() {
    let fixture_files = vec![
        "01_valid_start_with_connection.json",
        "02_no_start_node.json",
        "04_loop_detection.json",
    ];

    for fixture_name in fixture_files {
        let flow_json = load_flow_fixture(fixture_name);

        // Verify flow.uuid is a string
        assert!(
            flow_json["uuid"].is_string(),
            "Flow uuid in {} must be a string",
            fixture_name
        );

        // Verify node.uuid fields are strings
        for (i, node) in flow_json["nodes"].as_array().expect("value should be an array").iter().enumerate() {
            assert!(
                node["uuid"].is_string(),
                "Node {} uuid in {} must be a string",
                i + 1,
                fixture_name
            );
        }

        // Verify connection.id fields are strings
        for (i, conn) in flow_json["connections"]
            .as_array()
            .expect("connections should be an array")
            .iter()
            .enumerate()
        {
            assert!(
                conn["id"].is_string(),
                "Connection {} id in {} must be a string",
                i + 1,
                fixture_name
            );

            // Verify connection.source_node_uuid is a string
            assert!(
                conn["source_node_uuid"].is_string(),
                "Connection {} source_node_uuid in {} must be a string",
                i + 1,
                fixture_name
            );

            // Verify connection.target_node_uuid is a string
            assert!(
                conn["target_node_uuid"].is_string(),
                "Connection {} target_node_uuid in {} must be a string",
                i + 1,
                fixture_name
            );
        }
    }
}

// ============================================================================
// Scenario 4: WebSocket Integration Tests
// ============================================================================

/// 测试 WebSocket 基本连接流程结构
/// 验证: Entry → WS Connect → WS Close
#[test]
fn test_ws_basic_connect_flow_structure() {
    let flow_json = load_flow_fixture("ws_basic_connect.json");

    // 验证流程 UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "ws-basic-connect-001");

    // 验证流程名称
    assert!(flow_json["name"]
        .as_str()
        .expect("flow name should be a string")
        .contains("WebSocket Basic Connection Test"));

    // 验证节点数量和类型
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 3, "Flow should have 3 nodes");

    // 验证节点类型
    let node_types: Vec<&str> = nodes.iter().map(|n| n["type"].as_str().expect("node type should be a string")).collect();
    assert!(node_types.contains(&"entry"));
    assert!(node_types.contains(&"ws_connect"));
    assert!(node_types.contains(&"ws_close"));

    // 验证连接数量
    let connections = flow_json["connections"]
        .as_array()
        .expect("Flow must have connections");
    assert_eq!(connections.len(), 2, "Flow should have 2 connections");

    // 验证端口语义：WS Connect 的输出应该连接到 "true" 端口
    let ws_connect_to_close_conn = connections
        .iter()
        .find(|c| c["source_port_id"].as_str() == Some("true"))
        .expect("WS Connect should have output on 'true' port");
    assert_eq!(
        ws_connect_to_close_conn["source_port_id"].as_str().expect("source_port_id should be a string"),
        "true"
    );
}

/// 测试 WebSocket 发送等待流程结构
/// 验证: Entry → WS Connect → WS Send+Wait → WS Close
#[test]
fn test_ws_send_wait_flow_structure() {
    let flow_json = load_flow_fixture("ws_send_wait.json");

    // 验证流程 UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "ws-send-wait-001");

    // 验证节点数量和类型
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 4, "Flow should have 4 nodes");

    // 验证节点类型
    let node_types: Vec<&str> = nodes.iter().map(|n| n["type"].as_str().expect("node type should be a string")).collect();
    assert!(node_types.contains(&"entry"));
    assert!(node_types.contains(&"ws_connect"));
    assert!(node_types.contains(&"ws_send_collect"));
    assert!(node_types.contains(&"ws_close"));

    // 验证 WS Send+Collect 节点配置
    let ws_send_collect_node = nodes
        .iter()
        .find(|n| n["type"] == "ws_send_collect")
        .expect("Should have ws_send_collect node");
    let node_data = &ws_send_collect_node["data"];
    assert!(node_data["collect_response"].as_bool().expect("collect_response should be a boolean"));
    assert_eq!(node_data["collect_mode"].as_str().expect("collect_mode should be a string"), "first_match");
}

/// 测试 WebSocket 连接采集超时 + Conditional 判断流程结构
/// 验证: Entry → WS Connect (collect timeout) → Conditional → End
#[test]
fn test_ws_connect_wait_timeout_flow_structure() {
    let flow_json = load_flow_fixture("ws_connect_wait_timeout.json");

    // 验证流程 UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "ws-connect-wait-timeout-001");

    // 验证节点数量和类型
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 5, "Flow should have 5 nodes");

    // 验证节点类型
    let node_types: Vec<&str> = nodes.iter().map(|n| n["type"].as_str().expect("node type should be a string")).collect();
    assert!(node_types.contains(&"entry"));
    assert!(node_types.contains(&"ws_connect"));
    assert!(node_types.contains(&"conditional"));
    assert_eq!(node_types.iter().filter(|&&t| t == "end").count(), 2);

    // 验证 WS Connect 节点配置了采集消息
    let ws_connect_node = nodes
        .iter()
        .find(|n| n["type"] == "ws_connect")
        .expect("Should have ws_connect node");
    let node_data = &ws_connect_node["data"];
    assert!(node_data["collect_message"].as_bool().expect("collect_message should be a boolean"));
    assert_eq!(node_data["collect_mode"].as_str().expect("collect_mode should be a string"), "first_match");
    assert!(node_data["collect_timeout_ms"].as_u64().expect("collect_timeout_ms should be an integer") > 0);

    // 验证 Conditional 节点配置
    let conditional_node = nodes
        .iter()
        .find(|n| n["type"] == "conditional")
        .expect("Should have conditional node");
    let cond_data = &conditional_node["data"];
    assert_eq!(cond_data["condition_type"].as_str().expect("condition_type should be a string"), "simple");
    assert_eq!(
        cond_data["field_path"].as_str().expect("field_path should be a string"),
        "{{collect_timed_out}}"
    );
    assert_eq!(cond_data["operator"].as_str().expect("operator should be a string"), "equal");
    assert!(cond_data["expected_value"].as_bool().expect("expected_value should be a boolean"));
}

/// 测试 WebSocket 发送采集超时 + Conditional 判断流程结构
/// 验证: Entry → WS Connect → WS Send+Collect (timeout) → Conditional → End
#[test]
fn test_ws_send_wait_timeout_conditional_flow_structure() {
    let flow_json = load_flow_fixture("ws_send_wait_timeout_conditional.json");

    // 验证流程 UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "ws-send-wait-timeout-conditional-001");

    // 验证节点数量和类型
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 6, "Flow should have 6 nodes");

    // 验证节点类型
    let node_types: Vec<&str> = nodes.iter().map(|n| n["type"].as_str().expect("node type should be a string")).collect();
    assert!(node_types.contains(&"entry"));
    assert!(node_types.contains(&"ws_connect"));
    assert!(node_types.contains(&"ws_send_collect"));
    assert!(node_types.contains(&"conditional"));
    assert_eq!(node_types.iter().filter(|&&t| t == "end").count(), 2);

    // 验证 WS Send+Collect 节点配置了采集响应和超时
    let ws_send_collect_node = nodes
        .iter()
        .find(|n| n["type"] == "ws_send_collect")
        .expect("Should have ws_send_collect node");
    let node_data = &ws_send_collect_node["data"];
    assert!(node_data["collect_response"].as_bool().expect("collect_response should be a boolean"));
    assert!(node_data["collect_timeout_ms"].as_u64().expect("collect_timeout_ms should be an integer") > 0);

    // 验证 Conditional 节点配置
    let conditional_node = nodes
        .iter()
        .find(|n| n["type"] == "conditional")
        .expect("Should have conditional node");
    let cond_data = &conditional_node["data"];
    assert_eq!(
        cond_data["field_path"].as_str().expect("field_path should be a string"),
        "{{collect_timed_out}}"
    );
    assert_eq!(cond_data["operator"].as_str().expect("operator should be a string"), "equal");
}

/// 测试 WebSocket 完整流程（采集响应）结构
/// 验证: Entry → WS Connect (collect first msg) → WS Send+Collect (collect response) → WS Close
#[test]
fn test_ws_full_flow_wait_response_structure() {
    let flow_json = load_flow_fixture("ws_full_flow_wait_response.json");

    // 验证流程 UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "ws-full-flow-wait-response-001");

    // 验证节点数量和类型
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 4, "Flow should have 4 nodes");

    // 验证节点类型
    let node_types: Vec<&str> = nodes.iter().map(|n| n["type"].as_str().expect("node type should be a string")).collect();
    assert!(node_types.contains(&"entry"));
    assert!(node_types.contains(&"ws_connect"));
    assert!(node_types.contains(&"ws_send_collect"));
    assert!(node_types.contains(&"ws_close"));

    // 验证 WS Connect 节点配置了采集首条消息
    let ws_connect_node = nodes
        .iter()
        .find(|n| n["type"] == "ws_connect")
        .expect("Should have ws_connect node");
    let connect_data = &ws_connect_node["data"];
    assert!(connect_data["collect_message"].as_bool().expect("collect_message should be a boolean"));
    assert_eq!(
        connect_data["collect_mode"].as_str().expect("collect_mode should be a string"),
        "first_match"
    );

    // 验证 WS Send+Collect 节点配置了采集响应
    let ws_send_collect_node = nodes
        .iter()
        .find(|n| n["type"] == "ws_send_collect")
        .expect("Should have ws_send_collect node");
    let send_collect_data = &ws_send_collect_node["data"];
    assert!(send_collect_data["collect_response"].as_bool().expect("collect_response should be a boolean"));
    assert_eq!(
        send_collect_data["collect_mode"].as_str().expect("collect_mode should be a string"),
        "first_match"
    );

    // 验证连接顺序
    let connections = flow_json["connections"]
        .as_array()
        .expect("Flow must have connections");
    assert_eq!(connections.len(), 3, "Flow should have 3 connections");

    // 验证连接端口：entry 使用 output，WebSocket 操作成功使用 true
    // Entry → WS Connect
    let conn1 = &connections[0];
    assert_eq!(
        conn1["source_node_uuid"].as_str().expect("source_node_uuid should be a string"),
        "entry-full-wait"
    );
    assert_eq!(conn1["source_port_id"].as_str().expect("source_port_id should be a string"), "output");

    // WS Connect → WS Send+Collect
    let conn2 = &connections[1];
    assert_eq!(
        conn2["source_node_uuid"].as_str().expect("source_node_uuid should be a string"),
        "ws-connect-full-wait"
    );
    assert_eq!(conn2["source_port_id"].as_str().expect("source_port_id should be a string"), "true");

    // WS Send+Collect → WS Close
    let conn3 = &connections[2];
    assert_eq!(
        conn3["source_node_uuid"].as_str().expect("source_node_uuid should be a string"),
        "ws-send-wait-full-wait"
    );
    assert_eq!(conn3["source_port_id"].as_str().expect("source_port_id should be a string"), "true");
}

/// 测试 WebSocket 条件匹配流程结构
#[test]
fn test_ws_conditional_match_flow_structure() {
    let flow_json = load_flow_fixture("ws_conditional_match.json");

    // 验证流程 UUID
    let flow_uuid = flow_json["uuid"]
        .as_str()
        .expect("Flow must have uuid field");
    assert_eq!(flow_uuid, "ws-conditional-match-001");

    // 验证节点数量
    let nodes = flow_json["nodes"]
        .as_array()
        .expect("Flow must have nodes array");
    assert_eq!(nodes.len(), 4, "Flow should have 4 nodes");

    // 验证 WS Send+Collect 节点配置了条件采集
    let ws_send_collect_node = nodes
        .iter()
        .find(|n| n["type"] == "ws_send_collect")
        .expect("Should have ws_send_collect node");
    let node_data = &ws_send_collect_node["data"];
    assert_eq!(
        node_data["collect_mode"].as_str().expect("collect_mode should be a string"),
        "condition_match"
    );
    assert!(node_data["collect_condition"].is_object());

    // 验证 collect_condition 配置
    let collect_condition = &node_data["collect_condition"];
    assert_eq!(collect_condition["match_type"].as_str().expect("match_type should be a string"), "regex");
    assert!(!collect_condition["expression"].as_str().expect("expression should be a string").is_empty());
}

/// 验证 WebSocket 节点输出数据结构（collect_timed_out 字段）
#[test]
fn test_ws_output_data_structure() {
    // 验证 ws_connect 输出结构
    let connect_flow = load_flow_fixture("ws_basic_connect.json");
    let connect_node = connect_flow["nodes"]
        .as_array()
        .expect("nodes should be an array")
        .iter()
        .find(|n| n["type"] == "ws_connect")
        .expect("Should have ws_connect node");

    // 验证必要的配置字段存在
    let data = &connect_node["data"];
    assert!(data["url"].is_string(), "ws_connect should have url");
    assert!(
        data["connect_timeout_ms"].is_u64(),
        "ws_connect should have connect_timeout_ms"
    );
    assert!(
        data["collect_message"].is_boolean(),
        "ws_connect should have collect_message"
    );

    // 验证 ws_send_collect 输出结构
    let send_collect_flow = load_flow_fixture("ws_send_wait.json");
    let send_collect_node = send_collect_flow["nodes"]
        .as_array()
        .expect("nodes should be an array")
        .iter()
        .find(|n| n["type"] == "ws_send_collect")
        .expect("Should have ws_send_collect node");

    // 验证必要的配置字段存在
    let data = &send_collect_node["data"];
    assert!(
        data["connection_id"].is_string(),
        "ws_send_collect should have connection_id"
    );
    assert!(
        data["message"].is_string(),
        "ws_send_collect should have message"
    );
    assert!(
        data["collect_response"].is_boolean(),
        "ws_send_collect should have collect_response"
    );
    assert!(
        data["collect_timeout_ms"].is_u64(),
        "ws_send_collect should have collect_timeout_ms"
    );
}

/// 验证所有 WebSocket fixture 文件存在
#[test]
fn test_ws_fixture_files_exist() {
    let fixtures = fixtures_dir();

    let required_files = vec![
        "ws_basic_connect.json",
        "ws_send_wait.json",
        "ws_conditional_match.json",
        "ws_connect_wait_timeout.json",
        "ws_send_wait_timeout_conditional.json",
        "ws_full_flow_wait_response.json",
    ];

    for file_name in required_files {
        let file_path = fixtures.join(file_name);
        assert!(
            file_path.exists(),
            "WebSocket fixture file {:?} does not exist",
            file_path
        );
    }
}
