# Multi-Protocol Flow Executor

> Multi-protocol workflow orchestration and execution engine

**English** | [简体中文](./README.zh-CN.md)

A workflow orchestration and execution engine with a visual editor and a scriptable CLI, built as a cross-platform desktop application on **Rust + Tauri 2.x + React**.

## ✨ Features

### Visual flow editor
- 🎨 Drag-and-drop flow design on React Flow
- 📦 Node palette for adding every supported node type
- ⚙️ Live node configuration panel
- 📋 Flow list with create / delete / switch

### Execution engine
- ⚡ High-performance Rust execution engine
- 🔄 Core node types: Entry, HTTP, Conditional
- 📝 Live execution logs and detailed reports
- ✅ Flow validation and error detection
- 📊 Execution statistics and history
- 🚀 Built-in templates loaded on first launch

### Lifecycle stages
- 🧩 **Setup / Main / Teardown** as first-class stages of a flow
- 🔗 Setup exports variables straight into the Main context
- 📄 Teardown reads the Main report as the read-only `$report` variable
- 🧪 Stress and dataset runs execute Setup/Teardown once, so QPS and P99 stay pure Main measurements

### CLI mode
- 💻 Standalone command-line tool for batch execution and CI integration
- 📄 Runs JSON and YAML flow files
- 🔧 Flexible command-line arguments
- 📤 Structured JSON output for downstream tooling
- 🎯 JUnit XML, GitHub Step Summary, and self-contained HTML reports

### Global settings
- ⚙️ Configurable report and log locations
- 💾 Settings persisted automatically
- 📂 Directory picker and reveal-in-file-manager

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       Frontend (React + TypeScript)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  FlowCanvas │  │ NodeLibrary │  │ ConfigPanel │  │  Execution  │     │
│  │    canvas   │  │    palette  │  │   config    │  │   reports   │     │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
                              ↕ Tauri IPC
┌─────────────────────────────────────────────────────────────────────────┐
│                         Backend (Rust + Tauri 2.x)                       │
│                                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                      │
│  │  Executor   │  │  Validator  │  │   Storage   │                      │
│  └─────────────┘  └─────────────┘  └─────────────┘                      │
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                    NodeExecutor Implementations                    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────┐ ┌──────────┐            │  │
│  │  │  Entry   │ │   HTTP   │ │Conditional │ │   End    │            │  │
│  │  │ Executor │ │ Executor │ │  Executor  │ │ Executor │            │  │
│  │  └──────────┘ └──────────┘ └────────────┘ └──────────┘            │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────────────────┐
│                        Standalone CLI binaries                           │
│              run JSON / YAML flow files without a GUI                    │
└─────────────────────────────────────────────────────────────────────────┘
```

## 🚀 Getting started

### Requirements

- **Rust** >= 1.85.0
- **Node.js** >= 18 (`bun` also works)
- A package manager: `pnpm` / `npm` / `bun`

### Install dependencies

```bash
# Install frontend dependencies
npm install
# or
bun install
```

### Development

```bash
# Tauri dev mode (frontend + backend hot reload)
npx tauri dev
```

### Build the desktop app

```bash
# Production build (bundles frontend assets + Tauri backend)
npx tauri build
```

Artifacts land in `src-tauri/target/release/bundle/` (per-platform installer).

### Build the CLI

The project ships two binary entry points:

| Binary | Use case | Dependencies |
|--------|----------|--------------|
| `mpe` | GUI + CLI in one binary | Tauri + bundled frontend assets |
| `mpe-cli` | Pure CLI (servers / CI / Docker) | Plain Rust, no GUI dependencies |

#### Combined desktop binary (`mpe`)

```bash
# Debug build
cd src-tauri
cargo build

# Release build (size-optimized)
cd src-tauri
cargo build --release
```

Artifacts:
- Windows: `src-tauri\target\debug\mpe.exe` (or `release\mpe.exe`)
- Linux/macOS: `src-tauri/target/debug/mpe` (or `release/mpe`)

> The combined `mpe` binary needs the frontend build, so run `npm install` first.

#### Standalone CLI (pure Rust, recommended for Linux servers)

```bash
# Linux / macOS
cd src-tauri
cargo build -p mpe-cli --release
cp target/release/mpe-cli /usr/local/bin/mpe    # install on PATH

# Pick only the protocols you need (smaller binary, faster build).
# `plugin-manager` is not optional: the CLI reads the plugin connection
# config unconditionally, so a build without it fails to compile.
cargo build -p mpe-cli --release --no-default-features \
  -F plugin-manager -F protocol-http -F protocol-script
