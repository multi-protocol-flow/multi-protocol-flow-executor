// 数据库查询节点执行器
//
// 执行 SQL 查询并返回结果，支持参数化查询和多种获取模式。

use crate::db::traits::DatabasePool;
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::{replace_variables, NodeExecutor};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::traits::DbProvider;
use flow_engine_core::ExecutionErrorStrategy;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// 查询计时信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct DbQueryTiming {
    /// 查询执行时间（毫秒）
    pub execute_ms: u64,
    /// 总时间（毫秒）
    pub total_ms: u64,
}

/// 数据库查询节点执行器
pub struct DbQueryNodeExecutor {
    /// 数据库连接池管理器（可选，测试时可使用 mock）
    db_manager: Option<Arc<dyn DbProvider>>,
}

impl Default for DbQueryNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl DbQueryNodeExecutor {
    /// 创建新的数据库查询节点执行器
    pub fn new() -> Self {
        Self { db_manager: None }
    }

    /// 创建带数据库连接池管理器的执行器
    pub fn with_db_manager(db_manager: Arc<dyn DbProvider>) -> Self {
        Self {
            db_manager: Some(db_manager),
        }
    }

    /// 设置数据库连接池管理器
    pub fn set_db_manager(&mut self, db_manager: Arc<dyn DbProvider>) {
        self.db_manager = Some(db_manager);
    }
}

/// 最大返回行数限制
const MAX_ROWS: usize = 10000;

