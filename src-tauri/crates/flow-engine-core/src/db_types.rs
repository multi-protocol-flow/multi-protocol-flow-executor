// 数据库连接类型定义
//
// 与前端 src/types/database.ts 中的类型对齐

use serde::{Deserialize, Serialize};

/// 数据库驱动类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DbDriver {
    /// `PostgreSQL` 数据库
    #[default]
    Postgres,
    /// `MySQL` 数据库
    Mysql,
    /// `SQLite` 嵌入式数据库
    Sqlite,
    /// Redis 键值存储
    Redis,
}

impl std::fmt::Display for DbDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbDriver::Postgres => write!(f, "postgres"),
            DbDriver::Mysql => write!(f, "mysql"),
            DbDriver::Sqlite => write!(f, "sqlite"),
            DbDriver::Redis => write!(f, "redis"),
        }
    }
}

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbConnection {
    /// 连接 UUID
    pub id: String,
    /// 连接名称（显示用）
    pub name: String,
    /// 数据库驱动
    pub driver: DbDriver,

    // 连接参数
    /// 主机地址
    pub host: Option<String>,
    /// 端口号
    pub port: Option<u16>,
    /// 数据库名
    pub database: Option<String>,
    /// 用户名
    pub username: Option<String>,
    /// 密码（加密存储）
    pub password: Option<String>,
    /// 环境变量名（用于获取密码）
    pub password_from_env: Option<String>,
    /// 连接串（可选）
    pub connection_string: Option<String>,

    // 连接池配置
    /// 最小连接数（默认 1）
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    /// 最大连接数（默认 10）
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// 连接超时时间（毫秒，默认 10000）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// 空闲超时时间（毫秒，默认 300000）
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_ms: u64,

    // 运行时状态
    /// 连接状态
    #[serde(default = "default_status")]
    pub status: DbConnectionStatus,
    /// 最后Error信息
    pub last_error: Option<String>,
    /// 最后测试时间（时间戳）
    pub last_tested_at: Option<u64>,
}

fn default_min_connections() -> u32 {
    1
}

fn default_max_connections() -> u32 {
    10
}

fn default_connect_timeout() -> u64 {
    10000
}

fn default_idle_timeout() -> u64 {
    300000
}

fn default_status() -> DbConnectionStatus {
    DbConnectionStatus::Disconnected
}

/// 数据库连接状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DbConnectionStatus {
    /// 已连接，连接池可用
    Connected,
    /// 已断开，未建立连接
    #[default]
    Disconnected,
    /// 连接Error，上次操作Failed
    Error,
    /// 正在测试连接中
    Testing,
}

/// 测试数据库连接结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDbConnectionResult {
    /// 是否Success
    pub success: bool,
    /// Error信息（如果Failed）
    pub error: Option<String>,
}

impl TestDbConnectionResult {
    /// 创建Success结果
    pub fn success() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    /// 创建Failed结果
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_driver_serialize() {
        let driver = DbDriver::Postgres;
        let json = serde_json::to_string(&driver).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "\"postgres\"");

        let driver = DbDriver::Mysql;
        let json = serde_json::to_string(&driver).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "\"mysql\"");
    }

    #[test]
    fn test_db_driver_deserialize() {
        let driver: DbDriver =
            serde_json::from_str("\"postgres\"").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(driver, DbDriver::Postgres);

        let driver: DbDriver = serde_json::from_str("\"mysql\"").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(driver, DbDriver::Mysql);
    }

    #[test]
    fn test_db_connection_default_values() {
        let json = r#"{
            "id": "test-001",
            "name": "Test Connection",
            "driver": "postgres"
        }"#;

        let conn: DbConnection = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(conn.id, "test-001");
        assert_eq!(conn.name, "Test Connection");
        assert_eq!(conn.driver, DbDriver::Postgres);
        assert_eq!(conn.min_connections, 1);
        assert_eq!(conn.max_connections, 10);
        assert_eq!(conn.connect_timeout_ms, 10000);
        assert_eq!(conn.idle_timeout_ms, 300000);
        assert_eq!(conn.status, DbConnectionStatus::Disconnected);
    }

    #[test]
    fn test_db_connection_full() {
        let json = r#"{
            "id": "test-001",
            "name": "Test Connection",
            "driver": "postgres",
            "host": "localhost",
            "port": 5432,
            "database": "testdb",
            "username": "testuser",
            "password": "testpass",
            "minConnections": 2,
            "maxConnections": 20,
            "connectTimeoutMs": 5000,
            "idleTimeoutMs": 60000,
            "status": "disconnected"
        }"#;

        let conn: DbConnection = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(conn.host, Some("localhost".to_string()));
        assert_eq!(conn.port, Some(5432));
        assert_eq!(conn.database, Some("testdb".to_string()));
        assert_eq!(conn.username, Some("testuser".to_string()));
        assert_eq!(conn.password, Some("testpass".to_string()));
        assert_eq!(conn.min_connections, 2);
        assert_eq!(conn.max_connections, 20);
        assert_eq!(conn.connect_timeout_ms, 5000);
        assert_eq!(conn.idle_timeout_ms, 60000);
    }

    #[test]
    fn test_test_db_connection_result() {
        let success = TestDbConnectionResult::success();
        assert!(success.success);
        assert!(success.error.is_none());

        let failure = TestDbConnectionResult::failure("Connection failed");
        assert!(!failure.success);
        assert_eq!(failure.error, Some("Connection failed".to_string()));
    }

    #[test]
    fn test_db_connection_status_serialize() {
        let status = DbConnectionStatus::Connected;
        let json = serde_json::to_string(&status).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "\"connected\"");

        let status = DbConnectionStatus::Disconnected;
        let json = serde_json::to_string(&status).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "\"disconnected\"");
    }
}
