// PostgreSQL 连接池实现
//
// Wave 5: PostgreSQL 连接实现
// 使用 sqlx::postgres 进行数据库连接池管理

use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Column;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::traits::{DatabasePool, QueryResult};

/// `PostgreSQL` 连接池
///
/// 封装 `sqlx::postgres::PgPool，实现` `DatabasePool` trait。
/// 支持 `PostgreSQL` 标准连接字符串格式。
///
/// # 连接字符串格式
///
/// `postgres://user:password@host:port/database`
///
/// # 参数占位符
///
/// `PostgreSQL` 使用 `$1, $2, $3...` 格式的参数占位符
pub struct PostgresConnection {
    /// 连接池
    pool: PgPool,
    /// 连接名称（用于调试）
    name: String,
    /// 连接字符串（已清理密码，用于日志）
    connection_string_safe: String,
    /// 活跃连接数
    active_connections: Arc<RwLock<usize>>,
    /// 事务专用连接（钉扎到特定连接，确保事务内所有操作在同一连接上执行）
    tx_conn: Arc<Mutex<Option<sqlx::pool::PoolConnection<sqlx::postgres::Postgres>>>>,
}

impl PostgresConnection {
    /// 创建新的 `PostgreSQL` 连接池
    ///
    /// # 参数
    ///
    /// * `name` - 连接名称（用于调试）
    /// * `connection_string` - 连接字符串（格式: `postgres://user:password@host:port/database`）
    /// * `min_connections` - 最小连接数
    /// * `max_connections` - 最大连接数
    ///
    /// # 返回
    ///
    /// * `Ok(PostgresConnection)` - 连接池创建Success
    /// * `Err(String)` - 连接池创建Failed
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 连接到本地 PostgreSQL
    /// let conn = PostgresConnection::new(
    ///     "test",
    ///     "postgres://postgres:password@localhost:5432/mydb",
    ///     1,
    ///     10
    /// ).await?;
    /// ```
    pub async fn new(
        name: String,
        connection_string: String,
        min_connections: u32,
        max_connections: u32,
    ) -> Result<Self, String> {
        log::info!(
            "[PostgreSQL] 创建连接池: name={}, connection_string={}, min={}, max={}",
            name,
            Self::sanitize_connection_string(&connection_string),
            min_connections,
            max_connections
        );

        // 创建连接池选项
        let pool_options = PgPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections);

        // 连接数据库
        let pool = pool_options
            .connect(&connection_string)
            .await
            .map_err(|e| format!("PostgreSQL 连接池创建Failed: {}", e))?;

        log::info!("[PostgreSQL] 连接池创建Success: name={}", name);

        // 生成安全的连接字符串（隐藏密码）
        let connection_string_safe = Self::sanitize_connection_string(&connection_string);

