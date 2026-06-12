# flow-engine-sse

**Layer 1 (Protocol)** | 7 files, ~3,000 LOC | Depends on `flow-engine-core`

## Overview

SSE (Server-Sent Events) client crate. Uses `reqwest` streaming GET with no external SSE library. Directly parses the `text/event-stream` wire format: `event:`, `data:`, `id:`, `retry:` fields with multiline data support, comment skip (`:` prefix), and null-byte protection on `id`.

## Node Types (3)

| Plugin | Type ID | Ports | Config |
|--------|---------|-------|--------|
| `SseConnectPlugin` | `sse_connect` | true / false | url, timeout_ms, auto_reconnect |
| `SseListenPlugin` | `sse_listen` | true / false | connection_id, max_events, timeout_ms |
| `SseDisconnectPlugin` | `sse_disconnect` | out | connection_id |

## Core Files

- **sse_client.rs** — Wire-format SSE parser. `SseEvent` struct (event/data/id/retry), `SseClient` with `connect()` and `spawn_event_reader()`. Broadcast channel dispatches parsed events. CancellationToken for graceful shutdown.
- **sse_connection_manager.rs** — `SseProvider` trait impl. Connection lifecycle (Connecting/Connected/Disconnected/Error). Per-connection background reader tasks, connection pool via `HashMap<String, Arc<RwLock<SseConnection>>>`.
- **plugin.rs** — `NodePlugin` trait impls for all 3 node types. `SsePluginGroup::register_all()` for batch registration.
- **sse_connect_executor.rs** — Opens SSE connection, stores connection in manager.
- **sse_listen_executor.rs** — Collects events from broadcast channel with timeout + event filter.
- **sse_disconnect_executor.rs** — Cancels reader task, removes connection from pool.

## Features

- Last-Event-ID reconnection via `Last-Event-ID` header
- Event type filtering on listen
- Configurable read timeout and max event count
- Broadcast-based multi-consumer dispatch
- Graceful cancellation via `CancellationToken`
