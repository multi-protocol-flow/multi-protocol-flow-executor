# flow-engine-mcp Crate

**MCP (Model Context Protocol) 协议 crate**

封装 rmcp SDK，提供 MCP 客户端连接管理与工具调用能力。

## 结构

```
src/
├── lib.rs                      # 模块导出
├── mcp_client.rs               # MCP 协议客户端 (Streamable HTTP)
├── mcp_connection_manager.rs   # 连接池管理
├── mcp_connect_executor.rs     # mcp_connect 执行器
├── mcp_call_tool_executor.rs   # mcp_call_tool 执行器
├── mcp_read_resource_executor.rs   # mcp_read_resource 执行器
├── mcp_get_prompt_executor.rs  # mcp_get_prompt 执行器
├── mcp_close_executor.rs       # mcp_close 执行器
└── plugin.rs                   # NodePlugin 实现 + McpPluginGroup
```

## 5 种节点类型

| 类型 | 执行器文件 | 功能 |
|------|-----------|------|
| `mcp_connect` | `mcp_connect_executor.rs` | 建立 MCP 连接 (Streamable HTTP / stdio) |
| `mcp_call_tool` | `mcp_call_tool_executor.rs` | 调用 MCP 工具 |
| `mcp_read_resource` | `mcp_read_resource_executor.rs` | 读取 MCP 资源 |
| `mcp_get_prompt` | `mcp_get_prompt_executor.rs` | 获取提示词模板 |
| `mcp_close` | `mcp_close_executor.rs` | 关闭连接 |

## 快速定位

| 任务 | 位置 |
|------|------|
| 修改连接逻辑 | `mcp_client.rs` |
| 修改连接池 | `mcp_connection_manager.rs` |
| 添加节点类型 | `plugin.rs` (插件定义) + 新建 `mcp_*_executor.rs` |
| 工具发现 | 前端调用 `mcp_discover_tools` IPC 命令 |

## 依赖

- `rmcp`: MCP SDK (client + streamable-http + stdio)
- `flow-engine-core`: 基础类型 + NodePlugin trait
