//! UDP 提供者抽象 trait

use async_trait::async_trait;

/// UDP Socket 信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UdpSocketInfo {
    /// Socket 唯一标识
    pub uuid: String,
    /// Socket 名称
    pub name: String,
    /// Socket 状态
    pub status: String,
    /// 本地绑定地址
    pub local_addr: String,
    /// 本地绑定端口
    pub local_port: u16,
    /// 绑定时间（Unix 时间戳，毫秒）
    pub bound_at: u64,
    /// 最近一次错误信息
    pub last_error: Option<String>,
}

/// UDP 提供者 trait
///
/// 抽象 UDP Socket 管理功能，使得执行器可以与具体的 UDP 实现解耦。
#[async_trait]
pub trait UdpProvider: Send + Sync {
    /// 绑定 UDP Socket 到指定地址和端口
    ///
    /// # 参数
    /// - `uuid` - Socket 唯一标识（使用节点 UUID）
    /// - `name` - Socket 名称
    /// - `local_addr` - 本地绑定地址（如 "0.0.0.0"）
    /// - `local_port` - 本地绑定端口（0 表示自动分配）
    /// - `read_timeout_ms` - 读超时（毫秒，0 表示不超时）
    async fn bind(
        &self,
        uuid: String,
        name: String,
        local_addr: String,
        local_port: u16,
        read_timeout_ms: u64,
    ) -> Result<(), String>;

    /// 发送数据到指定目标地址
    ///
    /// # 参数
    /// - `uuid` - Socket 唯一标识
    /// - `data` - 要发送的数据（二进制）
    /// - `target_addr` - 目标地址
    /// - `target_port` - 目标端口
    /// - `send_timeout_ms` - 发送超时（毫秒）
    async fn send_to(
        &self,
        uuid: &str,
        data: Vec<u8>,
        target_addr: String,
        target_port: u16,
        send_timeout_ms: u64,
    ) -> Result<(), String>;

    /// 接收数据
    ///
    /// # 参数
    /// - `uuid` - Socket 唯一标识
    /// - `receive_timeout_ms` - 接收超时（毫秒）
    ///
    /// # 返回
    /// 成功时返回 (数据, 来源地址, 来源端口)，超时或无数据时返回 None
    async fn recv_from(
        &self,
        uuid: &str,
        receive_timeout_ms: u64,
    ) -> Result<Option<(Vec<u8>, String, u16)>, String>;

    /// 关闭 Socket
    ///
    /// # 参数
    /// - `uuid` - Socket 唯一标识
    async fn close(&self, uuid: &str) -> Result<(), String>;

    /// 获取 Socket 状态
    ///
    /// 返回状态的字符串表示（如 "Bound"、"Closed" 等），如果 Socket 不存在则返回 None
    fn get_status(&self, uuid: &str) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_socket_info_serialization() {
        let info = UdpSocketInfo {
            uuid: "socket-001".to_string(),
            name: "测试Socket".to_string(),
            status: "Bound".to_string(),
            local_addr: "0.0.0.0".to_string(),
            local_port: 9090,
            bound_at: 1700000000000,
            last_error: None,
        };
        let json = serde_json::to_string(&info).expect("序列化应成功");
        let deserialized: UdpSocketInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.uuid, "socket-001");
        assert_eq!(deserialized.local_addr, "0.0.0.0");
        assert_eq!(deserialized.local_port, 9090);
        assert!(deserialized.last_error.is_none());
    }

    #[test]
    fn test_udp_socket_info_with_error() {
        let info = UdpSocketInfo {
            uuid: "socket-002".to_string(),
            name: "错误Socket".to_string(),
            status: "Error".to_string(),
            local_addr: "127.0.0.1".to_string(),
            local_port: 8080,
            bound_at: 1700000000000,
            last_error: Some("端口已被占用".to_string()),
        };
        let json = serde_json::to_string(&info).expect("序列化应成功");
        let deserialized: UdpSocketInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.last_error, Some("端口已被占用".to_string()));
    }

    #[test]
    fn test_udp_socket_info_auto_assigned_port() {
        let info = UdpSocketInfo {
            uuid: "socket-003".to_string(),
            name: "自动端口".to_string(),
            status: "Bound".to_string(),
            local_addr: "0.0.0.0".to_string(),
            local_port: 0,
            bound_at: 1700000000000,
            last_error: None,
        };
        let json = serde_json::to_string(&info).expect("序列化应成功");
        let deserialized: UdpSocketInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.local_port, 0);
    }
}
