// 数据库连接类型定义
//
// 从 flow-engine-core 重新导出，保持类型统一。
// 避免与 flow_engine_core::db_types::DbConnection 出现类型不匹配。

pub use flow_engine_core::db_types::{
    DbConnection, DbConnectionStatus, DbDriver, TestDbConnectionResult,
};
