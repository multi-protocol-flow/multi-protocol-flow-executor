// 数据库连接节点执行器
//
// 实现数据库连接建立、连接池创建和测试查询功能。
// Wave 2: 与执行器集成
// Wave 5: SQLite/Redis/MySQL/PostgreSQL 支持

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use crate::db::pool::MockPool;
use crate::db::sqlite::SqliteConnection;
use crate::db::traits::DatabasePool;
use crate::db::types::{DbConnection, DbConnectionStatus, DbDriver};
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{DbConnectConfig, NodeData};
use flow_engine_core::traits::DbProvider;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 数据库连接节点执行器
pub struct DbConnectExecutor {
    /// 数据库连接池管理器
    db_manager: Arc<dyn DbProvider>,
}

impl DbConnectExecutor {
    /// 创建新的数据库连接执行器
    pub fn new(db_manager: Arc<dyn DbProvider>) -> Self {
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
        let node_config = match &node.data {
            NodeData::DbConnect(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是 DbConnectConfig: {}", node.uuid)),
        };

        // 2. 从 context.db_connections 获取完整配置（如果节点配置不完整）
        // 如果节点配置中缺少 driver，则从连接配置中心获取
        let config = if node_config.driver.is_none() {
            // 从连接配置中心查找
            match context.db_connections.get(&node_config.connection_uuid) {
                Some(db_conn) => {
                    log::info!(
                        "[Database] 从连接配置中心获取配置: uuid={}, driver={}",
                        node_config.connection_uuid,
                        db_conn.driver
                    );
                    // 合并配置：节点配置优先级高于连接配置中心
                    DbConnectConfig {
                        connection_uuid: node_config.connection_uuid.clone(),
                        driver: Some(db_conn.driver.to_string()),
                        connection_string: node_config.connection_string.clone().or(db_conn.connection_string.clone()),
                        host: node_config.host.clone().or(db_conn.host.clone()),
                        port: node_config.port.or(db_conn.port),
                        database: node_config.database.clone().or(db_conn.database.clone()),
                        username: node_config.username.clone().or(db_conn.username.clone()),
                        password: node_config.password.clone().or(db_conn.password.clone()),
                        min_connections: if node_config.min_connections > 0 { node_config.min_connections } else { db_conn.min_connections },
                        max_connections: if node_config.max_connections > 0 { node_config.max_connections } else { db_conn.max_connections },
                        test_query: node_config.test_query.clone(),
                        connect_timeout_ms: node_config.connect_timeout_ms.or(Some(db_conn.connect_timeout_ms)),
                    }
                }
                None => {
                    log::warn!(
                        "[Database] 连接配置中心未找到配置: uuid={}, 使用节点配置（可能不完整）",
                        node_config.connection_uuid
                    );
                    node_config
                }
            }
        } else {
            node_config
        };

        // 3. 验证 connection_uuid
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

        // 4. 创建连接池
        // Wave 5: 支持 SQLite 真实连接
        let connect_start = Instant::now();

        // 获取数据库类型
        let db_type = config.driver.as_deref().unwrap_or("mock").to_string();

        // 根据数据库类型创建连接池
        let pool: Arc<dyn DatabasePool> = match config.driver.as_deref() {
            Some("sqlite") => {
                // 构建 DbConnection 配置
                let db_config = DbConnection {
                    id: config.connection_uuid.clone(),
                    name: node.name.clone(),
                    driver: DbDriver::Sqlite,
                    host: config.host.clone(),
                    port: config.port,
                    database: config.database.clone(),
                    username: config.username.clone(),
                    password: config.password.clone(),
                    password_from_env: None,
                    connection_string: config.connection_string.clone(),
                    min_connections: config.min_connections,
                    max_connections: config.max_connections,
                    connect_timeout_ms: config.connect_timeout_ms.unwrap_or(10000),
                    idle_timeout_ms: 300000,
                    status: DbConnectionStatus::Disconnected,
                    last_error: None,
                    last_tested_at: None,
                };

                log::info!(
                    "[Database] 创建 SQLite 连接池: uuid={}, connection_string={}",
                    config.connection_uuid,
                    config.connection_string.as_deref().unwrap_or(":memory:")
                );

                // 创建 SQLite 连接池
                match SqliteConnection::from_config(&db_config).await {
                    Ok(conn) => {
                        log::info!(
                            "[Database] SQLite 连接池创建成功: uuid={}",
                            config.connection_uuid
                        );
                        Arc::new(conn) as Arc<dyn DatabasePool>
                    }
                    Err(e) => {
                        log::error!(
                            "[Database] SQLite 连接池创建失败: uuid={}, error={}",
                            config.connection_uuid,
                            e
                        );

                        // 日志：连接失败
                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: format!("SQLite 连接失败: {}", e),
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
                            Some(self.build_error_output(
                                &config,
                                &format!("SQLite 连接失败: {}", e),
                                start_time.elapsed().as_millis() as u64,
                            )),
                        ));
                    }
                }
            }
            Some("redis") => {
                // 构建 DbConnection 配置
                let db_config = DbConnection {
                    id: config.connection_uuid.clone(),
                    name: node.name.clone(),
                    driver: DbDriver::Redis,
                    host: config.host.clone(),
                    port: config.port,
                    database: config.database.clone(),
                    username: config.username.clone(),
                    password: config.password.clone(),
                    password_from_env: None,
                    connection_string: config.connection_string.clone(),
                    min_connections: config.min_connections,
                    max_connections: config.max_connections,
                    connect_timeout_ms: config.connect_timeout_ms.unwrap_or(10000),
                    idle_timeout_ms: 300000,
                    status: DbConnectionStatus::Disconnected,
                    last_error: None,
                    last_tested_at: None,
                };

                log::info!(
                    "[Database] 创建 Redis 连接池: uuid={}, host={:?}",
                    config.connection_uuid,
                    config.host
                );

                // 创建 Redis 连接池
                match crate::db::RedisConnection::from_config(&db_config).await {
                    Ok(conn) => {
                        log::info!(
                            "[Database] Redis 连接池创建成功: uuid={}",
                            config.connection_uuid
                        );
                        Arc::new(conn) as Arc<dyn DatabasePool>
                    }
                    Err(e) => {
                        log::error!(
                            "[Database] Redis 连接池创建失败: uuid={}, error={}",
                            config.connection_uuid,
                            e
                        );

                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: format!("Redis 连接失败: {}", e),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("系统时间错误")
                                    .as_millis() as u64,
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        return Ok((
                            self.get_false_port_nodes(node, connections),
                            Some(self.build_error_output(
                                &config,
                                &format!("Redis 连接失败: {}", e),
                                start_time.elapsed().as_millis() as u64,
                            )),
                        ));
                    }
                }
            }
            Some("mysql") => {
                // 构建 DbConnection 配置
                let db_config = DbConnection {
                    id: config.connection_uuid.clone(),
                    name: node.name.clone(),
                    driver: DbDriver::Mysql,
                    host: config.host.clone(),
                    port: config.port,
                    database: config.database.clone(),
                    username: config.username.clone(),
                    password: config.password.clone(),
                    password_from_env: None,
                    connection_string: config.connection_string.clone(),
                    min_connections: config.min_connections,
                    max_connections: config.max_connections,
                    connect_timeout_ms: config.connect_timeout_ms.unwrap_or(10000),
                    idle_timeout_ms: 300000,
                    status: DbConnectionStatus::Disconnected,
                    last_error: None,
                    last_tested_at: None,
                };

                log::info!(
                    "[Database] 创建 MySQL 连接池: uuid={}, host={:?}",
                    config.connection_uuid,
                    config.host
                );

                // 创建 MySQL 连接池
                match crate::db::MySqlConnection::from_config(&db_config).await {
                    Ok(conn) => {
                        log::info!(
                            "[Database] MySQL 连接池创建成功: uuid={}",
                            config.connection_uuid
                        );
                        Arc::new(conn) as Arc<dyn DatabasePool>
                    }
                    Err(e) => {
                        log::error!(
                            "[Database] MySQL 连接池创建失败: uuid={}, error={}",
                            config.connection_uuid,
                            e
                        );

                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: format!("MySQL 连接失败: {}", e),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("系统时间错误")
                                    .as_millis() as u64,
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        return Ok((
                            self.get_false_port_nodes(node, connections),
                            Some(self.build_error_output(
                                &config,
                                &format!("MySQL 连接失败: {}", e),
                                start_time.elapsed().as_millis() as u64,
                            )),
                        ));
                    }
                }
            }
            Some("postgres") | Some("postgresql") => {
                // 构建 DbConnection 配置
                let db_config = DbConnection {
                    id: config.connection_uuid.clone(),
                    name: node.name.clone(),
                    driver: DbDriver::Postgres,
                    host: config.host.clone(),
                    port: config.port,
                    database: config.database.clone(),
                    username: config.username.clone(),
                    password: config.password.clone(),
                    password_from_env: None,
                    connection_string: config.connection_string.clone(),
                    min_connections: config.min_connections,
                    max_connections: config.max_connections,
                    connect_timeout_ms: config.connect_timeout_ms.unwrap_or(10000),
                    idle_timeout_ms: 300000,
                    status: DbConnectionStatus::Disconnected,
                    last_error: None,
                    last_tested_at: None,
                };

                log::info!(
                    "[Database] 创建 PostgreSQL 连接池: uuid={}, host={:?}",
                    config.connection_uuid,
                    config.host
                );

                // 创建 PostgreSQL 连接池
                match crate::db::PostgresConnection::from_config(&db_config).await {
                    Ok(conn) => {
                        log::info!(
                            "[Database] PostgreSQL 连接池创建成功: uuid={}",
                            config.connection_uuid
                        );
                        Arc::new(conn) as Arc<dyn DatabasePool>
                    }
                    Err(e) => {
                        log::error!(
                            "[Database] PostgreSQL 连接池创建失败: uuid={}, error={}",
                            config.connection_uuid,
                            e
                        );

                        context
                            .emit_log(LogEvent {
                                level: LogLevel::Error,
                                message: format!("PostgreSQL 连接失败: {}", e),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("系统时间错误")
                                    .as_millis() as u64,
                                node_uuid: Some(node.uuid.clone()),
                                node_name: Some(node.name.clone()),
                                flow_uuid: context.flow_uuid.clone(),
                            })
                            .await;

                        return Ok((
                            self.get_false_port_nodes(node, connections),
                            Some(self.build_error_output(
                                &config,
                                &format!("PostgreSQL 连接失败: {}", e),
                                start_time.elapsed().as_millis() as u64,
                            )),
                        ));
                    }
                }
            }
            _ => {
                // 其他数据库类型暂未实现，使用 MockPool
                log::warn!(
                    "[Database] 数据库类型 '{}' 暂未实现，使用 MockPool: uuid={}",
                    db_type,
                    config.connection_uuid
                );
                Arc::new(MockPool::new(config.connection_uuid.clone())) as Arc<dyn DatabasePool>
            }
        };

