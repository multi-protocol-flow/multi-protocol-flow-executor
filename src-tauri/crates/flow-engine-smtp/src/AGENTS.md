# flow-engine-smtp

**Layer:** L1 (Protocol) | **Files:** 8 | **LOC:** ~4,056 | **Key dep:** lettre

## OVERVIEW

SMTP email sending crate. Uses `lettre` (Tokio1Executor) for async SMTP transport. Connection pool via `Arc<RwLock<HashMap>>`, credential decryption via `flow-engine-crypto` (AES-256-GCM), OAuth2 token flows for Gmail and Outlook.

## 3 NODE TYPES

| Node | Ports | Purpose |
|------|-------|---------|
| `smtp_connect` | true/false | Build transport, test connection, store in pool |
| `smtp_send` | true/false | Build MIME message (plain/HTML/both), send with optional base64 attachments |
| `smtp_disconnect` | out | Remove connection from pool (non-blocking) |

All 3 registered via `SmtpPluginGroup::register_all()` in `plugin.rs`.

## AUTHENTICATION

- **SASL PLAIN / LOGIN** — passwords encrypted with AES-256-GCM via `flow-engine-crypto::encryption`. Decrypted at connect time in `SmtpConnectionManager::apply_auth()`.
- **OAuth2 XOAUTH2** — for Gmail / Outlook. Two flows in `oauth2.rs`: PKCE (Tauri desktop, localhost callback listener) and Device Authorization Flow (CLI `mpe` binary, polling). Tokens encrypted via AES-256-GCM. Token refresh supported.

## TLS MODES

| Mode | Config value | Default port |
|------|-------------|-------------|
| Implicit TLS | `"implicit"` | 465 |
| STARTTLS | `"starttls"` | 587 |
| None (plaintext) | `"none"` | 25 |

## FILE MAP

| File | Responsibility |
|------|---------------|
| `lib.rs` | Module declarations |
| `smtp_connection_manager.rs` | Connection pool, transport builder, MIME message assembly, `SmtpProvider` impl |
| `smtp_connect_executor.rs` | Connect node logic |
| `smtp_send_executor.rs` | Send node logic |
| `smtp_disconnect_executor.rs` | Disconnect node logic |
| `oauth2.rs` | PKCE / Device Flow / token refresh / encrypted token storage |
| `sasl.rs` | SASL mechanism resolution + lettre `Credentials` builders |
| `plugin.rs` | 3 `NodePlugin` impls + `SmtpPluginGroup` + tests |

## DEPENDENCIES

`flow-engine-core` (types, traits, `SmtpProvider`), `flow-engine-crypto` (AES-256-GCM password/token decryption).
