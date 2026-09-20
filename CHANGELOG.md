# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Global Environments**: Environment definitions (variable sets such as `dev` / `staging` / `prod`) are now stored as standalone YAML files under `<flows_dir>/environments/` and shared across every flow, instead of being embedded in each `.mpf` file. A per-environment local override layer (`<name>.local.yaml`) is merged on top and is meant to be git-ignored for machine-private secrets.
- **Environment Resolution Order**: Execution variables are resolved as `flow.initial_variables` → environment file → local override → runtime variables (`--var` / explicit execution input).
- **Environment UI**: A resizable *Environments* section in the left sidebar (activate, rename, duplicate, delete), a canvas toolbar environment selector, and a context-aware environment manager dialog — global mode (YAML editing, local override toggling) and canvas mode (flow fallback variables plus a cascade comparison matrix).
- **Environment Storage Settings**: New global setting `environmentsDir` (falls back to `<flows_dir>/environments`), configurable in Settings → Storage.
- **CLI Environment Flag**: `mpe run` and `mpe debug` accept `-e, --env <NAME>`, plus the `MPE_ENVIRONMENTS_DIR` environment variable and `storage.environments_dir` config key.
- **CLI Stress Environment Flag**: `mpe stress run` also accepts `-e, --env <NAME>` and `--var KEY=VALUE`, resolving the same flows/environments directories as `run` / `debug` (previously stress iterations could only see `flow.initial_variables`).
- **CLI Environment Management**: new `mpe env` subcommands — `list` (name, variable count, local-override flag, path), `show <name> [--source merged|base|local]`, `set <name> KEY=VALUE... [--local]`, and `delete <name> [--local]` — so environments can be managed from CI without the GUI (`--format table` supported).
- **GUI Stress Environment**: the stress test dialog now runs with the active global environment merged in (flow fallback → environment file → local override), matching single-run execution and debugging.
- **Flow Export with Environment Bake**: Exporting a flow can now bake the active environment into `flow.initial_variables`, producing a self-contained `.mpf` file. A credential scan detects `key` / `secret` / `token` / `password` / `auth` / `cred` / `cert` / `private` variables and clears the values the user selects.

### Changed
- **Breaking**: Removed `environments` and `active_environment` from the flow data model — no compatibility shim is provided. Existing `.mpf` files still load, but any embedded environment definitions are **ignored**; re-create them as environment files under `<flows_dir>/environments/` (the environment name is the file name).
- The flow properties panel no longer edits initial variables inline; it links to the environment manager, keeping `flow.initial_variables` as the flow-scoped fallback layer only.

