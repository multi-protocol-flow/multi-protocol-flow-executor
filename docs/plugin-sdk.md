# MPE Plugin SDK — Author Guide

> **Status:** normative (M5). This guide covers writing plugins for the MPE
> host: the Rust SDK (first-class), the Python and Node templates (light),
> the `plugin.json` manifest, and the wire contract. The normative transport
> spec lives in `docs/plugin-protocol.md`; the overall architecture lives in
> `docs/plugin-architecture.md`.

## 1. How plugins work (30 seconds)

A plugin is a **sidecar process** that speaks JSON-RPC 2.0 over its stdin /
stdout, one JSON value per line (LF-delimited, CRLF tolerated). The host
(`flow-engine-plugin`) spawns the process, runs a `describe` handshake to
learn the node types it provides, and registers those types into the shared
`NodeRegistry` — so plugin nodes look exactly like built-in nodes to flows,
the executor, and reports.

Plugins can be written in any language; the host only requires:

- stdio in **binary mode** (no line-ending translation),
- **UTF-8** payloads (the host injects `PYTHONUTF8=1` /
  `PYTHONIOENCODING=utf-8` for interpreter plugins),
- framing: one JSON value per line, terminated by `\n` (a trailing `\r`
  before the `\n` is tolerated),
- pipelining-friendly dispatch: read the next request while the previous
  one is still executing, respond with the matching `id`.

## 2. Installation

A plugin is a **directory** inside the host plugins dir:

- Default: `get_data_dir()/plugins` (`%APPDATA%/multi-protocol-flow-executor/plugins`
  on Windows).
- Override: the `MPE_PLUGIN_DIR` environment variable (OD1).
- One plugin per subdirectory; each subdirectory needs a `plugin.json`
  manifest. The host scans synchronously at startup, describes every
  valid plugin, and **skips (never crashes on)** invalid manifests.
- A plugin that fails `describe` 3 consecutive times is **quarantined** for
  the host's lifetime (warn + skip, never wedges the host).

```
plugins/
└── myplugin/
    ├── plugin.json      # required
    ├── plugin.py        # your implementation (any language)
    └── frontend/        # P1 placeholder — no iframes in P0
```

## 3. The manifest (`plugin.json`)

```json
{
  "name": "myplugin",
  "version": "1.0.0",
  "description": "Adds two numbers",
  "min_host_version": "0.1.0",
  "entry": {
    "command": "python",
    "args": ["plugin.py"]
  },
  "env": { "MY_PLUGIN_MODE": "prod" },
  "permissions": [],
  "capabilities": { "streaming": false }
}
```

| Field | Meaning |
|-------|---------|
| `name` | Unique plugin name (convention: the directory name). |
| `version` | Semver. |
| `min_host_version` | Optional host version gate (incompatible → skip). |
| `entry` | **Command + args** — interpreter plugins are first-class (`"python"` + `["plugin.py"]`). Args are resolved against the host process's working directory; prefer absolute paths when the host cwd is unknown. |
| `env` | Extra environment variables injected at spawn (host UTF-8 env is always present on top). |
| `permissions` | Reserved; P0 ignores. |
| `capabilities.streaming` | `true` → the process stays resident (never idle-recycled); `false` → on-demand spawn + 60 s idle recycle. |

## 4. The wire contract (summary)

Full normative spec: `docs/plugin-protocol.md`. Shared wire types live in
`mpe-protocol` (host and SDK compile against the same crate so they cannot
drift).

### 4.1 `describe` handshake

The host sends `{"method":"describe","params":{}}` once per plugin, right
after spawn. Two response shapes are accepted:

1. **Object shape** (normative): the plugin echoes its manifest plus its
   node descriptions:

```json
{
  "plugin": { "name": "myplugin", "version": "1.0.0",
              "entry": {"command": "python", "args": ["plugin.py"]},
              "capabilities": {"streaming": false} },
  "nodes": [
    { "type_id": "myplugin:add", "display_name": "Add",
      "category": "demo",
      "ports": [{"id": "in", "label": "In", "kind": "in"},
                {"id": "out", "label": "Out", "kind": "out"}],
      "default_config": {"a": 0, "b": 0},
      "config_schema": {"type": "object"},
      "frontend": {"type": "inline", "content": "<p>add</p>"},
      "capabilities": {"streaming": false} }
  ]
}
```

