// 上下文模块 - 执行上下文和变量存储

use crate::report::NodeExecutionReport;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 执行上下文
///
/// 在流程执行期间维护所有必要的状态信息，包括流程标识、当前节点位置、
/// 执行变量和历史记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// 流程 UUID（前端生成）
    pub flow_uuid: String,

    /// 当前执行节点的 UUID
    pub current_node_uuid: Option<String>,

    /// 执行变量（用于变量替换）
    pub variables: VariableStorage,

    /// 执行历史记录（每个节点的详细执行报告）
    pub node_reports: Vec<NodeExecutionReport>,

    /// 流程触发时间（用于记录触发和开始执行的时间差）
    pub flow_trigger_time: u64,
}

impl ExecutionContext {
    /// 创建新的执行上下文
    ///
    /// # 参数
    ///
    /// * `flow_uuid` - 流程的唯一标识符
    /// * `flow_trigger_time` - 流程触发时间（毫秒时间戳）
    ///
    /// # 返回
    ///
    /// 初始化的执行上下文，包含空的变量存储和空的历史记录
    pub fn new(flow_uuid: String, flow_trigger_time: u64) -> Self {
        Self {
            flow_uuid,
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time,
        }
    }

    /// 添加节点执行报告到历史记录
    ///
    /// # 参数
    ///
    /// * `report` - 节点执行报告
    pub fn add_report(&mut self, report: NodeExecutionReport) {
        self.node_reports.push(report);
    }

    /// 设置当前执行的节点
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 当前节点的 UUID
    pub fn set_current_node(&mut self, node_uuid: String) {
        self.current_node_uuid = Some(node_uuid);
    }

    /// 获取当前节点 UUID
    pub fn current_node(&self) -> Option<&String> {
        self.current_node_uuid.as_ref()
    }
}

/// 变量存储（节点隔离）
///
/// 维护全局变量和各节点的局部变量，实现变量作用域隔离。
/// 全局变量在所有节点间共享，局部变量仅在其所属节点内有效。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableStorage {
    /// 全局变量（初始变量 + 显式提升的变量）
    pub global: HashMap<String, serde_json::Value>,

    /// 节点局部变量（node_uuid -> 变量映射）
    pub nodes: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl VariableStorage {
    /// 创建新的变量存储
    ///
    /// # 返回
    ///
    /// 初始化的变量存储，包含空的全局变量映射和空的节点变量映射
    pub fn new() -> Self {
        Self {
            global: HashMap::new(),
            nodes: HashMap::new(),
        }
    }

    /// 设置全局变量
    ///
    /// # 参数
    ///
    /// * `key` - 变量名
    /// * `value` - 变量值
    pub fn set_global(&mut self, key: String, value: serde_json::Value) {
        self.global.insert(key, value);
    }

    /// 获取全局变量
    ///
    /// # 参数
    ///
    /// * `key` - 变量名
    ///
    /// # 返回
    ///
    /// 变量值的引用，如果不存在则返回 None
    pub fn get_global(&self, key: &str) -> Option<&serde_json::Value> {
        self.global.get(key)
    }

    /// 设置节点局部变量
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 节点 UUID
    /// * `key` - 变量名
    /// * `value` - 变量值
    pub fn set_node_variable(&mut self, node_uuid: String, key: String, value: serde_json::Value) {
        self.nodes
            .entry(node_uuid)
            .or_insert_with(HashMap::new)
            .insert(key, value);
    }

    /// 获取节点局部变量
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 节点 UUID
    /// * `key` - 变量名
    ///
    /// # 返回
    ///
    /// 变量值的引用，如果不存在则返回 None
    pub fn get_node_variable(&self, node_uuid: &str, key: &str) -> Option<&serde_json::Value> {
        self.nodes.get(node_uuid)?.get(key)
    }

    /// 获取节点的所有局部变量
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 节点变量映射的引用，如果节点不存在则返回 None
    pub fn get_node_variables(
        &self,
        node_uuid: &str,
    ) -> Option<&HashMap<String, serde_json::Value>> {
        self.nodes.get(node_uuid)
    }

    /// 删除节点的所有局部变量
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 被删除的变量映射，如果节点不存在则返回 None
    pub fn remove_node_variables(
        &mut self,
        node_uuid: &str,
    ) -> Option<HashMap<String, serde_json::Value>> {
        self.nodes.remove(node_uuid)
    }

    /// 合并全局变量到指定节点的局部变量
    ///
    /// 用于在节点执行时提供完整的变量上下文
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 合并后的变量映射（包含全局变量和节点局部变量）
    pub fn get_merged_variables(&self, node_uuid: &str) -> HashMap<String, serde_json::Value> {
        let mut merged = HashMap::new();
        // 先添加全局变量
        for (k, v) in &self.global {
            merged.insert(k.clone(), v.clone());
        }
        // 再添加节点局部变量（会覆盖同名全局变量）
        if let Some(node_vars) = self.nodes.get(node_uuid) {
            for (k, v) in node_vars {
                merged.insert(k.clone(), v.clone());
            }
        }
        merged
    }
}

