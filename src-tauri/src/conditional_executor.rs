use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::error::{ErrorDetail, ErrorKind};
use crate::executor::NodeExecutor;
use crate::log_event::{LogEvent, LogLevel};
use crate::node::Node;
use crate::node_config::{ConditionOperator, ConditionType, ExpectedValueType, NodeData};
use evalexpr::ContextWithMutableVariables;
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

/// 条件节点执行器
pub struct ConditionalNodeExecutor;

impl Default for ConditionalNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalNodeExecutor {
    /// 创建新的条件节点执行器
    pub fn new() -> Self {
        Self
    }

    /// 根据类型解析期望值
    ///
    /// # 参数
    /// - `value`: 期望值（JSON Value）
    /// - `value_type`: 期望值类型
    ///
    /// # 返回
    /// - `Ok(Value)`: 解析后的 JSON 值
    /// - `Err(String)`: 解析失败
    fn parse_expected_value(
        value: &serde_json::Value,
        value_type: Option<&ExpectedValueType>,
    ) -> Result<serde_json::Value, String> {
        // 如果没有指定类型，直接返回原值
        let value_type = match value_type {
            Some(vt) => vt,
            None => return Ok(value.clone()),
        };

        // 将值转换为字符串进行解析
        let value_str = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            _ => return Ok(value.clone()), // 复杂类型直接返回
        };

