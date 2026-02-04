// 执行引擎模块 - FlowExecutor 实现

use crate::connection::Connection;
use crate::flow::Flow;
use crate::node::{Node, NodeType};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

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

/// 节点执行器 trait
///
/// 定义了所有节点执行器必须实现的接口
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
    fn execute(
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
    pub fn execute_flow(
        &self,
        flow: &Flow,
        variables: Option<serde_json::Value>,
    ) -> ExecutionResult {
        let start_time = Instant::now();
        let mut executed_nodes = Vec::new();
        let mut context = ExecutionContext {
            flow_id: flow.id.clone(),
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
            .map(|node| (node.id.clone(), node))
            .collect();

        // 构建连接映射：source_node_id -> Vec<target_node_id>
        let mut connection_map: HashMap<String, Vec<String>> = HashMap::new();
        for conn in &flow.connections {
            connection_map
                .entry(conn.source_node_id.clone())
                .or_default()
                .push(conn.target_node_id.clone());
        }

        // 从开始节点开始遍历
        let mut current_node_id = start_node.id.clone();
        let mut visited = std::collections::HashSet::new();

        while let Some(node) = node_map.get(&current_node_id) {
            // 检测循环（MVP 范围，简单防止无限循环）
            if visited.contains(&current_node_id) {
                return ExecutionResult {
                    success: false,
                    data: None,
                    error: Some(format!("检测到循环：节点 {} 已被访问", node.id)),
                    executed_nodes,
                };
            }

            visited.insert(current_node_id.clone());
            context.current_node_id = Some(current_node_id.clone());

            // 执行当前节点
            match self.execute_node(node, &context) {
                Ok(_result) => {
                    executed_nodes.push(current_node_id.clone());

                    // 开始节点立即传递
                    if matches!(node.node_type, NodeType::Start) {
                        // 查找下一个节点
                        let next_nodes = connection_map.get(&current_node_id);
                        if let Some(targets) = next_nodes {
                            if targets.is_empty() {
                                // 开始节点没有输出连接
                                return ExecutionResult {
                                    success: false,
                                    data: None,
                                    error: Some("开始节点没有输出连接".to_string()),
                                    executed_nodes,
                                };
                            }
                            // 简单遍历：取第一个连接的目标节点（MVP 范围）
                            current_node_id = targets[0].clone();
                        } else {
                            // 开始节点没有输出连接
                            return ExecutionResult {
                                success: false,
                                data: None,
                                error: Some("开始节点没有输出连接".to_string()),
                                executed_nodes,
                            };
                        }
                    } else {
                        // 其他节点（MVP 范围：只处理开始节点，其他节点尚未实现）
                        // 到达其他节点表示开始节点已成功执行，返回成功
                        break;
                    }
                }
                Err(error) => {
                    // MVP 范围：如果遇到未实现的节点类型，停止执行但返回成功
                    // 因为开始节点已经成功执行
                    if matches!(node.node_type, NodeType::End)
                        || matches!(node.node_type, NodeType::Http)
                    {
                        break;
                    } else {
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
                "flow_id": flow.id,
                "executed_nodes": executed_nodes,
                "execution_time": start_time.elapsed().as_millis() as u64,
            })),
            error: None,
            executed_nodes,
        }
    }

    /// 执行单个节点
    ///
    /// MVP 范围：仅处理开始节点（立即传递）
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文
    ///
    /// # 返回
    /// - `Result<(), String>`: 执行结果或错误
    fn execute_node(&self, node: &Node, _context: &ExecutionContext) -> Result<(), String> {
        match node.node_type {
            NodeType::Start => {
                // 开始节点立即传递（无逻辑，<1ms）
                Ok(())
            }
            _ => Err(format!("节点类型 {:?} 尚未实现", node.node_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::node::{Node, NodePosition, NodeType};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn test_flow_executor_success() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            id: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let flow = Flow {
            id: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None);

        // 验证执行结果（开始节点没有输出连接，应该失败）
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("开始节点没有输出连接"));
    }

    #[test]
    fn test_flow_executor_with_connection() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            id: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let end_node = Node {
            id: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_id: start_node.id.clone(),
            target_node_id: end_node.id.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            id: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None);

        // 验证执行结果
        assert!(result.success);
        assert!(result.error.is_none());
        assert!(result.data.is_some());

        let data = result.data.unwrap();
        assert_eq!(data["flow_id"], flow_id);
        assert_eq!(data["executed_nodes"].as_array().unwrap().len(), 1);
        assert!(data["executed_nodes"].as_array().unwrap()[0]
            .as_str()
            .unwrap()
            .starts_with("start-"));
        assert_eq!(result.executed_nodes.len(), 1);
    }

    #[test]
    fn test_flow_executor_no_start_node() {
        // 创建没有开始节点的流程
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            id: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let flow = Flow {
            id: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None);

        // 验证执行失败
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("流程缺少开始节点"));
        assert!(result.executed_nodes.is_empty());
    }

    #[test]
    fn test_flow_executor_loop_detection() {
        // 创建循环连接的流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            id: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        // 创建一个自连接
        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_id: start_node.id.clone(),
            target_node_id: start_node.id.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            id: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None);

        // 验证执行失败（检测到循环）
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("检测到循环"));
    }

    #[test]
    fn test_flow_executor_with_variables() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            id: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let end_node = Node {
            id: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_id: start_node.id.clone(),
            target_node_id: end_node.id.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            id: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let variables = json!({
            "test_var": "test_value",
            "number": 42
        });

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, Some(variables));

        // 验证执行成功
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.executed_nodes.len(), 1);
    }

    #[test]
    fn test_flow_executor_start_node_execution_time() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            id: format!("start-{}", flow_id),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let end_node = Node {
            id: format!("end-{}", flow_id),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_id: start_node.id.clone(),
            target_node_id: end_node.id.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            id: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let start_time = Instant::now();
        let result = executor.execute_flow(&flow, None);
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

    #[test]
    fn test_execute_node_unimplemented_type() {
        // 创建未实现的节点类型
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            id: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };

        let executor = FlowExecutor::new();
        let context = ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.id.clone()),
            variables: json!({}),
        };

        let result = executor.execute_node(&http_node, &context);

        // 验证执行失败
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("尚未实现"));
    }
}