### Fixed
- **Debug Sessions Resolve the Active Environment**: `start_debug` now receives the active global environment and merges it (flow fallback → environment file → local override) instead of silently debugging with `flow.initial_variables` only, which ran a production flow against the fallback endpoint.
- **CLI Stress Test Never Exited**: `mpe stress run` hung after the test finished (the event loop waited on an event sender released only at function exit), so `--output` reports were never written and CI jobs never completed.
- **AI Environment Tools Leaked Local Secrets**: the Copilot's `set_environment_variable` merged `.local.yaml` values into the shared environment file on write, and `list_environments` returned them to the model. Both paths now read and write the shared base file only; the local override is reported as a boolean flag.
- **AI Environment Listing Was Empty**: `list_environments` iterated a store snapshot taken before loading, so it returned no environments until the sidebar had been opened.
- **AI Variable Placeholder Guidance**: the Copilot prompt and node schemas documented `{{env.XXX}}` / `{{variables.XXX}}` / `{{context.steps.*}}`, which the engine cannot resolve (nodes failed with `Variable not found`); variables are now documented as bare keys (`{{API_URL}}`, `{{USER.id}}`).
- **Script Assertion Results Were Dropped**: `fn.expect(...)` results lived only inside the sandbox JS closure, so script node output never contained `test_results` / `all_assertions_passed`; the executor now collects them after the script runs.
- **AI Script API Guidance**: the Copilot prompt now documents the sandbox surface that actually exists (`fn.variables.*`, `fn.response.code/status/body/raw`, `fn.flow.*`, `fn.info.*`, verified `fn.util.*` helpers, the `fn.expect(...).to.equal/eql/beTrue/haveLength/...` chain without `toBe`).
- **Script Object Helpers Dropped Data**: `fn.util` object-taking helpers (`keys` / `values` / `entries` / `hasKey` / `pick` / `omit` / `merge` / `clone` / `jsonStringify` / `jsonPath`) read every property with a string-only conversion, so numbers, booleans, nested objects and arrays were silently discarded (`fn.util.pick({a: 1}, ['a'])` returned `{}`). They now convert JS values recursively (`js_value_to_json`), keep JSON types, drop `undefined` / function properties like `JSON.stringify`, and `pick` / `omit` accept a native key array instead of a hand-written JSON string. `hasKey` inspects own JS keys directly (it previously reported `hasKey({a: undefined}, 'a')` as `false`), and array conversion is depth- and size-bounded so a sparse `arr.length = 1e9` can neither overflow the stack nor trigger a multi-gigabyte host allocation. `fn.variables.set` / `setAll` / `all` got the same treatment: non-string values are stored as JSON instead of being dropped.
- **`fn.flow.abort()` Without a Message**: `fn.flow.abort` lacked the argument shim `fn.flow.stop` already had, so the documented `abort(msg?)` form threw `expected string, got undefined` and crashed the script; both now accept an optional message.
- **Copilot Advanced-Condition Example Was Invalid**: the prompt's `advanced_expression` example used `TOKEN != null`, but the engine evaluates these with evalexpr, which has no `null` literal (`Variable identifier is not bound to anything by context: null`); the prompt now shows a valid emptiness check and states the limitation.

## [0.4.1-beta] - 2026-09-18

### Added
- **HTTP Ingress & Mock Services**: Added `http:ingress` and `http:respond` mock service nodes to simulate incoming HTTP webhook endpoints and handle HTTP responses within flows.
- **Interactive Flow Debugging**: Added variable timeline inspection, in-place node step retries, and live stream probes for detailed runtime debugging.
- **Execution Report On-Demand Loading**: Added lazy inspection of execution details with skeleton placeholders, dual-threshold batch event pushing, and in-memory cold-channel caching.

### Changed
- **Breaking**: Built-in node type IDs now use the `<protocol>:<operation>` namespace shared with plugin nodes: `http` → `http:request`, `ws_connect` → `ws:connect`, `ws_send_collect` → `ws:send_collect`, `ws_close` → `ws:close`, `tcp_connect`/`tcp_send`/`tcp_receive`/`tcp_close` → `tcp:*`, `udp_bind`/`udp_send_to`/`udp_recv_from`/`udp_close` → `udp:*`, `sse_connect`/`sse_listen`/`sse_disconnect` → `sse:*`, `graphql_connect`/`graphql_query`/`graphql_subscribe`/`graphql_introspect`/`graphql_disconnect` → `graphql:*`, `sub_flow` → `flow:sub`, `variable_extractor` → `variable:extractor`. Flow files (`.mpf`) and templates that reference legacy IDs must be updated.
- Redesigned the node palette detail card into a bottom-docked, adaptive overlay.

### Performance
- **Canvas State Decoupling**: Extracted `FlowSurface` to decouple canvas state and preserve node DOM identity across drag operations, eliminating unnecessary re-renders.
- **GPU Compositing**: Added GPU composite layer hints (`will-change`) for the canvas viewport and dragging nodes.
- **Direct Backend Report Persistence**: Streamlined execution reports by persisting run data directly on the backend and emitting lightweight IPC events, eliminating redundant frontend save calls.
- **Virtualized Console Logging**: Replaced console log lists with a virtualized renderer and batch write buffer with end-of-run synchronous flushing.
- **AI Streaming Buffer**: Introduced streaming message batching buffers to eliminate main-thread reflow and lag during AI thought generation.
- Cached flow-scoped breakpoint lookups with `O(1)` set queries and added reference equality guards for custom edge subscriptions.