        Ok(Self {
            pool,
            name,
            connection_string_safe,
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
    /// * `Ok(PostgresConnection)` - 连接池创建Success
    /// * `Err(String)` - 连接池创建Failed
    pub async fn from_config(config: &super::types::DbConnection) -> Result<Self, String> {
        // 构建 PostgreSQL 连接字符串
        let connection_string = Self::build_connection_string(config)?;

        Self::new(
            config.name.clone(),
            connection_string,
            config.min_connections,
            config.max_connections,
        )
        .await
    }

    /// 构建 `PostgreSQL` 连接字符串
    ///
    /// 优先使用 `connection_string` 字段，否则根据其他字段构建
    fn build_connection_string(config: &super::types::DbConnection) -> Result<String, String> {
        // 如果提供了完整的连接字符串，直接使用
        if let Some(ref conn_str) = config.connection_string {
            return Ok(conn_str.clone());
        }

        // 否则根据各个字段构建连接字符串
        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(5432);
        let database = config.database.as_deref().unwrap_or("postgres");

        // 构建连接字符串
        let connection_string = if let Some(ref username) = config.username {
            let password = config.password.as_deref().unwrap_or("");
            format!(
                "postgres://{}:{}@{}:{}/{}",
                username, password, host, port, database
            )
        } else {
            // 无用户名，尝试无认证连接
            format!("postgres://{}:{}/{}", host, port, database)
        };

        Ok(connection_string)
    }

    /// 清理连接字符串，隐藏密码信息
    ///
    /// 将 `postgres://user:password@host:port/db` 转换为 `postgres://user:***@host:port/db`
    fn sanitize_connection_string(conn_str: &str) -> String {
        // 查找 :// 和 @ 之间的部分
        if let Some(proto_end) = conn_str.find("://") {
            let after_proto = proto_end + 3;
            if let Some(at_pos) = conn_str[after_proto..].find('@') {
                let absolute_at = after_proto + at_pos;
                let credentials = &conn_str[after_proto..absolute_at];

                // 如果包含冒号，说明有密码
                if let Some(colon_pos) = credentials.find(':') {
                    let username = &credentials[..colon_pos];
                    let after_at = &conn_str[absolute_at..];
                    return format!("{}://{}:***{}", &conn_str[..proto_end], username, after_at);
                }
            }
        }
        conn_str.to_string()
    }

    /// 获取连接名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取安全连接字符串（不含密码）
    pub fn connection_string_safe(&self) -> &str {
        &self.connection_string_safe
    }
}

#[async_trait]
impl DatabasePool for PostgresConnection {
    /// 执行 SQL 查询
    ///
    /// # 参数
    ///
    /// * `sql` - SQL 查询语句（使用 `$1, $2...` 占位符）
    /// * `params` - 参数列表（JSON 格式）
    ///
    /// # 返回
    ///
    /// * `Ok(QueryResult)` - 查询Success
    /// * `Err(String)` - 查询Failed
    ///
    /// # 占位符格式
    ///
    /// `PostgreSQL` 使用 `$1, $2, $3...` 作为参数占位符：
    /// - `SELECT * FROM users WHERE id = $1`
    /// - `INSERT INTO users (name, age) VALUES ($1, $2)`
    async fn execute(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        log::debug!("[PostgreSQL] 执行查询: sql={}, params={:?}", sql, params);

        // 增加活跃连接计数
        {
            let mut active = self.active_connections.write().await;
            *active += 1;
        }

        // 事务期间路由到专用连接，否则使用连接池
        let result = {
            let mut guard = self.tx_conn.lock().await;
            if let Some(ref mut conn) = *guard {
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

    /// 测试连接是否正常
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 连接正常
    /// * `Err(String)` - connection failed
    async fn test_connection(&self) -> Result<(), String> {
        log::info!("[PostgreSQL] 测试连接: name={}", self.name);

        // 执行简单的 SELECT 1 查询
        let result = self.execute("SELECT 1 AS test", vec![]).await;

        match result {
            Ok(_) => {
                log::info!("[PostgreSQL] 连接测试Success: name={}", self.name);
                Ok(())
            }
            Err(e) => {
                log::error!(
                    "[PostgreSQL] 连接测试Failed: name={}, error={}",
                    self.name,
                    e
                );
                Err(e)
            }
        }
    }

    /// 关闭连接池
    ///
    /// 释放所有连接资源。
    async fn close(&self) {
        log::info!("[PostgreSQL] 关闭连接池: name={}", self.name);

        // 释放事务专用连接
        {
            let mut guard = self.tx_conn.lock().await;
            if guard.is_some() {
                log::warn!(
                    "[PostgreSQL] 关闭连接池时存在未提交的事务，自动回滚: name={}",
                    self.name
                );
                *guard = None;
            }
        }

        self.pool.close().await;
        log::info!("[PostgreSQL] 连接池已关闭: name={}", self.name);
    }

    /// 开始数据库事务
    ///
    /// 从连接池获取一个专用连接，执行 BEGIN 语句。
    /// 事务内的后续所有查询将通过此专用连接执行（连接钉扎）。
    ///
    /// # 参数
    ///
    /// * `isolation_level` - 可选的事务隔离级别（如 "READ COMMITTED"、"SERIALIZABLE"）
    async fn begin_db_transaction(&self, isolation_level: Option<&str>) -> Result<(), String> {
        log::info!(
            "[PostgreSQL] 开始事务: name={}, isolation_level={:?}",
            self.name,
            isolation_level
        );

        let mut guard = self.tx_conn.lock().await;

        // 检查是否已有活跃事务
        if guard.is_some() {
            return Err(format!(
                "PostgreSQL 事务已存在: name={}，请先提交或回滚当前事务",
                self.name
            ));
        }

        // 从连接池获取专用连接
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| format!("PostgreSQL 获取事务connection failed: {}", e))?;

        // 如果指定了隔离级别，先设置隔离级别
        if let Some(level) = isolation_level {
            let set_isolation_sql = format!("SET TRANSACTION ISOLATION LEVEL {}", level);
            sqlx::query(&set_isolation_sql)
                .execute(&mut *conn)
                .await
                .map_err(|e| format!("PostgreSQL 设置事务隔离级别Failed ({}): {}", level, e))?;
            log::info!(
                "[PostgreSQL] 设置事务隔离级别: name={}, level={}",
                self.name,
                level
            );
        }

        // 执行 BEGIN
        sqlx::query("BEGIN")
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("PostgreSQL BEGIN Failed: {}", e))?;

        // 存入专用连接
        *guard = Some(conn);

        log::info!("[PostgreSQL] 事务已开始: name={}", self.name);
        Ok(())
    }

    /// 提交数据库事务
    ///
    /// 向数据库发送 COMMIT 命令，然后释放专用连接归还连接池。
    async fn commit_db_transaction(&self) -> Result<(), String> {
        log::info!("[PostgreSQL] 提交事务: name={}", self.name);

        let mut guard = self.tx_conn.lock().await;

        // 取出专用连接
        let mut conn = guard
            .take()
            .ok_or_else(|| format!("PostgreSQL 无活跃事务: name={}", self.name))?;

        // 执行 COMMIT
        sqlx::query("COMMIT")
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("PostgreSQL COMMIT Failed: {}", e))?;

        // 连接 drop 后自动归还连接池
        drop(conn);
        drop(guard);

        log::info!("[PostgreSQL] 事务已提交: name={}", self.name);
        Ok(())
    }

    /// 回滚数据库事务
    ///
    /// 向数据库发送 ROLLBACK 命令，然后释放专用连接归还连接池。
    async fn rollback_db_transaction(&self) -> Result<(), String> {
        log::info!("[PostgreSQL] 回滚事务: name={}", self.name);

        let mut guard = self.tx_conn.lock().await;

        // 取出专用连接
        let mut conn = guard
            .take()
            .ok_or_else(|| format!("PostgreSQL 无活跃事务: name={}", self.name))?;

        // 执行 ROLLBACK
        sqlx::query("ROLLBACK")
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("PostgreSQL ROLLBACK Failed: {}", e))?;

        // 连接 drop 后自动归还连接池
        drop(conn);
        drop(guard);

        log::info!("[PostgreSQL] 事务已回滚: name={}", self.name);
        Ok(())
    }

    /// 获取连接池状态信息
    ///
    /// # 返回
    ///
    /// 连接池状态字符串
    fn status_info(&self) -> String {
        format!(
            "PostgreSQL pool: {} (connection={}, size={})",
            self.name,
            self.connection_string_safe,
            self.pool.size()
        )
    }
}

impl PostgresConnection {
    /// 内部执行查询方法
    async fn execute_internal(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        use sqlx::Row;

        // 判断是否是查询语句（SELECT、SHOW、EXPLAIN 等）
        let sql_upper = sql.trim().to_uppercase();
        let is_query = sql_upper.starts_with("SELECT")
            || sql_upper.starts_with("SHOW")
            || sql_upper.starts_with("EXPLAIN")
            || sql_upper.starts_with("WITH")
            || sql_upper.starts_with("RETURNING");

        if is_query || sql_upper.contains("RETURNING") {
            // 执行查询
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 执行查询
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("PostgreSQL 查询执行Failed: {}", e))?;

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
                .map_err(|e| format!("PostgreSQL 执行Failed: {}", e))?;

            Ok(QueryResult::with_affected_rows(result.rows_affected()))
        }
    }

