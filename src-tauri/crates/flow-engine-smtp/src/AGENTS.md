# flow-engine-smtp

**Layer 1** | 8 文件 | ~2,400 行 | 依赖 core + crypto

## 概述

SMTP 邮件发送 crate (lettre)。支持连接池管理、多种认证方式 (SASL PLAIN/LOGIN、OAuth2 XOAUTH2)、TLS 加密、附件上传。

## 结构

```
flow-engine-smtp/src/
├── lib.rs                       # 模块导出
├── plugin.rs                    # NodePlugin 实现 (3 节点)
├── smtp_connection_manager.rs   # 连接池 + SmtpProvider trait 实现
├── smtp_connect_executor.rs     # SMTP 连接执行器
├── smtp_send_executor.rs        # 发送邮件执行器
├── smtp_disconnect_executor.rs  # 断开连接执行器
├── sasl.rs                      # SASL 认证 (PLAIN/LOGIN → lettre Mechanism)
└── oauth2.rs                    # OAuth2 流程 (PKCE、Device Flow、Token 刷新)
```

## 快速定位

| 任务 | 文件 |
|------|------|
| SMTP 连接 | `smtp_connect_executor.rs` |
| 发送邮件 | `smtp_send_executor.rs` |
| 断开连接 | `smtp_disconnect_executor.rs` |
| NodePlugin 注册 | `plugin.rs` |
| 连接池管理 | `smtp_connection_manager.rs` |
| SASL 认证 | `sasl.rs` |
| OAuth2 (Gmail/Outlook) | `oauth2.rs` |

## 认证方式

- **SASL PLAIN/LOGIN**: 用户名+密码，密码加密存储 (AES-256-GCM)
- **OAuth2 XOAUTH2**: Gmail/Outlook 支持，支持 PKCE 和 Device Flow
- **TLS 模式**: Implicit (465)、StartTls (587)、None (25)

## 节点类型

| 节点 | type_id | 端口 |
|------|---------|------|
| SMTP 连接 | `smtp_connect` | true/false |
| SMTP 发送 | `smtp_send` | true/false |
| SMTP 断开 | `smtp_disconnect` | out |

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-smtp          # 仅测 SMTP
cargo clippy -p flow-engine-smtp        # 仅 Lint SMTP
```
