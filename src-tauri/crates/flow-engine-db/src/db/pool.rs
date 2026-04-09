// 数据库连接池管理器
//
// 管理多个数据库连接池，支持 PostgreSQL、MySQL、SQLite、Redis。
// Wave 1: 基础连接池管理
// Wave 2: 与执行器集成
// Wave 5: SQLite/Redis 支持

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::traits::{DatabasePool, QueryResult, TransactionState};

/// 活跃的数据库连接池
///
/// 存储单个数据库连接池的元数据和实际的连接池。
pub struct ActivePool {
    /// 连接 UUID（前端配置的唯一标识）
    pub connection_uuid: String,
    /// 连接名称
    pub name: String,
    /// 数据库类型（postgres/mysql/sqlite/redis）
    pub db_type: String,
    /// 创建时间戳（Unix 时间戳，毫秒）
    pub created_at: u64,
    /// 最后使用时间戳（Unix 时间戳，毫秒）
    pub last_used_at: u64,
    /// 最后错误信息
    pub last_error: Option<String>,

    /// 实际的连接池（trait object）
    pool: Option<Arc<dyn DatabasePool>>,
}

impl std::fmt::Debug for ActivePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActivePool")
            .field("connection_uuid", &self.connection_uuid)
            .field("name", &self.name)
            .field("db_type", &self.db_type)
            .field("created_at", &self.created_at)
            .field("last_used_at", &self.last_used_at)
            .field("last_error", &self.last_error)
            .field("has_pool", &self.pool.is_some())
            .finish()
    }
}

impl ActivePool {
    /// 创建新的活跃连接池
    pub fn new(connection_uuid: String, name: String, db_type: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            connection_uuid,
            name,
            db_type,
            created_at: now,
            last_used_at: now,
            last_error: None,
            pool: None,
        }
    }

    /// 设置连接池
    pub fn set_pool(&mut self, pool: Arc<dyn DatabasePool>) {
        self.pool = Some(pool);
    }

    /// 获取连接池
    pub fn get_pool(&self) -> Option<Arc<dyn DatabasePool>> {
        self.pool.clone()
    }

    /// 更新最后使用时间
    pub fn touch(&mut self) {
        self.last_used_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// 设置错误信息
    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    /// 清除错误信息
    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    /// 检查连接池是否可用
    pub fn is_available(&self) -> bool {
        self.pool.is_some() && self.last_error.is_none()
    }
}

/// 事务信息
#[derive(Debug, Clone)]
pub struct TransactionInfo {
    /// 连接 UUID
    pub connection_uuid: String,
    /// 事务状态
    pub state: TransactionState,
    /// 创建时间戳
    pub created_at: u64,
    /// 隔离级别（可选）
    pub isolation_level: Option<String>,
}

impl TransactionInfo {
    /// 创建新的事务信息
    pub fn new(connection_uuid: String, isolation_level: Option<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            connection_uuid,
            state: TransactionState::Active,
            created_at: now,
            isolation_level,
        }
    }
}

/// 数据库连接池管理器
///
/// 管理所有活跃的数据库连接池，提供线程安全的连接池操作。
/// 使用 `Arc<RwLock<>>` 实现跨线程共享。
#[derive(Debug, Clone)]
pub struct DatabaseConnectionManager {
    /// `连接池：connection_uuid` -> `ActivePool`
    pools: Arc<RwLock<HashMap<String, ActivePool>>>,

    /// `事务状态：connection_uuid` -> `TransactionInfo`
    transactions: Arc<RwLock<HashMap<String, TransactionInfo>>>,
}

impl DatabaseConnectionManager {
    /// 创建新的连接池管理器
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建连接池
    pub async fn create_pool(
        &self,
        connection_uuid: String,
        name: String,
        db_type: String,
        pool: Arc<dyn DatabasePool>,
    ) -> Result<(), String> {
        let mut pools = self.pools.write().await;

        if pools.contains_key(&connection_uuid) {
            return Err(format!("连接池已存在: {}", connection_uuid));
        }

        let mut active_pool = ActivePool::new(connection_uuid.clone(), name, db_type);
        active_pool.set_pool(pool);

        pools.insert(connection_uuid.clone(), active_pool);

        log::info!(
            "[Database] 连接池创建成功: uuid={}, type={}",
            connection_uuid,
            pools.get(&connection_uuid).unwrap().db_type
        );

        Ok(())
    }

