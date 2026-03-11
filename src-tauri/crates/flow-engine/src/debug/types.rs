//! 调试模式类型定义
//!
//! 包含调试会话、命令、事件等核心数据结构

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
        /// 新的节点配置（JSON 格式的 NodeData）
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
        /// 节点 UUID
        node_uuid: String,
        /// 节点输出
        output: Option<serde_json::Value>,
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
    /// 解析后的配置（与 NodeExecutionReport 对齐）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_config: Option<serde_json::Value>,
    /// 当前节点修改变量的记录（与 NodeExecutionReport 对齐）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_changes: Option<Vec<crate::report::VariableChange>>,
    /// 跳过原因（与 NodeExecutionReport 对齐）
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
    /// 临时节点配置覆盖（节点 UUID -> NodeData JSON）
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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_debug_status_serialization() {
        let status = DebugStatus::Paused;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"paused\"");

        let deserialized: DebugStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, DebugStatus::Paused);
    }

    #[test]
    fn test_pause_reason_serialization() {
        let reason = PauseReason::Breakpoint {
            node_uuid: "node-123".to_string(),
        };
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("breakpoint"));
        assert!(json.contains("node-123"));

        let deserialized: PauseReason = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, reason);
    }

    #[test]
    fn test_end_reason_serialization() {
        let reason = EndReason::Completed;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "{\"type\":\"completed\"}");

        let deserialized: EndReason = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EndReason::Completed);
    }

    #[test]
    fn test_debug_command_serialization() {
        let cmd = DebugCommand::StartDebug {
            flow_uuid: "flow-123".to_string(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        println!("DebugCommand JSON: {}", json);
        assert!(
            json.contains("startDebug")
                || json.contains("start_debug")
                || json.contains("StartDebug")
        );
        assert!(json.contains("flow-123"));

        let deserialized: DebugCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, cmd);
    }

    #[test]
    fn test_debug_event_serialization() {
        let event = DebugEvent::SessionStarted {
            flow_uuid: "flow-456".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        println!("DebugEvent JSON: {}", json);
        assert!(
            json.contains("sessionStarted")
                || json.contains("session_started")
                || json.contains("SessionStarted")
        );
        assert!(json.contains("flow-456"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event);
    }

    #[test]
    fn test_execution_stack_entry() {
        let entry = ExecutionStackEntry {
            node_uuid: "node-001".to_string(),
            node_name: "HTTP Request".to_string(),
            node_type: "http".to_string(),
            started_at: 1234567890,
            ended_at: Some(1234567895),
            input: Some(json!({"url": "https://example.com"})),
            output: Some(json!({"status": 200})),
            success: true,
            error: None,
            used_port_id: Some("out".to_string()),
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("node-001"));
        assert!(json.contains("HTTP Request"));

        let deserialized: ExecutionStackEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.node_uuid, "node-001");
        assert_eq!(deserialized.success, true);
    }

    #[test]
    fn test_debug_session_state_creation() {
        let state = DebugSessionState::new("session-001".to_string(), "flow-001".to_string());

        assert_eq!(state.session_id, "session-001");
        assert_eq!(state.flow_uuid, "flow-001");
        assert_eq!(state.status, DebugStatus::Idle);
        assert!(state.current_node_uuid.is_none());
        assert!(state.variables.is_empty());
        assert!(state.execution_stack.is_empty());
    }

    #[test]
    fn test_debug_session_state_breakpoints() {
        let mut state = DebugSessionState::new("session-001".to_string(), "flow-001".to_string());

        state.set_breakpoints(vec!["node-001".to_string(), "node-003".to_string()]);

        assert!(state.has_breakpoint("node-001"));
        assert!(!state.has_breakpoint("node-002"));
        assert!(state.has_breakpoint("node-003"));
    }

    #[test]
    fn test_debug_session_state_variables() {
        let mut state = DebugSessionState::new("session-001".to_string(), "flow-001".to_string());

        state.set_variable("status".to_string(), json!(200));
        state.set_variable("body".to_string(), json!({"data": "test"}));

        assert_eq!(state.get_variable("status"), Some(&json!(200)));
        assert_eq!(state.get_variable("body"), Some(&json!({"data": "test"})));
        assert_eq!(state.get_variable("nonexistent"), None);
    }

    #[test]
    fn test_debug_session_state_stack() {
        let mut state = DebugSessionState::new("session-001".to_string(), "flow-001".to_string());

        let entry1 = ExecutionStackEntry {
            node_uuid: "node-001".to_string(),
            node_name: "Entry".to_string(),
            node_type: "entry".to_string(),
            started_at: 1000,
            ended_at: Some(1005),
            input: None,
            output: None,
            success: true,
            error: None,
            used_port_id: Some("out".to_string()),
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
        };

        state.push_stack_entry(entry1);
        assert_eq!(state.execution_stack.len(), 1);

        state.update_last_stack_entry(
            1010,
            Some(json!({"result": "ok"})),
            true,
            None,
            Some("out".to_string()),
            None,
            None,
            None,
        );
        assert_eq!(state.execution_stack[0].ended_at, Some(1010));
        assert_eq!(
            state.execution_stack[0].output,
            Some(json!({"result": "ok"}))
        );
    }

    #[test]
    fn test_all_debug_status_variants() {
        let statuses = vec![
            DebugStatus::Idle,
            DebugStatus::Running,
            DebugStatus::Paused,
            DebugStatus::Ended,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: DebugStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_all_pause_reason_variants() {
        let reasons = vec![
            PauseReason::Breakpoint {
                node_uuid: "node-001".to_string(),
            },
            PauseReason::StepComplete {
                node_uuid: "node-002".to_string(),
            },
            PauseReason::Error {
                message: "Test error".to_string(),
            },
        ];

        for reason in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            let deserialized: PauseReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, deserialized);
        }
    }

    #[test]
    fn test_all_end_reason_variants() {
        let reasons = vec![
            EndReason::Completed,
            EndReason::UserAbort,
            EndReason::Error {
                message: "Test error".to_string(),
            },
        ];

        for reason in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            let deserialized: EndReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, deserialized);
        }
    }

    #[test]
    fn test_all_debug_command_variants() {
        let commands = vec![
            DebugCommand::StartDebug {
                flow_uuid: "flow-001".to_string(),
            },
            DebugCommand::Continue,
            DebugCommand::StepOver,
            DebugCommand::SkipNode {
                selected_port: Some("true".to_string()),
                simulated_output: Some(json!({"status": 200})),
            },
            DebugCommand::RunToEnd,
            DebugCommand::EndDebug,
            DebugCommand::ModifyVariable {
                name: "status".to_string(),
                value: json!(404),
            },
            DebugCommand::ModifyNodeConfig {
                node_uuid: "node-001".to_string(),
                config: json!({"type": "http", "url": "https://example.com", "method": "get"}),
            },
            DebugCommand::SetBreakpoint {
                node_uuid: "node-002".to_string(),
                enabled: true,
            },
        ];

        for cmd in commands {
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: DebugCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }
    }

    #[test]
    fn test_all_debug_event_variants() {
        let events = vec![
            DebugEvent::SessionStarted {
                flow_uuid: "flow-001".to_string(),
            },
            DebugEvent::PausedAtNode {
                node_uuid: "node-001".to_string(),
                reason: PauseReason::Breakpoint {
                    node_uuid: "node-001".to_string(),
                },
                variables: HashMap::new(),
                execution_stack: vec![],
            },
            DebugEvent::NodeExecuted {
                node_uuid: "node-002".to_string(),
                output: Some(json!({"status": 200})),
            },
            DebugEvent::SessionEnded {
                reason: EndReason::Completed,
                report: None,
                execution_stack: vec![],
            },
            DebugEvent::VariableModified {
                name: "status".to_string(),
                value: json!(200),
            },
            DebugEvent::NodeConfigModified {
                node_uuid: "node-001".to_string(),
                config: json!({"type": "http", "url": "https://example.com", "method": "get"}),
            },
            DebugEvent::BreakpointModified {
                node_uuid: "node-002".to_string(),
                enabled: true,
            },
            DebugEvent::Error {
                message: "Test error".to_string(),
            },
            DebugEvent::Heartbeat {
                status: DebugStatus::Running,
                current_node_uuid: Some("node-001".to_string()),
                timestamp: 1234567890,
            },
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, deserialized);
        }
    }

    #[test]
    fn test_debug_session_state_dynamic_breakpoints() {
        let mut state = DebugSessionState::new("session-001".to_string(), "flow-001".to_string());

        // 初始状态没有断点
        assert!(!state.has_breakpoint("node-001"));
        assert!(!state.has_breakpoint("node-002"));

        // 添加断点
        state.add_breakpoint("node-001".to_string());
        assert!(state.has_breakpoint("node-001"));
        assert!(!state.has_breakpoint("node-002"));

        // 添加另一个断点
        state.add_breakpoint("node-002".to_string());
        assert!(state.has_breakpoint("node-001"));
        assert!(state.has_breakpoint("node-002"));

        // 重复添加不会重复
        state.add_breakpoint("node-001".to_string());
        assert_eq!(state.breakpoints.len(), 2);

        // 移除断点
        state.remove_breakpoint("node-001");
        assert!(!state.has_breakpoint("node-001"));
        assert!(state.has_breakpoint("node-002"));

        // 移除不存在的断点不会出错
        state.remove_breakpoint("node-003");
        assert_eq!(state.breakpoints.len(), 1);

        // 再次移除已移除的断点
        state.remove_breakpoint("node-001");
        assert_eq!(state.breakpoints.len(), 1);
    }

    #[test]
    fn test_ws_stream_created_event() {
        let event = DebugEvent::WsStreamCreated {
            connection_id: "conn-001".to_string(),
            url: "wss://example.com/ws".to_string(),
            owner_node_uuid: "node-001".to_string(),
            owner_node_name: "WS Connect".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        println!("WsStreamCreated JSON: {}", json);
        assert!(
            json.contains("ws_stream_created")
                || json.contains("WsStreamCreated")
                || json.contains("wsStreamCreated")
        );
        assert!(json.contains("conn-001"));
        assert!(json.contains("wss://example.com/ws"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_ws_message_received_event() {
        use crate::ws_stream::{MessageContent, MessageDirection};

        let msg = WsMessage::with_timestamp(
            MessageDirection::Received,
            MessageContent::Text {
                content: "hello".to_string(),
            },
            1234567890,
        );

        let event = DebugEvent::WsMessageReceived {
            connection_id: "conn-001".to_string(),
            owner_node_uuid: "node-001".to_string(),
            message: msg.clone(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("ws_message_received")
                || json.contains("WsMessageReceived")
                || json.contains("wsMessageReceived")
        );
        assert!(json.contains("conn-001"));
        assert!(json.contains("hello"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_ws_message_sent_event() {
        use crate::ws_stream::{MessageContent, MessageDirection};

        let msg = WsMessage::with_timestamp(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "ping".to_string(),
            },
            1234567890,
        );

        let event = DebugEvent::WsMessageSent {
            connection_id: "conn-001".to_string(),
            owner_node_uuid: "node-002".to_string(),
            message: msg.clone(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("ws_message_sent")
                || json.contains("WsMessageSent")
                || json.contains("wsMessageSent")
        );
        assert!(json.contains("conn-001"));
        assert!(json.contains("ping"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_ws_stream_owner_changed_event() {
        let event = DebugEvent::WsStreamOwnerChanged {
            connection_id: "conn-001".to_string(),
            from_node_uuid: "node-001".to_string(),
            to_node_uuid: "node-002".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("ws_stream_owner_changed")
                || json.contains("WsStreamOwnerChanged")
                || json.contains("wsStreamOwnerChanged")
        );
        assert!(json.contains("conn-001"));
        assert!(json.contains("node-001"));
        assert!(json.contains("node-002"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_ws_stream_closed_event() {
        let event = DebugEvent::WsStreamClosed {
            connection_id: "conn-001".to_string(),
            last_owner_node_uuid: "node-003".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("ws_stream_closed")
                || json.contains("WsStreamClosed")
                || json.contains("wsStreamClosed")
        );
        assert!(json.contains("conn-001"));
        assert!(json.contains("node-003"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_all_ws_debug_event_variants() {
        use crate::ws_stream::{MessageContent, MessageDirection};

        let events = vec![
            DebugEvent::WsStreamCreated {
                connection_id: "conn-001".to_string(),
                url: "wss://example.com/ws".to_string(),
                owner_node_uuid: "node-001".to_string(),
                owner_node_name: "Connect".to_string(),
            },
            DebugEvent::WsMessageReceived {
                connection_id: "conn-001".to_string(),
                owner_node_uuid: "node-002".to_string(),
                message: WsMessage::with_timestamp(
                    MessageDirection::Received,
                    MessageContent::Text {
                        content: "response".to_string(),
                    },
                    1234567890,
                ),
            },
            DebugEvent::WsMessageSent {
                connection_id: "conn-001".to_string(),
                owner_node_uuid: "node-002".to_string(),
                message: WsMessage::with_timestamp(
                    MessageDirection::Sent,
                    MessageContent::Text {
                        content: "request".to_string(),
                    },
                    1234567880,
                ),
            },
            DebugEvent::WsStreamOwnerChanged {
                connection_id: "conn-001".to_string(),
                from_node_uuid: "node-001".to_string(),
                to_node_uuid: "node-002".to_string(),
            },
            DebugEvent::WsStreamClosed {
                connection_id: "conn-001".to_string(),
                last_owner_node_uuid: "node-003".to_string(),
            },
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, deserialized);
        }
    }

    #[test]
    fn test_assertion_started_event() {
        let event = DebugEvent::AssertionStarted {
            node_uuid: "node-001".to_string(),
            node_name: "断言节点".to_string(),
            total_assertions: 3,
            mode: "all".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("assertion_started")
                || json.contains("AssertionStarted")
                || json.contains("assertionStarted")
        );
        assert!(json.contains("node-001"));
        assert!(json.contains("断言节点"));
        assert!(json.contains("3"));
        assert!(json.contains("all"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_assertion_completed_event() {
        use crate::node_config::AssertionResult;

        let failed_assertions = vec![AssertionResult {
            id: "assert-1".to_string(),
            description: Some("状态码检查".to_string()),
            passed: false,
            target: "$.status".to_string(),
            operator: "eq".to_string(),
            expected: json!(200),
            actual: json!(404),
            error: None,
        }];

        let event = DebugEvent::AssertionCompleted {
            node_uuid: "node-001".to_string(),
            node_name: "断言节点".to_string(),
            passed: false,
            passed_count: 2,
            failed_count: 1,
            total: 3,
            failed_assertions,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(
            json.contains("assertion_completed")
                || json.contains("AssertionCompleted")
                || json.contains("assertionCompleted")
        );
        assert!(json.contains("node-001"));
        assert!(json.contains("断言节点"));
        assert!(json.contains("false"));
        assert!(json.contains("2"));
        assert!(json.contains("1"));
        assert!(json.contains("3"));

        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_assertion_events_pass_scenario() {
        use crate::node_config::AssertionResult;

        // 测试通过场景
        let started = DebugEvent::AssertionStarted {
            node_uuid: "node-001".to_string(),
            node_name: "状态检查".to_string(),
            total_assertions: 2,
            mode: "all".to_string(),
        };

        let completed = DebugEvent::AssertionCompleted {
            node_uuid: "node-001".to_string(),
            node_name: "状态检查".to_string(),
            passed: true,
            passed_count: 2,
            failed_count: 0,
            total: 2,
            failed_assertions: vec![],
        };

        // 序列化和反序列化
        let started_json = serde_json::to_string(&started).unwrap();
        let completed_json = serde_json::to_string(&completed).unwrap();

        let started_de: DebugEvent = serde_json::from_str(&started_json).unwrap();
        let completed_de: DebugEvent = serde_json::from_str(&completed_json).unwrap();

        assert_eq!(started, started_de);
        assert_eq!(completed, completed_de);

        // 验证 completed 事件的 passed 字段
        if let DebugEvent::AssertionCompleted { passed, .. } = completed_de {
            assert!(passed);
        } else {
            panic!("Expected AssertionCompleted event");
        }
    }

    #[test]
    fn test_assertion_events_fail_scenario() {
        use crate::node_config::AssertionResult;

        // 测试失败场景
        let failed_assertions = vec![
            AssertionResult {
                id: "assert-1".to_string(),
                description: Some("状态码检查".to_string()),
                passed: false,
                target: "$.status".to_string(),
                operator: "eq".to_string(),
                expected: json!(200),
                actual: json!(500),
                error: None,
            },
            AssertionResult {
                id: "assert-2".to_string(),
                description: Some("响应体检查".to_string()),
                passed: false,
                target: "$.body.success".to_string(),
                operator: "eq".to_string(),
                expected: json!(true),
                actual: json!(false),
                error: None,
            },
        ];

        let completed = DebugEvent::AssertionCompleted {
            node_uuid: "node-001".to_string(),
            node_name: "API 断言".to_string(),
            passed: false,
            passed_count: 0,
            failed_count: 2,
            total: 2,
            failed_assertions,
        };

        let json = serde_json::to_string(&completed).unwrap();
        let deserialized: DebugEvent = serde_json::from_str(&json).unwrap();

        if let DebugEvent::AssertionCompleted {
            passed,
            failed_count,
            failed_assertions,
            ..
        } = deserialized
        {
            assert!(!passed);
            assert_eq!(failed_count, 2);
            assert_eq!(failed_assertions.len(), 2);
        } else {
            panic!("Expected AssertionCompleted event");
        }
    }
}
