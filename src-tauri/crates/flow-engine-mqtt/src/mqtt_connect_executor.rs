//! MQTT 连接节点执行器
//!
//! 实现 MQTT 连接建立、变量替换和输出构建功能。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::MqttProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// MQTT 连接节点执行器
pub struct MqttConnectExecutor {
    mqtt_manager: Arc<dyn MqttProvider>,
}

impl MqttConnectExecutor {
    /// 创建新的 MQTT 连接执行器
    pub fn new(mqtt_manager: Arc<dyn MqttProvider>) -> Self {
        Self { mqtt_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for MqttConnectExecutor {
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
            NodeData::MqttConnect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not MqttConnectConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（url, client_id, username, password）
        let url = replace_variables(&config.url, context).map_err(|e| {
            let error_msg = format!("URL variable substitution failed: {}", e);
            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );
            ErrorDetail::execution_error(error_msg)
        })?;

        let client_id = match &config.client_id {
            Some(id) => Some(replace_variables(id, context).map_err(|e| {
                ErrorDetail::execution_error(format!(
                    "client_id variable substitution failed: {}",
                    e
                ))
            })?),
            None => None,
        };

        let username = match &config.username {
            Some(u) => Some(replace_variables(u, context).map_err(|e| {
                ErrorDetail::execution_error(format!(
                    "username variable substitution failed: {}",
                    e
                ))
            })?),
            None => None,
        };

        let password = match &config.password {
            Some(p) => Some(replace_variables(p, context).map_err(|e| {
                ErrorDetail::execution_error(format!(
                    "password variable substitution failed: {}",
                    e
                ))
            })?),
            None => None,
        };

        // 3. 日志：开始连接
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: mqtt_connect, URL: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            url
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("MQTT 连接节点 {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建连接配置（使用替换后的值）
        let connect_config = flow_engine_core::node_config::MqttConnectConfig {
            url,
            client_id,
            username,
            password,
            ..config
        };

        // 5. 建立连接
        let connect_result = self.mqtt_manager.connect(&node.uuid, &connect_config).await;

        match connect_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[MQTT] 连接Success: uuid={}, url={}, reason_code={:?}",
                        node.uuid,
                        connect_config.url,
                        output.reason_code
                    );
                } else {
                    log::warn!(
                        "[MQTT] connection failed: uuid={}, url={}, error={:?}",
                        node.uuid,
                        connect_config.url,
                        output.error
                    );
                }

                // 6. 构建输出数据
                let broker_info_json = match &output.broker_info {
                    Some(info) => json!({
                        "version": info.version,
                        "session_present": info.session_present,
                        "assigned_client_id": info.assigned_client_id,
                        "server_keep_alive": info.server_keep_alive,
                        "topic_alias_max": info.topic_alias_max,
                        "receive_max": info.receive_max,
                        "shared_subscription_available": info.shared_subscription_available,
                        "wildcard_sub_available": info.wildcard_sub_available,
                    }),
                    None => serde_json::Value::Null,
                };

                let output_data = json!({
                    "success": output.success,
                    "broker_info": broker_info_json,
                    "client_id": connect_config.client_id,
                    "mqtt_version": connect_config.mqtt_version,
                    "reason_code": output.reason_code,
                    "request": {
                        "url": connect_config.url,
                        "mqtt_version": connect_config.mqtt_version,
                        "clean_session": connect_config.clean_session,
                        "keep_alive_secs": connect_config.keep_alive_secs,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": output.error,
                });

                // 7. 日志：连接完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!("MQTT 连接Success ({}ms)", duration.as_millis())
                } else {
                    format!(
                        "MQTT connection failed: {} ({}ms)",
                        output.error.as_deref().unwrap_or("Unknown error"),
                        duration.as_millis()
                    )
                };

                context
                    .emit_log(LogEvent {
                        level: log_level,
                        message: log_message,
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 8. 路由：Success走 true 端口，Failed走 false 端口
                if !output.success
                    && !node
                        .on_error
                        .as_ref()
                        .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    let error_msg = output
                        .error
                        .as_deref()
                        .unwrap_or("MQTT connection failed")
                        .to_string();
                    return Err(ErrorDetail::execution_error(error_msg));
                }
                let port = if output.success { "true" } else { "false" };
                let next_node_ids: Vec<String> = {
                    let filtered: Vec<String> = connections
                        .iter()
                        .filter(|conn| {
                            conn.source_node_uuid == node.uuid && conn.source_port_id == port
                        })
                        .map(|conn| conn.target_node_uuid.clone())
                        .collect();

                    if filtered.is_empty() {
                        vec![port.to_string()]
                    } else {
                        filtered
                    }
                };

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[MQTT] connection error: uuid={}, url={}, error={}",
                    node.uuid,
                    connect_config.url,
                    e
                );

                // 日志：connection error
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("MQTT connection error: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建Error输出数据
                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "broker_info": serde_json::Value::Null,
                    "client_id": connect_config.client_id,
                    "mqtt_version": connect_config.mqtt_version,
                    "reason_code": serde_json::Value::Null,
                    "request": {
                        "url": connect_config.url,
                        "mqtt_version": connect_config.mqtt_version,
                    },
                    "timing": {
                        "connect_ms": duration.as_millis() as u64,
                        "total_ms": duration.as_millis() as u64,
                    },
                    "error": e,
                });

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(e));
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

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::MqttConnectConfig;
    use flow_engine_core::traits::{MqttBrokerInfo, MqttConnectOutput, MqttConnectionInfo};

    /// 模拟 MQTT Provider（连接Success）
    struct MockMqttProvider {
        output: MqttConnectOutput,
    }

    impl MockMqttProvider {
        fn new_success() -> Self {
            Self {
                output: MqttConnectOutput {
                    success: true,
                    broker_info: Some(MqttBrokerInfo {
                        version: Some("5.0".to_string()),
                        session_present: Some(false),
                        assigned_client_id: None,
                        server_keep_alive: Some(60),
                        topic_alias_max: None,
                        receive_max: None,
                        shared_subscription_available: None,
                        wildcard_sub_available: None,
                    }),
                    reason_code: Some(0),
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: MqttConnectOutput {
                    success: false,
                    broker_info: None,
                    reason_code: None,
                    error: Some(error.to_string()),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl MqttProvider for MockMqttProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &MqttConnectConfig,
        ) -> Result<MqttConnectOutput, String> {
            Ok(self.output.clone())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttPublishConfig,
        ) -> Result<flow_engine_core::traits::MqttPublishOutput, String> {
            Err("not implemented".to_string())
        }

        async fn subscribe(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttSubscribeConfig,
        ) -> Result<flow_engine_core::traits::MqttSubscribeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn unsubscribe(
            &self,
            _uuid: &str,
            _connection_id: &str,
            _topic: &str,
        ) -> Result<(), String> {
            Err("not implemented".to_string())
        }

        async fn wait_message(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttWaitMessageConfig,
        ) -> Result<flow_engine_core::traits::MqttMessageOutput, String> {
            Err("not implemented".to_string())
        }

        async fn request(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttRequestConfig,
        ) -> Result<flow_engine_core::traits::MqttMessageOutput, String> {
            Err("not implemented".to_string())
        }

        async fn disconnect(
            &self,
            _uuid: &str,
            _config: &flow_engine_core::node_config::MqttDisconnectConfig,
        ) -> Result<(), String> {
            Ok(())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<MqttConnectionInfo> {
            vec![]
        }
    }

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, url: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "mqtt_connect".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttConnect(MqttConnectConfig {
                url: url.to_string(),
                client_id: Some("test-client".to_string()),
                username: None,
                password: None,
                ..MqttConnectConfig::default()
            }),
            on_error: None,
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            execution_id: String::new(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: vec![],
            flow_trigger_time: flow_engine_core::utils::unix_timestamp_ms(),
            log_sender: None,
            flow_file_path: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
            current_node_plugin_data: None,
        }
    }

    /// 测试：连接Success时输出 JSON 结构正确
    #[tokio::test]
    async fn test_connect_success_output_structure() {
        let mqtt_manager = Arc::new(MockMqttProvider::new_success());
        let executor = MqttConnectExecutor::new(mqtt_manager);
        let node = create_test_node("test-1", "测试连接", "mqtt://localhost:1883");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        // Success应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], true);
        assert!(data["broker_info"].is_object());
        assert_eq!(data["broker_info"]["version"], "5.0");
        assert_eq!(data["client_id"], "test-client");
        assert_eq!(data["mqtt_version"], "5.0");
        assert_eq!(data["reason_code"], 0);
        // timing 字段应存在
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        // request 字段应包含连接信息
        assert_eq!(data["request"]["url"], "mqtt://localhost:1883");
    }

    /// 测试：connection failed时路由到 false 端口
    #[tokio::test]
    async fn test_connect_failure_routes_to_false_port() {
        let mqtt_manager = Arc::new(MockMqttProvider::new_failure("Connection refused"));
        let executor = MqttConnectExecutor::new(mqtt_manager);
        let node = create_test_node("test-2", "Failed测试", "mqtt://unreachable:1883");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        // Failed应路由到 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Connection refused");
        assert!(data["broker_info"].is_null());
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_connect_wrong_node_data_type() {
        let mqtt_manager = Arc::new(MockMqttProvider::new_success());
        let executor = MqttConnectExecutor::new(mqtt_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            on_error: None,
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .message
            .contains("MqttConnectConfig"));
    }

    /// 测试：变量替换在 URL 和 `client_id` 中生效
    #[tokio::test]
    async fn test_connect_with_variable_replacement() {
        let mqtt_manager = Arc::new(MockMqttProvider::new_success());
        let executor = MqttConnectExecutor::new(mqtt_manager);

        let node = Node {
            uuid: "test-var".to_string(),
            node_type: "mqtt_connect".to_string(),
            name: "变量替换测试".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::MqttConnect(MqttConnectConfig {
                url: "mqtt://{{host}}:1883".to_string(),
                client_id: Some("client-{{env}}".to_string()),
                username: Some("user-{{env}}".to_string()),
                password: None,
                ..MqttConnectConfig::default()
            }),
            on_error: None,
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_context("test-flow");
        context.variables.set_variable(
            "host".to_string(),
            json!("broker.example.com"),
            String::new(),
        );
        context
            .variables
            .set_variable("env".to_string(), json!("prod"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap_or_else(|| panic!("expected Some"));
        // 验证变量替换生效
        assert_eq!(data["request"]["url"], "mqtt://broker.example.com:1883");
        assert_eq!(data["client_id"], "client-prod");
    }

    /// 测试：连接配置中的连接端口路由
    #[tokio::test]
    async fn test_connect_with_connections_routing() {
        let mqtt_manager = Arc::new(MockMqttProvider::new_success());
        let executor = MqttConnectExecutor::new(mqtt_manager);
        let node = create_test_node("test-route", "路由测试", "mqtt://localhost:1883");
        let mut context = create_context("test-flow");

        // 创建 true 端口连接到下一个节点
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-route".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap_or_else(|e| panic!("{e}"));
        // 应路由到连接中指定的目标节点
        assert_eq!(next_nodes, vec!["next-node"]);
    }
}
