# flow-engine-mqtt

**Layer 1** | 11 文件 | ~4,300 行 | 依赖 core

## 概述

MQTT 5.0 客户端 (rumqttc-next 0.30) + 7 种节点执行器 + NodePlugin 实现。支持 TCP、TLS (rustls)、WebSocket、WSS 四种传输，QoS 0/1/2，遗嘱消息 (LWT)，请求/响应模式。

## 结构

```
flow-engine-mqtt/src/
├── lib.rs                           # 模块导出
├── mqtt_client.rs                   # MQTT 客户端封装 (490 行)
├── mqtt_connection_manager.rs       # 连接池管理 + MqttProvider 实现 (900 行)
├── mqtt_connect_executor.rs         # 连接执行器 (611 行)
├── mqtt_publish_executor.rs         # 发布执行器
├── mqtt_subscribe_executor.rs       # 订阅执行器
├── mqtt_unsubscribe_executor.rs     # 取消订阅执行器
├── mqtt_disconnect_executor.rs      # 断开连接执行器
├── mqtt_wait_message_executor.rs    # 等待消息执行器
├── mqtt_request_executor.rs         # 请求/响应执行器
└── plugin.rs                        # NodePlugin 实现 (1,107 行)
```

## 快速定位

| 任务 | 文件 |
|------|------|
| NodePlugin 注册 | `plugin.rs` |
| MQTT 客户端创建 | `mqtt_client.rs` |
| 连接池管理 | `mqtt_connection_manager.rs` |
| 连接节点逻辑 | `mqtt_connect_executor.rs` |
| 发布节点逻辑 | `mqtt_publish_executor.rs` |
| 订阅节点逻辑 | `mqtt_subscribe_executor.rs` |
| 取消订阅节点逻辑 | `mqtt_unsubscribe_executor.rs` |
| 断开连接节点逻辑 | `mqtt_disconnect_executor.rs` |
| 等待消息节点逻辑 | `mqtt_wait_message_executor.rs` |
| 请求/响应节点逻辑 | `mqtt_request_executor.rs` |

## 节点类型 (7 种)

| 类型 | 执行器 | 端口 | 用途 |
|------|--------|------|------|
| MqttConnect | `mqtt_connect_executor.rs` | true/false | 建立 MQTT 连接 |
| MqttPublish | `mqtt_publish_executor.rs` | true/false | 发布消息到主题 |
| MqttSubscribe | `mqtt_subscribe_executor.rs` | true/false | 订阅主题，可选收集消息 |
| MqttUnsubscribe | `mqtt_unsubscribe_executor.rs` | true/false | 取消订阅主题 |
| MqttDisconnect | `mqtt_disconnect_executor.rs` | out | 断开 MQTT 连接 |
| MqttWaitMessage | `mqtt_wait_message_executor.rs` | true/false | 等待匹配主题的消息 |
| MqttRequest | `mqtt_request_executor.rs` | true/false | 请求/响应模式 (发布+订阅+等待) |

## 关键类型

| 类型 | 文件 | 用途 |
|------|------|------|
| `MqttProvider` trait | core/traits.rs | MQTT 操作抽象接口 |
| `MqttConnectionManager` | `mqtt_connection_manager.rs` | 连接池，实现 MqttProvider |
| `MqttConnection` | `mqtt_connection_manager.rs` | 单个 MQTT 连接 (client + EventLoop) |
| `MqttConnectionStatus` | `mqtt_connection_manager.rs` | 连接状态枚举 |
| `MqttClientResult` | `mqtt_client.rs` | 客户端创建结果 |
| `MqttPluginGroup` | `plugin.rs` | 批量注册 7 个 MQTT 插件 |

## 关键设计

### 传输方式 (4 种)
- **mqtt://** → 明文 TCP (默认端口 1883)
- **mqtts://** → TLS (rustls，默认端口 8883)
- **ws://** → WebSocket
- **wss://** → 安全 WebSocket

### TLS 模式 (2 种)
- **standard**: 使用 webpki-roots Mozilla 根证书验证
- **skip-verify**: 跳过证书验证 (仅开发/测试)

### 连接管理
- `MqttConnectionManager` 使用 `Arc<RwLock<HashMap>>` 保护连接池
- 后台 tokio 任务持续 poll `EventLoop` 分发消息
- 消息通过 `broadcast` 通道分发给多个消费者
- 共享模式 (`new_shared`) 适用于压力测试，disconnect 为 no-op

### 遗嘱消息 (LWT)
- 支持配置遗嘱主题、payload、QoS、retain
- MQTT 5.0 遗嘱属性：delay_interval

### 请求/响应模式
- `mqtt_request` 节点：订阅响应主题 → 发布请求 → 等待响应 → 取消订阅

## 依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| rumqttc-next | 0.30 | MQTT 5.0 客户端 (pure Rust) |
| rustls | 0.23 | TLS 加密 |
| webpki-roots | 0.26 | 根证书集合 |
| flow-engine-core | workspace | 基础类型、traits |

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-mqtt           # 仅测 MQTT
cargo clippy -p flow-engine-mqtt         # 仅 Lint MQTT
```
