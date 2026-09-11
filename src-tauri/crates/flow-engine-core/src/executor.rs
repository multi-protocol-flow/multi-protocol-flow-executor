// 执行器 trait 定义和变量替换工具函数
//
// NodeExecutor trait 和 replace_variables 函数从 flow-engine 迁移至此，
// 使 flow-engine-http 等子 crate 可以依赖 flow-engine-core 实现节点执行器，
// 避免循环依赖。

use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::error::ErrorDetail;
use crate::node::Node;
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::LazyLock;

#[allow(clippy::expect_used)]
static VARIABLE_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\{\{([^}]+)\}\}").expect("valid regex pattern"));
/// 替换模板中的变量
///
/// # 参数
/// - `template`: 包含变量占位符的模板字符串，使用 `{{variable_path}}` 语法
/// - `context`: 执行上下文，包含变量数据
///
/// # 返回
/// - `Result<String, String>`: 替换后的字符串，或Error信息
///
/// # 功能
/// - 支持简单的变量替换，如 `{{api_key}}`
/// - 支持嵌套路径访问，如 `{{user.profile.name}}`
/// - 严格模式：变量不存在时返回Error
/// - 不支持索引访问（如 `items[0]`）和函数调用
pub fn replace_variables(template: &str, context: &ExecutionContext) -> Result<String, String> {
    // 快速旁路：无变量占位符直接返回，避免正则扫描与额外分配
    if !template.contains("{{") {
        return Ok(template.to_string());
    }

    let mut result = template.to_string();

    for cap in VARIABLE_REGEX.captures_iter(template) {
        let full_match = cap.get(0).ok_or("Regex capture 0 missing")?.as_str();
        let variable_path = cap.get(1).ok_or("Regex capture 1 missing")?.as_str().trim();

        // 检查是否包含索引访问（不支持）
        if variable_path.contains('[') {
            return Err(format!("Index access not supported: {}", variable_path));
        }

        // 检查是否包含函数调用（不支持）
        if variable_path.contains('(') {
            return Err(format!("Function calls not supported: {}", variable_path));
        }

        // 按点分割路径
        let path_parts: Vec<&str> = variable_path.split('.').collect();

        // 直接借用查找，避免全量 HashMap 克隆
        let mut current_value: Option<&serde_json::Value> =
            context.variables.get_variable(path_parts[0]);
        let mut current_path = String::new();

        if current_value.is_none() {
            if variable_path.starts_with('$') {
                if let Some(dyn_val) =
                    crate::dynamic_variable::resolve_dynamic_variable(variable_path)
                {
                    result = result.replacen(full_match, &dyn_val, 1);
                    continue;
                } else {
                    return Err(format!("Unknown dynamic variable: {}", variable_path));
                }
            }
            return Err(format!("Variable not found: {}", variable_path));
        }

        for part in &path_parts[1..] {
            if !current_path.is_empty() {
                current_path.push('.');
            }
            current_path.push_str(part);

            match current_value {
                Some(serde_json::Value::Object(map)) => {
                    if let Some(value) = map.get(*part) {
                        current_value = Some(value);
                    } else {
                        return Err(format!("Variable not found: {}", current_path));
                    }
                }
                _ => {
                    return Err(format!(
                        "Variable path '{}' is not an object type",
                        current_path
                    ));
                }
            }
        }

        // 将值转换为字符串
        let replacement = match current_value {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Bool(b)) => b.to_string(),
            Some(serde_json::Value::Null) => "null".to_string(),
            Some(serde_json::Value::Array(_) | serde_json::Value::Object(_)) => {
                return Err(format!(
                    "Variable '{}' does not support direct replacement (complex type)",
                    variable_path
                ));
            }
            None => {
                return Err(format!("Variable '{}' is null/empty", variable_path));
            }
        };

        result = result.replacen(full_match, &replacement, 1);
    }

    Ok(result)
}

/// 递归解析 JSON 值中的所有变量占位符
///
/// 遍历 JSON 树，对每个字符串值尝试进行变量替换：
/// - 字符串值包含 `{{path}}` 的会被替换为实际变量值
/// - 如果变量不存在，保留原始值（不报错）
/// - 对象/数组/数值/布尔值原样保留
pub fn resolve_variables_in_json(
    value: &serde_json::Value,
    context: &ExecutionContext,
) -> serde_json::Value {
    match value {
        Value::String(s) => {
            if s.contains("{{") {
                match replace_variables(s, context) {
                    Ok(resolved) => Value::String(resolved),
                    Err(_) => value.clone(),
                }
            } else {
                value.clone()
            }
        }
        Value::Object(map) => {
            let resolved: Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), resolve_variables_in_json(v, context)))
                .collect();
            Value::Object(resolved)
        }
        Value::Array(arr) => {
            let resolved: Vec<Value> = arr
                .iter()
                .map(|v| resolve_variables_in_json(v, context))
                .collect();
            Value::Array(resolved)
        }
        other => other.clone(),
    }
}

