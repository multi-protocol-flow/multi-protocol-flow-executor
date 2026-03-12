// 数据库连接池抽象 trait
//
// 定义数据库连接池的通用接口，支持多种数据库类型。
// Wave 1: 基础 trait 定义
// Wave 5: SQLite/Redis 实现

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
    ///
    /// # 参数
    ///
    /// * `sql` - SQL 查询语句
    /// * `params` - 参数列表（JSON 格式，用于参数化查询）
    ///
    /// # 返回
    ///
    /// * `Ok(QueryResult)` - 查询成功，返回结果集
    /// * `Err(String)` - 查询失败，返回错误信息
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let result = pool.execute(
    ///     "SELECT * FROM users WHERE id = $1",
    ///     vec![json!(1)]
    /// ).await?;
    /// ```
    async fn execute(&self, sql: &str, params: Vec<serde_json::Value>) -> Result<QueryResult, String>;

    /// 测试连接是否正常
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 连接正常
    /// * `Err(String)` - 连接失败，返回错误信息
    async fn test_connection(&self) -> Result<(), String>;

    /// 关闭连接池
    ///
    /// 释放所有连接资源，关闭连接池。
    /// 调用后，连接池不可再使用。
    async fn close(&self);

    /// 获取连接池状态信息（用于调试）
    ///
    /// # 返回
    ///
    /// 连接池状态字符串（例如："PostgreSQL pool: 5/10 connections"）
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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_result_default() {
        let result = QueryResult::default();
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.row_count, 0);
        assert_eq!(result.affected_rows, 0);
        assert_eq!(result.columns.len(), 0);
        assert!(!result.truncated);
    }

    #[test]
    fn test_query_result_empty() {
        let result = QueryResult::empty();
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.row_count, 0);
    }

    #[test]
    fn test_query_result_with_affected_rows() {
        let result = QueryResult::with_affected_rows(10);
        assert_eq!(result.affected_rows, 10);
        assert_eq!(result.rows.len(), 0);
    }

    #[test]
    fn test_query_result_with_rows() {
        let rows = vec![
            serde_json::json!({"id": 1, "name": "Alice"}),
            serde_json::json!({"id": 2, "name": "Bob"}),
        ];
        let columns = vec!["id".to_string(), "name".to_string()];
        
        let result = QueryResult::with_rows(rows.clone(), columns.clone());
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.row_count, 2);
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.affected_rows, 0);
        assert!(!result.truncated);
    }

    #[test]
    fn test_query_result_mark_truncated() {
        let result = QueryResult::empty().mark_truncated();
        assert!(result.truncated);
    }

    #[test]
    fn test_isolation_level_default() {
        let level = IsolationLevel::default();
        assert_eq!(level, IsolationLevel::ReadCommitted);
    }

    #[test]
    fn test_isolation_level_from_str() {
        assert_eq!(
            IsolationLevel::from_str("READ COMMITTED"),
            Some(IsolationLevel::ReadCommitted)
        );
        assert_eq!(
            IsolationLevel::from_str("read_committed"),
            Some(IsolationLevel::ReadCommitted)
        );
        assert_eq!(
            IsolationLevel::from_str("REPEATABLE READ"),
            Some(IsolationLevel::RepeatableRead)
        );
        assert_eq!(
            IsolationLevel::from_str("SERIALIZABLE"),
            Some(IsolationLevel::Serializable)
        );
        assert_eq!(IsolationLevel::from_str("INVALID"), None);
    }

    #[test]
    fn test_isolation_level_to_postgres_str() {
        assert_eq!(
            IsolationLevel::ReadCommitted.to_postgres_str(),
            "READ COMMITTED"
        );
        assert_eq!(
            IsolationLevel::Serializable.to_postgres_str(),
            "SERIALIZABLE"
        );
    }

    #[test]
    fn test_isolation_level_to_mysql_str() {
        assert_eq!(
            IsolationLevel::RepeatableRead.to_mysql_str(),
            "REPEATABLE READ"
        );
    }

    #[test]
    fn test_transaction_state_serialization() {
        let state = TransactionState::Active;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""Active""#);

        let deserialized: TransactionState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TransactionState::Active);
    }

    #[test]
    fn test_isolation_level_serialization() {
        let level = IsolationLevel::ReadCommitted;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, r#""READ_COMMITTED""#);

        let deserialized: IsolationLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, IsolationLevel::ReadCommitted);
    }
}
