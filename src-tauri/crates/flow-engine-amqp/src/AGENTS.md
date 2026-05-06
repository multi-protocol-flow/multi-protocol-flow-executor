# flow-engine-amqp

**AMQP 0-9-1 + 1.0 双协议支持 crate**

支持 RabbitMQ (0-9-1) 和 Azure Service Bus/ActiveMQ Artemis (1.0) 两种 AMQP 协议变体。

## 结构

```
src/
├── lib.rs                          # 模块导出
├── amqp091/                        # AMQP 0-9-1 子模块 (lapin)
│   ├── plugin.rs                   # 11 个节点插件注册
│   ├── amqp_connection_manager.rs  # 连接池管理
│   ├── amqp_client.rs              # lapin 客户端封装
│   ├── amqp_connect_executor.rs    # 连接执行器
│   ├── amqp_close_executor.rs      # 关闭连接
│   ├── amqp_exchange_declare_executor.rs
│   ├── amqp_exchange_delete_executor.rs
│   ├── amqp_queue_declare_executor.rs
│   ├── amqp_queue_delete_executor.rs
│   ├── amqp_queue_bind_executor.rs
│   ├── amqp_publish_executor.rs    # 发布消息
│   ├── amqp_consume_executor.rs    # 消费消息
│   ├── amqp_get_executor.rs        # 获取单条消息
│   └── amqp_ack_executor.rs        # 消息确认
├── amqp10/                         # AMQP 1.0 子模块 (fe2o3-amqp)
│   ├── amqp10_connection_manager.rs
│   ├── amqp10_client.rs
│   ├── amqp10_connect_executor.rs
│   ├── amqp10_disconnect_executor.rs
│   ├── amqp10_send_executor.rs
│   ├── amqp10_receive_executor.rs
│   ├── amqp10_accept_executor.rs   # 确认消息
│   └── amqp10_reject_executor.rs   # 拒绝消息
```

## 快速定位

| 节点 | 执行器 | 协议 |
|------|--------|------|
| 连接 | `amqp_connect_executor.rs` / `amqp10_connect_executor.rs` | 0-9-1 / 1.0 |
| 关闭 | `amqp_close_executor.rs` / `amqp10_disconnect_executor.rs` | 0-9-1 / 1.0 |
| 发布/发送 | `amqp_publish_executor.rs` / `amqp10_send_executor.rs` | 0-9-1 / 1.0 |
| 消费/接收 | `amqp_consume_executor.rs` / `amqp10_receive_executor.rs` | 0-9-1 / 1.0 |
| 确认 | `amqp_ack_executor.rs` / `amqp10_accept_executor.rs` | 0-9-1 / 1.0 |
| 拒绝 | `amqp10_reject_executor.rs` | 1.0 |
| 交换器声明 | `amqp_exchange_declare_executor.rs` | 0-9-1 |
| 交换器删除 | `amqp_exchange_delete_executor.rs` | 0-9-1 |
| 队列声明 | `amqp_queue_declare_executor.rs` | 0-9-1 |
| 队列删除 | `amqp_queue_delete_executor.rs` | 0-9-1 |
| 队列绑定 | `amqp_queue_bind_executor.rs` | 0-9-1 |
| 获取消息 | `amqp_get_executor.rs` | 0-9-1 |
| 插件注册 | `plugin.rs` | 通用 |

## 约定

- 遵循 `flow-engine-core` 的 `NodePlugin` trait 模式
- 0-9-1 基于 lapin，1.0 基于 fe2o3-amqp
- 连接通过 `AmqpConnectionManager` / `Amqp10ConnectionManager` 统一管理
- 插件在 `AmqpPluginGroup::register_all()` 中批量注册

## 增量编译

```bash
cd src-tauri
cargo build -p flow-engine-amqp    # 仅 core + amqp
cargo test -p flow-engine-amqp     # 单 crate 测试
cargo clippy -p flow-engine-amqp   # 单 crate lint
```
