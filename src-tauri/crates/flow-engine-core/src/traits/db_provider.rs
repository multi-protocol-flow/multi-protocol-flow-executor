//! 数据库提供者抽象 trait 和相关类型
//!
//! 包含数据库连接池抽象、查询结果、事务状态和隔离级别等核心类型。
//! 从 flow-engine/src/db/traits.rs 迁移至 core crate。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// 查询返回的行数据（JSON 数组）
    pub rows: Vec<serde_json::Value>,
    /// 返回行数
    pub row_count: usize,
    /// 影响的行数（INSERT/UPDATE/DELETE）
    pub affected_rows: u64,
    /// 列名列表
    pub columns: Vec<String>,
    /// 是否被截断（超出 10000 行限制）
    pub truncated: bool,
}

impl Default for QueryResult {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            row_count: 0,
            affected_rows: 0,
            columns: Vec::new(),
            truncated: false,
        }
    }
}

impl QueryResult {
    /// 创建空的查询结果
    pub fn empty() -> Self {
        Self::default()
    }

    /// 创建仅包含影响行数的结果（用于 INSERT/UPDATE/DELETE）
    pub fn with_affected_rows(affected_rows: u64) -> Self {
        Self {
            affected_rows,
            ..Default::default()
        }
    }

    /// 创建包含行数据的结果
    pub fn with_rows(rows: Vec<serde_json::Value>, columns: Vec<String>) -> Self {
        let row_count = rows.len();
        Self {
            rows,
            row_count,
            columns,
            affected_rows: 0,
            truncated: false,
        }
    }

    /// 标记结果被截断
    pub fn mark_truncated(mut self) -> Self {
        self.truncated = true;
        self
    }
}

/// 数据库连接池抽象 trait
///
/// 定义数据库连接池的通用操作接口，支持 PostgreSQL、MySQL、SQLite、Redis 等多种数据库。
/// 所有方法都是异步的，使用 `async_trait` 实现跨 trait 异步支持。
#[async_trait]
pub trait DatabasePool: Send + Sync {
    /// 执行 SQL 查询
    async fn execute(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<QueryResult, String>;

    /// 测试连接是否正常
    async fn test_connection(&self) -> Result<(), String>;

    /// 关闭连接池
    async fn close(&self);

    /// 获取连接池状态信息（用于调试）
    fn status_info(&self) -> String;
}

/// 事务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionState {
    /// 事务已开始
    Active,
    /// 事务已提交
    Committed,
    /// 事务已回滚
    RolledBack,
}

/// 事务隔离级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IsolationLevel {
    /// 读未提交
    ReadUncommitted,
    /// 读已提交（默认）
    #[default]
    ReadCommitted,
    /// 可重复读
    RepeatableRead,
    /// 可序列化
    Serializable,
}

impl IsolationLevel {
    /// 从字符串解析隔离级别
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().replace('_', " ").as_str() {
            "READ UNCOMMITTED" => Some(Self::ReadUncommitted),
            "READ COMMITTED" => Some(Self::ReadCommitted),
            "REPEATABLE READ" => Some(Self::RepeatableRead),
            "SERIALIZABLE" => Some(Self::Serializable),
            _ => None,
        }
    }

    /// 转换为 PostgreSQL 隔离级别字符串
    pub fn to_postgres_str(&self) -> &'static str {
        match self {
            Self::ReadUncommitted => "READ UNCOMMITTED",
            Self::ReadCommitted => "READ COMMITTED",
            Self::RepeatableRead => "REPEATABLE READ",
            Self::Serializable => "SERIALIZABLE",
        }
    }

    /// 转换为 MySQL 隔离级别字符串
    pub fn to_mysql_str(&self) -> &'static str {
        match self {
            Self::ReadUncommitted => "READ UNCOMMITTED",
            Self::ReadCommitted => "READ COMMITTED",
            Self::RepeatableRead => "REPEATABLE READ",
            Self::Serializable => "SERIALIZABLE",
        }
    }
}

/// 数据库提供者 trait
///
/// 抽象数据库连接管理功能，使得执行器可以与具体的数据库实现解耦。
#[async_trait]
pub trait DbProvider: Send + Sync {
    /// 获取连接池
    async fn get_pool(&self, connection_uuid: &str) -> Option<std::sync::Arc<dyn DatabasePool>>;

    /// 关闭连接池
    async fn close_pool(&self, connection_uuid: &str) -> Result<(), String>;

    /// 开始事务
    async fn begin_transaction(
        &self,
        connection_uuid: &str,
        isolation_level: Option<String>,
    ) -> Result<(), String>;

    /// 提交事务
    async fn commit_transaction(&self, connection_uuid: &str) -> Result<(), String>;

    /// 回滚事务
    async fn rollback_transaction(&self, connection_uuid: &str) -> Result<(), String>;

    /// 获取事务状态
    async fn get_transaction_state(&self, connection_uuid: &str) -> Option<TransactionState>;
}
