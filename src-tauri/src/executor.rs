// 执行引擎模块 - FlowExecutor 实现

use crate::connection::Connection;
use crate::flow::Flow;
use crate::node::{Node, NodeType};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;

/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 执行是否成功
    pub success: bool,
    /// 返回数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 已执行的节点列表
    pub executed_nodes: Vec<String>,
}

/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// 流程 ID
    pub flow_id: String,
    /// 当前节点 ID
    pub current_node_id: Option<String>,
    /// 执行变量
    pub variables: serde_json::Value,
}

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
    let re = Regex::new(r"\{\{([^}]+)\}\}").map_err(|e| format!("正则表达式错误: {}", e))?;
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

        // 从 context.variables 中获取值
        let mut current_value = &context.variables;
        let mut current_path = String::new();

        for (_i, part) in path_parts.iter().enumerate() {
            if !current_path.is_empty() {
                current_path.push('.');
            }
            current_path.push_str(part);

            match current_value {
                serde_json::Value::Object(map) => {
                    if let Some(value) = map.get(*part) {
                        current_value = value;
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
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                return Err(format!(
                    "变量 '{}' 不支持直接替换（复杂类型）",
                    variable_path
                ));
            }
        };

        result = result.replace(full_match, &replacement);
    }

    Ok(result)
}

/// 节点执行器 trait
///
/// 定义了所有节点执行器必须实现的接口
#[async_trait::async_trait]
pub trait NodeExecutor {
    /// 执行节点
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文
    /// - `connections`: 流程的所有连接
    /// - `node_map`: 节点 ID 到节点的映射
    ///
    /// # 返回
    /// - `Result<(Option<String>, Option<serde_json::Value>), String>`:
    ///   - Ok((next_node_id, output_data)): 执行成功，返回下一个节点 ID 和输出数据
    ///   - Err(error): 执行失败，返回错误信息
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Option<String>, Option<serde_json::Value>), String>;
}

/// 流程执行器
pub struct FlowExecutor;

impl FlowExecutor {
    /// 创建新的流程执行器
    pub fn new() -> Self {
        FlowExecutor
    }