/// 节点执行器 trait
///
/// 定义了所有节点执行器必须实现的接口。
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// 执行节点
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文（可变借用，允许执行器修改变量等）
    /// - `connections`: 流程的所有连接
    /// - `node_map`: 节点 UUID 到节点的映射
    ///
    /// # 返回
    /// - `Ok((next_node_ids, output_data))`: 执行Success
    ///   - `next_node_ids`: 下一个要执行的节点 UUID 列表（空则, terminating current branch）
    ///   - `output_data`: 节点输出数据
    /// - `Err(error_detail)`: 执行Failed，包含结构化Error信息
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_replace_variables_fast_path() {
        let context = ExecutionContext::new("flow-1".to_string(), 0);
        let plain = "Hello world without variables";
        let res = replace_variables(plain, &context).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(res, plain);
    }

    #[test]
    fn test_replace_variables_borrowed_lookup() {
        let mut context = ExecutionContext::new("flow-1".to_string(), 0);
        context.variables.set_variable(
            "user".to_string(),
            json!({"profile": {"name": "Alice", "age": 30}}),
            "node-1".to_string(),
        );
        context
            .variables
            .set_variable("active".to_string(), json!(true), "node-1".to_string());

        let template =
            "User {{user.profile.name}} is {{user.profile.age}} years old, active: {{active}}";
        let res = replace_variables(template, &context).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(res, "User Alice is 30 years old, active: true");
    }

    #[test]
    fn test_replace_variables_error_compat() {
        let mut context = ExecutionContext::new("flow-1".to_string(), 0);
        context.variables.set_variable(
            "user".to_string(),
            json!({"profile": "not an object"}),
            "node-1".to_string(),
        );

        let err1 = replace_variables("{{missing}}", &context)
            .err()
            .unwrap_or_else(|| panic!("expected Err"));
        assert_eq!(err1, "Variable not found: missing");

        let err2 = replace_variables("{{user.missing}}", &context)
            .err()
            .unwrap_or_else(|| panic!("expected Err"));
        assert_eq!(err2, "Variable not found: missing");
        let err3 = replace_variables("{{user.profile.missing}}", &context)
            .err()
            .unwrap_or_else(|| panic!("expected Err"));
        assert_eq!(
            err3,
            "Variable path 'profile.missing' is not an object type"
        );

        let err4 = replace_variables("{{user[0]}}", &context)
            .err()
            .unwrap_or_else(|| panic!("expected Err"));
        assert_eq!(err4, "Index access not supported: user[0]");

        let err5 = replace_variables("{{fn()}}", &context)
            .err()
            .unwrap_or_else(|| panic!("expected Err"));
        assert_eq!(err5, "Function calls not supported: fn()");
    }

    #[test]
    fn test_replace_variables_dynamic_guid() {
        let context = ExecutionContext::new("flow-1".to_string(), 0);
        let template = "Bearer {{$guid}}";
        let res = replace_variables(template, &context).unwrap_or_else(|e| panic!("{e}"));
        assert!(res.starts_with("Bearer "));
        let guid = &res["Bearer ".len()..];
        assert_eq!(guid.len(), 36);
        assert!(uuid::Uuid::parse_str(guid).is_ok());
    }

    #[test]
    fn test_replace_variables_dynamic_timestamp() {
        let context = ExecutionContext::new("flow-1".to_string(), 0);
        let template = "timestamp={{$timestamp}}";
        let res = replace_variables(template, &context).unwrap_or_else(|e| panic!("{e}"));
        assert!(res.starts_with("timestamp="));
        let ts_str = &res["timestamp=".len()..];
        let ts: i64 = ts_str.parse().unwrap_or_else(|e| panic!("{e}"));
        assert!(ts > 1700000000);
    }

    #[test]
    fn test_replace_variables_dynamic_unknown() {
        let context = ExecutionContext::new("flow-1".to_string(), 0);
        let err = replace_variables("{{$notExist}}", &context)
            .err()
            .unwrap_or_else(|| panic!("expected Err"));
        assert_eq!(err, "Unknown dynamic variable: $notExist");
    }

    #[test]
    fn test_replace_variables_user_override_dynamic() {
        let mut context = ExecutionContext::new("flow-1".to_string(), 0);
        context.variables.set_variable(
            "$guid".to_string(),
            json!("custom-override-id"),
            "node-1".to_string(),
        );
        let res = replace_variables("id={{$guid}}", &context).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(res, "id=custom-override-id");
    }

    #[test]
    fn test_replace_variables_multiple_dynamic_distinct() {
        let context = ExecutionContext::new("flow-1".to_string(), 0);
        let template = "{{$guid}}|{{$guid}}";
        let res = replace_variables(template, &context).unwrap_or_else(|e| panic!("{e}"));
        let parts: Vec<&str> = res.split('|').collect();
        assert_eq!(parts.len(), 2);
        assert_ne!(
            parts[0], parts[1],
            "Multiple dynamic evaluations should generate distinct values"
        );
    }

    #[test]
    fn test_resolve_variables_in_json_dynamic() {
        let context = ExecutionContext::new("flow-1".to_string(), 0);
        let json_data = json!({
            "id": "{{$guid}}",
            "time": "{{$timestamp}}",
            "nested": {
                "email": "{{$randomEmail}}",
                "static": 123
            }
        });
        let resolved = resolve_variables_in_json(&json_data, &context);
        let id_str = resolved["id"].as_str().unwrap_or("");
        assert_eq!(id_str.len(), 36);
        assert!(uuid::Uuid::parse_str(id_str).is_ok());
        let email_str = resolved["nested"]["email"].as_str().unwrap_or("");
        assert!(email_str.contains('@'));
        assert_eq!(resolved["nested"]["static"], 123);
    }
}
