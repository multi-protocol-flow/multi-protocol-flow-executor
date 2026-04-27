/**
 * SMTP 节点配置类型
 * 与后端 src-tauri/crates/flow-engine-smtp/ 中的结构体对齐
 */

// ============================================================================
// 共享邮件类型
// ============================================================================

/**
 * 邮件服务提供商
 */
export type EmailProvider = 'gmail' | 'outlook' | 'yahoo' | 'custom';

/**
 * OAuth2 令牌
 */
export interface OAuth2Tokens {
  /** 访问令牌 */
  access_token: string;
  /** 刷新令牌 */
  refresh_token?: string;
  /** 过期时间（ISO 格式） */
  expires_at?: string;
  /** 邮件服务提供商 */
  provider?: EmailProvider;
}

/**
 * 邮件认证配置
 */
export interface EmailAuthConfig {
  /** 认证类型 */
  auth_type: 'plain' | 'login' | 'xoauth2';
  /** 用户名 */
  username: string;
  /** 密码 */
  password?: string;
  /** OAuth2 令牌 */
  oauth2_tokens?: OAuth2Tokens;
}

/**
 * 邮件附件
 */
export interface EmailAttachment {
  /** 文件名 */
  filename: string;
  /** MIME 类型 */
  content_type: string;
  /** 文件内容（Base64 编码） */
  content?: string;
  /** 文件路径 */
  file_path?: string;
}

// ============================================================================
// SMTP 节点配置
// ============================================================================

/**
 * SMTP Connect 节点配置
 */
export interface SmtpConnectConfig {
  type: 'smtp_connect';
  /** 连接 ID（留空自动生成） */
  connection_id?: string;
  /** SMTP 服务器地址 */
  host: string;
  /** SMTP 服务器端口 */
  port: number;
  /** TLS 模式 */
  tls_mode: 'none' | 'starttls' | 'implicit';
  /** 认证配置 */
  auth?: EmailAuthConfig;
}

/**
 * SMTP Send 节点配置
 */
export interface SmtpSendConfig {
  type: 'smtp_send';
  /** 连接 ID（引用 smtp_connect 节点的 UUID） */
  connection_id: string;
  /** 发件人地址 */
  from: string;
  /** 收件人列表 */
  to: string[];
  /** 抄送列表 */
  cc?: string[];
  /** 密送列表 */
  bcc?: string[];
  /** 邮件主题 */
  subject: string;
  /** 邮件正文 */
  body: string;
  /** 正文类型 */
  body_type: 'text' | 'html';
  /** 附件列表 */
  attachments?: EmailAttachment[];
  /** 回复地址 */
  reply_to?: string;
  /** 自定义邮件头 */
  headers?: Record<string, string>;
}

/**
 * SMTP Disconnect 节点配置
 */
export interface SmtpDisconnectConfig {
  type: 'smtp_disconnect';
  /** 连接 ID（引用 smtp_connect 节点的 UUID） */
  connection_id: string;
}

// ============================================================================
// SMTP 结果类型
// ============================================================================

/**
 * SMTP 连接结果
 */
export interface SmtpConnectResult {
  /** 是否成功 */
  success: boolean;
  /** 服务器信息 */
  server_info?: string;
  /** 错误信息 */
  error?: string;
}

/**
 * SMTP 发送结果
 */
export interface SmtpSendResult {
  /** 是否成功 */
  success: boolean;
  /** 消息 ID */
  message_id?: string;
  /** 错误信息 */
  error?: string;
}

/**
 * SMTP 断开结果
 */
export interface SmtpDisconnectResult {
  /** 是否成功 */
  success: boolean;
  /** 错误信息 */
  error?: string;
}