    /// 在指定连接上执行查询（事务期间使用）
    ///
    /// 与 `execute_internal` 逻辑完全一致，但使用事务专用连接替代连接池。
    async fn execute_on_connection(
        &self,
        conn: &mut sqlx::pool::PoolConnection<sqlx::postgres::Postgres>,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        use sqlx::Row;

        // 判断是否是查询语句（SELECT、SHOW、EXPLAIN 等）
        let sql_upper = sql.trim().to_uppercase();
        let is_query = sql_upper.starts_with("SELECT")
            || sql_upper.starts_with("SHOW")
            || sql_upper.starts_with("EXPLAIN")
            || sql_upper.starts_with("WITH")
            || sql_upper.starts_with("RETURNING");

        if is_query || sql_upper.contains("RETURNING") {
            // 执行查询
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 在事务专用连接上执行查询（通过 DerefMut 获取 &mut PgConnection）
            let pg_conn = conn.deref_mut();
            let rows = query
                .fetch_all(pg_conn)
                .await
                .map_err(|e| format!("PostgreSQL 事务查询执行Failed: {}", e))?;

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

            // 在事务专用连接上执行（通过 DerefMut 获取 &mut PgConnection）
            let pg_conn = conn.deref_mut();
            let result = query
                .execute(pg_conn)
                .await
                .map_err(|e| format!("PostgreSQL 事务执行Failed: {}", e))?;

            Ok(QueryResult::with_affected_rows(result.rows_affected()))
        }
    }

