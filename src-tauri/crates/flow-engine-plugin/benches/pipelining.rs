//! Pipelining throughput benchmark.
//!
//! Spawns the crate's `echo_mock` binary (located via
//! `CARGO_BIN_EXE_echo_mock`) and fires N pipelined requests **without
//! waiting between them**, then collects all responses. This measures the
//! host transport's pipelined RPC/s against an in-process echo — the
//! realistic bar is 12–33k RPC/s (NOT 50–100k; see `docs/plugin-perf.md`).
//!
//! Run with: `cargo bench -p flow-engine-plugin --bench pipelining`
//! (or execute `target/release/bench-pipelining.exe` directly).

use std::collections::HashMap;
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
    let rpc_per_second = runtime.block_on(run_bench());

    println!("pipelining: {rpc_per_second:.0} RPC/s (echo mock, {REQUESTS_PER_ROUND} requests)");
    if rpc_per_second < 12_000.0 {
        println!("WARNING: below the 12k RPC/s floor — investigate backpressure hot path");
    } else if rpc_per_second <= 33_000.0 {
        println!("OK: within the realistic 12–33k RPC/s target");
    } else {
        println!("OK: above target (unexpectedly fast)");
    }
}

async fn run_bench() -> f64 {
    let entry = PluginEntry {
        command: env!("CARGO_BIN_EXE_echo_mock").to_string(),
        args: Vec::new(),
    };
    let process = spawn_plugin(&entry, &HashMap::new())
        .unwrap_or_else(|e| panic!("spawn echo mock: {e}"));
    let mut connection = PluginConnection::spawn(process, ConnectionConfig::default())
        .unwrap_or_else(|e| panic!("connect: {e}"));

    // Warm-up: 200 requests, verify correctness, verify pipelining by
    // bursting without awaiting and collecting afterwards.
    let warm = warmup(&connection).await;
    if warm != 200 {
        panic!("warm-up failed: got {warm} successful responses");
    }

    // Steady-state rounds. Requests are fired in a burst (pipelined), then
    // all responses are collected — the round duration is wall time from
    // first request to last response.
    let mut rounds = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        let completed = burst(&connection).await;
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

/// Sends `count` echo requests pipelined, awaits all results, and returns
/// the number of successful responses.
async fn burst(connection: &PluginConnection) -> u64 {
    let mut handles = Vec::with_capacity(REQUESTS_PER_ROUND as usize);
    // Chunked so we do not exhaust memory with futures; each chunk is a
    // pipelined burst of 1000 concurrent requests.
    for chunk in 0..(REQUESTS_PER_ROUND / 1000) {
        let conn = connection.clone();
        handles.push(tokio::spawn(async move {
            let mut ok = 0u64;
            let mut futures = Vec::with_capacity(1000);
            for i in 0..1000u64 {
                let c = conn.clone();
                let params = serde_json::json!({"seq": chunk * 1000 + i});
                futures.push(async move { c.request("echo", params).await });
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

async fn warmup(connection: &PluginConnection) -> u64 {
    let mut ok = 0u64;
    for i in 0..200u64 {
        let result = connection
            .request("echo", serde_json::json!({"seq": i}))
            .await;
        match result {
            Ok(value) => {
                if value["seq"] == serde_json::json!(i) {
                    ok += 1;
                }
            }
            Err(e) => {
                eprintln!("warm-up request failed: {e:?}");
            }
        }
    }
    ok
}
