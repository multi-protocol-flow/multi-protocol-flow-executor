// WebSocket Close 节点执行器
//
// 实现 WebSocket 连接关闭功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::WsProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// WebSocket Close 节点执行器
pub struct WsCloseExecutor {
    ws_manager: Arc<dyn WsProvider>,
}

impl WsCloseExecutor {
    /// 创建新的 WebSocket Close 执行器
    pub fn new(ws_manager: Arc<dyn WsProvider>) -> Self {
        Self { ws_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for WsCloseExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::WsClose(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not WsCloseConfig: {}", node.uuid))),
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("Connection ID variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        // 3. Log: start closing
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: ws_close, ConnectionID: {}, CloseCode: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            config.close_code
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("WebSocket Close node {} started", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 关闭连接
        let close_result = self
            .ws_manager
            .close(
                &connection_id,
                config.close_code,
                config.close_reason.clone(),
                config.close_timeout_ms,
            )
            .await;

        // 5. 反查 URL
        let url = context.get_ws_url(&connection_id).cloned();

        match close_result {
            Ok(()) => {
                log::info!(
                    "[WebSocket] Connection closed successfully: connection_id={}, code={}",
                    connection_id,
                    config.close_code
                );

                // 关闭 WebSocket 消息流追踪
                context.close_ws_stream(&connection_id).await;

                // 发送调试事件：WsStreamClosed
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::WsStreamClosed {
                        connection_id: connection_id.clone(),
                        last_owner_node_uuid: node.uuid.clone(),
                    });
                }

                // 6. 构建输出数据（结构化格式，与 HTTP 报告保持一致）
                let duration = start_time.elapsed();
                let output_data = json!({
                    // 统一成功标识（与 HTTP 保持一致）
                    "success": true,
                    "connection_id": connection_id,
                    "url": url,
                    // 请求配置（与 HTTP 的 request 结构对应）
                    "request": {
                        "connection_id": connection_id,
                        "close_code": config.close_code,
                        "close_reason": config.close_reason,
                        "close_timeout_ms": config.close_timeout_ms,
                    },
                    // 耗时分解（与 HTTP 的 timing 结构对应）
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "close_code": config.close_code,
                    "close_reason": config.close_reason,
                });

                // 6. 日志：关闭完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "WebSocket connection closed successfully ({}ms), code={}",
                            duration.as_millis(),
                            config.close_code
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("system time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 7. 成功时走 true 端口
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| {
                        conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
                    })
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[WebSocket] Connection close failed: connection_id={}, error={}",
                    connection_id,
                    e
                );

                // 日志：关闭失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("WebSocket connection close failed: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("system time error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据（结构化格式，与 HTTP 报告保持一致）
                let duration = start_time.elapsed();
                let error_output = json!({
                    // 统一成功标识（与 HTTP 保持一致）
                    "success": false,
                    "connection_id": connection_id,
                    "url": url,
                    // 请求配置
                    "request": {
                        "connection_id": connection_id,
                        "close_code": config.close_code,
                        "close_reason": config.close_reason,
                        "close_timeout_ms": config.close_timeout_ms,
                    },
                    // 耗时分解
                    "timing": {
                        "close_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("Flow aborted: {}", e)));
                }

                // 关闭失败走 false 端口
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| {
                        conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                    })
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::WsCloseConfig;
    use crate::websocket_manager::WebSocketConnectionManager;
    use std::sync::Arc;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "ws_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsClose(WsCloseConfig {
                connection_id: connection_id.to_string(),
                close_code: 1000,
                close_reason: None,
                close_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext::new(flow_id.to_string(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time error")
            .as_millis() as u64)
    }

    #[tokio::test]
    async fn test_close_connection_not_found() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsCloseExecutor::new(ws_manager);
        let node = create_test_node("test-1", "test close", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（因为没有连接，所以是空数组）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "nonexistent-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
        assert!(data["url"].is_null()); // URL 不存在
    }

    #[tokio::test]
    async fn test_close_with_variable_replacement() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsCloseExecutor::new(ws_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "variable substitution test", "{{conn_id}}");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("ws-connection-123"),
            String::new(),
        );

        // 注意：这个测试会失败，因为我们没有真实的 WebSocket 连接
        // 但我们可以验证变量替换逻辑不会崩溃
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（因为没有连接，所以是空数组）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "ws-connection-123");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
        assert!(data["url"].is_null()); // URL 不存在
    }

    #[tokio::test]
    async fn test_close_with_custom_code() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsCloseExecutor::new(ws_manager);

        // 创建带自定义关闭码的节点
        let node = Node {
            uuid: "test-custom-code".to_string(),
            node_type: "ws_close".to_string(),
            name: "custom close code".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsClose(WsCloseConfig {
                connection_id: "test-conn".to_string(),
                close_code: 1001, // Going Away
                close_reason: Some("user left".to_string()),
                close_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        // 注意：这个测试会失败，因为没有真实的 WebSocket 连接
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（因为没有连接，所以是空数组）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec![] as Vec<String>);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "test-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
        assert!(data["url"].is_null()); // URL 不存在
    }

    #[tokio::test]
    async fn test_close_to_echo_server() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsCloseExecutor::new(ws_manager.clone());

        // 先建立连接
        let connect_result = ws_manager
            .connect(
                "echo-close-conn".to_string(),
                "Echo Close Test".to_string(),
                "wss://echo.websocket.org".to_string(),
                vec![],
                vec![],
                30000,
                false,   // use_tls
                false,   // tls_skip_verify
                None,    // ping_interval_ms
                None,    // ping_timeout_ms
            )
            .await;

        // 网络测试：如果连接成功，继续测试
        if connect_result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = ws_manager.receive("echo-close-conn", 3000).await;

            // 创建关闭节点
            let node = create_test_node("close-test", "close test", "echo-close-conn");
            let mut context = create_context("test-flow");
            
            // 设置 URL 映射
            context.set_ws_url("echo-close-conn".to_string(), "wss://echo.websocket.org".to_string());

            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            // 验证关闭成功
            match result {
                Ok((next_nodes, output_data)) => {
                    assert_eq!(next_nodes, vec![] as Vec<String>);
                    assert!(output_data.is_some());
                    let data = output_data.unwrap();
                    assert_eq!(data["connection_id"], "echo-close-conn");
                    assert_eq!(data["close_code"], 1000);
                    assert_eq!(data["success"], true);
                    assert_eq!(data["url"], "wss://echo.websocket.org");
                }
                Err(e) => {
                    // 不应该发生错误
                    panic!("Executor returned error: {}", e);
                }
            }
        } else {
            println!("Skipping test: unable to connect to echo server");
        }
    }

    #[tokio::test]
    async fn test_close_with_reason() {
        let ws_manager = Arc::new(WebSocketConnectionManager::new());
        let executor = WsCloseExecutor::new(ws_manager.clone());

        // 先建立连接
        let connect_result = ws_manager
            .connect(
                "close-reason-conn".to_string(),
                "close reason test".to_string(),
                "wss://echo.websocket.org".to_string(),
                vec![],
                vec![],
                30000,
                false,   // use_tls
                false,   // tls_skip_verify
                None,    // ping_interval_ms
                None,    // ping_timeout_ms
            )
            .await;

        // 网络测试：如果连接成功，继续测试
        if connect_result.is_ok() {
            // 接收并丢弃初始问候消息
            let _ = ws_manager.receive("close-reason-conn", 3000).await;

            // 创建带关闭原因的节点
            let node = Node {
                uuid: "close-reason-test".to_string(),
                node_type: "ws_close".to_string(),
                name: "close reason test".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::WsClose(WsCloseConfig {
                connection_id: "close-reason-conn".to_string(),
                close_code: 1000,
                close_reason: Some("test complete".to_string()),
                close_timeout_ms: 5000,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
            };
            let mut context = create_context("test-flow");
            
            // 设置 URL 映射
            context.set_ws_url("close-reason-conn".to_string(), "wss://echo.websocket.org".to_string());

            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            // 验证关闭成功
            match result {
                Ok((next_nodes, output_data)) => {
                    assert_eq!(next_nodes, vec![] as Vec<String>);
                    assert!(output_data.is_some());
                    let data = output_data.unwrap();
                    assert_eq!(data["connection_id"], "close-reason-conn");
                    assert_eq!(data["close_reason"], "test complete");
                    assert_eq!(data["url"], "wss://echo.websocket.org");
                }
                Err(e) => {
                    // 不应该发生错误
                    panic!("Executor returned error: {}", e);
                }
            }
        } else {
            println!("Skipping test: unable to connect to echo server");
        }
    }
}