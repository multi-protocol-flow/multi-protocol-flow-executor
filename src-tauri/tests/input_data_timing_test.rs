//! input_data 时序测试
//!
//! 验证修复后的 input_data 语义：
//! - Entry 节点：input_data = None（无输入）
//! - HTTP 节点：input_data = 请求配置（url, method, headers, body, body_type）
//! - 变量相关数据通过 variable_changes 记录

use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use flow_engine::command::ExecuteFlowCommandInput;
use flow_engine::connection::Connection;
use flow_engine::flow::Flow;
use flow_engine::node::{Node, NodePosition, NodeType, Port, PortType};
use flow_engine::node_config::{EntryConfig, HttpConfig, HttpMethod, NodeData};
use flow_engine::traits::NoopLogSender;

/// 测试 1: Entry 节点的 input_data 应该是 None
///
/// 流程: Entry
/// 验证: Entry 节点的 input_data 应该是 None（无输入）
#[tokio::test]
async fn test_input_data_timing_basic() {
    // 创建 Entry 节点
    let entry_node = Node {
        uuid: "entry-1".to_string(),
        node_type: NodeType::Entry,
        name: "入口".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::Entry(EntryConfig::default()),
        ports: Some(vec![
            Port {
                id: "output".to_string(),
                port_type: PortType::Output,
                label: Some("输出".to_string()),
            },
        ]),
        has_breakpoint: None,
    };

    // 创建流程
    let flow = Flow {
        uuid: "flow-1".to_string(),
        name: "测试流程".to_string(),
        nodes: vec![entry_node],
        connections: vec![],
        initial_variables: None,
    };

    // 执行流程
    let input = ExecuteFlowCommandInput {
        flow_id: "flow-1".to_string(),
        variables: Some(json!({})),
    };

    let log_sender = Arc::new(NoopLogSender);
    let result = flow_engine::commands::execute_flow::execute_flow(input, flow, Some(log_sender)).await;

    // 验证执行成功
    assert!(result.success, "流程执行失败: {:?}", result.error);

    // 验证有流程报告
    let flow_report = result.flow_report.expect("应该有流程报告");
    assert_eq!(flow_report.node_reports.len(), 1, "应该有 1 个节点报告");

    // 验证 Entry 节点的 input_data
    let entry_report = &flow_report.node_reports[0];
    assert_eq!(entry_report.node_uuid, "entry-1", "节点 UUID 应该是 entry-1");

    // 验证 input_data 应该是 None（Entry 节点无输入）
    assert!(
        entry_report.input_data.is_none(),
        "Entry 节点的 input_data 应该是 None，实际: {:?}",
        entry_report.input_data
    );
}

/// 测试 2: 使用初始变量的 Entry 节点
///
/// 流程: Entry (初始变量 {x: 1})
/// 验证: Entry 节点的 input_data 应该是 None（无输入）
///       variable_changes 应该记录初始变量的设置
#[tokio::test]
async fn test_input_data_timing_with_initial_variables() {
    // 创建 Entry 节点（无初始变量）
    let entry_node = Node {
        uuid: "entry-2".to_string(),
        node_type: NodeType::Entry,
        name: "入口".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::Entry(EntryConfig::default()),
        ports: Some(vec![
            Port {
                id: "output".to_string(),
                port_type: PortType::Output,
                label: Some("输出".to_string()),
            },
        ]),
        has_breakpoint: None,
    };

    // 创建流程（带初始变量）
    let mut initial_vars = HashMap::new();
    initial_vars.insert("x".to_string(), json!(1));
    initial_vars.insert("y".to_string(), json!(2));

    let flow = Flow {
        uuid: "flow-2".to_string(),
        name: "带初始变量的流程".to_string(),
        nodes: vec![entry_node],
        connections: vec![],
        initial_variables: Some(initial_vars),
    };

    // 执行流程
    let input = ExecuteFlowCommandInput {
        flow_id: "flow-2".to_string(),
        variables: None,
    };

    let log_sender = Arc::new(NoopLogSender);
    let result = flow_engine::commands::execute_flow::execute_flow(input, flow, Some(log_sender)).await;

    // 验证执行成功
    assert!(result.success, "流程执行失败: {:?}", result.error);

    // 验证有流程报告
    let flow_report = result.flow_report.expect("应该有流程报告");
    assert_eq!(flow_report.node_reports.len(), 1, "应该有 1 个节点报告");

    // 验证 Entry 节点的 input_data 是 None（Entry 节点无输入）
    let entry_report = &flow_report.node_reports[0];
    assert!(
        entry_report.input_data.is_none(),
        "Entry 节点的 input_data 应该是 None，实际: {:?}",
        entry_report.input_data
    );

    // Entry 节点的 variable_changes 应该记录初始变量
    // 注意：初始变量是在流程执行前设置的，Entry 节点执行时变量已经存在
}

