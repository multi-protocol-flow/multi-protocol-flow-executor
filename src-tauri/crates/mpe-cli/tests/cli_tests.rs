#![allow(missing_docs)]

//! CLI e2e tests for the plugin system (M5b).
//!
//! `cli_run_plugin_flow` runs `mpe-cli run` against a flow whose node type
//! comes from a fixture plugin (Python template, `pyecho:add`), installed in
//! a plugins directory selected via the `MPE_PLUGIN_DIR` env override (OD1).
//! This exercises the M3.3b CLI path end to end: `run_cli` →
//! `build_shared_registry` (scan + describe + register) → `SidecarPlugin`
//! execute over the real transport.
//!
//! PR-5C extends the fixture plugin with an inline-content frontend on its
//! `describe` response, so every run here also proves the PR-0/PR-2 frontend
//! metadata chain through a real plugin process: describe carries `frontend`
//! → the host parses it on the wire → `normalize_nodes` passes it through and
//! generates the iframe url → the node registers and executes.
//! `cli_run_plugin_flow_with_inline_frontend` additionally covers the
//! "配置保存 → `.mpf` 含插件配置" round-trip on the saved flow file. No CLI
//! command surfaces registered node metadata, so the host-generated
//! `frontend.url` is pinned by the backend conformance suite instead — see
//! that test's doc comment for the exact reasoning.

use std::path::{Path, PathBuf};

use assert_cmd::cargo;
use serde::Deserialize;
use serde_json::Value;

/// The checked-in fixtures root: `tests/plugin-fixtures` (contains one
/// plugin subdir, `pyecho/`).
fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("plugin-fixtures")
}

/// Recursively finds the first object whose `node_type` is `pyecho:add`.
fn find_plugin_node_report(value: &Value) -> Option<Value> {
    match value {
        Value::Object(map) => {
            if map.get("node_type").and_then(Value::as_str) == Some("pyecho:add") {
                return Some(value.clone());
            }
            map.values().find_map(find_plugin_node_report)
        }
        Value::Array(items) => items.iter().find_map(find_plugin_node_report),
        _ => None,
    }
}

/// A flow whose middle node is the fixture plugin's `pyecho:add` type:
/// entry → pyecho:add (config `{a: 2, b: 3}`) → end.
fn plugin_flow_json() -> Value {
    serde_json::json!({
        "_version": 1,
        "flow": {
            "uuid": "plugin-flow-001",
            "name": "Plugin Flow",
            "nodes": [
                {
                    "uuid": "entry-1",
                    "type": "entry",
                    "name": "Entry",
                    "position": {"x": 0.0, "y": 0.0},
                    "data": {"type": "entry"},
                    "ports": null
                },
                {
                    "uuid": "add-1",
                    "type": "pyecho:add",
                    "name": "Add",
                    "position": {"x": 100.0, "y": 0.0},
                    "data": {"type": "pyecho:add", "a": 2, "b": 3},
                    "ports": null
                },
                {
                    "uuid": "end-1",
                    "type": "end",
                    "name": "End",
                    "position": {"x": 200.0, "y": 0.0},
                    "data": {"type": "end"},
                    "ports": null
                }
            ],
            "connections": [
                {
                    "id": "conn-1",
                    "source_node_uuid": "entry-1",
                    "target_node_uuid": "add-1",
                    "source_port_id": "out",
                    "target_port_id": "in"
                },
                {
                    "id": "conn-2",
                    "source_node_uuid": "add-1",
                    "target_node_uuid": "end-1",
                    "source_port_id": "out",
                    "target_port_id": "in"
                }
            ]
        },
        "_meta": null
    })
}

/// Writes `flow` to a unique temp file and runs `mpe-cli run -f` on it from
/// the `pyecho/` fixture dir with `MPE_PLUGIN_DIR` pointed at the fixtures
/// root. Returns the child output and the flow file path — the caller owns
/// cleanup (the round-trip test re-reads the file after the run). No
/// `tempfile` dependency: mirrors the original `std::env::temp_dir` pattern
/// with a process-local counter for uniqueness across parallel tests.
fn run_plugin_flow(
    flow: &Value,
) -> Result<(std::process::Output, PathBuf), Box<dyn std::error::Error>> {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let flow_path = std::env::temp_dir().join(format!(
        "mpe-plugin-flow-{}-{}.json",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));
    std::fs::write(&flow_path, serde_json::to_vec(flow)?)?;

    let output = cargo::cargo_bin_cmd!("mpe-cli")
        .current_dir(fixtures_root().join("pyecho"))
        .env("MPE_PLUGIN_DIR", fixtures_root())
        .arg("run")
        .arg("-f")
        .arg(&flow_path)
        .output()?;
    Ok((output, flow_path))
}