    /// 获取连接池
    pub async fn get_pool(&self, connection_uuid: &str) -> Option<Arc<dyn DatabasePool>> {
        let mut pools = self.pools.write().await;

        if let Some(active_pool) = pools.get_mut(connection_uuid) {
            active_pool.touch();
            active_pool.get_pool()
        } else {
            None
        }
    }

    /// 获取连接池元数据
    pub async fn get_pool_metadata(&self, connection_uuid: &str) -> Option<PoolMetadata> {
        let pools = self.pools.read().await;

        pools.get(connection_uuid).map(|p| PoolMetadata {
            connection_uuid: p.connection_uuid.clone(),
            name: p.name.clone(),
            db_type: p.db_type.clone(),
            created_at: p.created_at,
            last_used_at: p.last_used_at,
            last_error: p.last_error.clone(),
            is_available: p.is_available(),
        })
    }

    /// 关闭连接池
    pub async fn close_pool(&self, connection_uuid: &str) -> Result<(), String> {
        let mut pools = self.pools.write().await;

        if let Some(active_pool) = pools.remove(connection_uuid) {
            if let Some(pool) = active_pool.get_pool() {
                pool.close().await;
            }

            let mut transactions = self.transactions.write().await;
            transactions.remove(connection_uuid);

            log::info!("[Database] 连接池已关闭: uuid={}", connection_uuid);
            Ok(())
        } else {
            Err(format!("连接池不存在: {}", connection_uuid))
        }
    }

    /// 开始事务
    pub async fn begin_transaction(
        &self,
        connection_uuid: &str,
        isolation_level: Option<String>,
    ) -> Result<(), String> {
        let mut transactions = self.transactions.write().await;

        if let Some(info) = transactions.get(connection_uuid) {
            if info.state == TransactionState::Active {
                return Err(format!(
                    "连接 {} 已有活跃事务，请先提交或回滚",
                    connection_uuid
                ));
            }
        }

        let info = TransactionInfo::new(connection_uuid.to_string(), isolation_level);
        transactions.insert(connection_uuid.to_string(), info);

        log::info!("[Database] 事务已开始: uuid={}", connection_uuid);
        Ok(())
    }

    /// 提交事务
    pub async fn commit_transaction(&self, connection_uuid: &str) -> Result<(), String> {
        let mut transactions = self.transactions.write().await;

        if let Some(info) = transactions.get_mut(connection_uuid) {
            if info.state != TransactionState::Active {
                return Err(format!(
                    "连接 {} 没有活跃事务（当前状态: {:?}）",
                    connection_uuid, info.state
                ));
            }

            info.state = TransactionState::Committed;
            log::info!("[Database] 事务已提交: uuid={}", connection_uuid);
            Ok(())
        } else {
            Err(format!("连接 {} 没有活跃事务", connection_uuid))
        }
    }

    /// 回滚事务
    pub async fn rollback_transaction(&self, connection_uuid: &str) -> Result<(), String> {
        let mut transactions = self.transactions.write().await;

        if let Some(info) = transactions.get_mut(connection_uuid) {
            if info.state != TransactionState::Active {
                return Err(format!(
                    "连接 {} 没有活跃事务（当前状态: {:?}）",
                    connection_uuid, info.state
                ));
            }

            info.state = TransactionState::RolledBack;
            log::info!("[Database] 事务已回滚: uuid={}", connection_uuid);
            Ok(())
        } else {
            Err(format!("连接 {} 没有活跃事务", connection_uuid))
        }
    }

    /// 获取事务状态
    pub async fn get_transaction_state(&self, connection_uuid: &str) -> Option<TransactionState> {
        let transactions = self.transactions.read().await;
        transactions.get(connection_uuid).map(|info| info.state.clone())
    }

    /// 清理已提交/回滚的事务
    pub async fn cleanup_transactions(&self) {
        let mut transactions = self.transactions.write().await;
        transactions.retain(|_, info| info.state == TransactionState::Active);
    }

    /// 获取连接池数量
    pub async fn pool_count(&self) -> usize {
        let pools = self.pools.read().await;
        pools.len()
    }