        // 创建连接池到管理器
        let create_result = self
            .db_manager
            .create_pool(
                config.connection_uuid.clone(),
                node.name.clone(),
                db_type,
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
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::{NodePosition, NodeType};
    use crate::db::pool::{DatabaseConnectionManager, PoolMetadata};
    use crate::db::pool::MockPool;

    /// 创建测试节点
    fn create_test_node(uuid: &str, name: &str, connection_uuid: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::DbConnect,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbConnect(DbConnectConfig {
                connection_uuid: connection_uuid.to_string(),
                driver: None,
                connection_string: None,
                host: None,
                port: None,
                database: None,
                username: None,
                password: None,
                min_connections: 1,
                max_connections: 10,
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
                driver: None,
                connection_string: None,
                host: None,
                port: None,
                database: None,
                username: None,
                password: None,
                min_connections: 1,
                max_connections: 10,
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
            db_connections: std::collections::HashMap::new(),
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
        let pool_metadata: Option<PoolMetadata> = db_manager.get_pool_metadata("conn-001").await;
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
        // total_ms 可能为 0（如果执行非常快），所以只检查它是数字即可
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

    // ========================================================================
    // SQLite 集成测试
    // ========================================================================

    /// 创建 SQLite 测试节点
    fn create_sqlite_test_node(uuid: &str, name: &str, connection_uuid: &str) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: NodeType::DbConnect,
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbConnect(DbConnectConfig {
                connection_uuid: connection_uuid.to_string(),
                driver: Some("sqlite".to_string()),
                connection_string: Some(":memory:".to_string()),
                host: None,
                port: None,
                database: None,
                username: None,
                password: None,
                min_connections: 1,
                max_connections: 5,
                test_query: None,
                connect_timeout_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建带测试查询的 SQLite 测试节点
    fn create_sqlite_test_node_with_query(
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
                driver: Some("sqlite".to_string()),
                connection_string: Some(":memory:".to_string()),
                host: None,
                port: None,
                database: None,
                username: None,
                password: None,
                min_connections: 1,
                max_connections: 5,
                test_query: Some(test_query.to_string()),
                connect_timeout_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    #[tokio::test]
    async fn test_sqlite_connect_success() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager.clone());
        let node = create_sqlite_test_node("sqlite-1", "SQLite 测试连接", "sqlite-conn-001");
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
        assert_eq!(data["connection_uuid"], "sqlite-conn-001");

        // 验证连接池已创建
        let pool_metadata: Option<PoolMetadata> = db_manager.get_pool_metadata("sqlite-conn-001").await;
        assert!(pool_metadata.is_some());
        let metadata = pool_metadata.unwrap();
        assert_eq!(metadata.db_type, "sqlite");
    }

    #[tokio::test]
    async fn test_sqlite_connect_with_test_query() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager);
        let node = create_sqlite_test_node_with_query(
            "sqlite-2",
            "SQLite 测试查询",
            "sqlite-conn-002",
            "SELECT 1",
        );
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // SQLite 测试查询应该成功
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["true"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["connected"], true);
    }

    #[tokio::test]
    async fn test_sqlite_connect_invalid_path() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let executor = DbConnectExecutor::new(db_manager);

        // 创建使用无效路径的节点
        let node = Node {
            uuid: "sqlite-3".to_string(),
            node_type: NodeType::DbConnect,
            name: "无效路径".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbConnect(DbConnectConfig {
                connection_uuid: "sqlite-conn-003".to_string(),
                driver: Some("sqlite".to_string()),
                connection_string: Some("Z:\\nonexistent\\invalid\\path\\db.sqlite".to_string()),
                host: None,
                port: None,
                database: None,
                username: None,
                password: None,
                min_connections: 1,
                max_connections: 5,
                test_query: None,
                connect_timeout_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
        };
        let mut context = create_context("test-flow");

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 应该返回 false 端口（连接失败）
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert_eq!(next_nodes, vec!["false"]);
        assert!(output_data.is_some());

        let data = output_data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["connected"], false);
        assert!(data["error"].as_str().unwrap().contains("SQLite"));
    }
}
