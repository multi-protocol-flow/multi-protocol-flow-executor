//! MQTT 提供者抽象 trait

use async_trait::async_trait;
use crate::node_config::{
    MqttConnectConfig, MqttPublishConfig, MqttSubscribeConfig,
    MqttDisconnectConfig, MqttWaitMessageConfig, MqttRequestConfig,
};

/// MQTT 连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttConnectOutput {
    /// 是否Success
    pub success: bool,
    /// Broker 信息
    #[serde(default)]
    pub broker_info: Option<MqttBrokerInfo>,
    /// 原因码
    #[serde(default)]
    pub reason_code: Option<u8>,
    /// Error信息
    #[serde(default)]
    pub error: Option<String>,
}

/// MQTT Broker 信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttBrokerInfo {
    /// 协议版本
    #[serde(default)]
    pub version: Option<String>,
    /// 是否存在会话
    #[serde(default)]
    pub session_present: Option<bool>,
    /// 分配的客户端 ID
    #[serde(default)]
    pub assigned_client_id: Option<String>,
    /// 服务端保活时间（秒）
    #[serde(default)]
    pub server_keep_alive: Option<u16>,
    /// 最大主题别名数
    #[serde(default)]
    pub topic_alias_max: Option<u16>,
    /// 最大接收数
    #[serde(default)]
    pub receive_max: Option<u16>,
    /// 是否支持共享订阅
    #[serde(default)]
    pub shared_subscription_available: Option<bool>,
    /// 是否支持通配符订阅
    #[serde(default)]
    pub wildcard_sub_available: Option<bool>,
}

/// MQTT 发布输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttPublishOutput {
    /// 是否Success
    pub success: bool,
    /// 原因码
    #[serde(default)]
    pub reason_code: Option<u8>,
    /// Error信息
    #[serde(default)]
    pub error: Option<String>,
}

/// MQTT 订阅输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttSubscribeOutput {
    /// 是否Success
    pub success: bool,
    /// 原因码
    #[serde(default)]
    pub reason_code: Option<u8>,
    /// 接收到的消息列表
    #[serde(default)]
    pub messages: Vec<MqttReceivedMessage>,
    /// Error信息
    #[serde(default)]
    pub error: Option<String>,
}

/// MQTT 接收到的消息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttReceivedMessage {
    /// 主题
    pub topic: String,
    /// 负载数据
    pub payload: Vec<u8>,
    /// `QoS` 等级
    pub qos: u8,
    /// 是否为保留消息
    pub retain: bool,
    /// 负载格式指示符
    #[serde(default)]
    pub payload_format: Option<u8>,
    /// 时间戳（毫秒）
    pub timestamp_ms: i64,
}

/// MQTT 消息输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttMessageOutput {
    /// 是否Success
    pub success: bool,
    /// 接收到的消息列表
    pub messages: Vec<MqttReceivedMessage>,
    /// 时间信息
    #[serde(default)]
    pub timing: Option<MqttTimingInfo>,
    /// Error信息
    #[serde(default)]
    pub error: Option<String>,
}

/// MQTT 时间信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttTimingInfo {
    /// 总耗时（毫秒）
    pub total_ms: u64,
    /// 首条消息耗时（毫秒）
    #[serde(default)]
    pub first_message_ms: Option<u64>,
}

/// MQTT 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttConnectionInfo {
    /// 连接 UUID
    pub uuid: String,
    /// Broker URL
    pub url: String,
    /// 客户端 ID
    pub client_id: String,
    /// 连接状态
    pub status: String,
    /// 连接时间戳（毫秒）
    pub connected_at: i64,
}

/// MQTT 提供者 trait
///
/// 抽象 MQTT 连接管理功能，使得执行器可以与具体的 MQTT 实现解耦。
#[async_trait]
pub trait MqttProvider: Send + Sync {
    /// 建立 MQTT 连接
    async fn connect(
        &self,
        uuid: &str,
        config: &MqttConnectConfig,
    ) -> Result<MqttConnectOutput, String>;

    /// 发布消息
    async fn publish(
        &self,
        uuid: &str,
        config: &MqttPublishConfig,
    ) -> Result<MqttPublishOutput, String>;

    /// 订阅主题
    async fn subscribe(
        &self,
        uuid: &str,
        config: &MqttSubscribeConfig,
    ) -> Result<MqttSubscribeOutput, String>;

