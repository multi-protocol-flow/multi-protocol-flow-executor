// Redis 连接池实现
//
// Wave 5: Redis 连接实现
// 使用 redis::Client + ConnectionManager 进行连接池管理

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::traits::{DatabasePool, QueryResult};

/// Redis 连接池
///
/// 封装 redis::aio::ConnectionManager，实现 DatabasePool trait。
/// 使用 redis:// URL 格式连接，支持 PING 命令测试连接。
pub struct RedisConnection {
    /// 连接管理器
    manager: Arc<RwLock<ConnectionManager>>,
    /// 连接名称（用于调试）
    name: String,
    /// 连接 URL（不含密码）
    connection_url_safe: String,
    /// 活跃操作数
    active_operations: Arc<RwLock<usize>>,
}

impl RedisConnection {
    /// 创建新的 Redis 连接池
    ///
    /// # 参数
    ///
    /// * `name` - 连接名称（用于调试）
    /// * `url` - Redis 连接 URL（例如 `redis://127.0.0.1:6379`）
    /// * `response_timeout_ms` - 响应超时时间（毫秒）
    /// * `connection_timeout_ms` - 连接超时时间（毫秒）
    ///
    /// # 返回
    ///
    /// * `Ok(RedisConnection)` - 连接池创建成功
    /// * `Err(String)` - 连接池创建失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let conn = RedisConnection::new(
    ///     "test",
    ///     "redis://127.0.0.1:6379",
    ///     5000,
    ///     10000
    /// ).await?;
    /// ```
    pub async fn new(
        name: String,
        url: String,
        response_timeout_ms: u64,
        connection_timeout_ms: u64,
    ) -> Result<Self, String> {
        log::info!(
            "[Redis] 创建连接池: name={}, url={}, response_timeout={}ms, connection_timeout={}ms",
            name,
            Self::sanitize_url(&url),
            response_timeout_ms,
            connection_timeout_ms
        );

        // 创建 Redis 客户端
        let client = redis::Client::open(url.as_str())
            .map_err(|e| format!("Redis 客户端创建失败: {}", e))?;

        // 配置 ConnectionManager
        let config = redis::aio::ConnectionManagerConfig::new()
            .set_response_timeout(Duration::from_millis(response_timeout_ms))
            .set_connection_timeout(Duration::from_millis(connection_timeout_ms))
            .set_number_of_retries(3)
            .set_max_delay(5000) // 毫秒
            .set_exponent_base(2);

        // 创建连接管理器
        let manager = ConnectionManager::new_with_config(client, config)
            .await
            .map_err(|e| format!("Redis 连接管理器创建失败: {}", e))?;

        log::info!("[Redis] 连接池创建成功: name={}", name);

        // 生成安全的 URL（隐藏密码）
        let connection_url_safe = Self::sanitize_url(&url);

        Ok(Self {
            manager: Arc::new(RwLock::new(manager)),
            name,
            connection_url_safe,
            active_operations: Arc::new(RwLock::new(0)),
        })
    }

    /// 从 DbConnection 配置创建连接池
    ///
    /// # 参数
    ///
    /// * `config` - 数据库连接配置
    ///
    /// # 返回
    ///
    /// * `Ok(RedisConnection)` - 连接池创建成功
    /// * `Err(String)` - 连接池创建失败
    pub async fn from_config(config: &super::types::DbConnection) -> Result<Self, String> {
        // 构建 Redis URL
        let url = Self::build_redis_url(config)?;

        Self::new(
            config.name.clone(),
            url,
            config.connect_timeout_ms,
            config.connect_timeout_ms,
        )
        .await
    }

    /// 构建 Redis 连接 URL
    fn build_redis_url(config: &super::types::DbConnection) -> Result<String, String> {
        // 如果提供了完整的连接字符串，直接使用
        if let Some(ref conn_str) = config.connection_string {
            return Ok(conn_str.clone());
        }

        // 否则根据各个字段构建 URL
        let host = config.host.as_deref().unwrap_or("127.0.0.1");
        let port = config.port.unwrap_or(6379);
        let database = config.database.as_deref().unwrap_or("0");

        // 构建基础 URL
        let url = if let Some(ref username) = config.username {
            let password = config.password.as_deref().unwrap_or("");
            format!("redis://{}:{}@{}:{}/{}", username, password, host, port, database)
        } else if let Some(ref password) = config.password {
            format!("redis://:{}@{}:{}/{}", password, host, port, database)
        } else {
            format!("redis://{}:{}/{}", host, port, database)
        };

        Ok(url)
    }

