//! Echo mock plugin — the conformance/bench reference implementation of the
//! plugin side of the MPE protocol.
//!
//! Reads `LF`-delimited JSON-RPC frames from stdin and answers each request
//! on stdout (pipelining naturally — every request is answered as soon as it
//! is read):
//!
//! | method        | response                                        |
//! |---------------|-------------------------------------------------|
//! | `describe`    | `result` = a `DescribeResult` (inline + iframe) |
//! | `echo`        | `result` = the request `params`                 |
//! | `fail`        | error `-32000` "mock failure"                   |
//! | `log`         | emits a `log` notification, then echoes         |
//! | `crlf`        | echoes with a `\r\n` terminator (CRLF check)    |
//! | `cancel`      | (notification from the host; ignored)           |
//! | anything else | error `-32601` "Method not found"               |
//!
//! Extra behavior driven by environment variables (used by the spawn-hygiene
//! integration tests):
//! - `MPE_ECHO_MOCK_FLOOD_STDERR=1` — write 128 KiB of junk to stderr before
//!   the loop (proves the host drains stderr so the pipe never stalls);
//! - `MPE_ECHO_MOCK_MODE=once` — write one `{"ok":true}` line and exit;
//! - `MPE_ECHO_MOCK_MODE=sleep` — sleep forever without reading stdin.

use std::io::{BufRead, BufReader, Write};

use mpe_protocol::error::JsonRpcError;
use mpe_protocol::notification::Notification;
use mpe_protocol::request::Request;

/// Bytes written to stderr when `MPE_ECHO_MOCK_FLOOD_STDERR` is set.
const STDERR_FLOOD_BYTES: usize = 128 * 1024;

fn main() {
    let mode = std::env::var("MPE_ECHO_MOCK_MODE").unwrap_or_default();
    if std::env::var("MPE_ECHO_MOCK_FLOOD_STDERR").is_ok() {
        flood_stderr();
    }
    match mode.as_str() {
        "once" => {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}\n");
            let _ = stdout.flush();
        }
        "sleep" => {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3600);
            while std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        _ => run_loop(),
    }
}

/// Writes 128 KiB of junk to stderr so a non-draining host would stall us.
fn flood_stderr() {
    let blob = vec![b'x'; STDERR_FLOOD_BYTES];
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(&blob);
    let _ = stderr.flush();
}

/// The JSON-RPC echo loop over stdin/stdout (binary mode, never text).
fn run_loop() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut line = String::new();
    loop {
        line.clear();
        let n = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(_e) => break,
        };
        if n == 0 {
            break; // stdin EOF: host is done with us
        }
        let line = line.trim_end_matches(['\n', '\r']);
        if line.is_empty() {
            continue;
        }
        let request: Request = match serde_json::from_str(line) {
            Ok(request) => request,
            Err(_parse_err) => {
                // A fire-and-forget notification (no top-level id) is not a
                // Request; acknowledge it silently. Anything else is a parse
                // error answered with a standard JSON-RPC error.
                if serde_json::from_str::<Notification>(line).is_ok() {
                    continue;
                }
                let _ = write_json(
                    &mut writer,
                    &serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": JsonRpcError::parse_error()
                    }),
                );
                continue;
            }
        };
        handle_request(&mut writer, &request);
    }
}

fn handle_request(writer: &mut impl Write, request: &Request) {
    match request.method.as_str() {
        "describe" => {
            // Object-shape describe (docs/plugin-protocol.md §9): a manifest
            // echo plus one node type per provided frontend kind. The iframe
            // node deliberately OMITS `url` so the conformance suite can prove
            // the host generates it during normalization (PR-0/PR-2).
            let _ = write_json(
                writer,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.id,
                    "result": describe_result()
                }),
            );
        }
        "echo" => {
            let _ = write_json(
                writer,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.id,
                    "result": request.params
                }),
            );
        }
        "fail" => {
            let _ = write_json(
                writer,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.id,
                    "error": { "code": -32000, "message": "mock failure" }
                }),
            );
        }
        "log" => {
            // Emit an event notification first (with correlation fields),
            // then answer the request — exercises notification demux while a
            // request is pending.
            let notification = Notification::new(
                "log",
                serde_json::json!({
                    "execution_id": "exec-1",
                    "node_instance_id": "node-1",
                    "level": "info",
                    "message": "mock log"
                }),
            );
            let _ = write_json(writer, &notification);
            let _ = write_json(
                writer,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.id,
                    "result": request.params
                }),
            );
        }
        "crlf" => {
            // CRLF terminator — Python `print()` style. The host framing
            // must strip the trailing CR.
            let payload = serde_json::json!({
                "jsonrpc": "2.0",
                "id": request.id,
                "result": request.params
            });
            let _ = writeln!(writer, "{payload}\r");
            let _ = writer.flush();
        }
        _ => {
            let _ = write_json(
                writer,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.id,
                    "error": { "code": -32601, "message": "Method not found" }
                }),
            );
        }
    }
}

/// The `describe` response payload (object shape): the manifest echo plus two
/// node types — one `inline` frontend (with content) and one `iframe`
/// frontend that omits its `url` so the host must generate it during
/// normalization. Kept as JSON literals, mirroring the other mock responses.
fn describe_result() -> serde_json::Value {
    serde_json::json!({
        "plugin": {
            "name": "demo",
            "version": "0.0.0",
            "entry": {"command": "echo_mock"}
        },
        "nodes": [
            {
                "type_id": "echo:inline",
                "display_name": "Echo Inline",
                "ports": [
                    {"id": "in", "label": "In", "kind": "in"},
                    {"id": "out", "label": "Out", "kind": "out"}
                ],
                "frontend": {"type": "inline", "content": "<p>echo</p>"}
            },
            {
                "type_id": "echo:iframe",
                "display_name": "Echo Iframe",
                "ports": [
                    {"id": "in", "label": "In", "kind": "in"},
                    {"id": "out", "label": "Out", "kind": "out"}
                ],
                "frontend": {"type": "iframe"}
            }
        ]
    })
}

fn write_json(writer: &mut impl Write, value: &impl serde::Serialize) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    writer.write_all(&bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()
}
