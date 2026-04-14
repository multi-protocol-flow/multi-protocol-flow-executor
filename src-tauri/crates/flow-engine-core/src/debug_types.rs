//! 调试模式类型定义
//!
//! 包含调试会话、命令、事件等核心数据结构
//! 从 flow-engine/src/debug/types.rs 迁移至 core crate

use crate::report::FlowExecutionReport;
use crate::ws_stream::WsMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// 调试状态枚举
// ============================================================================

/// 调试会话状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DebugStatus {
    /// 空闲状态（无调试会话）
    Idle,
    /// 运行中（正在执行节点）
    Running,
    /// 已暂停（在断点或单步处）
    Paused,
    /// 已结束（调试完成或终止）
    Ended,
}

// ============================================================================
// 暂停原因枚举
// ============================================================================

/// 暂停原因
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PauseReason {
    /// 到达断点
    Breakpoint {
        /// 断点所在节点 UUID
        node_uuid: String,
    },
    /// 单步执行完成
    StepComplete {
        /// 当前节点 UUID
        node_uuid: String,
    },
    /// 执行出错
    Error {
        /// 错误信息
        message: String,
    },
}

// ============================================================================
// 结束原因枚举
// ============================================================================

/// 调试结束原因
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EndReason {
    /// 流程执行完成
    Completed,
    /// 用户终止调试
    UserAbort,
    /// 执行出错
    Error {
        /// 错误信息
        message: String,
    },
}

// ============================================================================
// 调试命令枚举
// ============================================================================

/// 调试命令（从前端发送到后端）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebugCommand {
    /// 开始调试
    StartDebug {
        /// 流程 UUID
        flow_uuid: String,
    },
    /// 继续执行（到下一个断点）
    Continue,
    /// 单步执行
    StepOver,
    /// 跳过当前节点
    SkipNode {
        /// 选择的输出分支（用于条件/HTTP/WebSocket 节点）
        selected_port: Option<String>,
        /// 模拟输出（用于 HTTP/WebSocket 节点）
        simulated_output: Option<serde_json::Value>,
    },
    /// 执行到底（忽略后续断点）
    RunToEnd,
    /// 结束调试
    EndDebug,
    /// 修改变量
    ModifyVariable {
        /// 变量名
        name: String,
        /// 新值
        value: serde_json::Value,
    },
    /// 修改节点配置（临时，仅调试期间生效）
    ModifyNodeConfig {
        /// 节点 UUID
        node_uuid: String,
        /// 新的节点配置（JSON 格式的 `NodeData`）
        config: serde_json::Value,
    },
    /// 动态设置断点（调试期间生效）
    SetBreakpoint {
        /// 节点 UUID
        node_uuid: String,
        /// 是否启用断点
        enabled: bool,
    },
}

// ============================================================================
// 调试事件枚举
// ============================================================================

