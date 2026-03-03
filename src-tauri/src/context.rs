// 上下文模块 - 执行上下文和变量存储

use crate::log_event::LogEvent;
use crate::report::NodeExecutionReport;
use crate::variable::Variable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::Mutex;

/// 执行上下文
///
/// 在流程执行期间维护所有必要的状态信息，包括流程标识、当前节点位置、
/// 执行变量和历史记录。
#[derive(Clone, Serialize, Deserialize)]
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

    /// 日志发送器（Tauri 模式有值，CLI 模式为 None）
    /// 使用 #[serde(skip)] 跳过序列化，因为 Channel 不支持 Serialize/Deserialize
    #[serde(skip)]
    pub log_sender: Option<Arc<Mutex<Channel<LogEvent>>>>,

    /// Flow file path for resolving relative paths (CLI mode)
    /// Used to resolve relative file paths based on the flow file directory.
    /// Skipped during serialization as it's only needed at runtime.
    #[serde(skip)]
    pub flow_file_path: Option<PathBuf>,
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
            log_sender: None,
            flow_file_path: None,
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

    /// 设置日志发送器（Tauri 模式调用）
    ///
    /// # 参数
    ///
    /// * `sender` - Tauri IPC Channel 封装在 Arc<Mutex> 中
    ///
    /// # 返回
    ///
    /// 更新后的 ExecutionContext
    pub fn with_log_sender(mut self, sender: Arc<Mutex<Channel<LogEvent>>>) -> Self {
        self.log_sender = Some(sender);
        self
    }

    /// Set flow file path for relative path resolution (CLI mode)
    ///
    /// # Arguments
    ///
    /// * `path` - Absolute path to the flow file
    ///
    /// # Returns
    ///
    /// Updated ExecutionContext with flow_file_path set
    pub fn with_flow_file_path(mut self, path: PathBuf) -> Self {
        self.flow_file_path = Some(path);
        self
    }

    /// Resolve file path (supports relative paths)
    ///
    /// # Arguments
    ///
    /// * `file_path` - Original file path (may be absolute or relative)
    ///
    /// # Returns
    ///
    /// Resolved absolute path. If the base path cannot be determined, returns the original path.
    pub fn resolve_file_path(&self, file_path: &str) -> PathBuf {
        let path = PathBuf::from(file_path);

        // Return absolute paths directly
        if path.is_absolute() {
            return path;
        }

        // Resolve relative paths based on flow file directory
        if let Some(flow_path) = &self.flow_file_path {
            if let Some(parent) = flow_path.parent() {
                return parent.join(path);
            }
        }

        path
    }

    /// 发送日志事件（Channel 可用时）
    ///
    /// 如果 log_sender 为 None（CLI 模式），此方法静默跳过，不会报错。
    ///
    /// # 参数
    ///
    /// * `event` - 要发送的日志事件
    pub async fn emit_log(&self, event: LogEvent) {
        if let Some(sender) = &self.log_sender {
            let sender = sender.lock().await;
            let _ = sender.send(event);
        }
    }
}

/// 变量池（重构后的变量存储）
///
/// 使用 Vec<Variable> 存储所有变量，每个变量包含来源节点信息。
/// 支持按名称查找和按节点级联删除。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VariablePool {
    /// 变量列表
    pub variables: Vec<Variable>,
}

/// VariableStorage 是 VariablePool 的类型别名（向后兼容）
pub type VariableStorage = VariablePool;

