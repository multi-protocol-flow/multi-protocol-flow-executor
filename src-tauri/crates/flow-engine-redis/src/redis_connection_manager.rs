//! Redis 连接管理器
//!
//! 实现 `RedisProvider` trait，管理 Redis 连接池的生命周期。

use async_trait::async_trait;
use flow_engine_core::traits::{
    RedisCloseOutput, RedisCommandOutput, RedisConnectOutput, RedisConnectionInfo, RedisProvider,
};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Redis 连接包装
struct RedisConnection {
    /// 连接管理器
    manager: ConnectionManager,
    /// 主机地址
    host: String,
    /// 端口
    port: u16,
    /// 数据库索引
    db_index: i64,
}

/// Redis 连接管理器
///
/// 持有所有 Redis 连接的 HashMap，线程安全。
pub struct RedisConnectionManager {
    connections: Arc<RwLock<HashMap<String, RedisConnection>>>,
}

impl RedisConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }



    /// 执行 Redis 命令，返回 RedisCommandOutput
    async fn execute_command_inner(
        manager: &mut ConnectionManager,
        command: &str,
        args: &[String],
    ) -> Result<RedisCommandOutput, String> {
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
                    Some(value) => Ok(RedisCommandOutput {
                        success: true,
                        command: Some("GET".to_string()),
                        row_count: Some(1),
                        data: Some(serde_json::json!([{"key": key, "value": value}])),
                        columns: Some(vec!["key".to_string(), "value".to_string()]),
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    }),
                    None => Ok(RedisCommandOutput {
                        success: true,
                        command: Some("GET".to_string()),
                        row_count: Some(0),
                        data: Some(serde_json::json!([])),
                        columns: None,
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    }),
                }
            }
            "SET" => {
                if args.len() < 2 {
                    return Err("SET 命令需要指定 key 和 value".to_string());
                }
                let key = &args[0];
                let value = &args[1];

                let mut cmd = redis::cmd("SET");
                cmd.arg(key).arg(value);

                let mut i = 2;
                while i < args.len() {
                    match args[i].to_uppercase().as_str() {
                        "EX" => {
                            if i + 1 < args.len() {
                                if let Ok(seconds) = args[i + 1].parse::<i64>() {
                                    cmd.arg("EX").arg(seconds);
                                }
                                i += 2;
                            } else {
                                i += 1;
                            }
                        }
                        "PX" => {
                            if i + 1 < args.len() {
                                if let Ok(ms) = args[i + 1].parse::<i64>() {
                                    cmd.arg("PX").arg(ms);
                                }
                                i += 2;
                            } else {
                                i += 1;
                            }
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
                    .query_async(manager)
                    .await
                    .map_err(|e| format!("Redis SET 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("SET".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "result": result}])),
                    columns: Some(vec!["key".to_string(), "result".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "DEL" => {
                if args.is_empty() {
                    return Err("DEL 命令需要指定 key".to_string());
                }
                let count: i64 = manager
                    .del(args)
                    .await
                    .map_err(|e| format!("Redis DEL 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("DEL".to_string()),
                    row_count: None,
                    data: None,
                    columns: None,
                    affected_rows: Some(count as u64),
                    truncated: false,
                    error: None,
                })
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
                    Some(value) => Ok(RedisCommandOutput {
                        success: true,
                        command: Some("HGET".to_string()),
                        row_count: Some(1),
                        data: Some(serde_json::json!([{"key": key, "field": field, "value": value}])),
                        columns: Some(vec!["key".to_string(), "field".to_string(), "value".to_string()]),
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    }),
                    None => Ok(RedisCommandOutput {
                        success: true,
                        command: Some("HGET".to_string()),
                        row_count: Some(0),
                        data: Some(serde_json::json!([])),
                        columns: None,
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    }),
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

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("HSET".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "field": field, "added": result}])),
                    columns: Some(vec!["key".to_string(), "field".to_string(), "added".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "HDEL" => {
                if args.len() < 2 {
                    return Err("HDEL 命令需要指定 key 和 field".to_string());
                }
                let key = &args[0];
                let fields: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

                let count: i64 = manager
                    .hdel(key, &fields[..])
                    .await
                    .map_err(|e| format!("Redis HDEL 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("HDEL".to_string()),
                    row_count: None,
                    data: None,
                    columns: None,
                    affected_rows: Some(count as u64),
                    truncated: false,
                    error: None,
                })
            }
            "HGETALL" => {
                if args.is_empty() {
                    return Err("HGETALL 命令需要指定 key".to_string());
                }
                let key = &args[0];
                let result: HashMap<String, String> = manager
                    .hgetall(key)
                    .await
                    .map_err(|e| format!("Redis HGETALL 命令失败: {}", e))?;

                if result.is_empty() {
                    return Ok(RedisCommandOutput {
                        success: true,
                        command: Some("HGETALL".to_string()),
                        row_count: Some(0),
                        data: Some(serde_json::json!([])),
                        columns: None,
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    });
                }

                let rows: Vec<serde_json::Value> = result
                    .into_iter()
                    .map(|(k, v)| serde_json::json!({"field": k, "value": v}))
                    .collect();

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("HGETALL".to_string()),
                    row_count: Some(rows.len() as u64),
                    data: Some(serde_json::json!(rows)),
                    columns: Some(vec!["field".to_string(), "value".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
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

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("TTL".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "ttl": ttl}])),
                    columns: Some(vec!["key".to_string(), "ttl".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "EXPIRE" => {
                if args.len() < 2 {
                    return Err("EXPIRE 命令需要指定 key 和 seconds".to_string());
                }
                let key = &args[0];
                let seconds: i64 = args[1]
                    .parse()
                    .map_err(|_| "EXPIRE 的 seconds 必须是整数".to_string())?;

                let result: i64 = manager
                    .expire(key, seconds)
                    .await
                    .map_err(|e| format!("Redis EXPIRE 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("EXPIRE".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "success": result == 1}])),
                    columns: Some(vec!["key".to_string(), "success".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
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
                    return Ok(RedisCommandOutput {
                        success: true,
                        command: Some("KEYS".to_string()),
                        row_count: Some(0),
                        data: Some(serde_json::json!([])),
                        columns: None,
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    });
                }

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

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("KEYS".to_string()),
                    row_count: Some(rows.len() as u64),
                    data: Some(serde_json::json!(rows)),
                    columns: Some(vec!["key".to_string()]),
                    affected_rows: None,
                    truncated,
                    error: None,
                })
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

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("INCR".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "value": result}])),
                    columns: Some(vec!["key".to_string(), "value".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
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

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("DECR".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "value": result}])),
                    columns: Some(vec!["key".to_string(), "value".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "LPUSH" => {
                if args.len() < 2 {
                    return Err("LPUSH 命令需要指定 key 和 value".to_string());
                }
                let key = &args[0];
                let values: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

                let result: i64 = manager
                    .lpush(key, &values[..])
                    .await
                    .map_err(|e| format!("Redis LPUSH 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("LPUSH".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "length": result}])),
                    columns: Some(vec!["key".to_string(), "length".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "RPUSH" => {
                if args.len() < 2 {
                    return Err("RPUSH 命令需要指定 key 和 value".to_string());
                }
                let key = &args[0];
                let values: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

                let result: i64 = manager
                    .rpush(key, &values[..])
                    .await
                    .map_err(|e| format!("Redis RPUSH 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("RPUSH".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "length": result}])),
                    columns: Some(vec!["key".to_string(), "length".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "LRANGE" => {
                if args.len() < 3 {
                    return Err("LRANGE 命令需要指定 key、start 和 stop".to_string());
                }
                let key = &args[0];
                let start: isize = args[1]
                    .parse()
                    .map_err(|_| "LRANGE 的 start 必须是整数".to_string())?;
                let stop: isize = args[2]
                    .parse()
                    .map_err(|_| "LRANGE 的 stop 必须是整数".to_string())?;

                let result: Vec<String> = manager
                    .lrange(key, start, stop)
                    .await
                    .map_err(|e| format!("Redis LRANGE 命令失败: {}", e))?;

                if result.is_empty() {
                    return Ok(RedisCommandOutput {
                        success: true,
                        command: Some("LRANGE".to_string()),
                        row_count: Some(0),
                        data: Some(serde_json::json!([])),
                        columns: None,
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    });
                }

                let rows: Vec<serde_json::Value> = result
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| serde_json::json!({"index": i, "value": v}))
                    .collect();

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("LRANGE".to_string()),
                    row_count: Some(rows.len() as u64),
                    data: Some(serde_json::json!(rows)),
                    columns: Some(vec!["index".to_string(), "value".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "PING" => {
                let result: String = manager
                    .ping()
                    .await
                    .map_err(|e| format!("Redis PING 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("PING".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"result": result}])),
                    columns: Some(vec!["result".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "INFO" => {
                let result: String = redis::cmd("INFO")
                    .query_async(manager)
                    .await
                    .map_err(|e| format!("Redis INFO 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("INFO".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"info": result}])),
                    columns: Some(vec!["info".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "DBSIZE" => {
                let result: i64 = redis::cmd("DBSIZE")
                    .query_async(manager)
                    .await
                    .map_err(|e| format!("Redis DBSIZE 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("DBSIZE".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key_count": result}])),
                    columns: Some(vec!["key_count".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            // Set 类型命令
            "SADD" => {
                if args.len() < 2 {
                    return Err("SADD 命令需要指定 key 和至少一个 member".to_string());
                }
                let key = &args[0];
                let members: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
                let count: i64 = manager
                    .sadd(key, &members[..])
                    .await
                    .map_err(|e| format!("Redis SADD 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("SADD".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "added": count}])),
                    columns: Some(vec!["key".to_string(), "added".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "SMEMBERS" => {
                if args.is_empty() {
                    return Err("SMEMBERS 命令需要指定 key".to_string());
                }
                let key = &args[0];
                let members: Vec<String> = manager
                    .smembers(key)
                    .await
                    .map_err(|e| format!("Redis SMEMBERS 命令失败: {}", e))?;

                if members.is_empty() {
                    return Ok(RedisCommandOutput {
                        success: true,
                        command: Some("SMEMBERS".to_string()),
                        row_count: Some(0),
                        data: Some(serde_json::json!([])),
                        columns: None,
                        affected_rows: None,
                        truncated: false,
                        error: None,
                    });
                }

                let rows: Vec<serde_json::Value> = members
                    .into_iter()
                    .map(|m| serde_json::json!({"member": m}))
                    .collect();

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("SMEMBERS".to_string()),
                    row_count: Some(rows.len() as u64),
                    data: Some(serde_json::json!(rows)),
                    columns: Some(vec!["member".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            "SREM" => {
                if args.len() < 2 {
                    return Err("SREM 命令需要指定 key 和至少一个 member".to_string());
                }
                let key = &args[0];
                let members: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
                let count: i64 = manager
                    .srem(key, &members[..])
                    .await
                    .map_err(|e| format!("Redis SREM 命令失败: {}", e))?;

                Ok(RedisCommandOutput {
                    success: true,
                    command: Some("SREM".to_string()),
                    row_count: Some(1),
                    data: Some(serde_json::json!([{"key": key, "removed": count}])),
                    columns: Some(vec!["key".to_string(), "removed".to_string()]),
                    affected_rows: None,
                    truncated: false,
                    error: None,
                })
            }
            _ => Err(format!(
                "不支持的 Redis 命令: {}。支持的命令: GET, SET, DEL, HGET, HSET, HDEL, HGETALL, TTL, EXPIRE, KEYS, INCR, DECR, LPUSH, RPUSH, LRANGE, PING, INFO, DBSIZE, SADD, SMEMBERS, SREM",
                command
            )),
        }
    }
}

impl Default for RedisConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RedisProvider for RedisConnectionManager {
    async fn connect(
        &self,
        connection_id: &str,
        host: &str,
        port: u16,
        password: Option<&str>,
        db_index: i64,
        connect_timeout_ms: u64,
    ) -> Result<RedisConnectOutput, String> {
        log::info!(
            "[Redis] 创建连接: id={}, host={}, port={}, db={}",
            connection_id,
            host,
            port,
            db_index
        );

        // 使用 ConnectionInfo 避免 URL 特殊字符解析问题
        use redis::{ConnectionAddr, ConnectionInfo, RedisConnectionInfo};
        let connection_info = ConnectionInfo {
            addr: ConnectionAddr::Tcp(host.to_string(), port),
            redis: RedisConnectionInfo {
                db: db_index,
                username: None,
                password: password.map(|s| s.to_string()),
                protocol: redis::ProtocolVersion::RESP2,
            },
        };
        let client = redis::Client::open(connection_info)
            .map_err(|e| format!("Redis 客户端创建失败: {}", e))?;

        let config = redis::aio::ConnectionManagerConfig::new()
            .set_connection_timeout(Duration::from_millis(connect_timeout_ms))
            .set_number_of_retries(3)
            .set_max_delay(5000)
            .set_exponent_base(2);

        let mut manager = ConnectionManager::new_with_config(client, config)
            .await
            .map_err(|e| format!("Redis 连接失败: {}", e))?;

        // 选库
        if db_index != 0 {
            let mut cmd = redis::cmd("SELECT");
            cmd.arg(db_index);
            cmd.query_async::<()>(&mut manager)
                .await
                .map_err(|e| format!("Redis SELECT {} 失败: {}", db_index, e))?;
        }

        let conn = RedisConnection {
            manager,
            host: host.to_string(),
            port,
            db_index,
        };

        self.connections
            .write()
            .await
            .insert(connection_id.to_string(), conn);

        log::info!("[Redis] 连接成功: id={}", connection_id);

        Ok(RedisConnectOutput {
            success: true,
            connection_id: Some(connection_id.to_string()),
            server_info: Some(format!("{}:{}", host, port)),
            error: None,
        })
    }

    async fn command(
        &self,
        connection_id: &str,
        command: &str,
        args: &[String],
    ) -> Result<RedisCommandOutput, String> {
        let mut connections = self.connections.write().await;
        let conn = connections
            .get_mut(connection_id)
            .ok_or_else(|| format!("Redis 连接不存在: {}", connection_id))?;

        Self::execute_command_inner(&mut conn.manager, command, args).await
    }

    async fn close(&self, connection_id: &str) -> Result<RedisCloseOutput, String> {
        let mut connections = self.connections.write().await;
        connections
            .remove(connection_id)
            .ok_or_else(|| format!("Redis 连接不存在: {}", connection_id))?;

        log::info!("[Redis] 连接关闭: id={}", connection_id);

        Ok(RedisCloseOutput {
            success: true,
            error: None,
        })
    }

    fn get_status(&self, connection_id: &str) -> Option<String> {
        let connections = self.connections.blocking_read();
        if connections.contains_key(connection_id) {
            Some("connected".to_string())
        } else {
            None
        }
    }

    fn list_connections(&self) -> Vec<RedisConnectionInfo> {
        let connections = self.connections.blocking_read();
        connections
            .iter()
            .map(|(id, conn)| RedisConnectionInfo {
                connection_id: id.clone(),
                host: conn.host.clone(),
                port: conn.port,
                db_index: conn.db_index,
            })
            .collect()
    }
}
