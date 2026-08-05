// SQLite 连接池实现
//
// Wave 5: SQLite 连接实现
// 使用 sqlx::sqlite 进行数据库连接池管理

use async_trait::async_trait;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Column;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::traits::{DatabasePool, QueryResult};

/// `SQLite` 连接池
///
/// 封装 `sqlx::sqlite::SqlitePool，实现` `DatabasePool` trait。
/// 支持文件路径连接字符串和内存数据库 (`:memory:`)。
pub struct SqliteConnection {
    /// 连接池
    pool: SqlitePool,
    /// 连接名称（用于调试）
    name: String,
    /// 连接字符串（文件路径或 `:memory:`）
    connection_string: String,
    /// 活跃连接数
    active_connections: Arc<RwLock<usize>>,
    /// 事务专用连接（钉扎到单个连接，确保事务内所有操作在同一连接上执行）
    tx_conn: Arc<Mutex<Option<sqlx::pool::PoolConnection<sqlx::sqlite::Sqlite>>>>,
}

impl SqliteConnection {
    /// 创建新的 `SQLite` 连接池
    ///
    /// # 参数
    ///
    /// * `name` - 连接名称（用于调试）
    /// * `connection_string` - 连接字符串（文件路径或 `:memory:`）
    /// * `min_connections` - 最小连接数
    /// * `max_connections` - 最大连接数
    ///
    /// # 返回
    ///
    /// * `Ok(SqliteConnection)` - 连接池创建Success
    /// * `Err(String)` - 连接池创建Failed
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 文件数据库
    /// let conn = SqliteConnection::new("test", "test.db", 1, 10).await?;
    ///
    /// // 内存数据库
    /// let conn = SqliteConnection::new("memory", ":memory:", 1, 10).await?;
    /// ```
    pub async fn new(
        name: String,
        connection_string: String,
        min_connections: u32,
        max_connections: u32,
    ) -> Result<Self, String> {
        log::info!(
            "[SQLite] 创建连接池: name={}, connection_string={}, min={}, max={}",
            name,
            connection_string,
            min_connections,
            max_connections
        );

        // 创建连接池选项
        let pool_options = SqlitePoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections);

        // 连接数据库
        let pool = pool_options
            .connect(&connection_string)
            .await
            .map_err(|e| format!("SQLite 连接池创建Failed: {}", e))?;

        log::info!("[SQLite] 连接池创建Success: name={}", name);

        Ok(Self {
            pool,
            name,
            connection_string,
            active_connections: Arc::new(RwLock::new(0)),
            tx_conn: Arc::new(Mutex::new(None)),
        })
    }

    /// 从 `DbConnection` 配置创建连接池
    ///
    /// # 参数
    ///
    /// * `config` - 数据库连接配置
    ///
    /// # 返回
    ///
    /// * `Ok(SqliteConnection)` - 连接池创建Success
    /// * `Err(String)` - 连接池创建Failed
    pub async fn from_config(config: &super::types::DbConnection) -> Result<Self, String> {
        // 获取连接字符串
        let connection_string = config.connection_string.clone().unwrap_or_else(|| {
            // 如果没有提供连接字符串，使用默认路径
            // 对于 SQLite，database 字段可以作为文件名
            config
                .database
                .clone()
                .unwrap_or_else(|| ":memory:".to_string())
        });

        Self::new(
            config.name.clone(),
            connection_string,
            config.min_connections,
            config.max_connections,
        )
        .await
    }

    /// 获取连接字符串
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// 获取连接名称
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[async_trait]
impl DatabasePool for SqliteConnection {
    /// 执行 SQL 查询
    ///
    /// # 参数
    ///
    /// * `sql` - SQL 查询语句
    /// * `params` - 参数列表（JSON 格式）
    ///
    /// # 返回
    ///
    /// * `Ok(QueryResult)` - 查询Success
    /// * `Err(String)` - 查询Failed
    async fn execute(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        log::debug!("[SQLite] 执行查询: sql={}, params={:?}", sql, params);

        // 增加活跃连接计数
        {
            let mut active = self.active_connections.write().await;
            *active += 1;
        }

        // 事务期间路由到专用连接，否则使用连接池
        let result = {
            let mut guard = self.tx_conn.lock().await;
            if let Some(ref mut pool_conn) = *guard {
                // 通过 DerefMut 获取底层 SqliteConnection
                let conn: &mut sqlx::sqlite::SqliteConnection = pool_conn.deref_mut();
                self.execute_on_connection(conn, sql, params).await
            } else {
                drop(guard);
                self.execute_internal(sql, params).await
            }
        };

        // 减少活跃连接计数
        {
            let mut active = self.active_connections.write().await;
            *active = active.saturating_sub(1);
        }

        result
    }

