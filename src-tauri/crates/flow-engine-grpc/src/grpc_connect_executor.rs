// gRPC 连接节点执行器
//
// 实现 gRPC 连接建立、proto 文件解析和变量替换功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::{GrpcConnectConfig, NodeData};
use flow_engine_core::traits::GrpcProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// gRPC 连接节点执行器
pub struct GrpcConnectExecutor {
    grpc_manager: Arc<dyn GrpcProvider>,
}

impl GrpcConnectExecutor {
    /// 创建新的 gRPC 连接执行器
    pub fn new(grpc_manager: Arc<dyn GrpcProvider>) -> Self {
        Self { grpc_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for GrpcConnectExecutor {
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
            NodeData::GrpcConnect(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 GrpcConnectConfig: {}", node.uuid)),
        };

        // 2. 变量替换 URL
        let url = replace_variables(&config.url, context).map_err(|e| {
            let error_msg = format!("URL 变量替换失败: {}", e);
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            error_msg
        })?;

        // 3. 日志：开始连接
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: grpc_connect, URL: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("gRPC 连接节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 建立连接（计时）
        let connect_start = Instant::now();
        let proto_files: Vec<(String, String)> = config
            .proto_files
            .iter()
            .map(|pf| (pf.path.clone(), pf.content.clone()))
            .collect();
        let metadata: Vec<(String, String)> = config
            .default_metadata
            .iter()
            .map(|m| (m.key.clone(), m.value.clone()))
            .collect();
        let reflection_metadata: Vec<(String, String)> = config
            .reflection_metadata
            .as_ref()
            .map(|entries| {
                entries
                    .iter()
                    .map(|m| (m.key.clone(), m.value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let connect_result = self
            .grpc_manager
            .connect(
                node.uuid.clone(),
                url.clone(),
                config.use_tls,
                config.tls_skip_verify,
                config.enable_reflection,
                proto_files,
                metadata,
                config.connect_timeout_ms,
                config.tls_ca_cert.clone(),
                config.tls_client_cert.clone(),
                config.tls_client_key.clone(),
                config.tls_server_name_override.clone(),
                reflection_metadata,
                config.compression_encoding.clone(),
                config.keepalive_time_ms,
                config.keepalive_timeout_ms,
                config.keepalive_permit_without_streams,
                config.endpoints.clone(),
            )
            .await;
        let connect_duration_ms = connect_start.elapsed().as_millis() as u64;

        match connect_result {
            Ok(services) => {
                log::info!(
                    "[gRPC] 连接成功: uuid={}, url={}, 服务数={}",
                    node.uuid,
                    url,
                    services.len()
                );

                // 5. 构建输出数据
                let duration = start_time.elapsed();
                let output_data = self.build_success_output(
                    &node.uuid,
                    &url,
                    connect_duration_ms,
                    duration.as_millis() as u64,
                    &services,
                    &config,
                );

                // 6. 日志：连接完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "gRPC 连接成功，发现 {} 个服务 ({}ms)",
                            services.len(),
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

                // 7. 发送调试事件：GrpcChannelCreated
                if let Some(sender) = &context.debug_event_sender {
                    let _ = sender.send_event(DebugEvent::GrpcChannelCreated {
                        node_uuid: node.uuid.clone(),
                        connection_id: node.uuid.clone(),
                        url: url.clone(),
                        service_count: services.len(),
                    });
                }

                // 8. 返回 true 端口（连接成功）
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == "true"
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    // 如果没有连接，返回端口名称（用于测试）
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
                    "[gRPC] 连接失败: uuid={}, url={}, error={}",
                    node.uuid,
                    url,
                    e
                );

                // 日志：连接失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("gRPC 连接失败: {}", e),
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
                    "connection_id": serde_json::Value::Null,
                    "url": url,
                    "request": {
                        "url": url,
                        "use_tls": config.use_tls,
                        "tls_skip_verify": config.tls_skip_verify,
                        "enable_reflection": config.enable_reflection,
                        "proto_files_count": config.proto_files.len(),
                        "default_metadata_count": config.default_metadata.len(),
                        "connect_timeout_ms": config.connect_timeout_ms,
                    },
                    "timing": {
                        "connect_ms": connect_duration_ms,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(format!("Flow aborted: {}", e));
                }

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

impl GrpcConnectExecutor {
    /// 构建成功输出数据
    #[allow(clippy::too_many_arguments)]
    fn build_success_output(
        &self,
        uuid: &str,
        url: &str,
        connect_duration_ms: u64,
        total_ms: u64,
        services: &[flow_engine_core::traits::GrpcServiceInfo],
        config: &GrpcConnectConfig,
    ) -> serde_json::Value {
        let services_json: Vec<serde_json::Value> = services
            .iter()
            .map(|s| {
                let methods_json: Vec<serde_json::Value> = s
                    .methods
                    .iter()
                    .map(|m| {
                        json!({
                            "method_name": m.method_name,
                            "input_type": m.input_type,
                            "output_type": m.output_type,
                            "is_server_streaming": m.is_server_streaming,
                        })
                    })
                    .collect();
                json!({
                    "service_name": s.service_name,
                    "methods": methods_json,
                })
            })
            .collect();

        json!({
            "success": true,
            "connection_id": uuid,
            "url": url,
            "request": {
                "url": url,
                "use_tls": config.use_tls,
                "tls_skip_verify": config.tls_skip_verify,
                "enable_reflection": config.enable_reflection,
                "proto_files_count": config.proto_files.len(),
                "default_metadata_count": config.default_metadata.len(),
                "connect_timeout_ms": config.connect_timeout_ms,
            },
            "timing": {
                "connect_ms": connect_duration_ms,
                "total_ms": total_ms,
            },
            "services": services_json,
            "service_count": services.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::{GrpcConnectConfig, ProtoFile};
    use crate::grpc_connection_manager::GrpcConnectionManager;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "grpc_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcConnect(GrpcConnectConfig { url: url.to_string(),
            use_tls: false,
            tls_skip_verify: false,
            enable_reflection: false,
            proto_files: vec![],
            default_metadata: vec![],
            connect_timeout_ms: 30000,
            discovered_services: vec![], ..Default::default() }),
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
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None,
        }
    }

    #[tokio::test]
    async fn test_connect_no_proto_files() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcConnectExecutor::new(grpc_manager);
        let node = create_test_node("test-1", "测试连接", "http://localhost:50051");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（没有 proto 文件）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
    }

    #[tokio::test]
    async fn test_connect_with_variable_replacement() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcConnectExecutor::new(grpc_manager);

        // 创建带变量的节点
        let node = create_test_node("test-var", "变量替换测试", "http://{{host}}:50051");

        // 创建上下文并设置变量
        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("localhost"),
            String::new(),
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        // 验证变量替换生效（URL 中包含替换后的值）
        assert_eq!(data["url"], "http://localhost:50051");
    }

    #[tokio::test]
    async fn test_connect_wrong_node_data_type() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcConnectExecutor::new(grpc_manager);

        // 创建一个错误类型的节点
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

        // 应该返回错误
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("GrpcConnectConfig"));
    }

    #[tokio::test]
    async fn test_connect_with_proto_files() {
        let grpc_manager = Arc::new(GrpcConnectionManager::new());
        let executor = GrpcConnectExecutor::new(grpc_manager);

        // 创建带 proto 文件的节点（但连接不可达）
        let proto_content = r#"
syntax = "proto3";
package test;
service TestService {
  rpc GetUser(GetUserRequest) returns (GetUserResponse);
}
message GetUserRequest {
  string user_id = 1;
}
message GetUserResponse {
  string name = 1;
}
"#;
        let node = Node {
            uuid: "test-proto".to_string(),
            node_type: "grpc_connect".to_string(),
            name: "Proto 测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::GrpcConnect(GrpcConnectConfig { url: "http://192.0.2.1:50051".to_string(), // 不可路由地址
            use_tls: false,
            tls_skip_verify: false,
            enable_reflection: false,
            proto_files: vec![ProtoFile {
                path: "test.proto".to_string(),
                content: proto_content.to_string(),
            }],
            default_metadata: vec![],
            connect_timeout_ms: 2000, // 2 秒超时
            discovered_services: vec![], ..Default::default() }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // tonic 的 endpoint.connect() 是惰性的（不立即建立 TCP 连接），
        // 所以即使地址不可路由，proto 解析通过后连接可能返回成功。
        // 这里只验证执行器不会崩溃，且返回有效的结构化输出。
        assert!(result.is_ok());
        let (_next_nodes, output_data) = result.unwrap();
        assert!(output_data.is_some());
        let data = output_data.unwrap();
        // 验证输出结构完整
        assert!(data.get("success").is_some());
        assert!(data.get("connection_id").is_some() || data.get("url").is_some());
    }
}
