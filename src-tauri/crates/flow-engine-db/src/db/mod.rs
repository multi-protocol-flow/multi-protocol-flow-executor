//! 数据库模块
//!
//! 提供数据库连接池管理功能，支持 PostgreSQL、MySQL、SQLite。
//! Wave 1: 基础连接池管理（pool.rs, traits.rs）
//! Wave 5: SQLite/MySQL/PostgreSQL 实现

/// `MySQL` 数据库驱动，基于 sqlx 实现连接池与事务管理
pub mod mysql;
/// 数据库连接池管理，维护全局连接池的注册、查询与生命周期
pub mod pool;
/// `PostgreSQL` 数据库驱动，基于 sqlx 实现连接池与事务管理
pub mod postgres;
/// `SQLite` 数据库驱动，基于 sqlx 实现本地文件数据库的连接与查询
pub mod sqlite;
/// 数据库通用 trait 定义，抽象不同数据库后端的连接池行为
pub mod traits;
/// 数据库通用类型定义，包含连接状态、驱动类型、测试结果等
pub mod types;

// Re-exports
pub use mysql::MySqlConnection;
pub use pool::{DatabaseConnectionManager, MockPool, PoolMetadata, TransactionInfo};
pub use postgres::PostgresConnection;
pub use sqlite::SqliteConnection;
pub use traits::{DatabasePool, IsolationLevel, QueryResult, TransactionState};
pub use types::{DbConnection, DbConnectionStatus, DbDriver, TestDbConnectionResult};
