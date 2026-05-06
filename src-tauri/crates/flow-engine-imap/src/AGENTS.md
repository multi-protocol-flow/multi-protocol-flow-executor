# flow-engine-imap

IMAP 邮件协议 crate。提供邮件收发、邮箱管理、搜索、IDLE 实时监听等功能。

## WHERE TO LOOK

| 节点类型 | 执行器文件 | 功能 |
|----------|-----------|------|
| imap_connect | imap_connect_executor.rs | 建立 IMAP 连接 |
| imap_select | imap_select_executor.rs | 选择邮箱(INBOX等) |
| imap_search | imap_search_executor.rs | 搜索邮件(支持复杂条件) |
| imap_fetch | imap_fetch_executor.rs | 获取邮件内容与元数据 |
| imap_store_flags | imap_store_flags_executor.rs | 设置邮件标志(已读/标记/删除) |
| imap_copy | imap_copy_executor.rs | 复制邮件到其他邮箱 |
| imap_move | imap_move_executor.rs | 移动邮件到其他邮箱 |
| imap_expunge | imap_expunge_executor.rs | 永久删除已标记邮件 |
| imap_list | imap_list_executor.rs | 列出所有邮箱/文件夹 |
| imap_folder_create | imap_folder_create_executor.rs | 创建新文件夹 |
| imap_folder_delete | imap_folder_delete_executor.rs | 删除文件夹 |
| imap_idle | imap_idle_executor.rs | IDLE 模式监听新邮件 |
| imap_close | imap_close_executor.rs | 关闭 IMAP 连接 |

## 核心文件

| 文件 | 作用 |
|------|------|
| imap_connection_manager.rs | 连接池管理，TLS 支持 |
| plugin.rs | 13 个节点的 NodePlugin 注册 |
| lib.rs | 模块导出，ImapPluginGroup |

## 约定

- 使用 async-imap + mail-parser 解析邮件
- 连接管理器模式: ImapConnectionManager 管理会话
- TLS 模式: implicit(993) / starttls(143) / none
- 端口: true/false 双端口，close 节点为单输出端口
- 邮件标志: \Seen \Flagged \Deleted \Draft \Answered
