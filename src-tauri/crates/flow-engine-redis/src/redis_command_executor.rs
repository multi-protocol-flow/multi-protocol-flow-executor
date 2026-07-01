//! Redis 命令执行器

use async_trait::async_trait;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::connection::Connection;
use flow_engine_core::executor::{NodeExecutor, replace_variables};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::ExecutionErrorStrategy;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::RedisProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Redis 命令执行器
pub struct RedisCommandExecutor {
    redis_manager: Arc<dyn RedisProvider>,
}

impl RedisCommandExecutor {
    /// Create a new `RedisCommandExecutor`.
    ///
    /// # Arguments
    ///
    /// * `redis_manager` - The Redis connection manager used to execute commands.
    pub fn new(redis_manager: Arc<dyn RedisProvider>) -> Self {
        Self { redis_manager }
    }
}

#[async_trait]
impl NodeExecutor for RedisCommandExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        let config = match &node.data {
            NodeData::RedisCommand(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error("[E_REDIS_COMMAND] Node data is not RedisCommandConfig".to_string())),
        };

        // 变量替换
        let connection_id = replace_variables(&config.connection_id, context).map_err(ErrorDetail::execution_error)?;
        let command = replace_variables(&config.command, context).map_err(ErrorDetail::execution_error)?;
        let args: Vec<String> = config
            .args
            .iter()
            .map(|a| replace_variables(a, context))
            .collect::<Result<Vec<_>, String>>().map_err(|e: String| ErrorDetail::execution_error(e))?;

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Executing Redis command: {} {:?}", command, args),
                node_uuid: Some(node.uuid.clone()),
                ..Default::default()
            })
            .await;

        let result = match self
            .redis_manager
            .command(&connection_id, &command, &args)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                // 检查 on_error 策略：AbortFlow 时返回错误而不是路由到 false 端口
                if !node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse) {
                    return Err(ErrorDetail::execution_error(format!("[E_REDIS_COMMAND] Redis command execution failed: {}", e)));
                }
                let elapsed = start_time.elapsed().as_millis() as u64;
                let error_output = json!({
                    "success": false,
                    "command": command,
                    "row_count": 0,
                    "data": serde_json::Value::Null,
                    "columns": serde_json::Value::Null,
                    "affected_rows": serde_json::Value::Null,
                    "truncated": false,
                    "error": e,
                    "timing_ms": elapsed,
                });
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("[E_REDIS_COMMAND] Redis command execution failed: {}", e),
                        node_uuid: Some(node.uuid.clone()),
                        ..Default::default()
                    })
                    .await;
                let next_node_ids = route_to_port(node, _connections, "false");
                return Ok((next_node_ids, Some(error_output)));
            }
        };

        let elapsed = start_time.elapsed().as_millis() as u64;

        let data = json!({
            "success": result.success,
            "command": result.command,
            "row_count": result.row_count,
            "data": result.data,
            "columns": result.columns,
            "affected_rows": result.affected_rows,
            "truncated": result.truncated,
            "error": result.error,
            "timing_ms": elapsed,
        });

        // 记录命令输出
        if let Some(ref err) = result.error {
            context
                .emit_log(LogEvent {
                    level: LogLevel::Error,
                    message: format!("Redis command failed: {}", err),
                    node_uuid: Some(node.uuid.clone()),
                    ..Default::default()
                })
                .await;
        }

        let next_port = if result.success { "true" } else { "false" };
        let next_node_ids = route_to_port(node, _connections, next_port);

        Ok((next_node_ids, Some(data)))
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
