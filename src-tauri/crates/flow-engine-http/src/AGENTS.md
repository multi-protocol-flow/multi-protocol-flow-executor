# flow-engine-http

**Generated:** 2026-06-12 | **Branch:** dev | **Layer:** L1 (Protocol)

## OVERVIEW

Layer 1 protocol crate. HTTP client built on hyper 1.x + tokio-rustls (rustls with ring) supporting HTTP/1.1 and HTTP/2 via ALPN. Provides DNS/TCP/TLS/TTFB/download phase-level timing. Contains `HttpNodeExecutor` (HTTP node execution) and `HttpPlugin` (NodePlugin trait implementation). 13 source files, ~5,900 lines.

## STRUCTURE

```
src/
├── lib.rs               # Crate root, 3 mod declarations
├── http_executor.rs     # HTTP node executor (~2,800 lines)
├── plugin.rs            # HttpPlugin: type_id, ports, config_schema, resolve_port, execute dispatch
└── http/
    ├── mod.rs           # Re-exports public API
    ├── client.rs        # HttpClient: hyper dispatch, timing capture
    ├── request.rs       # RequestBuilder: raw HTTP byte serialization
    ├── response.rs      # Response parser: status, headers, body extraction
    ├── body.rs          # BodyType enum + multipart/form-data builder
    ├── connector.rs     # TlsOptions, TimingConnector, DNS resolution
    ├── timing.rs        # HttpTiming: DNS/TCP/TLS/TTFB/download phases
    ├── cookie.rs        # Cookie jar for session persistence
    ├── proxy.rs         # HTTP proxy via tokio-socks
    └── error.rs         # HttpError: DNS/TCP/TLS/protocol error types
```

## WHERE TO LOOK

| Task | File |
|------|------|
| HTTP node execution / variable resolution / response handling | `http_executor.rs` |
| NodePlugin registration, config schema, port resolution | `plugin.rs` |
| Low-level hyper client, request dispatch | `http/client.rs` |
| Body type construction (JSON, form-data, urlencoded, binary) | `http/body.rs` |
| TLS options, proxy configuration, stream wrapping | `http/connector.rs`, `http/proxy.rs` |
| Phase-level timing instrumentation | `http/timing.rs`, `http/connector.rs` |

## BODY TYPES

`BodyType` enum: `None`, `Body` (raw JSON string), `FormData` (multipart/form-data with boundary), `UrlEncoded` (x-www-form-urlencoded), `Binary` (base64-decoded bytes). Content-Type auto-set per type. Manual override via `content_type` config field or explicit `Content-Type` header in the headers map.

## COMPILE IMPACT

Modifying this crate triggers rebuild of `flow-engine-executor` → `flow-engine` → `app` (~3 crates). Fast iteration: `cargo build -p flow-engine-http`.
