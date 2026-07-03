//! IMAP 提供者抽象 trait

use async_trait::async_trait;
use crate::email_types::ImapConfig;

/// IMAP 连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapConnectOutput {
    /// 是否Success
    pub success: bool,
    /// 连接 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// 服务器问候信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_greeting: Option<String>,
    /// 服务器能力列表
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 选择邮箱输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapSelectOutput {
    /// 是否Success
    pub success: bool,
    /// 邮箱名称
    pub mailbox: String,
    /// 邮件总数
    #[serde(default)]
    pub exists: u32,
    /// 最近邮件数
    #[serde(default)]
    pub recent: u32,
    /// 未读邮件数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unseen: Option<u32>,
    /// UID 有效性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid_validity: Option<u64>,
    /// 下一个 UID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid_next: Option<u64>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 邮件摘要信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapMessageInfo {
    /// UID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,
    /// 序号
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<u32>,
    /// 主题
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// 发件人
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// 收件人
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,
    /// 抄送
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
    /// 日期
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// 标志
    #[serde(default)]
    pub flags: Vec<String>,
    /// 大小（字节）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// 正文文本（纯文本）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_text: Option<String>,
    /// 正文 HTML
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_html: Option<String>,
    /// 附件列表
    #[serde(default)]
    pub attachments: Vec<crate::email_types::EmailAttachment>,
    /// 内部日期
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_date: Option<String>,
}

