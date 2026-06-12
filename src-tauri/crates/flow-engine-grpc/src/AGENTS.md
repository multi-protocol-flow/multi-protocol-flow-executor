# flow-engine-grpc

**Layer:** 1 (Protocol) | **Files:** 12 | **LOC:** ~7,141 | **Key deps:** tonic 0.12, prost, protox, prost-reflect

## Overview

Runtime gRPC client crate. No compile-time codegen. Parses `.proto` files at runtime via protox + prost-reflect, builds dynamic gRPC calls through tonic's channel stack.

## Node Types (3)

| Node | Executor | Purpose |
|------|----------|---------|
| `GrpcConnect` | `grpc_connect_executor.rs` (477 LOC) | Create connection, run reflection, store channel in context |
| `GrpcCall` | `grpc_call_executor.rs` (1,838 LOC) | Execute RPC with 4 streaming modes |
| `GrpcClose` | `grpc_close_executor.rs` (298 LOC) | Drop channel, clean up connection state |

## Key Modules

| File | LOC | Responsibility |
|------|-----|----------------|
| `grpc_connection_manager.rs` | 1,152 | Connection pool (address + TLS config keyed), service reflection cache |
| `grpc_call_executor.rs` | 1,838 | Unary / ServerStreaming / ClientStreaming / BidiStreaming dispatch, request serialization, response deserialization, streaming iteration |
| `proto_parser.rs` | 959 | `.proto` file parsing, symbol resolution, message descriptor lookup |
| `reflection.rs` | 788 | gRPC Server Reflection client, enumerates services/methods from live server |
| `plugin.rs` | 565 | `NodePlugin` trait impls for all 3 node types, registers into executor plugin system |
| `tls.rs` | 479 | TLS config builder: plaintext, standard (native roots), skip-verify (dangerous) |
| `error_detail.rs` | 451 | Decode `google.rpc.Status` + `google.protobuf.Any` detail payloads from RPC errors |
| `codec.rs` | 56 | Request/response prost-reflect codec wrappers |
| `helpers.rs` | 50 | Path-to-descriptor resolution, JSON-to-dynamic-value conversion, metadata merge |

## Streaming Modes (grpc_call_executor.rs)

- **Unary**: single request, single response
- **ServerStreaming**: single request, stream of responses
- **ClientStreaming**: stream of requests, single response
- **BidiStreaming**: bidirectional stream of messages

## TLS Support

Three modes configured via `GrpcConnect` node data: `plaintext` (insecure), `standard` (native CA roots), `skip-verify` (no certificate validation, for internal/test).

## Dependencies

- `flow-engine-core` (Layer 0) — Node types, FlowContext, variable system
- Channel creation delegates to tonic's `Endpoint::from_static` with optional `tls_config()`
