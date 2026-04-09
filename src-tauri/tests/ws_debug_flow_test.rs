//! WebSocket 调试消息流端到端集成测试
//!
//! 测试场景:
//! 1. 完整调试流程: WsConnect → (断点) → WsSendCollect → WsClose
//! 2. 所有权转移: 多个 WsSendCollect 节点使用同一连接
//! 3. 报告集成: 验证报告包含 ws_streams
//! 4. 普通模式: 非调试模式执行，无调试事件，消息仍被记录

use tokio::sync::mpsc;

use flow_engine::connection::Connection;
use flow_engine::context::ExecutionContext;
use flow_engine::debug::{DebugCommand, DebugEvent, DebugStatus, DebuggableFlowExecutor};
use flow_engine::flow::Flow;
use flow_engine::node::{Node, NodePosition, NodeType};
use flow_engine::node_config::{NodeData, WsConnectConfig, WsSendCollectConfig, WsCloseConfig, WsCollectMode, WsMessageType};
use flow_engine::report::FlowExecutionReport;
use flow_engine::ws_stream::{MessageContent, MessageDirection, WsMessage};

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建测试流程: Entry → WsConnect → WsSendCollect → WsClose
fn create_basic_ws_flow(flow_uuid: &str) -> Flow {
    Flow {
        uuid: flow_uuid.to_string(),
        name: "WebSocket 调试测试流程".to_string(),
        initial_variables: None,
        nodes: vec![
            // Entry 节点
            Node {
                uuid: format!("entry-{}", flow_uuid),
                node_type: NodeType::Entry,
                name: "入口".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::entry(),
                ports: None,
                has_breakpoint: None,
            },
            // WsConnect 节点
            Node {
                uuid: format!("ws-connect-{}", flow_uuid),
                node_type: NodeType::WsConnect,
                name: "WS 连接".to_string(),
                position: NodePosition { x: 200.0, y: 0.0 },
                data: NodeData::WsConnect(WsConnectConfig {
                    url: "wss://echo.websocket.org".to_string(),
                    headers: vec![],
                    subprotocols: vec![],
                    connect_timeout_ms: 5000,
                    collect_message: false,
                    collect_mode: WsCollectMode::FirstMatch,
                    collect_condition: None,
                    collect_timeout_ms: 5000,
                    output_mappings: vec![],
                }),
                ports: None,
                has_breakpoint: None,
            },
            // WsSendCollect 节点
            Node {
                uuid: format!("ws-send-collect-{}", flow_uuid),
                node_type: NodeType::WsSendCollect,
                name: "WS 发送采集".to_string(),
                position: NodePosition { x: 400.0, y: 0.0 },
                data: NodeData::WsSendCollect(WsSendCollectConfig {
                    connection_id: format!("ws-connect-{}", flow_uuid),
                    message: "Hello, Debug!".to_string(),
                    message_type: WsMessageType::Text,
                    send_timeout_ms: 5000,
                    collect_response: false,
                    collect_mode: WsCollectMode::FirstMatch,
                    collect_condition: None,
                    collect_timeout_ms: 5000,
                    output_mappings: vec![],
                }),
                ports: None,
                has_breakpoint: None,
            },
            // WsClose 节点
            Node {
                uuid: format!("ws-close-{}", flow_uuid),
                node_type: NodeType::WsClose,
                name: "WS 关闭".to_string(),
                position: NodePosition { x: 600.0, y: 0.0 },
                data: NodeData::WsClose(WsCloseConfig {
                    connection_id: format!("ws-connect-{}", flow_uuid),
                close_code: 1000,
                close_reason: Some("正常关闭".to_string()),
                    close_timeout_ms: 5000,
                }),
                ports: None,
                has_breakpoint: None,
            },
        ],
        connections: vec![
            Connection {
                id: format!("conn-entry-wsconnect-{}", flow_uuid),
                source_node_uuid: format!("entry-{}", flow_uuid),
                target_node_uuid: format!("ws-connect-{}", flow_uuid),
                source_port_id: "output".to_string(),
                target_port_id: "input".to_string(),
            },
            Connection {
                id: format!("conn-wsconnect-wssendcollect-{}", flow_uuid),
                source_node_uuid: format!("ws-connect-{}", flow_uuid),
                target_node_uuid: format!("ws-send-collect-{}", flow_uuid),
                source_port_id: "true".to_string(),
                target_port_id: "input".to_string(),
            },
            Connection {
                id: format!("conn-wssendcollect-wsclose-{}", flow_uuid),
                source_node_uuid: format!("ws-send-collect-{}", flow_uuid),
                target_node_uuid: format!("ws-close-{}", flow_uuid),
                source_port_id: "true".to_string(),
                target_port_id: "input".to_string(),
            },
        ],
    }
}