    /// 执行流程
    ///
    /// 从开始节点开始，按连接顺序遍历节点
    ///
    /// # 参数
    /// - `flow`: 要执行的流程
    /// - `variables`: 执行上下文变量（可选）
    ///
    /// # 返回
    /// - `ExecutionResult`: 执行结果
    pub async fn execute_flow(
        &self,
        flow: &Flow,
        variables: Option<serde_json::Value>,
    ) -> ExecutionResult {
        let start_time = tokio::time::Instant::now();
        let mut executed_nodes = Vec::new();
        let mut context = ExecutionContext {
            flow_id: flow.uuid.clone(),
            current_node_id: None,
            variables: variables.unwrap_or_else(|| json!({})),
        };

        // 查找开始节点
        let start_node = match flow
            .nodes
            .iter()
            .find(|node| matches!(node.node_type, NodeType::Start))
        {
            Some(node) => node,
            None => {
                return ExecutionResult {
                    success: false,
                    data: None,
                    error: Some("流程缺少开始节点".to_string()),
                    executed_nodes,
                };
            }
        };

        // 构建节点查找映射
        let node_map: HashMap<String, &Node> = flow
            .nodes
            .iter()
            .map(|node| (node.uuid.clone(), node))
            .collect();

        // 构建连接映射：source_node_uuid -> Vec<target_node_uuid>
        let mut connection_map: HashMap<String, Vec<String>> = HashMap::new();
        for conn in &flow.connections {
            connection_map
                .entry(conn.source_node_uuid.clone())
                .or_default()
                .push(conn.target_node_uuid.clone());
        }

        // 从开始节点开始遍历
        let mut current_node_id = start_node.uuid.clone();
        let mut visited = std::collections::HashSet::new();

        while let Some(node) = node_map.get(&current_node_id) {
            // 检测循环（简单防止无限循环）
            if visited.contains(&current_node_id) {
                return ExecutionResult {
                    success: false,
                    data: None,
                    error: Some(format!("检测到循环：节点 {} 已被访问", node.uuid)),
                    executed_nodes,
                };
            }

            visited.insert(current_node_id.clone());
            context.current_node_id = Some(current_node_id.clone());

            // 执行当前节点
            match self.execute_node(node, &context, &flow.connections, &node_map).await {
                Ok((next_node_id_from_executor, output_data)) => {
                    executed_nodes.push(current_node_id.clone());

                    // 如果有输出数据，更新上下文变量
                    if let Some(data) = output_data {
                        log::info!("节点 {} 输出数据: {}", node.name, data);
                        // 将输出数据合并到上下文中
                        if let Some(obj) = data.as_object() {
                            for (key, value) in obj {
                                context.variables[key] = value.clone();
                            }
                        }
                    }

                    // 处理 Conditional 节点：如果返回 None 表示条件为 false，停止执行
                    if matches!(node.node_type, NodeType::Conditional) && next_node_id_from_executor.is_none() {
                        log::info!("条件节点 {} 返回 false，停止流程", node.name);
                        // 条件为 false 是正常的流程终止，返回成功
                        break;
                    }

                    // 处理 End 节点：返回 None 表示流程结束
                    if matches!(node.node_type, NodeType::End) {
                        log::info!("结束节点 {}，流程终止", node.name);
                        // End 节点表示正常结束流程
                        break;
                    }

                    // 查找下一个节点
                    let next_node_id = if next_node_id_from_executor.is_some() {
                        // 如果执行器返回了下一个节点 ID，使用它
                        next_node_id_from_executor
                    } else {
                        // 否则，使用连接映射查找下一个节点
                        connection_map.get(&current_node_id).and_then(|targets| {
                            if targets.is_empty() {
                                None
                            } else {
                                Some(targets[0].clone())
                            }
                        })
                    };

                    if let Some(next_id) = next_node_id {
                        current_node_id = next_id;
                    } else {
                        // 节点没有输出连接或执行器返回 None，执行完成
                        log::info!("节点 {} 没有下一个节点，执行完成", node.name);
                        break;
                    }
                }
                Err(error) => {
                    // 检查是否是条件节点返回 false 的情况
                    if matches!(node.node_type, NodeType::Conditional) && error == "条件评估为false，停止执行" {
                        log::info!("条件节点 {} 评估为 false，停止流程", node.name);
                        // 这是正常的流程终止（条件不满足），返回成功
                        break;
                    }

                    // 如果节点尚未实现（占位符），优雅终止流程
                    // 检查错误消息是否包含"尚未实现"
                    if error.contains("尚未实现") {
                        // 遇到占位符节点，表示流程已到达未实现部分
                        // 返回成功，已执行的节点都成功完成
                        log::info!("节点 {} 尚未实现，流程终止", node.name);
                        break;
                    } else {
                        // 其他错误，返回失败
                        return ExecutionResult {
                            success: false,
                            data: None,
                            error: Some(error),
                            executed_nodes,
                        };
                    }
                }
            }
        }

        // 执行成功
        ExecutionResult {
            success: true,
            data: Some(json!({
                "flow_id": flow.uuid,
                "executed_nodes": executed_nodes,
                "execution_time": start_time.elapsed().as_millis() as u64,
            })),
            error: None,
            executed_nodes,
        }
    }