2. **Array shape** (SDK legacy): a bare JSON array of node descriptions
   with no manifest echo — the host falls back to the on-disk manifest
   (this is what `mpe-plugin-sdk` emits today; both load).

Node field notes:

- `type_id` MUST be unique and SHOULD be namespaced (`vendor:name`).
  **Built-in host ids are reserved** — a plugin colliding with a built-in
  is rejected with a warning; plugin-vs-plugin collisions are first-wins.
- `ports` default to empty; `kind` is `"in"` (left) or `"out"` (right).
- `frontend`: P0 supports only `{"type":"inline"}`; anything else is
  warned about and dropped (never a crash). No iframes in P0.
- `capabilities.streaming` per node mirrors the manifest semantics.

### 4.2 `execute`

Request params:

```json
{
  "config": { "a": 2, "b": 3 },
  "providers": {},
  "referenced_variables": ["initial_variables.host"],
  "execution_id": "exec-2f8a1c9e-9b3d-4e6f-8a1c-5d7e9f0a2b3c",
  "node_instance_id": "node-uuid"
}
```

- `config` is the node config **after host-side variable resolution** —
  plugins always receive final values, never `{{...}}` placeholders.
- `providers` carries JSON-serializable providers (typed protocol
  providers cannot be encoded; P1).
- `execution_id` / `node_instance_id` correlate logs and events; every
  `log`-style notification should echo them back.
- `execution_id` is a **per-execution instance id** — a fresh
  `Uuid::new_v4()` value unique to each flow run, not the flow's stable
  uuid. Plugins must treat it as opaque state-keying material (e.g. for
  resident per-run state) and never assume it equals `flow.uuid`. The host
  generates it once per `execute_flow` invocation, so **every node in one run
  shares the same `execution_id`** (connect/find/close can key a per-run pool
  on it), while different runs — repeated executions, stress virtual users,
  dataset rows — each get a distinct value.

Response result:

```json
{
  "success": true,
  "next_ports": ["out"],
  "output_data": { "sum": 5 },
  "errors": [{ "code": "E_X", "message": "..." }],
  "report_data": { "plugin_name": "myplugin", "metric": 42 },
  "variable_updates": { "last_sum": 5 },
  "provider_updates": {}
}
```

- `success: false` routes per the node's `on_error` strategy.
- `next_ports` are **port ids**, resolved through the flow's connections
  by the host wrapper.
- `report_data` (≤ 1 MiB) surfaces as the node report's `plugin_data`.
- **`variable_updates` keys MUST be flat top-level variable names — no
  dots** (locked A10 contract). A dotted key is warned about and skipped
  by the host; the plugin should never rely on it.
- `provider_updates` are absorbed as JSON values; typed providers are P1.

### 4.3 `validate`, `cancel`, `flowEnded`, `shutdown`

- `validate` (request): respond `{"valid": true}` or
  `{"valid": false, "message": "..."}` as a **success result** (not a
  JSON-RPC error).
- `cancel` (notification, `params: {"id": <request_id>}`): best-effort P0
  hook; a no-op is acceptable.
- `flowEnded` (notification, `params: {"execution_id": <id>}`): host-side
  per-execution resource release hook (see §2.1). Implement
  `Plugin::flow_ended(&self, _execution_id: &str)` (SDK default no-op) to
  drop per-run state like a Mongo connection pool; the value matches the
  `execution_id` from every `execute` of that run. Older plugins silently
  ignore it.
- `shutdown`: the host may send either a request (answer `null`, then
  exit) or a notification (exit). A clean EOF after shutdown is a clean
  exit; EOF with pending requests is a crash.
- Unknown methods → JSON-RPC error `-32601`. Malformed frames → skip +
  log to stderr, never exit.

## 5. Rust SDK (first-class)

Crate: `mpe-plugin-sdk` (leaf crate — depends only on `mpe-protocol`,
`tokio`, `serde`, `serde_json`; **no host types**). The SDK owns the event
loop, framing, single-writer stdout task, and spawn-per-request
pipelining; the author writes pure handlers.

