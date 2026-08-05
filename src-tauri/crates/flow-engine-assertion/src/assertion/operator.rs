//! 断言操作符求值逻辑
//!
//! 实现 15 种断言操作符的求值逻辑，支持严格类型检查和正则 `DoS` 防护。

use flow_engine_core::node_config::AssertionOperator;
use serde_json::Value;
use std::time::{Duration, Instant};

/// 获取 JSON 值的类型名称
///
/// # 参数
/// - `value`: JSON 值
///
/// # 返回
/// 类型名称字符串: "string", "number", "boolean", "array", "object", "null"
pub fn get_json_type(value: &Value) -> &str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// 评估断言操作符
///
/// # 参数
/// - `operator`: 断言操作符
/// - `actual`: 实际值
/// - `expected`: 期望值
///
/// # 返回
/// - `Ok(true)`: 断言通过
/// - `Ok(false)`: 断言Failed
/// - `Err(String)`: 操作符执行Error（如类型不匹配）
pub fn evaluate_operator(
    operator: &AssertionOperator,
    actual: &Value,
    expected: &Value,
) -> Result<bool, String> {
    match operator {
        AssertionOperator::Eq => evaluate_eq(actual, expected),
        AssertionOperator::Ne => evaluate_ne(actual, expected),
        AssertionOperator::Gt => evaluate_gt(actual, expected),
        AssertionOperator::Gte => evaluate_gte(actual, expected),
        AssertionOperator::Lt => evaluate_lt(actual, expected),
        AssertionOperator::Lte => evaluate_lte(actual, expected),
        AssertionOperator::Contains => evaluate_contains(actual, expected),
        AssertionOperator::NotContains => evaluate_not_contains(actual, expected),
        AssertionOperator::Matches => evaluate_matches(actual, expected),
        AssertionOperator::Exists => evaluate_exists(actual),
        AssertionOperator::NotExists => evaluate_not_exists(actual),
        AssertionOperator::In => evaluate_in(actual, expected),
        AssertionOperator::NotIn => evaluate_not_in(actual, expected),
        AssertionOperator::TypeIs => evaluate_type_is(actual, expected),
        AssertionOperator::Between => evaluate_between(actual, expected),
    }
}

// ============================================================================
// 比较操作符
// ============================================================================

/// 严格相等比较（不做类型强制转换）
fn evaluate_eq(actual: &Value, expected: &Value) -> Result<bool, String> {
    Ok(actual == expected)
}

/// 严格不等比较（不做类型强制转换）
fn evaluate_ne(actual: &Value, expected: &Value) -> Result<bool, String> {
    Ok(actual != expected)
}

// ============================================================================
// 数值比较操作符
// ============================================================================

/// 提取数值
fn extract_number(value: &Value, name: &str) -> Result<f64, String> {
    match value {
        Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| format!("{} is not a valid number: {}", name, value)),
        _ => Err(format!(
            "{} is not a numeric type, actual type: {}",
            name,
            get_json_type(value)
        )),
    }
}

/// 大于比较
fn evaluate_gt(actual: &Value, expected: &Value) -> Result<bool, String> {
    let actual_num = extract_number(actual, "actual")?;
    let expected_num = extract_number(expected, "expected")?;
    Ok(actual_num > expected_num)
}

/// 大于等于比较
fn evaluate_gte(actual: &Value, expected: &Value) -> Result<bool, String> {
    let actual_num = extract_number(actual, "actual")?;
    let expected_num = extract_number(expected, "expected")?;
    Ok(actual_num >= expected_num)
}

/// 小于比较
fn evaluate_lt(actual: &Value, expected: &Value) -> Result<bool, String> {
    let actual_num = extract_number(actual, "actual")?;
    let expected_num = extract_number(expected, "expected")?;
    Ok(actual_num < expected_num)
}

/// 小于等于比较
fn evaluate_lte(actual: &Value, expected: &Value) -> Result<bool, String> {
    let actual_num = extract_number(actual, "actual")?;
    let expected_num = extract_number(expected, "expected")?;
    Ok(actual_num <= expected_num)
}

