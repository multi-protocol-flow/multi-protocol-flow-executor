/**
 * IMAP 节点配置类型
 * 与后端 src-tauri/crates/flow-engine-imap/ 中的结构体对齐
 */

import type { EmailAuthConfig } from './smtp';

// ============================================================================
// IMAP 节点配置
// ============================================================================

/**
 * IMAP Connect 节点配置
 */
export interface ImapConnectConfig {
  type: 'imap_connect';
  /** 连接 ID（留空自动生成） */
  connection_id?: string;
  /** IMAP 服务器地址 */
  host: string;
  /** IMAP 服务器端口 */
  port: number;
  /** TLS 模式 */
  tls_mode: 'none' | 'starttls' | 'implicit';
  /** 认证配置 */
  auth?: EmailAuthConfig;
}

/**
 * IMAP Select 节点配置
 */
export interface ImapSelectConfig {
  type: 'imap_select';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** 邮箱名称 */
  mailbox: string;
  /** 是否只读模式 */
  readonly?: boolean;
}

/**
 * IMAP Search 节点配置
 */
export interface ImapSearchConfig {
  type: 'imap_search';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** 邮箱名称（可选，使用当前已选择的邮箱） */
  mailbox?: string;
  /** IMAP 搜索条件字符串 */
  criteria: string;
  /** 是否使用 UID 模式 */
  uid?: boolean;
}

/**
 * IMAP Fetch 节点配置
 */
export interface ImapFetchConfig {
  type: 'imap_fetch';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** UID 集合（如 "1:5" 或 "1,3,5"） */
  uid_set: string;
  /** 获取项（如 "(ENVELOPE BODY.PEEK[] FLAGS)"） */
  items: string;
  /** 最大消息数 */
  max_messages?: number;
}

/**
 * IMAP Store Flags 节点配置
 */
export interface ImapStoreFlagsConfig {
  type: 'imap_store_flags';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** UID 集合 */
  uid_set: string;
  /** 操作类型 */
  operation: 'set' | 'add' | 'remove';
  /** 标志列表 */
  flags: string[];
}

/**
 * IMAP Copy 节点配置
 */
export interface ImapCopyConfig {
  type: 'imap_copy';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** UID 集合 */
  uid_set: string;
  /** 目标邮箱 */
  destination_mailbox: string;
}

/**
 * IMAP Move 节点配置
 */
export interface ImapMoveConfig {
  type: 'imap_move';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** UID 集合 */
  uid_set: string;
  /** 目标邮箱 */
  destination_mailbox: string;
}

/**
 * IMAP Expunge 节点配置
 */
export interface ImapExpungeConfig {
  type: 'imap_expunge';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
}

/**
 * IMAP List 节点配置
 */
export interface ImapListConfig {
  type: 'imap_list';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** 引用名称 */
  reference_name?: string;
  /** 邮箱匹配模式 */
  mailbox_pattern?: string;
}

/**
 * IMAP Folder Create 节点配置
 */
export interface ImapFolderCreateConfig {
  type: 'imap_folder_create';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** 文件夹名称 */
  folder_name: string;
}

/**
 * IMAP Folder Delete 节点配置
 */
export interface ImapFolderDeleteConfig {
  type: 'imap_folder_delete';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** 文件夹名称 */
  folder_name: string;
}

/**
 * IMAP Idle 节点配置
 */
export interface ImapIdleConfig {
  type: 'imap_idle';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
  /** 邮箱名称 */
  mailbox: string;
  /** 超时时间（秒） */
  timeout_secs?: number;
}

/**
 * IMAP Close 节点配置
 */
export interface ImapCloseConfig {
  type: 'imap_close';
  /** 连接 ID（引用 imap_connect 节点的 UUID） */
  connection_id: string;
}

// ============================================================================
// IMAP 结果类型
// ============================================================================

/**
 * IMAP 连接结果
 */
export interface ImapConnectResult {
  /** 是否成功 */
  success: boolean;
  /** 服务器信息 */
  server_info?: string;
  /** 服务器支持的能力列表 */
  capabilities?: string[];
  /** 错误信息 */
  error?: string;
}

/**
 * IMAP 搜索结果
 */
export interface ImapSearchResult {
  /** 是否成功 */
  success: boolean;
  /** 匹配的 UID 列表 */
  uids?: number[];
  /** 匹配数量 */
  count?: number;
  /** 错误信息 */
  error?: string;
}

/**
 * IMAP 附件信息
 */
export interface ImapAttachment {
  /** 文件名 */
  filename: string;
  /** MIME 类型 */
  content_type: string;
  /** 文件大小（字节） */
  size: number;
}

/**
 * IMAP 邮件消息
 */
export interface ImapMessage {
  /** 邮件 UID */
  uid: number;
  /** 主题 */
  subject?: string;
  /** 发件人 */
  from?: string;
  /** 收件人列表 */
  to?: string[];
  /** 日期 */
  date?: string;
  /** 标志列表 */
  flags?: string[];
  /** 纯文本正文 */
  body_text?: string;
  /** HTML 正文 */
  body_html?: string;
  /** 附件列表 */
  attachments?: ImapAttachment[];
}

/**
 * IMAP 获取结果
 */
export interface ImapFetchResult {
  /** 是否成功 */
  success: boolean;
  /** 消息列表 */
  messages?: ImapMessage[];
  /** 消息数量 */
  count?: number;
  /** 错误信息 */
  error?: string;
}

/**
 * IMAP 文件夹信息
 */
export interface ImapFolder {
  /** 文件夹名称 */
  name: string;
  /** 层级分隔符 */
  delimiter: string;
  /** 文件夹属性 */
  attributes: string[];
}

/**
 * IMAP 列表结果
 */
export interface ImapListResult {
  /** 是否成功 */
  success: boolean;
  /** 文件夹列表 */
  folders?: ImapFolder[];
  /** 错误信息 */
  error?: string;
}

/**
 * IMAP 通用操作结果
 */
export interface ImapGenericResult {
  /** 是否成功 */
  success: boolean;
  /** 错误信息 */
  error?: string;
}
