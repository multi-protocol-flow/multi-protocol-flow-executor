// WebSocket 连接管理器模块
//
// 管理 WebSocket 连接池，提供连接状态查询功能。
// Wave 1: 基础数据结构，不含实际连接逻辑

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio_tungstenite::tungstenite::protocol::Message as WebSocketMessage;

/// 连接状态
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    /// 正在连接
    #[default]
    Connecting,
    /// 已连接
    Connected,
    /// 正在关闭
    Closing,
    /// 已关闭
    Closed,
}

/// 活跃连接
///
/// 存储单个 WebSocket 连接的状态和元数据。
#[derive(Debug, Clone)]
pub struct ActiveConnection {
    /// 节点 UUID（连接的唯一标识）
    pub uuid: String,
    /// 节点名称（作为连接的可读标识符）
    pub name: String,
    /// 连接状态
    pub status: ConnectionStatus,
    /// 连接创建时间
    pub created_at: DateTime<Utc>,
    /// WebSocket 服务端 URL
    pub url: String,
    /// 消息缓冲区（用于暂存待处理消息）
    pub message_buffer: VecDeque<WebSocketMessage>,
}

impl ActiveConnection {
    /// 创建新的活跃连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    /// * `name` - 节点名称
    /// * `url` - WebSocket URL
    ///
    /// # 返回
    ///
    /// 初始化的活跃连接，状态为 Connecting
    pub fn new(uuid: String, name: String, url: String) -> Self {
        Self {
            uuid,
            name,
            status: ConnectionStatus::Connecting,
            created_at: Utc::now(),
            url,
            message_buffer: VecDeque::new(),
        }
    }

    /// 更新连接状态
    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    /// 添加消息到缓冲区
    pub fn push_message(&mut self, message: WebSocketMessage) {
        self.message_buffer.push_back(message);
    }

    /// 从缓冲区取出消息
    pub fn pop_message(&mut self) -> Option<WebSocketMessage> {
        self.message_buffer.pop_front()
    }

    /// 获取缓冲区消息数量
    pub fn buffer_size(&self) -> usize {
        self.message_buffer.len()
    }
}

/// WebSocket 连接管理器
///
/// 管理所有活跃的 WebSocket 连接，提供线程安全的连接池操作。
/// 使用 Arc<Mutex<>> 实现跨线程共享。
#[derive(Debug, Clone)]
pub struct WebSocketConnectionManager {
    /// 连接池：UUID -> ActiveConnection
    connections: Arc<Mutex<HashMap<String, ActiveConnection>>>,
}

impl WebSocketConnectionManager {
    /// 创建新的连接管理器
    ///
    /// # 返回
    ///
    /// 初始化的连接管理器，包含空的连接池
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 根据 UUID 获取连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 连接的克隆，如果不存在则返回 None
    pub fn get_connection(&self, uuid: &str) -> Option<ActiveConnection> {
        let connections = self.connections.lock().unwrap();
        connections.get(uuid).cloned()
    }

    /// 获取所有连接
    ///
    /// # 返回
    ///
    /// 所有活跃连接的列表
    pub fn get_all_connections(&self) -> Vec<ActiveConnection> {
        let connections = self.connections.lock().unwrap();
        connections.values().cloned().collect()
    }

    /// 添加或更新连接
    ///
    /// # 参数
    ///
    /// * `connection` - 要添加的连接
    ///
    /// # 返回
    ///
    /// 如果已存在同名 UUID 的连接，返回旧连接
    pub fn upsert_connection(&self, connection: ActiveConnection) -> Option<ActiveConnection> {
        let mut connections = self.connections.lock().unwrap();
        connections.insert(connection.uuid.clone(), connection)
    }

    /// 移除连接
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    ///
    /// # 返回
    ///
    /// 被移除的连接，如果不存在则返回 None
    pub fn remove_connection(&self, uuid: &str) -> Option<ActiveConnection> {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(uuid)
    }

    /// 更新连接状态
    ///
    /// # 参数
    ///
    /// * `uuid` - 节点 UUID
    /// * `status` - 新状态
    ///
    /// # 返回
    ///
    /// 操作是否成功（连接是否存在）
    pub fn update_status(&self, uuid: &str, status: ConnectionStatus) -> bool {
        let mut connections = self.connections.lock().unwrap();
        if let Some(conn) = connections.get_mut(uuid) {
            conn.set_status(status);
            true
        } else {
            false
        }
    }

    /// 获取连接数量
    pub fn connection_count(&self) -> usize {
        let connections = self.connections.lock().unwrap();
        connections.len()
    }

    /// 清空所有连接
    pub fn clear(&self) {
        let mut connections = self.connections.lock().unwrap();
        connections.clear();
    }
}