```rust
use std::future::Future;
use mpe_plugin_sdk::prelude::*;

#[derive(Default)]
struct AddPlugin;

impl Plugin for AddPlugin {
    fn describe(&self) -> Vec<NodeDescription> {
        vec![NodeDescription::new("myplugin:add", "Add")]
    }

    // The runtime spawns execute futures, so the future must be Send; an
    // `async fn` cannot express that bound on stable.
    #[allow(clippy::manual_async_fn)]
    fn execute(&self, ctx: &mut ExecuteContext) -> impl Future<Output = ExecuteResult> + Send {
        async move {
            let a = ctx.config().get("a").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            let b = ctx.config().get("b").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            ctx.set_variable("last_sum", serde_json::json!(a + b)); // flat key (A10)
            ExecuteResult::ok(serde_json::json!({ "sum": a + b }))
        }
    }
}

mpe_plugin_main!(AddPlugin);
```

- `Plugin::execute` is RPITIT (`-> impl Future + Send`), not `async fn` —
  the compiler cannot prove `async fn` futures are `Send` in a generic
  runtime. Keep the `#[allow(clippy::manual_async_fn)]` with the comment.
- `ExecuteContext` gives you the resolved `config()`, `providers()`, the
  log channel (`ctx.log(level, message)` — emitted as correlation-bearing
  notifications), and `set_variable` for flat-key write-back.
- `NodeDescription::new(type_id, display_name)` + builder-style optional
  fields keep minimal plugins clean.
- Demo plugin: `crates/mpe-plugin-sdk/src/bin/demo_plugin.rs`
  (`mpe_demo_plugin` binary; `cargo build -p mpe-plugin-sdk --bin
  mpe_demo_plugin`).
- Test it without the host: the SDK's `tests/roundtrip.rs` spawns the demo
  binary and speaks JSON-RPC over stdio; the host's conformance suite
  (`cargo test -p flow-engine-plugin --test conformance`) runs the same
  surface against the built binary.

## 6. Python and Node templates (light)

Copy-and-adapt templates in `sdk-templates/`:

- `sdk-templates/python/plugin.py` + `manifest.json` — one node type
  `pyecho:add` (adds `a` + `b`). Binary stdio, UTF-8 forced both by the
  template and by host-injected env, LF framing with CRLF tolerance,
  execute on a worker thread (pipelined), flat `variable_updates`.
- `sdk-templates/node/plugin.js` + `manifest.json` — one node type
  `nodedemo:echo` (echoes the resolved config). Binary-safe manual buffer
  framing, async execute handlers (pipelined, out-of-order responses by
  id), CRLF tolerance.

Run them by hand to smoke-test:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"describe","params":{}}' |
  python sdk-templates/python/plugin.py
echo '{"jsonrpc":"2.0","id":2,"method":"execute","params":{"config":{"a":2,"b":3}}}' |
  node sdk-templates/node/plugin.js
```

The host conformance suite drives both templates automatically
(`python_template_conformance`, `node_template_conformance`), skipping
with a log when the interpreter is unavailable.

## 7. Known limitations (P0)

- `mpe run-node` stays built-in-only; plugin node types run via `mpe run`
  flows (`TODO(plugin)` marker in `SUPPORTED_SINGLE_NODE_TYPES`).
- The plugin process is spawned with the **host's working directory**;
  relative entry args (`args: ["plugin.py"]`) resolve against it.
- Typed providers cannot cross the wire (P1); `provider_updates` are JSON
  values only.
- A plugin compiled with `windows_subsystem = "windows"` has no usable
  stdio and cannot use this transport.
- P1: iframe frontends, streaming (`execute_stream`), heartbeat, cancel
  RPC, connection-pool SDK, marketplace, hot reload.

## 8. End-to-end check

```bash
# build the SDK demo plugin
cargo build -p mpe-plugin-sdk --bin mpe_demo_plugin

# conformance across all runtimes (echo mock + python + node + sdk)
cargo test -p flow-engine-plugin --test conformance

# e2e: mpe run with a fixture plugin
cargo test -p mpe-cli --test cli_tests

# pipelining bench (echo mock + real SDK plugin)
cargo bench -p flow-engine-plugin --bench pipelining
```
