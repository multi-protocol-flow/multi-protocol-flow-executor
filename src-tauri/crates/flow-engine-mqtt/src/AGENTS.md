# flow-engine-mqtt

**Layer:** 1 (Protocol) | **Depends on:** flow-engine-core | **Feature:** `protocol-mqtt`

## OVERVIEW

MQTT 5.0 protocol crate. Uses `rumqttc-next` 0.30 with rustls TLS and native WebSocket transport. Provides 7 node types for MQTT connection lifecycle and messaging. Supports TCP (`mqtt://`), TLS (`mqtts://`), WebSocket (`ws://`), and secure WebSocket (`wss://`) transports with configurable TLS modes (skip verify, custom CA). LWT (Last Will) with full MQTT 5.0 properties is supported on connect. 11 files, ~6,600 lines.

## STRUCTURE

| File | Lines | Role |
|------|-------|------|
| `lib.rs` | 25 | Module declarations, re-exports `MqttConnection`, `MqttConnectionManager`, `MqttPluginGroup` |
| `mqtt_client.rs` | 426 | Client factory — URL parsing, transport selection, TLS config, LWT, MQTT 5.0 options |
| `mqtt_connection_manager.rs` | 804 | Connection pool — background eventloop polling, per-flow connection lifecycle |
| `mqtt_connect_executor.rs` | 553 | `mqtt_connect` — create client, handshake, store connection in manager |
| `mqtt_publish_executor.rs` | 812 | `mqtt_publish` — publish to topic with QoS/retain/PayloadFormatIndicator |
| `mqtt_subscribe_executor.rs` | 785 | `mqtt_subscribe` — subscribe with topic filter, collect messages (first/all) |
| `mqtt_unsubscribe_executor.rs` | 374 | `mqtt_unsubscribe` — unsubscribe from topic filter |
| `mqtt_disconnect_executor.rs` | 311 | `mqtt_disconnect` — cleanly disconnect and remove connection |
| `mqtt_wait_message_executor.rs` | 702 | `mqtt_wait_message` — block on topic filter with timeout, single or batch |
| `mqtt_request_executor.rs` | 883 | `mqtt_request` — combined publish + subscribe wait (request/response pattern) |
| `plugin.rs` | 958 | 7 `NodePlugin` impls + `MqttPluginGroup` (batch register) + unit tests |

## NODE TYPES

| Type | Ports | Port logic |
|------|-------|------------|
| `mqtt_connect` | true / false | `$.success` bool |
| `mqtt_publish` | true / false | `$.success` bool |
| `mqtt_subscribe` | true / false | `$.success` bool |
| `mqtt_unsubscribe` | true / false | `$.success` bool |
| `mqtt_disconnect` | out (single) | Always `"out"` |
| `mqtt_wait_message` | true / false | `$.success` bool |
| `mqtt_request` | true / false | `$.success` bool |

All nodes share color `#06b6d4` (cyan) and category `"mqtt"`.

## TRANSPORT & TLS

| URL Scheme | Transport | TLS |
|------------|-----------|-----|
| `mqtt://host:port` | TCP | None |
| `mqtts://host:port` | TCP | rustls + webpki-roots (skip_verify opt) |
| `ws://host:port/path` | WebSocket | None |
| `wss://host:port/path` | WebSocket | rustls + webpki-roots (skip_verify opt) |

## KEY PATTERNS

- **Request/Response:** `mqtt_request` publishes on `request_topic` and waits on `response_topic` with timeout, implementing the MQTT 5.0 request-response pattern.
- **Connection Management:** `MqttConnectionManager` is registered as a context provider (`"mqtt"`) and shared across all MQTT executors within a flow execution.
- **Plugin Registration:** `MqttPluginGroup::register_all(&mut registry)` registers all 7 plugins at once.
