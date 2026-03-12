// 数据库连接节点执行器
//
// 实现数据库连接建立、连接池创建和测试查询功能。
// Wave 2: 与执行器集成

use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::db::pool::{DatabaseConnectionManager, MockPool};
use crate::db::traits::DatabasePool;
use crate::executor::NodeExecutor;
use crate::log_event::{LogEvent, LogLevel};
use crate::node::Node;
use crate::node_config::{DbConnectConfig, NodeData};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 数据库连接节点执行器
pub struct DbConnectExecutor {
    /// 数据库连接池管理器
    db_manager: Arc<DatabaseConnectionManager>,
}

impl DbConnectExecutor {
    /// 创建新的数据库连接执行器
    pub fn new(db_manager: Arc<DatabaseConnectionManager>) -> Self {
        Self { db_manager }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for DbConnectExecutor {
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
            NodeData::DbConnect(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 DbConnectConfig: {}", node.uuid)),
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

        // 3. 日志：开始连接
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: db_connect, connection_uuid: {}",
            context.flow_uuid,
            node.uuid,
            node.name,
            config.connection_uuid
        );

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("数据库连接节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 4. 创建连接池（使用 Mock 实现）
        // Wave 2: 使用 Mock 连接池，后续 Wave 会实现真实连接
        let connect_start = Instant::now();
        let pool = Arc::new(MockPool::new(config.connection_uuid.clone()));

        // 创建连接池到管理器
        let create_result = self
            .db_manager
            .create_pool(
                config.connection_uuid.clone(),
                node.name.clone(),
                "mock".to_string(), // 数据库类型，Mock 使用 "mock"
                pool.clone(),
            )
            .await;
        let connect_duration_ms = connect_start.elapsed().as_millis() as u64;

        match create_result {
            Ok(()) => {
                log::info!(
                    "[Database] 连接池创建成功: uuid={}",
                    config.connection_uuid
                );

                // 5. 执行测试查询（如果配置了）
                let test_query_result = if let Some(ref test_query) = config.test_query {
                    if !test_query.is_empty() {
                        log::info!(
                            "[Database] 执行测试查询: uuid={}, query={}",
                            config.connection_uuid,
                            test_query
                        );

                        match pool.test_connection().await {
                            Ok(()) => {
                                log::info!(
                                    "[Database] 测试查询成功: uuid={}",
                                    config.connection_uuid
                                );
                                Ok(())
                            }
                            Err(e) => {
                                log::error!(
                                    "[Database] 测试查询失败: uuid={}, error={}",
                                    config.connection_uuid,
                                    e
                                );
                                Err(e)
                            }
                        }
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                };

                match test_query_result {
                    Ok(()) => {
                        // 6. 构建输出数据（成功）
                        let output_data = self.build_success_output(
                            &config,
                            &node.uuid,
                            connect_duration_ms,
                            start_time.elapsed().as_millis() as u64,
                        );

                        // 7. 应用变量映射
                        self.apply_output_mappings(&config, &output_data, context)?;

                        // 8. 日志：连接完成
                        let duration = start_time.elapsed();
                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Info,
                                message: format!(
                                    "数据库连接成功 ({}ms)",
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

                        // 9. 返回 true 端口
                        Ok((
                            self.get_true_port_nodes(node, connections),
                            Some(output_data),
                        ))
                    }
                    Err(e) => {
                        // 测试查询失败
                        log::error!(
                            "[Database] 测试查询失败: uuid={}, error={}",
                            config.connection_uuid,
                            e
                        );

                        // 日志：测试查询失败
                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: format!("数据库测试查询失败: {}", e),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("系统时间错误")
                                    .as_millis() as u64,
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        // 清理：关闭连接池
                        let _ = self.db_manager.close_pool(&config.connection_uuid).await;

                        // 返回 false 端口
                        Ok((
                            self.get_false_port_nodes(node, connections),
                            Some(self.build_error_output(
                                &config,
                                &format!("测试查询失败: {}", e),
                                start_time.elapsed().as_millis() as u64,
                            )),
                        ))
                    }
                }
            }
            Err(e) => {
                // 连接池创建失败
                log::error!(
                    "[Database] 连接池创建失败: uuid={}, error={}",
                    config.connection_uuid,
                    e
                );

                // 日志：连接失败
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("数据库连接池创建失败: {}", e),
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

impl DbConnectExecutor {
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
        config: &DbConnectConfig,
        connection_id: &str,
        connect_ms: u64,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一成功标识
            "success": true,
            "connected": true,
            "connection_id": connection_id,
            "connection_uuid": config.connection_uuid,
            // 请求配置
            "request": {
                "connection_uuid": config.connection_uuid,
                "test_query": config.test_query,
                "connect_timeout_ms": config.connect_timeout_ms,
            },
            // 耗时分解
            "timing": {
                "connect_ms": connect_ms,
                "total_ms": total_ms,
            },
        })
    }

    /// 构建错误输出数据
    fn build_error_output(
        &self,
        config: &DbConnectConfig,
        error: &str,
        total_ms: u64,
    ) -> serde_json::Value {
        json!({
            // 统一成功标识
            "success": false,
            "connected": false,
            "connection_id": serde_json::Value::Null,
            "connection_uuid": config.connection_uuid,
            // 请求配置
            "request": {
                "connection_uuid": config.connection_uuid,
                "test_query": config.test_query,
                "connect_timeout_ms": config.connect_timeout_ms,
            },
            // 耗时分解
            "timing": {
                "connect_ms": serde_json::Value::Null,
                "total_ms": total_ms,
            },
            "error": error,
        })
    }

    /// 应用输出变量映射
    fn apply_output_mappings(
        &self,
        _config: &DbConnectConfig,
        _output_data: &serde_json::Value,
        _context: &mut ExecutionContext,
    ) -> Result<(), String> {
        // 注意：DbConnectConfig 目前没有 output_mappings 字段
        // 如果需要，可以在 DbConnectConfig 中添加
        // 这里预留接口，暂不实现
        Ok(())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::VariableStorage;
    use crate::node::{NodePosition, NodeType};

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_uuid: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::DbConnect,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbConnect(DbConnectConfig {
                connection_uuid: connection_uuid.to_string(),
                test_query: None,
                connect_timeout_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建带测试查询的节点
    fn create_test_node_with_query(
        uuid: &str,
        name: &str,
        connection_uuid: &str,
        test_query: &str,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::DbConnect,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbConnect(DbConnectConfig {
                connection_uuid: connection_uuid.to_string(),
                test_query: Some(test_query.to_string()),
                connect_timeout_ms: None,
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
            last_node_output: None,
            debug_event_sender: None,
        }
    }

    #[tokio::test]
    async fn test_connect_success() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager.clone());
        let node = create_test_node("test-1", "测试连接", "conn-001");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 true 端口（连接成功）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connected"], true);
        assert_eq!(data["connection_uuid"], "conn-001");

        // 验证连接池已创建
        let pool_metadata = db_manager.get_pool_metadata("conn-001").await;
        assert!(pool_metadata.is_some());
    }

    #[tokio::test]
    async fn test_connect_with_test_query() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager);
        let node = create_test_node_with_query("test-2", "测试查询", "conn-002", "SELECT 1");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // MockPool 的 test_connection 会成功
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connected"], true);
    }

    #[tokio::test]
    async fn test_connect_empty_connection_uuid() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager);
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
        assert_eq!(data["connected"], false);
        assert!(!data["error"].is_null());
    }

    #[tokio::test]
    async fn test_connect_duplicate_uuid() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager.clone());

        // 第一次连接
        let node1 = create_test_node("test-4a", "第一次连接", "conn-004");
        let mut context1 = create_context("test-flow");

        let result1 = executor
            .execute(&node1, &mut context1, &[], &HashMap::new())
            .await;
        assert!(result1.is_ok());
        let (next_nodes1, _) = result1.unwrap();
        assert_eq!(next_nodes1, vec!["true"]);

        // 第二次连接相同 UUID
        let node2 = create_test_node("test-4b", "第二次连接", "conn-004");
        let mut context2 = create_context("test-flow");

        let result2 = executor
            .execute(&node2, &mut context2, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接池已存在）
        assert!(result2.is_ok());
        let (next_nodes2, output_data2) = result2.unwrap();
        assert_eq!(next_nodes2, vec!["false"]);

        let data = output_data2.unwrap();
        assert_eq!(data["success"], false);
        assert!(data["error"].as_str().unwrap().contains("已存在"));
    }

    #[tokio::test]
    async fn test_connect_with_false_port_routing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager);

        // 创建空 UUID 节点（会失败）
        let node = create_test_node("test-5", "失败测试", "");
        let mut context = create_context("test-flow");

        // 创建 false 端口连接
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "test-5".to_string(),
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
    async fn test_connect_timing_info() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager);
        let node = create_test_node("test-6", "计时测试", "conn-006");
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output_data) = result.unwrap();
        let data = output_data.unwrap();

        // 验证 timing 信息存在
        assert!(data["timing"].is_object());
        assert!(data["timing"]["connect_ms"].is_number());
        assert!(data["timing"]["total_ms"].is_number());
        assert!(data["timing"]["total_ms"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_invalid_node_data_type() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager);

        // 创建一个使用错误数据类型的节点
        let node = Node {
            uuid: "test-7".to_string(),
            node_type: NodeType::DbConnect,
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
        assert!(result.unwrap_err().contains("不是 DbConnectConfig"));
    }
}
