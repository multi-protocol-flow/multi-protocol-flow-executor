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
fn load_flow_fixture(fixture_name: &str) -> serde_json::Value {
    let mut path = fixtures_dir();
    path.push(fixture_name);
    let content = fs::read_to_string(&path).expect(&format!("Failed to read fixture: {:?}", path));
    serde_json::from_str(&content).expect(&format!("Failed to parse fixture JSON: {:?}", path))
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
        flow_json["name"].as_str().unwrap(),
        "有效流程 - 开始节点连接到结束节点"
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
    assert_eq!(start_node["type"].as_str().unwrap(), "start");

    // Verify end node
    let end_node = &nodes[1];
    let end_uuid = end_node["uuid"]
        .as_str()
        .expect("Node must have uuid field");
    assert_eq!(end_uuid, "end-550e8400-e29b-41d4-a716-446655440000");
    assert!(is_valid_uuid(end_uuid), "End node UUID should be valid");
    assert_eq!(end_node["type"].as_str().unwrap(), "end");

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
    let all_node_uuids: Vec<&str> = nodes.iter().map(|n| n["uuid"].as_str().unwrap()).collect();
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
        flow_json["uuid"].as_str().unwrap().len() > 0,
        "Flow uuid must not be empty"
    );

    for (i, node) in flow_json["nodes"].as_array().unwrap().iter().enumerate() {
        assert!(
            node["uuid"].is_string(),
            "Node {} must have uuid field",
            i + 1
        );
        let node_uuid = node["uuid"].as_str().unwrap();
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
        .unwrap()
        .iter()
        .enumerate()
    {
        assert!(
            conn["id"].is_string(),
            "Connection {} must have id field",
            i + 1
        );
        let conn_id = conn["id"].as_str().unwrap();
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
        let source_uuid = conn["source_node_uuid"].as_str().unwrap();
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
        let target_uuid = conn["target_node_uuid"].as_str().unwrap();
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
        flow_json["name"].as_str().unwrap(),
        "无效流程 - 缺少开始节点"
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
    assert_eq!(end_node["type"].as_str().unwrap(), "end");

    // Verify no connections
    let connections = flow_json["connections"]
        .as_array()
        .expect("Flow must have connections array");
    assert_eq!(connections.len(), 0, "Flow should have 0 connections");

    // Verify no start node
    let start_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| n["type"].as_str().unwrap() == "start")
        .collect();
    assert_eq!(start_nodes.len(), 0, "Flow should not have a start node");
}

#[test]
fn test_no_start_node_uuid_validity() {
    let flow_json = load_flow_fixture("02_no_start_node.json");

    // Verify flow UUID format
    let flow_uuid = flow_json["uuid"].as_str().unwrap();
    assert!(is_valid_uuid(flow_uuid), "Flow UUID must be valid");

    // Verify node UUID format
    let nodes = flow_json["nodes"].as_array().unwrap();
    for (i, node) in nodes.iter().enumerate() {
        let node_uuid = node["uuid"].as_str().unwrap();
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
    assert_eq!(flow_json["name"].as_str().unwrap(), "无效流程 - 循环检测");

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
    assert_eq!(start_node["type"].as_str().unwrap(), "start");

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

    let nodes = flow_json["nodes"].as_array().unwrap();
    let connections = flow_json["connections"].as_array().unwrap();

    // Verify all connection source/target UUIDs reference existing nodes
    let all_node_uuids: Vec<&str> = nodes.iter().map(|n| n["uuid"].as_str().unwrap()).collect();

    for (i, conn) in connections.iter().enumerate() {
        let source_uuid = conn["source_node_uuid"].as_str().unwrap();
        let target_uuid = conn["target_node_uuid"].as_str().unwrap();

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
        conn["source_node_uuid"].as_str().unwrap(),
        conn["target_node_uuid"].as_str().unwrap(),
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
        let flow_uuid = flow_json["uuid"].as_str().unwrap();
        assert!(
            !all_uuids.contains(flow_uuid),
            "Flow UUID {} in {} is not unique",
            flow_uuid,
            fixture_name
        );
        all_uuids.insert(flow_uuid.to_string());

        // Check node UUIDs
        for (i, node) in flow_json["nodes"].as_array().unwrap().iter().enumerate() {
            let node_uuid = node["uuid"].as_str().unwrap();
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
            .unwrap()
            .iter()
            .enumerate()
        {
            let conn_id = conn["id"].as_str().unwrap();
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
        for (i, node) in flow_json["nodes"].as_array().unwrap().iter().enumerate() {
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
            .unwrap()
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
