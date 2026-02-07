 use crate::connection::Connection;
 use crate::executor::{ExecutionContext, NodeExecutor};
 use crate::node::{Node, NodePosition};
 use crate::node_config::{ConditionOperator, ConditionType, NodeData};
 use evalexpr::ContextWithMutableVariables;
 use std::collections::HashMap;

/// 条件节点执行器
pub struct ConditionalNodeExecutor;

impl ConditionalNodeExecutor {
    /// 创建新的条件节点执行器
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ConditionalNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Option<String>, Option<serde_json::Value>), String> {
        log::info!("执行条件节点: {}", node.name);

        // 从Node的data字段提取ConditionalConfig
        let conditional_config = match &node.data {
            NodeData::Conditional(config) => config.clone(),
            _ => {
                let error_msg = format!("节点数据类型不是ConditionalConfig: {}", node.uuid);
                log::error!("{}", error_msg);
                return Err(error_msg);
            }
        };

        log::info!("条件类型: {:?}", conditional_config.condition_type);

        // 判断条件类型并评估
        let condition_result = match conditional_config.condition_type {
            ConditionType::Simple => {
                // 简单条件：评估field_path + operator + expected_value
                log::info!("评估简单条件");
                self.evaluate_simple_condition(
                    &conditional_config,
                    &context.variables,
                )?
            }
            ConditionType::Advanced => {
                // 高级条件：评估advanced_expression
                log::info!("评估高级条件");
                self.evaluate_advanced_condition(
                    &conditional_config,
                    &context.variables,
                )?
            }
        };

        if condition_result {
            log::info!("条件评估为true，继续执行");
            // 查找下一个节点（通过连接）
            let next_node_id = connections
                .iter()
                .find(|conn| conn.source_node_uuid == node.uuid)
                .map(|conn| conn.target_node_uuid.clone());

            // 返回next_node_id和空output_data
            Ok((next_node_id, None))
        } else {
            log::info!("条件评估为false，停止执行");
            Err("条件评估为false，停止执行".to_string())
        }
    }
}