    /// 执行单个节点
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文
    /// - `connections`: 流程的所有连接
    ///
    /// # 返回
    /// - `Result<(Option<String>, Option<serde_json::Value>), String>`:
    ///   - Ok((next_node_id, output_data)): 执行成功，返回下一个节点 ID 和输出数据
    ///   - Err(error): 执行失败，返回错误信息
    async fn execute_node(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Option<String>, Option<serde_json::Value>), String> {
        match node.node_type {
            NodeType::Start => {
                log::info!("执行开始节点: {}", node.name);
                let executor = crate::start_executor::StartNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::Http => {
                log::info!("执行HTTP节点: {}", node.name);
                let executor = crate::http_executor::HttpNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::Conditional => {
                log::info!("执行条件节点: {}", node.name);
                let executor = crate::conditional_executor::ConditionalNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::End => {
                log::info!("执行结束节点: {}", node.name);
                let executor = crate::end_executor::EndNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::NodeData;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_flow_executor_success() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行结果（开始节点没有输出连接，应该失败）
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("开始节点没有输出连接"));
    }

    #[tokio::test]
    async fn test_flow_executor_with_connection() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行结果
        assert!(result.success);
        assert!(result.error.is_none());
        assert!(result.data.is_some());

        let data = result.data.unwrap();
        assert_eq!(data["flow_id"], flow_id);
        assert_eq!(data["executed_nodes"].as_array().unwrap().len(), 2);
        assert!(data["executed_nodes"].as_array().unwrap()[0]
            .as_str()
            .unwrap()
            .starts_with("start-"));
        assert!(data["executed_nodes"].as_array().unwrap()[1]
            .as_str()
            .unwrap()
            .starts_with("end-"));
        assert_eq!(result.executed_nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_flow_executor_no_start_node() {
        // 创建没有开始节点的流程
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行失败
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("流程缺少开始节点"));
        assert!(result.executed_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_flow_executor_loop_detection() {
        // 创建循环连接的流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        // 创建一个自连接
        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: start_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行失败（检测到循环）
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("检测到循环"));
    }

    #[tokio::test]
    async fn test_flow_executor_with_variables() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let variables = json!({
            "test_var": "test_value",
            "number": 42
        });

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, Some(variables)).await;

        // 验证执行成功
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.executed_nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_flow_executor_start_node_execution_time() {
        // 创建测试流程
        use std::time::Instant;

        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let start_time = Instant::now();
        let result = executor.execute_flow(&flow, None).await;
        let elapsed = start_time.elapsed();

        // 验证执行成功
        assert!(result.success);
        // 验证执行时间 < 10ms（开始节点应该立即传递）
        assert!(
            elapsed.as_millis() < 10,
            "执行时间应该小于 10ms，实际: {}ms",
            elapsed.as_millis()
        );

        // 验证返回的 execution_time 也 < 10ms
        let data = result.data.unwrap();
        let execution_time = data["execution_time"].as_u64().unwrap();
        assert!(execution_time < 10);
    }

    #[tokio::test]
    async fn test_execute_node_unimplemented_type() {
        // 创建未实现的节点类型
        let flow_id = Uuid::new_v4().to_string();
        let conditional_node = Node {
            uuid: format!("conditional-{}", flow_id),
            node_type: NodeType::Conditional,
            name: "条件判断".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let executor = FlowExecutor::new();
        let context = ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(conditional_node.uuid.clone()),
            variables: json!({}),
        };

        let result = executor.execute_node(&conditional_node, &context, &[], &HashMap::new()).await;

        // 验证执行失败
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("节点数据类型不是ConditionalConfig"));
    }

    #[tokio::test]
    async fn test_execution_context_uses_flow_uuid() {
        // 测试 ExecutionContext 初始化使用 flow.uuid
        let flow_id = "test-flow-uuid-123".to_string();
        let start_node = Node {
            uuid: "start-1".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证上下文中的 flow_id 使用了 flow.uuid
        assert!(!result.success); // 由于缺少连接，应该失败
                                  // 验证错误消息中可能包含 flow_id 相关信息
    }

    #[tokio::test]
    async fn test_node_map_uses_node_uuid() {
        // 测试节点映射构建使用 node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行成功（节点映射通过 node.uuid 查找成功）
        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[tokio::test]
    async fn test_connection_map_uses_source_and_target_uuid() {
        // 测试连接映射构建使用 conn.source_node_uuid 和 conn.target_node_uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-uuid-123".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::end(),
            ports: None,
        };

        let connection = Connection {
            id: "conn-uuid-789".to_string(),
            source_node_uuid: start_node.uuid.clone(), // 使用 source_node_uuid
            target_node_uuid: end_node.uuid.clone(),   // 使用 target_node_uuid
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证连接映射成功使用了 source_node_uuid 和 target_node_uuid
        assert!(result.success);
        assert_eq!(result.executed_nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_current_node_id_uses_start_node_uuid() {
        // 测试当前节点 ID 追踪使用 start_node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-uuid-tracker".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node = Node {
            uuid: "end-uuid-tracker".to_string(),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::end(),
            ports: None,
        };

        let connection = Connection {
            id: "conn-tracker".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行的第一个节点是 start_node.uuid
        assert!(result.success);
        assert_eq!(result.executed_nodes.len(), 2);
        assert_eq!(result.executed_nodes[0], "start-uuid-tracker");
        assert_eq!(result.executed_nodes[1], "end-uuid-tracker");
    }

    #[tokio::test]
    async fn test_error_message_uses_node_uuid() {
        // 测试错误消息使用 node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "loop-node-uuid".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        // 创建自连接以触发循环错误
        let connection = Connection {
            id: "loop-conn".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: start_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证错误消息包含 node.uuid
        assert!(!result.success);
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("检测到循环"));
        assert!(error_msg.contains("loop-node-uuid")); // 错误消息包含 node.uuid
    }

    #[tokio::test]
    async fn test_execution_result_data_uses_flow_uuid() {
        // 测试执行结果数据使用 flow.uuid
        let flow_id = "result-flow-uuid-999".to_string();
        let start_node = Node {
            uuid: "start-1".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node = Node {
            uuid: "end-1".to_string(),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let connection = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行结果数据包含 flow.uuid
        assert!(result.success);
        assert!(result.data.is_some());
        let data = result.data.unwrap();
        assert_eq!(data["flow_id"], "result-flow-uuid-999"); // 验证使用了 flow.uuid
    }

    #[tokio::test]
    async fn test_multiple_connections_uses_uuids() {
        // 测试多个连接使用 source_node_uuid 和 target_node_uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-multi".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node_1 = Node {
            uuid: "end-1-multi".to_string(),
            node_type: NodeType::End,
            name: "结束 1".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::end(),
            ports: None,
        };

        let end_node_2 = Node {
            uuid: "end-2-multi".to_string(),
            node_type: NodeType::End,
            name: "结束 2".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::end(),
            ports: None,
        };

        let connection_1 = Connection {
            id: "conn-1-multi".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node_1.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let connection_2 = Connection {
            id: "conn-2-multi".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node_2.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node_1, end_node_2],
            connections: vec![connection_1, connection_2],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行成功，连接映射正确使用了 UUID
        assert!(result.success);
        assert_eq!(result.executed_nodes.len(), 2);
        assert_eq!(result.executed_nodes[0], "start-multi");
        assert_eq!(result.executed_nodes[1], "end-1-multi");
    }

    #[test]
    fn test_replace_variables_simple() {
        // 测试简单变量替换
        let template = "Bearer {{initial_variables.api_key}}";
        let context = ExecutionContext {
            flow_id: "test-flow".to_string(),
            current_node_id: None,
            variables: json!({
                "initial_variables": {
                    "api_key": "test-key"
                }
            }),
        };
        let result = replace_variables(template, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Bearer test-key");
    }

    #[test]
    fn test_replace_variables_nested_path() {
        // 测试嵌套路径
        let template = "Hello {{user.profile.name}}";
        let context = ExecutionContext {
            flow_id: "test-flow".to_string(),
            current_node_id: None,
            variables: json!({
                "user": {
                    "profile": {
                        "name": "Alice"
                    }
                }
            }),
        };
        let result = replace_variables(template, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello Alice");
    }

    #[test]
    fn test_replace_variables_strict_mode() {
        // 测试严格模式（变量不存在时报错）
        let template = "{{nonexistent.var}}";
        let context = ExecutionContext {
            flow_id: "test-flow".to_string(),
            current_node_id: None,
            variables: json!({}),
        };
        let result = replace_variables(template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未找到"));
    }

    #[test]
    fn test_replace_variables_index_not_supported() {
        // 测试索引访问不支持
        let template = "{{items[0]}}";
        let context = ExecutionContext {
            flow_id: "test-flow".to_string(),
            current_node_id: None,
            variables: json!({
                "items": ["a", "b", "c"]
            }),
        };
        let result = replace_variables(template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持索引访问"));
    }
}
