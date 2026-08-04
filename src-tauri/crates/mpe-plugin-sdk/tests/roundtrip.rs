//! Integration tests that spawn the compiled `mpe_demo_plugin` binary and
//! drive it over piped stdio exactly like the host does: JSON-RPC 2.0
//! requests on stdin, framed responses on stdout.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use mpe_plugin_sdk::types::NodeDescription;

/// Path to the compiled sample plugin (set by cargo for `[[bin]]` targets).
const DEMO_BIN: &str = env!("CARGO_BIN_EXE_mpe_demo_plugin");

/// Spawns the demo plugin with piped stdio.
fn spawn_plugin() -> Result<(Child, ChildStdin, BufReader<ChildStdout>), Box<dyn std::error::Error>>
{
    let mut child = Command::new(DEMO_BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "plugin stdin not available".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "plugin stdout not available".to_string())?;
    Ok((child, stdin, BufReader::new(stdout)))
}

/// Reads one LF-delimited frame (CRLF-tolerant) from the plugin.
fn read_frame(stdout: &mut BufReader<ChildStdout>) -> Result<String, Box<dyn std::error::Error>> {
    let mut line = String::new();
    let n = stdout.read_line(&mut line)?;
    if n == 0 {
        return Err("plugin stdout closed unexpectedly".into());
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// Sends requests (one per element; `\r\n` used when the element is flagged)
/// and collects the same number of **responses** (frames carrying an `id`),
/// keyed by request id. Fire-and-forget notification frames (e.g. the demo's
/// `log` notification) are read and discarded.
fn exchange(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    requests: &[(&str, bool)],
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    for (request, crlf) in requests {
        let delimiter = if *crlf { "\r\n" } else { "\n" };
        stdin.write_all(request.as_bytes())?;
        stdin.write_all(delimiter.as_bytes())?;
    }
    stdin.flush()?;
    let mut responses = Vec::with_capacity(requests.len());
    while responses.len() < requests.len() {
        let frame = read_frame(stdout)?;
        let value: serde_json::Value = serde_json::from_str(&frame)?;
        if value.get("id").is_some() {
            responses.push(value);
        }
    }
    Ok(responses)
}

/// Sends a `describe` request and asserts the response parses as node
/// descriptions; then an `execute` request (echo) and asserts the config
/// round-trips; then a `validate` request and asserts the default success.
#[test]
fn sdk_plugin_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (mut child, mut stdin, mut stdout) = spawn_plugin()?;

    // Pipelined: all three requests are written before any response is read,
    // proving the runtime processes requests concurrently and correlates by
    // id (responses may arrive in any order).
    let requests = [
        (
            r#"{"jsonrpc":"2.0","id":1,"method":"describe","params":{}}"#,
            false,
        ),
        (
            r#"{"jsonrpc":"2.0","id":2,"method":"execute","params":{"config":{"url":"https://example.com"}}}"#,
            true, // CRLF frame: the runtime must tolerate it.
        ),
        (
            r#"{"jsonrpc":"2.0","id":3,"method":"validate","params":{"config":{}}}"#,
            false,
        ),
    ];
    let responses = exchange(&mut stdin, &mut stdout, &requests)?;
    assert_eq!(responses.len(), 3);

    for response in &responses {
        let id = response["id"]
            .as_u64()
            .ok_or_else(|| format!("missing id in {response}"))?;
        match id {
            1 => {
                let nodes: Vec<NodeDescription> =
                    serde_json::from_value(response["result"].clone())?;
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].type_id, "demo:echo");
                assert_eq!(nodes[0].display_name, "Demo Echo");
            }
            2 => {
                assert_eq!(response["result"]["success"], true);
                assert_eq!(
                    response["result"]["output_data"]["url"],
                    "https://example.com"
                );
                assert!(response["result"]["variable_updates"].is_object());
            }
            3 => {
                assert_eq!(response["result"]["valid"], true);
            }
            other => return Err(format!("unexpected response id {other}").into()),
        }
    }

    drop(stdin); // half-close stdin: the plugin should exit cleanly on EOF.
    let status = child.wait()?;
    assert!(status.success(), "plugin exited with {status:?}");
    Ok(())
}

/// An unknown method must produce a JSON-RPC `-32601` error response.
#[test]
fn sdk_unknown_method_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let (mut child, mut stdin, mut stdout) = spawn_plugin()?;

    let responses = exchange(
        &mut stdin,
        &mut stdout,
        &[(
            r#"{"jsonrpc":"2.0","id":7,"method":"bogus_method","params":{}}"#,
            false,
        )],
    )?;
    assert_eq!(responses.len(), 1);
    let response = &responses[0];
    assert_eq!(response["id"], 7);
    assert_eq!(response["error"]["code"], -32601);
    assert_eq!(response["error"]["message"], "Method not found");

    drop(stdin);
    let status = child.wait()?;
    assert!(status.success(), "plugin exited with {status:?}");
    Ok(())
}
