# flow-engine-kafka

**概述**: Apache Kafka 协议支持 crate。基于 `rdkafka` 实现客户端，支持 Schema Registry HTTP 接口。

**结构**: 双子系统设计
- Kafka 核心子系统: `rdkafka` 客户端 + 连接池管理
- Schema Registry 子系统: 独立 HTTP 客户端

## 节点类型 → 执行器映射

| 节点类型 | 执行器文件 | 说明 |
|---------|-----------|------|
| `kafka_connect` | `kafka_connect_executor.rs` | 连接 broker，支持 SASL/SSL |
| `kafka_produce` | `kafka_produce_executor.rs` | 生产消息，支持压缩/序列化 |
| `kafka_consume` | `kafka_consume_executor.rs` | 消费消息，支持手动提交 offset |
| `kafka_consume_batch` | `kafka_consume_batch_executor.rs` | 批量消费，count_timeout 策略 |
| `kafka_admin` | `kafka_admin_executor.rs` | Topic/分区/配置管理 |
| `kafka_schema_registry` | `kafka_schema_registry_executor.rs` | Schema 注册/查询/兼容性 |
| `kafka_close` | `kafka_close_executor.rs` | 关闭连接释放资源 |

## 核心模块

| 文件 | 职责 |
|------|------|
| `kafka_client.rs` | `rdkafka` Producer/Consumer 封装 |
| `kafka_connection_manager.rs` | 连接生命周期管理 + 连接池 |
| `schema_registry_client.rs` | Schema Registry REST API 客户端 |
| `plugin.rs` | 7 个 NodePlugin 注册 + KafkaPluginGroup |

## 快速定位

- 修改序列化/压缩逻辑 → `kafka_produce_executor.rs`
- 修改 offset 管理 → `kafka_consume_executor.rs`
- 添加 admin 操作 → `kafka_admin_executor.rs`
- Schema 兼容性问题 → `schema_registry_client.rs`
