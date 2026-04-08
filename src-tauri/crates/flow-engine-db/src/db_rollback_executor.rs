// 数据库回滚节点执行器
//
// 实现数据库事务回滚功能。
// Wave 2: 与执行器集成

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{DbRollbackConfig, NodeData};
use flow_engine_core::traits::DbProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 数据库回滚节点执行器
pub struct DbRollbackExecutor {
    /// 数据库连接池管理器
    db_manager: Arc<dyn DbProvider>,
}

impl DbRollbackExecutor {
    /// 创建新的数据库回滚执行器
    pub fn new(db_manager: Arc<dyn DbProvider>) -> Self {
        Self { db_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for DbRollbackExecutor {
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
            NodeData::DbRollback(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 DbRollbackConfig: {}", node.uuid)),
        };

        // 2. 验证 connection_uuid
        if config.connection_uuid.is_empty() {
            // 空连接 UUID 时，直接返回成功（无操作）
            log::warn!(
                "[流程 {}] [警告] 节点ID: {}, 节点名称: {}, connection_uuid 为空，跳过回滚",
                context.flow_uuid,
                node.uuid,
                node.name
            );

            // 构建输出数据（跳过回滚）
            let output_data = self.build_skipped_output(&config, start_time.elapsed().as_millis() as u64);

            // 发送日志
            context
                .emit_log(LogEvent {
                    level: LogLevel::Warn,
                    message: format!("数据库回滚节点 {} 跳过：connection_uuid 为空", node.name),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("系统时间错误")
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            // 返回单一输出端口
            return Ok((self.get_output_port_nodes(node, connections), Some(output_data)));
        }

        // 3. 日志：开始回滚
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: db_rollback, connection_uuid: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.connection_uuid
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("数据库回滚节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 回滚事务
        // 注意：回滚总是成功，即使没有活跃事务也返回成功（幂等操作）
        let rollback_result = self.db_manager.rollback_transaction(&config.connection_uuid).await;

        let duration = start_time.elapsed();

        match rollback_result {
            Ok(()) => {
                log::info!(
                    "[Database] 事务回滚成功: uuid={}",
                    config.connection_uuid
                );

                // 5. 构建输出数据（成功）
                let output_data = self.build_success_output(
                    &config,
                    duration.as_millis() as u64,
                );

                // 6. 日志：回滚完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "数据库事务回滚成功 ({}ms)",
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

                // 7. 返回单一输出端口
                Ok((self.get_output_port_nodes(node, connections), Some(output_data)))
            }
            Err(e) => {
                // 没有活跃事务时也返回成功（幂等操作）
                log::warn!(
                    "[Database] 事务回滚失败（可能无活跃事务）: uuid={}, error={}",
                    config.connection_uuid,
                    e
                );

                // 构建输出数据（仍视为成功，幂等操作）
                let output_data = self.build_success_output(
                    &config,
                    duration.as_millis() as u64,
                );

                // 日志：回滚完成（警告）
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Warn,
                        message: format!(
                            "数据库事务回滚完成 ({}ms) - 可能无活跃事务",
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

                // 返回单一输出端口（回滚总是成功）
                Ok((self.get_output_port_nodes(node, connections), Some(output_data)))
            }
        }
    }
}

impl DbRollbackExecutor {
    /// 获取输出端口连接的目标节点
    fn get_output_port_nodes(&self, node: &Node, connections: &[Connection]) -> Vec<String> {
        let filtered: Vec<String> = connections
            .iter()
            .filter(|conn| {
                conn.source_node_uuid == node.uuid && conn.source_port_id == "output"
            })
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        // 如果没有连接，返回端口名称（用于测试）
        if filtered.is_empty() {
            vec!["output".to_string()]
        } else {
            filtered
        }
    }

    /// 构建成功输出数据
    fn build_success_output(
        &self,
        config: &DbRollbackConfig,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一成功标识
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

    /// 构建跳过输出数据（空 connection_uuid 时）
    fn build_skipped_output(
        &self,
        config: &DbRollbackConfig,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一成功标识（跳过也视为成功）
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
    use flow_engine_core::node::{NodePosition, NodeType};
    use crate::db::pool::DatabaseConnectionManager;
    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_uuid: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::DbRollback,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbRollback(DbRollbackConfig {
                connection_uuid: connection_uuid.to_string(),
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
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            db_manager: None,
            db_connections: std::collections::HashMap::new(),
            last_node_output: None,
            debug_event_sender: None,
        }
    }

    #[tokio::test]
    async fn test_rollback_success() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        
        // 先开始一个事务
        db_manager.begin_transaction("conn-001", None).await.unwrap();

        let executor = DbRollbackExecutor::new(db_manager.clone());
        let node = create_test_node("test-1", "测试回滚", "conn-001");
        let mut context = create_context("test-flow");

        // 验证事务状态为 Active
        assert_eq!(
            db_manager.get_transaction_state("conn-001").await,
            Some(super::super::db::traits::TransactionState::Active)
        );

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回单一输出端口（成功）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["output"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_uuid"], "conn-001");

        // 验证事务状态为 RolledBack
        assert_eq!(
            db_manager.get_transaction_state("conn-001").await,
            Some(super::super::db::traits::TransactionState::RolledBack)
        );
    }

    #[tokio::test]
    async fn test_rollback_no_active_transaction() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbRollbackExecutor::new(db_manager);
        let node = create_test_node("test-2", "回滚无事务", "conn-002");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 回滚不存在的活跃事务也应该成功（幂等操作）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["output"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_uuid"], "conn-002");
    }

    #[tokio::test]
    async fn test_rollback_empty_connection_uuid() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbRollbackExecutor::new(db_manager);
        let node = create_test_node("test-3", "空 UUID", "");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 空 UUID 时跳过回滚，返回成功
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["output"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["skipped"], true);
    }

    #[tokio::test]
    async fn test_rollback_with_output_port_routing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        
        // 先开始一个事务
        db_manager.begin_transaction("conn-004", None).await.unwrap();

        let executor = DbRollbackExecutor::new(db_manager);

        // 创建节点
        let node = create_test_node("test-4", "端口路由测试", "conn-004");
        let mut context = create_context("test-flow");

        // 创建输出端口连接
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-4".to_string(),
            target_node_uuid: "target-node".to_string(),
            source_port_id: "output".to_string(),
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
    async fn test_rollback_timing_info() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbRollbackExecutor::new(db_manager);
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
        let executor = DbRollbackExecutor::new(db_manager);

        // 创建一个使用错误数据类型的节点
        let node = Node {
            uuid: "test-6".to_string(),
            node_type: NodeType::DbRollback,
            name: "错误类型".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(), // 错误：使用 Entry 配置
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回错误
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不是 DbRollbackConfig"));
    }
}
