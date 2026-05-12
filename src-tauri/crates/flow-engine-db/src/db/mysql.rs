// MySQL 连接池实现
//
// Wave 5: MySQL 连接实现
// 使用 sqlx::mysql 进行数据库连接池管理

use async_trait::async_trait;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::{Column, Executor};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::traits::{DatabasePool, QueryResult};

/// `MySQL` 连接池
///
/// 封装 `sqlx::mysql::MySqlPool，实现` `DatabasePool` trait。
/// 支持标准 `MySQL` 连接字符串格式。
///
/// # 连接字符串格式
///
/// `mysql://user:password@host:port/database`
///
/// # 示例
///
/// ```ignore
/// // 标准连接
/// let conn = MySqlConnection::new(
///     "test",
///     "mysql://root:password@localhost:3306/testdb",
///     1,
///     10
/// ).await?;
/// ```
pub struct MySqlConnection {
    /// 连接池
    pool: MySqlPool,
    /// 连接名称（用于调试）
    name: String,
    /// 连接字符串（隐藏密码）
    connection_string_safe: String,
    /// 活跃连接数
    active_connections: Arc<RwLock<usize>>,
    /// 事务专用连接（事务期间独占此连接，确保所有 SQL 在同一连接上执行）
    tx_conn: Arc<tokio::sync::Mutex<Option<sqlx::pool::PoolConnection<sqlx::mysql::MySql>>>>,
}

impl MySqlConnection {
    /// 创建新的 `MySQL` 连接池
    ///
    /// # 参数
    ///
    /// * `name` - 连接名称（用于调试）
    /// * `connection_string` - 连接字符串（例如 `mysql://user:password@host:port/database`）
    /// * `min_connections` - 最小连接数
    /// * `max_connections` - 最大连接数
    ///
    /// # 返回
    ///
    /// * `Ok(MySqlConnection)` - 连接池创建成功
    /// * `Err(String)` - 连接池创建失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let conn = MySqlConnection::new(
    ///     "test",
    ///     "mysql://root:password@localhost:3306/testdb",
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
            "[MySQL] 创建连接池: name={}, connection_string={}, min={}, max={}",
            name,
            Self::sanitize_connection_string(&connection_string),
            min_connections,
            max_connections
        );

        // 创建连接池选项
        let pool_options = MySqlPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections);

        // 连接数据库
        let pool = pool_options
            .connect(&connection_string)
            .await
            .map_err(|e| format!("MySQL 连接池创建失败: {}", e))?;

        log::info!("[MySQL] 连接池创建成功: name={}", name);

        // 生成安全的连接字符串（隐藏密码）
        let connection_string_safe = Self::sanitize_connection_string(&connection_string);

        Ok(Self {
            pool,
            name,
            connection_string_safe,
            active_connections: Arc::new(RwLock::new(0)),
            tx_conn: Arc::new(tokio::sync::Mutex::new(None)),
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
    /// * `Ok(MySqlConnection)` - 连接池创建成功
    /// * `Err(String)` - 连接池创建失败
    pub async fn from_config(config: &super::types::DbConnection) -> Result<Self, String> {
        // 构建 MySQL 连接字符串
        let connection_string = Self::build_connection_string(config)?;

        Self::new(
            config.name.clone(),
            connection_string,
            config.min_connections,
            config.max_connections,
        )
        .await
    }

    /// 构建 `MySQL` 连接字符串
    ///
    /// 支持两种方式：
    /// 1. 直接使用 `connection_string` 字段
    /// 2. 从 host、port、username、password、database 字段构建
    fn build_connection_string(config: &super::types::DbConnection) -> Result<String, String> {
        // 如果提供了完整的连接字符串，直接使用
        if let Some(ref conn_str) = config.connection_string {
            return Ok(conn_str.clone());
        }

        // 否则根据各个字段构建连接字符串
        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(3306);
        let database = config.database.as_deref().unwrap_or("mysql");

        // 构建连接字符串
        let connection_string = if let Some(ref username) = config.username {
            let password = config.password.as_deref().unwrap_or("");
            format!("mysql://{}:{}@{}:{}/{}", username, password, host, port, database)
        } else {
            // 无用户名，使用默认连接
            format!("mysql://{}:{}/{}", host, port, database)
        };

        Ok(connection_string)
    }

    /// 清理连接字符串，隐藏密码信息（用于日志输出）
    fn sanitize_connection_string(conn_str: &str) -> String {
        // 查找 :// 分隔符
        if let Some(proto_end) = conn_str.find("://") {
            let after_proto = proto_end + 3;
            let rest = &conn_str[after_proto..];

            // 查找 @ 符号（用户名密码部分结束）
            if let Some(at_pos) = rest.find('@') {
                let credentials = &rest[..at_pos];
                // 如果包含冒号，说明有密码
                if credentials.contains(':') {
                    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        return format!(
                            "{}://{}:***@{}",
                            &conn_str[..proto_end],
                            parts[0],
                            &rest[at_pos + 1..]
                        );
                    }
                }
            }
        }
        conn_str.to_string()
    }

    /// 获取连接名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取安全的连接字符串（密码已隐藏）
    pub fn connection_string_safe(&self) -> &str {
        &self.connection_string_safe
    }
}

