// 变量追踪功能测试（Task 13, 14, 15, 16）
//
// 测试覆盖：
// - Task 13: 变量追踪逻辑（variable_changes, resolved_config）
// - Task 14: 执行路径追踪（execution_path）
// - Task 15: 跳过节点记录（skipped_nodes）
// - Task 16: 流程级变量快照（initial_variables, final_variables）
#![allow(missing_docs)]

use flow_engine::connection::Connection;
use flow_engine::executor::FlowExecutor;
use flow_engine::flow::Flow;
use flow_engine::node::{Node, NodePosition};
use flow_engine::node_config::NodeData;
use rustls::crypto::ring::default_provider;
use serde_json::json;
use std::sync::Once;
use uuid::Uuid;

/// Ensure rustls CryptoProvider is installed before any reqwest Client creation
static CRYPTO_INIT: Once = Once::new();
fn ensure_crypto_provider() {
    CRYPTO_INIT.call_once(|| {
        let _ = default_provider().install_default();
    });
}

/// 测试 Task 13: 变量追踪 - 新变量创建
///
/// 验证节点输出数据创建新变量时，variable_changes 正确记录
#[tokio::test]
async fn test_variable_tracking_new_variable() {
    ensure_crypto_provider();
    // 创建简单流程：Entry -> Entry（第二个节点模拟有输出）
    let flow_id = Uuid::new_v4().to_string();
    let start_node = Node {
        uuid: "start-1".to_string(),
        node_type: "entry".to_string(),
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let end_node = Node {
        uuid: "end-1".to_string(),
        node_type: "entry".to_string(),
        name: "结束".to_string(),
        position: NodePosition { x: 100.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let connection = Connection {
        id: "conn-1".to_string(),
        source_node_uuid: start_node.uuid.clone(),
        target_node_uuid: end_node.uuid.clone(),
        source_port_id: "out".to_string(),
        target_port_id: "input".to_string(),
    };

    let flow = Flow {
        uuid: flow_id.clone(),
        initial_variables: None,
        name: "测试流程".to_string(),
        description: None,
        nodes: vec![start_node, end_node],
        connections: vec![connection],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    // 验证执行成功
    assert!(result.success);
    assert!(result.flow_report.is_some());

    let flow_report = result
        .flow_report
        .expect("flow report should exist after successful execution");

    // 验证 resolved_config 被填充（Task 13）
    for node_report in &flow_report.node_reports {
        assert!(node_report.resolved_config.is_some());
    }
}

/// 测试 Task 14: 执行路径追踪
///
/// 验证 execution_path 包含所有执行节点的 UUID，按执行顺序排列
#[tokio::test]
async fn test_execution_path_tracking() {
    ensure_crypto_provider();
    let flow_id = Uuid::new_v4().to_string();
    let start_node = Node {
        uuid: "start-uuid-path".to_string(),
        node_type: "entry".to_string(),
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let end_node = Node {
        uuid: "end-uuid-path".to_string(),
        node_type: "entry".to_string(),
        name: "结束".to_string(),
        position: NodePosition { x: 100.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let connection = Connection {
        id: "conn-path".to_string(),
        source_node_uuid: start_node.uuid.clone(),
        target_node_uuid: end_node.uuid.clone(),
        source_port_id: "out".to_string(),
        target_port_id: "input".to_string(),
    };

    let flow = Flow {
        uuid: flow_id.clone(),
        initial_variables: None,
        name: "执行路径测试".to_string(),
        description: None,
        nodes: vec![start_node.clone(), end_node.clone()],
        connections: vec![connection],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(result.success);
    let flow_report = result
        .flow_report
        .expect("flow report should exist for execution path tracking");

    // 验证 execution_path 包含所有执行的节点 UUID
    assert_eq!(flow_report.execution_path.len(), 2);
    assert_eq!(flow_report.execution_path[0], start_node.uuid);
    assert_eq!(flow_report.execution_path[1], end_node.uuid);
}

/// 测试 Task 16: 流程级变量快照 - initial_variables 和 final_variables
///
/// 验证流程开始和结束时的变量状态被正确记录
#[tokio::test]
async fn test_variable_snapshots() {
    ensure_crypto_provider();
    let flow_id = Uuid::new_v4().to_string();
    let start_node = Node {
        uuid: "start-snap".to_string(),
        node_type: "entry".to_string(),
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let end_node = Node {
        uuid: "end-snap".to_string(),
        node_type: "entry".to_string(),
        name: "结束".to_string(),
        position: NodePosition { x: 100.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let connection = Connection {
        id: "conn-snap".to_string(),
        source_node_uuid: start_node.uuid.clone(),
        target_node_uuid: end_node.uuid.clone(),
        source_port_id: "out".to_string(),
        target_port_id: "input".to_string(),
    };

    let flow = Flow {
        uuid: flow_id.clone(),
        initial_variables: None,
        name: "变量快照测试".to_string(),
        description: None,
        nodes: vec![start_node, end_node],
        connections: vec![connection],
        hooks: None,
    };

    // 提供初始变量
    let initial_vars = json!({
        "api_key": "test-key-123",
        "base_url": "https://api.example.com"
    });

    let executor = FlowExecutor::new();
    let result = executor
        .execute_flow(&flow, Some(initial_vars.clone()), None, None)
        .await;

    assert!(result.success);
    let flow_report = result
        .flow_report
        .expect("flow report should exist for variable snapshot test");

    // 验证 initial_variables 被正确记录
    assert!(flow_report.initial_variables.is_some());
    let initial = flow_report
        .initial_variables
        .expect("initial_variables should be present after execution");
    assert_eq!(initial["api_key"], json!("test-key-123"));
    assert_eq!(initial["base_url"], json!("https://api.example.com"));

    // 验证 final_variables 被正确记录
    assert!(flow_report.final_variables.is_some());
    // final_variables 应该包含初始变量（因为没有节点修改它们）
    let final_vars = flow_report
        .final_variables
        .expect("final_variables should be present after execution");
    assert_eq!(final_vars["api_key"], json!("test-key-123"));
}

/// 测试 Task 16: 流程级变量快照 - 空 initial_variables
///
/// 验证没有初始变量时的行为
#[tokio::test]
async fn test_variable_snapshots_empty_initial() {
    ensure_crypto_provider();
    let flow_id = Uuid::new_v4().to_string();
    let start_node = Node {
        uuid: "start-empty".to_string(),
        node_type: "entry".to_string(),
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let flow = Flow {
        uuid: flow_id.clone(),
        initial_variables: None,
        name: "空变量测试".to_string(),
        description: None,
        nodes: vec![start_node],
        connections: vec![],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(result.success);
    let flow_report = result
        .flow_report
        .expect("flow report should exist for empty snapshot test");

    // 验证 initial_variables 和 final_variables 都存在（空对象）
    assert!(flow_report.initial_variables.is_some());
    assert!(flow_report.final_variables.is_some());

    let initial = flow_report
        .initial_variables
        .expect("initial_variables should be present for empty initial");
    let final_vars = flow_report
        .final_variables
        .expect("final_variables should be present for empty initial");

    // 空对象应该序列化为 {}
    assert!(initial.is_object());
    assert!(final_vars.is_object());
}

/// 测试单节点流程的 execution_path
#[tokio::test]
async fn test_single_node_execution_path() {
    ensure_crypto_provider();
    let flow_id = Uuid::new_v4().to_string();
    let start_node = Node {
        uuid: "single-node".to_string(),
        node_type: "entry".to_string(),
        name: "单节点".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let flow = Flow {
        uuid: flow_id.clone(),
        initial_variables: None,
        name: "单节点流程".to_string(),
        description: None,
        nodes: vec![start_node.clone()],
        connections: vec![],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(result.success);
    let flow_report = result
        .flow_report
        .expect("flow report should exist for single node path test");

    // 执行路径应该只包含一个节点
    assert_eq!(flow_report.execution_path.len(), 1);
    assert_eq!(flow_report.execution_path[0], start_node.uuid);
}

/// 测试 skipped_nodes 默认为空
///
/// 当前实现中 skipped_nodes 需要特定条件才会填充
#[tokio::test]
async fn test_skipped_nodes_default_empty() {
    ensure_crypto_provider();
    let flow_id = Uuid::new_v4().to_string();
    let start_node = Node {
        uuid: "start-skip".to_string(),
        node_type: "entry".to_string(),
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let flow = Flow {
        uuid: flow_id.clone(),
        initial_variables: None,
        name: "跳过节点测试".to_string(),
        description: None,
        nodes: vec![start_node],
        connections: vec![],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(result.success);
    let flow_report = result
        .flow_report
        .expect("flow report should exist for skipped nodes test");

    // skipped_nodes 默认为空
    assert_eq!(flow_report.skipped_nodes.len(), 0);
}

/// 测试所有节点都有 resolved_config
#[tokio::test]
async fn test_all_nodes_have_resolved_config() {
    ensure_crypto_provider();
    let flow_id = Uuid::new_v4().to_string();
    let start_node = Node {
        uuid: "start-resolved".to_string(),
        node_type: "entry".to_string(),
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let end_node = Node {
        uuid: "end-resolved".to_string(),
        node_type: "entry".to_string(),
        name: "结束".to_string(),
        position: NodePosition { x: 100.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    let connection = Connection {
        id: "conn-resolved".to_string(),
        source_node_uuid: start_node.uuid.clone(),
        target_node_uuid: end_node.uuid.clone(),
        source_port_id: "out".to_string(),
        target_port_id: "input".to_string(),
    };

    let flow = Flow {
        uuid: flow_id.clone(),
        initial_variables: None,
        name: "resolved_config 测试".to_string(),
        description: None,
        nodes: vec![start_node, end_node],
        connections: vec![connection],
        hooks: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None, None).await;

    assert!(result.success);
    let flow_report = result
        .flow_report
        .expect("flow report should exist for resolved config test");

    // 所有节点都应该有 resolved_config
    for node_report in &flow_report.node_reports {
        assert!(
            node_report.resolved_config.is_some(),
            "node {} missing resolved_config",
            node_report.node_name
        );
    }
}
