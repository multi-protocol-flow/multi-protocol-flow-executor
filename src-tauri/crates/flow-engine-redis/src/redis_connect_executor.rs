//! Redis 连接执行器

use async_trait::async_trait;
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::RedisProvider;
use flow_engine_core::ExecutionErrorStrategy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Redis 连接执行器
pub struct RedisConnectExecutor {
    redis_manager: Arc<dyn RedisProvider>,
}

impl RedisConnectExecutor {
    /// Create a new `RedisConnectExecutor`.
    ///
    /// # Arguments
    ///
    /// * `redis_manager` - The Redis connection manager used to establish connections.
    pub fn new(redis_manager: Arc<dyn RedisProvider>) -> Self {
        Self { redis_manager }
    }
}

#[async_trait]
impl NodeExecutor for RedisConnectExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        let config = match &node.data {
            NodeData::RedisConnect(config) => config.clone(),
            _ => {
                return Err(ErrorDetail::execution_error(
                    "[E_REDIS_CONNECTION] Node data is not RedisConnectConfig".to_string(),
                ))
            }
        };

        // 变量替换
        let host =
            replace_variables(&config.host, context).map_err(ErrorDetail::execution_error)?;
        let port_str = replace_variables(&config.port, context)
            .map_err(ErrorDetail::execution_error)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| ErrorDetail::execution_error(format!("Invalid Redis port: {}", port_str)))?;
        let password = config
            .password
            .as_ref()
            .map(|p| replace_variables(p, context))
            .transpose()
            .map_err(ErrorDetail::execution_error)?
            .filter(|p| !p.is_empty()); // 空密码转为 None，避免 redis-rs 发送 AUTH ""

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "Connecting to Redis: {}:{} (db: {})",
                    host, port, config.db_index
                ),
                node_uuid: Some(node.uuid.clone()),
                ..Default::default()
            })
            .await;

        let result = match self
            .redis_manager
            .connect(
                &node.uuid,
                &host,
                port,
                password.as_deref(),
                config.db_index,
                config.connect_timeout_ms,
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                // 检查 on_error 策略：AbortFlow 时返回Error而不是路由到 false 端口
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "[E_REDIS_CONNECTION] Redis connection failed: {}",
                        e
                    )));
                }
                let elapsed = start_time.elapsed().as_millis() as u64;
                let error_output = json!({
                    "success": false,
                    "connection_id": serde_json::Value::Null,
                    "server_info": serde_json::Value::Null,
                    "error": e,
                    "timing_ms": elapsed,
                });
                let next_node_ids = route_to_port(node, _connections, "false");
                return Ok((next_node_ids, Some(error_output)));
            }
        };

        let elapsed = start_time.elapsed().as_millis() as u64;

        let output = json!({
            "success": result.success,
            "connection_id": result.connection_id,
            "server_info": result.server_info,
            "error": result.error,
            "timing_ms": elapsed,
        });

        let next_port = if result.success { "true" } else { "false" };
        let next_node_ids = route_to_port(node, _connections, next_port);

        Ok((next_node_ids, Some(output)))
    }
}

/// 根据端口 ID 路由到下游节点
fn route_to_port(node: &Node, connections: &[Connection], port: &str) -> Vec<String> {
    let filtered: Vec<String> = connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == port)
        .map(|conn| conn.target_node_uuid.clone())
        .collect();

    if filtered.is_empty() {
        vec![port.to_string()]
    } else {
        filtered
    }
}
