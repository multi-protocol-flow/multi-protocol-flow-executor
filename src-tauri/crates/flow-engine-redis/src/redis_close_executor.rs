//! Redis 关闭连接执行器

use async_trait::async_trait;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::connection::Connection;
use flow_engine_core::executor::{NodeExecutor, replace_variables};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::RedisProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Redis 关闭连接执行器
pub struct RedisCloseExecutor {
    redis_manager: Arc<dyn RedisProvider>,
}

impl RedisCloseExecutor {
    /// Create a new `RedisCloseExecutor`.
    ///
    /// # Arguments
    ///
    /// * `redis_manager` - The Redis connection manager used to close connections.
    pub fn new(redis_manager: Arc<dyn RedisProvider>) -> Self {
        Self { redis_manager }
    }
}

#[async_trait]
impl NodeExecutor for RedisCloseExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();

        let config = match &node.data {
            NodeData::RedisClose(config) => config.clone(),
            _ => return Err("[E_REDIS_CONNECTION] Node data is not RedisCloseConfig".to_string()),
        };

        let connection_id = replace_variables(&config.connection_id, context)?;

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Closing Redis connection: {}", connection_id),
                node_uuid: Some(node.uuid.clone()),
                ..Default::default()
            })
            .await;

        let result = self
            .redis_manager
            .close(&connection_id)
            .await?;

        let elapsed = start_time.elapsed().as_millis() as u64;

        let output = json!({
            "success": result.success,
            "error": result.error,
            "timing_ms": elapsed,
        });

        let next_node_ids = route_to_port(node, _connections, "out");

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
