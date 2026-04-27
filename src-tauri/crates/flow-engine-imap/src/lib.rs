//! IMAP 协议支持模块
//!
//! 提供 IMAP 邮件接收、邮箱管理、搜索、IDLE 等功能。

/// IMAP 连接池管理器
pub mod imap_connection_manager;
/// IMAP 连接节点执行器
pub mod imap_connect_executor;
/// IMAP 选择邮箱节点执行器
pub mod imap_select_executor;
/// IMAP 搜索节点执行器
pub mod imap_search_executor;
/// IMAP 获取邮件节点执行器
pub mod imap_fetch_executor;
/// IMAP 标记邮件节点执行器
pub mod imap_store_flags_executor;
/// IMAP 复制邮件节点执行器
pub mod imap_copy_executor;
/// IMAP 移动邮件节点执行器
pub mod imap_move_executor;
/// IMAP 清除邮件节点执行器
pub mod imap_expunge_executor;
/// IMAP 列出文件夹节点执行器
pub mod imap_list_executor;
/// IMAP 创建文件夹节点执行器
pub mod imap_folder_create_executor;
/// IMAP 删除文件夹节点执行器
pub mod imap_folder_delete_executor;
/// IMAP IDLE 监听节点执行器
pub mod imap_idle_executor;
/// IMAP 关闭连接节点执行器
pub mod imap_close_executor;
/// IMAP 节点插件注册
pub mod plugin;

pub use imap_connection_manager::ImapConnectionManager;
pub use plugin::ImapPluginGroup;
