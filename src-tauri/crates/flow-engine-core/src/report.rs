// 报告结构定义模块

/// 执行状态枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// 执行Success
    Success,
    /// 执行Failed
    Failed,
    /// 跳过执行
    Skipped,
    /// 执行取消
    Cancelled,
}

/// 执行模式枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// 正常执行模式
    #[default]
    Normal,
    /// 调试执行模式
    Debug,
}
impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Success => write!(f, "success"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Skipped => write!(f, "skipped"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 变量变更记录
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VariableChange {
    /// 变量名
    pub key: String,
    /// 旧值（新创建的变量为 None）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_value: Option<serde_json::Value>,
    /// 新值
    pub new_value: serde_json::Value,
}

/// 节点执行报告
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionReport {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点类型
    pub node_type: String,
    /// 节点名称
    pub node_name: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 输入数据（完整存储，不截断）
    pub input_data: Option<serde_json::Value>,
    /// 输出数据（完整存储，不截断）
    pub output_data: Option<serde_json::Value>,
    /// 实际使用的输出端口 ID（仅已执行节点有值）
    /// - Entry node：固定为 "out"
    /// - Http 节点：固定为 "out"
    /// - Conditional 节点：根据条件评估结果为 "true" 或 "false"
    pub used_port_id: Option<String>,
    /// Error详情
    pub error: Option<crate::error::ErrorDetail>,
    /// 解析后的配置（变量解析后的实际值）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_config: Option<serde_json::Value>,
    /// 当前节点修改变量的记录
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_changes: Option<Vec<VariableChange>>,
    /// 跳过原因
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    /// 等待依赖节点的耗时（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_duration_ms: Option<u64>,
    /// 插件节点通过 `SidecarPlugin` 报告的自定义数据（M3.2 写入，M3.3 在报告构造时消费）
    /// 为空时从序列化输出中省略，旧 JSON 消费者不受影响
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_data: Option<serde_json::Value>,
    /// 子流程嵌套执行报告树
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_flow_report: Option<Box<FlowExecutionReport>>,
}

/// 报告元数据（用于历史列表）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportMetadata {
    /// 报告唯一 ID
    pub uuid: String,
    /// 关联的流程 ID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 执行状态 ("success" | "failed")
    pub status: String,
    /// 执行开始时间（毫秒）
    pub timestamp: u64,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 执行模式（正常/调试）
    #[serde(default)]
    pub execution_mode: ExecutionMode,
}

/// 跳过的节点信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkippedNodeInfo {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称
    pub node_name: String,
    /// 跳过原因
    pub reason: String,
}

/// WebSocket 消息流段摘要
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WsSegmentSummary {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点名称
    pub node_name: String,
    /// 节点类型 (`ws:connect`, `ws_send_wait`, `ws:close`)
    pub node_type: String,
    /// 消息数量
    pub message_count: usize,
}

/// WebSocket 连接消息流摘要
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WsStreamSummary {
    /// 连接 ID
    pub connection_id: String,
    /// WebSocket URL
    pub url: String,
    /// 接收消息总数
    pub total_received: usize,
    /// 发送消息总数
    pub total_sent: usize,
    /// 丢弃消息数
    pub dropped_messages: usize,
    /// 消息流段摘要列表
    pub segments: Vec<WsSegmentSummary>,
}

impl From<&crate::ws_stream::WsStreamSegment> for WsSegmentSummary {
    fn from(segment: &crate::ws_stream::WsStreamSegment) -> Self {
        Self {
            node_uuid: segment.node_uuid.clone(),
            node_name: segment.node_name.clone(),
            node_type: segment.node_type.clone(),
            message_count: segment.messages.len(),
        }
    }
}

