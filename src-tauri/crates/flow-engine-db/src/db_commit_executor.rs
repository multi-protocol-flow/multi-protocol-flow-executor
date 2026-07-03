// 数据库提交节点执行器
//
// 实现数据库事务提交功能。
// Wave 2: 与执行器集成

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::{ExecutionErrorStrategy, Node};
use flow_engine_core::node_config::{DbCommitConfig, NodeData};
use flow_engine_core::traits::DbProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 数据库提交节点执行器
pub struct DbCommitExecutor {
    /// 数据库连接池管理器
    db_manager: Arc<dyn DbProvider>,
}

impl DbCommitExecutor {
    /// 创建新的数据库提交执行器
    pub fn new(db_manager: Arc<dyn DbProvider>) -> Self {
        Self { db_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for DbCommitExecutor {
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
            NodeData::DbCommit(config) => config.clone(),
            _ => return Err(ErrorDetail::execution_error(format!("Node data type is not DbCommitConfig: {}", node.uuid))),
        };

        // 2. 验证 connection_uuid
        if config.connection_uuid.is_empty() {
            // 空连接 UUID 时，直接返回Success（无操作）
            log::warn!(
                "[流程 {}] [警告] 节点ID: {}, NodeName: {}, connection_uuid 为空，跳过提交",
                context.flow_uuid,
                node.uuid,
                node.name
            );

            // 构建输出数据（跳过提交）
            let output_data = self.build_skipped_output(&config, start_time.elapsed().as_millis() as u64);

            // 发送日志
            context
                .emit_log(LogEvent {
                    level: LogLevel::Warn,
                    message: format!("数据库提交节点 {} 跳过：connection_uuid 为空", node.name),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("系统时间Error")
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            // 返回 true 端口（跳过也视为Success）
            let next_nodes = get_nodes_by_port(node, connections, "true");
            return Ok((next_nodes, Some(output_data)));
        }

        // 3. 日志：开始提交
        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: db_commit, connection_uuid: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.connection_uuid
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("数据库提交节点 {} start execution", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间Error")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 提交事务
        // 注意：提交总是Success，即使没有活跃事务也返回Success（幂等操作）
        let commit_result = self.db_manager.commit_transaction(&config.connection_uuid).await;

        let duration = start_time.elapsed();

        match commit_result {
            Ok(()) => {
                log::info!(
                    "[Database] 事务提交Success: uuid={}",
                    config.connection_uuid
                );

                // 5. 构建输出数据（Success）
                let output_data = self.build_success_output(
                    &config,
                    duration.as_millis() as u64,
                );

                // 6. 日志：提交完成
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "数据库事务提交Success ({}ms)",
                            duration.as_millis()
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间Error")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 7. 返回 true 端口（提交Success）
                let next_nodes = get_nodes_by_port(node, connections, "true");
                Ok((next_nodes, Some(output_data)))
            }
            Err(e) => {
                let error_msg = format!("事务提交Failed: {}", e);
                log::warn!(
                    "[Database] 事务提交Failed: uuid={}, error={}",
                    config.connection_uuid,
                    e
                );

                // 根据 on_error 策略决定路由行为
                match node.on_error {
                    Some(ExecutionErrorStrategy::AbortFlow) => {
                        // 中止流程
                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: error_msg.clone(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("系统时间Error")
                                    .as_millis() as u64,
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        return Err(ErrorDetail::execution_error(error_msg));
                    }
                    _ => {
                        // RouteToFalse（或 None）：路由到 false 端口，仍视为幂等Success
                        log::warn!(
                            "[Database] 事务提交Failed，路由到 false 端口: uuid={}, error={}",
                            config.connection_uuid,
                            e
                        );

                        // 构建输出数据（仍视为Success，幂等操作）
                        let output_data = self.build_success_output(
                            &config,
                            duration.as_millis() as u64,
                        );

                        // 日志：提交Failed（警告）
                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Warn,
                                message: format!(
                                    "数据库事务提交Failed，路由到 false 端口 ({}ms)",
                                    duration.as_millis()
                                ),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("系统时间Error")
                                    .as_millis() as u64,
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        let next_nodes = get_nodes_by_port(node, connections, "false");
                        Ok((next_nodes, Some(output_data)))
                    }
                }
            }
        }
    }
}

impl DbCommitExecutor {
    /// 构建Success输出数据
    fn build_success_output(
        &self,
        config: &DbCommitConfig,
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
        config: &DbCommitConfig,
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
// 端口路由辅助函数
// ============================================================================

/// 根据端口 ID 过滤连接，返回目标节点 UUID 列表
fn get_nodes_by_port(node: &Node, connections: &[Connection], port_id: &str) -> Vec<String> {
    connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == port_id)
        .map(|conn| conn.target_node_uuid.clone())
        .collect()
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
            node_type: "db_commit".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbCommit(DbCommitConfig {
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
                .expect("系统时间Error")
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
    async fn test_commit_success() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let pool = Arc::new(MockPool::new("test-pool".to_string()));
        db_manager
            .create_pool("conn-001".to_string(), "Test".to_string(), "mock".to_string(), pool)
            .await
            .unwrap();
        
        // 先开始一个事务
        db_manager
            .begin_transaction("conn-001", None)
            .await
            .unwrap();

        let executor = DbCommitExecutor::new(db_manager.clone());
        let node = create_test_node("test-1", "测试提交", "conn-001");
        let mut context = create_context("test-flow");

        // 验证事务状态为 Active
        let state = db_manager.get_transaction_state("conn-001").await;
        assert_eq!(state, Some(crate::db::traits::TransactionState::Active));

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 true 端口（Success）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert!(next_nodes.is_empty());
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_uuid"], "conn-001");

        // 验证事务状态为 Committed
        let state = db_manager.get_transaction_state("conn-001").await;
        assert_eq!(state, Some(crate::db::traits::TransactionState::Committed));
    }

    #[tokio::test]
    async fn test_commit_no_active_transaction() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCommitExecutor::new(db_manager);
        let node = create_test_node("test-2", "提交无事务", "conn-002");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 提交不存在的事务，路由到 false 端口（幂等操作）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert!(next_nodes.is_empty());
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_uuid"], "conn-002");
    }

    #[tokio::test]
    async fn test_commit_empty_connection_uuid() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCommitExecutor::new(db_manager);
        let node = create_test_node("test-3", "空 UUID", "");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 空 UUID 时跳过提交，路由到 true 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert!(next_nodes.is_empty());
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["skipped"], true);
    }

    #[tokio::test]
    async fn test_commit_with_output_port_routing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let pool = Arc::new(MockPool::new("test-pool".to_string()));
        db_manager
            .create_pool("conn-004".to_string(), "Test".to_string(), "mock".to_string(), pool)
            .await
            .unwrap();
        
        // 先开始一个事务
        db_manager
            .begin_transaction("conn-004", None)
            .await
            .unwrap();

        let executor = DbCommitExecutor::new(db_manager);

        // 创建节点
        let node = create_test_node("test-4", "端口路由测试", "conn-004");
        let mut context = create_context("test-flow");

        // 创建 true 端口连接（Success路由）
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-4".to_string(),
            target_node_uuid: "target-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 应该返回 true 端口连接的目标节点
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["target-node"]);
    }

    #[tokio::test]
    async fn test_commit_with_false_port_routing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCommitExecutor::new(db_manager);

        // 创建节点（conn-xxx 不存在，提交会Failed）
        let node = create_test_node("test-false", "false 端口路由测试", "conn-nonexistent");
        let mut context = create_context("test-flow");

        // 创建 false 端口连接（Failed路由）
        let connections = vec![Connection {
            id: "conn-false".to_string(),
            source_node_uuid: "test-false".to_string(),
            target_node_uuid: "error-handler".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 应该返回 false 端口连接的目标节点（幂等但不阻断）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["error-handler"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
    }

    #[tokio::test]
    async fn test_commit_timing_info() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbCommitExecutor::new(db_manager);
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
        let executor = DbCommitExecutor::new(db_manager);

        // 创建一个使用Error数据类型的节点
        let node = Node {
            uuid: "test-6".to_string(),
            node_type: "db_commit".to_string(),
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
        assert!(result.unwrap_err().contains("不是 DbCommitConfig"));
    }
}
