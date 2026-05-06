# flow-engine-sse

SSE (Server-Sent Events) 协议支持 crate。

## 概述

提供 SSE 客户端连接、事件监听、断线续传等功能。使用 reqwest 流式请求解析 SSE 协议格式，不依赖外部 SSE 库。

## 文件映射

| 执行器 | 文件 | 说明 |
|--------|------|------|
| `sse_connect` | `sse_connect_executor.rs` + `plugin.rs` | 建立 SSE 连接，支持 Last-Event-ID 断线续传 |
| `sse_listen` | `sse_listen_executor.rs` + `plugin.rs` | 监听服务端推送事件，支持事件过滤/超时 |
| `sse_disconnect` | `sse_disconnect_executor.rs` + `plugin.rs` | 关闭 SSE 连接，释放资源 |
| 核心 | `sse_client.rs` | reqwest 流式客户端，解析 event/data/id/retry 字段 |
| 管理 | `sse_connection_manager.rs` | 连接池生命周期管理，实现 `SseProvider` trait |
| 入口 | `lib.rs` | 模块导出 |

## 节点类型

- `sse_connect`: 双端口 (true/false)，配置 url/timeout_ms/auto_reconnect
- `sse_listen`: 双端口 (true/false)，配置 connection_id/max_events/timeout_ms
- `sse_disconnect`: 单端口 (out)，配置 connection_id

## 依赖

- `flow-engine-core` (Layer 0)
- reqwest + futures-util + tokio