    /// 清理 URL，隐藏密码信息
    fn sanitize_url(url: &str) -> String {
        // 简单地替换密码部分
        if let Some(at_pos) = url.find('@') {
            if let Some(proto_end) = url.find("://") {
                let after_proto = proto_end + 3;
                if after_proto < at_pos {
                    let before = &url[after_proto..at_pos];
                    if before.contains(':') {
                        // 有密码，隐藏它
                        return format!(
                            "{}://***:***@{}",
                            &url[..proto_end],
                            &url[at_pos + 1..]
                        );
                    }
                }
            }
        }
        url.to_string()
    }

    /// 获取连接名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取安全连接 URL
    pub fn connection_url_safe(&self) -> &str {
        &self.connection_url_safe
    }

    /// 解析 Redis 命令
    ///
    /// 命令格式支持两种方式：
    /// 1. JSON 数组格式：`["SET", "key", "value"]`
    /// 2. 命令字符串格式：`SET key value`
    fn parse_command(sql: &str) -> Result<(String, Vec<String>), String> {
        let trimmed = sql.trim();

        // 尝试解析为 JSON 数组
        if trimmed.starts_with('[') {
            let parts: Vec<serde_json::Value> = serde_json::from_str(trimmed)
                .map_err(|e| format!("Redis 命令 JSON 解析失败: {}", e))?;

            if parts.is_empty() {
                return Err("Redis 命令不能为空".to_string());
            }

            let command = parts[0]
                .as_str()
                .ok_or("Redis 命令必须是字符串")?
                .to_uppercase();

            let args: Vec<String> = parts[1..]
                .iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => v.to_string(),
                })
                .collect();

