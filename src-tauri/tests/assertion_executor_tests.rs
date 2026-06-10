// 断言执行器集成测试 - TDD GREEN 阶段

#![allow(missing_docs)]
use flow_engine::context::{ExecutionContext, VariableStorage};
use flow_engine::node::{Node, NodePosition};
use flow_engine::node_config::{
    Assertion, AssertionConfig, AssertionMode, AssertionOperator, NodeData, OnFailureMode,
    AssertionOutput,
};
use flow_engine::executor::NodeExecutor;
use flow_engine::assertion_executor::AssertionNodeExecutor;
use flow_engine::connection::Connection;
use std::sync::Arc;

// ============================================================================
// 测试辅助函数
// ============================================================================

/// 创建执行上下文
fn create_context(flow_id: &str) -> ExecutionContext {
    ExecutionContext {
        flow_uuid: flow_id.to_string(),
        current_node_uuid: None,
        variables: VariableStorage::new(),
        node_reports: Vec::new(),
        flow_trigger_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间错误")
            .as_millis() as u64,
        log_sender: None,
        flow_file_path: None,
        ws_connection_urls: std::collections::HashMap::new(),
        ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        last_node_output: None,
        debug_event_sender: None,
        execution_event_sender: None,
        cancel_receiver: None,
        providers: std::collections::HashMap::new(),
    }
}

/// 创建断言节点
fn create_assertion_node(uuid: &str, config: AssertionConfig) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: "assertion".to_string(),
        name: "断言节点".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::Assertion(config),
        ports: None,
        has_breakpoint: None,
    }
}

/// 创建默认断言配置
fn create_default_assertion_config(assertions: Vec<Assertion>) -> AssertionConfig {
    AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions,
        source_node_id: None,
    }
}

/// 创建单条断言
fn create_assertion(
    id: &str,
    target: &str,
    operator: AssertionOperator,
    expected: serde_json::Value,
) -> Assertion {
    Assertion {
        id: id.to_string(),
        description: None,
        target: target.to_string(),
        operator,
        expected,
        enabled: true,
    }
}

/// 执行断言节点并返回结果
async fn execute_assertion(
    node: &Node,
    context: &mut ExecutionContext,
    connections: &[Connection],
) -> (Vec<String>, AssertionOutput) {
    let executor = AssertionNodeExecutor::new();
    let result = executor
        .execute(node, context, connections, &std::collections::HashMap::new())
        .await;
    assert!(result.is_ok(), "执行应该成功");
    let (next_nodes, output) = result.unwrap();
    let output: AssertionOutput = serde_json::from_value(output.unwrap()).expect("输出反序列化失败");
    (next_nodes, output)
}

// ============================================================================
// 测试：15 种操作符
// ============================================================================

#[tokio::test]
/// 测试 eq 操作符 - 相等
async fn test_operator_eq_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Eq,
        serde_json::json!(200),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
    assert_eq!(output.passed_count, 1);
}

#[tokio::test]
/// 测试 eq 操作符 - 不相等
async fn test_operator_eq_fail() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 404}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Eq,
        serde_json::json!(200),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(!output.passed, "断言应该失败");
    assert_eq!(output.failed_count, 1);
}

#[tokio::test]
/// 测试 ne 操作符 - 不相等
async fn test_operator_ne_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 404}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Ne,
        serde_json::json!(200),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 gt 操作符 - 大于
