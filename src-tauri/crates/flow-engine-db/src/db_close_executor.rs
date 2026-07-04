// 数据库关闭节点执行器
//
// 实现数据库连接池关闭功能。
// Wave 2: 与执行器集成

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{DbCloseConfig, NodeData};
use flow_engine_core::traits::DbProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 数据库关闭节点执行器
pub struct DbCloseExecutor {
    /// 数据库连接池管理器
    db_manager: Arc<dyn DbProvider>,
}

impl DbCloseExecutor {
    /// 创建新的数据库关闭执行器
    pub fn new(db_manager: Arc<dyn DbProvider>) -> Self {
        Self { db_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for DbCloseExecutor {
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
            NodeData::DbClose(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not DbCloseConfig: {}", node.uuid))),
        };

        // 2. 验证 connection_uuid
        if config.connection_uuid.is_empty() {
            // 空连接 UUID 时，直接返回Success（无操作）
            log::warn!(
                "[Flow {}] [Warn] NodeID: {}, NodeName: {}, connection_uuid is empty, skipping close",
                context.flow_uuid,
                node.uuid,
                node.name
            );

            // 构建输出数据（跳过关闭）
            let output_data = self.build_skipped_output(&config, start_time.elapsed().as_millis() as u64);

            // 发送日志
            context
                .emit_log(LogEvent {
                    level: LogLevel::Warn,
                    message: format!("Database close node {} skipped: connection_uuid is empty", node.name),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("system time error")
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            // 返回单一输出端口
            return Ok((self.get_output_port_nodes(node, connections), Some(output_data)));
        }

        // 3. 日志：开始关闭
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: db_close, connection_uuid: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.connection_uuid
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Database close node {} start execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 关闭连接池
        // 注意：关闭总是Success，即使连接池不存在也返回Success
        let close_result = self.db_manager.close_pool(&config.connection_uuid).await;

        let duration = start_time.elapsed();

        match close_result {
            Ok(()) => {
                log::info!(
                    "[Database] Pool closed: uuid={}",
                    config.connection_uuid
                );

                // 5. 构建输出数据（Success）
                let output_data = self.build_success_output(
                    &config,
                    duration.as_millis() as u64,
                );

                // 6. 日志：关闭完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "Database pool closed ({}ms)",
                            duration.as_millis()
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

                // 7. 返回单一输出端口
                Ok((self.get_output_port_nodes(node, connections), Some(output_data)))
            }
            Err(e) => {
                // 连接池不存在时也返回Success（幂等操作）
                log::warn!(
                    "[Database] Pool close failed (may already be closed): uuid={}, error={}",
                    config.connection_uuid,
                    e
                );

                // 构建输出数据（仍视为Success，幂等操作）
                let output_data = self.build_success_output(
                    &config,
                    duration.as_millis() as u64,
                );

                // 日志：关闭完成（警告）
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Warn,
                        message: format!(
                            "Database pool closed ({}ms) - pool may already be closed",
                            duration.as_millis()
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

                // 返回单一输出端口（关闭总是Success）
                Ok((self.get_output_port_nodes(node, connections), Some(output_data)))
            }
        }
    }
}

impl DbCloseExecutor {
    /// 获取输出端口连接的目标节点
    fn get_output_port_nodes(&self, node: &Node, connections: &[Connection]) -> Vec<String> {
        let filtered: Vec<String> = connections
            .iter()
            .filter(|conn| {
                conn.source_node_uuid == node.uuid && conn.source_port_id == "out"
            })
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        // 如果没有连接，返回端口名称（用于测试）
        if filtered.is_empty() {
            vec!["out".to_string()]
        } else {
            filtered
        }
    }

    /// 构建Success输出数据
    fn build_success_output(
        &self,
        config: &DbCloseConfig,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一Success标识
            "success": true,
            "connection_uuid": config.connection_uuid,
            // 请求配置
            "request": {
                "connection_uuid": config.connection_uuid,
            },
            // 耗时分解
            "timing": {
                "total_ms": total_ms,
            },
        })
    }

    /// 构建跳过输出数据（空 `connection_uuid` 时）
    fn build_skipped_output(
        &self,
        config: &DbCloseConfig,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一Success标识（跳过也视为Success）
            "success": true,
            "skipped": true,
            "connection_uuid": config.connection_uuid,
            // 请求配置
            "request": {
                "connection_uuid": config.connection_uuid,
            },
            // 耗时分解
            "timing": {
                "total_ms": total_ms,
            },
        })
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::{NodePosition};
    use crate::db::pool::DatabaseConnectionManager;
    use crate::db::pool::MockPool;

    /// 创建Test Node
    fn create_test_node(uuid: &str, name: &str, connection_uuid: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "db_close".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbClose(DbCloseConfig {
                connection_uuid: connection_uuid.to_string(),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
                .expect("system time error")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_close_success() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        
        // 先创建一个连接池
        let pool = Arc::new(MockPool::new("test-pool".to_string()));
        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool,
            )
            .await
            .unwrap();

        let executor = DbCloseExecutor::new(db_manager.clone());
        let node = create_test_node("test-1", "测试关闭", "conn-001");
        let mut context = create_context("test-flow");

        // 验证连接池存在
        assert_eq!(db_manager.pool_count().await, 1);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回单一输出端口（Success）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_uuid"], "conn-001");

        // 验证连接池已关闭
        assert_eq!(db_manager.pool_count().await, 0);
    }

    #[tokio::test]
    async fn test_close_non_existent_pool() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCloseExecutor::new(db_manager);
        let node = create_test_node("test-2", "关闭不存在的连接", "conn-002");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 关闭不存在的连接池也应该Success（幂等操作）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_uuid"], "conn-002");
    }

    #[tokio::test]
    async fn test_close_empty_connection_uuid() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCloseExecutor::new(db_manager);
        let node = create_test_node("test-3", "空 UUID", "");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 空 UUID 时跳过关闭，返回Success
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["out"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["skipped"], true);
    }

    #[tokio::test]
    async fn test_close_with_output_port_routing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCloseExecutor::new(db_manager);

        // 创建节点
        let node = create_test_node("test-4", "端口路由测试", "conn-004");
        let mut context = create_context("test-flow");

        // 创建输出端口连接
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-4".to_string(),
            target_node_uuid: "target-node".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 应该返回输出端口连接的目标节点
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["target-node"]);
    }

    #[tokio::test]
    async fn test_close_timing_info() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCloseExecutor::new(db_manager);
        let node = create_test_node("test-5", "计时测试", "conn-005");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // 验证 timing 信息存在
        assert!(data["timing"].is_object());
        assert!(data["timing"]["total_ms"].is_number());
    }

    #[tokio::test]
    async fn test_invalid_node_data_type() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCloseExecutor::new(db_manager);

        // 创建一个使用Error数据类型的节点
        let node = Node {
            uuid: "test-6".to_string(),
            node_type: "db_close".to_string(),
            name: "Error类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(), // Error：使用 Entry 配置
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回Error
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Node data type is not DbCloseConfig"));
    }
}