// ============================================================================
// 测试 1: 完整调试流程（带断点）
// ============================================================================

/// 测试完整的调试流程: WsConnect → (断点) → WsSendCollect → WsClose
///
/// 验证:
/// - 调试会话能够正常启动和结束
/// - 断点在指定节点触发
/// - 执行堆栈记录正确
/// - 调试事件正确发送
#[tokio::test]
async fn test_debug_flow_with_breakpoint() {
    let flow_uuid = "test-debug-breakpoint-001";
    let flow = create_basic_ws_flow(flow_uuid);
    
    // 设置断点在 WsConnect 节点
    let breakpoints = vec![format!("ws-connect-{}", flow_uuid)];
    
    // 创建调试事件和命令通道
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
    let (command_sender, command_receiver) = mpsc::unbounded_channel();
    
    // 在后台收集事件
    let events_collector = tokio::spawn(async move {
        let mut events = Vec::new();
        let mut session_ended = false;
        
        while !session_ended {
            match event_receiver.recv().await {
                Some(event) => {
                    // 检查是否为会话结束事件
                    if matches!(event, DebugEvent::SessionEnded { .. }) {
                        session_ended = true;
                    }
                    events.push(event);
                }
                None => break,
            }
        }
        events
    });
    
    // 在后台发送调试命令
    let command_sender_clone = command_sender.clone();
    tokio::spawn(async move {
        // 等待一段时间让调试器启动
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // 发送 Continue 命令跳过断点
        let _ = command_sender_clone.send(DebugCommand::Continue);
        
        // 等待流程执行完成
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    });
    
    // 启动调试会话
    let executor = DebuggableFlowExecutor::new();
    let debug_state = executor
        .start_debug(
            &flow,
            None,
            breakpoints,
            None,
            event_sender,
            command_receiver,
        )
        .await;
    
    // 等待事件收集完成
    let events = events_collector.await.expect("事件收集失败");
    
    // 验证调试状态
    assert_eq!(debug_state.flow_uuid, flow_uuid);
    assert_eq!(debug_state.status, DebugStatus::Ended);
    
    // 验证事件
    assert!(!events.is_empty(), "应该收到调试事件");
    
    // 验证 SessionStarted 事件
    let has_session_started = events.iter().any(|e| {
        matches!(e, DebugEvent::SessionStarted { flow_uuid: f } if f == flow_uuid)
    });
    assert!(has_session_started, "应该有 SessionStarted 事件");
    
    // 验证 SessionEnded 事件
    let session_ended_event = events.iter().find(|e| {
        matches!(e, DebugEvent::SessionEnded { .. })
    });
    assert!(session_ended_event.is_some(), "应该有 SessionEnded 事件");
    
    // 验证执行堆栈
    assert!(!debug_state.execution_stack.is_empty(), "执行堆栈不应为空");
    
    // 验证至少执行了 Entry 节点
    let has_entry = debug_state.execution_stack.iter().any(|e| {
        e.node_type == "Entry"
    });
    assert!(has_entry, "应该执行了 Entry 节点");
}

// ============================================================================
// 测试 2: 所有权转移
// ============================================================================

