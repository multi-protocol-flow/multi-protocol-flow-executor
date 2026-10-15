# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Declarative Flow YAML DSL & Bi-directional Lossless Conversion**: Introduced `DeclarativeFlow` and `DeclarativeNode` models in `flow-engine-core`, stripping UI coordinates, connection UUIDs, and internal port IDs into clean, human-readable semantic YAML workflows with Sugiyama topological auto-layout and deterministic UUIDv5 generation.
- **CLI Flow Conversion & Format Detection**: Added `mpe flow convert <in.mpf> <out.yaml>` and transparent declarative DSL detection in `deserialize_flow_file` across all loaders and CLI runners.
- **Standardized CI Exit Codes & 3-Stage Aligned JUnit XML**: Standardized exit codes in `mpe-cli` (`0` success, `1` logic failure, `2` invalid arguments / missing files, `3` infrastructure timeout / unreachable). Upgraded JUnit XML generation to produce aligned `<testsuite>` entries (`{flow} :: Setup`, `{flow} :: Main`, `{flow} :: Teardown`) with skipped stage propagation and output snapshots on failure.
- **GitHub Actions Step Summary Native Support**: Added automatic `$GITHUB_STEP_SUMMARY` detection and `--summary <path>` Markdown summary generator with execution status badges, lifecycle stage breakdown table, and collapsible failure details.
- **Ingress Shared Gateway & Multi-Flow Path-Isolated Routing**: Introduced `IngressRouter` in `flow-engine-executor` supporting shared multi-flow listening on port 9527 with slugified path-prefix isolation, custom port overrides, and relative path forwarding.
- **`mpe serve` Long-Running Ingress Runtime**: Added `mpe serve` command for running daemonized ingress mock services, executing single Setup on startup, handling concurrent Tokio HTTP dispatch, and executing Teardown during graceful shutdown (SIGINT / Ctrl+C).
- **HTTP Ingress Webhook Mock Template**: Added a built-in flow template (`http_ingress_webhook_mock`) demonstrating webhook ingestion via `http:ingress` coupled with `http:respond` for mock HTTP responses.
- **Interactive AI Tool (`ask_user`)**: Added the `ask_user` tool and interactive input cards within the AI Copilot stream, allowing the model to request user input (text, choices, confirmations) and suspending tool loop execution until user submission.
- **Thinking Token Count & Physics Droplet Animation**: Added localized thinking character and word count metrics (`ThinkingSpinner`) and upgraded the thinking step loading indicator to an animated dot stacking and falling effect.
- **Datasource Read Whitelist for Plugins**: Expanded `pluginRequestActions` with `listDatasources`, `browseDatasource`, and `getDatasourceTemplates`. Sanitizes connection credentials at the host level in accordance with PR-5 sandbox boundaries before proxying datasource calls.
- **Plugin Dynamic Node Discovery**: Added `getRegisteredNodes` to the `PLUGIN_REQUEST_ACTIONS` whitelist, allowing sandboxed iframe plugins to discover host-registered node types via `get_registered_node_types`.
- **Canvas Tab Bar File Path Copy**: Added "Copy Flow File Absolute Path" to the canvas flow tab bar context menu.
- **Entry Node in Palette**: Added the Entry node option into `NodePalette` with uniqueness constraints (disabled if an entry node already exists on the canvas).
### Changed
- **Node Context Menu Duplication**: Renamed the canvas node context menu item from "Copy" to "Create Duplicate" (`clone_node`) to accurately describe its canvas-cloning behavior.
- **Terminal Stdio Isolation**: Switched subprocess execution in Tauri terminal commands to default to `Stdio::null` to prevent background process hangs, with stdin piping support for interactive commands.
- **AI Copilot Navigation & Morphology Stability**: Added a scroll-to-bottom action in the AI Copilot chat panel when scrolled up, and stabilized scroll position across docked and floating panel morphology toggles.
- **Panel Container Spacing Standard**: Standardized root container padding and spacing across `HttpIngressConfig` and `HttpRespondConfig` panels to align with design tokens.

### Fixed
- **Ingress Execution Priority & Validation**: Flow executor and debugger now prioritize `http:ingress` as the start node in ingress mode, with pre-execution validation rejecting missing or multiple ingress nodes before server initialization.
- **Entry Node Deletion & Duplication Guard**: Improved entry node singleton safeguards—prevents duplicate entry/ingress additions while allowing safe deletion of redundant entries when multiples exist, ensuring the last remaining entry cannot be accidentally removed.
- **Real-Time Streaming Report in Execution Panel**: Optimized `ExecutionPanel` during running state to dynamically construct live reports for nodes with custom/streaming viewers, preventing UI fallback to empty loading states following human-in-the-loop (`ask_human`) prompt submissions.
- **AI Thinking Indicator Alignment**: Fixed horizontal center alignment between the thinking process header and deep-thinking status badge in `AIMessageRow` and `AISpotlightDialog`.

## [0.5.0-beta] - 2026-10-12

