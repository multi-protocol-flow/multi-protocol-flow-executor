// 数据驱动执行会话管理器
//!
//! 确保同一时刻只有一个数据驱动执行在运行。

use std::sync::{Arc, Mutex};

/// 数据驱动执行会话管理器
///
/// 使用 `Arc<Mutex<Option<String>>>` 追踪当前运行的执行会话，
/// 确保同一时刻只有一个数据驱动执行在运行。
pub struct DatasetSessionManager {
    /// 当前运行的会话 ID（None 表示空闲）
    active_session: Arc<Mutex<Option<String>>>,
}

impl DatasetSessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        Self {
            active_session: Arc::new(Mutex::new(None)),
        }
    }

    /// 尝试开始一个新的执行会话
    ///
    /// # 返回
    /// - `Ok(())`: Success获取会话
    /// - `Err(msg)`: 已有执行在运行
    pub fn start_session(&self, session_id: String) -> Result<(), String> {
        let mut guard = self
            .active_session
            .lock()
            .map_err(|e| format!("failed to acquire session lock: {}", e))?;
        if guard.is_some() {
            return Err("data-driven execution already running".to_string());
        }
        *guard = Some(session_id);
        Ok(())
    }

    /// 结束当前执行会话
    ///
    /// # 返回
    /// - `Ok(())`: Success释放会话
    /// - `Err(msg)`: 没有运行中的会话
    pub fn end_session(&self) -> Result<(), String> {
        let mut guard = self
            .active_session
            .lock()
            .map_err(|e| format!("failed to acquire session lock: {}", e))?;
        if guard.is_none() {
            return Err("no running data-driven execution session".to_string());
        }
        *guard = None;
        Ok(())
    }

    /// 查询当前是否正在执行
    pub fn is_running(&self) -> bool {
        self.active_session
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    /// 获取当前会话 ID
    pub fn current_session_id(&self) -> Option<String> {
        self.active_session.lock().ok().and_then(|g| g.clone())
    }
}

impl Default for DatasetSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_start_and_end() {
        let manager = DatasetSessionManager::new();
        assert!(!manager.is_running());
        assert!(manager.current_session_id().is_none());

        // 开始会话
        manager
            .start_session("session-001".to_string())
            .unwrap_or_else(|e| panic!("开始会话应Success: {e}"));
        assert!(manager.is_running());
        assert_eq!(
            manager.current_session_id(),
            Some("session-001".to_string())
        );

        // 结束会话
        manager
            .end_session()
            .unwrap_or_else(|e| panic!("结束会话应Success: {e}"));
        assert!(!manager.is_running());
        assert!(manager.current_session_id().is_none());
    }

    #[test]
    fn test_double_start_fails() {
        let manager = DatasetSessionManager::new();
        manager
            .start_session("session-001".to_string())
            .unwrap_or_else(|e| panic!("第一次开始应Success: {e}"));

        let result = manager.start_session("session-002".to_string());
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("already running"));
    }

    #[test]
    fn test_end_without_start_fails() {
        let manager = DatasetSessionManager::new();
        let result = manager.end_session();
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("no running"));
    }

    #[test]
    fn test_session_reuse_after_end() {
        let manager = DatasetSessionManager::new();

        manager
            .start_session("session-001".to_string())
            .unwrap_or_else(|e| panic!("第一次开始应Success: {e}"));
        manager
            .end_session()
            .unwrap_or_else(|e| panic!("结束应Success: {e}"));

        // 结束后可以重新开始
        manager
            .start_session("session-002".to_string())
            .unwrap_or_else(|e| panic!("重新开始应Success: {e}"));
        assert_eq!(
            manager.current_session_id(),
            Some("session-002".to_string())
        );
    }
}