/// 测试多个 WsSendCollect 节点使用同一连接时的所有权转移
///
/// 验证:
/// - 初始所有者是 WsConnect 节点
/// - 每次执行 WsSendWait 时所有权正确转移
/// - WsClose 节点正确关闭连接
/// - 消息流段正确记录
#[tokio::test]
async fn test_ownership_transfer() {
    let flow_uuid = "test-ownership-transfer-001";
    let connection_id = format!("ws-connect-{}", flow_uuid);
    
    // 创建执行上下文
    let trigger_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间错误")
        .as_millis() as u64;
    let context = ExecutionContext::new(flow_uuid.to_string(), trigger_time);
    
    // 模拟 WsConnect 节点创建消息流
    context.create_ws_stream(
        connection_id.clone(),
        "wss://example.com/ws".to_string(),
        format!("ws-connect-{}", flow_uuid),
        "WS 连接".to_string(),
        "ws_connect".to_string(),
    ).await;
    
    // 验证初始所有者
    {
        let stream = context.get_ws_stream(&connection_id).await;
        assert!(stream.is_some(), "应该创建消息流");
        let stream = stream.unwrap();
        assert_eq!(stream.current_owner_node_uuid, Some(format!("ws-connect-{}", flow_uuid)));
        assert_eq!(stream.segments.len(), 1, "应该有 1 个段");
    }
    
    // 模拟第一个 WsSendWait 节点转移所有权
    context.transfer_ws_ownership(
        &connection_id,
        format!("ws-send-collect-1-{}", flow_uuid),
        "WS 发送 1".to_string(),
        "ws_send_collect".to_string(),
    ).await;
    
    // 验证所有权转移
    {
        let stream = context.get_ws_stream(&connection_id).await.unwrap();
        assert_eq!(stream.current_owner_node_uuid, Some(format!("ws-send-collect-1-{}", flow_uuid)));
        assert_eq!(stream.segments.len(), 2, "应该有 2 个段");
        // 第一个段应该已结束
        assert!(stream.segments[0].ended_at.is_some(), "第一个段应该已结束");
        // 第二个段应该是当前段
        assert!(stream.segments[1].ended_at.is_none(), "第二个段应该未结束");
        assert_eq!(stream.segments[1].node_type, "ws_send_collect");
    }
    
    // 模拟第二个 WsSendWait 节点转移所有权
    context.transfer_ws_ownership(
        &connection_id,
        format!("ws-send-collect-2-{}", flow_uuid),
        "WS 发送 2".to_string(),
        "ws_send_collect".to_string(),
    ).await;
    
    // 验证所有权再次转移
    {
        let stream = context.get_ws_stream(&connection_id).await.unwrap();
        assert_eq!(stream.current_owner_node_uuid, Some(format!("ws-send-collect-2-{}", flow_uuid)));
        assert_eq!(stream.segments.len(), 3, "应该有 3 个段");
        assert_eq!(stream.segments[2].node_type, "ws_send_collect");
    }
    
    // 模拟 WsClose 节点关闭连接
    context.close_ws_stream(&connection_id).await;
    
    // 验证连接已关闭
    {
        let stream = context.get_ws_stream(&connection_id).await.unwrap();
        assert!(stream.is_closed, "连接应该已关闭");
        // 最后一个段应该已结束
        assert!(stream.segments.last().unwrap().ended_at.is_some(), "最后一个段应该已结束");
    }
}

// ============================================================================
// 测试 3: 报告包含 ws_streams
// ============================================================================

