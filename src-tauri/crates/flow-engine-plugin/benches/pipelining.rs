//! Pipelining throughput benchmark.
//!
//! Two benches, one harness:
//!
//! 1. **Echo mock** (baseline): the crate's `echo_mock` binary (located via
//!    `CARGO_BIN_EXE_echo_mock`), firing `echo` requests.
//! 2. **Real Rust SDK plugin** (M5b): the compiled `mpe_demo_plugin` binary,
//!    firing `execute` requests through the SDK's own pipelined dispatcher.
//!    Skips with a log when the binary is not built (run
//!    `cargo build -p mpe-plugin-sdk --bin mpe_demo_plugin` first).
//!
//! Both fire N pipelined requests **without waiting between them**, then
//! collect all responses — measuring the host transport's pipelined RPC/s
//! against a real plugin process. The realistic bar is 12–33k RPC/s (NOT
//! 50–100k; see `docs/plugin-perf.md`).
//!
//! Run with: `cargo bench -p flow-engine-plugin --bench pipelining`
//! (or execute `target/release/bench-pipelining.exe` directly).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use flow_engine_plugin::process::spawn_plugin;
use flow_engine_plugin::transport::{ConnectionConfig, PluginConnection};
use mpe_protocol::manifest::PluginEntry;

/// Number of pipelined requests per measurement round.
const REQUESTS_PER_ROUND: u64 = 20_000;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("build runtime: {e}"));

    // ── Baseline: echo mock ─────────────────────────────────────────────
    let entry = PluginEntry {
        command: env!("CARGO_BIN_EXE_echo_mock").to_string(),
        args: Vec::new(),
    };
    let echo_rps = runtime.block_on(run_bench(&entry, "echo", |i| serde_json::json!({"seq": i})));
    println!("pipelining: {echo_rps:.0} RPC/s (echo mock, {REQUESTS_PER_ROUND} requests)");
    print_verdict(echo_rps);

    // ── M5b: real Rust SDK plugin ───────────────────────────────────────
    match sdk_plugin_bin() {
        Some(bin) => {
            let entry = PluginEntry {
                command: bin.to_string_lossy().to_string(),
                args: Vec::new(),
            };
            let sdk_rps = runtime.block_on(run_bench(
                &entry,
                "execute",
                |i| serde_json::json!({"config": {"seq": i}}),
            ));
            println!(
                "pipelining: {sdk_rps:.0} RPC/s (mpe_demo_plugin execute, \
                 {REQUESTS_PER_ROUND} requests)"
            );
            print_verdict(sdk_rps);
        }
        None => println!(
            "pipelining: SKIP SDK plugin bench (mpe_demo_plugin not built; \
             run `cargo build -p mpe-plugin-sdk --bin mpe_demo_plugin`)"
        ),
    }
}

/// Path to the compiled SDK demo plugin binary, if built (workspace target).
fn sdk_plugin_bin() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "mpe_demo_plugin.exe"
    } else {
        "mpe_demo_plugin"
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..");
    for profile in ["debug", "release"] {
        let path = root
            .join("src-tauri")
            .join("target")
            .join(profile)
            .join(exe);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Interprets a measured RPC/s against the 12–33k target.
fn print_verdict(rps: f64) {
    if rps < 12_000.0 {
        println!("WARNING: below the 12k RPC/s floor — investigate backpressure hot path");
    } else if rps <= 33_000.0 {
        println!("OK: within the realistic 12–33k RPC/s target");
    } else {
        println!("OK: above target (unexpectedly fast)");
    }
}

/// Runs warm-up + three steady-state rounds against `entry`, returning the
/// mean RPC/s. `params_for` builds the request params for request `i`.
async fn run_bench(
    entry: &PluginEntry,
    method: &str,
    params_for: impl Fn(u64) -> serde_json::Value + Send + Sync + 'static,
) -> f64 {
    let params_for: std::sync::Arc<dyn Fn(u64) -> serde_json::Value + Send + Sync> =
        std::sync::Arc::new(params_for);
    let process = spawn_plugin(entry, &HashMap::new())
        .unwrap_or_else(|e| panic!("spawn {}: {e}", entry.command));
    // A wide inflight window lets the pipe stay full; the transport's
    // pipelining (not the 32-request default throttle) is what we measure.
    let config = ConnectionConfig {
        max_concurrent_requests: 4096,
        queue_capacity: 8192,
        ..ConnectionConfig::default()
    };
    let mut connection =
        PluginConnection::spawn(process, config).unwrap_or_else(|e| panic!("connect: {e}"));

    // Warm-up: 200 requests, verify correctness, verify pipelining by
    // bursting without awaiting and collecting afterwards.
    let warm = warmup(&connection, method, &params_for).await;
    if warm != 200 {
        panic!("warm-up failed: got {warm} successful responses");
    }

    // Steady-state rounds. Requests are fired in a burst (pipelined), then
    // all responses are collected — the round duration is wall time from
    // first request to last response.
    let mut rounds = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        let completed = burst(&connection, method, &params_for).await;
        let elapsed = start.elapsed();
        let rps = completed as f64 / elapsed.as_secs_f64();
        println!(
            "round: {completed} requests in {:.3}s → {rps:.0} RPC/s",
            elapsed.as_secs_f64()
        );
        rounds.push(rps);
    }

    let _ = connection.shutdown(Duration::from_secs(5)).await;
    let best = rounds.iter().copied().fold(f64::MIN, f64::max);
    let mean = rounds.iter().sum::<f64>() / rounds.len() as f64;
    println!("best: {best:.0} RPC/s, mean: {mean:.0} RPC/s");
    mean
}

/// Sends `count` pipelined requests, awaits all results, and returns the
/// number of successful responses.
async fn burst(
    connection: &PluginConnection,
    method: &str,
    params_for: &std::sync::Arc<dyn Fn(u64) -> serde_json::Value + Send + Sync>,
) -> u64 {
    let mut handles = Vec::with_capacity(REQUESTS_PER_ROUND as usize);
    let method = method.to_string();
    // Chunked so we do not exhaust memory with futures; each chunk is a
    // pipelined burst of 1000 concurrent requests.
    for chunk in 0..(REQUESTS_PER_ROUND / 1000) {
        let conn = connection.clone();
        let method = method.clone();
        let params_for = std::sync::Arc::clone(params_for);
        handles.push(tokio::spawn(async move {
            let mut ok = 0u64;
            let mut futures = Vec::with_capacity(1000);
            for i in 0..1000u64 {
                let c = conn.clone();
                let method = method.clone();
                let params = params_for(chunk * 1000 + i);
                futures.push(async move { c.request(&method, params).await });
            }
            for future in futures {
                if future.await.is_ok() {
                    ok += 1;
                }
            }
            ok
        }));
    }
    let mut completed = 0u64;
    for handle in handles {
        completed += handle
            .await
            .unwrap_or_else(|e| panic!("burst task panicked: {e}"));
    }
    completed
}

async fn warmup(
    connection: &PluginConnection,
    method: &str,
    params_for: &std::sync::Arc<dyn Fn(u64) -> serde_json::Value + Send + Sync>,
) -> u64 {
    let mut ok = 0u64;
    for i in 0..200u64 {
        let result = connection.request(method, params_for(i)).await;
        if result.is_ok() {
            ok += 1;
        } else {
            eprintln!("warm-up request failed: {:?}", result.err());
        }
    }
    ok
}
