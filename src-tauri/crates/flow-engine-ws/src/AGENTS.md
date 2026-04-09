# flow-engine-ws

**Layer 1** | 5 文件 | 5,360 行 | 依赖 core

## 概述

WebSocket 连接管理 + 3 种节点执行器。基于 tokio-tungstenite。

## 结构

```
flow-engine-ws/src/
├── lib.rs                      # 模块导出
├── websocket_manager.rs        # 连接池管理 (1,936 行)
├── ws_connect_executor.rs      # 连接执行器 (913 行)
├── ws_send_collect_executor.rs # 发送收集执行器 (2,058 行)
└── ws_close_executor.rs        # 关闭连接执行器
```

## 快速定位

| 任务 | 文件 |
|------|------|
| WebSocket 连接管理 | `websocket_manager.rs` |
| 连接节点逻辑 | `ws_connect_executor.rs` |
| 发送收集逻辑 | `ws_send_collect_executor.rs` |
| 关闭连接逻辑 | `ws_close_executor.rs` |

## 节点类型 (3 种)

| 类型 | 执行器 | 端口 | 用途 |
|------|--------|------|------|
| WsConnect | `ws_connect_executor.rs` | true/false | 建立连接 |
| WsSendCollect | `ws_send_collect_executor.rs` | true/false | 发送消息并收集响应 |
| WsClose | `ws_close_executor.rs` | out | 关闭连接 |

## 关键特性

- **连接池**: `WebSocketConnectionManager` 在流程级别共享连接
- **Collect 模式**: 支持 connect/collect/both 三种收集模式
- **消息流**: 通过 `ws_stream.rs` (core crate) 的数据结构支持调试模式消息查看

## 输出数据字段

WsSendCollect 节点输出:
- `collected_messages`: 收集到的消息列表 (Array of `{data, type, parsed?}`)
- `matched_message`: 匹配的消息对象或 null
- `message_count`: 收集到的消息总数
- `collect_timed_out`: 是否收集超时
- `collect_response_ms`: 收集响应耗时 (ms)

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-ws            # 仅测 WS
cargo clippy -p flow-engine-ws          # 仅 Lint WS
```
