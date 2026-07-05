//! AMQP 发布消息节点执行器
//!
//! 实现 AMQP 0-9-1 消息发布功能，支持 14 种消息属性和 3 种确认模式（none/confirm/transaction）。

use std::collections::HashMap;
use flow_engine_core::error::ErrorDetail;
use std::sync::Arc;
use std::time::Instant;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::{AmqpPublishConfig, NodeData};
use flow_engine_core::traits::AmqpProvider;
use serde_json::json;

/// AMQP 发布消息节点执行器
pub struct AmqpPublishExecutor {
    amqp_manager: Arc<dyn AmqpProvider>,
}

impl AmqpPublishExecutor {
    /// 创建新的 AMQP 发布执行器
    pub fn new(amqp_manager: Arc<dyn AmqpProvider>) -> Self {
        Self { amqp_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AmqpPublishExecutor {
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
            NodeData::AmqpPublish(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(format!(
                    "Node data type is not AmqpPublishConfig: {}",
                    node.uuid
                )))
            }
        };

        // 2. 变量替换（exchange, routing_key, body, message_id, correlation_id,
        //    reply_to, expiration, message_type, user_id, app_id, cluster_id）
        let exchange = match &config.exchange {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("exchange variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let routing_key = match &config.routing_key {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("routing_key variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let body = match &config.body {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("body variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let message_id = match &config.message_id {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("message_id variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let correlation_id = match &config.correlation_id {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("correlation_id variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let reply_to = match &config.reply_to {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("reply_to variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let expiration = match &config.expiration {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("expiration variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let message_type = match &config.message_type {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("message_type variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let user_id = match &config.user_id {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("user_id variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let app_id = match &config.app_id {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("app_id variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        let cluster_id = match &config.cluster_id {
            Some(v) => Some(replace_variables(v, context).map_err(|e| {
                let error_msg = format!("cluster_id variable substitution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                ErrorDetail::execution_error(error_msg)
            })?),
            None => None,
        };

        // 变量替换 headers 值
        let headers = match &config.headers {
            Some(h) => {
                let mut replaced = HashMap::new();
                for (k, v) in h {
                    let replaced_val = match v.as_str() {
                        Some(s) => {
                            let s_replaced = replace_variables(s, context).map_err(|e| {
                                let error_msg = format!("headers[{}] variable substitution failed: {}", k, e);
                                log::error!(
                                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                                    context.flow_uuid,
                                    node.uuid,
                                    node.name,
                                    error_msg
                                );
                                ErrorDetail::execution_error(error_msg)
                            })?;
                            json!(s_replaced)
                        }
                        None => v.clone(),
                    };
                    replaced.insert(k.clone(), replaced_val);
                }
                Some(replaced)
            }
            None => None,
        };

        // 3. 日志：开始发布
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: amqp_publish, exchange: {:?}, routing_key: {:?}",
            context.flow_uuid,
            node.uuid,
            node.name,
            exchange,
            routing_key
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("AMQP 发布节点 {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 构建替换后的配置
        let resolved_config = AmqpPublishConfig {
            connection_id: config.connection_id.clone(),
            exchange,
            routing_key,
            body,
            body_type: config.body_type.clone(),
            content_type: config.content_type.clone(),
            content_encoding: config.content_encoding.clone(),
            delivery_mode: config.delivery_mode,
            priority: config.priority,
            message_id,
            correlation_id,
            reply_to,
            expiration,
            timestamp: config.timestamp,
            message_type,
            user_id,
            app_id,
            cluster_id,
            headers,
            mandatory: config.mandatory,
            confirm_mode: config.confirm_mode.clone(),
        };

        // 5. 发布消息
        let publish_result = self
            .amqp_manager
            .publish(&config.connection_id, &resolved_config)
            .await;

        match publish_result {
            Ok(output) => {
                let duration = start_time.elapsed();

                if output.success {
                    log::info!(
                        "[AMQP] 发布Success: uuid={}, exchange={:?}, routing_key={:?}, confirm_mode={}, confirm_result={:?} ({}ms)",
                        node.uuid,
                        resolved_config.exchange,
                        resolved_config.routing_key,
                        resolved_config.confirm_mode,
                        output.confirm_result,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "[AMQP] 发布Failed: uuid={}, error={:?} ({}ms)",
                        node.uuid,
                        output.error,
                        duration.as_millis()
                    );
                }

                // 6. 构建输出数据
                let output_data = json!({
                    "success": output.success,
                    "confirm_result": output.confirm_result,
                    "returned_message": output.returned_message,
                    "error": output.error,
                    "request": {
                        "connection_id": resolved_config.connection_id,
                        "exchange": resolved_config.exchange,
                        "routing_key": resolved_config.routing_key,
                        "body": resolved_config.body,
                        "body_type": resolved_config.body_type,
                        "content_type": resolved_config.content_type,
                        "content_encoding": resolved_config.content_encoding,
                        "delivery_mode": resolved_config.delivery_mode,
                        "priority": resolved_config.priority,
                        "message_id": resolved_config.message_id,
                        "correlation_id": resolved_config.correlation_id,
                        "reply_to": resolved_config.reply_to,
                        "expiration": resolved_config.expiration,
                        "timestamp": resolved_config.timestamp,
                        "message_type": resolved_config.message_type,
                        "user_id": resolved_config.user_id,
                        "app_id": resolved_config.app_id,
                        "cluster_id": resolved_config.cluster_id,
                        "headers": resolved_config.headers,
                        "mandatory": resolved_config.mandatory,
                        "confirm_mode": resolved_config.confirm_mode,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 7. 日志：发布完成
                let log_level = if output.success {
                    LogLevel::Info
                } else {
                    LogLevel::Error
                };
                let log_message = if output.success {
                    format!(
                        "AMQP 发布Success, 确认结果: {:?} ({}ms)",
                        output.confirm_result,
                        duration.as_millis()
                    )
                } else {
                    format!(
                        "AMQP 发布Failed: {} ({}ms)",
                        output.error.as_deref().unwrap_or("未知Error"),
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
                let port = if output.success { "true" } else { "false" };
                let next_node_ids = resolve_port_targets(connections, &node.uuid, port);

                Ok((next_node_ids, Some(output_data)))
            }
            Err(e) => {
                log::error!(
                    "[AMQP] 发布异常: uuid={}, exchange={:?}, routing_key={:?}, error={}",
                    node.uuid,
                    resolved_config.exchange,
                    resolved_config.routing_key,
                    e
                );

                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("AMQP 发布异常: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                let duration = start_time.elapsed();
                let error_output = json!({
                    "success": false,
                    "confirm_result": serde_json::Value::Null,
                    "returned_message": serde_json::Value::Null,
                    "error": e,
                    "request": {
                        "connection_id": resolved_config.connection_id,
                        "exchange": resolved_config.exchange,
                        "routing_key": resolved_config.routing_key,
                    },
                    "timing": {
                        "total_ms": duration.as_millis() as u64,
                    },
                });

                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("AMQP publish Failed: {}", e)));
                }

                // 返回 false 端口
                let next_node_ids = resolve_port_targets(connections, &node.uuid, "false");

                Ok((next_node_ids, Some(error_output)))
            }
        }
    }
}

/// 解析端口路由目标
fn resolve_port_targets(connections: &[Connection], node_uuid: &str, port: &str) -> Vec<String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node_uuid && conn.source_port_id == port)
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    if filtered.is_empty() {
        vec![port.to_string()]
    } else {
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::{
        Amqp10AcceptConfig, Amqp10ConnectConfig, Amqp10DisconnectConfig, Amqp10ReceiveConfig,
        Amqp10RejectConfig, Amqp10SendConfig, AmqpAckConfig, AmqpCloseConfig,
        AmqpConnectConfig, AmqpConsumeConfig, AmqpExchangeDeclareConfig, AmqpExchangeDeleteConfig,
        AmqpGetConfig, AmqpManagementConfig, AmqpPublishConfig, AmqpQueueBindConfig,
        AmqpQueueDeclareConfig, AmqpQueueDeleteConfig,
    };
    use flow_engine_core::traits::{AmqpConnectOutput, AmqpConnectionInfo, AmqpPublishOutput};

    /// Mock AmqpProvider（发布Success）
    struct MockAmqpProvider {
        output: AmqpPublishOutput,
    }

    impl MockAmqpProvider {
        fn new_success(confirm_result: Option<&str>) -> Self {
            Self {
                output: AmqpPublishOutput {
                    success: true,
                    confirm_result: confirm_result.map(|s| s.to_string()),
                    returned_message: None,
                    error: None,
                },
            }
        }

        fn new_failure(error: &str) -> Self {
            Self {
                output: AmqpPublishOutput {
                    success: false,
                    confirm_result: None,
                    returned_message: None,
                    error: Some(error.to_string()),
                },
            }
        }

        fn new_with_returned_message() -> Self {
            Self {
                output: AmqpPublishOutput {
                    success: true,
                    confirm_result: Some("ack".to_string()),
                    returned_message: Some(json!({
                        "reply_code": 312,
                        "reply_text": "NO_ROUTE",
                        "exchange": "",
                        "routing_key": "non-existent",
                        "body": "test message"
                    })),
                    error: None,
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl flow_engine_core::traits::AmqpProvider for MockAmqpProvider {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<AmqpConnectOutput, String> {
            Ok(AmqpConnectOutput {
                success: true,
                connection_id: "test-conn".to_string(),
                server_properties: None,
                error: None,
            })
        }

        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> {
            Ok(())
        }

        async fn exchange_declare(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn exchange_delete(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeleteConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_declare(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpQueueDeclareOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_delete(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeleteConfig,
        ) -> Result<flow_engine_core::traits::AmqpQueueDeleteOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> {
            Ok(())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &AmqpPublishConfig,
        ) -> Result<AmqpPublishOutput, String> {
            Ok(self.output.clone())
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &AmqpConsumeConfig,
        ) -> Result<flow_engine_core::traits::AmqpConsumeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn get(
            &self,
            _uuid: &str,
            _config: &AmqpGetConfig,
        ) -> Result<flow_engine_core::traits::AmqpGetOutput, String> {
            Err("not implemented".to_string())
        }

        async fn ack(
            &self,
            _uuid: &str,
            _config: &AmqpAckConfig,
        ) -> Result<flow_engine_core::traits::AmqpAckOutput, String> {
            Err("not implemented".to_string())
        }

        async fn management(
            &self,
            _uuid: &str,
            _config: &AmqpManagementConfig,
        ) -> Result<flow_engine_core::traits::AmqpManagementOutput, String> {
            Err("not implemented".to_string())
        }

        async fn amqp10_connect(
            &self,
            _uuid: &str,
            _config: &Amqp10ConnectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ConnectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_send(
            &self,
            _uuid: &str,
            _config: &Amqp10SendConfig,
        ) -> Result<flow_engine_core::traits::Amqp10SendOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_receive(
            &self,
            _uuid: &str,
            _config: &Amqp10ReceiveConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ReceiveOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_accept(
            &self,
            _uuid: &str,
            _config: &Amqp10AcceptConfig,
        ) -> Result<flow_engine_core::traits::Amqp10AcceptOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_reject(
            &self,
            _uuid: &str,
            _config: &Amqp10RejectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10RejectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_disconnect(
            &self,
            _uuid: &str,
            _config: &Amqp10DisconnectConfig,
        ) -> Result<(), String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            Some("connected".to_string())
        }

        fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
            vec![]
        }
    }

    /// Mock AmqpProvider（publish 返回 Err）
    struct MockAmqpProviderError;

    #[async_trait::async_trait]
    impl flow_engine_core::traits::AmqpProvider for MockAmqpProviderError {
        async fn connect(
            &self,
            _uuid: &str,
            _config: &AmqpConnectConfig,
        ) -> Result<AmqpConnectOutput, String> {
            Ok(AmqpConnectOutput {
                success: true,
                connection_id: "test".to_string(),
                server_properties: None,
                error: None,
            })
        }

        async fn close(&self, _uuid: &str, _config: &AmqpCloseConfig) -> Result<(), String> {
            Ok(())
        }

        async fn exchange_declare(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn exchange_delete(
            &self,
            _uuid: &str,
            _config: &AmqpExchangeDeleteConfig,
        ) -> Result<flow_engine_core::traits::AmqpExchangeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_declare(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeclareConfig,
        ) -> Result<flow_engine_core::traits::AmqpQueueDeclareOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_delete(
            &self,
            _uuid: &str,
            _config: &AmqpQueueDeleteConfig,
        ) -> Result<flow_engine_core::traits::AmqpQueueDeleteOutput, String> {
            Err("not implemented".to_string())
        }

        async fn queue_bind(&self, _uuid: &str, _config: &AmqpQueueBindConfig) -> Result<(), String> {
            Ok(())
        }

        async fn publish(
            &self,
            _uuid: &str,
            _config: &AmqpPublishConfig,
        ) -> Result<AmqpPublishOutput, String> {
            Err("channel closed".to_string())
        }

        async fn consume(
            &self,
            _uuid: &str,
            _config: &AmqpConsumeConfig,
        ) -> Result<flow_engine_core::traits::AmqpConsumeOutput, String> {
            Err("not implemented".to_string())
        }

        async fn get(
            &self,
            _uuid: &str,
            _config: &AmqpGetConfig,
        ) -> Result<flow_engine_core::traits::AmqpGetOutput, String> {
            Err("not implemented".to_string())
        }

        async fn ack(
            &self,
            _uuid: &str,
            _config: &AmqpAckConfig,
        ) -> Result<flow_engine_core::traits::AmqpAckOutput, String> {
            Err("not implemented".to_string())
        }

        async fn management(
            &self,
            _uuid: &str,
            _config: &AmqpManagementConfig,
        ) -> Result<flow_engine_core::traits::AmqpManagementOutput, String> {
            Err("not implemented".to_string())
        }

        async fn amqp10_connect(
            &self,
            _uuid: &str,
            _config: &Amqp10ConnectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ConnectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_send(
            &self,
            _uuid: &str,
            _config: &Amqp10SendConfig,
        ) -> Result<flow_engine_core::traits::Amqp10SendOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_receive(
            &self,
            _uuid: &str,
            _config: &Amqp10ReceiveConfig,
        ) -> Result<flow_engine_core::traits::Amqp10ReceiveOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_accept(
            &self,
            _uuid: &str,
            _config: &Amqp10AcceptConfig,
        ) -> Result<flow_engine_core::traits::Amqp10AcceptOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_reject(
            &self,
            _uuid: &str,
            _config: &Amqp10RejectConfig,
        ) -> Result<flow_engine_core::traits::Amqp10RejectOutput, String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        async fn amqp10_disconnect(
            &self,
            _uuid: &str,
            _config: &Amqp10DisconnectConfig,
        ) -> Result<(), String> {
            Err("AMQP 1.0 尚未实现".to_string())
        }

        fn get_status(&self, _uuid: &str) -> Option<String> {
            None
        }

        fn list_connections(&self) -> Vec<AmqpConnectionInfo> {
            vec![]
        }
    }

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, config: AmqpPublishConfig) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "amqp_publish".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::AmqpPublish(config),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建默认发布配置
    fn default_publish_config() -> AmqpPublishConfig {
        AmqpPublishConfig {
            connection_id: "test-conn".to_string(),
            exchange: Some("test-exchange".to_string()),
            routing_key: Some("test.routing.key".to_string()),
            body: Some("hello world".to_string()),
            body_type: Some("text".to_string()),
            ..AmqpPublishConfig::default()
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
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
        }
    }

    /// 测试：发布Success时输出 JSON 结构正确
    #[tokio::test]
    async fn test_amqp_publish_success_output_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(None));
        let executor = AmqpPublishExecutor::new(amqp_manager);
        let node = create_test_node("test-1", "测试发布", default_publish_config());
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // Success应路由到 true 端口
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert!(data["confirm_result"].is_null()); // none 模式
        assert!(data["returned_message"].is_null());
        assert!(data["error"].is_null());
        assert!(data["timing"]["total_ms"].is_number());
    }

    /// 测试：节点数据类型不匹配时返回Error
    #[tokio::test]
    async fn test_amqp_publish_wrong_node_data_type() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(None));
        let executor = AmqpPublishExecutor::new(amqp_manager);

        let node = Node {
            uuid: "test-wrong".to_string(),
            node_type: "http".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Entry(flow_engine_core::node_config::EntryConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("AmqpPublishConfig"));
    }

    /// 测试：变量替换在 exchange/routing_key/body 中生效
    #[tokio::test]
    async fn test_amqp_publish_variable_replacement() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(None));
        let executor = AmqpPublishExecutor::new(amqp_manager);

        let config = AmqpPublishConfig {
            connection_id: "conn-1".to_string(),
            exchange: Some("{{exchange_name}}".to_string()),
            routing_key: Some("{{region}}.orders".to_string()),
            body: Some("order-{{order_id}}".to_string()),
            message_id: Some("msg-{{order_id}}".to_string()),
            ..AmqpPublishConfig::default()
        };
        let node = create_test_node("test-var", "变量替换", config);

        let mut context = create_context("test-flow");
        context
            .variables
            .set_variable("exchange_name".to_string(), json!("orders"), String::new());
        context
            .variables
            .set_variable("region".to_string(), json!("cn"), String::new());
        context
            .variables
            .set_variable("order_id".to_string(), json!("12345"), String::new());

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // 验证变量替换生效
        assert_eq!(data["request"]["exchange"], "orders");
        assert_eq!(data["request"]["routing_key"], "cn.orders");
        assert_eq!(data["request"]["body"], "order-12345");
        assert_eq!(data["request"]["message_id"], "msg-12345");
    }

    /// 测试：发布Failed时路由到 false 端口
    #[tokio::test]
    async fn test_amqp_publish_failure_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_failure("exchange not found"));
        let executor = AmqpPublishExecutor::new(amqp_manager);
        let node = create_test_node("test-2", "Failed测试", default_publish_config());
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        // Failed应路由到 false 端口
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "exchange not found");
    }

    /// 测试：confirm 确认模式（publisher confirms）
    #[tokio::test]
    async fn test_amqp_publish_confirm_mode() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(Some("ack")));
        let executor = AmqpPublishExecutor::new(amqp_manager);

        let config = AmqpPublishConfig {
            connection_id: "conn-1".to_string(),
            exchange: Some("test-exchange".to_string()),
            routing_key: Some("test.key".to_string()),
            body: Some("test body".to_string()),
            confirm_mode: "confirm".to_string(),
            ..AmqpPublishConfig::default()
        };
        let node = create_test_node("test-confirm", "确认模式", config);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["confirm_result"], "ack");
        assert_eq!(data["request"]["confirm_mode"], "confirm");
    }

    /// 测试：transaction 事务模式
    #[tokio::test]
    async fn test_amqp_publish_transaction_mode() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(None));
        let executor = AmqpPublishExecutor::new(amqp_manager);

        let config = AmqpPublishConfig {
            connection_id: "conn-1".to_string(),
            exchange: Some("test-exchange".to_string()),
            routing_key: Some("test.key".to_string()),
            body: Some("transactional body".to_string()),
            confirm_mode: "transaction".to_string(),
            ..AmqpPublishConfig::default()
        };
        let node = create_test_node("test-tx", "事务模式", config);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();
        assert_eq!(data["request"]["confirm_mode"], "transaction");
    }

    /// 测试：14 种消息属性全部传递
    #[tokio::test]
    async fn test_amqp_publish_all_message_properties() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(None));
        let executor = AmqpPublishExecutor::new(amqp_manager);

        let mut headers = HashMap::new();
        headers.insert("x-custom-header".to_string(), json!("custom-value"));
        headers.insert("x-trace-id".to_string(), json!("trace-123"));

        let config = AmqpPublishConfig {
            connection_id: "conn-1".to_string(),
            exchange: Some("my-exchange".to_string()),
            routing_key: Some("my.routing.key".to_string()),
            body: Some("{\"key\":\"value\"}".to_string()),
            body_type: Some("json".to_string()),
            content_type: Some("application/json".to_string()),
            content_encoding: Some("utf-8".to_string()),
            delivery_mode: 2,         // 持久化
            priority: 5,
            message_id: Some("msg-001".to_string()),
            correlation_id: Some("corr-001".to_string()),
            reply_to: Some("amq.rabbitmq.reply-to".to_string()), // Direct Reply-To
            expiration: Some("60000".to_string()), // 60秒 TTL
            timestamp: Some(1714000000),
            message_type: Some("OrderCreated".to_string()),
            user_id: Some("guest".to_string()),
            app_id: Some("my-app".to_string()),
            cluster_id: Some("cluster-01".to_string()),
            headers: Some(headers),
            mandatory: true,
            confirm_mode: "confirm".to_string(),
        };
        let node = create_test_node("test-props", "全部属性", config);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        let request = &data["request"];
        assert_eq!(request["connection_id"], "conn-1");
        assert_eq!(request["exchange"], "my-exchange");
        assert_eq!(request["routing_key"], "my.routing.key");
        assert_eq!(request["body"], "{\"key\":\"value\"}");
        assert_eq!(request["body_type"], "json");
        assert_eq!(request["content_type"], "application/json");
        assert_eq!(request["content_encoding"], "utf-8");
        assert_eq!(request["delivery_mode"], 2);
        assert_eq!(request["priority"], 5);
        assert_eq!(request["message_id"], "msg-001");
        assert_eq!(request["correlation_id"], "corr-001");
        assert_eq!(request["reply_to"], "amq.rabbitmq.reply-to");
        assert_eq!(request["expiration"], "60000");
        assert_eq!(request["timestamp"], 1714000000);
        assert_eq!(request["message_type"], "OrderCreated");
        assert_eq!(request["user_id"], "guest");
        assert_eq!(request["app_id"], "my-app");
        assert_eq!(request["cluster_id"], "cluster-01");
        assert_eq!(request["mandatory"], true);
        assert_eq!(request["confirm_mode"], "confirm");

        // 验证 headers
        let headers_val = &request["headers"];
        assert_eq!(headers_val["x-custom-header"], "custom-value");
        assert_eq!(headers_val["x-trace-id"], "trace-123");
    }

    /// 测试：mandatory 标志 + returned_message
    #[tokio::test]
    async fn test_amqp_publish_mandatory_with_returned_message() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_with_returned_message());
        let executor = AmqpPublishExecutor::new(amqp_manager);

        let config = AmqpPublishConfig {
            connection_id: "conn-1".to_string(),
            exchange: Some("".to_string()),
            routing_key: Some("non-existent".to_string()),
            body: Some("test message".to_string()),
            mandatory: true,
            ..AmqpPublishConfig::default()
        };
        let node = create_test_node("test-mandatory", "强制路由", config);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        assert_eq!(data["success"], true);
        assert_eq!(data["confirm_result"], "ack");
        // mandatory 模式下消息被退回
        assert!(data["returned_message"].is_object());
        assert_eq!(data["returned_message"]["reply_code"], 312);
        assert_eq!(data["returned_message"]["reply_text"], "NO_ROUTE");
    }

    /// 测试：provider publish 方法返回 Err 时走 false 端口
    #[tokio::test]
    async fn test_amqp_publish_provider_error_routes_to_false() {
        let amqp_manager = Arc::new(MockAmqpProviderError);
        let executor = AmqpPublishExecutor::new(amqp_manager);
        let node = create_test_node("test-err", "异常测试", default_publish_config());
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "channel closed");
    }

    /// 测试：端口路由（true 端口连接到下一个节点）
    #[tokio::test]
    async fn test_amqp_publish_connections_routing() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(None));
        let executor = AmqpPublishExecutor::new(amqp_manager);
        let node = create_test_node("test-route", "路由测试", default_publish_config());
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
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-node"]);
    }

    /// 测试：报告结构包含所有必要字段
    #[tokio::test]
    async fn test_amqp_publish_report_structure() {
        let amqp_manager = Arc::new(MockAmqpProvider::new_success(Some("ack")));
        let executor = AmqpPublishExecutor::new(amqp_manager);

        let config = AmqpPublishConfig {
            connection_id: "conn-report".to_string(),
            exchange: Some("report-exchange".to_string()),
            routing_key: Some("report.key".to_string()),
            body: Some("report body".to_string()),
            confirm_mode: "confirm".to_string(),
            ..AmqpPublishConfig::default()
        };
        let node = create_test_node("test-report", "报告结构", config);
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.expect("应有输出数据");

        // 验证顶层字段
        assert!(data.get("success").is_some());
        assert!(data.get("confirm_result").is_some());
        assert!(data.get("returned_message").is_some());
        assert!(data.get("error").is_some());
        assert!(data.get("request").is_some());
        assert!(data.get("timing").is_some());

        // 验证 timing 子结构
        let timing = &data["timing"];
        assert!(timing["total_ms"].is_number());
    }
}