#[async_trait::async_trait]
impl NodeExecutor for DbQueryNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = Instant::now();

        log::info!(
            "[Flow {}] [Start] NodeID: {}, NodeName: {}, Type: db_query",
            context.flow_uuid,
            node.uuid,
            node.name
        );

        // 发送执行开始日志到前端
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Database query node {} start execution", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 从 Node 的 data 字段提取 DbQueryConfig
        let db_config = match &node.data {
            NodeData::DbQuery(config) => config.clone(),
            _ => {
                let error_msg = format!("Node data type is not DbQueryConfig: {}", node.uuid);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                return Err(ErrorDetail::execution_error(error_msg));
            }
        };

        // 获取连接 UUID
        let connection_uuid = db_config.connection_uuid.clone();
        if connection_uuid.is_empty() {
            let error_msg = "Connection UUID is required".to_string();

            // 检查 on_error 策略
            if !node
                .on_error
                .as_ref()
                .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
            {
                return Err(ErrorDetail::execution_error(format!(
                    "Flow aborted: {}",
                    error_msg
                )));
            }

            log::error!(
                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                error_msg
            );

            // 返回Failed输出，走 false 端口
            let output_data = build_error_output(&error_msg, &DbQueryTiming::default());
            return Ok((get_false_port_nodes(node, connections), Some(output_data)));
        }

        // 获取 SQL 语句
        let sql = match &db_config.sql {
            Some(sql) => {
                // 替换 SQL 中的变量
                match replace_variables(sql, context) {
                    Ok(replaced) => replaced,
                    Err(e) => {
                        let error_msg = format!("SQL variable substitution failed: {}", e);

                        // 检查 on_error 策略
                        if !node
                            .on_error
                            .as_ref()
                            .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                        {
                            return Err(ErrorDetail::execution_error(format!(
                                "Flow aborted: {}",
                                error_msg
                            )));
                        }

                        log::error!(
                            "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                            context.flow_uuid,
                            node.uuid,
                            node.name,
                            error_msg
                        );

                        // 返回Failed输出，走 false 端口
                        let output_data = build_error_output(&error_msg, &DbQueryTiming::default());
                        return Ok((get_false_port_nodes(node, connections), Some(output_data)));
                    }
                }
            }
            None => {
                let error_msg = "SQL statement is required".to_string();

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "Flow aborted: {}",
                        error_msg
                    )));
                }

                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );

                // 返回Failed输出，走 false 端口
                let output_data = build_error_output(&error_msg, &DbQueryTiming::default());
                return Ok((get_false_port_nodes(node, connections), Some(output_data)));
            }
        };

        // 记录 SQL 日志
        log::info!("[Flow {}] [DB Query] SQL: {}", context.flow_uuid, sql);

        // 发送 SQL 日志到前端
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("Executing SQL: {}", sql),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 获取参数列表（替换变量）
        let params: Vec<serde_json::Value> = match &db_config.params {
            Some(param_strings) => {
                let mut resolved_params = Vec::new();
                for param_str in param_strings {
                    match replace_variables(param_str, context) {
                        Ok(replaced) => {
                            // 尝试解析为 JSON 值，如果Failed则作为字符串
                            let value = serde_json::from_str(&replaced)
                                .unwrap_or(serde_json::Value::String(replaced));
                            resolved_params.push(value);
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Parameter variable substitution failed: {}", e);

                            // 检查 on_error 策略
                            if !node
                                .on_error
                                .as_ref()
                                .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                            {
                                return Err(ErrorDetail::execution_error(format!(
                                    "Flow aborted: {}",
                                    error_msg
                                )));
                            }

                            log::error!(
                                "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                                context.flow_uuid,
                                node.uuid,
                                node.name,
                                error_msg
                            );

                            // 返回Failed输出，走 false 端口
                            let output_data =
                                build_error_output(&error_msg, &DbQueryTiming::default());
                            return Ok((
                                get_false_port_nodes(node, connections),
                                Some(output_data),
                            ));
                        }
                    }
                }
                resolved_params
            }
            None => Vec::new(),
        };

        // 获取数据库连接池
        let pool: Arc<dyn DatabasePool> = if let Some(db_manager) = &self.db_manager {
            match db_manager.get_pool(&connection_uuid).await {
                Some(pool) => pool,
                None => {
                    let error_msg = format!("Connection pool not found: {}", connection_uuid);

                    // 检查 on_error 策略
                    if !node
                        .on_error
                        .as_ref()
                        .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                    {
                        return Err(ErrorDetail::execution_error(format!(
                            "Flow aborted: {}",
                            error_msg
                        )));
                    }

                    log::error!(
                        "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                        context.flow_uuid,
                        node.uuid,
                        node.name,
                        error_msg
                    );

                    // 返回Failed输出，走 false 端口
                    let output_data = build_error_output(&error_msg, &DbQueryTiming::default());
                    return Ok((get_false_port_nodes(node, connections), Some(output_data)));
                }
            }
        } else {
            // 没有配置 db_manager，使用 mock 实现
            log::warn!(
                "[Flow {}] [Warn] NodeID: {}, NodeName: {}, using mock pool",
                context.flow_uuid,
                node.uuid,
                node.name
            );
            Arc::new(crate::db::MockPool::new("mock".to_string()))
        };

        // 执行查询（支持取消中断）
        let execute_start = Instant::now();
        let result = if let Some(ref rx) = context.cancel_receiver {
            // 有取消信号接收器时，使用 tokio::select! 监听取消
            let mut cancel_rx = rx.clone();
            tokio::select! {
                r = pool.execute(&sql, params) => r,
                _ = cancel_rx.changed() => {
                    log::info!(
                        "[Flow {}] [Cancel] NodeID: {}, NodeName: {}, database query cancelled",
                        context.flow_uuid,
                        node.uuid,
                        node.name
                    );
                    return Err(ErrorDetail::execution_error("Flow cancelled".to_string()));
                }
            }
        } else {
            // 无取消信号接收器时，直接执行
            pool.execute(&sql, params).await
        };
        let execute_ms = execute_start.elapsed().as_millis() as u64;

        // 处理查询结果
        let query_result = match result {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("Query execution failed: {}", e);
                log::error!(
                    "[Flow {}] [Error] NodeID: {}, NodeName: {}, Error: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );

                // 发送Error日志到前端
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: format!("Query execution failed: {}", e),
                        timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 检查 on_error 策略
                if !node
                    .on_error
                    .as_ref()
                    .is_none_or(|s| *s == ExecutionErrorStrategy::RouteToFalse)
                {
                    return Err(ErrorDetail::execution_error(format!(
                        "Flow aborted: {}",
                        error_msg
                    )));
                }

                // 返回Failed输出，走 false 端口
                let timing = DbQueryTiming {
                    execute_ms,
                    total_ms: start_time.elapsed().as_millis() as u64,
                };
                let output_data = build_error_output(&error_msg, &timing);
                return Ok((get_false_port_nodes(node, connections), Some(output_data)));
            }
        };

        // 应用 10000 行限制
        let (rows, truncated) = if query_result.rows.len() > MAX_ROWS {
            log::warn!(
                "[Flow {}] [Warn] NodeID: {}, NodeName: {}, result count {} exceeds limit {}, truncated",
                context.flow_uuid,
                node.uuid,
                node.name,
                query_result.rows.len(),
                MAX_ROWS
            );

            // 发送截断警告日志到前端
            context
                .emit_log(LogEvent {
                    level: LogLevel::Warn,
                    message: format!(
                        "Result count {} exceeds limit {}, truncated",
                        query_result.rows.len(),
                        MAX_ROWS
                    ),
                    timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            (query_result.rows[..MAX_ROWS].to_vec(), true)
        } else {
            (query_result.rows, query_result.truncated)
        };

        // 根据 fetch_mode 处理结果
        let fetch_mode = db_config.fetch_mode.as_deref().unwrap_or("all");
        let (processed_rows, row_count) = match fetch_mode {
            "one" => {
                if rows.is_empty() {
                    (vec![], 0)
                } else {
                    (vec![rows[0].clone()], 1)
                }
            }
            "scalar" => {
                if rows.is_empty() {
                    (vec![], 0)
                } else {
                    // 获取第一行的第一列
                    let first_row = &rows[0];
                    let scalar_value = if let serde_json::Value::Object(map) = first_row {
                        if let Some((_, value)) = map.iter().next() {
                            value.clone()
                        } else {
                            serde_json::Value::Null
                        }
                    } else {
                        first_row.clone()
                    };
                    (vec![scalar_value], 1)
                }
            }
            _ => {
                // "all" 或默认
                (rows.clone(), rows.len())
            }
        };

        // 计算总时间
        let total_ms = start_time.elapsed().as_millis() as u64;
        let timing = DbQueryTiming {
            execute_ms,
            total_ms,
        };

        // 判断是否为 SELECT 查询（有返回行）
        let is_select = !query_result.columns.is_empty();

        // 构建输出数据
        let output_data = if is_select {
            // SELECT 查询
            serde_json::json!({
                "success": true,
                "rows": processed_rows,
                "row_count": row_count,
                "affected_rows": query_result.affected_rows,
                "columns": query_result.columns,
                "truncated": truncated,
                "timing": timing
            })
        } else {
            // INSERT/UPDATE/DELETE 查询
            serde_json::json!({
                "success": true,
                "rows": [],
                "row_count": 0,
                "affected_rows": query_result.affected_rows,
                "last_insert_id": serde_json::Value::Null,
                "timing": timing
            })
        };

        // 记录完成日志
        log::info!(
            "[Flow {}] [Complete] NodeUUID: {}, Duration: {}ms, rows returned: {}",
            context.flow_uuid,
            node.uuid,
            total_ms,
            row_count
        );

        // 发送完成日志到前端
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!(
                    "Database query node {} completed ({}ms, {} rows)",
                    node.name, total_ms, row_count
                ),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 返回Success，走 true 端口
        let next_node_ids: Vec<String> = connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "true")
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        Ok((next_node_ids, Some(output_data)))
    }
}

