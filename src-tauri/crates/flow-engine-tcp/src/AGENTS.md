# flow-engine-tcp

**Layer 1 (Protocol)** | LOC: ~10,700 | 18 .rs files | Deps: tokio, rustls, tokio-rustls, base64

## OVERVIEW

TCP/UDP protocol crate providing TCP and UDP client execution with TLS, message framing, flow recording, and binary protocol parsing. 8 node types registered via `TcpPluginGroup` and `UdpPluginGroup`:

- **TCP** (4, green `#10B981`): `tcp_connect`, `tcp_send`, `tcp_receive`, `tcp_close`
- **UDP** (4, orange `#F97316`): `udp_bind`, `udp_send_to`, `udp_recv_from`, `udp_close`

All nodes have true/false output ports. The TCP connection manager (`TcpConnectionManager`) is stored as a flow-level provider accessible by all TCP send/receive/close executors in the same flow.

## STRUCTURE

| File | Responsibility |
|------|---------------|
| `lib.rs` | Module declarations (18 modules) |
| `plugin.rs` | `NodePlugin` trait impls for all 8 node types + `TcpPluginGroup`/`UdpPluginGroup` batch registration |
| `tcp_connection_manager.rs` | Connection pool (`HashMap<uuid, ActiveTcpConnection>`), `TcpProvider` trait impl, TLS connect/send/receive/close |
| `tcp_connect_executor.rs` | TCP connect node executor |
| `tcp_send_executor.rs` | TCP send node executor |
| `tcp_receive_executor.rs` | TCP receive node executor |
| `tcp_close_executor.rs` | TCP close node executor |
| `udp_socket_manager.rs` | UDP socket pool, `UdpProvider` trait impl |
| `udp_bind_executor.rs` | UDP bind node executor |
| `udp_send_to_executor.rs` | UDP sendto node executor |
| `udp_recv_from_executor.rs` | UDP recvfrom node executor |
| `udp_close_executor.rs` | UDP close node executor |
| `framing.rs` | `TcpFramingEngine` — 5 framing modes |
| `parser.rs` | `ProtocolParser` trait + 3 built-in parsers (LengthPrefix, Delimiter, TLV) |
| `encoding.rs` | Base64 encode/decode, UTF-8 text preview, `build_binary_output()` |
| `recording.rs` | `RecordingManager` + `TcpRecording` — record send/receive, save/load JSON + external .bin files |
| `tls.rs` | 3 TLS modes: plaintext, standard (webpki-roots), skip-verify (NoVerifier) |
| `templates.rs` | 5 protocol templates: Redis RESP, Custom TLV, Fixed Header, HTTP-like, Blank |

## FRAMING MODES

| Mode | Description |
|------|-------------|
| `LengthPrefix` | N-byte big-endian length prefix (1/2/4/8 bytes) |
| `Delimiter` | String delimiter boundary (e.g. `\r\n`, `<EOF>`) |
| `FixedLength` | Exact fixed-size frames (truncates or errors on mismatch) |
| `Timeout` | Read all available data until timeout |
| `Manual` | Passthrough, no framing applied |

## TLS MODES

Configured via `use_tls` + `tls_skip_verify` + optional `tls_ca_cert` (PEM):

- **Plaintext** (`use_tls=false`): Unencrypted TCP
- **Standard** (`use_tls=true`, `tls_skip_verify=false`): rustls with webpki-roots + optional custom CA
- **Skip-verify** (`use_tls=true`, `tls_skip_verify=true`): No certificate validation (dev/test only)

## ENCODING

- `data_encoding: "base64"`: Decode Base64 before sending
- `data_encoding: "text"` / empty: Send raw bytes directly
- Output format: `{ data: "<base64>", encoding: "base64", size: N, text_preview: "<utf8 or null>" }`

## WHERE TO LOOK

| Task | File |
|------|------|
| New TCP node type | `plugin.rs` + corresponding executor + config in core |
| Framing logic | `framing.rs` |
| Binary protocol parsing | `parser.rs` |
| TLS config | `tls.rs` |
| Connection pool internals | `tcp_connection_manager.rs` |
| UDP socket management | `udp_socket_manager.rs` |
| Flow recording | `recording.rs` |
| Built-in protocol templates | `templates.rs` |
| Data encoding | `encoding.rs` |