    /// 获取活跃连接池数量（可用的连接池）
    pub async fn active_pool_count(&self) -> usize {
        let pools = self.pools.read().await;
        pools.values().filter(|p| p.is_available()).count()
    }

    /// 获取所有连接池的元数据
    pub async fn get_all_pools_metadata(&self) -> Vec<PoolMetadata> {
        let pools = self.pools.read().await;
        pools
            .values()
            .map(|p| PoolMetadata {
                connection_uuid: p.connection_uuid.clone(),
                name: p.name.clone(),
                db_type: p.db_type.clone(),
                created_at: p.created_at,
                last_used_at: p.last_used_at,
                last_error: p.last_error.clone(),
                is_available: p.is_available(),
            })
            .collect()
    }

    /// 清空所有连接池
    pub async fn clear(&self) {
        let mut pools = self.pools.write().await;
        let mut transactions = self.transactions.write().await;

        for (_, active_pool) in pools.drain() {
            if let Some(pool) = active_pool.get_pool() {
                pool.close().await;
            }
        }

        transactions.clear();

        log::info!("[Database] 所有连接池已清空");
    }

    /// 更新连接池错误状态
    pub async fn set_pool_error(&self, connection_uuid: &str, error: &str) {
        let mut pools = self.pools.write().await;
        if let Some(active_pool) = pools.get_mut(connection_uuid) {
            active_pool.set_error(error.to_string());
        }
    }

    /// 清除连接池错误状态
    pub async fn clear_pool_error(&self, connection_uuid: &str) {
        let mut pools = self.pools.write().await;
        if let Some(active_pool) = pools.get_mut(connection_uuid) {
            active_pool.clear_error();
        }
    }
}

impl Default for DatabaseConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

// DbProvider trait 实现
#[async_trait::async_trait]
impl flow_engine_core::traits::DbProvider for DatabaseConnectionManager {
    async fn create_pool(
        &self,
        connection_uuid: String,
        name: String,
        db_type: String,
        pool: Arc<dyn DatabasePool>,
    ) -> Result<(), String> {
        DatabaseConnectionManager::create_pool(self, connection_uuid, name, db_type, pool).await
    }

    async fn get_pool(&self, connection_uuid: &str) -> Option<Arc<dyn DatabasePool>> {
        DatabaseConnectionManager::get_pool(self, connection_uuid).await
    }

    async fn close_pool(&self, connection_uuid: &str) -> Result<(), String> {
        DatabaseConnectionManager::close_pool(self, connection_uuid).await
    }

    async fn begin_transaction(
        &self,
        connection_uuid: &str,
        isolation_level: Option<String>,
    ) -> Result<(), String> {
        DatabaseConnectionManager::begin_transaction(self, connection_uuid, isolation_level).await
    }

    async fn commit_transaction(&self, connection_uuid: &str) -> Result<(), String> {
        DatabaseConnectionManager::commit_transaction(self, connection_uuid).await
    }

    async fn rollback_transaction(&self, connection_uuid: &str) -> Result<(), String> {
        DatabaseConnectionManager::rollback_transaction(self, connection_uuid).await
    }

    async fn get_transaction_state(&self, connection_uuid: &str) -> Option<TransactionState> {
        DatabaseConnectionManager::get_transaction_state(self, connection_uuid).await
    }
}

/// 连接池元数据（不包含实际的连接池）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolMetadata {
    /// 连接 UUID
    pub connection_uuid: String,
    /// 连接名称
    pub name: String,
    /// 数据库类型
    pub db_type: String,
    /// 创建时间戳
    pub created_at: u64,
    /// 最后使用时间戳
    pub last_used_at: u64,
    /// 最后错误信息
    pub last_error: Option<String>,
    /// 是否可用
    pub is_available: bool,
}

// ============================================================================
// Mock 连接池实现（用于测试）
// ============================================================================

/// Mock 连接池（用于测试）
pub struct MockPool {
    name: String,
    should_fail: bool,
}

impl MockPool {
    /// 创建新的 Mock 连接池
    pub fn new(name: String) -> Self {
        Self {
            name,
            should_fail: false,
        }
    }

    /// 创建会失败的 Mock 连接池
    pub fn failing(name: String) -> Self {
        Self {
            name,
            should_fail: true,
        }
    }
}