    /// 绑定参数到查询
    ///
    /// `PostgreSQL` 使用 `$1, $2, $3...` 格式的占位符
    fn bind_params<'q>(
        &self,
        mut query: sqlx::query::Query<'q, sqlx::postgres::Postgres, sqlx::postgres::PgArguments>,
        params: Vec<serde_json::Value>,
    ) -> sqlx::query::Query<'q, sqlx::postgres::Postgres, sqlx::postgres::PgArguments> {
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
    fn column_to_json(row: &sqlx::postgres::PgRow, col_name: &str) -> serde_json::Value {
        use sqlx::Row;

        // 尝试获取不同类型的值
        // 注意：PostgreSQL 的类型系统更复杂，这里尝试常见类型

        // 尝试布尔值
        if let Ok(v) = row.try_get::<Option<bool>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试整数（i64）
        if let Ok(v) = row.try_get::<Option<i64>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试整数（i32）
        if let Ok(v) = row.try_get::<Option<i32>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试浮点数（f64）
        if let Ok(v) = row.try_get::<Option<f64>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试字符串
        if let Ok(v) = row.try_get::<Option<String>, _>(col_name) {
            return v
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试字节数组
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

        // 尝试时间戳（chrono::DateTime）
        if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(col_name) {
            return v
                .map(|dt| serde_json::Value::String(dt.to_rfc3339()))
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试 NaiveDateTime
        if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(col_name) {
            return v
                .map(|dt| serde_json::Value::String(dt.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试 NaiveDate
        if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(col_name) {
            return v
                .map(|d| serde_json::Value::String(d.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试 NaiveTime
        if let Ok(v) = row.try_get::<Option<chrono::NaiveTime>, _>(col_name) {
            return v
                .map(|t| serde_json::Value::String(t.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试 UUID
        if let Ok(v) = row.try_get::<Option<uuid::Uuid>, _>(col_name) {
            return v
                .map(|u| serde_json::Value::String(u.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }

        // 尝试 JSON/JSONB
        if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(col_name) {
            return v.unwrap_or(serde_json::Value::Null);
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

    #[test]
    fn test_sanitize_connection_string_with_password() {
        let conn_str = "postgres://user:secret123@localhost:5432/mydb";
        let safe = PostgresConnection::sanitize_connection_string(conn_str);
        assert!(!safe.contains("secret123"));
        assert!(safe.contains("user"));
        assert!(safe.contains("***"));
        assert!(safe.contains("localhost:5432"));
        assert!(safe.contains("mydb"));
    }

    #[test]
    fn test_sanitize_connection_string_without_password() {
        let conn_str = "postgres://localhost:5432/mydb";
        let safe = PostgresConnection::sanitize_connection_string(conn_str);
        assert_eq!(safe, conn_str);
    }

    #[test]
    fn test_sanitize_connection_string_with_user_no_password() {
        let conn_str = "postgres://user@localhost:5432/mydb";
        let safe = PostgresConnection::sanitize_connection_string(conn_str);
        assert_eq!(safe, conn_str);
    }

    #[test]
    fn test_build_connection_string_from_config() {
        use super::super::types::{DbConnection, DbConnectionStatus, DbDriver};

        let config = DbConnection {
            id: "test-001".to_string(),
            name: "Test PostgreSQL".to_string(),
            driver: DbDriver::Postgres,
            host: Some("192.168.1.100".to_string()),
            port: Some(5433),
            database: Some("testdb".to_string()),
            username: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            password_from_env: None,
            connection_string: None,
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 10000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let conn_str =
            PostgresConnection::build_connection_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(conn_str.contains("testuser"));
        assert!(conn_str.contains("testpass"));
        assert!(conn_str.contains("192.168.1.100"));
        assert!(conn_str.contains("5433"));
        assert!(conn_str.contains("testdb"));
    }

    #[test]
    fn test_build_connection_string_from_connection_string_field() {
        use super::super::types::{DbConnection, DbConnectionStatus, DbDriver};

        let config = DbConnection {
            id: "test-002".to_string(),
            name: "Test PostgreSQL 2".to_string(),
            driver: DbDriver::Postgres,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            password_from_env: None,
            connection_string: Some("postgres://custom:pass@custom.host:9999/customdb".to_string()),
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 10000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let conn_str =
            PostgresConnection::build_connection_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(conn_str, "postgres://custom:pass@custom.host:9999/customdb");
    }

    #[test]
    fn test_build_connection_string_defaults() {
        use super::super::types::{DbConnection, DbConnectionStatus, DbDriver};

        let config = DbConnection {
            id: "test-003".to_string(),
            name: "Test PostgreSQL 3".to_string(),
            driver: DbDriver::Postgres,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            password_from_env: None,
            connection_string: None,
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 10000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let conn_str =
            PostgresConnection::build_connection_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("5432"));
        assert!(conn_str.contains("postgres"));
    }

    #[test]
    fn test_build_connection_string_with_user_no_password() {
        use super::super::types::{DbConnection, DbConnectionStatus, DbDriver};

        let config = DbConnection {
            id: "test-004".to_string(),
            name: "Test PostgreSQL 4".to_string(),
            driver: DbDriver::Postgres,
            host: Some("db.example.com".to_string()),
            port: None,
            database: Some("myapp".to_string()),
            username: Some("appuser".to_string()),
            password: None,
            password_from_env: None,
            connection_string: None,
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 10000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let conn_str =
            PostgresConnection::build_connection_string(&config).unwrap_or_else(|e| panic!("{e}"));
        assert!(conn_str.contains("appuser"));
        assert!(conn_str.contains("db.example.com"));
        assert!(conn_str.contains("myapp"));
    }

    // 注意: 以下测试需要实际的 PostgreSQL 服务器运行
    // 可以通过设置环境变量 ENABLE_POSTGRES_INTEGRATION_TESTS=1 来启用

    #[cfg(feature = "postgres-integration-tests")]
    mod integration_tests {
        use super::*;
        use std::env;

        fn get_postgres_url() -> String {
            env::var("POSTGRES_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/test".to_string())
        }

        #[tokio::test]
        async fn test_postgres_connection_and_test() {
            let conn = PostgresConnection::new("test-conn".to_string(), get_postgres_url(), 1, 5)
                .await
                .unwrap();

            assert_eq!(conn.name(), "test-conn");
            assert!(conn.status_info().contains("test-conn"));

            let result = conn.test_connection().await;
            assert!(result.is_ok());

            conn.close().await;
        }

        #[tokio::test]
        async fn test_postgres_execute_select() {
            let conn = PostgresConnection::new("test-select".to_string(), get_postgres_url(), 1, 5)
                .await
                .unwrap();

            let result = conn.execute("SELECT 1 AS value", vec![]).await.unwrap();
            assert_eq!(result.rows.len(), 1);
            assert_eq!(result.row_count, 1);
            assert!(result.columns.contains(&"value".to_string()));

            conn.close().await;
        }

        #[tokio::test]
        async fn test_postgres_execute_with_params() {
            let conn = PostgresConnection::new("test-params".to_string(), get_postgres_url(), 1, 5)
                .await
                .unwrap();

            // 使用 $1 占位符
            let result = conn
                .execute(
                    "SELECT $1::text AS name, $2::integer AS age",
                    vec![serde_json::json!("Alice"), serde_json::json!(30)],
                )
                .await
                .unwrap();

            assert_eq!(result.rows.len(), 1);
            let row = &result.rows[0];
            assert_eq!(row.get("name"), Some(&serde_json::json!("Alice")));
            assert_eq!(row.get("age"), Some(&serde_json::json!(30)));

            conn.close().await;
        }

        #[tokio::test]
        async fn test_postgres_insert_update_delete() {
            let conn = PostgresConnection::new("test-crud".to_string(), get_postgres_url(), 1, 5)
                .await
                .unwrap();

            // 创建临时表
            conn.execute(
                "CREATE TEMP TABLE test_users (id SERIAL PRIMARY KEY, name TEXT, age INTEGER)",
                vec![],
            )
            .await
            .unwrap();

            // 插入
            let insert_result = conn
                .execute(
                    "INSERT INTO test_users (name, age) VALUES ($1, $2)",
                    vec![serde_json::json!("Bob"), serde_json::json!(25)],
                )
                .await
                .unwrap();
            assert_eq!(insert_result.affected_rows, 1);

            // 查询
            let select_result = conn
                .execute(
                    "SELECT * FROM test_users WHERE name = $1",
                    vec![serde_json::json!("Bob")],
                )
                .await
                .unwrap();
            assert_eq!(select_result.rows.len(), 1);

            // 更新
            let update_result = conn
                .execute(
                    "UPDATE test_users SET age = $1 WHERE name = $2",
                    vec![serde_json::json!(26), serde_json::json!("Bob")],
                )
                .await
                .unwrap();
            assert_eq!(update_result.affected_rows, 1);

            // 删除
            let delete_result = conn
                .execute(
                    "DELETE FROM test_users WHERE name = $1",
                    vec![serde_json::json!("Bob")],
                )
                .await
                .unwrap();
            assert_eq!(delete_result.affected_rows, 1);

            conn.close().await;
        }

        #[tokio::test]
        async fn test_postgres_from_config() {
            use super::super::types::{DbConnection, DbConnectionStatus, DbDriver};

            let config = DbConnection {
                id: "test-config".to_string(),
                name: "Test From Config".to_string(),
                driver: DbDriver::Postgres,
                connection_string: Some(get_postgres_url()),
                min_connections: 1,
                max_connections: 5,
                connect_timeout_ms: 10000,
                idle_timeout_ms: 300000,
                ..Default::default()
            };

            let conn = PostgresConnection::from_config(&config).await.unwrap();
            assert_eq!(conn.name(), "Test From Config");

            let result = conn.test_connection().await;
            assert!(result.is_ok());

            conn.close().await;
        }
    }
}