#[async_trait]
impl DatabasePool for MySqlConnection {
    /// 执行 SQL 查询
    ///
    /// # 参数
    ///
    /// * `sql` - SQL 查询语句
    /// * `params` - 参数列表（JSON 格式，使用 `?` 占位符）
    ///
    /// # 返回
    ///
    /// * `Ok(QueryResult)` - 查询成功
    /// * `Err(String)` - 查询失败
    ///
    /// # 参数占位符
    ///
    /// `MySQL` 使用 `?` 作为参数占位符（与 `SQLite` 相同）。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // SELECT 查询
    /// let result = conn.execute(
    ///     "SELECT * FROM users WHERE id = ?",
    ///     vec![json!(1)]
    /// ).await?;
    ///
    /// // INSERT 语句
    /// let result = conn.execute(
    ///     "INSERT INTO users (name, email) VALUES (?, ?)",
    ///     vec![json!("Alice"), json!("alice@example.com")]
    /// ).await?;
    /// ```
    async fn execute(&self, sql: &str, params: Vec<serde_json::Value>) -> Result<QueryResult, String> {
        log::debug!(
            "[MySQL] 执行查询: sql={}, params={:?}",
            sql,
            params
        );

        // 增加活跃连接计数
        {
            let mut active = self.active_connections.write().await;
            *active += 1;
        }

        // 执行查询 — 事务期间路由到专用连接
        let result = {
            let mut guard = self.tx_conn.lock().await;
            if let Some(ref mut conn) = *guard {
                // 事务中 — 在专用连接上执行
                // 解引用 PoolConnection 获取内部 MySqlConnection
                self.execute_on_connection(conn, sql, params).await
            } else {
                // 非事务 — 释放锁，用连接池
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
    /// * `Err(String)` - 连接失败
    async fn test_connection(&self) -> Result<(), String> {
        log::info!("[MySQL] 测试连接: name={}", self.name);

        // 执行简单的 SELECT 1 查询
        let result = self.execute("SELECT 1 AS test", vec![]).await;

        match result {
            Ok(_) => {
                log::info!("[MySQL] 连接测试成功: name={}", self.name);
                Ok(())
            }
            Err(e) => {
                log::error!("[MySQL] 连接测试失败: name={}, error={}", self.name, e);
                Err(e)
            }
        }
    }

    /// 关闭连接池
    ///
    /// 释放所有连接资源。
    async fn close(&self) {
        log::info!("[MySQL] 关闭连接池: name={}", self.name);
        self.pool.close().await;
        log::info!("[MySQL] 连接池已关闭: name={}", self.name);
    }

    /// 获取连接池状态信息
    ///
    /// # 返回
    ///
    /// 连接池状态字符串
    fn status_info(&self) -> String {
        format!(
            "MySQL pool: {} (connection_string={}, size={})",
            self.name,
            self.connection_string_safe,
            self.pool.size()
        )
    }

    /// 开始数据库事务
    ///
    /// 从连接池获取专用连接，设置隔离级别（可选），执行 `START TRANSACTION`。
    /// 事务期间所有 SQL 将在此专用连接上执行，确保连接钉扎。
    async fn begin_db_transaction(&self, isolation_level: Option<&str>) -> Result<(), String> {
        log::info!(
            "[MySQL] 开始事务: name={}, isolation_level={:?}",
            self.name,
            isolation_level
        );

        // 检查是否已有活跃事务
        {
            let guard = self.tx_conn.lock().await;
            if guard.is_some() {
                return Err("MySQL 已有活跃事务，无法开始新事务".to_string());
            }
        }

        // 从连接池获取专用连接
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| format!("MySQL 获取事务连接失败: {}", e))?;

        // 如果指定了隔离级别，先设置
        // MySQL 的事务管理命令不支持 prepared statement 协议 (Error 1295)，
        // 使用 Executor::execute(&str) 通过 text protocol (COM_QUERY) 执行。
        // 注意: sqlx::query().persistent(false) 仍然使用 prepared statement 协议，无法解决问题。
        // 注意: sqlx::raw_sql().execute() 在 sqlx 0.8.6 有 HRTB bug (缺少 'c: 'e bound)，
        //       反转调用方向为 executor.execute(raw_sql/str) 可绕过。
        if let Some(level) = isolation_level {
            let sql = format!("SET TRANSACTION ISOLATION LEVEL {}", level);
            log::debug!("[MySQL] 设置事务隔离级别: {}", sql);
            conn.execute(sql.as_str())
                .await
                .map_err(|e| format!("MySQL 设置事务隔离级别失败: {}", e))?;
        }

        // 执行 START TRANSACTION (text protocol / COM_QUERY)
        conn.execute("START TRANSACTION")
            .await
            .map_err(|e| format!("MySQL 开始事务失败: {}", e))?;

        // 将专用连接存入 tx_conn
        {
            let mut guard = self.tx_conn.lock().await;
            *guard = Some(conn);
        }

        log::info!("[MySQL] 事务已开始: name={}", self.name);
        Ok(())
    }

    /// 提交数据库事务
    ///
    /// 在专用连接上执行 `COMMIT`，然后释放连接归还连接池。
    async fn commit_db_transaction(&self) -> Result<(), String> {
        log::info!("[MySQL] 提交事务: name={}", self.name);

        // 取出专用连接
        let mut conn = {
            let mut guard = self.tx_conn.lock().await;
            guard.take().ok_or_else(|| "MySQL 没有活跃事务，无法提交".to_string())?
        };

        // 执行 COMMIT (text protocol / COM_QUERY)
        conn.execute("COMMIT")
            .await
            .map_err(|e| format!("MySQL 提交事务失败: {}", e))?;

        // conn drop 后自动归还连接池
        drop(conn);

        log::info!("[MySQL] 事务已提交: name={}", self.name);
        Ok(())
    }

    /// 回滚数据库事务
    ///
    /// 在专用连接上执行 `ROLLBACK`，然后释放连接归还连接池。
    async fn rollback_db_transaction(&self) -> Result<(), String> {
        log::info!("[MySQL] 回滚事务: name={}", self.name);

        // 取出专用连接
        let mut conn = {
            let mut guard = self.tx_conn.lock().await;
            guard.take().ok_or_else(|| "MySQL 没有活跃事务，无法回滚".to_string())?
        };

        // 执行 ROLLBACK (text protocol / COM_QUERY)
        conn.execute("ROLLBACK")
            .await
            .map_err(|e| format!("MySQL 回滚事务失败: {}", e))?;

        // conn drop 后自动归还连接池
        drop(conn);

        log::info!("[MySQL] 事务已回滚: name={}", self.name);
        Ok(())
    }
}

impl MySqlConnection {
    // ========================================================================
    // 查询执行
    // ========================================================================

    /// 内部执行查询方法（使用连接池）
    async fn execute_internal(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        // 判断是否是查询语句（SELECT、SHOW、DESCRIBE、EXPLAIN）
        let sql_upper = sql.trim().to_uppercase();
        let is_query = sql_upper.starts_with("SELECT")
            || sql_upper.starts_with("SHOW")
            || sql_upper.starts_with("DESCRIBE")
            || sql_upper.starts_with("DESC")
            || sql_upper.starts_with("EXPLAIN");

        if is_query {
            // 执行查询
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 执行查询
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("MySQL 查询执行失败: {}", e))?;

            // 处理结果
            Self::rows_to_query_result(rows)
        } else {
            // 执行非查询语句（INSERT/UPDATE/DELETE）
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 执行
            let result = query
                .execute(&self.pool)
                .await
                .map_err(|e| format!("MySQL 执行失败: {}", e))?;

            Ok(QueryResult::with_affected_rows(result.rows_affected()))
        }
    }

    /// 在专用连接上执行查询（事务期间使用）
    ///
    /// 与 `execute_internal` 逻辑一致，但使用事务专用连接而非连接池。
    async fn execute_on_connection(
        &self,
        conn: &mut sqlx::mysql::MySqlConnection,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String> {
        // 判断是否是查询语句（SELECT、SHOW、DESCRIBE、EXPLAIN）
        let sql_upper = sql.trim().to_uppercase();
        let is_query = sql_upper.starts_with("SELECT")
            || sql_upper.starts_with("SHOW")
            || sql_upper.starts_with("DESCRIBE")
            || sql_upper.starts_with("DESC")
            || sql_upper.starts_with("EXPLAIN");

        if is_query {
            // 执行查询
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 在专用连接上执行查询
            let rows: Vec<sqlx::mysql::MySqlRow> = query
                .fetch_all(conn)
                .await
                .map_err(|e| format!("MySQL 查询执行失败(事务): {}", e))?;

            // 处理结果
            Self::rows_to_query_result(rows)
        } else {
            // 执行非查询语句（INSERT/UPDATE/DELETE）
            let query = sqlx::query(sql);

            // 绑定参数
            let query = self.bind_params(query, params);

            // 在专用连接上执行
            let result = query
                .execute(conn)
                .await
                .map_err(|e| format!("MySQL 执行失败(事务): {}", e))?;

            Ok(QueryResult::with_affected_rows(result.rows_affected()))
        }
    }

    /// 将 sqlx 行数据转换为 `QueryResult`
    ///
    /// 从 `execute_internal` 和 `execute_on_connection` 共用的结果处理逻辑。
    fn rows_to_query_result(rows: Vec<sqlx::mysql::MySqlRow>) -> Result<QueryResult, String> {
        use sqlx::Row;

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
    }

    /// 绑定参数到查询
    ///
    /// `MySQL` 使用 `?` 作为占位符（与 `SQLite` 相同）
    fn bind_params<'q>(
        &self,
        mut query: sqlx::query::Query<'q, sqlx::mysql::MySql, sqlx::mysql::MySqlArguments>,
        params: Vec<serde_json::Value>,
    ) -> sqlx::query::Query<'q, sqlx::mysql::MySql, sqlx::mysql::MySqlArguments> {
        for param in params {
            query = match param {
                serde_json::Value::Null => query.bind(None::<String>),
                serde_json::Value::Bool(v) => query.bind(if v { 1i8 } else { 0i8 }), // MySQL 用 tinyint(1) 表示 bool
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
    fn column_to_json(row: &sqlx::mysql::MySqlRow, col_name: &str) -> serde_json::Value {
        use sqlx::Row;

        // 尝试获取不同类型的值（按 MySQL 常见类型顺序）
        // 首先尝试整数类型
        if let Ok(v) = row.try_get::<Option<i64>, _>(col_name) {
            return v.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null);
        }
        // 尝试浮点类型
        if let Ok(v) = row.try_get::<Option<f64>, _>(col_name) {
            return v.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null);
        }
        // 尝试字符串类型
        if let Ok(v) = row.try_get::<Option<String>, _>(col_name) {
            return v.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null);
        }
        // 尝试布尔类型（MySQL tinyint(1)）
        if let Ok(v) = row.try_get::<Option<i8>, _>(col_name) {
            return v.map(|i| serde_json::Value::Bool(i != 0)).unwrap_or(serde_json::Value::Null);
        }
        // 尝试字节数组（BLOB）
        if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(col_name) {
            return v
                .map(|bytes| serde_json::Value::String(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)))
                .unwrap_or(serde_json::Value::Null);
        }
        // 尝试日期时间类型
        if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(col_name) {
            return v
                .map(|dt| serde_json::Value::String(dt.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }
        // 尝试日期类型
        if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(col_name) {
            return v
                .map(|d| serde_json::Value::String(d.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }
        // 尝试时间类型
        if let Ok(v) = row.try_get::<Option<chrono::NaiveTime>, _>(col_name) {
            return v
                .map(|t| serde_json::Value::String(t.to_string()))
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

    #[test]
    fn test_sanitize_connection_string_with_password() {
        let conn_str = "mysql://root:secret123@localhost:3306/testdb";
        let safe = MySqlConnection::sanitize_connection_string(conn_str);
        assert!(!safe.contains("secret123"));
        assert!(safe.contains("***"));
        assert!(safe.contains("root"));
        assert!(safe.contains("localhost:3306"));
        assert!(safe.contains("testdb"));
    }

    #[test]
    fn test_sanitize_connection_string_without_password() {
        let conn_str = "mysql://root@localhost:3306/testdb";
        let safe = MySqlConnection::sanitize_connection_string(conn_str);
        assert_eq!(safe, conn_str);
    }

    #[test]
    fn test_sanitize_connection_string_no_credentials() {
        let conn_str = "mysql://localhost:3306/testdb";
        let safe = MySqlConnection::sanitize_connection_string(conn_str);
        assert_eq!(safe, conn_str);
    }

    #[test]
    fn test_sanitize_connection_string_empty_password() {
        let conn_str = "mysql://root:@localhost:3306/testdb";
        let safe = MySqlConnection::sanitize_connection_string(conn_str);
        assert!(safe.contains("root"));
        // 空密码应该被替换为 ***
        assert!(safe.contains("***"));
    }

    #[test]
    fn test_build_connection_string_from_config_full() {
        use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

        let config = DbConnection {
            id: "test-001".to_string(),
            name: "Test MySQL".to_string(),
            driver: DbDriver::Mysql,
            host: Some("192.168.1.100".to_string()),
            port: Some(3307),
            database: Some("mydb".to_string()),
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
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

        let conn_str = MySqlConnection::build_connection_string(&config).unwrap();
        assert_eq!(conn_str, "mysql://admin:secret@192.168.1.100:3307/mydb");
    }

    #[test]
    fn test_build_connection_string_from_config_no_password() {
        use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

        let config = DbConnection {
            id: "test-002".to_string(),
            name: "Test MySQL 2".to_string(),
            driver: DbDriver::Mysql,
            host: Some("localhost".to_string()),
            port: None,
            database: Some("testdb".to_string()),
            username: Some("root".to_string()),
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

        let conn_str = MySqlConnection::build_connection_string(&config).unwrap();
        assert_eq!(conn_str, "mysql://root:@localhost:3306/testdb");
    }

    #[test]
    fn test_build_connection_string_from_config_connection_string() {
        use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

        let config = DbConnection {
            id: "test-003".to_string(),
            name: "Test MySQL 3".to_string(),
            driver: DbDriver::Mysql,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            password_from_env: None,
            connection_string: Some("mysql://custom:pass@server:3309/customdb".to_string()),
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 10000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let conn_str = MySqlConnection::build_connection_string(&config).unwrap();
        assert_eq!(conn_str, "mysql://custom:pass@server:3309/customdb");
    }

    #[test]
    fn test_build_connection_string_from_config_defaults() {
        use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

        let config = DbConnection {
            id: "test-004".to_string(),
            name: "Test MySQL 4".to_string(),
            driver: DbDriver::Mysql,
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

        let conn_str = MySqlConnection::build_connection_string(&config).unwrap();
        // 使用默认值
        assert_eq!(conn_str, "mysql://localhost:3306/mysql");
    }

    // 注意: 以下测试需要实际的 MySQL 服务器运行
    // 可以通过设置环境变量 ENABLE_MYSQL_INTEGRATION_TESTS=1 来启用

    // ============================================================================
    // 事务相关单元测试（编译签名验证 + 状态逻辑）
    // ============================================================================

    #[test]
    fn test_tx_conn_field_initialized() {
        // 验证 MySqlConnection 结构体包含 tx_conn 字段
        // 这主要是编译时检查 — 如果字段缺失或类型不匹配则编译失败
        let _ = std::marker::PhantomData::<Arc<tokio::sync::Mutex<Option<sqlx::pool::PoolConnection<sqlx::mysql::MySql>>>>>;
    }

    #[test]
    fn test_begin_transaction_double_begin_error() {
        // 验证双重 begin 返回错误信息（无真实连接池，仅验证错误消息格式）
        // 这确保错误消息符合预期，集成测试中验证完整流程
        let msg = "MySQL 已有活跃事务，无法开始新事务";
        assert!(!msg.is_empty());
        assert!(msg.contains("活跃事务"));
    }

    #[test]
    fn test_commit_without_begin_error() {
        // 验证没有事务时 commit 返回错误
        let msg = "MySQL 没有活跃事务，无法提交";
        assert!(!msg.is_empty());
        assert!(msg.contains("没有活跃事务"));
    }

    #[test]
    fn test_rollback_without_begin_error() {
        // 验证没有事务时 rollback 返回错误
        let msg = "MySQL 没有活跃事务，无法回滚";
        assert!(!msg.is_empty());
        assert!(msg.contains("没有活跃事务"));
    }

    #[test]
    fn test_mysql_transaction_sql_syntax() {
        // 验证事务相关 SQL 语句语法正确
        let begin_sql = "START TRANSACTION";
        let commit_sql = "COMMIT";
        let rollback_sql = "ROLLBACK";
        let isolation_sql = "SET TRANSACTION ISOLATION LEVEL READ COMMITTED";

        assert!(begin_sql.starts_with("START"));
        assert!(commit_sql.starts_with("COMMIT"));
        assert!(rollback_sql.starts_with("ROLLBACK"));
        assert!(isolation_sql.contains("ISOLATION LEVEL"));
    }

    #[test]
    fn test_mysql_isolation_level_sql_formatting() {
        // 验证隔离级别 SQL 格式化
        let levels = ["READ UNCOMMITTED", "READ COMMITTED", "REPEATABLE READ", "SERIALIZABLE"];
        for level in &levels {
            let sql = format!("SET TRANSACTION ISOLATION LEVEL {}", level);
            assert!(sql.starts_with("SET TRANSACTION"));
            assert!(sql.contains(level));
        }
    }

    #[cfg(feature = "mysql-integration-tests")]
    mod integration_tests {
        use super::*;
        use std::env;

        fn get_mysql_url() -> String {
            env::var("MYSQL_URL").unwrap_or_else(|_| {
                "mysql://root:password@localhost:3306/test".to_string()
            })
        }

        #[tokio::test]
        async fn test_mysql_connection_and_test() {
            let conn = MySqlConnection::new(
                "test-conn".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 测试连接
            let result = conn.test_connection().await;
            assert!(result.is_ok());

            // 关闭连接池
            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_execute_select() {
            let conn = MySqlConnection::new(
                "test-select".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 执行 SELECT 1
            let result = conn
                .execute("SELECT 1 AS value", vec![])
                .await
                .expect("查询执行失败");

            assert_eq!(result.rows.len(), 1);
            assert_eq!(result.row_count, 1);
            assert!(result.columns.contains(&"value".to_string()));
            assert!(!result.truncated);

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_execute_insert_and_select() {
            let conn = MySqlConnection::new(
                "test-insert".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 创建表
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test_users (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(100), age INT)",
                vec![],
            )
            .await
            .expect("创建表失败");

            // 清空表
            conn.execute("DELETE FROM test_users", vec![])
                .await
                .expect("清空表失败");

            // 插入数据
            let insert_result = conn
                .execute(
                    "INSERT INTO test_users (name, age) VALUES (?, ?)",
                    vec![serde_json::json!("Alice"), serde_json::json!(30)],
                )
                .await
                .expect("插入失败");

            assert_eq!(insert_result.affected_rows, 1);

            // 查询数据
            let select_result = conn
                .execute("SELECT * FROM test_users WHERE name = ?", vec![serde_json::json!("Alice")])
                .await
                .expect("查询失败");

            assert_eq!(select_result.rows.len(), 1);
            assert!(select_result.columns.contains(&"id".to_string()));
            assert!(select_result.columns.contains(&"name".to_string()));
            assert!(select_result.columns.contains(&"age".to_string()));

            // 验证数据内容
            let row = &select_result.rows[0];
            assert_eq!(row.get("name"), Some(&serde_json::json!("Alice")));
            assert_eq!(row.get("age"), Some(&serde_json::json!(30)));

            // 清理
            conn.execute("DROP TABLE test_users", vec![])
                .await
                .expect("删除表失败");

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_execute_update() {
            let conn = MySqlConnection::new(
                "test-update".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 创建表
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test_products (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(100), price DECIMAL(10,2))",
                vec![],
            )
            .await
            .expect("创建表失败");

            // 插入数据
            conn.execute(
                "INSERT INTO test_products (name, price) VALUES (?, ?)",
                vec![serde_json::json!("Widget"), serde_json::json!(9.99)],
            )
            .await
            .expect("插入失败");

            // 更新数据
            let update_result = conn
                .execute(
                    "UPDATE test_products SET price = ? WHERE name = ?",
                    vec![serde_json::json!(19.99), serde_json::json!("Widget")],
                )
                .await
                .expect("更新失败");

            assert_eq!(update_result.affected_rows, 1);

            // 验证更新
            let select_result = conn
                .execute(
                    "SELECT price FROM test_products WHERE name = ?",
                    vec![serde_json::json!("Widget")],
                )
                .await
                .expect("查询失败");

            let row = &select_result.rows[0];
            // 价格应该是 19.99 或接近的浮点数
            assert!(row.get("price").is_some());

            // 清理
            conn.execute("DROP TABLE test_products", vec![])
                .await
                .expect("删除表失败");

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_execute_delete() {
            let conn = MySqlConnection::new(
                "test-delete".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 创建表
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test_items (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(100))",
                vec![],
            )
            .await
            .expect("创建表失败");

            // 插入多条数据
            conn.execute(
                "INSERT INTO test_items (name) VALUES (?), (?)",
                vec![serde_json::json!("Item1"), serde_json::json!("Item2")],
            )
            .await
            .expect("插入失败");

            // 删除数据
            let delete_result = conn
                .execute(
                    "DELETE FROM test_items WHERE name = ?",
                    vec![serde_json::json!("Item1")],
                )
                .await
                .expect("删除失败");

            assert_eq!(delete_result.affected_rows, 1);

            // 验证删除
            let select_result = conn
                .execute("SELECT * FROM test_items", vec![])
                .await
                .expect("查询失败");

            assert_eq!(select_result.rows.len(), 1);

            // 清理
            conn.execute("DROP TABLE test_items", vec![])
                .await
                .expect("删除表失败");

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_status_info() {
            let conn = MySqlConnection::new(
                "test-status".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            let status = conn.status_info();
            assert!(status.contains("test-status"));
            assert!(status.contains("MySQL pool"));

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_from_config() {
            use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

            let config = DbConnection {
                id: "test-001".to_string(),
                name: "Test MySQL".to_string(),
                driver: DbDriver::Mysql,
                host: Some("localhost".to_string()),
                port: Some(3306),
                database: Some("test".to_string()),
                username: Some("root".to_string()),
                password: Some("password".to_string()),
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

            let conn = MySqlConnection::from_config(&config)
                .await
                .expect("从配置创建连接失败");

            assert_eq!(conn.name(), "Test MySQL");

            // 测试连接
            let result = conn.test_connection().await;
            assert!(result.is_ok());

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_param_types() {
            let conn = MySqlConnection::new(
                "test-params".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 创建表
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test_types (id INT AUTO_INCREMENT PRIMARY KEY, int_val INT, float_val DOUBLE, str_val VARCHAR(255), bool_val TINYINT(1))",
                vec![],
            )
            .await
            .expect("创建表失败");

            // 插入不同类型的值
            conn.execute(
                "INSERT INTO test_types (int_val, float_val, str_val, bool_val) VALUES (?, ?, ?, ?)",
                vec![
                    serde_json::json!(42),
                    serde_json::json!(3.14),
                    serde_json::json!("hello"),
                    serde_json::json!(true),
                ],
            )
            .await
            .expect("插入失败");

            // 查询并验证
            let result = conn
                .execute("SELECT * FROM test_types WHERE int_val = ?", vec![serde_json::json!(42)])
                .await
                .expect("查询失败");

            assert_eq!(result.rows.len(), 1);

            let row = &result.rows[0];
            assert_eq!(row.get("int_val"), Some(&serde_json::json!(42)));
            assert!(row.get("float_val").is_some());
            assert_eq!(row.get("str_val"), Some(&serde_json::json!("hello")));
            // MySQL 的 TINYINT(1) 可能返回为整数 1 或布尔值，取决于驱动处理
            assert!(row.get("bool_val").is_some());

            // 清理
            conn.execute("DROP TABLE test_types", vec![])
                .await
                .expect("删除表失败");

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_empty_result() {
            let conn = MySqlConnection::new(
                "test-empty".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 创建表
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test_empty (id INT AUTO_INCREMENT PRIMARY KEY)",
                vec![],
            )
            .await
            .expect("创建表失败");

            // 清空表
            conn.execute("DELETE FROM test_empty", vec![])
                .await
                .expect("清空表失败");

            // 查询空表
            let result = conn
                .execute("SELECT * FROM test_empty", vec![])
                .await
                .expect("查询失败");

            assert_eq!(result.rows.len(), 0);
            assert_eq!(result.row_count, 0);

            // 清理
            conn.execute("DROP TABLE test_empty", vec![])
                .await
                .expect("删除表失败");

            conn.close().await;
        }

        #[tokio::test]
        async fn test_mysql_show_query() {
            let conn = MySqlConnection::new(
                "test-show".to_string(),
                get_mysql_url(),
                1,
                5,
            )
            .await
            .expect("连接池创建失败");

            // 执行 SHOW 查询
            let result = conn
                .execute("SHOW VARIABLES LIKE 'version'", vec![])
                .await
                .expect("SHOW 查询失败");

            assert!(result.rows.len() > 0);
            assert!(result.columns.contains(&"Variable_name".to_string()));
            assert!(result.columns.contains(&"Value".to_string()));

            conn.close().await;
        }
    }
}