/// IMAP 搜索输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapSearchOutput {
    /// 是否Success
    pub success: bool,
    /// 匹配的 UID 列表
    #[serde(default)]
    pub uids: Vec<u32>,
    /// 匹配的邮件摘要（如果请求了预取）
    #[serde(default)]
    pub messages: Vec<ImapMessageInfo>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 获取输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapFetchOutput {
    /// 是否Success
    pub success: bool,
    /// 获取到的邮件列表
    #[serde(default)]
    pub messages: Vec<ImapMessageInfo>,
    /// 已跳过的邮件数（超过 `max_fetch_count` 限制）
    #[serde(default)]
    pub skipped_count: usize,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 设置标志输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapStoreFlagsOutput {
    /// 是否Success
    pub success: bool,
    /// 操作类型
    pub operation: String,
    /// 受影响的 UID 列表
    #[serde(default)]
    pub affected_uids: Vec<u32>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 复制输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapCopyOutput {
    /// 是否Success
    pub success: bool,
    /// 源邮箱
    pub source_mailbox: String,
    /// 目标邮箱
    pub target_mailbox: String,
    /// 复制的邮件数
    #[serde(default)]
    pub copied_count: usize,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 移动输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapMoveOutput {
    /// 是否Success
    pub success: bool,
    /// 源邮箱
    pub source_mailbox: String,
    /// 目标邮箱
    pub target_mailbox: String,
    /// 移动的邮件数
    #[serde(default)]
    pub moved_count: usize,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 永久删除输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapExpungeOutput {
    /// 是否Success
    pub success: bool,
    /// 已删除的邮件序号列表
    #[serde(default)]
    pub expunged_seqs: Vec<u32>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 邮箱信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapMailboxInfo {
    /// 邮箱名称
    pub name: String,
    /// 分隔符
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// 属性列表（如 `\HasNoChildren`）
    #[serde(default)]
    pub attributes: Vec<String>,
}

/// IMAP 列出邮箱输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapListOutput {
    /// 是否Success
    pub success: bool,
    /// 邮箱列表
    #[serde(default)]
    pub mailboxes: Vec<ImapMailboxInfo>,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 创建文件夹输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapFolderCreateOutput {
    /// 是否Success
    pub success: bool,
    /// 文件夹名称
    pub folder_name: String,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 删除文件夹输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapFolderDeleteOutput {
    /// 是否Success
    pub success: bool,
    /// 文件夹名称
    pub folder_name: String,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP IDLE 输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapIdleOutput {
    /// 是否Success
    pub success: bool,
    /// IDLE 期间检测到的变化
    #[serde(default)]
    pub events: Vec<String>,
    /// IDLE 持续时间（毫秒）
    #[serde(default)]
    pub duration_ms: u64,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 关闭连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapCloseOutput {
    /// 是否Success
    pub success: bool,
    /// Error信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// IMAP 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// 服务器地址
    pub host: String,
    /// 端口号
    pub port: u16,
}

/// IMAP 提供者 trait
///
/// 抽象 IMAP 连接管理功能，使得执行器可以与具体的 IMAP 实现解耦。
#[async_trait]
pub trait ImapProvider: Send + Sync {
    /// 建立 IMAP 连接
    async fn connect(
        &self,
        connection_id: &str,
        config: &ImapConfig,
    ) -> Result<ImapConnectOutput, String>;

    /// 选择邮箱
    async fn select(
        &self,
        connection_id: &str,
        mailbox: &str,
        readonly: bool,
    ) -> Result<ImapSelectOutput, String>;

    /// 搜索邮件
    async fn search(
        &self,
        connection_id: &str,
        mailbox: Option<&str>,
        criteria: &str,
        max_results: Option<usize>,
    ) -> Result<ImapSearchOutput, String>;

    /// 获取邮件
    async fn fetch(
        &self,
        connection_id: &str,
        uids: &str,
        fetch_items: &str,
        max_fetch_count: usize,
        max_size_bytes: usize,
    ) -> Result<ImapFetchOutput, String>;

    /// 设置邮件标志
    async fn store_flags(
        &self,
        connection_id: &str,
        uids: &str,
        operation: &str,
        flags: &[String],
    ) -> Result<ImapStoreFlagsOutput, String>;

    /// 复制邮件
    async fn copy(
        &self,
        connection_id: &str,
        uids: &str,
        target_mailbox: &str,
    ) -> Result<ImapCopyOutput, String>;

    /// 移动邮件
    async fn r#move(
        &self,
        connection_id: &str,
        uids: &str,
        target_mailbox: &str,
    ) -> Result<ImapMoveOutput, String>;

    /// 永久删除已标记邮件
    async fn expunge(
        &self,
        connection_id: &str,
    ) -> Result<ImapExpungeOutput, String>;

    /// 列出邮箱
    async fn list(
        &self,
        connection_id: &str,
        reference_name: &str,
        pattern: &str,
    ) -> Result<ImapListOutput, String>;

    /// 创建文件夹
    async fn create_folder(
        &self,
        connection_id: &str,
        folder_name: &str,
    ) -> Result<ImapFolderCreateOutput, String>;

    /// 删除文件夹
    async fn delete_folder(
        &self,
        connection_id: &str,
        folder_name: &str,
    ) -> Result<ImapFolderDeleteOutput, String>;

    /// IDLE 监听
    async fn idle(
        &self,
        connection_id: &str,
        mailbox: Option<&str>,
        timeout_seconds: u64,
    ) -> Result<ImapIdleOutput, String>;

    /// 关闭 IMAP 连接
    async fn close(
        &self,
        connection_id: &str,
    ) -> Result<ImapCloseOutput, String>;

    /// 测试 IMAP 连接
    async fn test_connection(&self, config: &ImapConfig) -> Result<bool, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imap_connect_output_serialization() {
        let output = ImapConnectOutput {
            success: true,
            connection_id: Some("conn-1".to_string()),
            server_greeting: Some("* OK [CAPABILITY IMAP4rev1]".to_string()),
            capabilities: vec!["IMAP4rev1".to_string(), "IDLE".to_string()],
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("conn-1"));

        let deserialized: ImapConnectOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.capabilities.len(), 2);
    }

    #[test]
    fn test_imap_select_output_serialization() {
        let output = ImapSelectOutput {
            success: true,
            mailbox: "INBOX".to_string(),
            exists: 42,
            recent: 3,
            unseen: Some(5),
            uid_validity: Some(1234567890),
            uid_next: Some(43),
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapSelectOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.exists, 42);
    }

    #[test]
    fn test_imap_message_info_serialization() {
        let msg = ImapMessageInfo {
            uid: Some(101),
            seq: Some(1),
            subject: Some("Test Subject".to_string()),
            from: Some("sender@example.com".to_string()),
            to: Some(vec!["recipient@example.com".to_string()]),
            cc: None,
            date: Some("2024-01-01T00:00:00Z".to_string()),
            flags: vec!["\\Seen".to_string()],
            size: Some(1024),
            body_text: Some("Hello".to_string()),
            body_html: None,
            attachments: vec![],
            internal_date: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ImapMessageInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uid, Some(101));
        assert_eq!(deserialized.subject, Some("Test Subject".to_string()));
    }

    #[test]
    fn test_imap_search_output_serialization() {
        let output = ImapSearchOutput {
            success: true,
            uids: vec![1, 2, 3, 5, 8],
            messages: vec![],
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapSearchOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.uids.len(), 5);
    }

    #[test]
    fn test_imap_fetch_output_serialization() {
        let output = ImapFetchOutput {
            success: true,
            messages: vec![],
            skipped_count: 0,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapFetchOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
    }

    #[test]
    fn test_imap_store_flags_output_serialization() {
        let output = ImapStoreFlagsOutput {
            success: true,
            operation: "add".to_string(),
            affected_uids: vec![1, 2, 3],
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapStoreFlagsOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.affected_uids.len(), 3);
    }

    #[test]
    fn test_imap_copy_output_serialization() {
        let output = ImapCopyOutput {
            success: true,
            source_mailbox: "INBOX".to_string(),
            target_mailbox: "Archive".to_string(),
            copied_count: 3,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapCopyOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.copied_count, 3);
    }

    #[test]
    fn test_imap_move_output_serialization() {
        let output = ImapMoveOutput {
            success: true,
            source_mailbox: "INBOX".to_string(),
            target_mailbox: "Trash".to_string(),
            moved_count: 2,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapMoveOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.moved_count, 2);
    }

    #[test]
    fn test_imap_expunge_output_serialization() {
        let output = ImapExpungeOutput {
            success: true,
            expunged_seqs: vec![3, 5],
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapExpungeOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.expunged_seqs.len(), 2);
    }

    #[test]
    fn test_imap_mailbox_info_serialization() {
        let info = ImapMailboxInfo {
            name: "INBOX".to_string(),
            delimiter: Some(".".to_string()),
            attributes: vec!["\\HasNoChildren".to_string()],
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ImapMailboxInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "INBOX");
    }

    #[test]
    fn test_imap_list_output_serialization() {
        let output = ImapListOutput {
            success: true,
            mailboxes: vec![ImapMailboxInfo {
                name: "INBOX".to_string(),
                delimiter: Some(".".to_string()),
                attributes: vec![],
            }],
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapListOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.mailboxes.len(), 1);
    }

    #[test]
    fn test_imap_folder_create_output_serialization() {
        let output = ImapFolderCreateOutput {
            success: true,
            folder_name: "NewFolder".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapFolderCreateOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.folder_name, "NewFolder");
    }

    #[test]
    fn test_imap_folder_delete_output_serialization() {
        let output = ImapFolderDeleteOutput {
            success: true,
            folder_name: "OldFolder".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapFolderDeleteOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.folder_name, "OldFolder");
    }

    #[test]
    fn test_imap_idle_output_serialization() {
        let output = ImapIdleOutput {
            success: true,
            events: vec!["EXISTS".to_string(), "RECENT".to_string()],
            duration_ms: 5000,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapIdleOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.events.len(), 2);
        assert_eq!(deserialized.duration_ms, 5000);
    }

    #[test]
    fn test_imap_close_output_serialization() {
        let output = ImapCloseOutput {
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ImapCloseOutput = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
    }

    #[test]
    fn test_imap_connection_info_serialization() {
        let info = ImapConnectionInfo {
            connection_id: "conn-1".to_string(),
            host: "imap.gmail.com".to_string(),
            port: 993,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ImapConnectionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.host, "imap.gmail.com");
        assert_eq!(deserialized.port, 993);
    }
}
