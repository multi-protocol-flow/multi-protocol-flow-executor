# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- Cached flow-scoped breakpoint lookups with $O(1)$ set queries and added reference equality guards for custom edge subscriptions.

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