#[async_trait::async_trait]
impl DatabasePool for MockPool {
    async fn execute(&self, _sql: &str, _params: Vec<serde_json::Value>) -> Result<QueryResult, String> {
        if self.should_fail {
            Err("Mock pool is configured to fail".to_string())
        } else {
            Ok(QueryResult::with_rows(
                vec![serde_json::json!({"mock": true})],
                vec!["mock".to_string()],
            ))
        }
    }

    async fn test_connection(&self) -> Result<(), String> {
        if self.should_fail {
            Err("Mock pool connection test failed".to_string())
        } else {
            Ok(())
        }
    }

    async fn close(&self) {
        log::info!("[MockPool] 连接池已关闭: {}", self.name);
    }

    fn status_info(&self) -> String {
        format!("Mock pool: {} (failing: {})", self.name, self.should_fail)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = DatabaseConnectionManager::new();
        assert_eq!(manager.pool_count().await, 0);
        assert_eq!(manager.active_pool_count().await, 0);
    }

    #[tokio::test]
    async fn test_manager_default() {
        let manager = DatabaseConnectionManager::default();
        assert_eq!(manager.pool_count().await, 0);
    }

    #[tokio::test]
    async fn test_create_pool_success() {
        let manager = DatabaseConnectionManager::new();
        let pool = Arc::new(MockPool::new("test-pool".to_string()));

        let result = manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(manager.pool_count().await, 1);
    }

    #[tokio::test]
    async fn test_create_pool_duplicate() {
        let manager = DatabaseConnectionManager::new();
        let pool1 = Arc::new(MockPool::new("test-pool-1".to_string()));
        let pool2 = Arc::new(MockPool::new("test-pool-2".to_string()));

        let result1 = manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool1,
            )
            .await;
        assert!(result1.is_ok());

