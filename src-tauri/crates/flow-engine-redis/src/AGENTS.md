# flow-engine-redis

**Layer 1** | Protocol crate | 6 files, ~1,328 LOC

## OVERVIEW

Redis protocol node executor. Uses `redis-rs` async client. Connection pool managed by `RedisConnectionManager` keyed by host/port/db.

## STRUCTURE

| File | Purpose |
|------|---------|
| `lib.rs` | Module declarations, re-exports `RedisConnectionManager` + `RedisPluginGroup` |
| `plugin.rs` | Registers 3 node plugins: `RedisConnectPlugin`, `RedisCommandPlugin`, `RedisClosePlugin`. Port defs, default configs, icon/color metadata |
| `redis_connection_manager.rs` | ~740 lines. Connection pool manager. Pools keyed by `(host, port, db)`, lazy init, TTL cleanup |
| `redis_connect_executor.rs` | Opens connection via manager, stores handle in context, routes true/false ports |
| `redis_command_executor.rs` | Executes arbitrary Redis commands (GET, SET, etc.) via the pooled connection |
| `redis_close_executor.rs` | Closes and removes the Redis connection from the pool |

## DEPENDENCIES

- `flow-engine-core` (Layer 0): NodePlugin trait, NodeData types, ExecutionContext, node enums
- `redis-rs` with aio + tokio-comp features

## CONVENTIONS

- Connection config via `NodeData::RedisConnect`
- Commands via `NodeData::RedisCommand` (command field + args array)
- Connection handle stored in execution context under a node-scoped key
- On connect failure, routes through the `"false"` port