    /// 开始事务
    ///
    /// 从连接池获取专用连接并执行 `BEGIN`。
    /// `SQLite` 不支持隔离级别设置，`isolation_level` 参数将被忽略。
    async fn begin_db_transaction(&self, _isolation_level: Option<&str>) -> Result<(), String> {
        let mut guard = self.tx_conn.lock().await;
        if guard.is_some() {
            return Err("SQLite 事务已在进行中，不支持嵌套事务".to_string());
        }

        log::info!("[SQLite] 开始事务: name={}", self.name);

        // 获取专用连接
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| format!("SQLite 获取事务connection failed: {}", e))?;

        // 执行 BEGIN
        sqlx::query("BEGIN")
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("SQLite BEGIN Failed: {}", e))?;

        *guard = Some(conn);
        log::info!("[SQLite] 事务已开始: name={}", self.name);
        Ok(())
    }

    /// 提交事务
    ///
    /// 执行 `COMMIT` 并释放专用连接归还连接池。
    async fn commit_db_transaction(&self) -> Result<(), String> {
        let mut guard = self.tx_conn.lock().await;
        let mut conn = guard
            .take()
            .ok_or_else(|| "SQLite 没有进行中的事务，无法提交".to_string())?;

        log::info!("[SQLite] 提交事务: name={}", self.name);

        // 执行 COMMIT
        sqlx::query("COMMIT")
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("SQLite COMMIT Failed: {}", e))?;

        // conn drop 后自动归还连接池
        drop(conn);
        drop(guard);

        log::info!("[SQLite] 事务已提交: name={}", self.name);
        Ok(())
    }

    /// 回滚事务
    ///
    /// 执行 `ROLLBACK` 并释放专用连接归还连接池。
    async fn rollback_db_transaction(&self) -> Result<(), String> {
        let mut guard = self.tx_conn.lock().await;
        let mut conn = guard
            .take()
            .ok_or_else(|| "SQLite 没有进行中的事务，无法回滚".to_string())?;

        log::info!("[SQLite] 回滚事务: name={}", self.name);

        // 执行 ROLLBACK
        sqlx::query("ROLLBACK")
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("SQLite ROLLBACK Failed: {}", e))?;

        // conn drop 后自动归还连接池
        drop(conn);
        drop(guard);

        log::info!("[SQLite] 事务已回滚: name={}", self.name);
        Ok(())
    }

    /// 测试连接是否正常
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 连接正常
    /// * `Err(String)` - connection failed
    async fn test_connection(&self) -> Result<(), String> {
        log::info!("[SQLite] 测试连接: name={}", self.name);

        // 执行简单的 SELECT 1 查询
        let result = self.execute("SELECT 1 AS test", vec![]).await;

        match result {
            Ok(_) => {
                log::info!("[SQLite] 连接测试Success: name={}", self.name);
                Ok(())
            }
            Err(e) => {
                log::error!("[SQLite] 连接测试Failed: name={}, error={}", self.name, e);
                Err(e)
            }
        }
    }

    /// 关闭连接池
    ///
    /// 释放所有连接资源。
    async fn close(&self) {
        log::info!("[SQLite] 关闭连接池: name={}", self.name);
        self.pool.close().await;
        log::info!("[SQLite] 连接池已关闭: name={}", self.name);
    }

    /// 获取连接池状态信息
    ///
    /// # 返回
    ///
    /// 连接池状态字符串
    fn status_info(&self) -> String {
        format!(
            "SQLite pool: {} (connection_string={}, size={})",
            self.name,
            self.connection_string,
            self.pool.size()
        )
    }
}