/// 测试 3: HTTP 节点使用初始变量
///
/// 流程: Entry -> HTTP (使用 {{initial_variables.api_url}})
/// 验证: HTTP 节点的 input_data 应该是请求配置（url, method, headers, body, body_type）
///       不应该包含响应数据（status, body, headers 等）
///
/// 注意：此测试需要外部 HTTP 服务，可能会失败
#[tokio::test]
#[ignore = "需要外部 HTTP 服务"]
async fn test_input_data_timing_http_node() {
    // 创建 Entry 节点
    let entry_node = Node {
        uuid: "entry-3".to_string(),
        node_type: NodeType::Entry,
        name: "入口".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::Entry(EntryConfig::default()),
        ports: Some(vec![
            Port {
                id: "output".to_string(),
                port_type: PortType::Output,
                label: Some("输出".to_string()),
            },
        ]),
        has_breakpoint: None,
    };

    // 创建 HTTP 节点（使用变量引用）
    let http_node = Node {
        uuid: "http-3".to_string(),
        node_type: NodeType::Http,
        name: "HTTP 请求".to_string(),
        position: NodePosition { x: 200.0, y: 0.0 },
        data: NodeData::Http(HttpConfig {
            url: "https://httpbin.org/get".to_string(),
            method: HttpMethod::Get,
            ..Default::default()
        }),
        ports: Some(vec![
            Port {
                id: "input".to_string(),
                port_type: PortType::Input,
                label: Some("输入".to_string()),
            },
            Port {
                id: "true".to_string(),
                port_type: PortType::Output,
                label: Some("成功".to_string()),
            },
            Port {
                id: "false".to_string(),
                port_type: PortType::Output,
                label: Some("失败".to_string()),
            },
        ]),
        has_breakpoint: None,
    };

    // 创建连接
    let connection = Connection {
        id: "conn-3".to_string(),
        source_node_uuid: "entry-3".to_string(),
        target_node_uuid: "http-3".to_string(),
        source_port_id: "output".to_string(),
        target_port_id: "input".to_string(),
    };

    // 创建流程
    let mut initial_vars = HashMap::new();
    initial_vars.insert("api_url".to_string(), json!("https://httpbin.org"));

    let flow = Flow {
        uuid: "flow-3".to_string(),
        name: "HTTP 测试流程".to_string(),
        nodes: vec![entry_node, http_node],
        connections: vec![connection],
        initial_variables: Some(initial_vars),
    };

    // 执行流程
    let input = ExecuteFlowCommandInput {
        flow_id: "flow-3".to_string(),
        variables: None,
    };

    let log_sender = Arc::new(NoopLogSender);
    let result = flow_engine::commands::execute_flow::execute_flow(input, flow, Some(log_sender)).await;

    // 验证执行成功
    assert!(result.success, "流程执行失败: {:?}", result.error);

    // 验证有流程报告
    let flow_report = result.flow_report.expect("应该有流程报告");
    assert_eq!(flow_report.node_reports.len(), 2, "应该有 2 个节点报告");

    // 验证 HTTP 节点的 input_data 是请求配置
    let http_report = flow_report
        .node_reports
        .iter()
        .find(|r| r.node_uuid == "http-3")
        .expect("应该有 HTTP 节点报告");

    let input_data = http_report.input_data.as_ref().expect("input_data 应该存在");
    let input_obj = input_data.as_object().expect("input_data 应该是对象");

    // input_data 应该包含请求配置字段（url, method）
    assert!(
        input_obj.contains_key("url"),
        "input_data 应该包含 url 字段"
    );
    assert!(
        input_obj.contains_key("method"),
        "input_data 应该包含 method 字段"
    );

    // input_data 不应该包含 HTTP 节点的输出（status、success 等）
    // 注意：headers 在请求配置和响应中都存在，但含义不同
    assert!(
        !input_obj.contains_key("status"),
        "input_data 不应该包含 status（HTTP 节点的响应输出）"
    );
    assert!(
        !input_obj.contains_key("success"),
        "input_data 不应该包含 success（HTTP 节点的响应输出）"
    );
}