impl Default for WebSocketConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_connection(uuid: &str, name: &str, url: &str) -> ActiveConnection {
        ActiveConnection::new(uuid.to_string(), name.to_string(), url.to_string())
    }

    #[test]
    fn test_connection_status_default() {
        let status = ConnectionStatus::default();
        assert_eq!(status, ConnectionStatus::Connecting);
    }

    #[test]
    fn test_connection_status_serialization() {
        let status = ConnectionStatus::Connected;
        let json = serde_json::to_string(&status).expect("序列化失败");
        assert_eq!(json, r#""connected""#);

        let deserialized: ConnectionStatus = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(deserialized, ConnectionStatus::Connected);
    }

    #[test]
    fn test_active_connection_creation() {
        let conn = create_test_connection("uuid-1", "测试连接", "wss://example.com/ws");

        assert_eq!(conn.uuid, "uuid-1");
        assert_eq!(conn.name, "测试连接");
        assert_eq!(conn.url, "wss://example.com/ws");
        assert_eq!(conn.status, ConnectionStatus::Connecting);
        assert!(conn.buffer_size() == 0);
    }

    #[test]
    fn test_active_connection_set_status() {
        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");

        conn.set_status(ConnectionStatus::Connected);
        assert_eq!(conn.status, ConnectionStatus::Connected);

        conn.set_status(ConnectionStatus::Closing);
        assert_eq!(conn.status, ConnectionStatus::Closing);
    }

    #[test]
    fn test_active_connection_message_buffer() {
        use tokio_tungstenite::tungstenite::{Bytes, Utf8Bytes};

        let mut conn = create_test_connection("uuid-1", "测试", "wss://example.com/ws");

        // 添加消息
        conn.push_message(WebSocketMessage::Text(Utf8Bytes::from("hello")));
        conn.push_message(WebSocketMessage::Binary(Bytes::from(vec![1, 2, 3])));
        assert_eq!(conn.buffer_size(), 2);

        // 取出消息（FIFO）
        let msg1 = conn.pop_message();
        assert!(matches!(msg1, Some(WebSocketMessage::Text(ref t)) if t.as_str() == "hello"));
        assert_eq!(conn.buffer_size(), 1);

        let msg2 = conn.pop_message();
        assert!(matches!(msg2, Some(WebSocketMessage::Binary(ref b)) if b.as_ref() == &[1, 2, 3]));
        assert_eq!(conn.buffer_size(), 0);

        // 空缓冲区
        let msg3 = conn.pop_message();
        assert!(msg3.is_none());
    }

    #[test]
    fn test_manager_creation() {
        let manager = WebSocketConnectionManager::new();
        assert_eq!(manager.connection_count(), 0);
        assert!(manager.get_all_connections().is_empty());
    }

    #[test]
    fn test_manager_default() {
        let manager = WebSocketConnectionManager::default();
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn test_manager_upsert_connection() {
        let manager = WebSocketConnectionManager::new();
        let conn1 = create_test_connection("uuid-1", "连接1", "wss://example.com/ws1");

        // 添加新连接
        let result = manager.upsert_connection(conn1);
        assert!(result.is_none());
        assert_eq!(manager.connection_count(), 1);

        // 更新已存在的连接
        let conn2 = create_test_connection("uuid-1", "连接1更新", "wss://example.com/ws2");
        let old = manager.upsert_connection(conn2);
        assert!(old.is_some());
        assert_eq!(old.unwrap().name, "连接1");
        assert_eq!(manager.connection_count(), 1);

        // 验证更新成功
        let current = manager.get_connection("uuid-1").unwrap();
        assert_eq!(current.name, "连接1更新");
        assert_eq!(current.url, "wss://example.com/ws2");
    }

    #[test]
    fn test_manager_get_connection() {
        let manager = WebSocketConnectionManager::new();
        let conn = create_test_connection("uuid-1", "连接1", "wss://example.com/ws");

        manager.upsert_connection(conn);

        // 存在的连接
        let found = manager.get_connection("uuid-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "连接1");

        // 不存在的连接
        let not_found = manager.get_connection("uuid-999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_manager_get_all_connections() {
        let manager = WebSocketConnectionManager::new();

        manager.upsert_connection(create_test_connection("uuid-1", "连接1", "wss://a.com"));
        manager.upsert_connection(create_test_connection("uuid-2", "连接2", "wss://b.com"));
        manager.upsert_connection(create_test_connection("uuid-3", "连接3", "wss://c.com"));

        let all = manager.get_all_connections();
        assert_eq!(all.len(), 3);

        let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"连接1"));
        assert!(names.contains(&"连接2"));
        assert!(names.contains(&"连接3"));
    }

    #[test]
    fn test_manager_remove_connection() {
        let manager = WebSocketConnectionManager::new();
        manager.upsert_connection(create_test_connection("uuid-1", "连接1", "wss://a.com"));

        // 移除存在的连接
        let removed = manager.remove_connection("uuid-1");
        assert!(removed.is_some());
        assert_eq!(manager.connection_count(), 0);

        // 移除不存在的连接
        let not_removed = manager.remove_connection("uuid-999");
        assert!(not_removed.is_none());
    }

    #[test]
    fn test_manager_update_status() {
        let manager = WebSocketConnectionManager::new();
        manager.upsert_connection(create_test_connection("uuid-1", "连接1", "wss://a.com"));

        // 更新存在的连接状态
        let success = manager.update_status("uuid-1", ConnectionStatus::Connected);
        assert!(success);

        let conn = manager.get_connection("uuid-1").unwrap();
        assert_eq!(conn.status, ConnectionStatus::Connected);

        // 更新不存在的连接
        let fail = manager.update_status("uuid-999", ConnectionStatus::Connected);
        assert!(!fail);
    }

    #[test]
    fn test_manager_clear() {
        let manager = WebSocketConnectionManager::new();
        manager.upsert_connection(create_test_connection("uuid-1", "连接1", "wss://a.com"));
        manager.upsert_connection(create_test_connection("uuid-2", "连接2", "wss://b.com"));

        assert_eq!(manager.connection_count(), 2);

        manager.clear();

        assert_eq!(manager.connection_count(), 0);
        assert!(manager.get_all_connections().is_empty());
    }

    #[test]
    fn test_manager_clone() {
        let manager1 = WebSocketConnectionManager::new();
        manager1.upsert_connection(create_test_connection("uuid-1", "连接1", "wss://a.com"));

        // 克隆管理器（共享同一内部状态）
        let manager2 = manager1.clone();

        // 通过 manager2 修改
        manager2.upsert_connection(create_test_connection("uuid-2", "连接2", "wss://b.com"));

        // manager1 应该看到变化（Arc 共享）
        assert_eq!(manager1.connection_count(), 2);
        assert_eq!(manager2.connection_count(), 2);
    }
}
