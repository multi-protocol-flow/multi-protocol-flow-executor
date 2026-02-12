// End 节点执行器

use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::executor::NodeExecutor;
use crate::node::Node;
use std::collections::HashMap;
use std::time::Instant;

/// End 节点执行器
pub struct EndNodeExecutor;

impl Default for EndNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl EndNodeExecutor {
    /// 创建新的 End 节点执行器
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeExecutor for EndNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 执行前日志
        let start_time = Instant::now();
        log::info!("[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: end", context.flow_uuid, node.uuid, node.name);

        // 标记流程结束
        log::info!("[流程 {}] [流程结束]", context.flow_uuid);

        // 从 context 中收集执行结果
        let executed_nodes = context
            .variables
            .get_global("executed_nodes")
            .and_then(|v| v.as_array())
            .map(|arr: &Vec<serde_json::Value>| arr.len())
            .unwrap_or(0);

        // 尝试获取总执行时间（如果存在于上下文中）
        let total_duration_ms = context
            .variables
            .get_global("execution_time_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // 流程结束日志
        log::info!(
            "[流程 {}] [流程结束] 总执行时间: {}ms, 已执行节点数: {}",
            context.flow_uuid,
            total_duration_ms,
            executed_nodes
        );

        let variables_count = context.variables.global.len();

        // 汇总执行状态（success/error）
        let status = if context.variables.get_global("error").is_some()
            || context.variables.get_global("errors").is_some()
        {
            "error"
        } else {
            "success"
        };

        // 记录执行结果
        log::info!("[流程 {}] [执行统计] 已执行节点数: {}, 变量数: {}", context.flow_uuid, executed_nodes, variables_count);
        log::info!("[流程 {}] [流程最终状态] {}", context.flow_uuid, status);

         // 记录变量详情（如果存在）
         if !context.variables.global.is_empty() {
             log::info!("[流程 {}] [最终变量] {}", context.flow_uuid, serde_json::json!(context.variables.global));
         }

        // 计算End节点执行时间
        let duration_ms = start_time.elapsed().as_millis();

        // 执行后日志
        log::info!("[流程 {}] [完成] 节点ID: {}, 耗时: {}ms, 状态: success", context.flow_uuid, node.uuid, duration_ms);

        // 返回空列表表示流程结束，没有下一个节点
        Ok((vec![], None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ExecutionContext, VariableStorage};
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::NodeData;
    use serde_json::json;

    /// 创建测试上下文的辅助函数
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
        }
    }

    #[tokio::test]
    async fn test_end_executor_basic() {
        // 创建 End 节点
        let flow_id = "test-flow".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = ExecutionContext {
            flow_uuid: flow_id.clone(),
            current_node_uuid: Some(end_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
        };

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty()); // End 节点没有下一个节点
        assert!(output_data.is_none()); // 目前框架阶段不返回数据
    }

    #[tokio::test]
    async fn test_end_executor_with_variables() {
        // 创建带变量的 End 节点
        let flow_id = "test-flow-with-vars".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束节点".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = create_test_context(json!({
            "executed_nodes": ["node-1", "node-2"],
            "http_response": {
                "status":200,
                "body": {"message": "success"}
            }
        }));

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_success_status() {
        // 测试成功状态
        let flow_id = "test-flow-success".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = create_test_context(json!({
            "executed_nodes": ["node-1", "node-2", "node-3"],
            "result": "completed"
        }));

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_error_status() {
        // 测试错误状态（variables 中包含 error）
        let flow_id = "test-flow-error".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = create_test_context(json!({
            "executed_nodes": ["node-1", "node-2"],
            "error": "HTTP request failed"
        }));

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功（即使有错误，End 节点也正常执行）
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_with_errors_array() {
        // 测试错误状态（variables 中包含 errors 数组）
        let flow_id = "test-flow-errors".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = create_test_context(json!({
            "executed_nodes": ["node-1"],
            "errors": [
                {"node": "node-2", "message": "Connection timeout"},
                {"node": "node-3", "message": "Invalid response"}
            ]
        }));

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功（即使有错误数组，End 节点也正常执行）
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_empty_variables() {
        // 测试空变量的情况
        let flow_id = "test-flow-empty".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = ExecutionContext {
            flow_uuid: flow_id.clone(),
            current_node_uuid: Some(end_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
        };

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_null_variables() {
        // 测试 variables 为 null 的情况
        let flow_id = "test-flow-null".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = ExecutionContext {
            flow_uuid: flow_id.clone(),
            current_node_uuid: Some(end_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
        };

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_complex_variables() {
        // 测试复杂的嵌套变量
        let flow_id = "test-flow-complex".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = create_test_context(json!({
            "executed_nodes": ["node-1", "node-2", "node-3", "node-4"],
            "http_response": {
                "status": 200,
                "headers": {
                    "content-type": "application/json",
                    "server": "nginx"
                },
                "body": {
                    "data": {
                        "items": [{"id": 1}, {"id": 2}]
                    }
                }
            },
            "transformed_data": {
                "count": 2,
                "items": ["item-1", "item-2"]
            },
            "execution_time_ms": 1500
        }));

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_no_executed_nodes() {
        // 测试没有 executed_nodes 的情况
        let flow_id = "test-flow-no-nodes".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = create_test_context(json!({
            "message": "No nodes executed"
        }));

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_end_executor_empty_executed_nodes_array() {
        // 测试 executed_nodes 为空数组的情况
        let flow_id = "test-flow-empty-nodes".to_string();
        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let context = create_test_context(json!({
            "executed_nodes": []
        }));

        let executor = EndNodeExecutor::new();
        let result = executor
            .execute(&end_node, &context, &[], &std::collections::HashMap::new())
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());
    }
}
