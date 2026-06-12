# flow-engine-ws

**Generated:** 2026-06-12 | **6 files**, ~5,500 LOC | tokio-tungstenite 0.28

WebSocket 协议 crate。实现 3 个 WebSocket 节点类型的执行逻辑，基于 tokio-tungstenite + rustls 构建。

## 文件职责

| 文件 | 行数 | 职责 |
|------|------|------|
| `lib.rs` | 14 | 模块声明，crate 入口 |
| `plugin.rs` | 398 | 3 个 NodePlugin trait 实现 + WsPluginGroup 批量注册 |
| `websocket_manager.rs` | 2,786 | 连接池管理，TLS 配置，消息收发，ping/pong 保活 |
| `ws_connect_executor.rs` | 994 | WsConnect 节点执行器 — 建立连接 + 可选响应采集 |
| `ws_send_collect_executor.rs` | 1,449 | WsSendCollect 节点执行器 — 发送消息 + 采集响应 |
| `ws_close_executor.rs` | 486 | WsClose 节点执行器 — 按 connection_id 关闭连接 |

## 3 个节点类型

- **WsConnect** — 建立 WebSocket 连接，支持自定义 headers/subprotocols/TLS 跳过验证，可配置 connect_timeout/collect_mode/collect_timeout
- **WsSendCollect** — 向指定 connection_id 发送消息（text/binary），支持 FirstMatch/All/RegexMatch/JsonPathMatch/NoCollect 五种采集模式
- **WsClose** — 关闭指定 connection_id 的连接，可配置 close_code/reason/timeout

连接端口: 3 个节点均输出 true（成功）/ false（失败）双端口。

## 连接池

`WebSocketConnectionManager` 提供流程级别连接复用。执行器通过 `WsProvider` trait 与连接池交互，每个连接由 UUID 标识，支持并发消息收发和超时控制。

## 采集模式

WsConnect 和 WsSendCollect 共享采集逻辑:
- **connect** — 仅建立连接不采集
- **collect** — 采集但不发消息（仅 WsSendCollect 支持）
- **both** — 发送后采集匹配响应
- **NoCollect** — 不等待任何响应

匹配方式: 首条匹配、全部匹配、正则匹配、JsonPath 匹配。

## 关键依赖

tokio-tungstenite, futures-util, tokio, rustls, webpki-roots, serde_json_path, regex, evalexpr。

## 关键入口

`WsPluginGroup::register_all(registry)` — 3 个插件的批量注册函数，供 flow-engine-executor 在初始化时调用。