    /// 取消订阅
    async fn unsubscribe(
        &self,
        uuid: &str,
        connection_id: &str,
        topic: &str,
    ) -> Result<(), String>;

    /// 等待消息
    async fn wait_message(
        &self,
        uuid: &str,
        config: &MqttWaitMessageConfig,
    ) -> Result<MqttMessageOutput, String>;

    /// 请求-响应模式
    async fn request(
        &self,
        uuid: &str,
        config: &MqttRequestConfig,
    ) -> Result<MqttMessageOutput, String>;

    /// 断开连接
    async fn disconnect(
        &self,
        uuid: &str,
        config: &MqttDisconnectConfig,
    ) -> Result<(), String>;

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String>;

    /// 列出所有连接
    fn list_connections(&self) -> Vec<MqttConnectionInfo>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_connect_output_serialization() {
        let output = MqttConnectOutput {
            success: true,
            broker_info: Some(MqttBrokerInfo {
                version: Some("MQTT 5.0".to_string()),
                session_present: Some(false),
                assigned_client_id: Some("client-001".to_string()),
                server_keep_alive: Some(60),
                topic_alias_max: Some(10),
                receive_max: Some(20),
                shared_subscription_available: Some(true),
                wildcard_sub_available: Some(true),
            }),
            reason_code: Some(0),
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应Success");
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("MQTT 5.0"));
        assert!(json.contains("client-001"));

        let deserialized: MqttConnectOutput =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert!(deserialized.success);
        assert!(deserialized.broker_info.is_some());
    }

    #[test]
    fn test_mqtt_publish_output_serialization() {
        let output = MqttPublishOutput {
            success: true,
            reason_code: Some(0),
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应Success");
        let deserialized: MqttPublishOutput =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert!(deserialized.success);
        assert_eq!(deserialized.reason_code, Some(0));
    }

    #[test]
    fn test_mqtt_received_message_serialization() {
        let msg = MqttReceivedMessage {
            topic: "test/topic".to_string(),
            payload: vec![72, 101, 108, 108, 111],
            qos: 1,
            retain: false,
            payload_format: None,
            timestamp_ms: 1700000000000,
        };

        let json = serde_json::to_string(&msg).expect("序列化应Success");
        assert!(json.contains("test/topic"));

        let deserialized: MqttReceivedMessage =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.topic, "test/topic");
        assert_eq!(deserialized.payload, vec![72, 101, 108, 108, 111]);
        assert_eq!(deserialized.qos, 1);
    }

    #[test]
    fn test_mqtt_timing_info_serialization() {
        let timing = MqttTimingInfo {
            total_ms: 150,
            first_message_ms: Some(50),
        };

        let json = serde_json::to_string(&timing).expect("序列化应Success");
        let deserialized: MqttTimingInfo =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.total_ms, 150);
        assert_eq!(deserialized.first_message_ms, Some(50));
    }

    #[test]
    fn test_mqtt_connection_info_serialization() {
        let info = MqttConnectionInfo {
            uuid: "conn-uuid-001".to_string(),
            url: "mqtt://localhost:1883".to_string(),
            client_id: "client-001".to_string(),
            status: "connected".to_string(),
            connected_at: 1700000000000,
        };

        let json = serde_json::to_string(&info).expect("序列化应Success");
        assert!(json.contains("conn-uuid-001"));
        assert!(json.contains("mqtt://localhost:1883"));

        let deserialized: MqttConnectionInfo =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.uuid, "conn-uuid-001");
        assert_eq!(deserialized.status, "connected");
    }

    #[test]
    fn test_mqtt_message_output_serialization() {
        let output = MqttMessageOutput {
            success: true,
            messages: vec![MqttReceivedMessage {
                topic: "response/topic".to_string(),
                payload: vec![1, 2, 3],
                qos: 0,
                retain: false,
                payload_format: None,
                timestamp_ms: 1700000000000,
            }],
            timing: Some(MqttTimingInfo {
                total_ms: 200,
                first_message_ms: Some(100),
            }),
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应Success");
        let deserialized: MqttMessageOutput =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert!(deserialized.success);
        assert_eq!(deserialized.messages.len(), 1);
        assert_eq!(deserialized.timing.as_ref().expect("应有时间信息").total_ms, 200);
    }
}