```

Artifact: `src-tauri/target/release/mpe-cli`

##### musl static build (runs on any Linux)

For a binary that does not depend on the host glibc version:

```bash
# Build the Docker toolchain image (waits for rustup the first time)
docker build -t mpe-musl-builder -f Dockerfile.musl .

# Compile the static mpe-cli
docker run --rm \
  -v $PWD:/src \
  -v musl-cargo:/root/.cargo/registry \
  mpe-musl-builder \
  sh -c "cd src-tauri && cargo build -p mpe-cli --release"

# Artifact is src-tauri/target/release/mpe-cli
file src-tauri/target/release/mpe-cli
# => ELF ... static-pie linked (no glibc dependency)
```

> The `mpe-v*-linux-x86_64.tar.gz` assets on GitHub Releases are these musl static builds.

#### Cross-compiling the Linux CLI from Windows

**Option 1: WSL (recommended)**

```bash
# Inside WSL (Ubuntu/Debian)
sudo apt install build-essential pkg-config libssl-dev
cd /path/to/multi-protocol-flow-executor/src-tauri
cargo build -p mpe-cli --release
# Artifact: target/release/mpe-cli — copy it to the Linux host
```

**Option 2: Rust cross-compilation**

```bash
# 1. Add the Linux target
rustup target add x86_64-unknown-linux-gnu

# 2. Install a cross linker (MSYS2 / mingw64), or just use WSL

# 3. Build
cd src-tauri
cargo build -p mpe-cli --release --target x86_64-unknown-linux-gnu
```

#### Install on Linux

```bash
cp src-tauri/target/release/mpe-cli /usr/local/bin/mpe
mpe --help
```

### Browser mode (frontend only, no Tauri backend)

The frontend runs standalone in a browser; every Tauri call falls back to a mock implementation.

```bash
# Browser dev mode (hot reload)
npm run dev:web

# Static site build, output to website/playground/
npm run build:web

# npm run dev behaves the same way in a plain editor
```

The build output in `website/playground/` can be served from any static host or GitHub Pages.

### Using the CLI

```bash
# Execute a flow
mpe run -f <path-to-flow.json>

# Validate a flow
mpe validate -f <path-to-flow.json>

# Stress test
mpe stress run -f <path-to-flow.json>

