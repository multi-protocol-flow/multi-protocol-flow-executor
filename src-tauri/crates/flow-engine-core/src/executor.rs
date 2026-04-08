// 执行器 trait 定义和变量替换工具函数
//
// NodeExecutor trait 和 replace_variables 函数从 flow-engine 迁移至此，
// 使 flow-engine-http 等子 crate 可以依赖 flow-engine-core 实现节点执行器，
// 避免循环依赖。

use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::node::Node;
use async_trait::async_trait;
use std::collections::HashMap;

/// 替换模板中的变量
///
/// # 参数
/// - `template`: 包含变量占位符的模板字符串，使用 `{{variable_path}}` 语法
/// - `context`: 执行上下文，包含变量数据
///
/// # 返回
/// - `Result<String, String>`: 替换后的字符串，或错误信息
///
/// # 功能
/// - 支持简单的变量替换，如 `{{initial_variables.api_key}}`
/// - 支持嵌套路径访问，如 `{{user.profile.name}}`
/// - 严格模式：变量不存在时返回错误
/// - 不支持索引访问（如 `items[0]`）和函数调用
pub fn replace_variables(template: &str, context: &ExecutionContext) -> Result<String, String> {
    let re = regex::Regex::new(r"\{\{([^}]+)\}\}").map_err(|e| format!("正则表达式错误: {}", e))?;
    let mut result = template.to_string();

    for cap in re.captures_iter(template) {
        let full_match = cap.get(0).unwrap().as_str();
        let variable_path = cap.get(1).unwrap().as_str().trim();

        // 检查是否包含索引访问（不支持）
        if variable_path.contains('[') {
            return Err(format!("不支持索引访问: {}", variable_path));
        }

        // 检查是否包含函数调用（不支持）
        if variable_path.contains('(') {
            return Err(format!("不支持函数调用: {}", variable_path));
        }

        // 按点分割路径
        let path_parts: Vec<&str> = variable_path.split('.').collect();

        // 从合并后的变量中获取值
        let merged_vars = context.variables.get_merged_variables();
        let mut current_value: Option<&serde_json::Value> = merged_vars.get(path_parts[0]);
        let mut current_path = String::new();

        if current_value.is_none() {
            return Err(format!("未找到变量: {}", variable_path));
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
                        return Err(format!("未找到变量: {}", current_path));
                    }
                }
                _ => {
                    return Err(format!("变量路径 '{}' 不是对象类型", current_path));
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
                    "变量 '{}' 不支持直接替换（复杂类型）",
                    variable_path
                ));
            }
            None => {
                return Err(format!("变量 '{}' 为空", variable_path));
            }
        };

        result = result.replace(full_match, &replacement);
    }

    Ok(result)
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
    /// - `Ok((next_node_ids, output_data))`: 执行成功
    ///   - `next_node_ids`: 下一个要执行的节点 UUID 列表（空则终止当前分支）
    ///   - `output_data`: 节点输出数据
    /// - `Err(msg)`: 执行失败
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String>;
}
