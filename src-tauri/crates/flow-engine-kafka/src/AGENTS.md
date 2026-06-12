# flow-engine-kafka

## Overview

Apache Kafka integration crate using **rdkafka** for cluster communication and a lightweight HTTP client for **Schema Registry** interactions. Dual-subsystem design separates core Kafka operations from schema management.

## Subsystems

### Kafka Core (rdkafka)
Producer / consumer / admin operations via `rdkafka`. Manages connections, topic metadata, consumer group offsets, and partition assignments.

### Schema Registry (HTTP)
Interacts with Confluent Schema Registry REST API. Supports subject lookups, schema registration, compatibility checks, and Avro/Protobuf/JSON Schema serialization.

## Node Types (7)

| Node | Executor | Purpose |
|------|----------|---------|
| `kafka_connect` | `kafka_connect_executor.rs` | Bootstrap connection to a Kafka cluster |
| `kafka_produce` | `kafka_produce_executor.rs` | Publish messages to a topic |
| `kafka_consume` | `kafka_consume_executor.rs` | Consume a single message (poll) |
| `kafka_consume_batch` | `kafka_consume_batch_executor.rs` | Consume multiple messages in one batch |
| `kafka_admin` | `kafka_admin_executor.rs` | Admin operations (list topics, describe groups, alter configs) |
| `kafka_schema_registry` | `kafka_schema_registry_executor.rs` | Schema Registry CRUD and compatibility checks |
| `kafka_close` | `kafka_close_executor.rs` | Gracefully tear down a Kafka connection |

## Key Files

| File | Role |
|------|------|
| `kafka_client.rs` | Wraps `rdkafka::BaseConsumer` and `FutureProducer`, handles partition assignment and offset commits |
| `kafka_connection_manager.rs` | Manages a map of named Kafka connections, connection pooling and lifecycle |
| `schema_registry_client.rs` | HTTP client for Schema Registry REST API (subjects, schemas, compatibility) |
| `plugin.rs` | Registers all 7 node executors into the flow engine's plugin system |
| `lib.rs` | Crate entry point, re-exports plugin and public types |

## Crate Info

- **Layer:** 1 (protocol), depends on `flow-engine-core`
- **Size:** 12 files, ~3,600 lines
- **Feature flag:** `protocol-kafka` (included in `all-protocols`)