async fn test_operator_gt_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"count": 10}));

    let assertions = vec![create_assertion(
        "a1",
        "$.count",
        AssertionOperator::Gt,
        serde_json::json!(5),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 gte 操作符 - 大于等于
async fn test_operator_gte_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"count": 10}));

    let assertions = vec![create_assertion(
        "a1",
        "$.count",
        AssertionOperator::Gte,
        serde_json::json!(10),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 lt 操作符 - 小于
async fn test_operator_lt_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"count": 5}));

    let assertions = vec![create_assertion(
        "a1",
        "$.count",
        AssertionOperator::Lt,
        serde_json::json!(10),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 lte 操作符 - 小于等于
async fn test_operator_lte_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"count": 10}));

    let assertions = vec![create_assertion(
        "a1",
        "$.count",
        AssertionOperator::Lte,
        serde_json::json!(10),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 contains 操作符 - 字符串包含
async fn test_operator_contains_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"message": "Hello World"}));

    let assertions = vec![create_assertion(
        "a1",
        "$.message",
        AssertionOperator::Contains,
        serde_json::json!("World"),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 not_contains 操作符 - 不包含
async fn test_operator_not_contains_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"message": "Hello World"}));

    let assertions = vec![create_assertion(
        "a1",
        "$.message",
        AssertionOperator::NotContains,
        serde_json::json!("Error"),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 matches 操作符 - 正则匹配
async fn test_operator_matches_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"email": "test@example.com"}));

    let assertions = vec![create_assertion(
        "a1",
        "$.email",
        AssertionOperator::Matches,
        serde_json::json!("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 exists 操作符 - 字段存在
async fn test_operator_exists_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"name": "test"}));

    let assertions = vec![create_assertion(
        "a1",
        "$.name",
        AssertionOperator::Exists,
        serde_json::json!(true),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 not_exists 操作符 - 字段不存在
async fn test_operator_not_exists_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"name": "test"}));

    let assertions = vec![create_assertion(
        "a1",
        "$.error",
        AssertionOperator::NotExists,
        serde_json::json!(true),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 in 操作符 - 值在列表中
async fn test_operator_in_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::In,
        serde_json::json!([200, 201, 204]),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 not_in 操作符 - 值不在列表中
async fn test_operator_not_in_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 404}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::NotIn,
        serde_json::json!([200, 201, 204]),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 type_is 操作符 - 类型检查
async fn test_operator_type_is_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"count": 42}));

    let assertions = vec![create_assertion(
        "a1",
        "$.count",
        AssertionOperator::TypeIs,
        serde_json::json!("number"),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 between 操作符 - 范围检查
async fn test_operator_between_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Between,
        serde_json::json!([200, 299]),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

// ============================================================================
// 测试：JSONPath 提取
// ============================================================================

#[tokio::test]
/// 测试 JSONPath 嵌套路径提取
async fn test_jsonpath_nested_path() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({
        "body": {
            "data": {
                "items": [
                    {"id": 1, "name": "item1"},
                    {"id": 2, "name": "item2"}
                ]
            }
        }
    }));

    let assertions = vec![create_assertion(
        "a1",
        "$.body.data.items[0].id",
        AssertionOperator::Eq,
        serde_json::json!(1),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 JSONPath 通配符
async fn test_jsonpath_wildcard() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({
        "items": [
            {"name": "a"},
            {"name": "b"},
            {"name": "c"}
        ]
    }));

    let assertions = vec![create_assertion(
        "a1",
        "$.items[*].name",
        AssertionOperator::Contains,
        serde_json::json!("a"),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "断言应该通过");
}

#[tokio::test]
/// 测试 JSONPath 语法错误
async fn test_jsonpath_syntax_error() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let assertions = vec![create_assertion(
        "a1",
        "$.[invalid", // 无效的 JSONPath 语法
        AssertionOperator::Eq,
        serde_json::json!(200),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    // JSONPath 语法错误会导致断言失败
    assert!(!output.passed, "JSONPath 语法错误应该导致失败");
    assert!(output.results[0].error.is_some(), "应该有错误信息");
}

// ============================================================================
// 测试：all/any 模式
// ============================================================================

#[tokio::test]
/// 测试 all 模式 - 全部通过
async fn test_mode_all_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200, "count": 10}));

    let assertions = vec![
        create_assertion("a1", "$.status", AssertionOperator::Eq, serde_json::json!(200)),
        create_assertion("a2", "$.count", AssertionOperator::Gt, serde_json::json!(5)),
    ];
    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions,
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "all 模式，全部通过");
}

#[tokio::test]
/// 测试 all 模式 - 部分失败
async fn test_mode_all_partial_fail() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200, "count": 3}));

    let assertions = vec![
        create_assertion("a1", "$.status", AssertionOperator::Eq, serde_json::json!(200)),
        create_assertion("a2", "$.count", AssertionOperator::Gt, serde_json::json!(5)),
    ];
    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions,
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(!output.passed, "all 模式，有一失败则失败");
}

#[tokio::test]
/// 测试 any 模式 - 任一通过
async fn test_mode_any_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200, "count": 3}));

    let assertions = vec![
        create_assertion("a1", "$.status", AssertionOperator::Eq, serde_json::json!(200)),
        create_assertion("a2", "$.count", AssertionOperator::Gt, serde_json::json!(5)),
    ];
    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::Any,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions,
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "any 模式，有一通过则通过");
}

// ============================================================================
// 测试：预设
// ============================================================================

#[tokio::test]
/// 测试 http_success 预设
async fn test_preset_http_success() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let config = AssertionConfig {
        preset: Some("http_success".to_string()),
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions: vec![],
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "http_success 预设应该通过");
}

#[tokio::test]
/// 测试 http_client_error 预设
async fn test_preset_http_client_error() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 404}));

    let config = AssertionConfig {
        preset: Some("http_client_error".to_string()),
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions: vec![],
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "http_client_error 预设应该通过");
}

#[tokio::test]
/// 测试 http_server_error 预设
async fn test_preset_http_server_error() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 500}));

    let config = AssertionConfig {
        preset: Some("http_server_error".to_string()),
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions: vec![],
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "http_server_error 预设应该通过");
}

#[tokio::test]
/// 测试 json_response 预设
async fn test_preset_json_response() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({
        "headers": {"Content-Type": "application/json"},
        "body": {}
    }));

    let config = AssertionConfig {
        preset: Some("json_response".to_string()),
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions: vec![],
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "json_response 预设应该通过");
}

#[tokio::test]
/// 测试 response_fast 预设
async fn test_preset_response_fast() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"response_time": 100}));

    let config = AssertionConfig {
        preset: Some("response_fast".to_string()),
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions: vec![],
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "response_fast 预设应该通过");
}

// ============================================================================
// 测试：边界情况
// ============================================================================

