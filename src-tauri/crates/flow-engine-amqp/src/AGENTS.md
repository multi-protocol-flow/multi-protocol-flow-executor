# flow-engine-amqp

**Layer 1 (Protocol)** | ~26 files, ~16,380 LOC | Depends on `flow-engine-core`

## OVERVIEW

Dual-protocol AMQP support:

- **AMQP 0-9-1**: `lapin` (rustls) for RabbitMQ and traditional brokers.
- **AMQP 1.0**: `fe2o3-amqp` (rustls) for Azure Service Bus, ActiveMQ Artemis, and modern brokers.
- Optional `management` feature (reqwest) for RabbitMQ Management HTTP API.

## STRUCTURE

### `amqp091/` — 16 files (AMQP 0-9-1)

**Infrastructure:** `plugin.rs` (NodePlugin trait), `amqp_connection_manager.rs` (pool + lifecycle), `amqp_client.rs` (lapin wrapper).

**Executors:** `connect`, `close`, `exchange_declare`, `exchange_delete`, `queue_declare`, `queue_delete`, `queue_bind`, `publish`, `consume`, `get`, `ack`, `management` (feature-gated).

### `amqp10/` — 9 files (AMQP 1.0)

**Infrastructure:** `amqp10_connection_manager.rs` (pool, Session→Link model), `amqp10_client.rs` (fe2o3-amqp wrapper).

**Executors:** `connect`, `disconnect`, `send` (Sender link), `receive` (Receiver link), `accept` (settle), `reject`.

## DUAL PROTOCOL MAPPING

| Operation | 0-9-1 | 1.0 |
|-----------|-------|-----|
| Connect | `Connect` | `Connect` |
| Close | `Close` | `Disconnect` |
| Publish/Send | `Publish` | `Send` |
| Consume/Receive | `Consume` / `Get` | `Receive` |
| Ack/Accept | `Ack` | `Accept` |
| Reject | (nack) | `Reject` |

AMQP 0-9-1 adds broker management (exchanges, queues, bindings). AMQP 1.0 delegates those to the broker's native API.
