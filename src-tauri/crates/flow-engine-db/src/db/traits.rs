// 数据库连接池抽象 trait（从 flow-engine-core 重新导出）
//
// 核心类型已迁移至 flow-engine-core::traits 模块。
// 此文件保留重新导出、类型别名和测试。

// Re-export core types
pub use flow_engine_core::traits::{DatabasePool, IsolationLevel, QueryResult, TransactionState};

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
