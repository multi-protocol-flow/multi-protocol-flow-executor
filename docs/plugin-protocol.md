# MPE Plugin Wire Protocol — Specification

> **Status:** normative (M1). This document is the source of truth for the
> JSON-RPC 2.0 stdio transport between the MPE host (`flow-engine-plugin`)
> and sidecar plugins (`mpe-plugin-sdk` and language templates). The wire
> types live in `mpe-protocol` so host and SDK cannot drift.
>
> Locked decisions this spec honors: batch arrays forbidden by construction;
> `variable_updates` flat-top-level keys only (A10); event notifications
> carry `execution_id` + `node_instance_id`.

## 1. Framing

Each message is exactly **one JSON value per line**, terminated by a single
`LF` (`0x0A`, `mpe_protocol::LINE_DELIMITER`).

- **CRLF tolerance.** A trailing `CR` before the `LF` is stripped by the
  reader. Plugins that emit `\r\n` (e.g. Python `print()`, which writes CRLF
  on Windows) remain valid. Writers MUST emit a bare `\n`.
- **Encoding.** Every line must be byte-exact UTF-8. A line with invalid
  UTF-8 is a framing violation (see §1.1).
- **BOM tolerance.** A UTF-8 BOM (`EF BB BF`) at the very start of the
  stream is stripped from the first frame only.
- **Partial-line buffering.** The reader buffers partial frames across reads
  until the delimiter or EOF. A stream that reaches EOF in the middle of a
  line (no trailing `LF`) is a violation (`UnexpectedEof`).
- **Half-close (EOF).** The host half-closes stdin after `shutdown` so the
  plugin observes EOF and can exit; the plugin half-closes stdout when it is
  done writing. A clean EOF is one observed *after* shutdown with no pending
  requests; EOF with pending requests or without shutdown is a crash.
- **Max line.** `2 MiB` (`mpe_protocol::MAX_LINE_BYTES`). Lines longer than
  the cap are rejected while reading — the reader never buffers without
  bound (no unbounded `read_line`). The cap exceeds the 1 MiB `report_data`
  budget so report payloads always fit.

### 1.1 Malformed-line policy

A malformed line (invalid JSON, non-object, batch array, mixed/empty
response form, invalid UTF-8, over-long) is **skipped and logged**, never
fatal. The host keeps a consecutive-malformed counter; after **10**
consecutive malformed lines the plugin process is killed and all pending
requests fail with `PluginCrashed`. A single malformed line never affects
any other request.

## 2. Notification demux

A **notification** is a message with `method` + `params` and **no top-level
`id`** (`mpe_protocol::Notification` — the struct has no `id` field by
construction). Notifications are fire-and-forget in both directions:

- **Plugin → host.** Incoming frames are discriminated by shape: a frame
  with `result`/`error` is a response; a frame with `method` (and no id) is a
  notification. Notifications are demuxed to registered handlers **before**
  any pending-request lookup — a notification is never mistaken for a
  response (it cannot match a pending id).
- **Host → plugin.** `send_notification` (e.g. best-effort `cancel`) never
  blocks and never expects a response.

### 2.1 `flowEnded` notification (host → plugin)

When a flow execution finishes — success, failure, or cancellation — the host
broadcasts a fire-and-forget notification so plugins can release
**per-execution** resources (e.g. a Mongo connection pool keyed by
`execution_id`):

```json
{ "jsonrpc": "2.0", "method": "flowEnded", "params": { "execution_id": "…" } }
```

Contract:

- `execution_id` is the **per-execution instance id** the plugin received on
  every `execute` request of that run (`mpe_protocol::notification::
  FLOW_ENDED_METHOD`), so the plugin can match it against its own per-run
  state. It is **not** the flow's stable uuid and is unique per execution
  (stress virtual users / dataset rows / repeated runs each get a fresh one).
- The broadcast only reaches **running** plugins: idle on-demand processes are
  never spawned just to receive it (the host peeks the live connection).
- It is fire-and-forget: no response, no error surface; a plugin that does not
  implement `flow_ended` (older SDK) silently ignores it via the
  unknown-notification path (§2 demux), and the transport stays usable.
