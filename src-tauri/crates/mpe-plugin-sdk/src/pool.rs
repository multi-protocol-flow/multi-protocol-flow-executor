//! Keyed pool of long-lived plugin connections.
//!
//! Maps a `key` (e.g. a target host or connection name) to a shared
//! `Arc<dyn Any + Send + Sync>` resource — say an HTTP client or a database
//! handle — so a plugin process can reuse connections across executions.
//! The plugin downcasts to its concrete type with [`ConnectionPool::get`].
//!
//! The pool is process-local and lives for the plugin's lifetime; the host
//! never sees it. Concurrency note: `get_or_insert` may run the factory more
//! than once under contention (read-then-write is not atomic); the last
//! insertion wins, which is benign for idempotent factories.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A `key → Arc<dyn Any>` pool, shared across executions in one plugin
/// process.
#[derive(Default)]
pub struct ConnectionPool {
    inner: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl ConnectionPool {
    /// Creates an empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the stored connection for `key`, downcast to `T`. `None` when
    /// the key is absent or holds a different type.
    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        let guard = self.inner.read().ok()?;
        guard.get(key)?.clone().downcast::<T>().ok()
    }

    /// Returns the stored connection for `key`, creating it with `factory`
    /// on first use. Keys are caller-namespaced: mixing types under one key
    /// is a plugin bug and the last type wins.
    pub fn get_or_insert<T: Any + Send + Sync>(
        &self,
        key: impl Into<String>,
        factory: impl FnOnce() -> T,
    ) -> Arc<T> {
        let key = key.into();
        if let Some(existing) = self.get::<T>(&key) {
            return existing;
        }
        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = guard.get(&key) {
            if let Ok(existing) = existing.clone().downcast::<T>() {
                return existing;
            }
        }
        let created: Arc<T> = Arc::new(factory());
        let erased: Arc<dyn Any + Send + Sync> = created.clone();
        guard.insert(key, erased);
        created
    }

    /// Removes and drops the connection for `key`; `true` when it existed.
    pub fn remove(&self, key: &str) -> bool {
        self.inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(key)
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_or_insert_creates_once_and_reuses() -> Result<(), Box<dyn std::error::Error>> {
        let pool = ConnectionPool::new();
        let first = pool.get_or_insert("http", || 42u32);
        assert_eq!(*first, 42);
        let second = pool.get_or_insert("http", || 99u32);
        assert_eq!(*second, 42, "existing value must be reused, not rebuilt");
        assert!(Arc::ptr_eq(&first, &second));
        Ok(())
    }

    #[test]
    fn get_returns_none_for_missing_or_wrong_type() -> Result<(), Box<dyn std::error::Error>> {
        let pool = ConnectionPool::new();
        assert!(pool.get::<u32>("missing").is_none());
        pool.get_or_insert("k", || 42u32);
        assert!(
            pool.get::<String>("k").is_none(),
            "wrong type must not downcast"
        );
        assert_eq!(
            *pool
                .get::<u32>("k")
                .ok_or_else(|| "expected u32".to_string())?,
            42
        );
        Ok(())
    }

    #[test]
    fn remove_drops_the_entry() -> Result<(), Box<dyn std::error::Error>> {
        let pool = ConnectionPool::new();
        pool.get_or_insert("k", || 42u32);
        assert!(pool.remove("k"));
        assert!(!pool.remove("k"), "second remove must report absence");
        assert!(pool.get::<u32>("k").is_none());
        Ok(())
    }
}