#[tokio::test]
/// 测试空断言列表 - Pass + warning
async fn test_empty_assertions_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions: vec![],
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "空断言列表应该通过");
    assert_eq!(output.total, 0);
}

#[tokio::test]
/// 测试全禁用断言 - Pass + warning
async fn test_all_disabled_assertions_pass() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let mut assertion = create_assertion("a1", "$.status", AssertionOperator::Eq, serde_json::json!(999));
    assertion.enabled = false;

    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions: vec![assertion],
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(output.passed, "全禁用断言应该通过");
    assert_eq!(output.total, 0); // 禁用的不计入
}

#[tokio::test]
/// 测试类型不匹配 - 严格比较
async fn test_type_mismatch_strict_comparison() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": "200"})); // 字符串

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Eq,
        serde_json::json!(200), // 数字
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    assert!(!output.passed, "严格比较，类型不同应该失败");
}

#[tokio::test]
/// 测试 route_false 失败行为
async fn test_on_failure_route_false() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 404}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Eq,
        serde_json::json!(200),
    )];
    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: false,
        assertions,
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let connections = vec![
        Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "assertion-1".to_string(),
            target_node_uuid: "next-true".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "in".to_string(),
        },
        Connection {
            id: "conn-2".to_string(),
            source_node_uuid: "assertion-1".to_string(),
            target_node_uuid: "next-false".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "in".to_string(),
        },
    ];

    let (next_nodes, output) = execute_assertion(&node, &mut context, &connections).await;
    assert!(!output.passed, "断言应该失败");
    assert_eq!(next_nodes, vec!["next-false"], "应该路由到 false 端口");
}

#[tokio::test]
/// 测试 throw 失败行为
async fn test_on_failure_throw() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 404}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Eq,
        serde_json::json!(200),
    )];
    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::Throw,
        continue_on_failure: false,
        assertions,
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let executor = AssertionNodeExecutor::new();
    let result = executor
        .execute(&node, &mut context, &[], &std::collections::HashMap::new())
        .await;

    assert!(result.is_err(), "on_failure=throw 应该返回错误");
    let error = result.unwrap_err();
    assert!(error.contains("断言失败"), "错误消息应该包含 '断言失败'");
}

#[tokio::test]
/// 测试 continue_on_failure
async fn test_continue_on_failure() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 404, "count": 10}));

    let assertions = vec![
        create_assertion("a1", "$.status", AssertionOperator::Eq, serde_json::json!(200)),
        create_assertion("a2", "$.count", AssertionOperator::Gt, serde_json::json!(5)),
    ];
    let config = AssertionConfig {
        preset: None,
        mode: AssertionMode::All,
        on_failure: OnFailureMode::RouteFalse,
        continue_on_failure: true,
        assertions,
        source_node_id: None,
    };
    let node = create_assertion_node("assertion-1", config);

    let (_, output) = execute_assertion(&node, &mut context, &[]).await;
    // 两个断言都执行了
    assert_eq!(output.total, 2, "应该执行所有断言");
    assert_eq!(output.passed_count, 1);
    assert_eq!(output.failed_count, 1);
}

#[tokio::test]
/// 测试双端口路由 - true 端口
async fn test_dual_routing_true_port() {
    let mut context = create_context("test-flow");
    context.last_node_output = Some(serde_json::json!({"status": 200}));

    let assertions = vec![create_assertion(
        "a1",
        "$.status",
        AssertionOperator::Eq,
        serde_json::json!(200),
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    let connections = vec![
        Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "assertion-1".to_string(),
            target_node_uuid: "next-true".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "in".to_string(),
        },
        Connection {
            id: "conn-2".to_string(),
            source_node_uuid: "assertion-1".to_string(),
            target_node_uuid: "next-false".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "in".to_string(),
        },
    ];

    let (next_nodes, output) = execute_assertion(&node, &mut context, &connections).await;
    assert!(output.passed, "断言应该通过");
    assert_eq!(next_nodes, vec!["next-true"], "应该路由到 true 端口");
}

#[tokio::test]
/// 测试 Regex DoS 防护 (100ms timeout)
async fn test_regex_dos_protection() {
    let mut context = create_context("test-flow");
    // 创建一个可能导致 ReDoS 的输入
    context.last_node_output = Some(serde_json::json!({
        "input": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!"
    }));

    let assertions = vec![create_assertion(
        "a1",
        "$.input",
        AssertionOperator::Matches,
        serde_json::json!("^(a+)+$"), // 经典的 ReDoS 模式
    )];
    let config = create_default_assertion_config(assertions);
    let node = create_assertion_node("assertion-1", config);

    // 使用超时包装测试，确保不会无限阻塞
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        execute_assertion(&node, &mut context, &[])
    ).await;

    // 应该在 5 秒内完成（即使触发 ReDoS 防护）
    assert!(result.is_ok(), "正则匹配应该在超时前完成");
    let (_, output) = result.unwrap();
    // ReDoS 模式不应该匹配带 ! 的字符串
    assert!(!output.passed, "ReDoS 模式不应该匹配");
}