# Help
mpe --help
```

### Plugins

The plugin system lets third parties add node types as **separate processes** without rebuilding the host. Plugins are enabled by default and the plugin directory is scanned at startup.

**Installing a plugin:** drop a folder containing a `plugin.json` manifest and an entry script (`main.py`, `index.js`, …) into the plugin directory.

| Plugin directory | Notes |
|------------------|-------|
| Default | `get_data_dir()/plugins` (a `plugins` folder inside the app data directory) |
| Environment variable | `MPE_PLUGIN_DIR` overrides the default |

Official protocol plugins live in separate repositories under the [`multi-protocol-flow`](https://github.com/multi-protocol-flow) organisation (redis, db, mongo, grpc, kafka, smtp, mcp, llm, agent, openapi, postman, eolink). See [CONTRIBUTING.md](./CONTRIBUTING.md#repository-layout) for the list.

**Plugin authors:** start with [docs/plugin-sdk.md](./docs/plugin-sdk.md), the wire protocol in [docs/plugin-protocol.md](./docs/plugin-protocol.md), the performance notes in [docs/plugin-perf.md](./docs/plugin-perf.md), and the runnable templates in [`sdk-templates/`](./sdk-templates).

## 📖 Usage

### Visual editor

1. **Create a flow**: click *New Flow* in the left sidebar
2. **Add nodes**: drag nodes from the palette onto the canvas
3. **Connect nodes**: drag from an output port to another node's input port
4. **Configure**: select a node and edit it in the right-hand panel
5. **Run**: click *Run* in the toolbar
6. **Inspect**: open the detailed report when execution finishes

### Flow file example

Flow files use the **FlowFile wrapper format** with three top-level keys: `_version` (schema version), `flow` (the flow itself), and `_meta` (metadata):

```json
{
  "_version": 1,
  "flow": {
    "uuid": "flow-001",
    "name": "Example flow",
    "initial_variables": {
      "api_base": "https://api.example.com"
    },
    "nodes": [
      {
        "uuid": "entry-001",
        "type": "entry",
        "name": "Entry",
        "position": { "x": 0, "y": 0 },
        "data": { "type": "entry" }
      },
      {
        "uuid": "http-001",
        "type": "http:request",
        "name": "Fetch users",
        "position": { "x": 200, "y": 0 },
        "data": {
          "type": "http:request",
          "url": "{{api_base}}/users",
          "method": "get",
          "timeout_ms": 30000
        }
      }
    ],
    "connections": [
      {
        "id": "conn-001",
        "source_node_uuid": "entry-001",
        "target_node_uuid": "http-001",
        "source_port_id": "out",
        "target_port_id": "in"
      }
    ]
  },
  "_meta": null
}
```

### Node types

| Type id | Description | Purpose |
|---------|-------------|---------|
| `entry` | Entry node | Flow start; exactly one per flow |
| `http:request` | HTTP request node | GET/POST/PUT/DELETE/PATCH with JSON, form, and file bodies |
| `conditional` | Conditional branch | Routes execution by expression |
| `script` | Script node | Runs JavaScript for data extraction, variable manipulation, and assertions |
| `flow:sub` | Subflow node | Runs another `.mpf` flow inline |
| `http:ingress` / `http:respond` | Mock service entry/exit | Serves a flow over HTTP (`mpe serve`) |

Type ids are namespaced `<protocol>:<operation>`, the same scheme plugin nodes use
(`db:query`, `grpc:call`, …). A node file written against the pre-0.4 ids
(`http`, `ws_connect`) validates as an unknown node type.

#### Script node

Script nodes run JavaScript with a rich `fn.*` API:

- `fn.variables.*` — variable access
- `fn.response.*` — response data
- `fn.console.*` — logging
- `fn.flow.*` — flow control
- `fn.util.*` — helpers (encoding, crypto, time, UUID, …)
- `fn.expect()` — chained assertions

Full API reference: [docs/script-node.md](./docs/script-node.md)

### Variable references

`{{name}}` is resolved against the flow's variable storage by **bare key** — there is
no namespace prefix. Nested paths walk into objects (`{{user.profile.name}}`);
index access (`[0]`) and function calls are rejected.

| Syntax | Meaning |
|--------|---------|
| `{{api_base}}` | A variable carried by the flow (initial variables, environment file, `--var`, or a node that wrote it) |
| `{{user.id}}` | Nested path inside an object-valued variable |
| `{{$uuid}}`, `{{$timestamp}}`, `{{$randomInt}}` | Built-in dynamic generators, evaluated at run time |
| `{{$report.status}}` | Main-stage report, available **only in Teardown** |

An unknown key fails the node (`Variable not found: <key>`) rather than substituting
an empty string. Response data from a previous node is *not* injected as variables —
read it through a `variable:extractor` node (JSONPath → variables) or a `script` node
via `fn.response.*`.

#### More documentation

CLI usage in depth: [CLI_USAGE.md](./CLI_USAGE.md)

## 🧪 Testing

> **Where tests live:**
>
> | Location | Type | Notes |
> |----------|------|-------|
> | `mod tests` inside each crate | Unit tests | Inline unit tests next to the code |
> | `src-tauri/tests/*.rs` (15 files) | Integration tests | End-to-end flows, CLI, reports |
> | `src-tauri/crates/**/tests/*.rs` | Integration tests | Per-protocol integration suites |
> | `tests/` (repo root) | Fixtures | Flow JSON files, certificates, test servers |

### Backend tests

> **Test runner:** [cargo-nextest](https://nexte.st) (`cargo nextest run`) — process-per-test parallelism, output capture, retries, per-test grouping. nextest does not support doctests, so run those with `cargo test --doc`.

```bash
# Enter the backend directory
cd src-tauri

# ── Everything ────────────────────────────────────────────────
cargo nextest run                  # all tests (unit + integration)
cargo nextest run --nocapture      # stream println output (implies serial)
cargo nextest run --test-threads 4 # limit parallel test processes

# ── Doctests (not supported by nextest) ───────────────────────
cargo test --doc

# ── Unit tests only ───────────────────────────────────────────
cargo nextest run --lib            # every crate's unit tests

# ── Single crate ──────────────────────────────────────────────
cargo nextest run -p flow-engine-core --lib        # core unit tests
cargo nextest run -p flow-engine-executor          # executor
cargo nextest run -p flow-engine-http              # HTTP protocol
cargo nextest run -p flow-engine-assertion         # assertions
cargo nextest run -p flow-engine-ws                # WebSocket
cargo nextest run -p flow-engine-script            # script
cargo nextest run -p flow-engine-sse               # SSE
cargo nextest run -p flow-engine-graphql           # GraphQL
cargo nextest run -p flow-engine-tcp               # TCP/UDP
cargo nextest run -p mpe-cli                       # CLI

# Protocol plugins (db/mcp/grpc/kafka/redis/mongo/smtp/llm/agent…)
# are separate repositories; clone one and run:
#   cargo nextest run --manifest-path <plugin>/Cargo.toml

# ── By test name ──────────────────────────────────────────────
cargo nextest run cli_tests                # CLI integration tests
cargo nextest run proptest                 # property tests
cargo nextest run variable                 # variable handling
cargo nextest run report                   # reports
cargo nextest run ws_debug                 # WebSocket debug streams
cargo nextest run http_executor            # HTTP executor
cargo nextest run assertion_executor       # assertion executor
cargo nextest run dependency_test          # dependency detection
cargo nextest run report_storage           # report storage

# ── By module path (unit tests) ───────────────────────────────
cargo nextest run -p flow-engine-core --lib 'variable::tests::'
cargo nextest run -p flow-engine-core --lib 'node::tests::'
cargo nextest run -p flow-engine-core --lib 'context::tests::'

# ── Specific integration test files ───────────────────────────
cargo nextest run --test integration                # main integration suite
cargo nextest run --test cli_tests                  # CLI integration
cargo nextest run --test dependency_test            # dependency detection
cargo nextest run --test variable_tracking_test     # variable tracking
cargo nextest run --test ws_debug_flow_test         # WS debug stream

# ── Other combos ──────────────────────────────────────────────
cargo nextest run --lib --nocapture              # unit tests with live output
cargo nextest run -p flow-engine-core            # core incl. integration
cargo nextest run --workspace                    # every workspace package

# ── Tests that need external services ─────────────────────────
# Some HTTP / WebSocket tests need a multi-protocol echo server and are
# marked #[ignore]. That echo service is not part of this repository
# (/echo/ is git-ignored). Start an equivalent service, then use:
#   cargo nextest run --include-ignored
#   cargo nextest run -p flow-engine-http --include-ignored
#   cargo nextest run -p flow-engine-ws --include-ignored
#   cargo nextest run cli_tests --include-ignored
```

### Frontend tests

```bash
# Unit tests
npm run test

# Type checking
npm run typecheck
```

## 📁 Repository structure

```
multi-protocol-flow-executor/
├── src/                        # Frontend (React + TypeScript)
│   ├── components/
│   │   ├── canvas/             # Flow canvas, custom nodes and edges
│   │   ├── panels/             # Node configuration panels
│   │   ├── sidebar/            # Sidebar
│   │   ├── execution/          # Execution and report UI
│   │   ├── dialogs/            # Dialogs
│   │   └── ui/                 # Base UI primitives (shadcn/ui)
│   ├── stores/                 # Zustand stores
│   ├── services/               # Tauri IPC and storage services
│   ├── types/                  # TypeScript types
│   ├── hooks/                  # Custom hooks
│   ├── layouts/                # Layout components
│   ├── i18n/                   # Localisation catalogues
│   └── lib/                    # Utilities and registries
│
├── src-tauri/                  # Backend (Rust workspace)
│   ├── src/                    # Tauri app layer (the `app` crate only)
│   │   ├── lib.rs              # Library entry + invoke_handler registration
│   │   ├── main.rs             # Single binary: args → CLI, no args → GUI
│   │   ├── tauri_commands.rs   # Wrappers for the 88 IPC commands
│   │   ├── updater.rs          # Update checks, download, install
│   │   └── plugin_protocol.rs  # Plugin frontend custom protocol
│   ├── crates/                 # Workspace crates
│   │   ├── flow-engine-core/       # Base types, variables, context
│   │   ├── flow-engine-executor/   # Engine: orchestration, debug, stress, ingress
│   │   ├── flow-engine-{http,ws,tcp,sse,graphql,script,assertion}/  # Protocol executors
│   │   ├── flow-engine-plugin/     # Plugin host (sidecar lifecycle)
│   │   ├── flow-engine-plugin-marketplace/  # Marketplace client + installer
│   │   ├── flow-engine/            # Facade crate (re-exports everything)
│   │   └── mpe-cli/                # Pure Rust CLI binary
│   ├── tests/                  # Integration tests
│   ├── Cargo.toml              # Workspace + app manifest
│   └── tauri.conf.json         # Tauri configuration
│
├── tests/                      # Test fixtures
│   ├── cli/                    # CLI test cases
│   │   ├── 01_simple_entry_http.json
│   │   ├── 02_http_all_methods.json
│   │   ├── ...
│   │   └── edge_cases/         # Edge cases
│   ├── certs/                  # Self-signed TLS fixtures (see its README)
│   └── fixtures/
│       └── flows/              # Flow fixtures
│
├── docs/                       # Documentation
│   ├── executor_architecture.md # Architecture design
│   ├── node_structure.md       # Node structure
│   ├── plugin-*.md             # Plugin architecture / SDK / wire / performance
│   └── executor/               # Executor internals
│
├── sdk-templates/              # Runnable plugin templates (Python / Node.js)
├── README.md                   # This file (English)
├── README.zh-CN.md             # Simplified Chinese README
├── CONTRIBUTING.md             # Contribution guide and DCO
├── SECURITY.md                 # Security policy
├── CLI_USAGE.md                # CLI reference
├── CHANGELOG.md                # Release notes (single source of truth)
├── LICENSE                     # MIT
└── AGENTS.md                   # Project knowledge base
```

> The protocol plugins (redis / db / mongo / grpc / kafka / smtp / mcp / llm / agent / openapi / postman / eolink)
> are separate repositories and are not part of this working tree (`/plugins/` is git-ignored).
> The full list is in [CONTRIBUTING.md](./CONTRIBUTING.md#repository-layout).

## 🔧 Development

### Frontend

- React 19 + TypeScript
- State: Zustand
- UI: shadcn/ui + Tailwind CSS
- Canvas: @xyflow/react
- Forms: react-hook-form + zod

```bash
npm run dev                        # dev server
npm run typecheck                  # tsc --noEmit
npm run test                       # vitest
```

### Backend

- Rust 2021 edition
- The engine is modular: implement the `NodeExecutor` trait to add node types
- Lint with `cargo clippy`

**Formatting:**

- Frontend: ESLint + Prettier
- Backend: `cargo fmt`
- `unsafe_code` is forbidden

### Adding a node type

1. **Backend**: add the node's string id constant in `src-tauri/crates/flow-engine-core/src/node_types.rs`
   (ids are namespaced `<protocol>:<operation>`, e.g. `myproto:call`)
2. **Backend**: add the matching `NodeData` variant in `src-tauri/crates/flow-engine-core/src/node_config.rs`
3. **Backend**: implement the `NodeExecutor` trait in a new `*_executor.rs`:

```rust
#[async_trait::async_trait]
impl NodeExecutor for MyNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail>;
}
```

4. **Backend**: register the `NodePlugin` in `default_registry()` in `src-tauri/crates/flow-engine-executor/src/default_registry.rs`
5. **Frontend**: add the node component under `src/components/canvas/nodes/`
6. **Frontend**: add the config panel under `src/components/panels/`
7. **Frontend**: register in all three registries — `src/lib/NodeRegistry.ts`, `src/lib/configPanelRegistry.ts`, `src/lib/viewerRegistry.tsx`
8. Add unit and integration tests

Third-party node types do not need any of this — ship a [plugin](#plugins) instead.

### Conventions

- TypeScript is `strict`; `as any`, `@ts-ignore`, and `@ts-expect-error` are rejected
- Only ES2020 runtime APIs are allowed in the frontend (see `AGENTS.md` for the reasoning)
- New i18n keys must be added to both `zh-CN` and `en-US` and to `i18n-types.ts`
- No `unwrap()` in production Rust paths

## 🛠️ Tech stack

| Layer | Technology |
|-------|------------|
| Frontend framework | React 19 + TypeScript |
| State management | Zustand |
| UI components | shadcn/ui + TailwindCSS 4 |
| Flow canvas | @xyflow/react |
| Desktop framework | Tauri 2.x |
| Backend language | Rust |
| HTTP client | reqwest |
| Expression evaluation | evalexpr |

## 📄 License

[MIT](./LICENSE) © MPE Flow contributors.

This repository is MIT-licensed. The protocol plugins live in their own
repositories; their licences are defined and documented there, not here.

## 🤝 Contributing

Read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a pull request — commits must carry a DCO sign-off (`git commit -s`). For security issues, do **not** open a public issue; follow [SECURITY.md](./SECURITY.md).

---

**Architecture deep-dive: [docs/executor_architecture.md](./docs/executor_architecture.md)**  
**Project knowledge base: [AGENTS.md](./AGENTS.md)**