            return Ok((command, args));
        }

        // 否则解析为空格分隔的命令
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Redis 命令不能为空".to_string());
        }

        let command = parts[0].to_uppercase();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok((command, args))
    }

    /// 执行 Redis 命令并返回结果
    async fn execute_command(
        &self,
        command: &str,
        args: Vec<String>,
    ) -> Result<QueryResult, String> {
        let mut manager = self.manager.write().await;

        match command {
            "GET" => {
                if args.is_empty() {
                    return Err("GET 命令需要指定 key".to_string());
                }
                let key = &args[0];
                let result: Option<String> = manager
                    .get(key)
                    .await
                    .map_err(|e| format!("Redis GET 命令失败: {}", e))?;

                match result {
                    Some(value) => Ok(QueryResult::with_rows(
                        vec![serde_json::json!({"key": key, "value": value})],
                        vec!["key".to_string(), "value".to_string()],
                    )),
                    None => Ok(QueryResult::empty()),
                }
            }
            "SET" => {
                if args.len() < 2 {
                    return Err("SET 命令需要指定 key 和 value".to_string());
                }
                let key = &args[0];
                let value = &args[1];

                // 处理可选的 EX/PX/TTL 参数
                let mut cmd = redis::cmd("SET");
                cmd.arg(key).arg(value);

                // 检查是否有 EX 或 PX 参数
                let mut i = 2;
                while i + 1 < args.len() {
                    match args[i].to_uppercase().as_str() {
                        "EX" => {
                            if let Ok(seconds) = args[i + 1].parse::<i64>() {
                                cmd.arg("EX").arg(seconds);
                            }
                            i += 2;
                        }
                        "PX" => {
                            if let Ok(ms) = args[i + 1].parse::<i64>() {
                                cmd.arg("PX").arg(ms);
                            }
                            i += 2;
                        }
                        "NX" => {
                            cmd.arg("NX");
                            i += 1;
                        }
                        "XX" => {
                            cmd.arg("XX");
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }

                let result: String = cmd
                    .query_async(&mut *manager)
                    .await
                    .map_err(|e| format!("Redis SET 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "result": result})],
                    vec!["key".to_string(), "result".to_string()],
                ))
            }
            "DEL" => {
                if args.is_empty() {
                    return Err("DEL 命令需要指定 key".to_string());
                }
                let count: i64 = manager
                    .del(&args)
                    .await
                    .map_err(|e| format!("Redis DEL 命令失败: {}", e))?;

                Ok(QueryResult::with_affected_rows(count as u64))
            }
            "HGET" => {
                if args.len() < 2 {
                    return Err("HGET 命令需要指定 key 和 field".to_string());
                }
                let key = &args[0];
                let field = &args[1];
                let result: Option<String> = manager
                    .hget(key, field)
                    .await
                    .map_err(|e| format!("Redis HGET 命令失败: {}", e))?;

                match result {
                    Some(value) => Ok(QueryResult::with_rows(
                        vec![serde_json::json!({"key": key, "field": field, "value": value})],
                        vec!["key".to_string(), "field".to_string(), "value".to_string()],
                    )),
                    None => Ok(QueryResult::empty()),
                }
            }
            "HSET" => {
                if args.len() < 3 {
                    return Err("HSET 命令需要指定 key、field 和 value".to_string());
                }
                let key = &args[0];
                let field = &args[1];
                let value = &args[2];

                let result: i64 = manager
                    .hset(key, field, value)
                    .await
                    .map_err(|e| format!("Redis HSET 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "field": field, "added": result})],
                    vec!["key".to_string(), "field".to_string(), "added".to_string()],
                ))
            }
            "HDEL" => {
                if args.len() < 2 {
                    return Err("HDEL 命令需要指定 key 和 field".to_string());
                }
                let key = &args[0];
                let fields = &args[1..];

                let count: i64 = manager
                    .hdel(key, fields)
                    .await
                    .map_err(|e| format!("Redis HDEL 命令失败: {}", e))?;

                Ok(QueryResult::with_affected_rows(count as u64))
            }
            "HGETALL" => {
                if args.is_empty() {
                    return Err("HGETALL 命令需要指定 key".to_string());
                }
                let key = &args[0];
                let result: std::collections::HashMap<String, String> = manager
                    .hgetall(key)
                    .await
                    .map_err(|e| format!("Redis HGETALL 命令失败: {}", e))?;

                if result.is_empty() {
                    return Ok(QueryResult::empty());
                }

                let rows: Vec<serde_json::Value> = result
                    .into_iter()
                    .map(|(k, v)| serde_json::json!({"field": k, "value": v}))
                    .collect();

                Ok(QueryResult::with_rows(
                    rows,
                    vec!["field".to_string(), "value".to_string()],
                ))
            }
            "TTL" => {
                if args.is_empty() {
                    return Err("TTL 命令需要指定 key".to_string());
                }
                let key = &args[0];
                let ttl: i64 = manager
                    .ttl(key)
                    .await
                    .map_err(|e| format!("Redis TTL 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "ttl": ttl})],
                    vec!["key".to_string(), "ttl".to_string()],
                ))
            }
            "EXPIRE" => {
                if args.len() < 2 {
                    return Err("EXPIRE 命令需要指定 key 和 seconds".to_string());
                }
                let key = &args[0];
                let seconds: i64 = args[1]
                    .parse()
                    .map_err(|_| "EXPIRE 的 seconds 必须是整数")?;

                let result: i64 = manager
                    .expire(key, seconds)
                    .await
                    .map_err(|e| format!("Redis EXPIRE 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "success": result == 1})],
                    vec!["key".to_string(), "success".to_string()],
                ))
            }
            "KEYS" => {
                if args.is_empty() {
                    return Err("KEYS 命令需要指定 pattern".to_string());
                }
                let pattern = &args[0];
                let keys: Vec<String> = manager
                    .keys(pattern)
                    .await
                    .map_err(|e| format!("Redis KEYS 命令失败: {}", e))?;

                if keys.is_empty() {
                    return Ok(QueryResult::empty());
                }

                // 限制返回数量
                let truncated = keys.len() > 10000;
                let keys_to_return: Vec<String> = if truncated {
                    keys.into_iter().take(10000).collect()
                } else {
                    keys
                };

                let rows: Vec<serde_json::Value> = keys_to_return
                    .into_iter()
                    .map(|k| serde_json::json!({"key": k}))
                    .collect();

                let mut result = QueryResult::with_rows(rows, vec!["key".to_string()]);
                if truncated {
                    result = result.mark_truncated();
                }
                Ok(result)
            }
            "INCR" => {
                if args.is_empty() {
                    return Err("INCR 命令需要指定 key".to_string());
                }
                let key = &args[0];
                let mut delta: i64 = 1;
                if args.len() > 1 {
                    if let Ok(d) = args[1].parse::<i64>() {
                        delta = d;
                    } else {
                        return Err("INCR 命令的 delta 必须是整数".to_string());
                    }
                }
                let result: i64 = manager
                    .incr(key, delta)
                    .await
                    .map_err(|e| format!("Redis INCR 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "value": result})],
                    vec!["key".to_string(), "value".to_string()],
                ))
            }
            "DECR" => {
                if args.is_empty() {
                    return Err("DECR 命令需要指定 key".to_string());
                }
                let key = &args[0];
                let mut delta: i64 = 1;
                if args.len() > 1 {
                    if let Ok(d) = args[1].parse::<i64>() {
                        delta = d;
                    } else {
                        return Err("DECR 命令的 delta 必须是整数".to_string());
                    }
                }
                let result: i64 = manager
                    .decr(key, delta)
                    .await
                    .map_err(|e| format!("Redis DECR 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "value": result})],
                    vec!["key".to_string(), "value".to_string()],
                ))
            }
            "LPUSH" => {
                if args.len() < 2 {
                    return Err("LPUSH 命令需要指定 key 和 value".to_string());
                }
                let key = &args[0];
                let values = &args[1..];

                let result: i64 = manager
                    .lpush(key, values)
                    .await
                    .map_err(|e| format!("Redis LPUSH 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "length": result})],
                    vec!["key".to_string(), "length".to_string()],
                ))
            }
            "RPUSH" => {
                if args.len() < 2 {
                    return Err("RPUSH 命令需要指定 key 和 value".to_string());
                }
                let key = &args[0];
                let values = &args[1..];

                let result: i64 = manager
                    .rpush(key, values)
                    .await
                    .map_err(|e| format!("Redis RPUSH 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key": key, "length": result})],
                    vec!["key".to_string(), "length".to_string()],
                ))
            }
            "LRANGE" => {
                if args.len() < 3 {
                    return Err("LRANGE 命令需要指定 key、start 和 stop".to_string());
                }
                let key = &args[0];
                let start: isize = args[1]
                    .parse()
                    .map_err(|_| "LRANGE 的 start 必须是整数")?;
                let stop: isize = args[2]
                    .parse()
                    .map_err(|_| "LRANGE 的 stop 必须是整数")?;

                let result: Vec<String> = manager
                    .lrange(key, start, stop)
                    .await
                    .map_err(|e| format!("Redis LRANGE 命令失败: {}", e))?;

                if result.is_empty() {
                    return Ok(QueryResult::empty());
                }

                let rows: Vec<serde_json::Value> = result
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| serde_json::json!({"index": i, "value": v}))
                    .collect();

                Ok(QueryResult::with_rows(
                    rows,
                    vec!["index".to_string(), "value".to_string()],
                ))
            }
            "PING" => {
                let result: String = manager
                    .ping()
                    .await
                    .map_err(|e| format!("Redis PING 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"result": result})],
                    vec!["result".to_string()],
                ))
            }
            "INFO" => {
                let result: String = redis::cmd("INFO")
                    .query_async(&mut *manager)
                    .await
                    .map_err(|e| format!("Redis INFO 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"info": result})],
                    vec!["info".to_string()],
                ))
            }
            "DBSIZE" => {
                let result: i64 = redis::cmd("DBSIZE")
                    .query_async(&mut *manager)
                    .await
                    .map_err(|e| format!("Redis DBSIZE 命令失败: {}", e))?;

                Ok(QueryResult::with_rows(
                    vec![serde_json::json!({"key_count": result})],
                    vec!["key_count".to_string()],
                ))
            }
            // 未知命令，尝试作为原始命令执行
            _ => {
                Err(format!("不支持的 Redis 命令: {}。支持的命令: GET, SET, DEL, HGET, HSET, HDEL, HGETALL, TTL, EXPIRE, KEYS, INCR, DECR, LPUSH, RPUSH, LRANGE, PING, INFO, DBSIZE", command))
            }
        }
    }
}

