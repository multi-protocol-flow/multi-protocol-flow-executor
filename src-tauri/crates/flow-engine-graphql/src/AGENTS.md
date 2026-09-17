# flow-engine-graphql

**Layer 1 (Protocol)** | **~6,100 LOC** | **9 .rs files** | **Optional feature: `protocol-graphql`**

## Overview

GraphQL protocol support covering Query/Mutation via HTTP POST (multipart), Subscription via `graphql-transport-ws` (WebSocket), and standard Introspection. Auth supports Bearer, Cookie, and API Key header injection. Connection lifecycle is managed by `GraphqlConnectionManager` for multiple concurrent endpoints.

## Structure

| File | Role |
|------|------|
| `lib.rs` | Module declarations, re-exports of `GraphqlClient`, `GraphqlConnectionManager`, response types |
| `plugin.rs` | 5 `NodePlugin` impls (Connect/Query/Subscribe/Introspect/Disconnect) + `GraphqlPluginGroup` batch registration |
| `graphql_client.rs` | Low-level HTTP+WS client: multipart POST, `graphql-transport-ws` framing, timing capture |
| `graphql_connection_manager.rs` | `GraphqlConnectionManager` + `GraphqlConnection` — connection pool, lifecycle tracking |
| `graphql_connect_executor.rs` | Connects to endpoint, stores connection handle |
| `graphql_disconnect_executor.rs` | Cleans up and disconnects |
| `graphql_introspect_executor.rs` | Runs `__schema` introspection, returns full schema metadata |
| `graphql_query_executor.rs` | Executes query/mutation with variables, returns `data` + `errors` |
| `graphql_subscribe_executor.rs` | Opens subscription, streams events back |

## Node Types (5)

`graphql:connect`, `graphql:query`, `graphql:subscribe`, `graphql:introspect`, `graphql:disconnect`

## Key Dependencies

`flow-engine-core`, `reqwest` (json/stream/multipart), `tokio-tungstenite` (rustls), `futures-util`, `tokio`, `serde`, `serde_json`, `chrono`, `log`
