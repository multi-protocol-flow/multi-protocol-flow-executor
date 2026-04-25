// 数据库事务开始节点执行器
//
// 实现数据库事务开始功能，支持隔离级别配置。
// Wave 2: 与执行器集成

use std::str::FromStr;

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use crate::db::traits::IsolationLevel;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{DbBeginConfig, NodeData};
use flow_engine_core::traits::DbProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 数据库事务开始节点执行器
pub struct DbBeginExecutor {
    /// 数据库连接池管理器
    db_manager: Arc<dyn DbProvider>,
}

impl DbBeginExecutor {
    /// 创建新的事务开始执行器
    pub fn new(db_manager: Arc<dyn DbProvider>) -> Self {
        Self { db_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for DbBeginExecutor {
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
            NodeData::DbBegin(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 DbBeginConfig: {}", node.uuid)),
        };

        // 2. 验证 connection_uuid
        if config.connection_uuid.is_empty() {
            let error_msg = "connection_uuid 不能为空".to_string();
            log::error!(
                "[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );

            // 返回 false 端口
            return Ok((
                self.get_false_port_nodes(node, connections),
                Some(self.build_error_output(&config, &error_msg, start_time.elapsed().as_millis() as u64)),
            ));
        }

        // 3. 日志：开始事务
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: db_begin, connection_uuid: {}, isolation_level: {:?}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.connection_uuid,
            config.isolation_level
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("事务开始节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 验证隔离级别（如果配置了）
        let isolation_level_str = config.isolation_level.clone();
        if let Some(ref level_str) = isolation_level_str {
            if !level_str.is_empty() {
                // 验证隔离级别是否有效
                if IsolationLevel::from_str(level_str).is_err() {
                    let error_msg = format!("无效的隔离级别: {}", level_str);
                    log::error!(
                        "[Database] 无效的隔离级别: uuid={}, level={}",
                        config.connection_uuid,
                        level_str
                    );

                    // 日志：隔离级别无效
                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Error,
                            message: format!("无效的隔离级别: {}", level_str),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("系统时间错误")
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;

                    // 返回 false 端口
                    return Ok((
                        self.get_false_port_nodes(node, connections),
                        Some(self.build_error_output(&config, &error_msg, start_time.elapsed().as_millis() as u64)),
                    ));
                }
            }
        }

        // 5. 开始事务
        let begin_result = self
            .db_manager
            .begin_transaction(&config.connection_uuid, isolation_level_str.clone())
            .await;

        match begin_result {
            Ok(()) => {
                log::info!(
                    "[Database] 事务开始成功: uuid={}, isolation_level={:?}",
                    config.connection_uuid,
                    isolation_level_str
                );

                // 6. 构建输出数据（成功）
                let output_data = self.build_success_output(
                    &config,
                    isolation_level_str,
                    start_time.elapsed().as_millis() as u64,
                );

                // 7. 日志：事务开始完成
                let duration = start_time.elapsed();
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!(
                            "事务开始成功 ({}ms)",
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

                // 8. 返回 true 端口
                Ok((
                    self.get_true_port_nodes(node, connections),
                    Some(output_data),
                ))
            }
            Err(e) => {
                // 事务开始失败
                log::error!(
                    "[Database] 事务开始失败: uuid={}, error={}",
                    config.connection_uuid,
                    e
                );

                // 日志：事务开始失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("事务开始失败: {}", e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 返回 false 端口
                Ok((
                    self.get_false_port_nodes(node, connections),
                    Some(self.build_error_output(&config, &e, start_time.elapsed().as_millis() as u64)),
                ))
            }
        }
    }
}

impl DbBeginExecutor {
    /// 获取 true 端口连接的目标节点
    fn get_true_port_nodes(&self, node: &Node, connections: &[Connection]) -> Vec<String> {
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
    }

    /// 获取 false 端口连接的目标节点
    fn get_false_port_nodes(&self, node: &Node, connections: &[Connection]) -> Vec<String> {
        let filtered: Vec<String> = connections
            .iter()
            .filter(|conn| {
                conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
            })
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        // 如果没有连接，返回端口名称（用于测试）
        if filtered.is_empty() {
            vec!["false".to_string()]
        } else {
            filtered
        }
    }

    /// 构建成功输出数据
    fn build_success_output(
        &self,
        config: &DbBeginConfig,
        isolation_level: Option<String>,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一成功标识
            "success": true,
            "connection_uuid": config.connection_uuid,
            "isolation_level": isolation_level,
            // 请求配置
            "request": {
                "connection_uuid": config.connection_uuid,
                "isolation_level": config.isolation_level,
            },
            // 耗时分解
            "timing": {
                "total_ms": total_ms,
            },
        })
    }

    /// 构建错误输出数据
    fn build_error_output(
        &self,
        config: &DbBeginConfig,
        error: &str,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一成功标识
            "success": false,
            "connection_uuid": config.connection_uuid,
            "isolation_level": config.isolation_level,
            // 请求配置
            "request": {
                "connection_uuid": config.connection_uuid,
                "isolation_level": config.isolation_level,
            },
            // 耗时分解
            "timing": {
                "total_ms": total_ms,
            },
            "error": error,
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
    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_uuid: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "db_begin".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbBegin(DbBeginConfig {
                connection_uuid: connection_uuid.to_string(),
                isolation_level: None,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建带隔离级别的节点
    fn create_test_node_with_isolation(
        uuid: &str,
        name: &str,
        connection_uuid: &str,
        isolation_level: &str,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "db_begin".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbBegin(DbBeginConfig {
                connection_uuid: connection_uuid.to_string(),
                isolation_level: Some(isolation_level.to_string()),
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
            grpc_manager: None,
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_begin_success() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager);
        let node = create_test_node("test-1", "测试事务开始", "conn-001");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 true 端口（事务开始成功）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connection_uuid"], "conn-001");
    }

    #[tokio::test]
    async fn test_begin_with_isolation_level() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager);
        let node = create_test_node_with_isolation("test-2", "带隔离级别", "conn-002", "READ COMMITTED");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 true 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["isolation_level"], "READ COMMITTED");
    }

    #[tokio::test]
    async fn test_begin_empty_connection_uuid() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager);
        let node = create_test_node("test-3", "空 UUID", "");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（空 UUID）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(!data["error"].is_null());
    }

    #[tokio::test]
    async fn test_begin_invalid_isolation_level() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager);
        let node = create_test_node_with_isolation("test-4", "无效隔离级别", "conn-004", "INVALID LEVEL");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（无效隔离级别）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().unwrap().contains("无效的隔离级别"));
    }

    #[tokio::test]
    async fn test_begin_duplicate_transaction() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager.clone());

        // 第一次开始事务
        let node1 = create_test_node("test-5a", "第一次事务", "conn-005");
        let mut context1 = create_context("test-flow");

        let result1 = executor
            .execute(&node1, &mut context1, &[], &HashMap::new())
            .await;
        assert!(result1.is_ok());
        let (next_nodes1, _) = result1.unwrap();
        assert_eq!(next_nodes1, vec!["true"]);

        // 第二次开始相同连接的事务
        let node2 = create_test_node("test-5b", "第二次事务", "conn-005");
        let mut context2 = create_context("test-flow");

        let result2 = executor
            .execute(&node2, &mut context2, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（已有活跃事务）
        assert!(result2.is_ok());
        let (next_nodes2, output_data2) = result2.unwrap();
        assert_eq!(next_nodes2, vec!["false"]);

        let data = output_data2.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().unwrap().contains("已有活跃事务"));
    }

    #[tokio::test]
    async fn test_begin_with_false_port_routing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager);

        // 创建空 UUID 节点（会失败）
        let node = create_test_node("test-6", "失败测试", "");
        let mut context = create_context("test-flow");

        // 创建 false 端口连接
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-6".to_string(),
            target_node_uuid: "target-node".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 应该返回 false 端口连接的目标节点
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["target-node"]);
    }

    #[tokio::test]
    async fn test_begin_timing_info() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager);
        let node = create_test_node("test-7", "计时测试", "conn-007");
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
        let executor = DbBeginExecutor::new(db_manager);

        // 创建一个使用错误数据类型的节点
        let node = Node {
            uuid: "test-8".to_string(),
            node_type: "db_begin".to_string(),
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
        assert!(result.unwrap_err().contains("不是 DbBeginConfig"));
    }

    #[tokio::test]
    async fn test_begin_all_isolation_levels() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbBeginExecutor::new(db_manager);

        let isolation_levels = vec![
            "READ UNCOMMITTED",
            "READ COMMITTED",
            "REPEATABLE READ",
            "SERIALIZABLE",
        ];

        for (i, level) in isolation_levels.iter().enumerate() {
            let node = create_test_node_with_isolation(
                &format!("test-iso-{}", i),
                &format!("隔离级别 {}", level),
                &format!("conn-iso-{}", i),
                level,
            );
            let mut context = create_context("test-flow");

            let result = executor
                .execute(&node, &mut context, &[], &HashMap::new())
                .await;

            assert!(result.is_ok());
            let (next_nodes, output_data) = result.unwrap();
            assert_eq!(next_nodes, vec!["true"]);

            let data = output_data.unwrap();
            assert_eq!(data["success"], true);
            assert_eq!(data["isolation_level"], *level);
        }
    }
}