### Added
- **Flow Lifecycle Stages (Setup / Main / Teardown)**: A flow is now a first-class three-stage lifecycle. `flow.setup` and `flow.teardown` are optional sub-graphs (`FlowSubGraph`) that execute once before / after the business flow, and `flow.nodes` / `flow.connections` remain the Main stage. Both extra stages accept the full multi-protocol node set, so sign-in warm-up, token extraction, notification webhooks and resource cleanup are orchestrated with real nodes instead of embedded callbacks. Setup exports variables straight into the Main context; Main's variables are visible to Teardown.
- **Stage Switcher in the Canvas**: A compact segmented pill bar placed at the top-left of the canvas switches the view between `Setup` / `Main` / `Teardown`, avoiding floating toolbar occlusion. Hover tooltips explain each stage, eliminating permanent blocking text cards. Node add/update/delete/connect operations and undo/redo only touch the active stage. The node outline lists all three stages with a stage chip and jumps the canvas to a node's own stage when clicked.
- **Stage Following During Execution**: The canvas automatically tracks executing nodes across stages (`Setup` → `Main` → `Teardown`) during both normal flow runs and debug stepping.
- **Full Debug Lifecycle Orchestration**: The debug engine now steps through `Setup` → `Main` → `Teardown`. Breakpoints set on Setup and Teardown nodes pause correctly, variables from Setup seamlessly flow into Main, and `$report` is available in Teardown during debugging.
- **Deterministic Visual Execution & Unlinked Node Warning**: Disconnected root nodes in Setup and Teardown execute deterministically from top-to-bottom and left-to-right based on canvas `(y, x)` coordinates, avoiding random JSON array order. The canvas displays an amber warning pill if multiple unlinked nodes exist, advising users to connect them.
- **Stage Reports**: `FlowExecutionReport.stages` (`FlowStagesReport`) reports each stage's status, duration and node reports; `main_duration_ms` isolates the business flow. Report details render collapsible Setup / Main / Teardown panels, and the report canvas viewer can switch between the snapshot's stages.
- **`$report` in Teardown**: Teardown receives the Main stage report as the read-only `$report` variable (usable via `{{$report.status}}`, `{{$report.duration_ms}}`, `{{$report.node_reports}}`), enabling assertions and notification payloads based on the actual outcome.
- **Debug Stage Sync**: When a debug session pauses on a node belonging to Setup / Teardown, the canvas switches to that stage automatically; the tab bar shows per-stage debug status (completed / paused / waiting) and offers a jump-back banner when the paused node is on another stage.
- **Stress & Dataset Lifecycle Isolation**: Stress tests run Setup once before the ramp-up, loop only Main for every virtual user, then run Teardown once after all users stop. Setup/Teardown nodes never enter `MetricsCollector`, so QPS / P50 / P99 / error rate stay pure Main measurements; the report carries the extra stages in `StressTestReport.lifecycle`. Dataset execution likewise runs Setup once, consumes rows with Main, and runs Teardown over the `DatasetExecutionReport`.
- **Interactive Flow Slice Execution & Human-in-the-Loop (HITL)**: Run selected subgraphs or single nodes directly from the canvas via right-click "Run Selected Nodes" (`run_slice`), executing without writing ephemeral slice runs to persistent execution history. The execution engine adapts automatically to entry-less slices by deriving execution roots from zero-indegree nodes, and integrates `tool_bridge.rs` for dynamic tool-calling loopback scheduling across AI agents and plugins. Integrated HITL interactive cards (`ask_human`), slice badges, and stop controls directly into `ExecutionPanel`.
- **AI Copilot Slash Commands**: Built-in slash shortcut command dispatch engine (`/fix`, `/explain`, `/test`, etc.) with autocomplete popup overlay and full bilingual i18n support.
- **AI Context Compression & Request Pruning**: Automatic conversation context compression and prompt trimming to prevent context window overflow while preserving key execution details, backed by concurrency race protection in the AI store.
- **AI Copilot Full Trust Mode**: Optional full-trust execution setting for high-risk operations, bypassing manual confirmation cards for uninterrupted agentic workflow execution.
- **Subflow Exported Variable Inference & Script Extraction**: Static analysis engine in `variableExtraction.ts` statically deduces exported variable keys from subflows and extracts script variables. The subflow configuration panel displays expected exported variables preview with one-click copying, and softens variable validation warnings when dynamic variables are provided by full subflow exports.
- **Datasource Hierarchy & Cache Isolation**: Re-architected data source sidebar panel into a structured hierarchy with single-project penetration and multi-source default collapse. Strictly isolated browsing cache per connection instance, preventing cross-datasource cache leakage between multiple sources of the same plugin. Added end-to-end RPC and describe integration testing for OpenAPI datasource plugins.
- **Standardized UI Components & Flow List Navigation**: Introduced a standardized `Checkbox` component replacing native browser checkboxes with dark-mode compliance, enhanced `Switch` component contrast in dark mode, and added flow list active state glowing highlights with smooth scroll-into-view targeting.
- **CLI Lifecycle Fixture**: `tests/cli/lifecycle_stages.json` exercises the three-stage lifecycle end to end through `mpe run`.