/// 构建Error输出
fn build_error_output(error: &str, timing: &DbQueryTiming) -> serde_json::Value {
    serde_json::json!({
        "success": false,
        "rows": [],
        "row_count": 0,
        "affected_rows": 0,
        "columns": [],
        "truncated": false,
        "error": error,
        "timing": timing
    })
}

/// 获取 false 端口的下一个节点列表
fn get_false_port_nodes(node: &Node, connections: &[Connection]) -> Vec<String> {
    connections
        .iter()
        .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "false")
        .map(|conn| conn.target_node_uuid.clone())
        .collect()
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DatabaseConnectionManager, MockPool};
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::{DbQueryConfig, NodeData};
    use flow_engine_core::utils::extract_value_by_path;
    use serde_json::json;
    use std::sync::Arc;

    /// 创建测试用数据库查询节点
    fn create_db_query_node(
        uuid: &str,
        name: &str,
        connection_uuid: &str,
        sql: &str,
        params: Option<Vec<String>>,
        fetch_mode: Option<&str>,
    ) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "db_query".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbQuery(DbQueryConfig {
                connection_uuid: connection_uuid.to_string(),
                sql: Some(sql.to_string()),
                params,
                redis_command: None,
                redis_key: None,
                redis_value: None,
                redis_ttl: None,
                fetch_mode: fetch_mode.map(|s| s.to_string()),
                result_to: None,
                timeout_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 创建测试用执行上下文
    fn create_context(flow_id: &str, variables: serde_json::Value) -> ExecutionContext {
        let mut var_storage = VariableStorage::new();
        if let Some(obj) = variables.as_object() {
            for (key, value) in obj {
                var_storage.set_variable(key.clone(), value.clone(), String::new());
            }
        }

        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: var_storage,
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
            current_node_plugin_data: None,
        }
    }

    #[tokio::test]
    async fn test_db_query_with_mock_pool() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mock_pool = Arc::new(MockPool::new("test-pool".to_string()));

        // 创建连接池
        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                mock_pool,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "conn-001",
            "SELECT * FROM users",
            None,
            Some("all"),
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output) = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(next_nodes.is_empty()); // 没有连接
        assert!(output.is_some());

        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], true);
        assert!(output["rows"].is_array());
    }

    #[tokio::test]
    async fn test_db_query_missing_connection_uuid() {
        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "", // 空的 connection_uuid
            "SELECT * FROM users",
            None,
            Some("all"),
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::new();

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], false);
        assert!(output["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection UUID is required"));
    }

    #[tokio::test]
    async fn test_db_query_missing_sql() {
        let node = Node {
            uuid: "query-001".to_string(),
            node_type: "db_query".to_string(),
            name: "Test Query".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbQuery(DbQueryConfig {
                connection_uuid: "conn-001".to_string(),
                sql: None, // 空的 SQL
                params: None,
                redis_command: None,
                redis_key: None,
                redis_value: None,
                redis_ttl: None,
                fetch_mode: None,
                result_to: None,
                timeout_ms: None,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::new();

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], false);
        assert!(output["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("SQL"));
    }

    #[tokio::test]
    async fn test_db_query_with_params() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mock_pool = Arc::new(MockPool::new("test-pool".to_string()));

        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                mock_pool,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "conn-001",
            "SELECT * FROM users WHERE id = $1",
            Some(vec!["123".to_string()]),
            Some("all"),
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], true);
    }

    #[tokio::test]
    async fn test_db_query_fetch_mode_one() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mock_pool = Arc::new(MockPool::new("test-pool".to_string()));

        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                mock_pool,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "conn-001",
            "SELECT * FROM users LIMIT 1",
            None,
            Some("one"),
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], true);
        // MockPool 返回 1 行
        assert_eq!(output["row_count"], 1);
    }

    #[tokio::test]
    async fn test_db_query_fetch_mode_scalar() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mock_pool = Arc::new(MockPool::new("test-pool".to_string()));

        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                mock_pool,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "conn-001",
            "SELECT COUNT(*) FROM users",
            None,
            Some("scalar"),
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], true);
        assert_eq!(output["row_count"], 1);
    }

    #[tokio::test]
    async fn test_db_query_with_variable_replacement() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mock_pool = Arc::new(MockPool::new("test-pool".to_string()));

        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                mock_pool,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "conn-001",
            "SELECT * FROM users WHERE name = '{{user_name}}'",
            None,
            Some("all"),
        );

        let variables = json!({
            "user_name": "Alice"
        });

        let mut context = create_context("test-flow", variables);
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], true);
    }

    #[tokio::test]
    async fn test_db_query_connection_not_found() {
        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "non-existent-conn",
            "SELECT * FROM users",
            None,
            Some("all"),
        );

        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(output["success"], false);
        assert!(output["error"]
            .as_str()
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("Connection pool not found"));
    }

    #[tokio::test]
    async fn test_db_query_false_port_routing() {
        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "", // 空的 connection_uuid，会Failed
            "SELECT * FROM users",
            None,
            Some("all"),
        );

        // 创建连接：query-001 -> error-handler (false 端口)
        let connections = vec![Connection {
            id: "conn-001".to_string(),
            source_node_uuid: "query-001".to_string(),
            target_node_uuid: "error-handler".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        }];

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::new();

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));

        // 应该路由到 false 端口
        assert_eq!(next_nodes.len(), 1);
        assert_eq!(next_nodes[0], "error-handler");
        assert_eq!(output["success"], false);
    }

    #[tokio::test]
    async fn test_db_query_true_port_routing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mock_pool = Arc::new(MockPool::new("test-pool".to_string()));

        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                mock_pool,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "conn-001",
            "SELECT * FROM users",
            None,
            Some("all"),
        );

        // 创建连接：query-001 -> success-handler (true 端口)
        let connections = vec![Connection {
            id: "conn-001".to_string(),
            source_node_uuid: "query-001".to_string(),
            target_node_uuid: "success-handler".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        }];

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));

        // 应该路由到 true 端口
        assert_eq!(next_nodes.len(), 1);
        assert_eq!(next_nodes[0], "success-handler");
        assert_eq!(output["success"], true);
    }

    #[tokio::test]
    async fn test_db_query_timing() {
        let db_manager = Arc::new(DatabaseConnectionManager::new());
        let mock_pool = Arc::new(MockPool::new("test-pool".to_string()));

        db_manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                mock_pool,
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let node = create_db_query_node(
            "query-001",
            "Test Query",
            "conn-001",
            "SELECT * FROM users",
            None,
            Some("all"),
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = DbQueryNodeExecutor::with_db_manager(db_manager);

        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (_, output) = result.unwrap_or_else(|e| panic!("{e}"));
        let output = output.unwrap_or_else(|| panic!("expected Some"));

        // 验证 timing 字段存在
        assert!(output["timing"].is_object());
        assert!(output["timing"]["execute_ms"].is_number());
        assert!(output["timing"]["total_ms"].is_number());
    }

    #[test]
    fn test_extract_value_by_path() {
        let data = json!({
            "success": true,
            "rows": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ],
            "row_count": 2
        });

        // 测试简单路径
        let value = extract_value_by_path(&data, "$.success");
        assert_eq!(value, Some(json!(true)));

        // 测试数组索引
        let value = extract_value_by_path(&data, "$.rows[0].name");
        assert_eq!(value, Some(json!("Alice")));

        // 测试数字
        let value = extract_value_by_path(&data, "$.row_count");
        assert_eq!(value, Some(json!(2)));
    }

    #[test]
    fn test_build_error_output() {
        let timing = DbQueryTiming {
            execute_ms: 10,
            total_ms: 15,
        };

        let output = build_error_output("Test error", &timing);

        assert_eq!(output["success"], false);
        assert_eq!(output["error"], "Test error");
        assert_eq!(output["rows"], json!([]));
        assert_eq!(output["row_count"], 0);
        assert_eq!(output["timing"]["execute_ms"], 10);
        assert_eq!(output["timing"]["total_ms"], 15);
    }

    #[test]
    fn test_get_false_port_nodes() {
        let node = Node {
            uuid: "node-001".to_string(),
            node_type: "db_query".to_string(),
            name: "Test".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::DbQuery(DbQueryConfig::default()),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let connections = vec![
            Connection {
                id: "conn-001".to_string(),
                source_node_uuid: "node-001".to_string(),
                target_node_uuid: "error-handler".to_string(),
                source_port_id: "false".to_string(),
                target_port_id: "input".to_string(),
            },
            Connection {
                id: "conn-002".to_string(),
                source_node_uuid: "node-001".to_string(),
                target_node_uuid: "success-handler".to_string(),
                source_port_id: "true".to_string(),
                target_port_id: "input".to_string(),
            },
        ];

        let false_nodes = get_false_port_nodes(&node, &connections);

        assert_eq!(false_nodes.len(), 1);
        assert_eq!(false_nodes[0], "error-handler");
    }
}