impl From<&crate::ws_stream::WsConnectionStream> for WsStreamSummary {
    fn from(stream: &crate::ws_stream::WsConnectionStream) -> Self {
        use crate::ws_stream::MessageDirection;

        // 统计接收和发送的消息数量
        let mut total_received = 0;
        let mut total_sent = 0;

        for segment in &stream.segments {
            for msg in &segment.messages {
                match msg.direction {
                    MessageDirection::Received => total_received += 1,
                    MessageDirection::Sent => total_sent += 1,
                }
            }
        }

        // 统计丢弃的消息数量
        let dropped_messages: usize = stream
            .segments
            .iter()
            .map(|s| s.dropped_message_count)
            .sum();

        // 转换段摘要
        let segments: Vec<WsSegmentSummary> =
            stream.segments.iter().map(WsSegmentSummary::from).collect();

        Self {
            connection_id: stream.connection_id.clone(),
            url: stream.url.clone(),
            total_received,
            total_sent,
            dropped_messages,
            segments,
        }
    }
}

/// 插件流事件样本
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PluginStreamEvent {
    /// 事件类型（如 "log"、"data"、"status"）
    #[serde(rename = "type")]
    pub event_type: String,
    /// 事件时间戳（ISO 8601 或毫秒时间戳字符串）
    pub timestamp: String,
    /// 事件数据（任意 JSON 值）
    pub data: serde_json::Value,
}

/// 插件流统计信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub struct PluginStreamStats {
    /// 事件总数（样本可能被截断，count 始终为全量计数）
    pub count: usize,
    /// 首个事件时间戳
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_ts: Option<String>,
    /// 末个事件时间戳
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_ts: Option<String>,
}

/// 插件流记录（C2 存储：summary + 前 N 条样本，N 见 [`MAX_PLUGIN_STREAM_SAMPLES`]）
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PluginStreamRecord {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点类型
    pub node_type: String,
    /// 事件样本（上限 [`MAX_PLUGIN_STREAM_SAMPLES`] 条）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<PluginStreamEvent>,
    /// 流统计信息
    pub stats: PluginStreamStats,
}

/// 插件流记录中保存的事件样本上限
pub const MAX_PLUGIN_STREAM_SAMPLES: usize = 20;

