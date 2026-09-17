# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Breaking**: Built-in node type IDs now use the `<protocol>:<operation>` namespace shared with plugin nodes: `http` → `http:request`, `ws_connect` → `ws:connect`, `ws_send_collect` → `ws:send_collect`, `ws_close` → `ws:close`, `tcp_connect`/`tcp_send`/`tcp_receive`/`tcp_close` → `tcp:*`, `udp_bind`/`udp_send_to`/`udp_recv_from`/`udp_close` → `udp:*`, `sse_connect`/`sse_listen`/`sse_disconnect` → `sse:*`, `graphql_connect`/`graphql_query`/`graphql_subscribe`/`graphql_introspect`/`graphql_disconnect` → `graphql:*`, `sub_flow` → `flow:sub`, `variable_extractor` → `variable:extractor`. Flow files (`.mpf`) and templates that reference the old IDs no longer resolve to typed nodes: validation skips them and execution reports `Unregistered node type`. Re-add affected nodes on the canvas or update the `node_type` / `type` fields.

### Fixed
- Fixed node picker hover jitter: the row highlight border no longer animates in and out, and the detail card keeps a fixed height so the node list stops reflowing under the cursor.

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
