# flow-engine-imap

**Layer 1 (Protocol)** | LOC: ~4,800 | 16 .rs files | Deps: async-imap, mail-parser, flow-engine-core, flow-engine-crypto

## OVERVIEW

IMAP mail protocol crate: connect, mailbox management, search, fetch, store flags, IDLE monitoring, folder operations. 13 node types registered via `ImapPluginGroup`:

- **Connection** (3): `imap_connect` / `imap_idle` / `imap_close`
- **Mailbox** (4): `imap_select` / `imap_list` / `imap_folder_create` / `imap_folder_delete`
- **Message** (5): `imap_search` / `imap_fetch` / `imap_store_flags` / `imap_copy` / `imap_move`
- **Maintenance** (1): `imap_expunge`

Connection state via `ImapConnectionManager` (flow-level provider). TLS modes: implicit (993, immediate TLS), starttls (143, STARTTLS upgrade), none.

## CORE FILES

| File | Role |
|------|------|
| `imap_connection_manager.rs` | Connection pool (`HashMap<uuid, ImapSession>`), TLS connect / session lifecycle |
| `plugin.rs` | `NodePlugin` trait impls for all 13 types + `ImapPluginGroup` batch registration |
| `imap_connect_executor.rs` | Connect to IMAP server |
| `imap_select_executor.rs` | Select mailbox |
| `imap_search_executor.rs` | Search by criteria (FROM, SUBJECT, SINCE, etc.) |
| `imap_fetch_executor.rs` | Fetch headers, body, flags, attachments |
| `imap_store_flags_executor.rs` | Add/remove/set message flags |
| `imap_copy_executor.rs` / `imap_move_executor.rs` | Cross-mailbox copy/move |
| `imap_expunge_executor.rs` | Permanently remove deleted messages |
| `imap_list_executor.rs` | List mailboxes/folders |
| `imap_folder_create_executor.rs` / `imap_folder_delete_executor.rs` | Folder management |
| `imap_idle_executor.rs` | IDLE for new mail notifications |
| `imap_close_executor.rs` | Release connection |

## WHERE TO LOOK

| Task | File |
|------|------|
| Connection pool internals | `imap_connection_manager.rs` |
| New IMAP node type | `plugin.rs` + executor + core config |
| Search / Fetch | `imap_search_executor.rs` / `imap_fetch_executor.rs` |
| IDLE keepalive | `imap_idle_executor.rs` |
| Flag ops | `imap_store_flags_executor.rs` |