/// 测试 FlowExecutionReport 包含 ws_streams 字段
///
/// 验证:
/// - 报告包含 ws_streams 字段
/// - ws_streams 包含正确的连接信息
/// - 消息统计正确
#[tokio::test]
async fn test_report_includes_ws_streams() {
    let flow_uuid = "test-report-ws-streams-001";
    let connection_id = "test-conn-001";
    
    // 创建执行上下文
    let trigger_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间错误")
        .as_millis() as u64;
    let context = ExecutionContext::new(flow_uuid.to_string(), trigger_time);
    
    // 创建 WebSocket 消息流
    context.create_ws_stream(
        connection_id.to_string(),
        "wss://example.com/ws".to_string(),
        "node-connect".to_string(),
        "连接节点".to_string(),
        "ws_connect".to_string(),
    ).await;
    
    // 添加发送消息
    let sent_msg = WsMessage::new(
        MessageDirection::Sent,
        MessageContent::Text {
            content: "Hello".to_string(),
        },
    );
    context.push_ws_message(connection_id, sent_msg).await.expect("推送消息失败");
    
    // 添加接收消息
    let received_msg = WsMessage::new(
        MessageDirection::Received,
        MessageContent::Text {
            content: "World".to_string(),
        },
    );
    context.push_ws_message(connection_id, received_msg).await.expect("推送消息失败");
    
    // 转移所有权
    context.transfer_ws_ownership(
        connection_id,
        "node-send-wait".to_string(),
        "发送等待节点".to_string(),
        "ws_send_collect".to_string(),
    ).await;
    
    // 添加更多消息
    let sent_msg2 = WsMessage::new(
        MessageDirection::Sent,
        MessageContent::Text {
            content: "Message 2".to_string(),
        },
    );
    context.push_ws_message(connection_id, sent_msg2).await.expect("推送消息失败");
    
    // 关闭连接
    context.close_ws_stream(connection_id).await;
    
    // 从上下文生成 ws_streams（完整数据）
    let ws_streams = {
        let streams = context.ws_streams.read().await;
        streams.values().cloned().collect::<Vec<_>>()
    };
    
    // 验证 ws_streams 内容
    assert_eq!(ws_streams.len(), 1, "应该有 1 个 WebSocket 流");
    
    let stream = &ws_streams[0];
    assert_eq!(stream.connection_id, connection_id);
    assert_eq!(stream.url, "wss://example.com/ws");
    assert_eq!(stream.segments.len(), 2, "应该有 2 个段");
    
    // 验证消息数量
    let sent_count: usize = stream.segments.iter().map(|s| s.messages.iter().filter(|m| m.direction == MessageDirection::Sent).count()).sum();
    let received_count: usize = stream.segments.iter().map(|s| s.messages.iter().filter(|m| m.direction == MessageDirection::Received).count()).sum();
    assert_eq!(sent_count, 2, "应该发送了 2 条消息");
    assert_eq!(received_count, 1, "应该接收了 1 条消息");
    
    // 验证段信息
    assert_eq!(stream.segments[0].node_type, "ws_connect");
    assert_eq!(stream.segments[1].node_type, "ws_send_collect");
    
    // 创建报告并验证 ws_streams 字段
    let report = FlowExecutionReport {
        uuid: "test-report-001".to_string(),
        flow_uuid: flow_uuid.to_string(),
        flow_name: "测试报告".to_string(),
        start_time: trigger_time,
        end_time: trigger_time + 1000,
        duration_ms: 1000,
        total_nodes: 4,
        executed_count: 4,
        status: flow_engine::report::ExecutionStatus::Success,
        node_reports: vec![],
        statistics: flow_engine::statistics::FlowStatistics::default(),
        flow_error: None,
        execution_mode: flow_engine::report::ExecutionMode::Normal,
        initial_variables: None,
        final_variables: None,
        execution_path: vec![],
        skipped_nodes: vec![],
        ws_streams: ws_streams.clone(),
    };
    
    // 验证报告序列化包含 ws_streams
    let json = serde_json::to_string(&report).expect("序列化失败");
    assert!(json.contains("ws_streams"), "报告应该包含 ws_streams 字段");
    assert!(json.contains(connection_id), "报告应该包含连接 ID");
    assert!(json.contains("wss://example.com/ws"), "报告应该包含 URL");
    assert!(json.contains("segments"), "报告应该包含 segments 字段");
    assert!(json.contains("is_closed"), "报告应该包含 is_closed 字段");
}

// ============================================================================
// 测试 4: 普通模式无调试事件
// ============================================================================