        match value_type {
            ExpectedValueType::Number => {
                value_str
                    .parse::<f64>()
                    .map(|n| {
                        if n.fract() == 0.0 {
                            serde_json::Value::Number(serde_json::Number::from(n as i64))
                        } else {
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(n)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            )
                        }
                    })
                    .map_err(|e| format!("无法解析为数字: {}", e))
            }
            ExpectedValueType::Boolean => match value_str.to_lowercase().as_str() {
                "true" => Ok(serde_json::Value::Bool(true)),
                "false" => Ok(serde_json::Value::Bool(false)),
                _ => Err(format!("无法解析为布尔值: {}", value_str)),
            },
            ExpectedValueType::Null => {
                if value_str.is_empty() || value_str.to_lowercase() == "null" {
                    Ok(serde_json::Value::Null)
                } else {
                    Err(format!("期望 null 但得到: {}", value_str))
                }
            }
            ExpectedValueType::String => {
                // 字符串类型：直接使用字符串值
                Ok(serde_json::Value::String(value_str))
            }
        }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ConditionalNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 执行前日志
        let start_time = Instant::now();
        log::info!("[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: conditional", context.flow_uuid, node.uuid, node.name);

        // 发送日志：条件节点开始评估
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("条件节点 {} 开始评估", node.name),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        // 从 Node 的 data 字段提取 ConditionalConfig
        let conditional_config = match &node.data {
            NodeData::Conditional(config) => config.clone(),
            _ => {
                let error_msg = format!("节点数据类型不是ConditionalConfig: {}", node.uuid);
                log::error!("[流程 {}] [错误] 节点UUID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                return Err(error_msg);
            }
        };

        // 判断条件类型并评估
        let condition_result = match conditional_config.condition_type {
            ConditionType::Simple => {
                // 发送日志：条件类型
                context.emit_log(LogEvent {
                    level: LogLevel::Debug,
                    message: "条件类型: Simple".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                // 简单条件：评估 field_path + operator + expected_value
                let expression = format!(
                    "{:?} {:?} {:?}",
                    conditional_config.field_path,
                    conditional_config.operator,
                    conditional_config.expected_value
                );
                let result = self.evaluate_simple_condition(
                    &conditional_config,
                    &json!(context.variables.get_merged_variables()),
                )?;
                log::info!("[流程 {}] [条件判断] 条件类型: Simple, 表达式: {}, 结果: {}", context.flow_uuid, expression, result);

                // 发送日志：表达式详情
                context.emit_log(LogEvent {
                    level: LogLevel::Debug,
                    message: format!("表达式: {} = {}", expression, result),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                result
            }
            ConditionType::Advanced => {
                // 发送日志：条件类型
                context.emit_log(LogEvent {
                    level: LogLevel::Debug,
                    message: "条件类型: Advanced".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                // 高级条件：评估 advanced_expression
                let expression = conditional_config.advanced_expression.as_ref().unwrap_or(&"".to_string()).clone();
                let result = self.evaluate_advanced_condition(
                    &conditional_config,
                    &json!(context.variables.get_merged_variables()),
                )?;
                log::info!("[流程 {}] [条件判断] 条件类型: Advanced, 表达式: {}, 结果: {}", context.flow_uuid, expression, result);

                // 发送日志：表达式详情
                context.emit_log(LogEvent {
                    level: LogLevel::Debug,
                    message: format!("表达式: {} = {}", expression, result),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;

                result
            }
        };

        // 发送日志：条件结果
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("条件结果: {}", condition_result),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;
        let duration_ms = start_time.elapsed().as_millis();

        // 确定目标端口（true 或 false）
        let target_port_id = if condition_result { "true" } else { "false" };

        // 根据条件结果查找连接到目标端口的节点
        let next_node_ids = connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == target_port_id)
            .map(|conn| conn.target_node_uuid.clone())
            .collect::<Vec<_>>();

        // 执行后日志
        log::info!(
            "[流程 {}] [完成] 节点UUID: {}, 耗时: {}ms, 状态: success, 使用的端口: {}",
            context.flow_uuid,
            node.uuid,
            duration_ms,
            target_port_id
        );

        // 返回下一个节点 UUID 列表和布尔值作为输出数据
        // 空列表表示该分支无连接，流程终止
        Ok((next_node_ids, Some(json!(condition_result))))
    }
}

impl ConditionalNodeExecutor {
    /// 评估简单条件
    ///
    /// # 参数
    /// - `config`: 条件节点配置
    /// - `variables`: 变量数据
    ///
    /// # 返回
    /// - `Ok(bool)`: 条件评估结果
    /// - `Err(String)`: 条件评估失败（使用 ConditionSyntaxError 或 ConditionEvaluationError）
    fn evaluate_simple_condition(
        &self,
        config: &crate::node_config::ConditionalConfig,
        variables: &serde_json::Value,
    ) -> Result<bool, String> {
        // 获取 field_path
        let field_path = config.field_path.as_ref().ok_or_else(|| {
            let error = ErrorDetail::new(
                ErrorKind::ConditionSyntax,
                "简单条件缺少 field_path".to_string(),
            );
            log::error!("[错误] {}", error.message);
            error.message
        })?;

        // 获取 operator
        let operator = config.operator.as_ref().ok_or_else(|| {
            let error = ErrorDetail::new(
                ErrorKind::ConditionSyntax,
                "简单条件缺少 operator".to_string(),
            );
            log::error!("[错误] {}", error.message);
            error.message
        })?;

        log::info!("[条件评估] 字段路径: {}, 操作符: {:?}", field_path, operator);

        // 从 variables 中提取 field_path 指定的值
        let actual_value = self.extract_field_value(variables, field_path)?;

        log::info!("[条件评估] 提取的值: {:?}", actual_value);

        // 获取 expected_value（对于 exists 操作符不需要 expected_value）
        let expected_value_raw = config.expected_value.as_ref();

        // 解析 expected_value_type 并应用类型解析
        let expected_value_type = config
            .expected_value_type
            .as_ref()
            .and_then(|v| serde_json::from_value::<ExpectedValueType>(v.clone()).ok());

        let expected_value = match (expected_value_raw, &expected_value_type) {
            (Some(raw), Some(_)) => {
                // 有类型指定，使用类型解析
                Some(Self::parse_expected_value(raw, expected_value_type.as_ref())?)
            }
            (Some(raw), None) => Some(raw.clone()),
            (None, _) => None,
        };

        log::info!("[条件评估] 期望值类型: {:?}, 解析后期望值: {:?}", expected_value_type, expected_value);

        // 使用 operator 比较提取值和 expected_value
        let result = self.compare_values(operator, &actual_value, expected_value.as_ref())?;

        log::info!("[条件评估] 比较结果: {}", result);
        Ok(result)
    }

    /// 评估高级条件
    fn evaluate_advanced_condition(
        &self,
        config: &crate::node_config::ConditionalConfig,
        variables: &serde_json::Value,
    ) -> Result<bool, String> {
        // 获取advanced_expression
        let expression = config.advanced_expression.as_ref().ok_or_else(|| {
            log::error!("[错误] 高级条件缺少advanced_expression");
            "高级条件缺少advanced_expression".to_string()
        })?;

        log::info!("[条件评估] 高级表达式: {}", expression);

        // 创建evalexpr上下文
        let mut evalexpr_context = evalexpr::HashMapContext::new();

        // 将variables中的变量添加到evalexpr上下文中
        if let serde_json::Value::Object(map) = variables {
            for (key, value) in map {
                let evalexpr_value = match value {
                    serde_json::Value::String(s) => evalexpr::Value::String(s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            evalexpr::Value::Int(i)
                        } else if let Some(f) = n.as_f64() {
                            evalexpr::Value::Float(f)
                        } else {
                            evalexpr::Value::Float(0.0)
                        }
                    }
                    serde_json::Value::Bool(b) => evalexpr::Value::Boolean(*b),
                    serde_json::Value::Null => evalexpr::Value::Empty,
                    _ => {
                        log::warn!("不支持的变量类型 '{}': {:?}", key, value);
                        evalexpr::Value::Empty
                    }
                };
                evalexpr_context
                    .set_value(key.clone(), evalexpr_value)
                    .map_err(|e| {
                        log::error!("设置变量 '{}' 到上下文失败: {}", key, e);
                        format!("设置变量 '{}' 到上下文失败: {}", key, e)
                    })?;
            }
        }

        // 评估表达式
        let eval_result = evalexpr::eval_with_context(expression, &evalexpr_context).map_err(|e| {
            log::error!("[错误] 评估表达式 '{}' 失败: {}", expression, e);
            format!("评估表达式 '{}' 失败: {}", expression, e)
        })?;

        // 转换为布尔值
        let result = match eval_result {
            evalexpr::Value::Boolean(b) => b,
            evalexpr::Value::Int(i) => i != 0,
            evalexpr::Value::Float(f) => f != 0.0,
            evalexpr::Value::String(s) => !s.is_empty(),
            evalexpr::Value::Empty => false,
            _ => {
                log::error!("[错误] 表达式返回不支持的类型: {:?}", eval_result);
                return Err(format!("表达式返回不支持的类型: {:?}", eval_result));
            }
        };

        log::info!("[条件评估] 表达式评估结果: {}", result);
        Ok(result)
    }

    /// 从JSON中提取字段值（支持点号分隔的路径）
    fn extract_field_value(
        &self,
        data: &serde_json::Value,
        field_path: &str,
    ) -> Result<serde_json::Value, String> {
        let path_parts: Vec<&str> = field_path.split('.').collect();
        let mut current_value = data;
        let mut current_path = String::new();

        for part in path_parts.iter() {
            if !current_path.is_empty() {
                current_path.push('.');
            }
            current_path.push_str(part);

            match current_value {
                serde_json::Value::Object(map) => {
                    if let Some(value) = map.get(*part) {
                        current_value = value;
                    } else {
                        log::error!("[错误] 字段 '{}' 不存在", current_path);
                        return Err(format!("字段 '{}' 不存在", current_path));
                    }
                }
                _ => {
                    log::error!("[错误] 字段 '{}' 不是对象类型", current_path);
                    return Err(format!("字段 '{}' 不是对象类型", current_path));
                }
            }
        }

        Ok(current_value.clone())
    }

    /// 比较两个值
    fn compare_values(
        &self,
        operator: &ConditionOperator,
        actual: &serde_json::Value,
        expected: Option<&serde_json::Value>,
    ) -> Result<bool, String> {
        match operator {
            ConditionOperator::Equal => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("[错误] equal操作符需要expected_value");
                    "equal操作符需要expected_value".to_string()
                })?;
                let result = actual == expected_val;
                Ok(result)
            }
            ConditionOperator::NotEqual => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("[错误] not_equal操作符需要expected_value");
                    "not_equal操作符需要expected_value".to_string()
                })?;
                let result = actual != expected_val;
                Ok(result)
            }
            ConditionOperator::Greater => {
                self.compare_numeric(actual, expected, |a, b| a > b, "greater")
            }
            ConditionOperator::GreaterOrEqual => {
                self.compare_numeric(actual, expected, |a, b| a >= b, "greater_or_equal")
            }
            ConditionOperator::Less => {
                self.compare_numeric(actual, expected, |a, b| a < b, "less")
            }
            ConditionOperator::LessOrEqual => {
                self.compare_numeric(actual, expected, |a, b| a <= b, "less_or_equal")
            }
            ConditionOperator::Contains => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("[错误] contains操作符需要expected_value");
                    "contains操作符需要expected_value".to_string()
                })?;
                self.compare_contains(actual, expected_val)
            }
            ConditionOperator::StartsWith => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("[错误] starts_with操作符需要expected_value");
                    "starts_with操作符需要expected_value".to_string()
                })?;
                self.compare_starts_with(actual, expected_val)
            }
            ConditionOperator::EndsWith => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("[错误] ends_with操作符需要expected_value");
                    "ends_with操作符需要expected_value".to_string()
                })?;
                self.compare_ends_with(actual, expected_val)
            }
            ConditionOperator::Exists => {
                // exists操作符只需要检查字段是否存在，不需要expected_value
                let result = !matches!(actual, serde_json::Value::Null);
                Ok(result)
            }
        }
    }

    /// 比较数值
    fn compare_numeric<F>(
        &self,
        actual: &serde_json::Value,
        expected: Option<&serde_json::Value>,
        compare: F,
        operator_name: &str,
    ) -> Result<bool, String>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let actual_num = self.value_to_number(actual, "actual")?;
        let expected_val = expected.ok_or_else(|| {
            log::error!("[错误] {}操作符需要expected_value", operator_name);
            format!("{}操作符需要expected_value", operator_name)
        })?;
        let expected_num = self.value_to_number(expected_val, "expected")?;
        let result = compare(actual_num, expected_num);
        Ok(result)
    }

    /// 将JSON值转换为数值
    fn value_to_number(&self, value: &serde_json::Value, name: &str) -> Result<f64, String> {
        match value {
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Ok(f)
                } else {
                    Err(format!("{}不是有效的数字: {:?}", name, value))
                }
            }
            _ => Err(format!("{}不是数字类型: {:?}", name, value)),
        }
    }

    /// 比较字符串包含关系
    fn compare_contains(
        &self,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<bool, String> {
        let actual_str = self.value_to_string(actual, "actual")?;
        let expected_str = self.value_to_string(expected, "expected")?;
        let result = actual_str.contains(&expected_str);
        Ok(result)
    }

    /// 比较字符串前缀
    fn compare_starts_with(
        &self,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<bool, String> {
        let actual_str = self.value_to_string(actual, "actual")?;
        let expected_str = self.value_to_string(expected, "expected")?;
        let result = actual_str.starts_with(&expected_str);
        Ok(result)
    }

    /// 比较字符串后缀
    fn compare_ends_with(
        &self,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<bool, String> {
        let actual_str = self.value_to_string(actual, "actual")?;
        let expected_str = self.value_to_string(expected, "expected")?;
        let result = actual_str.ends_with(&expected_str);
        Ok(result)
    }

    /// 将JSON值转换为字符串
    fn value_to_string(&self, value: &serde_json::Value, name: &str) -> Result<String, String> {
        match value {
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Number(n) => Ok(n.to_string()),
            serde_json::Value::Bool(b) => Ok(b.to_string()),
            _ => Err(format!("{}不是字符串类型: {:?}", name, value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::context::{ExecutionContext, VariableStorage};
    use crate::node::{Node, NodeType, NodePosition};
    use crate::node_config::NodeData;
    use crate::node_config::{ConditionalConfig, ConditionOperator, ConditionType, ExpectedValueType};
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_node(config: ConditionalConfig) -> Node {
        Node {
            uuid: "test-node-id".to_string(),
            name: "Test Condition Node".to_string(),
            node_type: NodeType::Conditional,
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Conditional(config),
            ports: None,
        }
    }

    fn create_test_context(variables: serde_json::Value) -> ExecutionContext {
        let mut storage = VariableStorage::new();
        if let Some(obj) = variables.as_object() {
            for (key, value) in obj {
                storage.set_global(key.clone(), value.clone());
            }
        }

        ExecutionContext {
            flow_uuid: "test-flow-id".to_string(),
            current_node_uuid: None,
            variables: storage,
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
        }
    }

    #[tokio::test]
    async fn test_simple_condition_equal() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!(25)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_not_equal() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::NotEqual),
            expected_value: Some(serde_json::json!(30)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_greater() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::Greater),
            expected_value: Some(serde_json::json!(20)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_greater_or_equal() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::GreaterOrEqual),
            expected_value: Some(serde_json::json!(25)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_less() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::Less),
            expected_value: Some(serde_json::json!(30)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_less_or_equal() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::LessOrEqual),
            expected_value: Some(serde_json::json!(25)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_contains() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.name".to_string()),
            operator: Some(ConditionOperator::Contains),
            expected_value: Some(serde_json::json!("John")),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "name": "John Doe"
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_starts_with() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.name".to_string()),
            operator: Some(ConditionOperator::StartsWith),
            expected_value: Some(serde_json::json!("John")),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "name": "John Doe"
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_ends_with() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.name".to_string()),
            operator: Some(ConditionOperator::EndsWith),
            expected_value: Some(serde_json::json!("Doe")),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "name": "John Doe"
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_exists() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.email".to_string()),
            operator: Some(ConditionOperator::Exists),
            expected_value: None,
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "email": "test@example.com"
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_simple_condition_false() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!(30)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为false应该返回Ok（不再返回错误）");
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
        assert_eq!(output_data.unwrap(), json!(false));
    }

    #[tokio::test]
    async fn test_advanced_condition() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("age > 20 && name == \"John\"".to_string()),
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25,
            "name": "John"
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_advanced_condition_or() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("age > 30 || name == \"John\"".to_string()),
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25,
            "name": "John"
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该返回Ok（继续执行）");
    }

    #[tokio::test]
    async fn test_advanced_condition_false() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("age > 30 && name == \"Jane\"".to_string()),
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25,
            "name": "John"
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为false应该返回Ok（不再返回错误）");
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
        assert_eq!(output_data.unwrap(), json!(false));
    }

    #[tokio::test]
    async fn test_condition_true_with_connection() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("status".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!("active")),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "status": "active"
        }));

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-node-id".to_string(),
            target_node_uuid: "next-node-id".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "in".to_string(),
        }];

        let result = executor.execute(&node, &mut context, &connections, &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该成功");
        let (next_node_ids, output_data) = result.unwrap();
        assert_eq!(next_node_ids, vec!["next-node-id".to_string()]);
        assert_eq!(output_data.unwrap(), json!(true));
    }

    #[tokio::test]
    async fn test_simple_condition_empty_string() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.name".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!("")),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "name": ""
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "空字符串相等应该返回Ok");
    }

    #[tokio::test]
    async fn test_simple_condition_null_value() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.email".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::Value::Null),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "email": null
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "null值相等应该返回Ok");
    }

    #[tokio::test]
    async fn test_simple_condition_exists_with_null() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.email".to_string()),
            operator: Some(ConditionOperator::Exists),
            expected_value: None,
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "email": null
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "条件为false应该返回Ok（不再返回错误）");
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
        assert_eq!(output_data.unwrap(), json!(false));
    }

    #[tokio::test]
    async fn test_simple_condition_type_mismatch() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("user.age".to_string()),
            operator: Some(ConditionOperator::Greater), // 使用Greater来触发类型检查
            expected_value: Some(serde_json::json!(20)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "user": {
                "age": "twenty-five"  // 字符串而非数字
            }
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err(), "类型不匹配应该返回错误");
        assert!(result.unwrap_err().contains("actual不是数字类型"), "错误信息应该包含 actual不是数字类型");
    }

    #[tokio::test]
    async fn test_advanced_condition_syntax_error() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("age > 20 && name == \"\"".to_string()), // 语法错误
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25,
            "name": "John"
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err(), "语法错误应该返回错误");
        assert!(result.unwrap_err().contains("评估表达式"), "错误信息应该包含 评估表达式");
    }

    #[tokio::test]
    async fn test_simple_condition_missing_field_path() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: None, // 缺少field_path
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!(25)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err(), "缺少field_path应该返回错误");
        assert!(result.unwrap_err().contains("简单条件缺少 field_path"), "错误信息应该包含 简单条件缺少 field_path");
    }

    #[tokio::test]
    async fn test_simple_condition_missing_operator() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("age".to_string()),
            operator: None, // 缺少operator
            expected_value: Some(serde_json::json!(25)),
            advanced_expression: None,
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err(), "缺少operator应该返回错误");
        assert!(result.unwrap_err().contains("简单条件缺少 operator"), "错误信息应该包含 简单条件缺少 operator");
    }

    #[tokio::test]
    async fn test_advanced_condition_missing_expression() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: None, // 缺少advanced_expression
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_err(), "缺少advanced_expression应该返回错误");
        assert!(result.unwrap_err().contains("缺少advanced_expression"), "错误信息应该包含 缺少advanced_expression");
    }

    #[tokio::test]
    async fn test_advanced_condition_not_operator() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("!(age > 30)".to_string()),
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "非操作符应该正确工作");
    }

    #[tokio::test]
    async fn test_advanced_condition_variable_usage() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("status == \"active\" && count > 0".to_string()),
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "status": "active",
            "count": 5
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "变量使用应该正确工作");
    }

    #[tokio::test]
    async fn test_advanced_condition_numeric_operations() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("(age * 2) > 40 && (age + 5) < 60".to_string()),
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "age": 25
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "数值运算应该正确工作");
    }

    #[tokio::test]
    async fn test_advanced_condition_string_comparison() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Advanced,
            field_path: None,
            operator: None,
            expected_value: None,
            advanced_expression: Some("name != \"\" && name == \"John\"".to_string()),
            expected_value_type: None,
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "name": "John"
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok(), "字符串比较应该正确工作");
    }

    // ============================================================================
    // parse_expected_value 函数测试
    // ============================================================================

    #[test]
    fn test_parse_number() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("200"),
            Some(&ExpectedValueType::Number),
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::Number(200.into()));
    }

    #[test]
    fn test_parse_number_float() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("3.14"),
            Some(&ExpectedValueType::Number),
        )
        .unwrap();
        assert_eq!(
            result,
            serde_json::Value::Number(serde_json::Number::from_f64(3.14).unwrap())
        );
    }

    #[test]
    fn test_parse_boolean_true() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("true"),
            Some(&ExpectedValueType::Boolean),
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_parse_boolean_false() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("false"),
            Some(&ExpectedValueType::Boolean),
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::Bool(false));
    }

    #[test]
    fn test_parse_null() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("null"),
            Some(&ExpectedValueType::Null),
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::Null);
    }

    #[test]
    fn test_parse_null_empty() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!(""),
            Some(&ExpectedValueType::Null),
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::Null);
    }

    #[test]
    fn test_parse_string() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("hello"),
            Some(&ExpectedValueType::String),
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn test_parse_no_type_returns_original() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("hello"),
            None,
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn test_parse_number_from_json_number() {
        // 当值已经是 JSON Number 时
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!(200),
            Some(&ExpectedValueType::Number),
        )
        .unwrap();
        assert_eq!(result, serde_json::Value::Number(200.into()));
    }

    #[test]
    fn test_parse_boolean_invalid() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("yes"),
            Some(&ExpectedValueType::Boolean),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无法解析为布尔值"));
    }

    #[test]
    fn test_parse_number_invalid() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("not_a_number"),
            Some(&ExpectedValueType::Number),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无法解析为数字"));
    }

    #[test]
    fn test_parse_null_invalid() {
        let result = ConditionalNodeExecutor::parse_expected_value(
            &serde_json::json!("something"),
            Some(&ExpectedValueType::Null),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("期望 null 但得到"));
    }

    // ============================================================================
    // 带类型选择器的集成测试
    // ============================================================================

    #[tokio::test]
    async fn test_simple_condition_with_type_number() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("status".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!("200")), // 字符串 "200"
            advanced_expression: None,
            expected_value_type: Some(serde_json::json!("number")), // 解析为数字
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "status": 200 // 实际值是数字
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty()); // true 分支无连接
        assert_eq!(output_data.unwrap(), json!(true));
    }

    #[tokio::test]
    async fn test_simple_condition_with_type_boolean() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("enabled".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!("true")), // 字符串 "true"
            advanced_expression: None,
            expected_value_type: Some(serde_json::json!("boolean")), // 解析为布尔
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "enabled": true
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
        assert_eq!(output_data.unwrap(), json!(true));
    }

    #[tokio::test]
    async fn test_simple_condition_with_type_string() {
        let executor = ConditionalNodeExecutor::new();
        let config = ConditionalConfig {
            condition_type: ConditionType::Simple,
            field_path: Some("name".to_string()),
            operator: Some(ConditionOperator::Equal),
            expected_value: Some(serde_json::json!(200)), // 数字 200
            advanced_expression: None,
            expected_value_type: Some(serde_json::json!("string")), // 解析为字符串 "200"
        };
        let node = create_test_node(config);
        let mut context = create_test_context(serde_json::json!({
            "name": "200" // 字符串 "200"
        }));

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
        assert_eq!(output_data.unwrap(), json!(true));
    }
}