/// 流程执行报告
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FlowExecutionReport {
    /// 报告唯一 ID（新增）
    pub uuid: String,
    /// 流程 UUID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 总执行时长（毫秒）
    pub duration_ms: u64,
    /// 总节点数
    pub total_nodes: u32,
    /// 已执行节点数
    pub executed_count: u32,
    /// 整体执行状态
    pub status: ExecutionStatus,
    /// 节点执行报告列表（扁平结构）
    pub node_reports: Vec<NodeExecutionReport>,
    /// 流程统计信息
    pub statistics: crate::statistics::FlowStatistics,
    /// 流程级Error详情（可选）
    pub flow_error: Option<crate::error::ErrorDetail>,
    /// 执行模式（正常/调试）
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    /// 流程初始变量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_variables: Option<serde_json::Value>,
    /// 流程最终变量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_variables: Option<serde_json::Value>,
    /// 执行路径（节点 UUID 列表，按执行顺序）
    #[serde(default)]
    pub execution_path: Vec<String>,
    /// 跳过的节点及原因
    #[serde(default)]
    pub skipped_nodes: Vec<SkippedNodeInfo>,
    /// WebSocket 消息流完整数据列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ws_streams: Vec<crate::ws_stream::WsConnectionStream>,
    /// 插件流记录列表（`SidecarPlugin` 事件流，C2 存储）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugin_streams: Vec<PluginStreamRecord>,
    /// Flow 快照（执行时的流程完整副本）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_snapshot: Option<crate::flow::Flow>,

    /// 结束节点到达状态：None=无End节点, Some(true)=已到达, Some(false)=未到达
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_node_reached: Option<bool>,

    /// 流程级警告信息（如缺少 End 节点）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_warning: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws_stream::{
        MessageContent, MessageDirection, WsConnectionStream, WsMessage, WsStreamSegment,
    };

    /// 测试 `WsSegmentSummary` 序列化
    #[test]
    fn test_ws_segment_summary_serialization() {
        let summary = WsSegmentSummary {
            node_uuid: "node-1".to_string(),
            node_name: "Connect Node".to_string(),
            node_type: "ws:connect".to_string(),
            message_count: 5,
        };

        let json = serde_json::to_string(&summary).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("node-1"));
        assert!(json.contains("Connect Node"));
        assert!(json.contains("ws:connect"));
        assert!(json.contains("5"));
    }

    /// 测试 `WsStreamSummary` 序列化
    #[test]
    fn test_ws_stream_summary_serialization() {
        let summary = WsStreamSummary {
            connection_id: "conn-1".to_string(),
            url: "wss://example.com/ws".to_string(),
            total_received: 3,
            total_sent: 2,
            dropped_messages: 1,
            segments: vec![WsSegmentSummary {
                node_uuid: "node-1".to_string(),
                node_name: "Connect".to_string(),
                node_type: "ws:connect".to_string(),
                message_count: 5,
            }],
        };

        let json = serde_json::to_string(&summary).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("conn-1"));
        assert!(json.contains("wss://example.com/ws"));
        assert!(json.contains("total_received"));
        assert!(json.contains("total_sent"));
        assert!(json.contains("dropped_messages"));
        assert!(json.contains("segments"));
    }

    /// 测试 `WsSegmentSummary::from`
    #[test]
    fn test_ws_segment_summary_from_segment() {
        let mut segment = WsStreamSegment::new(
            "node-1".to_string(),
            "Test Node".to_string(),
            "ws:connect".to_string(),
        );

        // 添加 3 条消息
        for i in 0..3 {
            let msg = WsMessage::new(
                MessageDirection::Sent,
                MessageContent::Text {
                    content: format!("msg-{}", i),
                },
            );
            segment.push_message(msg).unwrap_or_else(|e| panic!("{e}"));
        }

        let summary = WsSegmentSummary::from(&segment);

        assert_eq!(summary.node_uuid, "node-1");
        assert_eq!(summary.node_name, "Test Node");
        assert_eq!(summary.node_type, "ws:connect");
        assert_eq!(summary.message_count, 3);
    }

    /// 测试 `WsStreamSummary::from`
    #[test]
    fn test_ws_stream_summary_from_stream() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        // 创建初始段
        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect".to_string(),
            "ws:connect".to_string(),
        );

        // 添加发送消息
        let sent_msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: "hello".to_string(),
            },
        );
        stream
            .push_message(sent_msg)
            .unwrap_or_else(|e| panic!("{e}"));

        // 添加接收消息
        let received_msg = WsMessage::new(
            MessageDirection::Received,
            MessageContent::Text {
                content: "world".to_string(),
            },
        );
        stream
            .push_message(received_msg)
            .unwrap_or_else(|e| panic!("{e}"));

        let summary = WsStreamSummary::from(&stream);

        assert_eq!(summary.connection_id, "conn-1");
        assert_eq!(summary.url, "wss://example.com/ws");
        assert_eq!(summary.total_sent, 1);
        assert_eq!(summary.total_received, 1);
        assert_eq!(summary.dropped_messages, 0);
        assert_eq!(summary.segments.len(), 1);
        assert_eq!(summary.segments[0].node_uuid, "node-1");
    }

    /// 测试 `WsStreamSummary::from` 多段场景
    #[test]
    fn test_ws_stream_summary_from_stream_multiple_segments() {
        let mut stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        // 创建初始段
        stream.create_initial_segment(
            "node-1".to_string(),
            "Connect".to_string(),
            "ws:connect".to_string(),
        );

        // 添加 2 条发送消息
        for i in 0..2 {
            let msg = WsMessage::new(
                MessageDirection::Sent,
                MessageContent::Text {
                    content: format!("msg-{}", i),
                },
            );
            stream.push_message(msg).unwrap_or_else(|e| panic!("{e}"));
        }

        // 转移所有权
        stream.transfer_ownership(
            "node-2".to_string(),
            "SendWait".to_string(),
            "ws_send_wait".to_string(),
        );

        // 添加 1 条接收消息
        let msg = WsMessage::new(
            MessageDirection::Received,
            MessageContent::Text {
                content: "response".to_string(),
            },
        );
        stream.push_message(msg).unwrap_or_else(|e| panic!("{e}"));

        let summary = WsStreamSummary::from(&stream);

        assert_eq!(summary.total_sent, 2);
        assert_eq!(summary.total_received, 1);
        assert_eq!(summary.segments.len(), 2);
        assert_eq!(summary.segments[0].node_uuid, "node-1");
        assert_eq!(summary.segments[1].node_uuid, "node-2");
    }

    /// 测试 `FlowExecutionReport` 的 `ws_streams` 字段序列化（空时跳过）
    #[test]
    fn test_flow_report_ws_streams_skip_when_empty() {
        let report = FlowExecutionReport {
            uuid: "test-uuid".to_string(),
            flow_uuid: "flow-uuid".to_string(),
            flow_name: "Test".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![],
            statistics: crate::statistics::FlowStatistics::default(),
            flow_error: None,
            execution_mode: ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![],
            plugin_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        // 空的 ws_streams 应该被跳过
        assert!(!json.contains("ws_streams"));
    }

    /// 测试 `FlowExecutionReport` 的 `ws_streams` 字段序列化（非空时包含）
    #[test]
    fn test_flow_report_ws_streams_included_when_not_empty() {
        let ws_stream =
            WsConnectionStream::new("conn-1".to_string(), "wss://example.com/ws".to_string());

        let report = FlowExecutionReport {
            uuid: "test-uuid".to_string(),
            flow_uuid: "flow-uuid".to_string(),
            flow_name: "Test".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![],
            statistics: crate::statistics::FlowStatistics::default(),
            flow_error: None,
            execution_mode: ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![ws_stream],
            plugin_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        // 非空的 ws_streams 应该被包含
        assert!(json.contains("ws_streams"));
        assert!(json.contains("conn-1"));
        assert!(json.contains("wss://example.com/ws"));
    }

    /// 测试 `ExecutionStatus::Cancelled` 序列化为 "cancelled"
    #[test]
    fn test_execution_status_cancelled_serialization() {
        let status = ExecutionStatus::Cancelled;
        let json = serde_json::to_string(&status).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            json.contains("cancelled"),
            "Cancelled 应序列化为 'cancelled'"
        );

        // 反序列化往返测试
        let deserialized: ExecutionStatus =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(status, deserialized);

        // Display trait 测试
        assert_eq!(format!("{}", status), "cancelled");
    }

    /// 测试 `flow_snapshot` 序列化往返
    #[test]
    fn test_flow_report_snapshot_serialization() {
        let flow = crate::flow::Flow {
            uuid: "flow-snap-001".to_string(),
            name: "Snapshot Flow".to_string(),
            description: None,
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![],
            connections: vec![],
            hooks: None,
        };

        let report = FlowExecutionReport {
            uuid: "report-snap-001".to_string(),
            flow_uuid: "flow-snap-001".to_string(),
            flow_name: "Snapshot Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![],
            statistics: crate::statistics::FlowStatistics::default(),
            flow_error: None,
            execution_mode: ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![],
            flow_snapshot: Some(flow.clone()),
            end_node_reached: None,
            flow_warning: None,
            plugin_streams: vec![],
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("flow_snapshot"));
        assert!(json.contains("flow-snap-001"));
        assert!(json.contains("Snapshot Flow"));

        // 反序列化往返
        let deserialized: FlowExecutionReport =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(deserialized.flow_snapshot, Some(flow));
    }

    /// 测试 `flow_snapshot` 向后兼容（JSON 中缺少字段时反序列化为 None）
    #[test]
    fn test_flow_report_snapshot_backward_compat() {
        let json = r#"{
            "uuid": "report-bc-001",
            "flow_uuid": "flow-bc-001",
            "flow_name": "BackCompat Flow",
            "start_time": 1000,
            "end_time": 2000,
            "duration_ms": 1000,
            "total_nodes": 1,
            "executed_count": 1,
            "status": "success",
            "node_reports": [],
            "statistics": {
                "total_nodes": 0,
                "executed_nodes": 0,
                "success_count": 0,
                "failed_count": 0,
                "skipped_count": 0,
                "total_duration_ms": 0
            },
            "flow_error": null,
            "execution_mode": "normal",
            "execution_path": [],
            "skipped_nodes": []
        }"#;

        let report: FlowExecutionReport =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(report.uuid, "report-bc-001");
        assert!(report.flow_snapshot.is_none());
    }

    /// 测试 `flow_snapshot` 为 None 时序列化结果不包含该字段
    #[test]
    fn test_flow_report_snapshot_skip_when_none() {
        let report = FlowExecutionReport {
            uuid: "report-skip-001".to_string(),
            flow_uuid: "flow-skip-001".to_string(),
            flow_name: "Skip Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![],
            statistics: crate::statistics::FlowStatistics::default(),
            flow_error: None,
            execution_mode: ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![],
            plugin_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };

        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            !json.contains("flow_snapshot"),
            "flow_snapshot 为 None 时不应出现在 JSON 中"
        );
    }

    /// 测试 `plugin_data`：设置后 serde 往返保留；None 时从 JSON 中省略
    #[test]
    fn report_plugin_data_roundtrip() {
        let mut report = NodeExecutionReport {
            sub_flow_report: None,
            node_uuid: "node-plugin-1".to_string(),
            node_type: "plugin:demo".to_string(),
            node_name: "Demo Plugin".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: None,
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
            plugin_data: Some(serde_json::json!({
                "plugin_name": "demo",
                "plugin_version": "1.0.0",
                "result": "ok",
            })),
        };

        // None 时字段从 JSON 中省略
        report.plugin_data = None;
        let json_none =
            serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            !json_none.contains("plugin_data"),
            "plugin_data 为 None 时不应出现在 JSON 中"
        );

        // 设置后序列化并往返
        report.plugin_data = Some(serde_json::json!({"result": "ok"}));
        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("plugin_data"));
        assert!(json.contains("ok"));

        let deserialized: NodeExecutionReport =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(
            deserialized.plugin_data,
            Some(serde_json::json!({"result": "ok"}))
        );
    }

    /// 测试 `plugin_streams`：记录往返、事件样本封顶 20、空流从 JSON 中省略
    #[test]
    fn report_plugin_streams_aggregation() {
        // 构造 25 个事件，应被封顶为 MAX_PLUGIN_STREAM_SAMPLES(20) 条
        let mut events: Vec<PluginStreamEvent> = Vec::new();
        for i in 0..25 {
            events.push(PluginStreamEvent {
                event_type: "log".to_string(),
                timestamp: format!("ts-{}", i),
                data: serde_json::json!({"seq": i}),
            });
        }
        events.truncate(MAX_PLUGIN_STREAM_SAMPLES);
        assert_eq!(events.len(), MAX_PLUGIN_STREAM_SAMPLES);
        assert_eq!(MAX_PLUGIN_STREAM_SAMPLES, 20);

        let record = PluginStreamRecord {
            node_uuid: "node-plugin-2".to_string(),
            node_type: "plugin:demo".to_string(),
            events,
            stats: PluginStreamStats {
                count: 25,
                first_ts: Some("ts-0".to_string()),
                last_ts: Some("ts-24".to_string()),
            },
        };

        // 非空流序列化往返
        let json = serde_json::to_string(&record).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("plugin")); // 节点类型
        let deserialized: PluginStreamRecord =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(deserialized, record);
        assert_eq!(deserialized.events.len(), MAX_PLUGIN_STREAM_SAMPLES);

        // 空流在 FlowExecutionReport 中省略
        let report = FlowExecutionReport {
            uuid: "report-plugin-1".to_string(),
            flow_uuid: "flow-plugin-1".to_string(),
            flow_name: "Plugin Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Success,
            node_reports: vec![],
            statistics: crate::statistics::FlowStatistics::default(),
            flow_error: None,
            execution_mode: ExecutionMode::Normal,
            initial_variables: None,
            final_variables: None,
            execution_path: vec![],
            skipped_nodes: vec![],
            ws_streams: vec![],
            plugin_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        };
        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(
            !json.contains("plugin_streams"),
            "plugin_streams 为空时不应出现在 JSON 中"
        );

        // 非空流包含在报告中
        let mut report = report;
        report.plugin_streams = vec![record];
        let json = serde_json::to_string(&report).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("plugin_streams"));
    }
}