impl VariablePool {
    /// 创建新的变量池
    ///
    /// # 返回
    ///
    /// 初始化的变量池，包含空的变量列表
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
        }
    }

    /// 设置变量（添加或更新）
    ///
    /// 如果同名变量已存在，则更新其值；否则添加新变量。
    ///
    /// # 参数
    ///
    /// * `name` - 变量名
    /// * `value` - 变量值
    /// * `source_node_uuid` - 来源节点 UUID
    pub fn set_variable(&mut self, name: String, value: Value, source_node_uuid: String) {
        // 查找是否已存在同名变量
        if let Some(existing) = self.variables.iter_mut().find(|v| v.name == name) {
            existing.value = value;
            existing.source_node_uuid = source_node_uuid;
        } else {
            self.variables.push(Variable {
                name,
                value,
                source_node_uuid,
                description: None,
            });
        }
    }

    /// 获取变量值
    ///
    /// # 参数
    ///
    /// * `name` - 变量名
    ///
    /// # 返回
    ///
    /// 变量值的引用，如果不存在则返回 None
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.iter().find(|v| v.name == name).map(|v| &v.value)
    }

    /// 获取变量（完整信息）
    ///
    /// # 参数
    ///
    /// * `name` - 变量名
    ///
    /// # 返回
    ///
    /// 变量的引用，如果不存在则返回 None
    pub fn get_variable_full(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }

    /// 删除节点的所有变量
    ///
    /// # 参数
    ///
    /// * `node_uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 被删除的变量数量
    pub fn remove_node_variables(&mut self, node_uuid: &str) -> usize {
        let original_len = self.variables.len();
        self.variables.retain(|v| v.source_node_uuid != node_uuid);
        original_len - self.variables.len()
    }

    /// 获取合并后的变量（用于变量替换）
    ///
    /// 返回变量名到值的映射，用于模板变量替换。
    ///
    /// # 返回
    ///
    /// 变量名 → 值的 HashMap
    pub fn get_merged_variables(&self) -> HashMap<String, Value> {
        self.variables
            .iter()
            .map(|v| (v.name.clone(), v.value.clone()))
            .collect()
    }

    // ===== 向后兼容方法 =====

    /// 设置全局变量（向后兼容）
    ///
    /// 使用空字符串作为 source_node_uuid
    #[deprecated(note = "请使用 set_variable 方法")]
    pub fn set_global(&mut self, key: String, value: Value) {
        self.set_variable(key, value, String::new());
    }

    /// 获取全局变量（向后兼容）
    #[deprecated(note = "请使用 get_variable 方法")]
    pub fn get_global(&self, key: &str) -> Option<&Value> {
        self.get_variable(key)
    }

    /// 设置节点局部变量（向后兼容）
    #[deprecated(note = "请使用 set_variable 方法")]
    pub fn set_node_variable(&mut self, node_uuid: String, key: String, value: Value) {
        self.set_variable(key, value, node_uuid);
    }

    /// 获取节点局部变量（向后兼容）
    #[deprecated(note = "请使用 get_variable 方法")]
    pub fn get_node_variable(&self, _node_uuid: &str, key: &str) -> Option<&Value> {
        self.get_variable(key)
    }

    /// 获取节点的所有局部变量（向后兼容）
    #[deprecated(note = "此方法在新架构中不再适用")]
    pub fn get_node_variables(
        &self,
        node_uuid: &str,
    ) -> Option<HashMap<String, Value>> {
        let node_vars: HashMap<String, Value> = self
            .variables
            .iter()
            .filter(|v| v.source_node_uuid == node_uuid)
            .map(|v| (v.name.clone(), v.value.clone()))
            .collect();
        if node_vars.is_empty() {
            None
        } else {
            Some(node_vars)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    // ===== VariablePool 新方法测试 =====

    #[test]
    fn test_variable_pool_creation() {
        let pool = VariablePool::new();
        assert!(pool.variables.is_empty());
    }

    #[test]
    fn test_set_variable() {
        let mut pool = VariablePool::new();

        // 添加新变量
        pool.set_variable(
            "token".to_string(),
            json!("abc123"),
            "auth-node".to_string(),
        );

        assert_eq!(pool.variables.len(), 1);
        assert_eq!(pool.get_variable("token"), Some(&json!("abc123")));

        // 更新现有变量
        pool.set_variable(
            "token".to_string(),
            json!("xyz789"),
            "refresh-node".to_string(),
        );

        assert_eq!(pool.variables.len(), 1); // 仍然是 1 个变量
        assert_eq!(pool.get_variable("token"), Some(&json!("xyz789")));

        // 验证 source_node_uuid 被更新
        let var = pool.get_variable_full("token").unwrap();
        assert_eq!(var.source_node_uuid, "refresh-node");
    }

    #[test]
    fn test_get_variable() {
        let mut pool = VariablePool::new();

        pool.set_variable("status".to_string(), json!(200), "http-node".to_string());

        assert_eq!(pool.get_variable("status"), Some(&json!(200)));
        assert!(pool.get_variable("nonexistent").is_none());
    }

    #[test]
    fn test_get_variable_full() {
        let mut pool = VariablePool::new();

        pool.set_variable(
            "user_id".to_string(),
            json!(12345),
            "entry-node".to_string(),
        );

        let var = pool.get_variable_full("user_id").unwrap();
        assert_eq!(var.name, "user_id");
        assert_eq!(var.value, json!(12345));
        assert_eq!(var.source_node_uuid, "entry-node");
    }

    #[test]
    fn test_remove_node_variables() {
        let mut pool = VariablePool::new();

        // 添加多个变量
        pool.set_variable("var1".to_string(), json!(1), "node-1".to_string());
        pool.set_variable("var2".to_string(), json!(2), "node-1".to_string());
        pool.set_variable("var3".to_string(), json!(3), "node-2".to_string());

        assert_eq!(pool.variables.len(), 3);

        // 删除 node-1 的变量
        let removed_count = pool.remove_node_variables("node-1");

        assert_eq!(removed_count, 2);
        assert_eq!(pool.variables.len(), 1);
        assert!(pool.get_variable("var1").is_none());
        assert!(pool.get_variable("var2").is_none());
        assert_eq!(pool.get_variable("var3"), Some(&json!(3)));

        // 删除不存在的节点
        let removed_count2 = pool.remove_node_variables("nonexistent");
        assert_eq!(removed_count2, 0);
    }

    #[test]
    fn test_get_merged_variables() {
        let mut pool = VariablePool::new();

        pool.set_variable("global1".to_string(), json!("value1"), "".to_string());
        pool.set_variable("local1".to_string(), json!("local-value"), "node-1".to_string());

        let merged = pool.get_merged_variables();

        assert_eq!(merged.len(), 2);
        assert_eq!(merged.get("global1"), Some(&json!("value1")));
        assert_eq!(merged.get("local1"), Some(&json!("local-value")));
    }

    #[test]
    fn test_variable_pool_serialization() {
        let mut pool = VariablePool::new();
        pool.set_variable("key1".to_string(), json!("value1"), "node-1".to_string());

        let json_str = serde_json::to_string(&pool).expect("序列化失败");
        let deserialized: VariablePool =
            serde_json::from_str(&json_str).expect("反序列化失败");

        assert_eq!(deserialized.variables.len(), 1);
        assert_eq!(deserialized.variables[0].name, "key1");
        assert_eq!(deserialized.variables[0].value, json!("value1"));
    }

    // ===== 向后兼容性测试 =====

    #[test]
    #[allow(deprecated)]
    fn test_backward_compatible_set_global() {
        let mut pool = VariablePool::new();
        pool.set_global("key1".to_string(), json!("value1"));

        assert_eq!(pool.get_variable("key1"), Some(&json!("value1")));
    }

    #[test]
    #[allow(deprecated)]
    fn test_backward_compatible_get_global() {
        let mut pool = VariablePool::new();
        pool.set_variable("key1".to_string(), json!("value1"), "".to_string());

        assert_eq!(pool.get_global("key1"), Some(&json!("value1")));
    }

    #[test]
    #[allow(deprecated)]
    fn test_backward_compatible_set_node_variable() {
        let mut pool = VariablePool::new();
        pool.set_node_variable("node-1".to_string(), "key1".to_string(), json!("value1"));

        assert_eq!(pool.get_variable("key1"), Some(&json!("value1")));
        let var = pool.get_variable_full("key1").unwrap();
        assert_eq!(var.source_node_uuid, "node-1");
    }

    #[test]
    #[allow(deprecated)]
    fn test_backward_compatible_get_node_variable() {
        let mut pool = VariablePool::new();
        pool.set_variable("key1".to_string(), json!("value1"), "node-1".to_string());

        // 新架构中不再区分节点，get_node_variable 简单地查找变量
        assert_eq!(pool.get_node_variable("node-1", "key1"), Some(&json!("value1")));
    }
}