#[async_trait]
impl DatabasePool for RedisConnection {
    /// 执行 Redis 命令
    ///
    /// # 参数
    ///
    /// * `sql` - Redis 命令（JSON 数组格式或命令字符串格式）
    /// * `params` - 额外参数（通常不使用，Redis 命令已在 sql 中指定）
    ///
    /// # 支持的命令格式
    ///
    /// 1. JSON 数组格式：`["SET", "key", "value"]`
    /// 2. 命令字符串格式：`SET key value`
    ///
    /// # 支持的命令
    ///
    /// - GET, SET, DEL
    /// - HGET, HSET, HDEL, HGETALL
    /// - TTL, EXPIRE
    /// - KEYS
    /// - INCR, DECR
    /// - LPUSH, RPUSH, LRANGE
    /// - PING, INFO, DBSIZE
    async fn execute(&self, sql: &str, _params: Vec<serde_json::Value>) -> Result<QueryResult, String> {
        log::debug!("[Redis] 执行命令: {}", sql);

        // 增加活跃操作计数
        {
            let mut active = self.active_operations.write().await;
            *active += 1;
        }

        // 解析命令
        let result = match Self::parse_command(sql) {
            Ok((command, args)) => self.execute_command(&command, args).await,
            Err(e) => Err(e),
        };

        // 减少活跃操作计数
        {
            let mut active = self.active_operations.write().await;
            *active = active.saturating_sub(1);
        }

        result
    }

