//! 表达式求值
//!
//! 供 `conditional` 分支节点与 `flow:loop` 的 `until` 条件复用的 evalexpr 封装：
//! 变量池 + `last_node_output` 注入、嵌套字段扁平化（`x.y.z` → `x__y__z`）、
//! 求值结果按 truthy 规则转布尔。

use evalexpr::{ContextWithMutableVariables, HashMapContext, Value as EvalValue};
use regex::Regex;

/// 用 evalexpr 求值布尔表达式
///
/// 变量注入顺序：先 `variables`，再 `last_node_output`（后者覆盖同名键）。
/// 支持 `x.y.z` / `$.x.y.z` 的点号扁平化（`x__y__z`）。
pub fn evaluate_boolean_expression(
    expression: &str,
    variables: &serde_json::Value,
    last_node_output: Option<&serde_json::Value>,
) -> Result<bool, String> {
    log::info!("[ExpressionEval] Expression: {}", expression);

    // 创建evalexpr上下文
    let mut evalexpr_context = HashMapContext::new();

    // 注入变量池（含嵌套展开）
    inject_json_to_context(&mut evalexpr_context, variables)?;

    // 注入 last_node_output（后注入，覆盖同名键，优先级更高）
    if let Some(output) = last_node_output {
        inject_json_to_context(&mut evalexpr_context, output)?;
    }

    // 预处理表达式：将 x.y.z 转换为 x__y__z
    let processed_expression = preprocess_expression(expression)?;

    // 评估表达式
    let eval_result = evalexpr::eval_with_context(&processed_expression, &evalexpr_context)
        .map_err(|e| {
            log::error!(
                "[Error] Evaluating expression '{}' (preprocessed: '{}') failed: {}",
                expression,
                processed_expression,
                e
            );
            format!("Evaluating expression '{}' failed: {}", expression, e)
        })?;

    // 转换为布尔值
    let result = match eval_result {
        EvalValue::Boolean(b) => b,
        EvalValue::Int(i) => i != 0,
        EvalValue::Float(f) => f != 0.0,
        EvalValue::String(s) => !s.is_empty(),
        EvalValue::Empty => false,
        _ => {
            log::error!(
                "[Error] Expression returned unsupported type: {:?}",
                eval_result
            );
            return Err(format!(
                "Expression returned unsupported type: {:?}",
                eval_result
            ));
        }
    };

    log::info!("[ExpressionEval] Expression eval result: {}", result);
    Ok(result)
}

/// 将 JSON Value 转换为 evalexpr Value
fn json_to_evalexpr_value(json: &serde_json::Value) -> EvalValue {
    match json {
        serde_json::Value::Null => EvalValue::Empty,
        serde_json::Value::Bool(b) => EvalValue::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if let Some(f) = n.as_f64() {
                    if f == i as f64 {
                        return EvalValue::Int(i);
                    }
                }
            }
            if let Some(f) = n.as_f64() {
                EvalValue::Float(f)
            } else {
                EvalValue::Empty
            }
        }
        serde_json::Value::String(s) => EvalValue::String(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            EvalValue::String(json.to_string())
        }
    }
}