- SDK: implement `Plugin::flow_ended(&self, _execution_id: &str)` (default
  no-op) to consume it. The `close` node remains the explicit, plugin-side
  release path; `flowEnded` is the host-side safety net for normal runs.

## 3. Error-object mapping

A JSON-RPC error response (`{"id":N,"error":{...}}`) resolves the pending
request with a typed host error: `PluginError::JsonRpc { code, message,
data }`, mapped canonically from `mpe_protocol::JsonRpcError`. Standard
codes retain their wire values (`-32700` parse, `-32600` invalid request,
`-32601` method not found, `-32602` invalid params, `-32603` internal,
`-32000..=-32099` server) and are identifiable via
`PluginError::is_standard_jsonrpc_code`. An error response never kills the
reader; the connection stays usable.

## 4. `variable_updates` contract

The `execute` result may carry `variable_updates` for the host to write back
into `VariableStorage`. **Keys MUST be flat top-level variable names — no
dots** (locked deviation A10): the host `VariableStorage` is flat and
exact-name-keyed, so a dotted key like `"runtime.total"` would silently keep
its placeholder. The host validates on receipt: a dotted key is warned +
skipped (non-fatal) via the host log (`log::warn!`), and the remaining keys
are still merged. Dotted keys are **not** recorded in the node report in P0.
`execute` must therefore return keys that are already top-level names.

> **P1:** record skipped dotted keys in the node report so users can audit
> which updates were dropped. P0 keeps the skip log-only.

## 5. Event correlation fields

Every event/log notification MUST carry two frozen params fields so streams
correlate to a single node execution (schema frozen day one):

```json
{ "execution_id": "…", "node_instance_id": "…", "level": "…", "message": "…" }
```

The host's correlation registry (see `transport::CorrelationFilter`)
subscribes handlers by method + optional `execution_id` / `node_instance_id`
filters; the shape is defined in `mpe_protocol::notification` so host and
SDK agree on the field names.

## 6. P1 frontend loading (CSP foundation)

- Locked: P0 ships NO iframe **rendering** in the frontend, but the host
  already generates the iframe `url` into the describe `frontend` metadata
  (see §9) so the later rendering PR has the URL ready. P1 default (OD3):
  `mpe-plugin://` custom Tauri protocol with its own origin — no blob, no
  CSP intersection. Platform split (F3): macOS/Linux use `mpe-plugin://`,
  Windows/Android use `http://` because custom schemes are not usable in
  iframe `src` there. Fallback: blob URL + `frame-src blob:; script-src
  blob:`.
- CSP staging note (M0.5): `src-tauri/tauri.conf.json` **cannot carry a
  staging comment** — the Tauri config parser rejects unknown keys
  (empirically tested: `$comment` fails with "unknown field"), and strict
  JSON forbids `//` comments. The P1 CSP change (`frame-src` /
  `connect-src` additions) must therefore land as a real, behavior-changing
  edit together with the P1 protocol registration — tracked here, not in
  tauri.conf.json.

## 7. Backpressure and pipe-full behavior

- The host bounds **in-flight requests** with a semaphore at
  `max_concurrent_requests` and the **writer queue** with a bounded channel
  (`queue_capacity`). When either is exhausted the caller **awaits** —
  requests are never rejected.
- A slow consumer (plugin not reading its stdin) backs up the OS pipe; the
  writer blocks, the queue fills, and callers wait. This is **head-of-line
  blocking across flows sharing the process** — documented and accepted,
  not redesigned. The host surfaces it via a slow-consumer counter exposed
  to stress metrics, plus a `log::warn` on queue-full.
- Every blocking step in a request (permit acquisition, enqueue, response
  wait) is bounded by the request timeout, so a wedged plugin can never hold
  a caller forever.

## 8. Per-request timeout and cancellation

Each request carries a host-side timeout (`request_timeout`, default 30 s).
On timeout the host removes the pending entry, fires a best-effort `cancel`
notification (`{"method":"cancel","params":{"id":<id>}}`, fire-and-forget),
and returns `Err(Timeout)`. Timeout is the **only** mechanism covering a
stuck-but-alive plugin. A plugin that actually exits drains all pending
requests with `Err(PluginCrashed)`.

## 9. Describe handshake (M2a')

