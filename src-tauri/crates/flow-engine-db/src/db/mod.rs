// 数据库模块
//
// 提供数据库连接池管理功能，支持 PostgreSQL、MySQL、SQLite、Redis。
// Wave 1: 基础连接池管理（pool.rs, traits.rs）
// Wave 5: SQLite/Redis/MySQL/PostgreSQL 实现

pub mod mysql;
pub mod pool;
pub mod postgres;
pub mod redis;
pub mod sqlite;
pub mod traits;
pub mod types;

// Re-exports
pub use mysql::MySqlConnection;
pub use pool::{DatabaseConnectionManager, MockPool, PoolMetadata, TransactionInfo};
pub use postgres::PostgresConnection;
pub use redis::RedisConnection;
pub use sqlite::SqliteConnection;
pub use traits::{DatabasePool, IsolationLevel, QueryResult, TransactionState};
pub use types::{DbConnection, DbConnectionStatus, DbDriver, TestDbConnectionResult};