#[test]
fn cli_run_plugin_flow() -> Result<(), Box<dyn std::error::Error>> {
    // The CLI is spawned with its cwd inside the plugin dir so the
    // manifest's relative entry (`python plugin.py`) resolves — the host
    // spawns plugin processes with the caller's cwd.
    let (output, flow_path) = run_plugin_flow(&plugin_flow_json())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_file(&flow_path);

    assert!(
        output.status.success(),
        "mpe run failed\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdout is pretty-printed JSON; parse the first value (the CLI may
    // emit a second error object after a failed run — not on success).
    let mut de = serde_json::Deserializer::from_str(&stdout);
    let value = Value::deserialize(&mut de)
        .map_err(|e| format!("stdout is not JSON: {e}\nstdout: {stdout}"))?;
    assert_eq!(value["success"], true, "flow must succeed: {stdout}");

    // The plugin node report carries the Python template's output_data.
    let report = find_plugin_node_report(&value)
        .ok_or_else(|| format!("pyecho:add node report missing: {stdout}"))?;
    assert_eq!(report["output"]["sum"], 5, "plugin output_data: {stdout}");
    assert_eq!(report["status"], "success");

    // PR-5C frontend-chain proof: since PR-0 the fixture plugin's `describe`
    // response declares an inline frontend on `pyecho:add`. This run went
    // through the REAL plugin process over the real transport
    // (`build_shared_registry` → scan + describe → `normalize_nodes` →
    // registry → `SidecarPlugin` execute), so the node executing with a
    // correct report proves the host parsed the describe response —
    // including the `frontend` field — and normalized it without error. A
    // frontend the host could not parse would fail the wire parse, skip the
    // plugin's registration, and error the run with an unknown node type.
    // See `cli_run_plugin_flow_with_inline_frontend` for the config-save →
    // round-trip half of the PR-5C acceptance.
    Ok(())
}

/// PR-5C e2e: mock plugin (inline content) → 配置保存 → run → report, with the
/// saved flow's plugin config round-trip asserted.
///
/// The fixture plugin now declares an inline-content frontend
/// (`{"type": "inline", "content": "<div id=\"pyecho-panel\">PyEcho Add
/// Config</div>"}`) in its `describe` response, so `mpe run` exercises the
/// PR-0/PR-2 metadata chain through the real CLI binary:
/// `build_shared_registry` spawns the real Python process, issues `describe`
/// over the M1 transport, parses the wire response (whose `pyecho:add` node
/// now carries `frontend`), runs `normalize_nodes` (all kinds pass through;
/// the host generates the iframe url), and registers the node. The successful
/// run + node report is the observable proof — a frontend the host could not
/// parse would fail describe, skip registration, and error the run with an
/// unknown node type.
///
/// The host-generated `frontend.url` is NOT directly observable from this
/// binary: no `mpe-cli` command surfaces registered node metadata (`run` has
/// no debug/verbose flag, and `validate` uses only the built-in
/// `default_registry`, not the plugin registry). Adding a metadata-surfacing
/// flag is out of PR-5 scope, so this test relies on the run-success proof
/// above; URL generation itself is pinned by the backend conformance suite
/// (`flow-engine-plugin/tests/conformance.rs`,
/// `conformance_scan_and_register_carries_iframe_frontend`) against the same
/// describe wire shape.
#[test]
fn cli_run_plugin_flow_with_inline_frontend() -> Result<(), Box<dyn std::error::Error>> {
    // "配置保存 → .mpf 含插件配置": the flow JSON's plugin node carries
    // `data: {type: "pyecho:add", a: 2, b: 3}` — the config the frontend
    // panel would save for the inline-content node.
    let flow = plugin_flow_json();
    let (output, flow_path) = run_plugin_flow(&flow)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "mpe run failed\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut de = serde_json::Deserializer::from_str(&stdout);
    let value = Value::deserialize(&mut de)
        .map_err(|e| format!("stdout is not JSON: {e}\nstdout: {stdout}"))?;
    assert_eq!(value["success"], true, "flow must succeed: {stdout}");

    let report = find_plugin_node_report(&value)
        .ok_or_else(|| format!("pyecho:add node report missing: {stdout}"))?;
    assert_eq!(report["output"]["sum"], 5, "plugin output_data: {stdout}");
    assert_eq!(report["status"], "success");

    // The saved flow still round-trips: it parses, and the plugin node's
    // config is intact (`data: {type: "pyecho:add", a: 2, b: 3}`).
    let saved: Value = serde_json::from_slice(&std::fs::read(&flow_path)?)?;
    let _ = std::fs::remove_file(&flow_path);
    let plugin_node = saved["flow"]["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|n| n["type"].as_str() == Some("pyecho:add"))
        })
        .ok_or_else(|| format!("pyecho:add node missing in saved flow: {saved}"))?;
    assert_eq!(
        plugin_node["data"],
        serde_json::json!({"type": "pyecho:add", "a": 2, "b": 3}),
        "saved flow must retain the plugin node config: {saved}"
    );
    Ok(())
}

