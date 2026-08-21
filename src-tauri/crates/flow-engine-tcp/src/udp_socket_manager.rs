//! UDP Socket 管理器模块
//!
//! 管理 UDP Socket 池，实现 `UdpProvider` trait。
//! 使用 `tokio::net::UdpSocket` 实现无连接的 UDP 通信。
//! 线程安全，使用 `Arc<Mutex<>>` 保护 Socket 池。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use flow_engine_core::traits::{UdpProvider, UdpSocketInfo};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

// ---------------------------------------------------------------------------
// Socket 状态
// ---------------------------------------------------------------------------

/// UDP Socket 状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdpSocketStatus {
    /// 正在绑定
    Binding,
    /// 已绑定
    Bound,
    /// 已关闭
    Closed,
    /// Error状态
    Error,
}

impl UdpSocketStatus {
    /// 转换为字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            UdpSocketStatus::Binding => "Binding",
            UdpSocketStatus::Bound => "Bound",
            UdpSocketStatus::Closed => "Closed",
            UdpSocketStatus::Error => "Error",
        }
    }
}

impl std::fmt::Display for UdpSocketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// 活跃 UDP Socket
// ---------------------------------------------------------------------------

/// 活跃的 UDP Socket
///
/// 存储单个 UDP Socket 的状态、元数据和实际的 socket。
pub struct ActiveUdpSocket {
    /// Socket 唯一标识（节点 UUID）
    pub uuid: String,
    /// Socket 名称
    pub name: String,
    /// Socket 状态
    pub status: UdpSocketStatus,
    /// 本地绑定地址
    pub local_addr: String,
    /// 本地绑定端口
    pub local_port: u16,
    /// 绑定时间（Unix 时间戳，毫秒）
    pub bound_at: u64,
    /// 最近一次Error信息
    pub last_error: Option<String>,

    /// UDP Socket（通过 Arc<tokio::sync::Mutex> 实现线程安全共享）
    socket: Arc<tokio::sync::Mutex<UdpSocket>>,
}

impl std::fmt::Debug for ActiveUdpSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveUdpSocket")
            .field("uuid", &self.uuid)
            .field("name", &self.name)
            .field("status", &self.status)
            .field("local_addr", &self.local_addr)
            .field("local_port", &self.local_port)
            .field("bound_at", &self.bound_at)
            .field("last_error", &self.last_error)
            .finish()
    }
}

impl ActiveUdpSocket {
    /// 创建新的活跃 Socket（绑定Success后调用）
    pub fn new(
        uuid: String,
        name: String,
        local_addr: String,
        local_port: u16,
        socket: UdpSocket,
    ) -> Self {
        let bound_at = flow_engine_core::utils::unix_timestamp_ms();

        Self {
            uuid,
            name,
            status: UdpSocketStatus::Bound,
            local_addr,
            local_port,
            bound_at,
            last_error: None,
            socket: Arc::new(tokio::sync::Mutex::new(socket)),
        }
    }

    /// 设置Error状态
    pub fn set_error(&mut self, error: String) {
        self.status = UdpSocketStatus::Error;
        self.last_error = Some(error);
    }

    /// 获取 Socket 的 Arc 引用
    pub fn get_socket(&self) -> Arc<tokio::sync::Mutex<UdpSocket>> {
        self.socket.clone()
    }

    /// 检查是否处于已绑定状态
    pub fn is_bound(&self) -> bool {
        self.status == UdpSocketStatus::Bound
    }

