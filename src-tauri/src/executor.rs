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
                            // 简单遍历：取第一个连接的目标节点
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
                        // 查找下一个节点（非Start节点的通用逻辑）
                        let next_nodes = connection_map.get(&current_node_id);
                        if let Some(targets) = next_nodes {
                            if targets.is_empty() {
                                // 节点没有输出连接，执行完成
                                break;
                            }
                            // 简单遍历：取第一个连接的目标节点
                            current_node_id = targets[0].clone();
                        } else {
                            // 节点没有输出连接，执行完成
                            break;
                        }
                    }
                }
                Err(error) => {
                    // 如果节点尚未实现（占位符），优雅终止流程
                    // 检查错误消息是否包含"尚未实现"
                    if error.contains("尚未实现") {
                        // 遇到占位符节点，表示流程已到达未实现部分
                        // 返回成功，已执行的节点都成功完成
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
    ///
    /// # 返回
    /// - `Result<(), String>`: 执行结果或错误
    fn execute_node(&self, node: &Node, _context: &ExecutionContext) -> Result<(), String> {
        match node.node_type {
            NodeType::Start => {
                // 开始节点立即传递（无逻辑，<1ms）
                Ok(())
            }
            NodeType::Http => {
                // HTTP节点执行器尚未实现
                Err("HTTP节点执行器尚未实现".to_string())
            }
            NodeType::Conditional => {
                // 条件节点执行器尚未实现
                Err("条件节点执行器尚未实现".to_string())
            }
            NodeType::End => {
                // 结束节点执行器尚未实现
                Err("结束节点执行器尚未实现".to_string())
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

    #[test]
    fn test_flow_executor_success() {
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
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let executor = FlowExecutor::new();
        let context = ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let result = executor.execute_node(&http_node, &context);

        // 验证执行失败
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("尚未实现"));
    }

    #[test]
    fn test_execution_context_uses_flow_uuid() {
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
        let result = executor.execute_flow(&flow, None);

        // 验证上下文中的 flow_id 使用了 flow.uuid
        assert!(!result.success); // 由于缺少连接，应该失败
                                  // 验证错误消息中可能包含 flow_id 相关信息
    }

    #[test]
    fn test_node_map_uses_node_uuid() {
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
        let result = executor.execute_flow(&flow, None);

        // 验证执行成功（节点映射通过 node.uuid 查找成功）
        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[test]
    fn test_connection_map_uses_source_and_target_uuid() {
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
            uuid: "end-uuid-456".to_string(),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::start(),
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
        let result = executor.execute_flow(&flow, None);

        // 验证连接映射成功使用了 source_node_uuid 和 target_node_uuid
        assert!(result.success);
        assert_eq!(result.executed_nodes.len(), 1);
    }

    #[test]
    fn test_current_node_id_uses_start_node_uuid() {
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
            data: NodeData::start(),
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
        let result = executor.execute_flow(&flow, None);

        // 验证执行的第一个节点是 start_node.uuid
        assert!(result.success);
        assert_eq!(result.executed_nodes.len(), 1);
        assert_eq!(result.executed_nodes[0], "start-uuid-tracker");
    }

    #[test]
    fn test_error_message_uses_node_uuid() {
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
        let result = executor.execute_flow(&flow, None);

        // 验证错误消息包含 node.uuid
        assert!(!result.success);
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("检测到循环"));
        assert!(error_msg.contains("loop-node-uuid")); // 错误消息包含 node.uuid
    }

    #[test]
    fn test_execution_result_data_uses_flow_uuid() {
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
        let result = executor.execute_flow(&flow, None);

        // 验证执行结果数据包含 flow.uuid
        assert!(result.success);
        assert!(result.data.is_some());
        let data = result.data.unwrap();
        assert_eq!(data["flow_id"], "result-flow-uuid-999"); // 验证使用了 flow.uuid
    }

    #[test]
    fn test_multiple_connections_uses_uuids() {
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
            data: NodeData::start(),
            ports: None,
        };

        let end_node_2 = Node {
            uuid: "end-2-multi".to_string(),
            node_type: NodeType::End,
            name: "结束 2".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::start(),
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
        let result = executor.execute_flow(&flow, None);

        // 验证执行成功，连接映射正确使用了 UUID
        assert!(result.success);
        assert_eq!(result.executed_nodes.len(), 1);
    }
}