### Fixed
- Fixed node picker hover jitter: the row highlight border no longer animates in and out, and the detail card maintains stable geometry.
- Cascaded `connection_uuid` cross-node reference updates during flow duplication and remapping.
- Fixed false-positive conflict detection when flows lack file paths or when switching between identical flows.

## [0.4.0-beta] - 2026-09-16

### Added
- **YAML Flow Storage & CLI Formatter**: Introduced native YAML flow serialization (`.mpf`) alongside backward-compatible JSON deserialization, adaptive drag-and-drop dual-format import on the canvas, and the `mpe flow fmt` CLI formatting command.
- **UUID Conflict Detection & Remapping**: Added automated UUID conflict detection and the `FlowConflictDialog` on flow duplicate or import, cascading node and connection reference updates, and atomic path-driven flow saves.
- **AI Copilot Terminal Tool**: Added a secure terminal execution tool with command whitelisting, high-risk confirmation cards, and multi-platform shell execution.
- **Customizable Canvas Tab Bar**: Added display mode switching for flow tabs (ultra-thin horizontal scrolling or multi-line wrapping) with local storage persistence.
- **Path-Driven Flow Management**: Added direct file path binding for subflow nodes, path-based flow loading in the sidebar, and file system renaming support.
- **Enhanced Response Viewers**: Optimized large response payload inspection with multi-view switching, CodeMirror virtualization, and adaptive auto-expansion.

### Fixed
- Fixed multi-tab flow execution state cross-contamination with `renderFlowUuid` isolation and disk-flush on tab switching.
- Fixed `RawJsonViewer` React hook ordering mismatch caused by conditional branches when switching dynamic data types.
- Fixed Copilot floating panel dialogue overflow pushing top toolbar out of view and added boundary validation for panel geometry persistence.
- Fixed flow search dialog sub-pixel blurriness and improved text rendering sharpness.
- Fixed Web mock flow persistence file extension to consistently use `.mpf`.

## [0.3.0-beta.1] - 2026-09-14

### Added
- Renamed installed application display name to concise `MPE`.

### Fixed
- Enforced strict artifact version inspection across Windows, macOS, and Linux release pipelines to prevent uploading mismatched packages.
- Enhanced in-app updater launch sequence and added immediate user feedback when launching installers on Linux (`.deb`) and Windows.
- Polished responsive layouts in the Settings dialog to prevent UI squeezing on smaller windows.

## [0.2.0-beta.2] - 2026-09-11

### Added
- Added cache management actions in Settings: clear local storage and restart the onboarding walkthrough.
- Added macOS DMG build and R2 incremental release automation script.

### Fixed
- Fixed blank UI text in production builds caused by esbuild lowering optional chaining on i18n translation proxies; set Vite target to `es2022`.
- Fixed welcome onboarding card overflow, adaptive boundary positioning, and backdrop mask contrast in light theme.
- Fixed button label collapse under WebKitGTK on Linux.

## [0.2.0-beta.1] - 2026-09-11

### Added
- Extracted heavy protocol crates (Kafka, Redis, MongoDB, SMTP, MQTT, AMQP) into independent sidecar plugins.
- Retained core native protocols (HTTP, WebSocket, TCP, UDP, SSE, GraphQL, Script, Assertion).
- Integrated official plugin SDK and protocol definitions for sidecar execution.

## [0.1.1-beta.2] - 2026-08-20

### Fixed
- Fixed Linux CLI static musl packaging dependencies.
- Stabilized Windows update artifact signature upload in CI workflows.

## [0.1.1-beta.1] - 2026-07-04

### Added
- Initial public beta release of MPE Flow (Multi-Protocol Flow Orchestration & Execution Engine).
- Visual canvas orchestration with React 19 and Tauri 2.x backend.
- Cross-platform support for Windows, macOS, and Linux (GUI and CLI).