/// 将 JSON 对象注入到 evalexpr 上下文中（含递归嵌套展开）
///
/// - 顶层字段直接注入为变量（如 `status`）
/// - 嵌套对象递归展开为扁平路径（如 `data.stepKey` → 变量 `data__stepKey`）
/// - 数组元素按索引展开（如 `items.0.name` → 变量 `items__0__name`）
fn inject_json_to_context(
    context: &mut HashMapContext,
    json: &serde_json::Value,
) -> Result<(), String> {
    if let serde_json::Value::Object(map) = json {
        // 注入顶层字段
        for (key, value) in map {
            let eval_value = json_to_evalexpr_value(value);
            context
                .set_value(key.clone(), eval_value)
                .map_err(|e| format!("Setting variable '{}' failed: {}", key, e))?;

            // 递归展开嵌套对象
            if matches!(value, serde_json::Value::Object(_)) {
                flatten_json_to_context(value, key, context)?;
            }
            // 展开数组元素
            if let serde_json::Value::Array(arr) = value {
                for (i, item) in arr.iter().enumerate() {
                    let item_key = format!("{}__{}", key, i);
                    match item {
                        serde_json::Value::Object(_) => {
                            flatten_json_to_context(item, &item_key, context)?;
                        }
                        _ => {
                            let eval_value = json_to_evalexpr_value(item);
                            context
                                .set_value(item_key.clone(), eval_value)
                                .map_err(|e| {
                                    format!("Setting variable '{}' failed: {}", item_key, e)
                                })?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// 递归展开 JSON 嵌套对象为扁平的变量路径（使用 `__` 分隔符）
fn flatten_json_to_context(
    json: &serde_json::Value,
    prefix: &str,
    context: &mut HashMapContext,
) -> Result<(), String> {
    if let serde_json::Value::Object(map) = json {
        for (key, value) in map {
            let flat_key = format!("{}__{}", prefix, key);
            match value {
                serde_json::Value::Object(_) => {
                    flatten_json_to_context(value, &flat_key, context)?;
                    let eval_value = json_to_evalexpr_value(value);
                    context
                        .set_value(flat_key, eval_value)
                        .map_err(|e| format!("Setting variable failed: {}", e))?;
                }
                serde_json::Value::Array(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        let item_key = format!("{}__{}", flat_key, i);
                        match item {
                            serde_json::Value::Object(_) => {
                                flatten_json_to_context(item, &item_key, context)?;
                            }
                            _ => {
                                let eval_value = json_to_evalexpr_value(item);
                                context
                                    .set_value(item_key, eval_value)
                                    .map_err(|e| format!("Setting variable failed: {}", e))?;
                            }
                        }
                    }
                    let eval_value = json_to_evalexpr_value(value);
                    context
                        .set_value(flat_key, eval_value)
                        .map_err(|e| format!("Setting variable failed: {}", e))?;
                }
                _ => {
                    let eval_value = json_to_evalexpr_value(value);
                    context
                        .set_value(flat_key, eval_value)
                        .map_err(|e| format!("Setting variable failed: {}", e))?;
                }
            }
        }
    }
    Ok(())
}

/// 预处理表达式：将标识符链中的点号转换为双下划线
///
/// 自动将表达式中的 `x.y.z` 格式转换为 `x__y__z`，
/// 以便与 `flatten_json_to_context` 展开的扁平变量名对应。
/// 字符串字面量内的内容不会被修改。
pub fn preprocess_expression(expr: &str) -> Result<String, String> {
    let string_re =
        Regex::new(r#""(?:[^"\\]|\\.)*""#).map_err(|e| format!("Invalid regex: {}", e))?;

    let mut result = String::with_capacity(expr.len() * 2);
    let mut last_end = 0;

    for cap in string_re.find_iter(expr) {
        if cap.start() > last_end {
            result.push_str(&flatten_identifier_chains(&expr[last_end..cap.start()])?);
        }
        result.push_str(cap.as_str());
        last_end = cap.end();
    }

    if last_end < expr.len() {
        result.push_str(&flatten_identifier_chains(&expr[last_end..])?);
    }

    Ok(result)
}

/// 将标识符链中的点号替换为双下划线
///
/// 处理两种格式：
/// - `$.x.y.z` — 先去除 `$.` 前缀，再将剩余的点号替换为 `__`
/// - `x.y.z` — 直接将点号替换为 `__`
///
/// 不含点号的标识符（如 `status`）不会被修改。
fn flatten_identifier_chains(s: &str) -> Result<String, String> {
    // 先处理 $. 前缀的 JSONPath 风格：$.x.y → x__y
    let jsonpath_re =
        Regex::new(r"\$\.[a-zA-Z_]\w*(?:\.\w+)*").map_err(|e| format!("Invalid regex: {}", e))?;
    let intermediate = jsonpath_re.replace_all(s, |caps: &regex::Captures| {
        // 去掉 "$." 前缀后，将剩余点号替换为 "__"
        caps[0][2..].replace('.', "__")
    });

    // 再处理常规标识符链：x.y.z → x__y__z
    let re = Regex::new(r"[a-zA-Z_]\w*(?:\.\w+)+").map_err(|e| format!("Invalid regex: {}", e))?;
    Ok(re
        .replace_all(&intermediate, |caps: &regex::Captures| {
            caps[0].replace('.', "__")
        })
        .into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================================
    // 辅助函数单元测试
    // ============================================================================

    #[test]
    fn test_preprocess_expression_basic() {
        assert_eq!(
            preprocess_expression(r#"data.stepKey == "value""#).unwrap_or_else(|e| panic!("{e}")),
            r#"data__stepKey == "value""#
        );
    }

    #[test]
    fn test_preprocess_expression_deep() {
        assert_eq!(
            preprocess_expression("data.result.title == \"test\"")
                .unwrap_or_else(|e| panic!("{e}")),
            "data__result__title == \"test\""
        );
    }

    #[test]
    fn test_preprocess_expression_no_dots() {
        assert_eq!(
            preprocess_expression("status == 200").unwrap_or_else(|e| panic!("{e}")),
            "status == 200"
        );
    }

    #[test]
    fn test_preprocess_expression_preserves_strings() {
        // 字符串字面量内的点号不应被替换
        assert_eq!(
            preprocess_expression(r#"url == "http://example.com""#)
                .unwrap_or_else(|e| panic!("{e}")),
            r#"url == "http://example.com""#
        );
    }

    #[test]
    fn test_flatten_identifier_chains() {
        assert_eq!(
            flatten_identifier_chains("data.stepKey == 200").unwrap_or_else(|e| panic!("{e}")),
            "data__stepKey == 200"
        );
        assert_eq!(
            flatten_identifier_chains("code == 200").unwrap_or_else(|e| panic!("{e}")),
            "code == 200"
        );
        assert_eq!(
            flatten_identifier_chains("a.b.c.d > 0").unwrap_or_else(|e| panic!("{e}")),
            "a__b__c__d > 0"
        );
    }

    #[test]
    fn test_flatten_jsonpath_prefix() {
        // $.status → status（去掉 $. 后无点号）
        assert_eq!(
            flatten_identifier_chains("$.status == 200").unwrap_or_else(|e| panic!("{e}")),
            "status == 200"
        );
        // $.body.data.token → body__data__token
        assert_eq!(
            flatten_identifier_chains(r#"$.body.data.token == "abc""#)
                .unwrap_or_else(|e| panic!("{e}")),
            r#"body__data__token == "abc""#
        );
        // 混合 $. 和普通标识符
        assert_eq!(
            flatten_identifier_chains("$.status >= 200 && $.status < 300")
                .unwrap_or_else(|e| panic!("{e}")),
            "status >= 200 && status < 300"
        );
    }

    #[test]
    fn test_preprocess_expression_jsonpath_style() {
        // $.status 在表达式中应被正确处理
        assert_eq!(
            preprocess_expression("$.status == 200").unwrap_or_else(|e| panic!("{e}")),
            "status == 200"
        );
        assert_eq!(
            preprocess_expression(r#"$.body.data.token == "abc""#)
                .unwrap_or_else(|e| panic!("{e}")),
            r#"body__data__token == "abc""#
        );
        // 混合使用
        assert_eq!(
            preprocess_expression("status == 200 && $.body.success == true")
                .unwrap_or_else(|e| panic!("{e}")),
            "status == 200 && body__success == true"
        );
    }

    #[test]
    fn test_evaluate_boolean_expression_truthy() {
        let vars = json!({"status": 200, "body": {"ok": true}});
        assert!(evaluate_boolean_expression("status == 200", &vars, None)
            .unwrap_or_else(|e| panic!("{e}")));
        assert!(
            evaluate_boolean_expression("$.body.ok == true", &vars, None)
                .unwrap_or_else(|e| panic!("{e}"))
        );
        // last_node_output 覆盖同名键
        let last = json!({"status": 500});
        assert!(
            !evaluate_boolean_expression("status == 200", &vars, Some(&last))
                .unwrap_or_else(|e| panic!("{e}"))
        );
    }

    #[test]
    fn test_evaluate_boolean_expression_error_message_keeps_source() {
        let err = evaluate_boolean_expression("status ==", &json!({}), None)
            .err()
            .unwrap_or_else(|| panic!("expected error"));
        assert!(err.contains("status =="), "unexpected message: {err}");
    }
}