/// 调试事件（从后端发送到前端）
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebugEvent {
    /// 调试会话已启动
    SessionStarted {
        /// 流程 UUID
        flow_uuid: String,
    },
    /// 在节点处暂停
    PausedAtNode {
        /// 当前节点 UUID
        node_uuid: String,
        /// 暂停原因
        reason: PauseReason,
        /// 当前上下文变量
        variables: HashMap<String, serde_json::Value>,
        /// 执行堆栈（已执行节点列表）
        execution_stack: Vec<ExecutionStackEntry>,
    },
    /// 节点执行完成
    NodeExecuted {
        /// 已完成的执行堆栈条目（包含完整的执行信息）
        entry: ExecutionStackEntry,
    },
    /// 调试会话已结束
    SessionEnded {
        /// 结束原因
        reason: EndReason,
        /// 执行报告（调试结束时生成）
        report: Option<FlowExecutionReport>,
        /// 执行堆栈（包含所有已执行节点）
        execution_stack: Vec<ExecutionStackEntry>,
    },
    /// 变量已修改
    VariableModified {
        /// 变量名
        name: String,
        /// 新值
        value: serde_json::Value,
    },
    /// 节点配置已修改（临时）
    NodeConfigModified {
        /// 节点 UUID
        node_uuid: String,
        /// 新的节点配置
        config: serde_json::Value,
    },
    /// 断点已修改（动态设置）
    BreakpointModified {
        /// 节点 UUID
        node_uuid: String,
        /// 是否启用断点
        enabled: bool,
    },
    /// 调试错误
    Error {
        /// 错误信息
        message: String,
    },
    /// 心跳事件（定期发送，用于检测会话是否存活）
    Heartbeat {
        /// 当前调试状态
        status: DebugStatus,
        /// 当前节点 UUID（如果正在执行或暂停）
        current_node_uuid: Option<String>,
        /// 时间戳（Unix 时间戳，毫秒）
        timestamp: u64,
    },
    /// WebSocket 流已创建
    WsStreamCreated {
        /// 连接 ID
        connection_id: String,
        /// WebSocket URL
        url: String,
        /// 所有者节点 UUID
        owner_node_uuid: String,
        /// 所有者节点名称
        owner_node_name: String,
    },
    /// WebSocket 消息已接收
    WsMessageReceived {
        /// 连接 ID
        connection_id: String,
        /// 所有者节点 UUID
        owner_node_uuid: String,
        /// 消息内容
        #[serde(flatten)]
        message: WsMessage,
    },
    /// WebSocket 消息已发送
    WsMessageSent {
        /// 连接 ID
        connection_id: String,
        /// 所有者节点 UUID
        owner_node_uuid: String,
        /// 消息内容
        #[serde(flatten)]
        message: WsMessage,
    },
    /// WebSocket 流所有权已转移
    WsStreamOwnerChanged {
        /// 连接 ID
        connection_id: String,
        /// 原所有者节点 UUID
        from_node_uuid: String,
        /// 新所有者节点 UUID
        to_node_uuid: String,
    },
    /// WebSocket 流已关闭
    WsStreamClosed {
        /// 连接 ID
        connection_id: String,
        /// 最后所有者节点 UUID
        last_owner_node_uuid: String,
    },
    /// 断言节点开始执行
    AssertionStarted {
        /// 节点 UUID
        node_uuid: String,
        /// 节点名称
        node_name: String,
        /// 断言数量
        total_assertions: usize,
        /// 断言模式 (all/any)
        mode: String,
    },
    /// 断言节点执行完成
    AssertionCompleted {
        /// 节点 UUID
        node_uuid: String,
        /// 节点名称
        node_name: String,
        /// 整体是否通过
        passed: bool,
        /// 通过数量
        passed_count: usize,
        /// 失败数量
        failed_count: usize,
        /// 总数量
        total: usize,
        /// 失败的断言详情
        failed_assertions: Vec<crate::node_config::AssertionResult>,
    },
    // ===== 数据库调试事件 =====
    /// 数据库连接已创建
    DbConnectionCreated {
        /// 连接 UUID
        connection_uuid: String,
        /// 数据库类型（postgres/mysql/sqlite/redis）
        db_type: String,
        /// 所有者节点 UUID
        node_uuid: String,
        /// 所有者节点名称
        node_name: String,
    },
    /// 数据库连接已关闭
    DbConnectionClosed {
        /// 连接 UUID
        connection_uuid: String,
        /// 最后所有者节点 UUID
        last_node_uuid: String,
    },
    /// 数据库事务已开始
    DbTransactionStarted {
        /// 连接 UUID
        connection_uuid: String,
        /// 所有者节点 UUID
        node_uuid: String,
        /// 所有者节点名称
        node_name: String,
        /// 隔离级别（可选）
        isolation_level: Option<String>,
    },
    /// 数据库事务已提交
    DbTransactionCommitted {
        /// 连接 UUID
        connection_uuid: String,
        /// 所有者节点 UUID
        node_uuid: String,
        /// 所有者节点名称
        node_name: String,
    },
    /// 数据库事务已回滚
    DbTransactionRolledback {
        /// 连接 UUID
        connection_uuid: String,
        /// 所有者节点 UUID
        node_uuid: String,
        /// 所有者节点名称
        node_name: String,
        /// 回滚原因（可选：用户手动/调试终止）
        reason: Option<String>,
    },
    /// 数据库查询已执行
    DbQueryExecuted {
        /// 连接 UUID
        connection_uuid: String,
        /// 所有者节点 UUID
        node_uuid: String,
        /// 所有者节点名称
        node_name: String,
        /// SQL 语句（截断到前 100 字符）
        sql: String,
        /// 返回行数
        row_count: usize,
        /// 执行时间（毫秒）
        execute_ms: u64,
        /// 是否成功
        success: bool,
    },
}

// ============================================================================
// 执行堆栈条目
// ============================================================================

/// 执行堆栈条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStackEntry {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称
    pub node_name: String,
    /// 节点类型
    pub node_type: String,
    /// 执行开始时间（Unix 时间戳，毫秒）
    pub started_at: u64,
    /// 执行结束时间（Unix 时间戳，毫秒）
    pub ended_at: Option<u64>,
    /// 输入数据
    pub input: Option<serde_json::Value>,
    /// 输出数据
    pub output: Option<serde_json::Value>,
    /// 是否执行成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 实际使用的输出端口 ID（可选）
    #[serde(default)]
    pub used_port_id: Option<String>,
    /// 解析后的配置（与 `NodeExecutionReport` 对齐）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_config: Option<serde_json::Value>,
    /// 当前节点修改变量的记录（与 `NodeExecutionReport` 对齐）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_changes: Option<Vec<crate::report::VariableChange>>,
    /// 跳过原因（与 `NodeExecutionReport` 对齐）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