### Changed
- **Global Environments**: Environment definitions (variable sets such as `dev` / `staging` / `prod`) are now stored as standalone YAML files under `<flows_dir>/environments/` and shared across every flow, instead of being embedded in each `.mpf` file. A per-environment local override layer (`<name>.local.yaml`) is merged on top and is meant to be git-ignored for machine-private secrets.
- **Environment Resolution Order**: Execution variables are resolved as `flow.initial_variables` → environment file → local override → runtime variables (`--var` / explicit execution input).
- **Environment UI**: A resizable *Environments* section in the left sidebar (activate, rename, duplicate, delete), a canvas toolbar environment selector, and a context-aware environment manager dialog — global mode (YAML editing, local override toggling) and canvas mode (flow fallback variables plus a cascade comparison matrix).
- **Environment Storage Settings**: New global setting `environmentsDir` (falls back to `<flows_dir>/environments`), configurable in Settings → Storage.
- **CLI Environment Flag**: `mpe run` and `mpe debug` accept `-e, --env <NAME>`, plus the `MPE_ENVIRONMENTS_DIR` environment variable and `storage.environments_dir` config key.
- **CLI Stress Environment Flag**: `mpe stress run` also accepts `-e, --env <NAME>` and `--var KEY=VALUE`, resolving the same flows/environments directories as `run` / `debug` (previously stress iterations could only see `flow.initial_variables`).
- **CLI Environment Management**: new `mpe env` subcommands — `list` (name, variable count, local-override flag, path), `show <name> [--source merged|base|local]`, `set <name> KEY=VALUE... [--local]`, and `delete <name> [--local]` — so environments can be managed from CI without the GUI (`--format table` supported).
- **GUI Stress Environment**: the stress test dialog now runs with the active global environment merged in (flow fallback → environment file → local override), matching single-run execution and debugging.
- **Flow Export with Environment Bake**: Exporting a flow can now bake the active environment into `flow.initial_variables`, producing a self-contained `.mpf` file. A credential scan detects `key` / `secret` / `token` / `password` / `auth` / `cred` / `cert` / `private` variables and clears the values the user selects.
- Replaced legacy `autoExecuteToGoal` toggle in favor of goal-driven agent prompt protocols.
- Refactored `AICopilotPanel` by extracting `AIMessageRow` and `AICopilotInput` with isolated local state, memoized card renderers (`React.memo`), and cached JSON formatting.
- Optimized `MarkdownViewer` rendering performance with component memoization and static plugin mappings.
- **Breaking**: Removed `environments` and `active_environment` from the flow data model — no compatibility shim is provided. Existing `.mpf` files still load, but any embedded environment definitions are **ignored**; re-create them as environment files under `<flows_dir>/environments/` (the environment name is the file name).
- The flow properties panel no longer edits initial variables inline; it links to the environment manager, keeping `flow.initial_variables` as the flow-scoped fallback layer only.
- **Breaking**: A stage failure decides the overall status: a failing Setup skips Main (`stages.main.status == "skipped"`) and still runs Teardown, while a failing Teardown marks the flow failed even though `stages.main.status` stays `success` — the report keeps the tested business outcome distinguishable from operational cleanup failure.
- Recoverable node failures (`used_port_id == "false"`, e.g. a script that throws) now mark the stage failed in Setup / Teardown exactly like they do in Main, instead of silently continuing.

### Removed
- **Flow Hooks**: `FlowHook` / `HookAction` / `HookTrigger`, the Entry-node hook editor, the `HookExecutor` trait and the webhook/script/notify `hook_runner` are gone — the Teardown stage replaces them with real nodes. `EntryConfig.hooks` is dropped from the data model, the schemas and the i18n catalogues; no compatibility shim is provided.

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
- **Plugin Panels Did Not Follow the Host Theme**: plugin frontends run in a sandboxed iframe with an opaque origin, so they could not inherit the host palette and fell back to the OS `prefers-color-scheme`. A dark host on a light OS (or the reverse) left plugin inputs with white backgrounds and near-white text — effectively unreadable. The host now sends its resolved `light` / `dark` theme in the `init` payload and pushes later switches over a new `themeChanged` message; both iframe bridges (config panels and report viewers) are wired up. The Tauri window theme is also synced to the host setting so plugins built against the older contract still follow along. The contract is documented in `docs/plugin-frontend-theme.md`.
- **Debug Session Variable Synchronization**: Ensured final resolved variables and call stack tabs are retained in the execution panel after a debug session terminates.
- **Relative Path Flow Moving**: Fixed file resolution errors when moving flow files across folders by relative paths.
- **Responsive Layout Collisions**: Fixed overlapping titles and action buttons in protocol viewers, node reports, and AI floating windows when displayed in narrow sidebars or compact viewport dimensions.

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
