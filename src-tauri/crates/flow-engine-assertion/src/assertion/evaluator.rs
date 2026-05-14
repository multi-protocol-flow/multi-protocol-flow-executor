//! 断言求值器
//!
//! 执行断言检查并返回结果，支持完整 `JSONPath` (RFC 9535) 语法和 all/any 模式。

use crate::assertion::operator::evaluate_operator;
use flow_engine_core::error::{ErrorDetail, ErrorKind};
use flow_engine_core::node_config::{
    Assertion, AssertionConfig, AssertionMode, AssertionOutput, AssertionResult,
};
use jsonpath_rust::JsonPath;
use serde_json::Value;

/// 断言求值器
///
/// 负责执行断言检查，支持：
/// - 完整 `JSONPath` 语法 (RFC 9535)
/// - all/any 组合模式
/// - `continue_on_failure` 行为
pub struct AssertionEvaluator;

impl AssertionEvaluator {
    /// 创建新的断言求值器
    pub fn new() -> Self {
        Self
    }

    /// 从输入数据中提取目标值
    ///
    /// # 参数
    /// - `input`: 输入 JSON 数据
    /// - `path`: `JSONPath` 路径表达式
    ///
    /// # 返回
    /// - `Ok(Value)`: 提取的值（字段不存在时返回 `Value::Null`）
    /// - `Err(ErrorDetail)`: `JSONPath` 语法错误
    ///
    /// # 支持的 `JSONPath` 语法
    /// - `$.status` - 顶级字段
    /// - `$.body.data.items[0].id` - 嵌套 + 数组索引
    /// - `$.body.items[*].name` - 通配符
    /// - `$.body.items[?(@.price < 100)]` - 过滤器
    pub fn extract_target(input: &Value, path: &str) -> Result<Value, ErrorDetail> {
        // 空路径返回 null
        if path.is_empty() {
            return Ok(Value::Null);
        }

        // 使用 JsonPath trait 的 query 方法
        // JsonPath trait 为 serde_json::Value 实现了 query 方法
        let result = input.query(path).map_err(|e| {
            ErrorDetail::new(
                ErrorKind::ConditionSyntax,
                format!("无效的 JSONPath 语法 '{}': {}", path, e),
            )
        })?;

        // 如果结果是数组且只有一个元素，返回该元素
        // 如果结果是数组且有多个元素，返回数组
        // 如果结果是空数组，返回 null
        if result.len() == 1 {
            // 单个结果：返回克隆的值
            Ok(result[0].clone())
        } else if result.is_empty() {
            Ok(Value::Null)
        } else {
            // 多个结果：转换为数组
            Ok(Value::Array(result.into_iter().cloned().collect()))
        }
    }

    /// 评估单个断言
    ///
    /// # 参数
    /// - `assertion`: 断言配置
    /// - `input`: 输入 JSON 数据
    ///
    /// # 返回
    /// 断言结果
    pub fn evaluate_single(assertion: &Assertion, input: &Value) -> AssertionResult {
        // 提取目标值
        let actual = match Self::extract_target(input, &assertion.target) {
            Ok(v) => v,
            Err(e) => {
                return AssertionResult {
                    id: assertion.id.clone(),
                    description: assertion.description.clone(),
                    passed: false,
                    target: assertion.target.clone(),
                    operator: format!("{:?}", assertion.operator),
                    expected: assertion.expected.clone(),
                    actual: Value::Null,
                    error: Some(e.message),
                };
            }
        };

        // 评估操作符
        let result = evaluate_operator(&assertion.operator, &actual, &assertion.expected);

        match result {
            Ok(passed) => AssertionResult {
                id: assertion.id.clone(),
                description: assertion.description.clone(),
                passed,
                target: assertion.target.clone(),
                operator: format!("{:?}", assertion.operator),
                expected: assertion.expected.clone(),
                actual,
                error: None,
            },
            Err(error_msg) => AssertionResult {
                id: assertion.id.clone(),
                description: assertion.description.clone(),
                passed: false,
                target: assertion.target.clone(),
                operator: format!("{:?}", assertion.operator),
                expected: assertion.expected.clone(),
                actual,
                error: Some(error_msg),
            },
        }
    }