impl SqliteConnection {
    /// 内部执行查询方法
    async fn execute_internal(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        use sqlx::Row;

        // 判断是否是查询语句（SELECT）
        let is_query = sql.trim().to_uppercase().starts_with("SELECT");

        if is_query {
            // 执行查询
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 执行查询
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("SQLite 查询执行Failed: {}", e))?;

            // 处理结果
            if rows.is_empty() {
                return Ok(QueryResult::empty());
            }

            // 获取列名
            let columns: Vec<String> = rows[0]
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();

            // 转换行数据为 JSON
            let mut json_rows = Vec::new();
            for row in rows {
                let mut json_row = serde_json::Map::new();
                for col in row.columns() {
                    let col_name = col.name();
                    let value = Self::column_to_json(&row, col_name);
                    json_row.insert(col_name.to_string(), value);
                }
                json_rows.push(serde_json::Value::Object(json_row));
            }

            let row_count = json_rows.len();
            let truncated = row_count > 10000;

            let result = if truncated {
                QueryResult::with_rows(json_rows.into_iter().take(10000).collect(), columns)
                    .mark_truncated()
            } else {
                QueryResult::with_rows(json_rows, columns)
            };

            Ok(result)
        } else {
            // 执行非查询语句（INSERT/UPDATE/DELETE）
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 执行
            let result = query
                .execute(&self.pool)
                .await
                .map_err(|e| format!("SQLite 执行Failed: {}", e))?;

            Ok(QueryResult::with_affected_rows(result.rows_affected()))
        }
    }

    /// 在事务专用连接上执行查询
    ///
    /// 逻辑与 `execute_internal` 完全一致，但使用 `&mut SqliteConnection` 替代 `&self.pool`。
    async fn execute_on_connection(
        &self,
        conn: &mut sqlx::sqlite::SqliteConnection,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        use sqlx::Row;

        let is_query = sql.trim().to_uppercase().starts_with("SELECT");

        if is_query {
            let query = sqlx::query(sql);
            let query = self.bind_params(query, params);

            let rows = query
                .fetch_all(conn)
                .await
                .map_err(|e| format!("SQLite 查询执行Failed: {}", e))?;

            if rows.is_empty() {
                return Ok(QueryResult::empty());
            }

            let columns: Vec<String> = rows[0]
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();

            let mut json_rows = Vec::new();
            for row in rows {
                let mut json_row = serde_json::Map::new();
                for col in row.columns() {
                    let col_name = col.name();
                    let value = Self::column_to_json(&row, col_name);
                    json_row.insert(col_name.to_string(), value);
                }
                json_rows.push(serde_json::Value::Object(json_row));
            }

            let row_count = json_rows.len();
            let truncated = row_count > 10000;

            let result = if truncated {
                QueryResult::with_rows(json_rows.into_iter().take(10000).collect(), columns)
                    .mark_truncated()
            } else {
                QueryResult::with_rows(json_rows, columns)
            };

            Ok(result)
        } else {
            let query = sqlx::query(sql);
            let query = self.bind_params(query, params);

            let result = query
                .execute(conn)
                .await
                .map_err(|e| format!("SQLite 执行Failed: {}", e))?;

            Ok(QueryResult::with_affected_rows(result.rows_affected()))
        }
    }

    /// 绑定参数到查询
    fn bind_params<'q>(
        &self,
        mut query: sqlx::query::Query<'q, sqlx::sqlite::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
        params: Vec<serde_json::Value>,
    ) -> sqlx::query::Query<'q, sqlx::sqlite::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
        for param in params {
            query = match param {
                serde_json::Value::Null => query.bind(None::<String>),
                serde_json::Value::Bool(v) => query.bind(v),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        query.bind(f)
                    } else {
                        query.bind(n.to_string())
                    }
                }
                serde_json::Value::String(v) => query.bind(v),
                serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                    query.bind(param.to_string())
                }
            };
        }
        query
    }

    /// 将列值转换为 JSON
    fn column_to_json(row: &sqlx::sqlite::SqliteRow, col_name: &str) -> serde_json::Value {
        use sqlx::Row;

        // 尝试获取不同类型的值
        if let Ok(v) = row.try_get::<Option<i64>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<f64>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<String>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<Option<bool>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }
        // 尝试获取字节数组
        if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(col_name) {
            return v
                .map(|bytes| {
                    serde_json::Value::String(base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &bytes,
                    ))
                })
                .unwrap_or(serde_json::Value::Null);
        }

        serde_json::Value::Null
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_connection_memory() {
        // 创建内存数据库连接池
        let conn = SqliteConnection::new("test-memory".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(conn.name(), "test-memory");
        assert_eq!(conn.connection_string(), ":memory:");
        assert!(conn.status_info().contains("test-memory"));
    }

    #[tokio::test]
    async fn test_sqlite_test_connection() {
        let conn = SqliteConnection::new("test-conn".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 测试连接
        let result = conn.test_connection().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sqlite_execute_select() {
        let conn = SqliteConnection::new("test-select".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 执行 SELECT 1
        let result = conn
            .execute("SELECT 1 AS value", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.row_count, 1);
        assert_eq!(result.columns, vec!["value"]);
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn test_sqlite_execute_insert_and_select() {
        let conn = SqliteConnection::new("test-insert".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 创建表
        let create_result = conn
            .execute(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)",
                vec![],
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(create_result.affected_rows, 0);

        // 插入数据
        let insert_result = conn
            .execute(
                "INSERT INTO users (name, age) VALUES (?, ?)",
                vec![serde_json::json!("Alice"), serde_json::json!(30)],
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(insert_result.affected_rows, 1);

        // 查询数据
        let select_result = conn
            .execute("SELECT * FROM users", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(select_result.rows.len(), 1);
        assert_eq!(select_result.row_count, 1);
        assert!(select_result.columns.contains(&"id".to_string()));
        assert!(select_result.columns.contains(&"name".to_string()));
        assert!(select_result.columns.contains(&"age".to_string()));

        // 验证数据内容
        let row = &select_result.rows[0];
        assert_eq!(row.get("name"), Some(&serde_json::json!("Alice")));
        assert_eq!(row.get("age"), Some(&serde_json::json!(30)));
    }

    #[tokio::test]
    async fn test_sqlite_execute_update() {
        let conn = SqliteConnection::new("test-update".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 创建表并插入数据
        conn.execute(
            "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL)",
            vec![],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        conn.execute(
            "INSERT INTO products (name, price) VALUES (?, ?)",
            vec![serde_json::json!("Widget"), serde_json::json!(9.99)],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 更新数据
        let update_result = conn
            .execute(
                "UPDATE products SET price = ? WHERE name = ?",
                vec![serde_json::json!(19.99), serde_json::json!("Widget")],
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(update_result.affected_rows, 1);

        // 验证更新
        let select_result = conn
            .execute(
                "SELECT price FROM products WHERE name = ?",
                vec![serde_json::json!("Widget")],
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        let row = &select_result.rows[0];
        assert_eq!(row.get("price"), Some(&serde_json::json!(19.99)));
    }

    #[tokio::test]
    async fn test_sqlite_execute_delete() {
        let conn = SqliteConnection::new("test-delete".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 创建表并插入数据
        conn.execute(
            "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)",
            vec![],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        conn.execute(
            "INSERT INTO items (name) VALUES (?), (?)",
            vec![serde_json::json!("Item1"), serde_json::json!("Item2")],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 删除数据
        let delete_result = conn
            .execute(
                "DELETE FROM items WHERE name = ?",
                vec![serde_json::json!("Item1")],
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(delete_result.affected_rows, 1);

        // 验证删除
        let select_result = conn
            .execute("SELECT * FROM items", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(select_result.rows.len(), 1);
    }

    #[tokio::test]
    async fn test_sqlite_close() {
        let conn = SqliteConnection::new("test-close".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 关闭连接池
        conn.close().await;

        // 关闭后连接池大小应为 0
        assert_eq!(conn.pool.size(), 0);
    }

    #[tokio::test]
    async fn test_sqlite_status_info() {
        let conn = SqliteConnection::new("test-status".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        let status = conn.status_info();
        assert!(status.contains("test-status"));
        assert!(status.contains(":memory:"));
    }

    #[tokio::test]
    async fn test_sqlite_from_config() {
        use super::super::types::{DbConnection, DbConnectionStatus, DbDriver};

        let config = DbConnection {
            id: "test-001".to_string(),
            name: "Test SQLite".to_string(),
            driver: DbDriver::Sqlite,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            password_from_env: None,
            connection_string: Some(":memory:".to_string()),
            min_connections: 1,
            max_connections: 5,
            connect_timeout_ms: 10000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let conn = SqliteConnection::from_config(&config)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(conn.name(), "Test SQLite");

        // 测试连接
        let result = conn.test_connection().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sqlite_from_config_with_database() {
        use super::super::types::{DbConnection, DbConnectionStatus, DbDriver};

        // 使用 database 字段作为文件名
        let config = DbConnection {
            id: "test-002".to_string(),
            name: "Test SQLite 2".to_string(),
            driver: DbDriver::Sqlite,
            host: None,
            port: None,
            database: Some(":memory:".to_string()),
            username: None,
            password: None,
            password_from_env: None,
            connection_string: None,
            min_connections: 1,
            max_connections: 5,
            connect_timeout_ms: 10000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let conn = SqliteConnection::from_config(&config)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(conn.connection_string(), ":memory:");
    }

    #[tokio::test]
    async fn test_sqlite_empty_result() {
        let conn = SqliteConnection::new("test-empty".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 创建表
        conn.execute("CREATE TABLE empty_table (id INTEGER PRIMARY KEY)", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 查询空表
        let result = conn
            .execute("SELECT * FROM empty_table", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.row_count, 0);
    }

    #[tokio::test]
    async fn test_sqlite_param_types() {
        let conn = SqliteConnection::new("test-params".to_string(), ":memory:".to_string(), 1, 5)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 创建表
        conn.execute(
            "CREATE TABLE test_types (id INTEGER PRIMARY KEY, bool_val INTEGER, int_val INTEGER, float_val REAL, str_val TEXT, null_val TEXT)",
            vec![],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 插入不同类型的值
        conn.execute(
            "INSERT INTO test_types (bool_val, int_val, float_val, str_val, null_val) VALUES (?, ?, ?, ?, ?)",
            vec![
                serde_json::json!(true),
                serde_json::json!(42),
                serde_json::json!(2.5),
                serde_json::json!("hello"),
                serde_json::Value::Null,
            ],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 查询并验证
        let result = conn
            .execute("SELECT * FROM test_types", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.rows.len(), 1);

        let row = &result.rows[0];
        assert_eq!(row.get("bool_val"), Some(&serde_json::json!(1))); // SQLite 存储 bool 为 0/1
        assert_eq!(row.get("int_val"), Some(&serde_json::json!(42)));
        assert!(row.get("float_val").is_some());
        assert_eq!(row.get("str_val"), Some(&serde_json::json!("hello")));
        assert_eq!(row.get("null_val"), Some(&serde_json::Value::Null));
    }

    #[tokio::test]
    async fn test_sqlite_transaction_commit() {
        let conn =
            SqliteConnection::new("test-tx-commit".to_string(), ":memory:".to_string(), 1, 5)
                .await
                .unwrap_or_else(|e| panic!("{e}"));

        // 创建表
        conn.execute(
            "CREATE TABLE tx_test (id INTEGER PRIMARY KEY, value TEXT)",
            vec![],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 开始事务
        conn.begin_db_transaction(None)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 在事务中插入数据
        conn.execute(
            "INSERT INTO tx_test (value) VALUES (?)",
            vec![serde_json::json!("tx_data")],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 事务内查询可以看到数据
        let result = conn
            .execute("SELECT * FROM tx_test", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.rows.len(), 1);

        // 提交事务
        conn.commit_db_transaction()
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 提交后数据仍然可见
        let result = conn
            .execute("SELECT * FROM tx_test", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];
        assert_eq!(row.get("value"), Some(&serde_json::json!("tx_data")));
    }

    #[tokio::test]
    async fn test_sqlite_transaction_rollback() {
        let conn =
            SqliteConnection::new("test-tx-rollback".to_string(), ":memory:".to_string(), 1, 5)
                .await
                .unwrap_or_else(|e| panic!("{e}"));

        // 创建表
        conn.execute(
            "CREATE TABLE tx_rollback (id INTEGER PRIMARY KEY, value TEXT)",
            vec![],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 开始事务
        conn.begin_db_transaction(None)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 在事务中插入数据
        conn.execute(
            "INSERT INTO tx_rollback (value) VALUES (?)",
            vec![serde_json::json!("will_be_rolled_back")],
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 回滚事务
        conn.rollback_db_transaction()
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 回滚后数据不可见
        let result = conn
            .execute("SELECT * FROM tx_rollback", vec![])
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.rows.len(), 0);
    }

    #[tokio::test]
    async fn test_sqlite_transaction_no_double_begin() {
        let conn =
            SqliteConnection::new("test-tx-nested".to_string(), ":memory:".to_string(), 1, 5)
                .await
                .unwrap_or_else(|e| panic!("{e}"));

        // 开始事务
        conn.begin_db_transaction(None)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        // 重复开始应报错
        let result = conn.begin_db_transaction(None).await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("已在进行中"));

        // 清理：回滚
        conn.rollback_db_transaction()
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    #[tokio::test]
    async fn test_sqlite_transaction_commit_without_begin() {
        let conn =
            SqliteConnection::new("test-tx-no-begin".to_string(), ":memory:".to_string(), 1, 5)
                .await
                .unwrap_or_else(|e| panic!("{e}"));

        // 未开始事务直接提交应报错
        let result = conn.commit_db_transaction().await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("没有进行中的事务"));
    }

    #[tokio::test]
    async fn test_sqlite_transaction_rollback_without_begin() {
        let conn = SqliteConnection::new(
            "test-tx-no-begin-rollback".to_string(),
            ":memory:".to_string(),
            1,
            5,
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        // 未开始事务直接回滚应报错
        let result = conn.rollback_db_transaction().await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("没有进行中的事务"));
    }
}
