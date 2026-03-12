// 数据库模块
//
// 提供数据库连接池管理功能，支持 PostgreSQL、MySQL、SQLite、Redis。
// Wave 1: 基础连接池管理（pool.rs, traits.rs）
// Wave 5: SQLite/Redis 实现

pub mod pool;
pub mod traits;

// Re-exports
pub use pool::{DatabaseConnectionManager, MockPool, PoolMetadata, TransactionInfo};
pub use traits::{DatabasePool, IsolationLevel, QueryResult, TransactionState};