// ============================================================================
// 调试会话状态
// ============================================================================

/// 调试会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSessionState {
    /// 会话 ID
    pub session_id: String,
    /// 流程 UUID
    pub flow_uuid: String,
    /// 当前调试状态
    pub status: DebugStatus,
    /// 当前暂停的节点 UUID（如果暂停）
    pub current_node_uuid: Option<String>,
    /// 当前暂停原因（如果暂停）
    pub pause_reason: Option<PauseReason>,
    /// 当前上下文变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 执行堆栈
    pub execution_stack: Vec<ExecutionStackEntry>,
    /// 断点列表（节点 UUID）
    pub breakpoints: Vec<String>,
    /// 是否处于单步模式
    pub is_stepping: bool,
    /// 是否跳过后续断点（执行到底模式）
    pub skip_breakpoints: bool,
    /// 临时节点配置覆盖（节点 UUID -> `NodeData` JSON）
    /// 仅调试期间生效，调试结束后丢弃
    #[serde(default)]
    pub temp_node_configs: HashMap<String, serde_json::Value>,
    /// 调试开始时间（毫秒时间戳）
    #[serde(default)]
    pub start_time_ms: u64,
}

impl Default for DebugSessionState {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            flow_uuid: String::new(),
            status: DebugStatus::Idle,
            current_node_uuid: None,
            pause_reason: None,
            variables: HashMap::new(),
            execution_stack: Vec::new(),
            breakpoints: Vec::new(),
            is_stepping: false,
            skip_breakpoints: false,
            temp_node_configs: HashMap::new(),
            start_time_ms: 0,
        }
    }
}

impl DebugSessionState {
    /// 创建新的调试会话
    pub fn new(session_id: String, flow_uuid: String) -> Self {
        Self {
            session_id,
            flow_uuid,
            ..Default::default()
        }
    }

    /// 设置断点列表
    pub fn set_breakpoints(&mut self, breakpoints: Vec<String>) {
        self.breakpoints = breakpoints;
    }

    /// 检查节点是否有断点
    pub fn has_breakpoint(&self, node_uuid: &str) -> bool {
        self.breakpoints.contains(&node_uuid.to_string())
    }

    /// 添加断点
    pub fn add_breakpoint(&mut self, node_uuid: String) {
        if !self.breakpoints.contains(&node_uuid) {
            self.breakpoints.push(node_uuid);
        }
    }

    /// 移除断点
    pub fn remove_breakpoint(&mut self, node_uuid: &str) {
        self.breakpoints.retain(|bp| bp != node_uuid);
    }

    /// 添加执行堆栈条目
    pub fn push_stack_entry(&mut self, entry: ExecutionStackEntry) {
        self.execution_stack.push(entry);
    }

    /// 更新最后一个堆栈条目的结束信息
    #[allow(clippy::too_many_arguments)]
    pub fn update_last_stack_entry(
        &mut self,
        ended_at: u64,
        output: Option<serde_json::Value>,
        success: bool,
        error: Option<String>,
        used_port_id: Option<String>,
        resolved_config: Option<serde_json::Value>,
        variable_changes: Option<Vec<crate::report::VariableChange>>,
        skip_reason: Option<String>,
    ) {
        if let Some(last_entry) = self.execution_stack.last_mut() {
            last_entry.ended_at = Some(ended_at);
            last_entry.output = output;
            last_entry.success = success;
            last_entry.error = error;
            last_entry.used_port_id = used_port_id;
            last_entry.resolved_config = resolved_config;
            last_entry.variable_changes = variable_changes;
            last_entry.skip_reason = skip_reason;
        }
    }

    /// 设置变量
    pub fn set_variable(&mut self, name: String, value: serde_json::Value) {
        self.variables.insert(name, value);
    }

    /// 获取变量
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// 设置临时节点配置
    pub fn set_temp_node_config(&mut self, node_uuid: String, config: serde_json::Value) {
        self.temp_node_configs.insert(node_uuid, config);
    }

    /// 获取临时节点配置
    pub fn get_temp_node_config(&self, node_uuid: &str) -> Option<&serde_json::Value> {
        self.temp_node_configs.get(node_uuid)
    }

    /// 清除所有临时节点配置
    pub fn clear_temp_node_configs(&mut self) {
        self.temp_node_configs.clear();
    }
}
