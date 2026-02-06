// 开始节点执行器 - StartNodeExecutor 实现

use crate::connection::Connection;
use crate::executor::{ExecutionContext, NodeExecutor};
use crate::node::Node;
use serde_json::Value;
use std::collections::HashMap;

/// 开始节点执行器
///
/// 开始节点立即传递，不执行任何逻辑
pub struct StartNodeExecutor;

impl StartNodeExecutor {
    /// 创建新的开始节点执行器
    pub fn new() -> Self {
        StartNodeExecutor
    }

    /// 查找下一个节点
    ///
    /// 遍历连接，找到从当前节点出发的所有目标节点
    /// MVP 范围：仅返回第一个目标节点
    ///
    /// # 参数
    /// - `node_id`: 当前节点 ID
    /// - `connections`: 流程的所有连接
    ///
    /// # 返回
    /// - `Option<String>`: 下一个节点 ID，如果没有连接则返回 None
    fn find_next_node(&self, node_id: &str, connections: &[Connection]) -> Option<String> {
        // 查找从当前节点出发的所有连接
        connections
            .iter()
            .find(|conn| conn.source_node_uuid == node_id)
            .map(|conn| conn.target_node_uuid.clone())
    }
}

impl NodeExecutor for StartNodeExecutor {
    /// 执行开始节点
    ///
    /// 开始节点立即传递，不执行任何逻辑（<1ms 执行时间）
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文（未使用）
    /// - `connections`: 流程的所有连接
    /// - `node_map`: 节点 ID 到节点的映射（未使用）
    ///
    /// # 返回
    /// - `Ok((next_node_id, output_data))`: 执行成功，返回下一个节点 ID 和空输出数据
    /// - `Err(error)`: 执行失败，返回错误信息
    fn execute(
        &self,
        node: &Node,
        _context: &ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Option<String>, Option<Value>), String> {
        // 立即传递，不执行任何逻辑（<1ms 执行时间）
        // 查找下一个节点
        let next_node_id = self.find_next_node(&node.uuid, connections);

        // 验证：开始节点必须有输出连接
        if next_node_id.is_none() {
            return Err("开始节点没有输出连接".to_string());
        }

        // 返回下一个节点 ID 和空输出数据（开始节点不产生数据）
        Ok((next_node_id, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::NodeData;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn test_start_node_executor_success() {
        // 创建测试流程
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

        let context = ExecutionContext {
            flow_id: "flow-1".to_string(),
            current_node_id: Some(start_node.uuid.clone()),
            variables: json!({}),
        };

        let mut node_map = HashMap::new();
        node_map.insert(start_node.uuid.clone(), &start_node);
        node_map.insert(end_node.uuid.clone(), &end_node);

        let executor = StartNodeExecutor::new();
        let start_time = Instant::now();
        let result = executor.execute(&start_node, &context, &[connection], &node_map);
        let elapsed = start_time.elapsed();

        // 验证执行成功
        assert!(result.is_ok());

        let (next_node_id, output_data) = result.unwrap();
        // 验证下一个节点 ID
        assert_eq!(next_node_id, Some("end-1".to_string()));
        // 验证输出数据为空（开始节点不产生数据）
        assert!(output_data.is_none());
        // 验证执行时间 < 1ms
        assert!(
            elapsed.as_millis() < 1,
            "执行时间应该小于 1ms，实际: {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_start_node_executor_no_connection() {
        // 创建没有连接的开始节点
        let start_node = Node {
            uuid: "start-1".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let context = ExecutionContext {
            flow_id: "flow-1".to_string(),
            current_node_id: Some(start_node.uuid.clone()),
            variables: json!({}),
        };

        let node_map = HashMap::new();

        let executor = StartNodeExecutor::new();
        let result = executor.execute(&start_node, &context, &[], &node_map);

        // 验证执行失败
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "开始节点没有输出连接");
    }

    #[test]
    fn test_start_node_executor_find_next_node() {
        // 测试 find_next_node 方法
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

        let executor = StartNodeExecutor::new();
        let next_node_id = executor.find_next_node(&start_node.uuid, &[connection]);

        // 验证找到下一个节点
        assert_eq!(next_node_id, Some("end-1".to_string()));
    }

    #[test]
    fn test_start_node_executor_find_next_node_empty_connections() {
        // 测试没有连接的情况
        let executor = StartNodeExecutor::new();
        let next_node_id = executor.find_next_node("start-1", &[]);

        // 验证返回 None
        assert!(next_node_id.is_none());
    }

    #[test]
    fn test_start_node_executor_multiple_connections() {
        // 测试多个连接的情况（MVP 范围：仅返回第一个）
        let start_node = Node {
            uuid: "start-1".to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node_1 = Node {
            uuid: "end-1".to_string(),
            node_type: NodeType::End,
            name: "结束 1".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::start(),
            ports: None,
        };

        let end_node_2 = Node {
            uuid: "end-2".to_string(),
            node_type: NodeType::End,
            name: "结束 2".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::start(),
            ports: None,
        };

        let connection_1 = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node_1.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let connection_2 = Connection {
            id: "conn-2".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node_2.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let context = ExecutionContext {
            flow_id: "flow-1".to_string(),
            current_node_id: Some(start_node.uuid.clone()),
            variables: json!({}),
        };

        let mut node_map = HashMap::new();
        node_map.insert(start_node.uuid.clone(), &start_node);
        node_map.insert(end_node_1.uuid.clone(), &end_node_1);
        node_map.insert(end_node_2.uuid.clone(), &end_node_2);

        let executor = StartNodeExecutor::new();
        let result = executor.execute(
            &start_node,
            &context,
            &[connection_1, connection_2],
            &node_map,
        );

        // 验证执行成功
        assert!(result.is_ok());

        let (next_node_id, _) = result.unwrap();
        // 验证返回第一个连接的目标节点（MVP 范围）
        assert_eq!(next_node_id, Some("end-1".to_string()));
    }

    #[test]
    fn test_start_node_executor_execution_time() {
        // 测试执行时间 < 1ms
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

        let context = ExecutionContext {
            flow_id: "flow-1".to_string(),
            current_node_id: Some(start_node.uuid.clone()),
            variables: json!({}),
        };

        let mut node_map = HashMap::new();
        node_map.insert(start_node.uuid.clone(), &start_node);
        node_map.insert(end_node.uuid.clone(), &end_node);

        let executor = StartNodeExecutor::new();

        // 执行 1000 次以获得稳定的执行时间
        let start_time = Instant::now();
        for _ in 0..1000 {
            let _ = executor.execute(&start_node, &context, &[connection.clone()], &node_map);
        }
        let elapsed = start_time.elapsed();

        // 验证平均执行时间 < 1ms
        let avg_time_ms = elapsed.as_millis() as f64 / 1000.0;
        assert!(
            avg_time_ms < 1.0,
            "平均执行时间应该小于 1ms，实际: {}ms",
            avg_time_ms
        );
    }
}
