// gRPC 调用节点执行器
//
// 实现 gRPC Unary 调用、变量替换和输出映射功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::GrpcProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// gRPC 调用节点执行器
pub struct GrpcCallExecutor {
    grpc_manager: Arc<dyn GrpcProvider>,
}

impl GrpcCallExecutor {
    /// 创建新的 gRPC 调用执行器
    pub fn new(grpc_manager: Arc<dyn GrpcProvider>) -> Self {
        Self { grpc_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for GrpcCallExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        // 1. 提取配置
        let config = match &node.data {
            NodeData::GrpcCall(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 GrpcCallConfig: {}", node.uuid)),
        };

        // 2. 变量替换 connection_id
        let connection_id = replace_variables(&config.connection_id, context).map_err(|e| {
            let error_msg = format!("连接 ID 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 变量替换 request_json
        let request_json = replace_variables(&config.request_json, context).map_err(|e| {
            let error_msg = format!("请求 JSON 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 4. 变量替换 metadata 值
        let metadata: Vec<(String, String)> = config
            .metadata
            .iter()
            .map(|m| {
                let value = replace_variables(&m.value, context).unwrap_or_else(|_| m.value.clone());
                (m.key.clone(), value)
            })
            .collect();

        // 5. 日志：开始调用
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: grpc_call, 连接ID: {}, 服务: {}/{}",
            context.flow_uuid,
            node.uuid,
            node.name,
            connection_id,
            config.service_name,
            config.method_name
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "gRPC 调用节点 {} 开始执行: {}/{}",
                    node.name, config.service_name, config.method_name
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 6. 执行 Unary 调用（计时）
        let call_start = Instant::now();
        let call_result = self
            .grpc_manager
            .call_unary(
                &connection_id,
                &config.service_name,
                &config.method_name,
                &request_json,
                metadata,
                config.timeout_ms,
            )
            .await;
        let call_duration_ms = call_start.elapsed().as_millis() as u64;

        match call_result {
            Ok(response_data) => {
                log::info!(
                    "[gRPC] 调用成功: connection_id={}, service={}, method={}, 耗时={}ms",
                    connection_id,
                    config.service_name,
                    config.method_name,
                    call_duration_ms
                );

                // 7. 构建成功输出数据
                let duration = start_time.elapsed();
                let output_data = json!({
                    "success": true,
                    "connection_id": connection_id,
                    "service_name": config.service_name,
                    "method_name": config.method_name,
                    "request": {
                        "connection_id": connection_id,
                        "service_name": config.service_name,
                        "method_name": config.method_name,
                        "request_json": config.request_json,
                        "timeout_ms": config.timeout_ms,
                        "metadata_count": config.metadata.len(),
                    },
                    "timing": {
                        "call_ms": call_duration_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "data": response_data,
                    "status": "ok",
                    "status_message": "",
                });

                // 8. 日志：调用完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "gRPC 调用成功: {}/{} ({}ms)",
                            config.service_name,
                            config.method_name,
                            duration.as_millis()
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 9. 返回 true 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec!["true".to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[gRPC] 调用失败: connection_id={}, service={}, method={}, error={}",
                    connection_id,
                    config.service_name,
                    config.method_name,
                    e
                );

                // 日志：调用失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!(
                            "gRPC 调用失败: {}/{} - {}",
                            config.service_name, config.method_name, e
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建错误输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "connection_id": connection_id,
                    "service_name": config.service_name,
                    "method_name": config.method_name,
                    "request": {
                        "connection_id": connection_id,
                        "service_name": config.service_name,
                        "method_name": config.method_name,
                        "request_json": config.request_json,
                        "timeout_ms": config.timeout_ms,
                        "metadata_count": config.metadata.len(),
                    },
                    "timing": {
                        "call_ms": call_duration_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "data": serde_json::Value::Null,
                    "status": "error",
                    "status_message": e,
                    "error": e,
                });

                // 返回 false 端口
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec!["false".to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::{GrpcCallConfig, GrpcMetadataEntry};
    use crate::grpc_connection_manager::GrpcConnectionManager;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_id: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "grpc_call".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: connection_id.to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: r#"{"user_id": "1"}"#.to_string(),
                timeout_ms: 30000,
                metadata: vec![],
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            grpc_manager: None,
            providers: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_call_connection_not_found() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);
        let node = create_test_node("test-1", "测试调用", "nonexistent-conn");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "nonexistent-conn");
        assert_eq!(data["success"], false);
        assert!(data["error"].is_string());
    }

    #[tokio::test]
    async fn test_call_with_variable_replacement() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);

        // 创建带变量的节点
        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "grpc_call".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "{{conn_id}}".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: r#"{"user_id": "{{uid}}"}"#.to_string(),
                timeout_ms: 30000,
                metadata: vec![],
            }),
            ports: None,
            has_breakpoint: None,
        };

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "conn_id".to_string(),
            json!("grpc-connection-123"),
            String::new(),
        );
        context.variables.set_variable(
            "uid".to_string(),
            json!("user-456"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["connection_id"], "grpc-connection-123");
        assert_eq!(data["success"], false);
    }

    #[tokio::test]
    async fn test_call_with_metadata_variable_replacement() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);

        let node = Node {
            uuid: "test-meta".to_string(),
            node_type: "grpc_call".to_string(),
            name: "Metadata 替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcCall(GrpcCallConfig {
                connection_id: "test-conn".to_string(),
                service_name: "test.TestService".to_string(),
                method_name: "GetUser".to_string(),
                request_json: "{}".to_string(),
                timeout_ms: 30000,
                metadata: vec![GrpcMetadataEntry {
                    key: "authorization".to_string(),
                    value: "Bearer {{token}}".to_string(),
                }],
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "token".to_string(),
            json!("my-secret-token"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 Ok，走 false 端口（连接不存在）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
    }

    #[tokio::test]
    async fn test_call_wrong_node_data_type() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcCallExecutor::new(grpc_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("GrpcCallConfig"));
    }
}
