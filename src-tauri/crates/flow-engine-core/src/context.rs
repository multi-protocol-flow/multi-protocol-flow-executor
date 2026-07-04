// 上下文模块 - 执行上下文和变量存储

use crate::log_event::LogEvent;
use crate::node_config::ProxyMode;
use crate::report::NodeExecutionReport;
use crate::variable::Variable;
use crate::traits::{LogSender, DebugEventSender, ExecutionEventSender};
use crate::execution_event::ExecutionEvent;
use crate::ws_stream::{WsConnectionStream, WsMessage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 代理状态（运行时，通过 context.providers["`proxy_state`"] 存取）
///
/// 代理节点设置时写入 providers，清除代理时从 providers 移除。
/// 协议执行器通过 `context.get_provider::<ProxyState>("proxy_state")` 读取。
#[derive(Debug, Clone)]
pub struct ProxyState {
    /// 代理模式
    pub mode: ProxyMode,
    /// 代理服务器地址（host:port 格式）
    pub host: String,
    /// 代理服务器端口
    pub port: u16,
    /// 代理用户名
    pub username: Option<String>,
    /// 代理密码
    pub password: Option<String>,
}

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

    /// 流程触发时间（用于记录触发和start execution的时间差）
    pub flow_trigger_time: u64,

    /// 日志发送器（Tauri 模式有值，CLI 模式为 None）
    /// 使用 #[serde(skip)] 跳过序列化，因为 dyn `LogSender` 不支持序列化
    #[serde(skip)]
    pub log_sender: Option<Arc<dyn LogSender>>,

    /// Flow file path for resolving relative paths (CLI mode)
    /// Used to resolve relative file paths based on the flow file directory.
    /// Skipped during serialization as it's only needed at runtime.
    #[serde(skip)]
    pub flow_file_path: Option<PathBuf>,

    /// WebSocket 连接 ID → URL 映射（用于反查）
    /// 存储每个连接 ID 对应的 WebSocket URL，供后续节点（如 WS Send+Wait）反查使用
    #[serde(skip)]
    pub ws_connection_urls: HashMap<String, String>,

    /// WebSocket 消息流追踪
    /// 存储每个连接的消息流，支持按节点追踪消息历史
    /// 使用 `tokio::sync::RwLock` 实现异步安全的并发访问
    #[serde(skip)]
    pub ws_streams: Arc<RwLock<HashMap<String, WsConnectionStream>>>,

    /// 最近一个节点的输出数据
    /// 每个节点执行后均更新此字段
    /// 下游节点通过 jsonpath（如 $.body.xxx）访问此数据
    #[serde(skip)]
    pub last_node_output: Option<Value>,

    /// 调试事件发送器（调试模式有值，普通模式为 None）
    /// 用于 WebSocket 执行器发送调试事件（如 WsStreamCreated、WsMessageSent 等）
    /// 通过 `DebugEventSender` trait 解耦，支持 mpsc 或其他实现
    #[serde(skip)]
    pub debug_event_sender: Option<Arc<dyn DebugEventSender>>,

    /// 执行事件发送器（正常模式有值，CLI 模式为 None）
    /// 用于实时推送节点执行进度事件
    #[serde(skip)]
    pub execution_event_sender: Option<Arc<dyn ExecutionEventSender>>,

    /// 取消信号接收器
    /// 通过 `tokio::sync::watch` 接收取消信号，实现非阻塞取消检查
    #[serde(skip)]
    pub cancel_receiver: Option<tokio::sync::watch::Receiver<bool>>,

    /// 通用 provider 注册表（替代具体的 `ws_manager`/`db_manager`）
    /// key: provider 名称（如 "ws"、"db"）
    /// value: 包装在 Arc 中的 provider 实例
    /// 使用 #[serde(skip)] 跳过序列化，因为 dyn Any 不支持序列化
    #[serde(skip)]
    pub providers: HashMap<String, Arc<dyn Any + Send + Sync>>,
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

            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(RwLock::new(HashMap::new())),

            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
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
    /// * `sender` - `LogSender` trait 实现封装在 Arc 中
    ///
    /// # 返回
    ///
    /// 更新后的 `ExecutionContext`
    pub fn with_log_sender(mut self, sender: Arc<dyn LogSender>) -> Self {
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
    /// Updated `ExecutionContext` with `flow_file_path` set
    pub fn with_flow_file_path(mut self, path: PathBuf) -> Self {
        self.flow_file_path = Some(path);
        self
    }

    /// 设置调试事件发送器（调试模式调用）
    ///
    /// # 参数
    ///
    /// * `sender` - `DebugEvent` 事件发送器
    ///
    /// # 返回
    ///
    /// 更新后的 `ExecutionContext`
    pub fn with_debug_event_sender(mut self, sender: Arc<dyn DebugEventSender>) -> Self {
        self.debug_event_sender = Some(sender);
        self
    }

    /// 设置执行事件发送器（正常模式调用）
    ///
    /// # 参数
    ///
    /// * `sender` - `ExecutionEvent` 事件发送器
    ///
    /// # 返回
    ///
    /// 更新后的 `ExecutionContext`
    pub fn with_execution_event_sender(mut self, sender: Arc<dyn ExecutionEventSender>) -> Self {
        self.execution_event_sender = Some(sender);
        self
    }

    /// 设置取消信号接收器
    ///
    /// # 参数
    ///
    /// * `receiver` - `tokio::sync::watch::Receiver<bool>` 取消信号接收器
    ///
    /// # 返回
    ///
    /// 更新后的 `ExecutionContext`
    pub fn with_cancel_receiver(mut self, receiver: tokio::sync::watch::Receiver<bool>) -> Self {
        self.cancel_receiver = Some(receiver);
        self
    }

    /// 检查流程是否已被取消（非阻塞）
    ///
    /// # 返回
    ///
    /// 如果收到取消信号则返回 `true`，否则返回 `false`
    pub fn is_cancelled(&self) -> bool {
        if let Some(receiver) = &self.cancel_receiver {
            *receiver.borrow()
        } else {
            false
        }
    }

    /// 获取通用 provider
    ///
    /// 从 providers 注册表中按名称获取 provider，并尝试向下转型为指定类型。
    ///
    /// # 参数
    ///
    /// * `name` - provider 名称（如 "ws"、"db"）
    ///
    /// # 返回
    ///
    /// 如果找到且类型匹配则返回 `Some(Arc<T>)`，否则返回 `None`
    pub fn get_provider<T: 'static + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        self.providers
            .get(name)
            .and_then(|p| p.clone().downcast::<T>().ok())
    }

    /// 设置通用 provider
    ///
    /// 向 providers 注册表中注册或替换一个 provider。
    ///
    /// # 参数
    ///
    /// * `name` - provider 名称（如 "ws"、"db"）
    /// * `provider` - 包装在 Arc 中的 provider 实例
    pub fn set_provider(&mut self, name: impl Into<String>, provider: Arc<dyn Any + Send + Sync>) {
        self.providers.insert(name.into(), provider);
    }

    /// 移除通用 provider
    ///
    /// 从 providers 注册表中移除指定名称的 provider。
    /// 用于代理节点清除代理状态。
    ///
    /// # 参数
    ///
    /// * `name` - provider 名称（如 "`proxy_state`"）
    pub fn remove_provider(&mut self, name: &str) {
        self.providers.remove(name);
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

    /// 发送日志事件（LogSender 可用时）
    ///
    /// 如果 `log_sender` 为 None（CLI 模式），此方法静默跳过，不会报错。
    ///
    /// # 参数
    ///
    /// * `event` - 要发送的日志事件
    pub async fn emit_log(&self, event: LogEvent) {
        if let Some(sender) = &self.log_sender {
            let _ = sender.send(event).await;
        }
    }

    /// 发送执行事件（ExecutionEventSender 可用时）
    ///
    /// 如果 `execution_event_sender` 为 None（CLI 模式），此方法静默跳过，不会报错。
    ///
    /// # 参数
    ///
    /// * `event` - 要发送的执行事件
    pub async fn emit_execution_event(&self, event: ExecutionEvent) {
        if let Some(sender) = &self.execution_event_sender {
            let _ = sender.send_event(event);
        }
    }

    /// 存储 WebSocket 连接 URL
    ///
    /// 将连接 ID 与对应的 WebSocket URL 关联存储，供后续节点反查使用。
    ///
    /// # 参数
    ///
    /// * `connection_id` - WebSocket 连接的唯一标识符
    /// * `url` - WebSocket 连接的 URL
    pub fn set_ws_url(&mut self, connection_id: String, url: String) {
        self.ws_connection_urls.insert(connection_id, url);
    }

    /// 获取 WebSocket 连接 URL
    ///
    /// 根据连接 ID 查找对应的 WebSocket URL。
    ///
    /// # 参数
    ///
    /// * `connection_id` - WebSocket 连接的唯一标识符
    ///
    /// # 返回
    ///
    /// 如果找到则返回 URL 的引用，否则返回 None
    pub fn get_ws_url(&self, connection_id: &str) -> Option<&String> {
        self.ws_connection_urls.get(connection_id)
    }

    // ===== WebSocket 消息流辅助方法 =====

    /// 获取 WebSocket 连接消息流
    ///
    /// 根据连接 ID 获取消息流的克隆副本。
    ///
    /// # 参数
    ///
    /// * `connection_id` - WebSocket 连接的唯一标识符
    ///
    /// # 返回
    ///
    /// 如果找到则返回消息流的克隆，否则返回 None
    pub async fn get_ws_stream(&self, connection_id: &str) -> Option<WsConnectionStream> {
        let streams = self.ws_streams.read().await;
        streams.get(connection_id).cloned()
    }

    /// 创建 WebSocket 连接消息流
    ///
    /// 创建新的消息流并设置初始所有者节点。
    ///
    /// # 参数
    ///
    /// * `connection_id` - WebSocket 连接的唯一标识符
    /// * `url` - WebSocket 连接的 URL
    /// * `node_uuid` - 初始所有者节点 UUID
    /// * `node_name` - 初始所有者节点名称
    /// * `node_type` - 初始所有者节点类型
    pub async fn create_ws_stream(
        &self,
        connection_id: String,
        url: String,
        node_uuid: String,
        node_name: String,
        node_type: String,
    ) {
        let mut stream = WsConnectionStream::new(connection_id.clone(), url);
        stream.create_initial_segment(node_uuid, node_name, node_type);

        let mut streams = self.ws_streams.write().await;
        streams.insert(connection_id, stream);
    }

    /// 转移 WebSocket 连接消息流所有权
    ///
    /// 将消息流的所有权从当前节点转移到新节点。
    ///
    /// # 参数
    ///
    /// * `connection_id` - WebSocket 连接的唯一标识符
    /// * `new_node_uuid` - 新所有者节点 UUID
    /// * `new_node_name` - 新所有者节点名称
    /// * `new_node_type` - 新所有者节点类型
    ///
    /// # 说明
    ///
    /// 此方法在 `ws_send_wait` 或 `ws_close` 节点start execution时调用。
    /// 如果连接 ID 不存在，则不做任何操作。
    pub async fn transfer_ws_ownership(
        &self,
        connection_id: &str,
        new_node_uuid: String,
        new_node_name: String,
        new_node_type: String,
    ) {
        let mut streams = self.ws_streams.write().await;
        if let Some(stream) = streams.get_mut(connection_id) {
            stream.transfer_ownership(new_node_uuid, new_node_name, new_node_type);
        }
    }

    /// 推送 WebSocket 消息到消息流
    ///
    /// 将消息推送到指定连接的当前段。
    ///
    /// # 参数
    ///
    /// * `connection_id` - WebSocket 连接的唯一标识符
    /// * `message` - 要推送的消息
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 推送Success
    /// * `Err(msg)` - 推送Failed（连接不存在、连接已关闭或消息过大）
    pub async fn push_ws_message(
        &self,
        connection_id: &str,
        message: WsMessage,
    ) -> Result<(), String> {
        let mut streams = self.ws_streams.write().await;
        if let Some(stream) = streams.get_mut(connection_id) {
            stream.push_message(message)
        } else {
            Err(format!("WebSocket 连接 {} 不存在", connection_id))
        }
    }

    /// 关闭 WebSocket 连接消息流
    ///
    /// 结束当前段并标记连接为已关闭。
    ///
    /// # 参数
    ///
    /// * `connection_id` - WebSocket 连接的唯一标识符
    ///
    /// # 说明
    ///
    /// 如果连接 ID 不存在，则不做任何操作。
    pub async fn close_ws_stream(&self, connection_id: &str) {
        let mut streams = self.ws_streams.write().await;
        if let Some(stream) = streams.get_mut(connection_id) {
            stream.close();
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

/// `VariableStorage` 是 `VariablePool` 的类型别名（向后兼容）
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
    /// 变量名 → 值的 `HashMap`
    pub fn get_merged_variables(&self) -> HashMap<String, Value> {
        self.variables
            .iter()
            .map(|v| (v.name.clone(), v.value.clone()))
            .collect()
    }

    // ===== 向后兼容方法 =====

    /// 设置全局变量（向后兼容）
    ///
    /// 使用空字符串作为 `source_node_uuid`
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
        let json = serde_json::to_string(&ctx).expect("序列化Failed");
        assert!(json.contains("test-flow"));

        // 反序列化
        let deserialized: ExecutionContext = serde_json::from_str(&json).expect("反序列化Failed");
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

        let json_str = serde_json::to_string(&pool).expect("序列化Failed");
        let deserialized: VariablePool =
            serde_json::from_str(&json_str).expect("反序列化Failed");

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

    // ===== WebSocket 消息流测试 =====

    #[tokio::test]
    async fn test_create_ws_stream() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 创建消息流
        ctx.create_ws_stream(
            "conn-1".to_string(),
            "wss://example.com/ws".to_string(),
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws_connect".to_string(),
        ).await;

        // 验证创建Success
        let stream = ctx.get_ws_stream("conn-1").await;
        assert!(stream.is_some());

        let stream = stream.unwrap();
        assert_eq!(stream.connection_id, "conn-1");
        assert_eq!(stream.url, "wss://example.com/ws");
        assert_eq!(stream.current_owner_node_uuid, Some("node-1".to_string()));
        assert_eq!(stream.segments.len(), 1);
    }

    #[tokio::test]
    async fn test_get_ws_stream_not_found() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 查询不存在的连接
        let stream = ctx.get_ws_stream("nonexistent").await;
        assert!(stream.is_none());
    }

    #[tokio::test]
    async fn test_transfer_ws_ownership() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 创建消息流
        ctx.create_ws_stream(
            "conn-1".to_string(),
            "wss://example.com/ws".to_string(),
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws_connect".to_string(),
        ).await;

        // 转移所有权
        ctx.transfer_ws_ownership(
            "conn-1",
            "node-2".to_string(),
            "Send Node".to_string(),
            "ws_send_wait".to_string(),
        ).await;

        // 验证所有权转移
        let stream = ctx.get_ws_stream("conn-1").await.unwrap();
        assert_eq!(stream.current_owner_node_uuid, Some("node-2".to_string()));
        assert_eq!(stream.segments.len(), 2);
        // 第一个段应该已结束
        assert!(stream.segments[0].ended_at.is_some());
        // 第二个段应该是当前段
        assert!(stream.segments[1].ended_at.is_none());
        assert_eq!(stream.segments[1].node_uuid, "node-2");
    }

    #[tokio::test]
    async fn test_transfer_ws_ownership_nonexistent() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 对不存在的连接转移所有权应该静默Failed（不做任何操作）
        ctx.transfer_ws_ownership(
            "nonexistent",
            "node-2".to_string(),
            "Send Node".to_string(),
            "ws_send_wait".to_string(),
        ).await;

        // 不应该有任何Error
    }

    #[tokio::test]
    async fn test_push_ws_message() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 创建消息流
        ctx.create_ws_stream(
            "conn-1".to_string(),
            "wss://example.com/ws".to_string(),
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws_connect".to_string(),
        ).await;

        // 推送消息
        let msg = WsMessage::new(
            crate::ws_stream::MessageDirection::Sent,
            crate::ws_stream::MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        let result = ctx.push_ws_message("conn-1", msg).await;
        assert!(result.is_ok());

        // 验证消息已推送
        let stream = ctx.get_ws_stream("conn-1").await.unwrap();
        assert_eq!(stream.total_message_count(), 1);
    }

    #[tokio::test]
    async fn test_push_ws_message_nonexistent() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 对不存在的连接推送消息
        let msg = WsMessage::new(
            crate::ws_stream::MessageDirection::Sent,
            crate::ws_stream::MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        let result = ctx.push_ws_message("nonexistent", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_close_ws_stream() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 创建消息流
        ctx.create_ws_stream(
            "conn-1".to_string(),
            "wss://example.com/ws".to_string(),
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws_connect".to_string(),
        ).await;

        // 关闭连接
        ctx.close_ws_stream("conn-1").await;

        // 验证已关闭
        let stream = ctx.get_ws_stream("conn-1").await.unwrap();
        assert!(stream.is_closed);
        assert!(stream.segments[0].ended_at.is_some());
    }

    #[tokio::test]
    async fn test_close_ws_stream_nonexistent() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 对不存在的连接关闭应该静默Failed（不做任何操作）
        ctx.close_ws_stream("nonexistent").await;

        // 不应该有任何Error
    }

    #[tokio::test]
    async fn test_push_ws_message_after_close() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 创建消息流
        ctx.create_ws_stream(
            "conn-1".to_string(),
            "wss://example.com/ws".to_string(),
            "node-1".to_string(),
            "Connect Node".to_string(),
            "ws_connect".to_string(),
        ).await;

        // 关闭连接
        ctx.close_ws_stream("conn-1").await;

        // 尝试推送消息
        let msg = WsMessage::new(
            crate::ws_stream::MessageDirection::Sent,
            crate::ws_stream::MessageContent::Text {
                content: "hello".to_string(),
            },
        );

        let result = ctx.push_ws_message("conn-1", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("closed"));
    }

    /// 测试 is_cancelled 默认返回 false
    #[test]
    fn test_is_cancelled_default() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);
        assert!(!ctx.is_cancelled(), "默认不应被取消");
    }

    /// 测试 is_cancelled 在收到信号后返回 true
    #[test]
    fn test_is_cancelled_with_signal() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000)
            .with_cancel_receiver(rx);

        assert!(!ctx.is_cancelled(), "初始状态不应被取消");

        tx.send(true).expect("发送取消信号Failed");
        assert!(ctx.is_cancelled(), "收到信号后应返回 true");
    }

    /// 测试 ExecutionEvent::FlowCancelled 序列化
    #[test]
    fn test_flow_cancelled_serialization() {
        let event = ExecutionEvent::FlowCancelled {
            flow_uuid: "flow-001".to_string(),
            reason: "用户取消".to_string(),
        };

        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("\"flowCancelled\""), "变体名应为 flowCancelled");
        assert!(json.contains("\"flowUuid\""), "字段名应为 flowUuid");
        assert!(json.contains("\"reason\""), "字段名应为 reason");

        // 反序列化往返测试
        let deserialized: ExecutionEvent = serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(json, serde_json::to_string(&deserialized).unwrap());
    }

    // ===== Provider 注册表测试 =====

    /// 测试 set_provider + get_provider 往返
    #[test]
    fn test_provider_set_get_roundtrip() {
        // 使用简单的 i32 类型测试 provider 存取
        let value: Arc<i32> = Arc::new(42);
        let mut ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        ctx.set_provider("test", value.clone() as Arc<dyn Any + Send + Sync>);

        let retrieved: Option<Arc<i32>> = ctx.get_provider("test");
        assert!(retrieved.is_some(), "应该能获取到 provider");
        assert_eq!(*retrieved.unwrap(), 42, "值应该匹配");
    }

    /// 测试 get_provider 对不存在的 key 返回 None
    #[test]
    fn test_provider_get_missing_key() {
        let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        let result: Option<Arc<i32>> = ctx.get_provider("nonexistent");
        assert!(result.is_none(), "不存在的 key 应返回 None");
    }

    /// 测试 get_provider 对Error类型返回 None
    #[test]
    fn test_provider_get_wrong_type() {
        let value: Arc<String> = Arc::new("hello".to_string());
        let mut ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        ctx.set_provider("test", value as Arc<dyn Any + Send + Sync>);

        // 尝试用Error的类型获取
        let result: Option<Arc<i32>> = ctx.get_provider("test");
        assert!(result.is_none(), "类型不匹配应返回 None");
    }

    /// 测试 providers 在序列化时被跳过
    #[test]
    fn test_provider_skipped_in_serialization() {
        let mut ctx = ExecutionContext::new("test-flow".to_string(), 1000);

        // 添加 provider
        let value: Arc<i32> = Arc::new(42);
        ctx.set_provider("test", value as Arc<dyn Any + Send + Sync>);

        // 序列化
        let json = serde_json::to_string(&ctx).expect("序列化Failed");

        // providers 不应出现在 JSON 中
        assert!(!json.contains("providers"), "providers 不应出现在序列化结果中");

        // 反序列化后 providers 应为空
        let deserialized: ExecutionContext = serde_json::from_str(&json).expect("反序列化Failed");
        assert!(
            deserialized.providers.is_empty(),
            "反序列化后 providers 应为空"
        );
    }
}
