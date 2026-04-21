// Entry 节点执行器 - EntryNodeExecutor 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use serde_json::Value;
use std::collections::HashMap;

/// Entry 节点执行器
///
/// Entry 节点立即传递，不执行任何逻辑，仅作为流程入口
pub struct EntryNodeExecutor;

impl Default for EntryNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl EntryNodeExecutor {
    /// 创建新的 Entry 节点执行器
    pub fn new() -> Self {
        EntryNodeExecutor
    }
}

#[async_trait::async_trait]
impl NodeExecutor for EntryNodeExecutor {
    /// 执行 Entry 节点
    ///
    /// Entry 节点立即传递，不执行任何逻辑（<1ms 执行时间）
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文（未使用）
    /// - `connections`: 流程的所有连接
    /// - `node_map`: 节点 UUID 到节点的映射（未使用）
    ///
    /// # 返回
    /// - `Ok((next_node_ids, output_data))`: 执行成功，返回下一个节点 UUID 列表和空输出数据
    ///   - `next_node_ids`: 下一个节点的 UUID 列表
    ///     - 返回 0 或 1 个节点 UUID（空列表表示流程终止）
    ///   - `output_data`: 节点输出数据（Entry 节点必须为 None，不产生数据）
    /// - `Err(error)`: 执行失败，返回错误信息
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<Value>), String> {
        // 记录开始时间
        let start_time = tokio::time::Instant::now();

        // 发送开始日志
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Entry 节点 {} 开始", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 执行前日志
        log::info!(
            "[流程 {}] [Entry] 节点 UUID: {}, 节点名称: {}",
            context.flow_uuid,
            node.uuid,
            node.name
        );

        // 处理初始变量
        if let NodeData::Entry(entry_config) = &node.data {
            for initial_var in &entry_config.initial_variables {
                context.variables.set_variable(
                    initial_var.name.clone(),
                    initial_var.value.clone(),
                    node.uuid.clone(),
                );
            }
        }

        // Entry 节点立即传递，不执行任何逻辑（<1ms 执行时间）
        // 查找下一个节点
        let next_node_ids = connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node.uuid)
            .map(|conn| conn.target_node_uuid.clone())
            .collect::<Vec<_>>();

        // 如果没有输出连接，这是流程的正常终止，不是错误
        if next_node_ids.is_empty() {
            // 发送无连接终止日志
            context
                .emit_log(LogEvent {
                    level: LogLevel::Info,
                    message: "Entry 节点无后续连接，流程终止".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            log::info!(
                "[流程 {}] Entry 节点 {} ({}) 没有输出连接，流程终止",
                context.flow_uuid,
                node.name,
                node.uuid
            );
            // 返回空列表表示流程终止（正常情况）
            return Ok((vec![], None));
        }

        // 计算执行时间
        let duration_ms = start_time.elapsed().as_millis();

        // 发送完成日志
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Entry 节点 {} 完成", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 执行后日志
        log::info!(
            "[流程 {}] [完成] 节点 UUID: {}, 耗时: {}ms, 状态: success",
            context.flow_uuid,
            node.uuid,
            duration_ms
        );

        // 返回下一个节点 UUID 列表和空输出数据（Entry 节点不产生数据）
        Ok((next_node_ids, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::{ExecutionContext, VariableStorage};
    use flow_engine_core::node::{Node, NodePosition, NodeType};
    use flow_engine_core::node_config::NodeData;
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn test_entry_node_executor_success() {
        // 创建测试流程
        let entry_node = Node {
            uuid: "entry-1".to_string(),
            node_type: NodeType::Entry,
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let next_node = Node {
            uuid: "next-1".to_string(),
            node_type: NodeType::Entry,
            name: "Next".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: entry_node.uuid.clone(),
            target_node_uuid: next_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let mut context = ExecutionContext {
            flow_uuid: "flow-1".to_string(),
            current_node_uuid: Some(entry_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };

        let mut node_map = HashMap::new();
        node_map.insert(entry_node.uuid.clone(), &entry_node);
        node_map.insert(next_node.uuid.clone(), &next_node);

        let executor = EntryNodeExecutor::new();
        let start_time = Instant::now();
        let result = executor
            .execute(&entry_node, &mut context, &[connection], &node_map)
            .await;
        let elapsed = start_time.elapsed();

        // 验证执行成功
        assert!(result.is_ok());

        let (next_node_ids, output_data) = result.unwrap();
        // 验证下一个节点 UUID 列表
        assert_eq!(next_node_ids, vec!["next-1".to_string()]);
        // 验证输出数据为空（Entry 节点不产生数据）
        assert!(output_data.is_none());
        // 验证执行时间 < 1ms
        assert!(
            elapsed.as_millis() < 1,
            "执行时间应该小于 1ms，实际: {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_entry_node_executor_no_connection() {
        // 创建没有连接的 Entry 节点 - 正常流程终止
        let entry_node = Node {
            uuid: "entry-1".to_string(),
            node_type: NodeType::Entry,
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext {
            flow_uuid: "flow-1".to_string(),
            current_node_uuid: Some(entry_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };

        let node_map = HashMap::new();

        let executor = EntryNodeExecutor::new();
        let result = executor.execute(&entry_node, &mut context, &[], &node_map).await;

        // 验证执行成功（没有连接是正常终止）
        assert!(result.is_ok());
        let (next_node_ids, output_data) = result.unwrap();
        // 验证下一个节点 UUID 列表为空（流程终止）
        assert!(next_node_ids.is_empty());
        // 验证输出数据为空（Entry 节点不产生数据）
        assert!(output_data.is_none());
    }

    #[tokio::test]
    async fn test_entry_node_executor_multiple_connections() {
        // 测试多个连接的情况
        let entry_node = Node {
            uuid: "entry-1".to_string(),
            node_type: NodeType::Entry,
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let next_node_1 = Node {
            uuid: "next-1".to_string(),
            node_type: NodeType::Entry,
            name: "Next 1".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let next_node_2 = Node {
            uuid: "next-2".to_string(),
            node_type: NodeType::Entry,
            name: "Next 2".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection_1 = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: entry_node.uuid.clone(),
            target_node_uuid: next_node_1.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let connection_2 = Connection {
            id: "conn-2".to_string(),
            source_node_uuid: entry_node.uuid.clone(),
            target_node_uuid: next_node_2.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let mut context = ExecutionContext {
            flow_uuid: "flow-1".to_string(),
            current_node_uuid: Some(entry_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };

        let mut node_map = HashMap::new();
        node_map.insert(entry_node.uuid.clone(), &entry_node);
        node_map.insert(next_node_1.uuid.clone(), &next_node_1);
        node_map.insert(next_node_2.uuid.clone(), &next_node_2);

        let executor = EntryNodeExecutor::new();
        let result = executor
            .execute(
                &entry_node,
                &mut context,
                &[connection_1, connection_2],
                &node_map,
            )
            .await;

        // 验证执行成功
        assert!(result.is_ok());

        let (next_node_ids, _) = result.unwrap();
        // 验证返回所有连接的目标节点 UUID
        assert_eq!(next_node_ids.len(), 2);
        assert!(next_node_ids.contains(&"next-1".to_string()));
        assert!(next_node_ids.contains(&"next-2".to_string()));
    }

    #[tokio::test]
    async fn test_entry_node_executor_execution_time() {
        // 测试执行时间 < 1ms
        let entry_node = Node {
            uuid: "entry-1".to_string(),
            node_type: NodeType::Entry,
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let next_node = Node {
            uuid: "next-1".to_string(),
            node_type: NodeType::Entry,
            name: "Next".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: entry_node.uuid.clone(),
            target_node_uuid: next_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let mut context = ExecutionContext {
            flow_uuid: "flow-1".to_string(),
            current_node_uuid: Some(entry_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };

        let mut node_map = HashMap::new();
        node_map.insert(entry_node.uuid.clone(), &entry_node);
        node_map.insert(next_node.uuid.clone(), &next_node);

        let executor = EntryNodeExecutor::new();

        // 执行 1000 次以获得稳定的执行时间
        let start_time = Instant::now();
        for _ in 0..1000 {
            let _ = executor
                .execute(&entry_node, &mut context, std::slice::from_ref(&connection), &node_map)
                .await;
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

    #[tokio::test]
    async fn test_execute_uses_node_uuid() {
        // 测试 execute 方法使用 node.uuid
        let node_uuid = "test-node-uuid-execute";
        let entry_node = Node {
            uuid: node_uuid.to_string(),
            node_type: NodeType::Entry,
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let next_node = Node {
            uuid: "next-node-uuid".to_string(),
            node_type: NodeType::Entry,
            name: "Next".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        // 连接使用 node.uuid 作为 source_node_uuid
        let connection = Connection {
            id: "conn-execute-uuid".to_string(),
            source_node_uuid: entry_node.uuid.clone(),
            target_node_uuid: next_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let mut context = ExecutionContext {
            flow_uuid: "flow-1".to_string(),
            current_node_uuid: Some(entry_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };

        let mut node_map = HashMap::new();
        node_map.insert(entry_node.uuid.clone(), &entry_node);
        node_map.insert(next_node.uuid.clone(), &next_node);

        let executor = EntryNodeExecutor::new();
        let result = executor
            .execute(&entry_node, &mut context, &[connection], &node_map)
            .await;

        // 验证 execute 方法正确使用了 node.uuid
        assert!(result.is_ok());
        let (next_node_ids, _) = result.unwrap();
        assert!(!next_node_ids.is_empty());
    }

    #[tokio::test]
    async fn test_node_execution_logic_correct() {
        // 测试节点执行逻辑正确
        let entry_node = Node {
            uuid: "logic-test-entry".to_string(),
            node_type: NodeType::Entry,
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let next_node = Node {
            uuid: "logic-test-next".to_string(),
            node_type: NodeType::Entry,
            name: "Next".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: "conn-logic-test".to_string(),
            source_node_uuid: entry_node.uuid.clone(),
            target_node_uuid: next_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let mut context = ExecutionContext {
            flow_uuid: "flow-logic-test".to_string(),
            current_node_uuid: Some(entry_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };

        let mut node_map = HashMap::new();
        node_map.insert(entry_node.uuid.clone(), &entry_node);
        node_map.insert(next_node.uuid.clone(), &next_node);

        let executor = EntryNodeExecutor::new();
        let result = executor
            .execute(&entry_node, &mut context, &[connection], &node_map)
            .await;

        // 验证执行逻辑正确：
        // 1. 执行成功
        assert!(result.is_ok());
        // 2. 返回下一个节点 UUID 列表
        let (next_node_ids, output_data) = result.unwrap();
        assert_eq!(next_node_ids, vec!["logic-test-next".to_string()]);
        // 3. Entry 节点不产生数据
        assert!(output_data.is_none());
    }

    #[tokio::test]
    async fn test_multiple_connections_same_source_uuid() {
        // 测试多个连接使用相同的 source_node_uuid
        let source_uuid = "multi-source-uuid";
        let target1_uuid = "multi-target-1-uuid";
        let target2_uuid = "multi-target-2-uuid";

        let connection1 = Connection {
            id: "conn-multi-1".to_string(),
            source_node_uuid: source_uuid.to_string(),
            target_node_uuid: target1_uuid.to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let connection2 = Connection {
            id: "conn-multi-2".to_string(),
            source_node_uuid: source_uuid.to_string(),
            target_node_uuid: target2_uuid.to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let entry_node = Node {
            uuid: source_uuid.to_string(),
            node_type: NodeType::Entry,
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext {
            flow_uuid: "flow-1".to_string(),
            current_node_uuid: Some(source_uuid.to_string()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };

        let mut node_map = HashMap::new();
        node_map.insert(source_uuid.to_string(), &entry_node);

        let executor = EntryNodeExecutor::new();
        let result = executor
            .execute(&entry_node, &mut context, &[connection1, connection2], &node_map)
            .await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, _) = result.unwrap();
        // 验证返回所有连接的目标节点 UUID
        assert_eq!(next_node_ids.len(), 2);
        assert!(next_node_ids.contains(&target1_uuid.to_string()));
        assert!(next_node_ids.contains(&target2_uuid.to_string()));
    }
}