    /// 评估所有断言
    ///
    /// # 参数
    /// - `config`: 断言配置（包含 mode, `continue_on_failure`, assertions）
    /// - `input`: 输入 JSON 数据
    ///
    /// # 返回
    /// 断言输出（包含整体结果、各断言结果等）
    pub fn evaluate_all(config: &AssertionConfig, input: &Value) -> AssertionOutput {
        let mut results = Vec::new();
        let mut failed_assertions = Vec::new();
        let mut passed_count = 0;
        let mut failed_count = 0;
        let mut has_fatal_error = false;

        for assertion in &config.assertions {
            // 跳过禁用的断言
            if !assertion.enabled {
                continue;
            }

            let result = Self::evaluate_single(assertion, input);
            let passed = result.passed;

            if passed {
                passed_count += 1;
            } else {
                failed_count += 1;
                failed_assertions.push(result.clone());

                // 检查是否为致命错误（JSONPath 语法错误）
                if result
                    .error
                    .as_ref()
                    .is_some_and(|e| e.contains("无效的 JSONPath 语法"))
                {
                    has_fatal_error = true;
                }
            }

            results.push(result);

            // 如果不继续执行后续断言，跳出循环
            // 但在 any 模式下，如果已经通过，也跳出
            if !config.continue_on_failure && !passed {
                match config.mode {
                    AssertionMode::All => {
                        // all 模式：失败后不继续（除非 continue_on_failure 为 true）
                        break;
                    }
                    AssertionMode::Any => {
                        // any 模式：继续执行直到找到通过的
                        // 但这里已经失败了，所以继续
                    }
                }
            }
        }

        // 计算整体结果
        let overall_passed = match config.mode {
            AssertionMode::All => {
                // all 模式：所有启用的断言都通过
                failed_count == 0 && !has_fatal_error
            }
            AssertionMode::Any => {
                // any 模式：至少一个断言通过
                passed_count > 0
            }
        };

        let total = passed_count + failed_count;

        AssertionOutput {
            passed: overall_passed,
            total,
            passed_count,
            failed_count,
            results,
            failed_assertions,
            input: input.clone(),
        }
    }
}