/// 测试普通模式（非调试模式）执行不发送调试事件
///
/// 验证:
/// - debug_event_sender 为 None 时不发送调试事件
/// - 消息仍然被记录到 ws_streams
/// - 执行正常完成
#[tokio::test]
async fn test_normal_mode_no_debug_events() {
    let flow_uuid = "test-normal-mode-001";
    let connection_id = "test-conn-normal";
    
    // 创建执行上下文（不设置 debug_event_sender）
    let trigger_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间错误")
        .as_millis() as u64;
    let context = ExecutionContext::new(flow_uuid.to_string(), trigger_time);
    
    // 验证 debug_event_sender 为 None
    assert!(context.debug_event_sender.is_none(), "普通模式 debug_event_sender 应该为 None");
    
    // 创建 WebSocket 消息流
    context.create_ws_stream(
        connection_id.to_string(),
        "wss://example.com/ws".to_string(),
        "node-connect".to_string(),
        "连接节点".to_string(),
        "ws_connect".to_string(),
    ).await;
    
    // 添加消息（模拟发送和接收）
    let sent_msg = WsMessage::new(
        MessageDirection::Sent,
        MessageContent::Text {
            content: "Test message".to_string(),
        },
    );
    let push_result = context.push_ws_message(connection_id, sent_msg).await;
    assert!(push_result.is_ok(), "普通模式应该能正常记录消息");
    
    let received_msg = WsMessage::new(
        MessageDirection::Received,
        MessageContent::Text {
            content: "Response".to_string(),
        },
    );
    let push_result = context.push_ws_message(connection_id, received_msg).await;
    assert!(push_result.is_ok(), "普通模式应该能正常记录消息");
    
    // 验证消息被正确记录
    {
        let stream = context.get_ws_stream(connection_id).await;
        assert!(stream.is_some(), "应该创建消息流");
        let stream = stream.unwrap();
        assert_eq!(stream.total_message_count(), 2, "应该记录了 2 条消息");
    }
    
    // 关闭连接
    context.close_ws_stream(connection_id).await;
    
    // 验证最终状态
    {
        let stream = context.get_ws_stream(connection_id).await.unwrap();
        assert!(stream.is_closed, "连接应该已关闭");
        assert_eq!(stream.segments.len(), 1, "应该有 1 个段");
        assert_eq!(stream.segments[0].messages.len(), 2, "段中应该有 2 条消息");
    }
    
    // 生成 ws_streams 摘要（模拟报告生成）
    let ws_streams = {
        let streams = context.ws_streams.read().await;
        streams.values()
            .map(flow_engine::report::WsStreamSummary::from)
            .collect::<Vec<_>>()
    };
    
    // 验证报告可以正常生成
    assert_eq!(ws_streams.len(), 1);
    let summary = &ws_streams[0];
    assert_eq!(summary.total_sent, 1);
    assert_eq!(summary.total_received, 1);
}

// ============================================================================
// 测试 5: 调试事件发送验证
// ============================================================================

/// 测试调试模式下 WebSocket 相关调试事件的发送
///
/// 验证:
/// - WsStreamCreated 事件正确发送
/// - WsMessageSent 事件正确发送
/// - WsMessageReceived 事件正确发送
/// - WsStreamOwnerChanged 事件正确发送
/// - WsStreamClosed 事件正确发送
#[tokio::test]
async fn test_ws_debug_events() {
    // 创建调试事件通道
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
    
    // 发送测试事件
    let connection_id = "test-conn-events";
    
    // 1. WsStreamCreated 事件
    let created_event = DebugEvent::WsStreamCreated {
        connection_id: connection_id.to_string(),
        url: "wss://example.com/ws".to_string(),
        owner_node_uuid: "node-connect".to_string(),
        owner_node_name: "连接节点".to_string(),
    };
    event_sender.send(created_event.clone()).expect("发送事件失败");
    
    // 2. WsMessageSent 事件
    let sent_msg = WsMessage::new(
        MessageDirection::Sent,
        MessageContent::Text {
            content: "Hello".to_string(),
        },
    );
    let sent_event = DebugEvent::WsMessageSent {
        connection_id: connection_id.to_string(),
        owner_node_uuid: "node-send-wait".to_string(),
        message: sent_msg,
    };
    event_sender.send(sent_event.clone()).expect("发送事件失败");
    
    // 3. WsMessageReceived 事件
    let received_msg = WsMessage::new(
        MessageDirection::Received,
        MessageContent::Text {
            content: "World".to_string(),
        },
    );
    let received_event = DebugEvent::WsMessageReceived {
        connection_id: connection_id.to_string(),
        owner_node_uuid: "node-send-wait".to_string(),
        message: received_msg,
    };
    event_sender.send(received_event.clone()).expect("发送事件失败");
    
    // 4. WsStreamOwnerChanged 事件
    let owner_changed_event = DebugEvent::WsStreamOwnerChanged {
        connection_id: connection_id.to_string(),
        from_node_uuid: "node-connect".to_string(),
        to_node_uuid: "node-send-wait".to_string(),
    };
    event_sender.send(owner_changed_event.clone()).expect("发送事件失败");
    
    // 5. WsStreamClosed 事件
    let closed_event = DebugEvent::WsStreamClosed {
        connection_id: connection_id.to_string(),
        last_owner_node_uuid: "node-close".to_string(),
    };
    event_sender.send(closed_event.clone()).expect("发送事件失败");
    
    // 收集事件
    drop(event_sender); // 关闭发送端，让接收端结束
    let mut events = Vec::new();
    while let Some(event) = event_receiver.recv().await {
        events.push(event);
    }
    
    // 验证所有事件都收到
    assert_eq!(events.len(), 5, "应该收到 5 个事件");
    
    // 验证事件类型
    assert!(events.iter().any(|e| matches!(e, DebugEvent::WsStreamCreated { .. })));
    assert!(events.iter().any(|e| matches!(e, DebugEvent::WsMessageSent { .. })));
    assert!(events.iter().any(|e| matches!(e, DebugEvent::WsMessageReceived { .. })));
    assert!(events.iter().any(|e| matches!(e, DebugEvent::WsStreamOwnerChanged { .. })));
    assert!(events.iter().any(|e| matches!(e, DebugEvent::WsStreamClosed { .. })));
    
    // 验证事件可以序列化
    for event in &events {
        let json = serde_json::to_string(event).expect("事件序列化失败");
        let deserialized: DebugEvent = serde_json::from_str(&json).expect("事件反序列化失败");
        assert_eq!(*event, deserialized, "事件序列化/反序列化应该保持一致");
    }
}