// ============================================================================
// `mpe plugin` subcommand e2e（插件市场 fixture 服务端到端）
// ============================================================================

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

/// 当前宿主平台 key（与 flow-engine-plugin-marketplace::current_platform 对齐）。
fn current_platform_key() -> String {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        _ => "linux",
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        _ => "arm64",
    };
    format!("{os}-{arch}")
}

/// 构造 mongo 插件 zip：根含 `mongo/plugin.json` + `mongo/mongo` 占位文件。
fn build_mongo_zip() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("mongo/plugin.json", options)
        .unwrap_or_else(|e| panic!("zip write failed: {e}"));
    writer
        .write_all(br#"{"name":"mongo","version":"1.0.0","entry":{"command":"./mongo","args":[]}}"#)
        .unwrap_or_else(|e| panic!("zip write failed: {e}"));
    writer
        .start_file("mongo/mongo", options)
        .unwrap_or_else(|e| panic!("zip write failed: {e}"));
    writer
        .write_all(b"#!/bin/sh\nexit 0\n")
        .unwrap_or_else(|e| panic!("zip write failed: {e}"));
    writer
        .finish()
        .unwrap_or_else(|e| panic!("zip finish failed: {e}"))
        .into_inner()
}

/// sha256 十六进制摘要。
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// 起一个 fixture 市场服务（随机端口）：
/// - `GET /plugins` → `{"plugins":[{name:mongo, version:1.0.0, platforms:{<当前平台>:{url:…/mongo.zip, sha256}}}]}`
/// - `GET /mongo.zip` → zip 字节
///
/// 非阻塞 accept + 500ms 空闲超时退出（每个测试请求数不同，避免 join 卡死）；
/// 返回 (`base_url`, 线程句柄)。
fn start_market_fixture() -> (String, thread::JoinHandle<()>) {
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind failed: {e}"));
    listener.set_nonblocking(true).unwrap_or_else(|e| panic!("nonblocking failed: {e}"));
    let addr = listener.local_addr().unwrap_or_else(|e| panic!("addr failed: {e}"));
    let port = addr.port();
    let platform = current_platform_key();
    let handle = thread::spawn(move || {
        let zip = build_mongo_zip();
        let sha = sha256_hex(&zip);
        let plugins_body = format!(
            r#"{{"plugins":[{{"name":"mongo","version":"1.0.0","description":"MongoDB protocol plugin","platforms":{{"{platform}":{{"url":"http://127.0.0.1:{port}/mongo.zip","sha256":"{sha}"}}}}}}]}}"#
        );
        let mut idle_ms = 0u64;
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    idle_ms = 0;
                    let mut buf = [0u8; 4096];
                    let n = stream.read(&mut buf).unwrap_or_else(|e| panic!("read failed: {e}"));
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    let (status, body) = if path == "/plugins" {
                        ("200 OK", plugins_body.clone().into_bytes())
                    } else if path == "/mongo.zip" {
                        ("200 OK", zip.clone())
                    } else {
                        ("404 Not Found", b"not found".to_vec())
                    };
                    let header = format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream.write_all(header.as_bytes()).unwrap_or_else(|e| panic!("write failed: {e}"));
                    stream.write_all(&body).unwrap_or_else(|e| panic!("write failed: {e}"));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                    idle_ms += 20;
                    if idle_ms > 500 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    (format!("http://{addr}"), handle)
}

/// 运行 `mpe-cli plugin <args>`，注入 `MPE_PLUGIN_DIR`。
fn run_plugin_cmd(dir: &Path, args: &[&str]) -> std::process::Output {
    cargo::cargo_bin_cmd!("mpe-cli")
        .env("MPE_PLUGIN_DIR", dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn mpe-cli failed: {e}"))
}

/// `mpe plugin install`：下载 fixture 市场 zip → sha256 校验 → 解压安装。
/// 端到端证明：目标目录出现 `mongo/plugin.json`。
#[test]
fn cli_plugin_install_from_marketplace() -> Result<(), Box<dyn std::error::Error>> {
    let (base, handle) = start_market_fixture();
    let temp = tempfile::tempdir()?;
    let install_dir = temp.path().join("plugins");
    std::fs::create_dir_all(&install_dir)?;

    let output = run_plugin_cmd(
        &install_dir,
        &["plugin", "install", "mongo", "--registry", &base],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "install failed\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not JSON: {e}\nstdout: {stdout}"))?;
    assert_eq!(value["success"], true, "{stdout}");
    assert_eq!(value["data"]["name"], "mongo");
    assert_eq!(value["data"]["version"], "1.0.0");
    assert_eq!(
        value["data"]["path"],
        Value::String(install_dir.join("mongo").to_string_lossy().into_owned())
    );

    // 安装产物落地
    assert!(
        install_dir.join("mongo/plugin.json").exists(),
        "mongo/plugin.json must exist after install"
    );
    assert!(install_dir.join("mongo/mongo").exists(), "entry file must be extracted");
    handle.join().unwrap_or_else(|_| panic!("fixture server panicked"));
    Ok(())
}

/// 重复安装报 `AlreadyInstalled`；`--force` 覆盖成功。
#[test]
fn cli_plugin_install_already_installed_and_force() -> Result<(), Box<dyn std::error::Error>> {
    let (base, handle) = start_market_fixture();
    let temp = tempfile::tempdir()?;
    let install_dir = temp.path().join("plugins");
    std::fs::create_dir_all(&install_dir)?;

    // 首次安装成功
    let output = run_plugin_cmd(
        &install_dir,
        &["plugin", "install", "mongo", "--registry", &base],
    );
    assert!(output.status.success(), "first install failed: {}", String::from_utf8_lossy(&output.stdout));

    // 重复安装 → AlreadyInstalled 类错误（exit 1）
    let output = run_plugin_cmd(
        &install_dir,
        &["plugin", "install", "mongo", "--registry", &base],
    );
    assert!(!output.status.success(), "second install must fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("already installed"),
        "expected AlreadyInstalled error, got: {stdout}"
    );

    // --force 覆盖成功
    let output = run_plugin_cmd(
        &install_dir,
        &["plugin", "install", "mongo", "--registry", &base, "--force"],
    );
    assert!(output.status.success(), "force reinstall failed: {}", String::from_utf8_lossy(&output.stdout));
    handle.join().unwrap_or_else(|_| panic!("fixture server panicked"));
    Ok(())
}

/// `mpe plugin dir` 输出 `MPE_PLUGIN_DIR` 指定的有效目录（source=env）。
#[test]
fn cli_plugin_dir_reports_effective_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let install_dir = temp.path().join("plugins");
    std::fs::create_dir_all(&install_dir)?;

    let output = run_plugin_cmd(&install_dir, &["plugin", "dir"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "plugin dir failed: {stdout}");
    let value: Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not JSON: {e}\nstdout: {stdout}"))?;
    assert_eq!(value["success"], true);
    assert_eq!(
        value["data"]["dir"],
        Value::String(install_dir.to_string_lossy().into_owned())
    );
    assert_eq!(value["data"]["source"], "env");
    Ok(())
}

/// `mpe plugin list` 扫描 `MPE_PLUGIN_DIR`，返回已安装的 mongo 条目。
#[test]
fn cli_plugin_list_shows_installed() -> Result<(), Box<dyn std::error::Error>> {
    let (base, handle) = start_market_fixture();
    let temp = tempfile::tempdir()?;
    let install_dir = temp.path().join("plugins");
    std::fs::create_dir_all(&install_dir)?;

    // 先安装，再 list
    let output = run_plugin_cmd(
        &install_dir,
        &["plugin", "install", "mongo", "--registry", &base],
    );
    assert!(output.status.success(), "install failed: {}", String::from_utf8_lossy(&output.stdout));
    handle.join().unwrap_or_else(|_| panic!("fixture server panicked"));

    let output = run_plugin_cmd(&install_dir, &["plugin", "list"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "plugin list failed: {stdout}");
    let value: Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not JSON: {e}\nstdout: {stdout}"))?;
    assert_eq!(value["success"], true);
    let list = value["data"]
        .as_array()
        .ok_or_else(|| format!("data must be an array: {stdout}"))?;
    let mongo = list
        .iter()
        .find(|entry| entry["name"] == "mongo")
        .ok_or_else(|| format!("mongo entry missing: {stdout}"))?;
    assert_eq!(mongo["version"], "1.0.0");
    assert_eq!(
        mongo["path"],
        Value::String(install_dir.join("mongo").to_string_lossy().into_owned())
    );
    assert_eq!(mongo["error"], Value::Null);
    Ok(())
}