impl Default for AssertionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node_config::{
        Assertion, AssertionConfig, AssertionMode, AssertionOperator,
    };
    use serde_json::json;

    // ========================================================================
    // JSONPath 提取测试
    // ========================================================================

    #[test]
    fn test_extract_target_simple_field() {
        let input = json!({
            "status": 200,
            "body": {"data": "test"}
        });

        // 顶级字段
        let result = AssertionEvaluator::extract_target(&input, "$.status");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!(200));
    }

    #[test]
    fn test_extract_target_nested_field() {
        let input = json!({
            "body": {
                "data": {
                    "token": "secret_token",
                    "user": {
                        "id": 1,
                        "name": "alice"
                    }
                }
            }
        });

        // 嵌套对象
        let result = AssertionEvaluator::extract_target(&input, "$.body.data.token");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("secret_token"));

        // 深层嵌套
        let result = AssertionEvaluator::extract_target(&input, "$.body.data.user.name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("alice"));
    }

    #[test]
    fn test_extract_target_array_index() {
        let input = json!({
            "body": {
                "items": [
                    {"id": 1, "name": "Item1"},
                    {"id": 2, "name": "Item2"},
                    {"id": 3, "name": "Item3"}
                ]
            }
        });

        // 数组索引 [0]
        let result = AssertionEvaluator::extract_target(&input, "$.body.items[0]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({"id": 1, "name": "Item1"}));

        // 数组索引 + 嵌套字段
        let result = AssertionEvaluator::extract_target(&input, "$.body.items[1].name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("Item2"));
    }

    #[test]
    fn test_extract_target_wildcard() {
        let input = json!({
            "body": {
                "items": [
                    {"id": 1, "name": "Item1"},
                    {"id": 2, "name": "Item2"}
                ]
            }
        });

        // 通配符 [*]
        let result = AssertionEvaluator::extract_target(&input, "$.body.items[*].name");
        assert!(result.is_ok());
        // 通配符返回数组
        assert_eq!(result.unwrap(), json!(["Item1", "Item2"]));
    }

    #[test]
    fn test_extract_target_filter() {
        let input = json!({
            "body": {
                "items": [
                    {"id": 1, "price": 50},
                    {"id": 2, "price": 150},
                    {"id": 3, "price": 75}
                ]
            }
        });

        // 过滤器 [?(@.price < 100)]
        let result = AssertionEvaluator::extract_target(&input, "$.body.items[?(@.price < 100)]");
        assert!(result.is_ok());
        let filtered = result.unwrap();
        // 过滤器返回数组
        assert!(filtered.is_array());
        let arr = filtered.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_extract_target_nonexistent_path() {
        let input = json!({"status": 200});

        // 不存在的路径返回 null
        let result = AssertionEvaluator::extract_target(&input, "$.not.exist");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_extract_target_empty_path() {
        let input = json!({"status": 200});

        // 空路径返回 null
        let result = AssertionEvaluator::extract_target(&input, "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_extract_target_invalid_syntax() {
        let input = json!({"status": 200});

        // 无效的 JSONPath 语法
        let result = AssertionEvaluator::extract_target(&input, "invalid[");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.error_kind, ErrorKind::ConditionSyntax);
        assert!(error.message.contains("无效的 JSONPath 语法"));
    }

    #[test]
    fn test_extract_target_complex_filter() {
        let input = json!({
            "body": {
                "users": [
                    {"name": "alice", "age": 25},
                    {"name": "bob", "age": 30},
                    {"name": "charlie", "age": 20}
                ]
            }
        });

        // 复杂过滤器
        let result =
            AssertionEvaluator::extract_target(&input, "$.body.users[?(@.age >= 25)].name");
        assert!(result.is_ok());
        let filtered = result.unwrap();
        assert!(filtered.is_array());
        let arr = filtered.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    // ========================================================================
    // evaluate_single 测试
    // ========================================================================

    #[test]
    fn test_evaluate_single_eq_pass() {
        let assertion = Assertion {
            id: "assert-1".to_string(),
            description: Some("状态码应该是 200".to_string()),
            target: "$.status".to_string(),
            operator: AssertionOperator::Eq,
            expected: json!(200),
            enabled: true,
        };
        let input = json!({"status": 200});

        let result = AssertionEvaluator::evaluate_single(&assertion, &input);

        assert!(result.passed);
        assert_eq!(result.id, "assert-1");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_evaluate_single_eq_fail() {
        let assertion = Assertion {
            id: "assert-1".to_string(),
            description: Some("状态码应该是 200".to_string()),
            target: "$.status".to_string(),
            operator: AssertionOperator::Eq,
            expected: json!(200),
            enabled: true,
        };
        let input = json!({"status": 404});

        let result = AssertionEvaluator::evaluate_single(&assertion, &input);

        assert!(!result.passed);
        assert_eq!(result.actual, json!(404));
    }

    #[test]
    fn test_evaluate_single_contains_string() {
        let assertion = Assertion {
            id: "assert-2".to_string(),
            description: None,
            target: "$.message".to_string(),
            operator: AssertionOperator::Contains,
            expected: json!("success"),
            enabled: true,
        };
        let input = json!({"message": "Operation success completed"});

        let result = AssertionEvaluator::evaluate_single(&assertion, &input);

        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_single_disabled() {
        let assertion = Assertion {
            id: "assert-3".to_string(),
            description: None,
            target: "$.status".to_string(),
            operator: AssertionOperator::Eq,
            expected: json!(200),
            enabled: false, // 禁用
        };
        let input = json!({"status": 500});

        let result = AssertionEvaluator::evaluate_single(&assertion, &input);

        // 禁用的断言仍然会评估，但应该在 evaluate_all 中跳过
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_single_jsonpath_error() {
        let assertion = Assertion {
            id: "assert-4".to_string(),
            description: None,
            target: "invalid[".to_string(),
            operator: AssertionOperator::Eq,
            expected: json!(200),
            enabled: true,
        };
        let input = json!({"status": 200});

        let result = AssertionEvaluator::evaluate_single(&assertion, &input);

        assert!(!result.passed);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("无效的 JSONPath 语法"));
    }

    // ========================================================================
    // evaluate_all 测试 - all 模式
    // ========================================================================

    #[test]
    fn test_evaluate_all_mode_all_pass() {
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: Default::default(),
            continue_on_failure: false,
            assertions: vec![
                Assertion {
                    id: "a1".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(200),
                    enabled: true,
                },
                Assertion {
                    id: "a2".to_string(),
                    description: None,
                    target: "$.body.success".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(true),
                    enabled: true,
                },
            ],
            source_node_id: None,
        };
        let input = json!({
            "status": 200,
            "body": {"success": true}
        });

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(output.passed);
        assert_eq!(output.total, 2);
        assert_eq!(output.passed_count, 2);
        assert_eq!(output.failed_count, 0);
        assert!(output.failed_assertions.is_empty());
    }

    #[test]
    fn test_evaluate_all_mode_all_partial_fail() {
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: Default::default(),
            continue_on_failure: true,
            assertions: vec![
                Assertion {
                    id: "a1".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(200),
                    enabled: true,
                },
                Assertion {
                    id: "a2".to_string(),
                    description: None,
                    target: "$.body.count".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(10),
                    enabled: true,
                },
            ],
            source_node_id: None,
        };
        let input = json!({
            "status": 200,
            "body": {"count": 5}
        });

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(!output.passed); // all 模式，有一个失败则整体失败
        assert_eq!(output.total, 2);
        assert_eq!(output.passed_count, 1);
        assert_eq!(output.failed_count, 1);
        assert_eq!(output.failed_assertions.len(), 1);
    }

    #[test]
    fn test_evaluate_all_mode_all_stop_on_failure() {
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: Default::default(),
            continue_on_failure: false, // 失败后停止
            assertions: vec![
                Assertion {
                    id: "a1".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(500), // 会失败
                    enabled: true,
                },
                Assertion {
                    id: "a2".to_string(),
                    description: None,
                    target: "$.body.count".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(10),
                    enabled: true,
                },
            ],
            source_node_id: None,
        };
        let input = json!({
            "status": 200,
            "body": {"count": 10}
        });

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(!output.passed);
        // continue_on_failure = false，所以第一个失败后停止
        assert_eq!(output.total, 1);
        assert_eq!(output.passed_count, 0);
        assert_eq!(output.failed_count, 1);
    }

    // ========================================================================
    // evaluate_all 测试 - any 模式
    // ========================================================================

    #[test]
    fn test_evaluate_all_mode_any_pass() {
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::Any,
            on_failure: Default::default(),
            continue_on_failure: false,
            assertions: vec![
                Assertion {
                    id: "a1".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(500), // 失败
                    enabled: true,
                },
                Assertion {
                    id: "a2".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(200), // 通过
                    enabled: true,
                },
            ],
            source_node_id: None,
        };
        let input = json!({"status": 200});

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(output.passed); // any 模式，有一个通过则整体通过
        assert!(output.passed_count > 0);
    }

    #[test]
    fn test_evaluate_all_mode_any_all_fail() {
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::Any,
            on_failure: Default::default(),
            continue_on_failure: true,
            assertions: vec![
                Assertion {
                    id: "a1".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(500),
                    enabled: true,
                },
                Assertion {
                    id: "a2".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(404),
                    enabled: true,
                },
            ],
            source_node_id: None,
        };
        let input = json!({"status": 200});

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(!output.passed); // any 模式，全部失败则整体失败
        assert_eq!(output.passed_count, 0);
        assert_eq!(output.failed_count, 2);
    }

    #[test]
    fn test_evaluate_all_skip_disabled() {
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: Default::default(),
            continue_on_failure: true,
            assertions: vec![
                Assertion {
                    id: "a1".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(200),
                    enabled: true,
                },
                Assertion {
                    id: "a2".to_string(),
                    description: None,
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(500),
                    enabled: false, // 禁用
                },
            ],
            source_node_id: None,
        };
        let input = json!({"status": 200});

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(output.passed);
        assert_eq!(output.total, 1); // 只计算启用的断言
        assert_eq!(output.passed_count, 1);
    }

    #[test]
    fn test_evaluate_all_empty_assertions() {
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: Default::default(),
            continue_on_failure: false,
            assertions: vec![],
            source_node_id: None,
        };
        let input = json!({"status": 200});

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(output.passed); // 空断言列表视为通过
        assert_eq!(output.total, 0);
    }

    // ========================================================================
    // 综合测试
    // ========================================================================

    #[test]
    fn test_complex_assertion_scenario() {
        // 模拟 HTTP 响应
        let input = json!({
            "status": 200,
            "headers": {
                "content-type": "application/json"
            },
            "body": {
                "code": 0,
                "message": "success",
                "data": {
                    "user": {
                        "id": 123,
                        "name": "test_user",
                        "email": "test@example.com"
                    },
                    "items": [
                        {"id": 1, "name": "Item1", "price": 100},
                        {"id": 2, "name": "Item2", "price": 200}
                    ]
                }
            }
        });

        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: Default::default(),
            continue_on_failure: true,
            assertions: vec![
                // 状态码检查
                Assertion {
                    id: "status_check".to_string(),
                    description: Some("HTTP 状态码应为 200".to_string()),
                    target: "$.status".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(200),
                    enabled: true,
                },
                // 业务码检查
                Assertion {
                    id: "code_check".to_string(),
                    description: Some("业务码应为 0".to_string()),
                    target: "$.body.code".to_string(),
                    operator: AssertionOperator::Eq,
                    expected: json!(0),
                    enabled: true,
                },
                // 用户 ID 检查
                Assertion {
                    id: "user_id_check".to_string(),
                    description: Some("用户 ID 应大于 0".to_string()),
                    target: "$.body.data.user.id".to_string(),
                    operator: AssertionOperator::Gt,
                    expected: json!(0),
                    enabled: true,
                },
                // 数组类型检查
                Assertion {
                    id: "items_type_check".to_string(),
                    description: Some("items 应为数组类型".to_string()),
                    target: "$.body.data.items".to_string(),
                    operator: AssertionOperator::TypeIs,
                    expected: json!("array"),
                    enabled: true,
                },
            ],
            source_node_id: None,
        };

        let output = AssertionEvaluator::evaluate_all(&config, &input);

        assert!(output.passed);
        assert_eq!(output.total, 4);
        assert_eq!(output.passed_count, 4);
        assert_eq!(output.failed_count, 0);
    }
}