impl ConditionalNodeExecutor {
    /// 评估简单条件
    fn evaluate_simple_condition(
        &self,
        config: &crate::node_config::ConditionalConfig,
        variables: &serde_json::Value,
    ) -> Result<bool, String> {
        // 获取field_path
        let field_path = config.field_path.as_ref().ok_or_else(|| {
            log::error!("简单条件缺少field_path");
            "简单条件缺少field_path".to_string()
        })?;

        // 获取operator
        let operator = config.operator.as_ref().ok_or_else(|| {
            log::error!("简单条件缺少operator");
            "简单条件缺少operator".to_string()
        })?;

        log::info!("字段路径: {}, 操作符: {:?}", field_path, operator);

        // 从variables中提取field_path指定的值
        let actual_value = self.extract_field_value(variables, field_path)?;

        log::info!("提取的值: {:?}", actual_value);

        // 获取expected_value（对于exists操作符不需要expected_value）
        let expected_value = config.expected_value.as_ref();

        // 使用operator比较提取值和expected_value
        let result = self.compare_values(operator, &actual_value, expected_value)?;

        log::info!("比较结果: {}", result);
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
            log::error!("高级条件缺少advanced_expression");
            "高级条件缺少advanced_expression".to_string()
        })?;

        log::info!("高级表达式: {}", expression);

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
            log::error!("评估表达式 '{}' 失败: {}", expression, e);
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
                log::error!("表达式返回不支持的类型: {:?}", eval_result);
                return Err(format!("表达式返回不支持的类型: {:?}", eval_result));
            }
        };

        log::info!("表达式评估结果: {}", result);
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
                        log::error!("字段 '{}' 不存在", current_path);
                        return Err(format!("字段 '{}' 不存在", current_path));
                    }
                }
                _ => {
                    log::error!("字段 '{}' 不是对象类型", current_path);
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
                    log::error!("equal操作符需要expected_value");
                    "equal操作符需要expected_value".to_string()
                })?;
                let result = actual == expected_val;
                log::debug!("equal: {:?} == {:?} = {}", actual, expected_val, result);
                Ok(result)
            }
            ConditionOperator::NotEqual => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("not_equal操作符需要expected_value");
                    "not_equal操作符需要expected_value".to_string()
                })?;
                let result = actual != expected_val;
                log::debug!("not_equal: {:?} != {:?} = {}", actual, expected_val, result);
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
                    log::error!("contains操作符需要expected_value");
                    "contains操作符需要expected_value".to_string()
                })?;
                self.compare_contains(actual, expected_val)
            }
            ConditionOperator::StartsWith => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("starts_with操作符需要expected_value");
                    "starts_with操作符需要expected_value".to_string()
                })?;
                self.compare_starts_with(actual, expected_val)
            }
            ConditionOperator::EndsWith => {
                let expected_val = expected.ok_or_else(|| {
                    log::error!("ends_with操作符需要expected_value");
                    "ends_with操作符需要expected_value".to_string()
                })?;
                self.compare_ends_with(actual, expected_val)
            }
            ConditionOperator::Exists => {
                // exists操作符只需要检查字段是否存在，不需要expected_value
                let result = !matches!(actual, serde_json::Value::Null);
                log::debug!("exists: {:?} = {}", actual, result);
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
            log::error!("{}操作符需要expected_value", operator_name);
            format!("{}操作符需要expected_value", operator_name)
        })?;
        let expected_num = self.value_to_number(expected_val, "expected")?;
        let result = compare(actual_num, expected_num);
        log::debug!("{}: {} {} {} = {}", operator_name, actual_num, operator_name, expected_num, result);
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
        log::debug!("contains: '{}' contains '{}' = {}", actual_str, expected_str, result);
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
        log::debug!("starts_with: '{}' starts with '{}' = {}", actual_str, expected_str, result);
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
        log::debug!("ends_with: '{}' ends with '{}' = {}", actual_str, expected_str, result);
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
    use crate::node_config::{ConditionalConfig, NodeData};
    use std::collections::HashMap;

    fn create_test_node(config: ConditionalConfig) -> Node {
        Node {
            uuid: "test-node-id".to_string(),
            name: "Test Condition Node".to_string(),
            node_type: crate::node::NodeType::Conditional,
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Conditional(config),
            ports: None,
        }
    }

    fn create_test_context(variables: serde_json::Value) -> ExecutionContext {
        ExecutionContext {
            flow_id: "test-flow-id".to_string(),
            variables,
            current_node_id: None,
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "name": "John Doe"
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "name": "John Doe"
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "name": "John Doe"
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "email": "test@example.com"
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "user": {
                "age": 25
            }
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_err(), "条件为false应该返回错误");
        assert_eq!(result.unwrap_err(), "条件评估为false，停止执行");
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "age": 25,
            "name": "John"
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "age": 25,
            "name": "John"
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "age": 25,
            "name": "John"
        }));

        let result = executor.execute(&node, &context, &[], &HashMap::new()).await;
        assert!(result.is_err(), "条件为false应该返回错误");
        assert_eq!(result.unwrap_err(), "条件评估为false，停止执行");
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
        };
        let node = create_test_node(config);
        let context = create_test_context(serde_json::json!({
            "status": "active"
        }));

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-node-id".to_string(),
            target_node_uuid: "next-node-id".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        }];

        let result = executor.execute(&node, &context, &connections, &HashMap::new()).await;
        assert!(result.is_ok(), "条件为true应该成功");
        let (next_node_id, output_data) = result.unwrap();
        assert_eq!(next_node_id, Some("next-node-id".to_string()));
        assert!(output_data.is_none(), "输出数据应该为空");
    }

    #[tokio::test]
    async fn test_extract_field_value() {
        let executor = ConditionalNodeExecutor::new();
        let data = serde_json::json!({
            "user": {
                "name": "John",
                "age": 25
            }
        });

        let result = executor.extract_field_value(&data, "user.name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!("John"));

        let result = executor.extract_field_value(&data, "user.age");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!(25));
    }

    #[tokio::test]
    async fn test_extract_field_value_error() {
        let executor = ConditionalNodeExecutor::new();
        let data = serde_json::json!({
            "user": {
                "name": "John"
            }
        });

        let result = executor.extract_field_value(&data, "user.age");
        assert!(result.is_err());
    }
}