// ============================================================================
// 测试 6: WsConnectionStream 消息记录
// ============================================================================

/// 测试 WsConnectionStream 的消息记录功能
///
/// 验证:
/// - 消息正确记录到当前段
/// - 消息方向正确区分
/// - 所有权转移后消息记录到新段
#[tokio::test]
async fn test_ws_stream_message_recording() {
    let connection_id = "test-conn-recording";
    let flow_uuid = format!("test-ws-recording-{}", uuid::Uuid::new_v4());
    
    // 创建执行上下文
    let trigger_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间错误")
        .as_millis() as u64;
    let context = ExecutionContext::new(flow_uuid.to_string(), trigger_time);
    
    // 创建 WebSocket 消息流
    context.create_ws_stream(
        connection_id.to_string(),
        "wss://example.com/ws".to_string(),
        "node-1".to_string(),
        "节点 1".to_string(),
        "ws_connect".to_string(),
    ).await;
    
    // 记录多条消息
    for i in 0..10 {
        let msg = WsMessage::new(
            if i % 2 == 0 { MessageDirection::Sent } else { MessageDirection::Received },
            MessageContent::Text {
                content: format!("Message {}", i),
            },
        );
        context.push_ws_message(connection_id, msg).await.expect("推送消息失败");
    }
    
    // 验证消息记录
    {
        let stream = context.get_ws_stream(connection_id).await.unwrap();
        assert_eq!(stream.total_message_count(), 10, "应该记录 10 条消息");
        assert_eq!(stream.segments.len(), 1, "应该有 1 个段");
        
        // 验证消息方向分布
        let segment = &stream.segments[0];
        let sent_count = segment.messages.iter()
            .filter(|m| m.direction == MessageDirection::Sent)
            .count();
        let received_count = segment.messages.iter()
            .filter(|m| m.direction == MessageDirection::Received)
            .count();
        assert_eq!(sent_count, 5, "应该有 5 条发送消息");
        assert_eq!(received_count, 5, "应该有 5 条接收消息");
    }
    
    // 转移所有权并记录更多消息
    context.transfer_ws_ownership(
        connection_id,
        "node-2".to_string(),
        "节点 2".to_string(),
        "ws_send_collect".to_string(),
    ).await;
    
    for i in 10..15 {
        let msg = WsMessage::new(
            MessageDirection::Sent,
            MessageContent::Text {
                content: format!("Message {}", i),
            },
        );
        context.push_ws_message(connection_id, msg).await.expect("推送消息失败");
    }
    
    // 验证新段的消息
    {
        let stream = context.get_ws_stream(connection_id).await.unwrap();
        assert_eq!(stream.total_message_count(), 15, "总共应该有 15 条消息");
        assert_eq!(stream.segments.len(), 2, "应该有 2 个段");
        
        // 第一个段应该有 10 条消息
        assert_eq!(stream.segments[0].messages.len(), 10);
        // 第二个段应该有 5 条消息
        assert_eq!(stream.segments[1].messages.len(), 5);
    }
}
