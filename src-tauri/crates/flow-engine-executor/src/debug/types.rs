//! 调试模式类型定义（从 flow-engine-core 重新导出）
//!
//! 类型定义已迁移至 flow-engine-core::debug_types 模块，
//! 此文件仅保留重新导出和测试。

// Re-export debug types from flow-engine-core
pub use flow_engine_core::debug_types::{
    DebugCommand, DebugEvent, DebugSessionState, DebugStatus, EndReason, ExecutionStackEntry,
    PauseReason,
};

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::ws_stream::WsMessage;
    use serde_json::json;
    use std::collections::HashMap;

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
                entry: ExecutionStackEntry {
                    node_uuid: "node-002".to_string(),
                    node_name: "HTTP Request".to_string(),
                    node_type: "Http".to_string(),
                    started_at: 1234567890,
                    ended_at: Some(1234567895),
                    input: None,
                    output: Some(json!({"status": 200})),
                    success: true,
                    error: None,
                    used_port_id: Some("out".to_string()),
                    resolved_config: None,
                    variable_changes: None,
                    skip_reason: None,
                },
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
        use flow_engine_core::ws_stream::{MessageContent, MessageDirection};

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
        use flow_engine_core::ws_stream::{MessageContent, MessageDirection};

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
        use flow_engine_core::ws_stream::{MessageContent, MessageDirection};

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
        use flow_engine_core::node_config::AssertionResult;

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
        use flow_engine_core::node_config::AssertionResult;

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
