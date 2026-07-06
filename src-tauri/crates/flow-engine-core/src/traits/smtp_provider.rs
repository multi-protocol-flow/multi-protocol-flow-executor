//! SMTP 提供者抽象 trait

use crate::email_types::SmtpConfig;
use async_trait::async_trait;

/// SMTP 连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmtpConnectOutput {
    /// 是否Success
    pub success: bool,
    /// 连接 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// 服务器问候信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_greeting: Option<String>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// SMTP 发送输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmtpSendOutput {
    /// 是否Success
    pub success: bool,
    /// 消息 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// 响应码
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_code: Option<u16>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// SMTP 断开连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmtpDisconnectOutput {
    /// 是否Success
    pub success: bool,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// SMTP 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmtpConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
}

/// SMTP 提供者 trait
///
/// 抽象 SMTP 连接管理功能，使得执行器可以与具体的 SMTP 实现解耦。
#[async_trait]
pub trait SmtpProvider: Send + Sync {
    /// 建立 SMTP 连接
    async fn connect(
        &self,
        connection_id: &str,
        config: &SmtpConfig,
    ) -> Result<SmtpConnectOutput, String>;

    /// 发送邮件
    async fn send(
        &self,
        connection_id: &str,
        from: &str,
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: &str,
        body_type: &str,
        body: &str,
        html_body: &str,
        attachments: &[serde_json::Value],
    ) -> Result<SmtpSendOutput, String>;

    /// 断开 SMTP 连接
    async fn disconnect(&self, connection_id: &str) -> Result<SmtpDisconnectOutput, String>;

    /// 测试 SMTP 连接
    async fn test_connection(&self, config: &SmtpConfig) -> Result<bool, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_connect_output_serialization() {
        let output = SmtpConnectOutput {
            success: true,
            connection_id: Some("conn-1".to_string()),
            server_greeting: Some("220 smtp.gmail.com ESMTP".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("conn-1"));

        let deserialized: SmtpConnectOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.connection_id, Some("conn-1".to_string()));
    }

    #[test]
    fn test_smtp_send_output_serialization() {
        let output = SmtpSendOutput {
            success: true,
            message_id: Some("<msg123@gmail.com>".to_string()),
            response_code: Some(250),
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: SmtpSendOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.response_code, Some(250));
    }

    #[test]
    fn test_smtp_disconnect_output_serialization() {
        let output = SmtpDisconnectOutput {
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: SmtpDisconnectOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
    }

    #[test]
    fn test_smtp_connection_info_serialization() {
        let info = SmtpConnectionInfo {
            connection_id: "conn-1".to_string(),
            host: "smtp.gmail.com".to_string(),
            port: 465,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("smtp.gmail.com"));

        let deserialized: SmtpConnectionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.connection_id, "conn-1");
        assert_eq!(deserialized.port, 465);
    }
}