    /// 转换为 `UdpSocketInfo`
    pub fn to_socket_info(&self) -> UdpSocketInfo {
        UdpSocketInfo {
            uuid: self.uuid.clone(),
            name: self.name.clone(),
            status: self.status.to_string(),
            local_addr: self.local_addr.clone(),
            local_port: self.local_port,
            bound_at: self.bound_at,
            last_error: self.last_error.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// UDP Socket 管理器
// ---------------------------------------------------------------------------

/// UDP Socket 管理器
///
/// 管理所有活跃的 UDP Socket，提供线程安全的 Socket 池操作。
/// 使用 `Arc<Mutex<>>` 实现跨线程共享。
#[derive(Debug, Clone)]
pub struct UdpSocketManager {
    /// Socket 池：UUID → `ActiveUdpSocket`
    sockets: Arc<Mutex<HashMap<String, ActiveUdpSocket>>>,
}

impl UdpSocketManager {
    /// 创建新的 Socket 管理器
    pub fn new() -> Self {
        Self {
            sockets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 更新 Socket Error状态
    fn update_socket_error(&self, uuid: &str, error: &str) {
        if let Ok(mut sockets) = self.sockets.lock() {
            if let Some(sock) = sockets.get_mut(uuid) {
                sock.set_error(error.to_string());
            }
        }
    }
}

impl Default for UdpSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UdpSocketManager {
    /// 带完整配置的 UDP Socket 绑定
    ///
    /// 使用 `socket2` 创建 socket，支持 `reuse_addr/broadcast/multicast` 等选项。
    /// 绑定Success后存入 Socket 池。
    pub async fn bind_with_options(
        &self,
        uuid: String,
        name: String,
        local_addr: String,
        local_port: u16,
        _read_timeout_ms: u64,
        reuse_addr: bool,
        broadcast: bool,
        multicast_group: Option<String>,
        multicast_ttl: Option<u32>,
        multicast_loop: Option<bool>,
        multicast_interface: Option<String>,
    ) -> Result<(), String> {
        // 1. 解析绑定地址
        let addr: std::net::SocketAddr =
            format!("{}:{}", local_addr, local_port)
                .parse()
                .map_err(|e| {
                    format!(
                        "Address parsing failed ({}:{}): {}",
                        local_addr, local_port, e
                    )
                })?;

        // 2. 使用 socket2 创建 socket（支持 pre-bind 选项）
        let domain = if addr.is_ipv6() {
            Domain::IPV6
        } else {
            Domain::IPV4
        };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| format!("Failed to create UDP socket: {}", e))?;

        // 3. reuse_addr 在 bind 之前设置
        if reuse_addr {
            socket
                .set_reuse_address(true)
                .map_err(|e| format!("Failed to set address reuse: {}", e))?;
        }

        // 4. 绑定
        socket
            .bind(&addr.into())
            .map_err(|e| format!("UDP bind failed ({}): {}", addr, e))?;

        // 5. 转换为 std::net::UdpSocket
        let std_socket: std::net::UdpSocket = socket.into();

        // 6. 设置 broadcast（bind 之后）
        if broadcast {
            std_socket
                .set_broadcast(true)
                .map_err(|e| format!("Failed to set broadcast: {}", e))?;
        }

        // 7. 设置多播选项（bind 之后）
        if let Some(ref group_str) = multicast_group {
            let group: std::net::Ipv4Addr = group_str
                .parse()
                .map_err(|e| format!("Invalid multicast group address '{}': {}", group_str, e))?;
            let iface: std::net::Ipv4Addr = match &multicast_interface {
                Some(ref iface_str) => iface_str
                    .parse()
                    .map_err(|e| format!("Invalid interface address '{}': {}", iface_str, e))?,
                None => std::net::Ipv4Addr::UNSPECIFIED,
            };
            std_socket
                .join_multicast_v4(&group, &iface)
                .map_err(|e| format!("Failed to join multicast group ({}): {}", group_str, e))?;
        }
        if let Some(ttl) = multicast_ttl {
            std_socket
                .set_multicast_ttl_v4(ttl)
                .map_err(|e| format!("Failed to set multicast TTL: {}", e))?;
        }
        if let Some(loopback) = multicast_loop {
            std_socket
                .set_multicast_loop_v4(loopback)
                .map_err(|e| format!("Failed to set multicast loopback: {}", e))?;
        }
        std_socket
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set nonblocking mode: {}", e))?;

        // 8. 转换为 tokio::net::UdpSocket
        let socket = UdpSocket::from_std(std_socket)
            .map_err(|e| format!("Failed to convert to tokio UdpSocket: {}", e))?;
        // 9. 获取实际绑定的地址和端口（端口 0 时自动分配）
        let actual_local_addr = socket
            .local_addr()
            .map(|sa| sa.ip().to_string())
            .unwrap_or(local_addr.clone());
        let actual_local_port = socket
            .local_addr()
            .map(|sa| sa.port())
            .unwrap_or(local_port);

        // 10. 创建 ActiveUdpSocket 并存入池
        let active_socket = ActiveUdpSocket::new(
            uuid.clone(),
            name,
            actual_local_addr,
            actual_local_port,
            socket,
        );

        if let Ok(mut sockets) = self.sockets.lock() {
            sockets.insert(uuid.clone(), active_socket);
        }

        log::info!(
            "[UDP] Bind successful: uuid={}, addr={}, reuse_addr={}, broadcast={}, multicast_group={:?}",
            uuid,
            addr,
            reuse_addr,
            broadcast,
            multicast_group,
        );
        Ok(())
    }
}

#[async_trait]
impl UdpProvider for UdpSocketManager {
    /// 绑定 UDP Socket 到指定地址和端口（使用默认配置）
    ///
    /// 通过 `bind_with_options` 实现，默认 `reuse_addr=true，其余选项关闭`。
    async fn bind(
        &self,
        uuid: String,
        name: String,
        local_addr: String,
        local_port: u16,
        read_timeout_ms: u64,
    ) -> Result<(), String> {
        self.bind_with_options(
            uuid,
            name,
            local_addr,
            local_port,
            read_timeout_ms,
            true,  // reuse_addr 默认开启
            false, // broadcast
            None,  // multicast_group
            None,  // multicast_ttl
            None,  // multicast_loop
            None,  // multicast_interface
        )
        .await
    }

    /// 发送数据到指定目标地址
    ///
    /// 查找 Socket 池中的 Socket，通过它发送数据到目标地址和端口。
    async fn send_to(
        &self,
        uuid: &str,
        data: Vec<u8>,
        target_addr: String,
        target_port: u16,
        send_timeout_ms: u64,
    ) -> Result<(), String> {
        // 1. 获取 Socket 引用（克隆后释放锁）
        let socket_arc = {
            let sockets = self
                .sockets
                .lock()
                .map_err(|e| format!("Failed to acquire socket lock: {}", e))?;
            let sock = sockets
                .get(uuid)
                .ok_or_else(|| format!("UDP socket '{}' not found", uuid))?;

            if !sock.is_bound() {
                return Err(format!("UDP socket '{}' is not in bound state", uuid));
            }

            sock.get_socket()
        }; // 锁在此释放

        // 2. 带超时发送数据
        let target = format!("{}:{}", target_addr, target_port);
        let send_timeout = Duration::from_millis(send_timeout_ms.clamp(100, 120000));
        let sock = socket_arc.lock().await;

        let send_result = timeout(send_timeout, sock.send_to(&data, &target)).await;

        match send_result {
            Ok(Ok(bytes_sent)) => {
                log::debug!(
                    "[UDP] Send successful: uuid={}, target={}, bytes={}/{}",
                    uuid,
                    target,
                    bytes_sent,
                    data.len()
                );
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("UDP send failed ({} → {}): {}", uuid, target, e);
                self.update_socket_error(uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                let error_msg = format!(
                    "UDP send timeout ({}ms): uuid={}, target={}",
                    send_timeout_ms, uuid, target
                );
                self.update_socket_error(uuid, &error_msg);
                Err(error_msg)
            }
        }
    }

    /// 接收数据
    ///
    /// 查找 Socket 池中的 Socket，从它接收数据。
    /// 返回 (数据, 来源地址, 来源端口)，超时时返回 Ok(None)。
    async fn recv_from(
        &self,
        uuid: &str,
        receive_timeout_ms: u64,
    ) -> Result<Option<(Vec<u8>, String, u16)>, String> {
        // 1. 获取 Socket 引用（克隆后释放锁）
        let socket_arc = {
            let sockets = self
                .sockets
                .lock()
                .map_err(|e| format!("Failed to acquire socket lock: {}", e))?;
            let sock = sockets
                .get(uuid)
                .ok_or_else(|| format!("UDP socket '{}' not found", uuid))?;

            if !sock.is_bound() {
                return Err(format!("UDP socket '{}' is not in bound state", uuid));
            }

            sock.get_socket()
        }; // 锁在此释放

        // 2. 带超时接收数据
        let recv_timeout = Duration::from_millis(receive_timeout_ms.clamp(100, 120000));
        let sock = socket_arc.lock().await;

        // 使用 64KB 缓冲区（UDP 最大数据报大小）
        let mut buf = vec![0u8; 65535];

        let recv_result = timeout(recv_timeout, sock.recv_from(&mut buf)).await;

        match recv_result {
            Ok(Ok((n, addr))) => {
                // Success接收 n 字节
                buf.truncate(n);
                let sender_addr = addr.ip().to_string();
                let sender_port = addr.port();
                log::debug!(
                    "[UDP] Receive successful: uuid={}, from={}:{}, bytes={}",
                    uuid,
                    sender_addr,
                    sender_port,
                    n
                );
                Ok(Some((buf, sender_addr, sender_port)))
            }
            Ok(Err(e)) => {
                let error_msg = format!("UDP receive failed ({}): {}", uuid, e);
                self.update_socket_error(uuid, &error_msg);
                Err(error_msg)
            }
            Err(_) => {
                // 超时，返回 None（表示在超时时间内没有数据可读）
                log::debug!(
                    "[UDP] Receive timeout ({}ms): uuid={}",
                    receive_timeout_ms,
                    uuid
                );
                Ok(None)
            }
        }
    }

    /// 关闭 Socket
    ///
    /// 从 Socket 池中移除，UdpSocket 在 drop 时自动关闭。
    async fn close(&self, uuid: &str) -> Result<(), String> {
        // 从池中移除（drop 时自动关闭 socket）
        {
            let mut sockets = self
                .sockets
                .lock()
                .map_err(|e| format!("获取 Socket 锁Failed: {}", e))?;
            sockets
                .remove(uuid)
                .ok_or_else(|| format!("UDP socket '{}' not found", uuid))?;
        }

        log::info!("[UDP] Socket closed: uuid={}", uuid);
        Ok(())
    }

    /// 获取 Socket 状态
    ///
    /// 返回状态的字符串表示，如果 Socket 不存在则返回 None。
    fn get_status(&self, uuid: &str) -> Option<String> {
        let sockets = self.sockets.lock().ok()?;
        sockets.get(uuid).map(|s| s.status.to_string())
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试管理器创建
    #[test]
    fn test_udp_socket_manager_new() {
        let manager = UdpSocketManager::new();
        assert!(manager.get_status("non-existent").is_none());
    }

    /// 测试管理器 Default trait
    #[test]
    fn test_udp_socket_manager_default() {
        let manager = UdpSocketManager::default();
        assert!(manager.get_status("non-existent").is_none());
    }

    /// 测试管理器 Clone
    #[test]
    fn test_udp_socket_manager_clone() {
        let manager = UdpSocketManager::new();
        let cloned = manager.clone();
        assert!(cloned.get_status("non-existent").is_none());
    }

    /// 测试 `UdpSocketStatus` 转换
    #[test]
    fn test_udp_socket_status_as_str() {
        assert_eq!(UdpSocketStatus::Binding.as_str(), "Binding");
        assert_eq!(UdpSocketStatus::Bound.as_str(), "Bound");
        assert_eq!(UdpSocketStatus::Closed.as_str(), "Closed");
        assert_eq!(UdpSocketStatus::Error.as_str(), "Error");
    }

    /// 测试 `UdpSocketStatus` Display
    #[test]
    fn test_udp_socket_status_display() {
        assert_eq!(format!("{}", UdpSocketStatus::Binding), "Binding");
        assert_eq!(format!("{}", UdpSocketStatus::Bound), "Bound");
    }

    /// 测试 UDP Socket 不存在的操作
    #[tokio::test]
    async fn test_udp_socket_not_found() {
        let manager = UdpSocketManager::new();

        // 发送到不存在的 socket
        let send_result = manager
            .send_to(
                "non-existent",
                vec![1, 2, 3],
                "127.0.0.1".to_string(),
                9999,
                1000,
            )
            .await;
        assert!(send_result.is_err());
        assert!(send_result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("not found"));

        // 从不存在的 socket 接收
        let recv_result = manager.recv_from("non-existent", 1000).await;
        assert!(recv_result.is_err());
        assert!(recv_result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("not found"));

        // 关闭不存在的 socket
        let close_result = manager.close("non-existent").await;
        assert!(close_result.is_err());
        assert!(close_result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("not found"));
    }

    /// 测试 UDP 接收超时
    #[tokio::test]
    async fn test_udp_recv_timeout() {
        let manager = UdpSocketManager::new();

        // 绑定一个 socket
        let bind_result = manager
            .bind(
                "timeout-socket".to_string(),
                "超时测试".to_string(),
                "127.0.0.1".to_string(),
                0,
                100,
            )
            .await;
        assert!(bind_result.is_ok(), "绑定应Success");

        // 短超时接收，没有人发送数据，应返回 Ok(None)
        let recv_result = manager.recv_from("timeout-socket", 100).await;
        assert!(recv_result.is_ok(), "接收应Success");
        assert!(
            recv_result.unwrap_or_else(|e| panic!("{e}")).is_none(),
            "超时应返回 None"
        );

        // 清理
        let close_result = manager.close("timeout-socket").await;
        assert!(close_result.is_ok());
    }

    /// 测试 UDP 发送接收往返
    #[tokio::test]
    async fn test_udp_send_recv_roundtrip() {
        let manager = UdpSocketManager::new();

        // 绑定两个 socket
        let bind1 = manager
            .bind(
                "sender".to_string(),
                "发送方".to_string(),
                "127.0.0.1".to_string(),
                0,
                5000,
            )
            .await;
        assert!(bind1.is_ok(), "sender 绑定应Success");

        let bind2 = manager
            .bind(
                "receiver".to_string(),
                "接收方".to_string(),
                "127.0.0.1".to_string(),
                0,
                5000,
            )
            .await;
        assert!(bind2.is_ok(), "receiver 绑定应Success");

        // 获取 receiver 的端口
        let recv_port = {
            let sockets = manager
                .sockets
                .lock()
                .unwrap_or_else(|e| panic!("应获取锁: {e}"));
            let sock = sockets
                .get("receiver")
                .unwrap_or_else(|| panic!("receiver 应存在"));
            sock.local_port
        };

        // 发送数据
        let test_data = b"Hello, UDP!".to_vec();
        let send_result = manager
            .send_to(
                "sender",
                test_data.clone(),
                "127.0.0.1".to_string(),
                recv_port,
                3000,
            )
            .await;
        assert!(send_result.is_ok(), "发送应Success");

        // 接收数据
        let recv_result = manager.recv_from("receiver", 3000).await;
        assert!(recv_result.is_ok(), "接收应Success");
        let received = recv_result.unwrap_or_else(|e| panic!("接收应Success: {e}"));
        assert!(received.is_some(), "应接收到数据");
        let (recv_data, sender_addr, sender_port) =
            received.unwrap_or_else(|| panic!("应接收到数据"));
        assert_eq!(recv_data, test_data, "接收数据应与发送数据一致");
        assert_eq!(sender_addr, "127.0.0.1", "来源地址应为 127.0.0.1");
        assert!(sender_port > 0, "来源端口应大于 0");

        // 清理
        manager
            .close("sender")
            .await
            .unwrap_or_else(|e| panic!("关闭 sender 应Success: {e}"));
        manager
            .close("receiver")
            .await
            .unwrap_or_else(|e| panic!("关闭 receiver 应Success: {e}"));
    }

    /// 测试 UDP Socket 池生命周期：绑定 → 发送 → 接收 → 关闭
    #[tokio::test]
    async fn test_udp_pool_lifecycle() {
        let manager = UdpSocketManager::new();

        // 1. 绑定
        let bind_result = manager
            .bind(
                "lifecycle-1".to_string(),
                "生命周期测试".to_string(),
                "127.0.0.1".to_string(),
                0,
                5000,
            )
            .await;
        assert!(bind_result.is_ok(), "绑定应Success");

        // 验证状态
        let status = manager.get_status("lifecycle-1");
        assert_eq!(status, Some("Bound".to_string()));

        // 验证端口不为 0（自动分配）
        let port = {
            let sockets = manager
                .sockets
                .lock()
                .unwrap_or_else(|e| panic!("应获取锁: {e}"));
            let sock = sockets
                .get("lifecycle-1")
                .unwrap_or_else(|| panic!("应存在"));
            assert!(sock.local_port > 0, "端口应已自动分配");
            assert!(sock.is_bound());
            assert!(sock.last_error.is_none());
            sock.local_port
        };

        // 2. 创建第二个 socket 用于收发
        let bind_result2 = manager
            .bind(
                "lifecycle-2".to_string(),
                "生命周期测试2".to_string(),
                "127.0.0.1".to_string(),
                0,
                5000,
            )
            .await;
        assert!(bind_result2.is_ok(), "第二个绑定应Success");

        // 3. 发送数据
        let data = b"lifecycle test".to_vec();
        let send_result = manager
            .send_to(
                "lifecycle-2",
                data.clone(),
                "127.0.0.1".to_string(),
                port,
                3000,
            )
            .await;
        assert!(send_result.is_ok(), "发送应Success");

        // 4. 接收数据
        let recv_result = manager.recv_from("lifecycle-1", 3000).await;
        assert!(recv_result.is_ok(), "接收应Success");
        let received = recv_result.unwrap_or_else(|e| panic!("{e}"));
        assert!(received.is_some());
        let (recv_data, _, _) = received.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(recv_data, data);

        // 5. 关闭
        let close_result = manager.close("lifecycle-1").await;
        assert!(close_result.is_ok(), "关闭应Success");
        assert!(manager.get_status("lifecycle-1").is_none());

        let close_result2 = manager.close("lifecycle-2").await;
        assert!(close_result2.is_ok(), "关闭应Success");
        assert!(manager.get_status("lifecycle-2").is_none());
    }

    /// 测试并发访问
    #[tokio::test]
    async fn test_udp_concurrent_access() {
        let manager = UdpSocketManager::new();

        // 并发绑定 5 个 socket
        let mut handles = Vec::new();
        for i in 0..5 {
            let mgr = manager.clone();
            handles.push(tokio::spawn(async move {
                let uuid = format!("concurrent-{}", i);
                mgr.bind(
                    uuid.clone(),
                    format!("并发测试 {}", i),
                    "127.0.0.1".to_string(),
                    0,
                    5000,
                )
                .await
                .map(|_| uuid)
            }));
        }

        let mut uuids = Vec::new();
        for handle in handles {
            let result = handle.await.unwrap_or_else(|e| panic!("任务应完成: {e}"));
            if let Ok(uuid) = result {
                uuids.push(uuid);
            }
        }
        assert_eq!(uuids.len(), 5, "应有 5 个绑定Success");

        // 验证所有 socket 都存在
        for uuid in &uuids {
            assert_eq!(manager.get_status(uuid), Some("Bound".to_string()));
        }

        // 并发关闭
        let mut close_handles = Vec::new();
        for uuid in uuids {
            let mgr = manager.clone();
            close_handles.push(tokio::spawn(async move { mgr.close(&uuid).await }));
        }

        for handle in close_handles {
            let result = handle.await.unwrap_or_else(|e| panic!("任务应完成: {e}"));
            assert!(result.is_ok(), "关闭应Success");
        }
    }

    /// 测试重复绑定同一 UUID 会覆盖旧 Socket
    #[tokio::test]
    async fn test_udp_bind_duplicate_uuid() {
        let manager = UdpSocketManager::new();

        // 第一次绑定
        let result1 = manager
            .bind(
                "dup-uuid".to_string(),
                "第一次绑定".to_string(),
                "127.0.0.1".to_string(),
                0,
                5000,
            )
            .await;
        assert!(result1.is_ok());

        let port1 = {
            let sockets = manager
                .sockets
                .lock()
                .unwrap_or_else(|e| panic!("应获取锁: {e}"));
            sockets
                .get("dup-uuid")
                .unwrap_or_else(|| panic!("应存在"))
                .local_port
        };

        // 第二次绑定（同一 UUID）
        let result2 = manager
            .bind(
                "dup-uuid".to_string(),
                "第二次绑定".to_string(),
                "127.0.0.1".to_string(),
                0,
                5000,
            )
            .await;
        assert!(result2.is_ok());

        // 验证端口已变化（被覆盖）
        let sockets = manager
            .sockets
            .lock()
            .unwrap_or_else(|e| panic!("应获取锁: {e}"));
        let sock = sockets.get("dup-uuid").unwrap_or_else(|| panic!("应存在"));
        assert_ne!(sock.local_port, port1, "端口应已变化");
        assert_eq!(sock.name, "第二次绑定");
    }

    /// 测试绑定Failed（无效地址）
    #[tokio::test]
    async fn test_udp_bind_invalid_address() {
        let manager = UdpSocketManager::new();

        // 使用无效 IP 地址和端口来触发绑定Failed
        let result = manager
            .bind(
                "invalid-1".to_string(),
                "无效地址".to_string(),
                "256.256.256.256".to_string(),
                12345,
                5000,
            )
            .await;

        assert!(result.is_err(), "绑定应Failed");
        let err_msg = result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"));
        // 无效地址可能在解析阶段Failed，也可能在 bind 阶段Failed
        assert!(
            err_msg.contains("bind failed") || err_msg.contains("Address parsing failed"),
            "Error信息应包含Failed原因，实际: {}",
            err_msg
        );
    }

    /// 测试 `ActiveUdpSocket` 创建和状态
    #[tokio::test]
    async fn test_active_udp_socket_new() {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|e| panic!("绑定应Success: {e}"));
        let port = socket
            .local_addr()
            .unwrap_or_else(|e| panic!("应获取地址: {e}"))
            .port();

        let active = ActiveUdpSocket::new(
            "uuid-1".to_string(),
            "测试Socket".to_string(),
            "127.0.0.1".to_string(),
            port,
            socket,
        );

        assert_eq!(active.uuid, "uuid-1");
        assert_eq!(active.name, "测试Socket");
        assert_eq!(active.status, UdpSocketStatus::Bound);
        assert_eq!(active.local_addr, "127.0.0.1");
        assert_eq!(active.local_port, port);
        assert!(active.bound_at > 0);
        assert!(active.last_error.is_none());
        assert!(active.is_bound());
    }

    /// 测试 `ActiveUdpSocket` Error状态
    #[tokio::test]
    async fn test_active_udp_socket_error() {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|e| panic!("绑定应Success: {e}"));
        let port = socket
            .local_addr()
            .unwrap_or_else(|e| panic!("应获取地址: {e}"))
            .port();

        let mut active = ActiveUdpSocket::new(
            "uuid-2".to_string(),
            "ErrorSocket".to_string(),
            "127.0.0.1".to_string(),
            port,
            socket,
        );

        active.set_error("绑定冲突".to_string());
        assert_eq!(active.status, UdpSocketStatus::Error);
        assert_eq!(active.last_error, Some("绑定冲突".to_string()));
        assert!(!active.is_bound());
    }

    /// 测试 `ActiveUdpSocket` Debug 输出
    #[tokio::test]
    async fn test_active_udp_socket_debug() {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|e| panic!("绑定应Success: {e}"));
        let port = socket
            .local_addr()
            .unwrap_or_else(|e| panic!("应获取地址: {e}"))
            .port();

        let active = ActiveUdpSocket::new(
            "uuid-debug".to_string(),
            "调试Socket".to_string(),
            "127.0.0.1".to_string(),
            port,
            socket,
        );

        let debug_str = format!("{:?}", active);
        assert!(debug_str.contains("uuid-debug"));
        assert!(debug_str.contains("调试Socket"));
        assert!(debug_str.contains("127.0.0.1"));
    }

    /// 测试 `ActiveUdpSocket` `to_socket_info`
    #[tokio::test]
    async fn test_active_udp_socket_to_info() {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|e| panic!("绑定应Success: {e}"));
        let port = socket
            .local_addr()
            .unwrap_or_else(|e| panic!("应获取地址: {e}"))
            .port();

        let active = ActiveUdpSocket::new(
            "uuid-info".to_string(),
            "信息Socket".to_string(),
            "127.0.0.1".to_string(),
            port,
            socket,
        );

        let info = active.to_socket_info();
        assert_eq!(info.uuid, "uuid-info");
        assert_eq!(info.name, "信息Socket");
        assert_eq!(info.status, "Bound");
        assert_eq!(info.local_addr, "127.0.0.1");
        assert_eq!(info.local_port, port);
        assert!(info.last_error.is_none());
    }
}