    /// 测试连接是否正常
    ///
    /// 使用 PING 命令测试连接
    async fn test_connection(&self) -> Result<(), String> {
        log::info!("[Redis] 测试连接: name={}", self.name);

        let mut manager = self.manager.write().await;
        let result: String = manager
            .ping()
            .await
            .map_err(|e| format!("Redis PING 失败: {}", e))?;

        if result == "PONG" {
            log::info!("[Redis] 连接测试成功: name={}", self.name);
            Ok(())
        } else {
            Err(format!("Redis PING 返回异常: {}", result))
        }
    }

    /// 关闭连接池
    ///
    /// 注意: ConnectionManager 没有显式的 close 方法，会在 drop 时自动清理
    async fn close(&self) {
        log::info!("[Redis] 关闭连接池: name={}", self.name);
        // ConnectionManager 在 drop 时会自动清理连接
        // 这里只记录日志
    }

    /// 获取连接池状态信息
    fn status_info(&self) -> String {
        format!(
            "Redis pool: {} (url={})",
            self.name, self.connection_url_safe
        )
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_url_with_password() {
        let url = "redis://user:password@127.0.0.1:6379/0";
        let safe = RedisConnection::sanitize_url(url);
        assert!(!safe.contains("password"));
        assert!(safe.contains("***"));
    }

    #[test]
    fn test_sanitize_url_without_password() {
        let url = "redis://127.0.0.1:6379/0";
        let safe = RedisConnection::sanitize_url(url);
        assert_eq!(safe, url);
    }

    #[test]
    fn test_parse_command_json_array() {
        let cmd = r#"["SET", "mykey", "myvalue"]"#;
        let (command, args) = RedisConnection::parse_command(cmd).unwrap();
        assert_eq!(command, "SET");
        assert_eq!(args, vec!["mykey", "myvalue"]);
    }

    #[test]
    fn test_parse_command_string() {
        let cmd = "GET mykey";
        let (command, args) = RedisConnection::parse_command(cmd).unwrap();
        assert_eq!(command, "GET");
        assert_eq!(args, vec!["mykey"]);
    }

    #[test]
    fn test_parse_command_with_numbers() {
        let cmd = r#"["EXPIRE", "mykey", 3600]"#;
        let (command, args) = RedisConnection::parse_command(cmd).unwrap();
        assert_eq!(command, "EXPIRE");
        assert_eq!(args, vec!["mykey", "3600"]);
    }

    #[test]
    fn test_parse_command_empty() {
        let result = RedisConnection::parse_command("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_empty_array() {
        let result = RedisConnection::parse_command("[]");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_redis_url_from_config() {
        use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

        let config = DbConnection {
            id: "test-001".to_string(),
            name: "Test Redis".to_string(),
            driver: DbDriver::Redis,
            host: Some("192.168.1.1".to_string()),
            port: Some(6380),
            database: Some("1".to_string()),
            username: None,
            password: Some("secret".to_string()),
            password_from_env: None,
            connection_string: None,
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 5000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let url = RedisConnection::build_redis_url(&config).unwrap();
        assert!(url.contains("192.168.1.1"));
        assert!(url.contains("6380"));
        assert!(url.contains("secret"));
    }

    #[test]
    fn test_build_redis_url_from_connection_string() {
        use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

        let config = DbConnection {
            id: "test-002".to_string(),
            name: "Test Redis 2".to_string(),
            driver: DbDriver::Redis,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            password_from_env: None,
            connection_string: Some("redis://custom.redis.server:7000/5".to_string()),
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 5000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let url = RedisConnection::build_redis_url(&config).unwrap();
        assert_eq!(url, "redis://custom.redis.server:7000/5");
    }

    #[test]
    fn test_build_redis_url_default() {
        use super::super::types::{DbConnection, DbDriver, DbConnectionStatus};

        let config = DbConnection {
            id: "test-003".to_string(),
            name: "Test Redis 3".to_string(),
            driver: DbDriver::Redis,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            password_from_env: None,
            connection_string: None,
            min_connections: 1,
            max_connections: 10,
            connect_timeout_ms: 5000,
            idle_timeout_ms: 300000,
            status: DbConnectionStatus::Disconnected,
            last_error: None,
            last_tested_at: None,
        };

        let url = RedisConnection::build_redis_url(&config).unwrap();
        assert!(url.contains("127.0.0.1"));
        assert!(url.contains("6379"));
    }

    // 注意: 以下测试需要实际的 Redis 服务器运行
    // 可以通过设置环境变量 ENABLE_REDIS_INTEGRATION_TESTS=1 来启用

    #[cfg(feature = "redis-integration-tests")]
    mod integration_tests {
        use super::*;
        use std::env;

        fn get_redis_url() -> String {
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/15".to_string())
        }

        #[tokio::test]
        async fn test_redis_connection_and_ping() {
            let conn = RedisConnection::new(
                "test-ping".to_string(),
                get_redis_url(),
                5000,
                5000,
            )
            .await
            .unwrap();

            let result = conn.test_connection().await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_redis_set_and_get() {
            let conn = RedisConnection::new(
                "test-set-get".to_string(),
                get_redis_url(),
                5000,
                5000,
            )
            .await
            .unwrap();

            // SET
            let set_result = conn
                .execute(r#"["SET", "test_key", "test_value"]"#, vec![])
                .await
                .unwrap();
            assert!(set_result.rows.len() > 0);

            // GET
            let get_result = conn
                .execute(r#"["GET", "test_key"]"#, vec![])
                .await
                .unwrap();
            assert_eq!(get_result.rows.len(), 1);
            assert_eq!(
                get_result.rows[0].get("value"),
                Some(&serde_json::json!("test_value"))
            );

            // DEL
            let del_result = conn
                .execute(r#"["DEL", "test_key"]"#, vec![])
                .await
                .unwrap();
            assert_eq!(del_result.affected_rows, 1);
        }

        #[tokio::test]
        async fn test_redis_hash_operations() {
            let conn = RedisConnection::new(
                "test-hash".to_string(),
                get_redis_url(),
                5000,
                5000,
            )
            .await
            .unwrap();

            // HSET
            let hset_result = conn
                .execute(r#"["HSET", "test_hash", "field1", "value1"]"#, vec![])
                .await
                .unwrap();
            assert!(hset_result.rows.len() > 0);

            // HGET
            let hget_result = conn
                .execute(r#"["HGET", "test_hash", "field1"]"#, vec![])
                .await
                .unwrap();
            assert_eq!(hget_result.rows.len(), 1);

            // HGETALL
            let hgetall_result = conn
                .execute(r#"["HGETALL", "test_hash"]"#, vec![])
                .await
                .unwrap();
            assert!(hgetall_result.rows.len() > 0);

            // 清理
            conn.execute(r#"["DEL", "test_hash"]"#, vec![]).await.unwrap();
        }
    }
}