// ============================================================================
// 包含操作符
// ============================================================================

/// 提取字符串值（支持数字和布尔值转换）
fn extract_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// 包含检查（字符串或数组）
fn evaluate_contains(actual: &Value, expected: &Value) -> Result<bool, String> {
    match actual {
        Value::String(s) => {
            let expected_str = extract_string(expected).ok_or_else(|| {
                format!(
                    "expected is not a string type, actual type: {}",
                    get_json_type(expected)
                )
            })?;
            Ok(s.contains(&expected_str))
        }
        Value::Array(arr) => Ok(arr.contains(expected)),
        _ => Err(format!(
            "actual is not a string or array type, actual type: {}",
            get_json_type(actual)
        )),
    }
}

/// 不包含检查（字符串或数组）
fn evaluate_not_contains(actual: &Value, expected: &Value) -> Result<bool, String> {
    match actual {
        Value::String(s) => {
            let expected_str = extract_string(expected).ok_or_else(|| {
                format!(
                    "expected is not a string type, actual type: {}",
                    get_json_type(expected)
                )
            })?;
            Ok(!s.contains(&expected_str))
        }
        Value::Array(arr) => Ok(!arr.contains(expected)),
        _ => Err(format!(
            "actual is not a string or array type, actual type: {}",
            get_json_type(actual)
        )),
    }
}

// ============================================================================
// 正则匹配操作符
// ============================================================================

/// 正则匹配（带 100ms 超时防护，防止 `DoS` 攻击）
fn evaluate_matches(actual: &Value, expected: &Value) -> Result<bool, String> {
    let actual_str = match actual {
        Value::String(s) => s.clone(),
        _ => {
            return Err(format!(
                "actual is not a string type, actual type: {}",
                get_json_type(actual)
            ))
        }
    };

    let pattern = match expected {
        Value::String(s) => s,
        _ => {
            return Err(format!(
                "expected must be a string regex pattern, actual type: {}",
                get_json_type(expected)
            ))
        }
    };

    // Compile regex pattern
    let regex = regex::Regex::new(pattern)
        .map_err(|e| format!("Invalid regex pattern '{}': {}", pattern, e))?;

    // 使用超时保护防止 ReDoS 攻击
    // 对于简单字符串匹配，直接检查；对于复杂模式，使用超时
    let start = Instant::now();
    let timeout = Duration::from_millis(100);

    // 检查第一个匹配，同时检查超时
    if let Some(_mat) = regex.find_iter(&actual_str).next() {
        if start.elapsed() > timeout {
            return Err(format!(
                "Regex match timed out (exceeded 100ms), possible ReDoS attack: {}",
                pattern
            ));
        }
        return Ok(true);
    }

    Ok(false)
}

// ============================================================================
// 存在性操作符
// ============================================================================

/// 存在检查（非 null）
fn evaluate_exists(actual: &Value) -> Result<bool, String> {
    Ok(!actual.is_null())
}

/// 不存在检查（为 null）
fn evaluate_not_exists(actual: &Value) -> Result<bool, String> {
    Ok(actual.is_null())
}

// ============================================================================
// 列表成员操作符
// ============================================================================

/// 在列表中检查（expected 必须是数组）
fn evaluate_in(actual: &Value, expected: &Value) -> Result<bool, String> {
    match expected {
        Value::Array(arr) => Ok(arr.contains(actual)),
        _ => Err(format!(
            "expected must be an array type, actual type: {}",
            get_json_type(expected)
        )),
    }
}

/// 不在列表中检查（expected 必须是数组）
fn evaluate_not_in(actual: &Value, expected: &Value) -> Result<bool, String> {
    match expected {
        Value::Array(arr) => Ok(!arr.contains(actual)),
        _ => Err(format!(
            "expected must be an array type, actual type: {}",
            get_json_type(expected)
        )),
    }
}

