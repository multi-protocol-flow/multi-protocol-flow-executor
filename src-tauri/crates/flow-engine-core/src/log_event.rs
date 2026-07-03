// 日志事件模块 - 用于 Tauri 2 Channel API 传递日志数据

use serde::{Deserialize, Serialize};

/// 日志级别枚举
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    /// 信息级别
    #[default]
    Info,
    /// 调试级别
    Debug,
    /// 警告级别
    Warn,
    /// Error级别
    Error,
}

/// 日志事件结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    /// 日志级别
    pub level: LogLevel,
    /// 日志消息内容
    pub message: String,
    /// 时间戳（毫秒）
    pub timestamp: u64,
    /// 关联节点 UUID
    pub node_uuid: Option<String>,
    /// 关联节点名称
    pub node_name: Option<String>,
    /// 关联流程 UUID
    pub flow_uuid: String,
}

impl Default for LogEvent {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            message: String::new(),
            timestamp: 0,
            node_uuid: None,
            node_name: None,
            flow_uuid: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_serialization() {
        let info = LogLevel::Info;
        let json = serde_json::to_string(&info).unwrap();
        assert_eq!(json, "\"INFO\"");

        let debug = LogLevel::Debug;
        let json = serde_json::to_string(&debug).unwrap();
        assert_eq!(json, "\"DEBUG\"");
    }

    #[test]
    fn test_log_event_serialization() {
        let event = LogEvent {
            level: LogLevel::Info,
            message: "Test message".to_string(),
            timestamp: 1234567890,
            node_uuid: Some("node-123".to_string()),
            node_name: Some("HTTP Request".to_string()),
            flow_uuid: "flow-456".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(json.contains("\"timestamp\":1234567890"));
        assert!(json.contains("\"nodeUuid\":\"node-123\""));
        assert!(json.contains("\"nodeName\":\"HTTP Request\""));
        assert!(json.contains("\"flowUuid\":\"flow-456\""));
    }

    #[test]
    fn test_log_event_deserialization() {
        let json = r#"{
            "level": "ERROR",
            "message": "Connection failed",
            "timestamp": 9876543210,
            "nodeUuid": "node-789",
            "flowUuid": "flow-101"
        }"#;

        let event: LogEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event.level, LogLevel::Error));
        assert_eq!(event.message, "Connection failed");
        assert_eq!(event.timestamp, 9876543210);
        assert_eq!(event.node_uuid, Some("node-789".to_string()));
        assert_eq!(event.node_name, None);
        assert_eq!(event.flow_uuid, "flow-101");
    }
}