        let result2 = manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection 2".to_string(),
                "mysql".to_string(),
                pool2,
            )
            .await;
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("已存在"));
    }

    #[tokio::test]
    async fn test_get_pool_success() {
        let manager = DatabaseConnectionManager::new();
        let pool = Arc::new(MockPool::new("test-pool".to_string()));

        manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool,
            )
            .await
            .unwrap();

        let retrieved = manager.get_pool("conn-001").await;
        assert!(retrieved.is_some());

        let result = retrieved
            .unwrap()
            .execute("SELECT 1", vec![])
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[tokio::test]
    async fn test_get_pool_not_found() {
        let manager = DatabaseConnectionManager::new();

        let retrieved = manager.get_pool("non-existent").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_close_pool_success() {
        let manager = DatabaseConnectionManager::new();
        let pool = Arc::new(MockPool::new("test-pool".to_string()));

        manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool,
            )
            .await
            .unwrap();

        assert_eq!(manager.pool_count().await, 1);

        let result = manager.close_pool("conn-001").await;
        assert!(result.is_ok());
        assert_eq!(manager.pool_count().await, 0);
    }

    #[tokio::test]
    async fn test_close_pool_not_found() {
        let manager = DatabaseConnectionManager::new();

        let result = manager.close_pool("non-existent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[tokio::test]
    async fn test_begin_transaction_success() {
        let manager = DatabaseConnectionManager::new();

        let result = manager.begin_transaction("conn-001", None).await;
        assert!(result.is_ok());

        let state = manager.get_transaction_state("conn-001").await;
        assert_eq!(state, Some(TransactionState::Active));
    }

    #[tokio::test]
    async fn test_begin_transaction_duplicate() {
        let manager = DatabaseConnectionManager::new();

        manager.begin_transaction("conn-001", None).await.unwrap();

        let result = manager.begin_transaction("conn-001", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("已有活跃事务"));
    }

    #[tokio::test]
    async fn test_commit_transaction_success() {
        let manager = DatabaseConnectionManager::new();

        manager.begin_transaction("conn-001", None).await.unwrap();

        let result = manager.commit_transaction("conn-001").await;
        assert!(result.is_ok());

        let state = manager.get_transaction_state("conn-001").await;
        assert_eq!(state, Some(TransactionState::Committed));
    }

    #[tokio::test]
    async fn test_commit_transaction_no_active() {
        let manager = DatabaseConnectionManager::new();

        let result = manager.commit_transaction("conn-001").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("没有活跃事务"));
    }

    #[tokio::test]
    async fn test_rollback_transaction_success() {
        let manager = DatabaseConnectionManager::new();

        manager.begin_transaction("conn-001", None).await.unwrap();

        let result = manager.rollback_transaction("conn-001").await;
        assert!(result.is_ok());

        let state = manager.get_transaction_state("conn-001").await;
        assert_eq!(state, Some(TransactionState::RolledBack));
    }

    #[tokio::test]
    async fn test_rollback_transaction_no_active() {
        let manager = DatabaseConnectionManager::new();

        let result = manager.rollback_transaction("conn-001").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("没有活跃事务"));
    }

    #[tokio::test]
    async fn test_cleanup_transactions() {
        let manager = DatabaseConnectionManager::new();

        manager.begin_transaction("conn-001", None).await.unwrap();
        manager.begin_transaction("conn-002", None).await.unwrap();
        manager.begin_transaction("conn-003", None).await.unwrap();

        manager.commit_transaction("conn-001").await.unwrap();
        manager.rollback_transaction("conn-002").await.unwrap();

        manager.cleanup_transactions().await;

        assert_eq!(
            manager.get_transaction_state("conn-003").await,
            Some(TransactionState::Active)
        );
    }

    #[tokio::test]
    async fn test_get_pool_metadata() {
        let manager = DatabaseConnectionManager::new();
        let pool = Arc::new(MockPool::new("test-pool".to_string()));

        manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool,
            )
            .await
            .unwrap();

        let metadata = manager.get_pool_metadata("conn-001").await;
        assert!(metadata.is_some());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.connection_uuid, "conn-001");
        assert_eq!(metadata.name, "Test Connection");
        assert_eq!(metadata.db_type, "postgres");
        assert!(metadata.is_available);
    }

    #[tokio::test]
    async fn test_get_all_pools_metadata() {
        let manager = DatabaseConnectionManager::new();

        manager
            .create_pool(
                "conn-001".to_string(),
                "Connection 1".to_string(),
                "postgres".to_string(),
                Arc::new(MockPool::new("pool-1".to_string())),
            )
            .await
            .unwrap();

        manager
            .create_pool(
                "conn-002".to_string(),
                "Connection 2".to_string(),
                "mysql".to_string(),
                Arc::new(MockPool::new("pool-2".to_string())),
            )
            .await
            .unwrap();

        let all_metadata = manager.get_all_pools_metadata().await;
        assert_eq!(all_metadata.len(), 2);

        let uuids: Vec<&str> = all_metadata
            .iter()
            .map(|m| m.connection_uuid.as_str())
            .collect();
        assert!(uuids.contains(&"conn-001"));
        assert!(uuids.contains(&"conn-002"));
    }

    #[tokio::test]
    async fn test_clear() {
        let manager = DatabaseConnectionManager::new();

        manager
            .create_pool(
                "conn-001".to_string(),
                "Connection 1".to_string(),
                "postgres".to_string(),
                Arc::new(MockPool::new("pool-1".to_string())),
            )
            .await
            .unwrap();

        manager.begin_transaction("conn-001", None).await.unwrap();

        assert_eq!(manager.pool_count().await, 1);

        manager.clear().await;

        assert_eq!(manager.pool_count().await, 0);
        assert_eq!(manager.get_transaction_state("conn-001").await, None);
    }

    #[tokio::test]
    async fn test_set_pool_error() {
        let manager = DatabaseConnectionManager::new();
        let pool = Arc::new(MockPool::new("test-pool".to_string()));

        manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool,
            )
            .await
            .unwrap();

        manager
            .set_pool_error("conn-001", "Connection failed")
            .await;

        let metadata = manager.get_pool_metadata("conn-001").await.unwrap();
        assert_eq!(metadata.last_error, Some("Connection failed".to_string()));
        assert!(!metadata.is_available);
    }

    #[tokio::test]
    async fn test_clear_pool_error() {
        let manager = DatabaseConnectionManager::new();
        let pool = Arc::new(MockPool::new("test-pool".to_string()));

        manager
            .create_pool(
                "conn-001".to_string(),
                "Test Connection".to_string(),
                "postgres".to_string(),
                pool,
            )
            .await
            .unwrap();

        manager
            .set_pool_error("conn-001", "Connection failed")
            .await;

        manager.clear_pool_error("conn-001").await;

        let metadata = manager.get_pool_metadata("conn-001").await.unwrap();
        assert_eq!(metadata.last_error, None);
        assert!(metadata.is_available);
    }

    #[tokio::test]
    async fn test_active_pool_count() {
        let manager = DatabaseConnectionManager::new();

        manager
            .create_pool(
                "conn-001".to_string(),
                "Connection 1".to_string(),
                "postgres".to_string(),
                Arc::new(MockPool::new("pool-1".to_string())),
            )
            .await
            .unwrap();

        manager
            .create_pool(
                "conn-002".to_string(),
                "Connection 2".to_string(),
                "mysql".to_string(),
                Arc::new(MockPool::new("pool-2".to_string())),
            )
            .await
            .unwrap();

        assert_eq!(manager.pool_count().await, 2);
        assert_eq!(manager.active_pool_count().await, 2);

        manager
            .set_pool_error("conn-001", "Failed")
            .await;

        assert_eq!(manager.pool_count().await, 2);
        assert_eq!(manager.active_pool_count().await, 1);
    }

    #[test]
    fn test_active_pool_creation() {
        let pool = ActivePool::new(
            "conn-001".to_string(),
            "Test Pool".to_string(),
            "postgres".to_string(),
        );

        assert_eq!(pool.connection_uuid, "conn-001");
        assert_eq!(pool.name, "Test Pool");
        assert_eq!(pool.db_type, "postgres");
        assert!(pool.last_error.is_none());
        assert!(!pool.is_available());
    }

    #[test]
    fn test_active_pool_set_and_get() {
        let mut pool = ActivePool::new(
            "conn-001".to_string(),
            "Test Pool".to_string(),
            "postgres".to_string(),
        );

        assert!(!pool.is_available());

        pool.set_pool(Arc::new(MockPool::new("test".to_string())));

        assert!(pool.is_available());
        assert!(pool.get_pool().is_some());
    }

    #[test]
    fn test_active_pool_error_handling() {
        let mut pool = ActivePool::new(
            "conn-001".to_string(),
            "Test Pool".to_string(),
            "postgres".to_string(),
        );

        pool.set_pool(Arc::new(MockPool::new("test".to_string())));
        assert!(pool.is_available());

        pool.set_error("Connection failed".to_string());
        assert_eq!(pool.last_error, Some("Connection failed".to_string()));
        assert!(!pool.is_available());

        pool.clear_error();
        assert!(pool.last_error.is_none());
        assert!(pool.is_available());
    }

    #[test]
    fn test_transaction_info_creation() {
        let info = TransactionInfo::new("conn-001".to_string(), Some("READ COMMITTED".to_string()));

        assert_eq!(info.connection_uuid, "conn-001");
        assert_eq!(info.state, TransactionState::Active);
        assert_eq!(info.isolation_level, Some("READ COMMITTED".to_string()));
    }

    #[test]
    fn test_pool_metadata_serialization() {
        let metadata = PoolMetadata {
            connection_uuid: "conn-001".to_string(),
            name: "Test Pool".to_string(),
            db_type: "postgres".to_string(),
            created_at: 1234567890,
            last_used_at: 1234567899,
            last_error: Some("Error".to_string()),
            is_available: false,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PoolMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.connection_uuid, "conn-001");
        assert_eq!(deserialized.name, "Test Pool");
        assert_eq!(deserialized.db_type, "postgres");
    }

    #[tokio::test]
    async fn test_mock_pool_execute() {
        let pool = MockPool::new("test".to_string());

        let result = pool.execute("SELECT 1", vec![]).await.unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.row_count, 1);

        let failing_pool = MockPool::failing("test".to_string());
        let result = failing_pool.execute("SELECT 1", vec![]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_pool_test_connection() {
        let pool = MockPool::new("test".to_string());
        assert!(pool.test_connection().await.is_ok());

        let failing_pool = MockPool::failing("test".to_string());
        assert!(failing_pool.test_connection().await.is_err());
    }

    #[test]
    fn test_mock_pool_status_info() {
        let pool = MockPool::new("test".to_string());
        assert!(pool.status_info().contains("test"));
        assert!(pool.status_info().contains("failing: false"));

        let failing_pool = MockPool::failing("test".to_string());
        assert!(failing_pool.status_info().contains("failing: true"));
    }
}
