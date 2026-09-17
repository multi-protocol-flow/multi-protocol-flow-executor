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
    /// 单步execution complete
    StepComplete {
        /// 当前节点 UUID
        node_uuid: String,
    },
    /// 执行出错
    Error {
        /// Error信息
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
    /// 流程execution complete
    Completed,
    /// 用户终止调试
    UserAbort,
    /// 执行出错
    Error {
        /// Error信息
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
    /// 单步步入（遇到 `flow:sub` 节点进入其内部首节点暂停；普通节点行为同 `StepOver`）
    StepInto,
    /// 单步步出（执行完当前子流程剩余节点，在父流程该 `flow:sub` 节点的后继节点暂停）
    StepOut,
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
    /// 原地重试当前暂停节点（使用修改后的配置或变量重新执行）
    RetryCurrentNode,
    /// 从指定历史节点快照恢复并继续暂停/执行
    RunFromSnapshot {
        /// 目标节点 UUID
        node_uuid: String,
    },
    /// 向指定长连接发送调试帧
    SendStreamFrame {
        /// 连接 ID
        connection_id: String,
        /// 载荷内容
        payload: String,
        /// 是否为二进制消息（Base64 编码）
        is_binary: bool,
    },
}
/// 调试调用栈帧（用于多层子流程嵌套调试）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugCallFrame {
    /// 栈帧 ID（0 为最顶层根流程，越深越大）
    pub frame_id: usize,
    /// 当前流程 UUID
    pub flow_uuid: String,
    /// 当前流程名称
    pub flow_name: String,
    /// 当前暂停/执行节点 UUID
    pub node_uuid: String,
    /// 当前暂停/执行节点名称
    pub node_name: String,
    /// 父流程中的 `SubFlow` 节点 UUID（顶层根流程为 None）
    pub parent_subflow_node_uuid: Option<String>,
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
        /// 多层子流程嵌套调用栈帧
        #[serde(default)]
        call_frames: Vec<DebugCallFrame>,
    },
    /// 节点execution complete
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
    /// 调试Error
    Error {
        /// Error信息
        message: String,
    },
    /// 快照已恢复事件
    SnapshotRestored {
        /// 恢复的目标节点 UUID
        node_uuid: String,
        /// 恢复后的变量表
        restored_variables: HashMap<String, serde_json::Value>,
        /// 截断后的堆栈长度
        stack_len: usize,
    },
    /// 探针发送帧完成事件
    StreamFrameSent {
        /// 连接 ID
        connection_id: String,
        /// 发送时间戳（毫秒）
        timestamp_ms: u64,
        /// 发送的载荷内容
        payload: String,
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
    /// 断言节点start execution
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
    /// 断言节点execution complete
    AssertionCompleted {
        /// 节点 UUID
        node_uuid: String,
        /// 节点名称
        node_name: String,
        /// 整体是否通过
        passed: bool,
        /// 通过数量
        passed_count: usize,
        /// Failed数量
        failed_count: usize,
        /// 总数量
        total: usize,
        /// Failed的断言详情
        failed_assertions: Vec<crate::node_config::AssertionResult>,
    },
    // ===== gRPC 调试事件 =====
    /// gRPC 通道已创建
    GrpcChannelCreated {
        /// 节点 UUID
        node_uuid: String,
        /// 连接 ID
        connection_id: String,
        /// gRPC 服务器 URL
        url: String,
        /// 发现的服务数量
        service_count: usize,
    },
    /// gRPC 方法已调用
    GrpcMethodCalled {
        /// 节点 UUID
        node_uuid: String,
        /// 连接 ID
        connection_id: String,
        /// 服务名称
        service: String,
        /// 方法名称
        method: String,
        /// 通信模式（`unary`/`server_streaming`/`client_streaming`/`bidi_streaming`）
        pattern: String,
        /// 调用耗时（毫秒）
        duration_ms: u64,
    },
    /// gRPC 通道已关闭
    GrpcChannelClosed {
        /// 节点 UUID
        node_uuid: String,
        /// 连接 ID
        connection_id: String,
        /// gRPC 服务器 URL
        url: String,
    },
    /// gRPC 流式消息已接收（实时推送，supplementary）
    GrpcStreamMessage {
        /// 节点 UUID
        node_uuid: String,
        /// 连接 ID
        connection_id: String,
        /// 调用 ID（用于区分同一连接的多次调用）
        call_id: String,
        /// 消息序号（从 1 开始）
        index: u32,
        /// 消息数据（JSON 字符串）
        data: String,
        /// 接收时间戳（相对于调用开始的毫秒数）
        received_at_ms: u64,
        /// 消息大小（字节）
        size_bytes: u64,
        /// 是否为最后一条消息
        is_last: bool,
    },
    /// gRPC 流式Error（实时推送，supplementary）
    GrpcStreamError {
        /// 节点 UUID
        node_uuid: String,
        /// 连接 ID
        connection_id: String,
        /// 调用 ID
        call_id: String,
        /// Error信息
        message: String,
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
    /// 是否执行Success
    pub success: bool,
    /// Error信息（如果Failed）
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
    /// 子流程嵌套执行报告树（与 `NodeExecutionReport` 对齐）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_flow_report: Option<Box<crate::report::FlowExecutionReport>>,
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
    /// 多层子流程嵌套调用栈帧
    #[serde(default)]
    pub call_frames: Vec<DebugCallFrame>,
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
            call_frames: Vec::new(),
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

    /// 记录最新堆栈条目的子流程嵌套报告（SubFlow 节点专用）
    ///
    /// 与 `NodeExecutionReport.sub_flow_report` 对齐：调试模式下子流程同样
    /// 作为单个节点执行，其内部执行报告树由此挂载，供报告视图展示。
    pub fn set_last_stack_entry_sub_flow_report(
        &mut self,
        sub_flow_report: Option<Box<crate::report::FlowExecutionReport>>,
    ) {
        if let Some(last_entry) = self.execution_stack.last_mut() {
            last_entry.sub_flow_report = sub_flow_report;
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== gRPC 调试事件序列化测试 =====

    #[test]
    fn test_grpc_channel_created_event() {
        let event = DebugEvent::GrpcChannelCreated {
            node_uuid: "node-001".to_string(),
            connection_id: "conn-grpc-1".to_string(),
            url: "http://localhost:50051".to_string(),
            service_count: 3,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("GrpcChannelCreated"), "应包含变体名");
        assert!(json.contains("conn-grpc-1"));
        assert!(json.contains("http://localhost:50051"));

        let deserialized: DebugEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(event, deserialized, "序列化往返应保持一致");
    }

    #[test]
    fn test_grpc_method_called_event() {
        let event = DebugEvent::GrpcMethodCalled {
            node_uuid: "node-002".to_string(),
            connection_id: "conn-grpc-1".to_string(),
            service: "package.ServiceName".to_string(),
            method: "GetUser".to_string(),
            pattern: "unary".to_string(),
            duration_ms: 120,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("GrpcMethodCalled"), "应包含变体名");
        assert!(json.contains("package.ServiceName"));
        assert!(json.contains("unary"));

        let deserialized: DebugEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(event, deserialized, "序列化往返应保持一致");

        // 验证 pattern 字段为 String 类型（非枚举）
        if let DebugEvent::GrpcMethodCalled {
            pattern,
            duration_ms,
            ..
        } = &deserialized
        {
            assert_eq!(pattern, "unary");
            assert_eq!(*duration_ms, 120);
        } else {
            panic!("反序列化后应为 GrpcMethodCalled 变体");
        }
    }

    #[test]
    fn test_grpc_channel_closed_event() {
        let event = DebugEvent::GrpcChannelClosed {
            node_uuid: "node-003".to_string(),
            connection_id: "conn-grpc-1".to_string(),
            url: "http://localhost:50051".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("GrpcChannelClosed"), "应包含变体名");
        assert!(json.contains("conn-grpc-1"));

        let deserialized: DebugEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(event, deserialized, "序列化往返应保持一致");
    }

    // ===== gRPC 流式调试事件序列化测试 =====

    #[test]
    fn test_grpc_stream_message_event() {
        let event = DebugEvent::GrpcStreamMessage {
            node_uuid: "node-020".to_string(),
            connection_id: "conn-grpc-1".to_string(),
            call_id: "call-001".to_string(),
            index: 1,
            data: r#"{"user_id": "abc"}"#.to_string(),
            received_at_ms: 50,
            size_bytes: 128,
            is_last: false,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("GrpcStreamMessage"), "应包含变体名");
        assert!(json.contains("call-001"));
        assert!(json.contains("conn-grpc-1"));

        let deserialized: DebugEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(event, deserialized, "序列化往返应保持一致");
    }

    #[test]
    fn test_grpc_stream_message_last_event() {
        let event = DebugEvent::GrpcStreamMessage {
            node_uuid: "node-020".to_string(),
            connection_id: "conn-grpc-1".to_string(),
            call_id: "call-001".to_string(),
            index: 5,
            data: r#"{"done": true}"#.to_string(),
            received_at_ms: 250,
            size_bytes: 64,
            is_last: true,
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("GrpcStreamMessage"));

        let deserialized: DebugEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(event, deserialized, "序列化往返应保持一致");

        if let DebugEvent::GrpcStreamMessage { is_last, index, .. } = &deserialized {
            assert!(*is_last);
            assert_eq!(*index, 5);
        } else {
            panic!("反序列化后应为 GrpcStreamMessage 变体");
        }
    }

    #[test]
    fn test_grpc_stream_error_event() {
        let event = DebugEvent::GrpcStreamError {
            node_uuid: "node-021".to_string(),
            connection_id: "conn-grpc-2".to_string(),
            call_id: "call-002".to_string(),
            message: "流式响应Error: internal (INTERNAL)".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("序列化应Success: {e}"));
        assert!(json.contains("GrpcStreamError"), "应包含变体名");
        assert!(json.contains("call-002"));
        assert!(json.contains("internal"));

        let deserialized: DebugEvent =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化应Success: {e}"));
        assert_eq!(event, deserialized, "序列化往返应保持一致");
    }

    #[test]
    fn test_snapshot_commands_and_events() {
        let cmd1 = DebugCommand::RetryCurrentNode;
        let json1 = serde_json::to_string(&cmd1).unwrap_or_else(|e| panic!("{e}"));
        let des1: DebugCommand = serde_json::from_str(&json1).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cmd1, des1);

        let cmd2 = DebugCommand::RunFromSnapshot {
            node_uuid: "node-100".to_string(),
        };
        let json2 = serde_json::to_string(&cmd2).unwrap_or_else(|e| panic!("{e}"));
        let des2: DebugCommand = serde_json::from_str(&json2).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cmd2, des2);

        let cmd3 = DebugCommand::SendStreamFrame {
            connection_id: "conn-ws-1".to_string(),
            payload: r#"{"type":"ping"}"#.to_string(),
            is_binary: false,
        };
        let json3 = serde_json::to_string(&cmd3).unwrap_or_else(|e| panic!("{e}"));
        let des3: DebugCommand = serde_json::from_str(&json3).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cmd3, des3);

        let mut vars = HashMap::new();
        vars.insert("k1".to_string(), serde_json::json!("v1"));
        let event1 = DebugEvent::SnapshotRestored {
            node_uuid: "node-100".to_string(),
            restored_variables: vars,
            stack_len: 2,
        };
        let json_ev1 = serde_json::to_string(&event1).unwrap_or_else(|e| panic!("{e}"));
        let des_ev1: DebugEvent = serde_json::from_str(&json_ev1).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(event1, des_ev1);

        let event2 = DebugEvent::StreamFrameSent {
            connection_id: "conn-ws-1".to_string(),
            timestamp_ms: 1234567,
            payload: "hello".to_string(),
        };
        let json_ev2 = serde_json::to_string(&event2).unwrap_or_else(|e| panic!("{e}"));
        let des_ev2: DebugEvent = serde_json::from_str(&json_ev2).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(event2, des_ev2);
    }
}
