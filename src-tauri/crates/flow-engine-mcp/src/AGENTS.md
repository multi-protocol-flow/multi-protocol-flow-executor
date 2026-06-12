# flow-engine-mcp

**Layer 1 (Protocol)** | 9 source files | ~3,963 LOC | Dep: rmcp 1.5, flow-engine-core

## Overview

MCP (Model Context Protocol) client crate. Wraps the `rmcp` SDK to provide MCP connectivity as flow nodes. Supports Streamable HTTP and Stdio transport modes. All rmcp types stay internal; the public API exposes only `flow-engine-core` types through 5 node types.

## Structure

| File | Role |
|------|------|
| `lib.rs` | Module declarations, re-exports `McpConnection`, `McpConnectionManager`, and all 5 plugin types |
| `mcp_client.rs` | `create_mcp_client()` - builds a connected `RunningService` from `McpConnectConfig`, handles HTTP headers, transport selection, server metadata extraction |
| `mcp_connection_manager.rs` | `McpConnectionManager` + `McpConnection` - connection pool via `RwLock<HashMap>`, implements `McpProvider` trait for tool/resource/prompt operations |
| `mcp_connect_executor.rs` | `McpConnectExecutor` - establishes MCP connection, stores `McpConnection` into context provider |
| `mcp_call_tool_executor.rs` | `McpCallToolExecutor` - finds connection by ID, calls `mcp_call_tool` on the server |
| `mcp_read_resource_executor.rs` | `McpReadResourceExecutor` - reads a resource URI from connected server |
| `mcp_get_prompt_executor.rs` | `McpGetPromptExecutor` - retrieves a prompt template by name |
| `mcp_close_executor.rs` | `McpCloseExecutor` - cancels the `RunningService` and removes connection from pool |
| `plugin.rs` | 5 `NodePlugin` impls + `McpPluginGroup::register_all()` + comprehensive unit tests |

## Node Types

| Type ID | Display Name | Ports | Key Config |
|---------|-------------|-------|------------|
| `mcp_connect` | MCP 连接 | true/false | url, protocol_version, transport (streamable_http/stdio), command, headers |
| `mcp_call_tool` | MCP 调用工具 | true/false | connection_id, tool_name, arguments |
| `mcp_read_resource` | MCP 读取资源 | true/false | connection_id, resource_uri |
| `mcp_get_prompt` | MCP 获取提示词 | true/false | connection_id, prompt_name, arguments |
| `mcp_close` | MCP 关闭 | out | connection_id |

All protocol nodes are amber (`#F59E0B`). The first 4 use `success` boolean to resolve ports; `mcp_close` always routes to `out`.

## Where to Look

| Task | File |
|------|------|
| Add a new MCP node type | `plugin.rs` + new executor file |
| Modify transport / client init | `mcp_client.rs` |
| Change connection lifecycle | `mcp_connection_manager.rs` |
| Register/unregister plugins | `McpPluginGroup::register_all()` in `plugin.rs` |
| Understand the McpProvider trait contract | `flow-engine-core::traits` |

## Dependencies

- `rmcp` 1.5 (client + streamable-http + child-process features)
- `reqwest` (for custom HTTP headers on connect)
- `flow-engine-core` (node_config types, traits, plugin system)
- `async-trait`, `serde`, `serde_json`, `tokio`, `log`