The host runs `describe` once per discovered plugin, immediately after
spawning its process, to learn the plugin's node types before any flow can
use them.

**Request.** `method: "describe"`, `params: {}` (the plugin echoes everything
the host needs; the host sends no manifest — it already validated the
on-disk `plugin.json` during discovery).

**Response.** `result` is a `DescribeResult`:

```json
{
  "plugin":  { "name": "demo", "version": "1.0.0",
               "entry": {"command": "demo"},
               "capabilities": {"streaming": true},
               "locales": ["zh-CN", "en-US"],
               "default_locale": "en-US" },
  "nodes": [
    { "type_id": "demo:ping", "display_name": "Ping",
      "category": "demo", "icon": "wave", "color": "#ff0000",
      "ports": [{"id": "in", "label": "In", "kind": "in"},
                {"id": "out", "label": "Out", "kind": "out"}],
      "default_config": {"target": "localhost"},
      "config_schema": {"type": "object"},
      "frontend": {"type": "inline", "content": "<p>ping</p>"},
      "capabilities": {"streaming": false} }
  ]
}
```

- `plugin` is the plugin's authoritative manifest echo; its `capabilities`
  **override** the on-disk manifest (residency decision, M2b).
- `plugin.locales` (BCP-47 tags) + `plugin.default_locale`: the plugin's UI
  language declaration. The host resolves the effective language **once at
  spawn** and injects it into the plugin process as the `MPE_LOCALE`
  environment variable: the host's current language when listed in
  `locales`, otherwise `default_locale`. A plugin declaring neither is
  spawned without `MPE_LOCALE` (unchanged pre-locale behavior). The value is
  fixed for the process lifetime — a host language change only takes effect
  after a plugin restart / reload (see `docs/plugin-sdk.md` §5.1).
- `nodes[].frontend` (`{ "type", "content"?, "url"? }`): **all** frontend
  kinds pass through to the host — nothing is dropped. `content` carries
  inline HTML for `type: "inline"`. The host **generates the iframe `url`**
  during normalization (locked D5/F3): per-plugin origin
  `<plugin-id>.localhost` with the `config-panel` page —
  `mpe-plugin://<plugin-id>.localhost/config-panel` on macOS/Linux and
  `http://<plugin-id>.localhost/config-panel` on Windows/Android (selected at
  runtime with `cfg!(windows)`, so the frontend needs zero platform
  branching). A plugin-provided `url` is replaced by this host-generated
  value. The reserved `config_panel` / `viewer` / `help` frontend fields are
  P1 and not part of the P0 schema.
- `nodes[].category` / `icon` / `color` / `config_schema` are optional
  (`null`-able); `ports` and `nodes` default to empty; `default_config`
  defaults to `null`.

**Quarantine policy.** A describe failure — spawn failure, RPC timeout,
connection crash, or a JSON-RPC error response — counts against the plugin
by name. After **3** consecutive failures (reset on success) the plugin is
quarantined for the rest of the host manager's lifetime: every later scan
skips it with a warning, and no node from it is ever registered. A plugin
that crashes during `describe` can therefore never wedge the host.

## 10. Spawn hygiene

The host applies at spawn time: a stderr drain task (the 64 KB pipe buffer
stalls a plugin forever otherwise), `PYTHONUTF8=1` /
`PYTHONIOENCODING=utf-8` for interpreter plugins, `CREATE_NO_WINDOW` +
non-inherited stdio handles on Windows (so grandchildren do not inherit the
stdout pipe and prevent EOF), and a Windows Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so the host's death kills the plugin
and its grandchildren. Known limitation: a plugin compiled with
`windows_subsystem = "windows"` has no usable stdio and cannot use this
transport.

## 11. Execute RPC (M3.2)

The host wraps every plugin node in a `SidecarPlugin` and issues one `execute`
RPC per node execution.

**Request.** `method: "execute"`, `params`:

```json
{
  "config": { "type": "demo:ping", "target": "resolved.example" },
  "providers": {},
  "referenced_variables": ["initial_variables.host"],
  "execution_id": "exec-2f8a1c9e-9b3d-4e6f-8a1c-5d7e9f0a2b3c",
  "node_instance_id": "node-uuid"
}
```

