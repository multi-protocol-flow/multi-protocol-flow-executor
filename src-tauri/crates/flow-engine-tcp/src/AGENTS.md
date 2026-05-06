# TCP/UDP Crate

**Layer:** 1 → flow-engine-core

## Overview

TCP + UDP protocol clients with TLS support, framing protocols, and message recording.
8 node types: 4 TCP nodes + 4 UDP nodes.

## Structure

```
src/
├── lib.rs                      # Crate entry, module exports
├── plugin.rs                   # 8 NodePlugin implementations
├── tls.rs                      # TLS connector (rustls)
├── framing.rs                  # Frame protocols (5 modes)
├── encoding.rs                 # Base64 + UTF-8 utilities
├── recording.rs                # Message recording
├── parser.rs, templates.rs     # Parser utilities & flow templates
├── tcp_connection_manager.rs   # Connection pool for TCP
├── tcp_connect_executor.rs     # Node: tcp_connect
├── tcp_send_executor.rs        # Node: tcp_send
├── tcp_receive_executor.rs     # Node: tcp_receive
├── tcp_close_executor.rs       # Node: tcp_close
├── udp_socket_manager.rs       # Socket management for UDP
├── udp_bind_executor.rs        # Node: udp_bind
├── udp_send_to_executor.rs     # Node: udp_send_to
├── udp_recv_from_executor.rs   # Node: udp_recv_from
└── udp_close_executor.rs       # Node: udp_close
```

## Node Types

### TCP Nodes

| Node | Type ID | File | Purpose |
|------|---------|------|---------|
| TCP Connect | `tcp_connect` | `tcp_connect_executor.rs` | Connect to host:port, optional TLS |
| TCP Send | `tcp_send` | `tcp_send_executor.rs` | Send data with framing |
| TCP Receive | `tcp_receive` | `tcp_receive_executor.rs` | Receive data with framing |
| TCP Close | `tcp_close` | `tcp_close_executor.rs` | Close connection |

### UDP Nodes

| Node | Type ID | File | Purpose |
|------|---------|------|---------|
| UDP Bind | `udp_bind` | `udp_bind_executor.rs` | Bind to local address/port |
| UDP SendTo | `udp_send_to` | `udp_send_to_executor.rs` | Send to target address |
| UDP RecvFrom | `udp_recv_from` | `udp_recv_from_executor.rs` | Receive with peer info |
| UDP Close | `udp_close` | `udp_close_executor.rs` | Close socket |

## Conventions

### Framing Modes (`framing.rs`)

5 frame protocols supported:

1. **LengthPrefix** - N-byte length prefix (big-endian)
2. **Delimiter** - Boundary marker (e.g., `\r\n`)
3. **FixedLength** - Fixed-size frames
4. **Timeout** - Read until timeout
5. **Manual** - Raw data passthrough

### TLS Config (`tls.rs`)

3 TLS modes:
- `plaintext` - No TLS
- `standard` - Verify with webpki-roots
- `skip-verify` - Skip cert verification (dev only)

Custom CA cert support via `tls_ca_cert` field.

### Encoding (`encoding.rs`)

- Data encoded as Base64 for JSON output
- UTF-8 text preview (truncated at 1024 bytes)
- Standard binary output: `{data, encoding, size, text_preview}`

### Connection Management

- TCP: `TcpConnectionManager` - connection pool keyed by node UUID
- UDP: `UdpSocketManager` - socket pool keyed by node UUID
- Both stored in `ExecutionContext` for cross-node reuse

## Where to Look

| Task | File |
|------|------|
| Add TCP node | `plugin.rs` → impl NodePlugin + executor |
| Modify framing | `framing.rs` - TcpFramingEngine |
| Add TLS feature | `tls.rs` - create_tls_connector* |
| Change encoding | `encoding.rs` - build_binary_output |
| Connection pool | `tcp_connection_manager.rs` |
| UDP multicast | `udp_bind_executor.rs` - multicast options |
| Message recording | `recording.rs` - RecordingSession |

## Testing

```bash
cd src-tauri
cargo test -p flow-engine-tcp
cargo clippy -p flow-engine-tcp
```