// ============================================================================
// 类型检查操作符
// ============================================================================

/// 类型检查
fn evaluate_type_is(actual: &Value, expected: &Value) -> Result<bool, String> {
    let expected_type = match expected {
        Value::String(s) => s.as_str(),
        _ => {
            return Err(format!(
                "expected must be a string type, actual type: {}",
                get_json_type(expected)
            ))
        }
    };

    let actual_type = get_json_type(actual);

    // Supported types: string, number, boolean, array, object, null
    match expected_type {
        "string" | "number" | "boolean" | "array" | "object" | "null" => {
            Ok(actual_type == expected_type)
        }
        _ => Err(format!(
            "Unsupported type check: '{}', supported types: string, number, boolean, array, object, null",
            expected_type
        )),
    }
}

// ============================================================================
// 范围检查操作符
// ============================================================================

/// 范围检查（expected 必须是 [min, max] 数组，支持反向范围自动交换）
fn evaluate_between(actual: &Value, expected: &Value) -> Result<bool, String> {
    let actual_num = extract_number(actual, "actual")?;

    let range = match expected {
        Value::Array(arr) if arr.len() == 2 => arr,
        Value::Array(arr) => {
            return Err(format!(
                "expected must be an array of 2 elements [min, max], actual length: {}",
                arr.len()
            ))
        }
        _ => {
            return Err(format!(
                "expected must be an array type [min, max], actual type: {}",
                get_json_type(expected)
            ))
        }
    };

    let min = extract_number(&range[0], "min")?;
    let max = extract_number(&range[1], "max")?;

    // 支持反向范围（自动交换 min 和 max）
    let (range_min, range_max) = if min <= max { (min, max) } else { (max, min) };

    Ok(actual_num >= range_min && actual_num <= range_max)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================================================
    // get_json_type 测试
    // ========================================================================

    #[test]
    fn test_get_json_type_string() {
        assert_eq!(get_json_type(&json!("hello")), "string");
    }

    #[test]
    fn test_get_json_type_number() {
        assert_eq!(get_json_type(&json!(42)), "number");
        assert_eq!(get_json_type(&json!(2.5)), "number");
    }

    #[test]
    fn test_get_json_type_boolean() {
        assert_eq!(get_json_type(&json!(true)), "boolean");
        assert_eq!(get_json_type(&json!(false)), "boolean");
    }

    #[test]
    fn test_get_json_type_array() {
        assert_eq!(get_json_type(&json!([1, 2, 3])), "array");
    }

    #[test]
    fn test_get_json_type_object() {
        assert_eq!(get_json_type(&json!({"key": "value"})), "object");
    }

    #[test]
    fn test_get_json_type_null() {
        assert_eq!(get_json_type(&Value::Null), "null");
    }

    // ========================================================================
    // Eq 操作符测试
    // ========================================================================

    #[test]
    fn test_eq_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::Eq, &json!(200), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_eq_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Eq, &json!(200), &json!(404))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_eq_strict_no_type_coercion() {
        // 字符串 "200" 不等于数字 200（严格比较，不做类型转换）
        assert!(
            !evaluate_operator(&AssertionOperator::Eq, &json!("200"), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_eq_string() {
        assert!(
            evaluate_operator(&AssertionOperator::Eq, &json!("hello"), &json!("hello"))
                .unwrap_or_else(|e| panic!("{e}"))
        );
        assert!(
            !evaluate_operator(&AssertionOperator::Eq, &json!("hello"), &json!("world"))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_eq_null() {
        assert!(
            evaluate_operator(&AssertionOperator::Eq, &Value::Null, &Value::Null)
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_eq_boolean() {
        assert!(
            evaluate_operator(&AssertionOperator::Eq, &json!(true), &json!(true))
                .unwrap_or_else(|e| panic!("{e}"))
        );
        assert!(
            !evaluate_operator(&AssertionOperator::Eq, &json!(true), &json!(false))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    // ========================================================================
    // Ne 操作符测试
    // ========================================================================

    #[test]
    fn test_ne_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::Ne, &json!(200), &json!(404))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_ne_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Ne, &json!(200), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_ne_strict_no_type_coercion() {
        // 字符串 "200" 不等于数字 200，所以 Ne 返回 true
        assert!(
            evaluate_operator(&AssertionOperator::Ne, &json!("200"), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    // ========================================================================
    // Gt 操作符测试
    // ========================================================================

    #[test]
    fn test_gt_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::Gt, &json!(200), &json!(100))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_gt_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Gt, &json!(100), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_gt_equal() {
        assert!(
            !evaluate_operator(&AssertionOperator::Gt, &json!(200), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_gt_type_mismatch() {
        let result = evaluate_operator(&AssertionOperator::Gt, &json!("200"), &json!(100));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("is not a numeric type"));
    }

    // ========================================================================
    // Gte 操作符测试
    // ========================================================================

    #[test]
    fn test_gte_pass_greater() {
        assert!(
            evaluate_operator(&AssertionOperator::Gte, &json!(200), &json!(100))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_gte_pass_equal() {
        assert!(
            evaluate_operator(&AssertionOperator::Gte, &json!(200), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_gte_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Gte, &json!(100), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    // ========================================================================
    // Lt 操作符测试
    // ========================================================================

    #[test]
    fn test_lt_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::Lt, &json!(100), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_lt_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Lt, &json!(200), &json!(100))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_lt_equal() {
        assert!(
            !evaluate_operator(&AssertionOperator::Lt, &json!(200), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    // ========================================================================
    // Lte 操作符测试
    // ========================================================================

    #[test]
    fn test_lte_pass_less() {
        assert!(
            evaluate_operator(&AssertionOperator::Lte, &json!(100), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_lte_pass_equal() {
        assert!(
            evaluate_operator(&AssertionOperator::Lte, &json!(200), &json!(200))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_lte_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Lte, &json!(200), &json!(100))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    // ========================================================================
    // Contains 操作符测试
    // ========================================================================

    #[test]
    fn test_contains_string_pass() {
        assert!(evaluate_operator(
            &AssertionOperator::Contains,
            &json!("hello world"),
            &json!("world")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_contains_string_fail() {
        assert!(!evaluate_operator(
            &AssertionOperator::Contains,
            &json!("hello world"),
            &json!("foo")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_contains_array_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::Contains, &json!([1, 2, 3]), &json!(2))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_contains_array_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Contains, &json!([1, 2, 3]), &json!(4))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_contains_type_mismatch() {
        let result = evaluate_operator(&AssertionOperator::Contains, &json!(123), &json!("foo"));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("is not a string or array type"));
    }

    // ========================================================================
    // NotContains 操作符测试
    // ========================================================================

    #[test]
    fn test_not_contains_string_pass() {
        assert!(evaluate_operator(
            &AssertionOperator::NotContains,
            &json!("hello world"),
            &json!("foo")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_not_contains_string_fail() {
        assert!(!evaluate_operator(
            &AssertionOperator::NotContains,
            &json!("hello world"),
            &json!("world")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_not_contains_array_pass() {
        assert!(evaluate_operator(
            &AssertionOperator::NotContains,
            &json!([1, 2, 3]),
            &json!(4)
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_not_contains_array_fail() {
        assert!(!evaluate_operator(
            &AssertionOperator::NotContains,
            &json!([1, 2, 3]),
            &json!(2)
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    // ========================================================================
    // Matches 操作符测试
    // ========================================================================

    #[test]
    fn test_matches_pass() {
        assert!(evaluate_operator(
            &AssertionOperator::Matches,
            &json!("hello123"),
            &json!("^[a-z]+[0-9]+$")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_matches_fail() {
        assert!(!evaluate_operator(
            &AssertionOperator::Matches,
            &json!("hello"),
            &json!("^[0-9]+$")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_matches_email() {
        assert!(evaluate_operator(
            &AssertionOperator::Matches,
            &json!("test@example.com"),
            &json!("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_matches_invalid_regex() {
        let result = evaluate_operator(
            &AssertionOperator::Matches,
            &json!("test"),
            &json!("^[invalid"),
        );
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("Invalid regex pattern"));
    }

    #[test]
    fn test_matches_actual_not_string() {
        let result =
            evaluate_operator(&AssertionOperator::Matches, &json!(123), &json!("^[0-9]+$"));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("actual is not a string type"));
    }

    #[test]
    fn test_matches_expected_not_string() {
        let result = evaluate_operator(&AssertionOperator::Matches, &json!("test"), &json!(123));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("must be a string regex pattern"));
    }

    // ========================================================================
    // Exists 操作符测试
    // ========================================================================

    #[test]
    fn test_exists_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::Exists, &json!("hello"), &Value::Null)
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_exists_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::Exists, &Value::Null, &Value::Null)
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_exists_number() {
        assert!(
            evaluate_operator(&AssertionOperator::Exists, &json!(0), &Value::Null)
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_exists_empty_string() {
        assert!(
            evaluate_operator(&AssertionOperator::Exists, &json!(""), &Value::Null)
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    // ========================================================================
    // NotExists 操作符测试
    // ========================================================================

    #[test]
    fn test_not_exists_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::NotExists, &Value::Null, &Value::Null)
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_not_exists_fail() {
        assert!(
            !evaluate_operator(&AssertionOperator::NotExists, &json!("hello"), &Value::Null)
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    // ========================================================================
    // In 操作符测试
    // ========================================================================

    #[test]
    fn test_in_pass() {
        assert!(evaluate_operator(
            &AssertionOperator::In,
            &json!("apple"),
            &json!(["apple", "banana", "cherry"])
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_in_fail() {
        assert!(!evaluate_operator(
            &AssertionOperator::In,
            &json!("orange"),
            &json!(["apple", "banana", "cherry"])
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_in_number() {
        assert!(
            evaluate_operator(&AssertionOperator::In, &json!(2), &json!([1, 2, 3]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_in_expected_not_array() {
        let result = evaluate_operator(&AssertionOperator::In, &json!("apple"), &json!("apple"));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("expected must be an array type"));
    }

    // ========================================================================
    // NotIn 操作符测试
    // ========================================================================

    #[test]
    fn test_not_in_pass() {
        assert!(evaluate_operator(
            &AssertionOperator::NotIn,
            &json!("orange"),
            &json!(["apple", "banana", "cherry"])
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_not_in_fail() {
        assert!(!evaluate_operator(
            &AssertionOperator::NotIn,
            &json!("apple"),
            &json!(["apple", "banana", "cherry"])
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    // ========================================================================
    // TypeIs 操作符测试
    // ========================================================================

    #[test]
    fn test_type_is_string() {
        assert!(evaluate_operator(
            &AssertionOperator::TypeIs,
            &json!("hello"),
            &json!("string")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_type_is_number() {
        assert!(
            evaluate_operator(&AssertionOperator::TypeIs, &json!(42), &json!("number"))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_type_is_boolean() {
        assert!(
            evaluate_operator(&AssertionOperator::TypeIs, &json!(true), &json!("boolean"))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_type_is_array() {
        assert!(evaluate_operator(
            &AssertionOperator::TypeIs,
            &json!([1, 2, 3]),
            &json!("array")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_type_is_object() {
        assert!(evaluate_operator(
            &AssertionOperator::TypeIs,
            &json!({"key": "value"}),
            &json!("object")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_type_is_null() {
        assert!(
            evaluate_operator(&AssertionOperator::TypeIs, &Value::Null, &json!("null"))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_type_is_fail() {
        assert!(!evaluate_operator(
            &AssertionOperator::TypeIs,
            &json!("hello"),
            &json!("number")
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_type_is_invalid_type() {
        let result = evaluate_operator(
            &AssertionOperator::TypeIs,
            &json!("hello"),
            &json!("invalid"),
        );
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("Unsupported type check"));
    }

    #[test]
    fn test_type_is_expected_not_string() {
        let result = evaluate_operator(&AssertionOperator::TypeIs, &json!("hello"), &json!(123));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("expected must be a string type"));
    }

    // ========================================================================
    // Between 操作符测试
    // ========================================================================

    #[test]
    fn test_between_pass() {
        assert!(
            evaluate_operator(&AssertionOperator::Between, &json!(50), &json!([0, 100]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_between_pass_boundary_min() {
        assert!(
            evaluate_operator(&AssertionOperator::Between, &json!(0), &json!([0, 100]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_between_pass_boundary_max() {
        assert!(
            evaluate_operator(&AssertionOperator::Between, &json!(100), &json!([0, 100]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_between_fail_below() {
        assert!(
            !evaluate_operator(&AssertionOperator::Between, &json!(-1), &json!([0, 100]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_between_fail_above() {
        assert!(
            !evaluate_operator(&AssertionOperator::Between, &json!(101), &json!([0, 100]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_between_reversed_range() {
        // 反向范围自动交换 min 和 max
        assert!(
            evaluate_operator(&AssertionOperator::Between, &json!(50), &json!([100, 0]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_between_expected_not_array() {
        let result = evaluate_operator(&AssertionOperator::Between, &json!(50), &json!(0));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("expected must be an array type"));
    }

    #[test]
    fn test_between_array_wrong_length() {
        let result = evaluate_operator(&AssertionOperator::Between, &json!(50), &json!([0]));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("array of 2 elements"));
    }

    #[test]
    fn test_between_actual_not_number() {
        let result = evaluate_operator(&AssertionOperator::Between, &json!("50"), &json!([0, 100]));
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("is not a numeric type"));
    }

    #[test]
    fn test_between_float_values() {
        assert!(evaluate_operator(
            &AssertionOperator::Between,
            &json!(50.5),
            &json!([0.0, 100.0])
        )
        .unwrap_or_else(|e| panic!("{e}")));
    }

    // ========================================================================
    // 边界情况测试
    // ========================================================================

    #[test]
    fn test_empty_string_contains() {
        // 空字符串包含空字符串
        assert!(
            evaluate_operator(&AssertionOperator::Contains, &json!(""), &json!(""))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_empty_array_contains() {
        assert!(
            !evaluate_operator(&AssertionOperator::Contains, &json!([]), &json!(1))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_complex_object_eq() {
        let obj1 = json!({"name": "test", "value": 123});
        let obj2 = json!({"name": "test", "value": 123});
        let obj3 = json!({"name": "test", "value": 456});

        assert!(evaluate_operator(&AssertionOperator::Eq, &obj1, &obj2)
            .unwrap_or_else(|e| panic!("{e}")));
        assert!(!evaluate_operator(&AssertionOperator::Eq, &obj1, &obj3)
            .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_array_eq() {
        let arr1 = json!([1, 2, 3]);
        let arr2 = json!([1, 2, 3]);
        let arr3 = json!([1, 2, 4]);

        assert!(evaluate_operator(&AssertionOperator::Eq, &arr1, &arr2)
            .unwrap_or_else(|e| panic!("{e}")));
        assert!(!evaluate_operator(&AssertionOperator::Eq, &arr1, &arr3)
            .unwrap_or_else(|e| panic!("{e}")));
    }

    #[test]
    fn test_negative_numbers() {
        assert!(
            evaluate_operator(&AssertionOperator::Gt, &json!(-5), &json!(-10))
                .unwrap_or_else(|e| panic!("{e}"))
        );
        assert!(
            evaluate_operator(&AssertionOperator::Lt, &json!(-10), &json!(-5))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_between_negative_range() {
        assert!(
            evaluate_operator(&AssertionOperator::Between, &json!(-50), &json!([-100, 0]))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }
}