- `config` is the node's config **after host-side variable resolution** — the
  plugin always receives final values, never `{{...}}` placeholders.
- `providers` carries the subset of host providers that are JSON-serializable
  (typed protocol providers cannot be encoded; P1).
- `referenced_variables` lists the `{{path}}` references found in the config
  (strategy A: only config-referenced).
- `execution_id` / `node_instance_id` correlate logs/events (see §5).
- `execution_id` is a **per-execution (per-run) instance id** — a fresh
  `Uuid::new_v4()` value unique to each flow run, not the flow's stable
  uuid. Plugins must treat it as opaque state-keying material and never
  assume it equals `flow.uuid`.

**Response.** `result` is an object:

```json
{
  "success": true,
  "next_ports": ["out"],
  "output_data": { "ok": true },
  "errors": [{ "code": "E_X", "message": "..." }],
  "report_data": { "plugin_name": "demo", "metric": 42 },
  "variable_updates": { "token": "abc" },
  "provider_updates": { "demo_svc": { "base_url": "http://x" } }
}
```

Host mapping (normative):

- `success: true` → the RPC returns `Ok`; `next_ports` (port ids) are resolved
  through the flow's connections into the next node ids; `output_data`
  (null-able) is the node output.
- `success: false` → the RPC returns `Err(ErrorDetail::Execution)` built from
  `errors[0]` (or a generic message when `errors` is empty).
- `report_data` (≤ 1 MiB, C3) → stored into
  `ExecutionContext.current_node_plugin_data` and surfaced as the node
  report's `plugin_data` (M3.3).
- `variable_updates` → **A10**: keys MUST be flat top-level variable names.
  A dotted key is warned + skipped (non-fatal); flat keys are merged into the
  host `VariableStorage` and appear in the report's `variable_changes`.
- `provider_updates` → absorbed into `context.providers` as
  `serde_json::Value` providers (typed providers are P1).
- A JSON-RPC error response (`error` instead of `result`) → single
  `Err(PluginError::JsonRpc{..})`; a dead/stuck connection → `Err(PluginError
  ::PluginCrashed)` / `Err(PluginError::Timeout)`, surfaced as one final
  `ErrorDetail` (`ErrorCode::PluginCrashed` / `ErrorKind::Timeout`). The
  host's wrapper owns the execute timeout (config `timeout_ms` /
  `execution_timeout_ms`, default 30 s) because plugin nodes have no executor
  timeout; the manager's restart-once policy handles the process.

## 12. `uiCall` RPC (config-panel discovery queries)

Config panels rendered inside a plugin iframe (`mpe-plugin://` frames) need
runtime discovery data that static `config_schema` cannot express: which
tools/resources/prompts a server exposes, a tool's `input_schema`, and so on.
The panel cannot talk to the plugin process directly, so the host relays the
query: frontend `plugin_ui_call` command → host `PluginManager::ui_call` →
`uiCall` RPC over the node type's plugin connection → plugin answer → back to
the panel.

**Request.** `method: "uiCall"`, `params`:

```json
{
  "method": "mcp.discover",
  "params": { "transport": "stdio", "command": "npx" }
}
```

- `method` is a plugin-defined sub-method (e.g. `"mcp.discover"`).
- `params` is an opaque payload whose shape is owned by the sub-method;
  `null` when the caller has no payload.

**Response.** `result` is any JSON value (the plugin's answer). Errors use the
standard JSON-RPC error object.

Host mapping (normative):

- The request has **no execution context**: it is a UI-time query unrelated to
  any flow run. Plugins must not mutate execution state (pools keyed by
  `execution_id`) from `uiCall`.
- The node type resolves to exactly one plugin (unknown types are a host
  protocol error before any request is sent), so a panel can never reach
  another plugin's process.
- Timeout uses the connection's per-request timeout (default 30 s, identical
  to execute RPCs); no separate timeout mechanism exists.
- A JSON-RPC error response surfaces as `PluginError::JsonRpc{..}` verbatim.
- Unknown sub-methods are the plugin's decision; the convention is to answer
  `-32601` with an explanatory message. A plugin that predates `uiCall`
  (SDK default) answers `-32601` "uiCall not supported".