impl Default for VariableStorage {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_creation() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        assert_eq!(ctx.flow_uuid, "test-flow");
        assert_eq!(ctx.flow_trigger_time, 1000);
        assert!(ctx.current_node_uuid.is_none());
        assert_eq!(ctx.node_reports.len(), 0);
    }

    #[test]
    fn test_execution_context_set_current_node() {
        let mut ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        ctx.set_current_node("node-1".to_string());

        assert_eq!(ctx.current_node(), Some(&"node-1".to_string()));
    }

    #[test]
    fn test_execution_context_serialization() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 序列化
        let json = serde_json::to_string(&ctx).expect("序列化失败");
        assert!(json.contains("test-flow"));

        // 反序列化
        let deserialized: ExecutionContext = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(deserialized.flow_uuid, ctx.flow_uuid);
        assert_eq!(deserialized.flow_trigger_time, ctx.flow_trigger_time);
    }

    #[test]
    fn test_variable_storage_creation() {
        let storage = VariableStorage::new();

        assert_eq!(storage.global.len(), 0);
        assert_eq!(storage.nodes.len(), 0);
    }

    #[test]
    fn test_global_variable_operations() {
        let mut storage = VariableStorage::new();

        storage.set_global("key1".to_string(), serde_json::json!("value1"));

        assert_eq!(
            storage.get_global("key1"),
            Some(&serde_json::json!("value1"))
        );
        assert!(storage.get_global("key2").is_none());
    }

    #[test]
    fn test_node_variable_operations() {
        let mut storage = VariableStorage::new();

        storage.set_node_variable(
            "node-1".to_string(),
            "key1".to_string(),
            serde_json::json!("value1"),
        );

        assert_eq!(
            storage.get_node_variable("node-1", "key1"),
            Some(&serde_json::json!("value1"))
        );
        assert!(storage.get_node_variable("node-1", "key2").is_none());
        assert!(storage.get_node_variable("node-2", "key1").is_none());
    }

    #[test]
    fn test_variable_storage_node_isolation() {
        let mut storage = VariableStorage::new();

        // 设置全局变量
        storage.set_global("global".to_string(), serde_json::json!("global-value"));

        // 为不同节点设置局部变量
        storage.set_node_variable(
            "node-1".to_string(),
            "local".to_string(),
            serde_json::json!("node1-value"),
        );
        storage.set_node_variable(
            "node-2".to_string(),
            "local".to_string(),
            serde_json::json!("node2-value"),
        );

        // 验证节点隔离
        assert_eq!(
            storage.get_node_variable("node-1", "local"),
            Some(&serde_json::json!("node1-value"))
        );
        assert_eq!(
            storage.get_node_variable("node-2", "local"),
            Some(&serde_json::json!("node2-value"))
        );
        assert_eq!(
            storage.get_global("global"),
            Some(&serde_json::json!("global-value"))
        );
    }

    #[test]
    fn test_get_merged_variables() {
        let mut storage = VariableStorage::new();

        // 设置全局变量
        storage.set_global("global1".to_string(), serde_json::json!("global-value1"));
        storage.set_global("override".to_string(), serde_json::json!("global-override"));

        // 设置节点局部变量
        storage.set_node_variable(
            "node-1".to_string(),
            "local1".to_string(),
            serde_json::json!("node-value1"),
        );
        storage.set_node_variable(
            "node-1".to_string(),
            "override".to_string(),
            serde_json::json!("node-override"),
        );

        let merged = storage.get_merged_variables("node-1");

        // 验证全局变量被包含
        assert_eq!(
            merged.get("global1"),
            Some(&serde_json::json!("global-value1"))
        );

        // 验证节点局部变量被包含
        assert_eq!(
            merged.get("local1"),
            Some(&serde_json::json!("node-value1"))
        );

        // 验证节点局部变量覆盖全局变量
        assert_eq!(
            merged.get("override"),
            Some(&serde_json::json!("node-override"))
        );
    }

    #[test]
    fn test_remove_node_variables() {
        let mut storage = VariableStorage::new();

        storage.set_node_variable(
            "node-1".to_string(),
            "key1".to_string(),
            serde_json::json!("value1"),
        );

        // 删除节点变量
        let removed = storage.remove_node_variables("node-1");

        assert!(removed.is_some());
        assert_eq!(
            removed.unwrap().get("key1"),
            Some(&serde_json::json!("value1"))
        );
        assert!(storage.get_node_variable("node-1", "key1").is_none());
    }
}
