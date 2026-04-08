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
├── ws_send_wait_executor.rs    # 发送等待执行器 (2,058 行)
└── ws_close_executor.rs        # 关闭连接执行器
```

## 快速定位

| 任务 | 文件 |
|------|------|
| WebSocket 连接管理 | `websocket_manager.rs` |
| 连接节点逻辑 | `ws_connect_executor.rs` |
| 发送等待逻辑 | `ws_send_wait_executor.rs` |
| 关闭连接逻辑 | `ws_close_executor.rs` |

## 节点类型 (3 种)

| 类型 | 执行器 | 端口 | 用途 |
|------|--------|------|------|
| WsConnect | `ws_connect_executor.rs` | true/false | 建立连接 |
| WsSendWait | `ws_send_wait_executor.rs` | true/false | 发送消息并等待响应 |
| WsClose | `ws_close_executor.rs` | out | 关闭连接 |

## 关键特性

- **连接池**: `WebSocketConnectionManager` 在流程级别共享连接
- **Wait 模式**: 支持 connect/wait/both 三种等待模式
- **消息流**: 通过 `ws_stream.rs` (core crate) 的数据结构支持调试模式消息查看

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-ws            # 仅测 WS
cargo clippy -p flow-engine-ws          # 仅 Lint WS
```
