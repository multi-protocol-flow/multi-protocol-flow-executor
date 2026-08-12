# flow-engine-core AGENTS.md

**生成时间:** 2026-06-12 | **Branch:** dev | **Lines:** ~17,500 | **Files:** 46

## OVERVIEW

Layer 0 leaf crate. Foundation of the entire workspace every other crate depends on. No workspace crate dependencies (only serde, serde_json, async-trait, tokio, regex, serde_json_path, base64, log, workspace-hack). Contains all shared type definitions, abstract traits (provider/executor/plugin), error classification, variable system, and node configuration structures. No protocol client implementations live here.

## STRUCTURE

```
src/
├── lib.rs              # Module declarations + re-exports
├── node.rs             # NodeType enum (89+ variants), Node struct, Port, PortType
├── node_config.rs      # NodeData enum (~14 protocol config structs + Entry/End/Conditional/Script/Assertion/VariableExtractor)
├── node_types.rs       # Node type string constants
├── flow.rs             # Flow struct (nodes + connections + initial_variables)
├── connection.rs       # Connection struct (source/target node references)
├── context.rs          # ExecutionContext (runtime variable store, proxy state)
├── variable.rs         # Variable / VariablePool / InitialVariable / OutputMapping
├── error.rs            # ErrorKind enum (fatal vs recoverable), ErrorDetail
├── executor.rs         # NodeExecutor trait (the core execution interface)
├── plugin.rs           # NodePlugin trait (validation + metadata + port definitions)
├── registry.rs         # NodeRegistry (maps NodeType to Box<dyn NodePlugin>)
├── report.rs           # FlowExecutionReport, NodeExecutionReport, ExecutionStatus
├── statistics.rs       # FlowStatistics (timing, node count, etc.)
├── execution_event.rs  # ExecutionEvent enum (real-time frontend push events)
├── ws_stream.rs        # WsConnectionStream, WsMessage, WsStreamSegment
├── debug_types.rs      # DebugSessionState, DebugCommand, DebugEvent, PauseReason
├── dataset.rs          # DatasetConfig, DataPickStrategy, DatasetExecutionReport
├── hook.rs             # FlowHook, HookAction, HookTrigger, HookContext
├── log_event.rs        # LogLevel, LogEvent
├── email_types.rs      # SmtpConfig, ImapConfig, EmailAuthConfig, EmailProvider
├── email_auth.rs       # EmailProviderPreset, OAuth2/SASL builders
├── utils.rs            # Shared utility functions
└── traits/             # 20 provider trait files (ws, tcp, udp, mqtt, kafka, ...)
    ├── mod.rs          # Re-exports all traits + ProviderKey pattern
    ├── provider_key.rs # ProviderKey trait + marker types (Ws, Grpc, ...)
    ├── log_sender.rs   # LogSender, QueuedLogSender, ConsoleLogSender
    ├── *provider.rs    # One file per protocol: WsProvider, GrpcProvider, etc.
    └── app_dir.rs / debug_event_sender.rs / execution_event_sender.rs / directory_opener.rs / hook_executor.rs
```

## WHERE TO LOOK

| Task | File |
|------|------|
| Add new node type | `node.rs` (NodeType variant) + `node_config.rs` (NodeData variant + config struct) + `node_types.rs` (string constant) + `registry.rs` (register plugin) |
| Modify variable replacement | `variable.rs` (VariablePool, template resolution) + `context.rs` (ExecutionContext variable scopes) |
| Modify error classification | `error.rs` (ErrorKind + ErrorDetail) |
| Add/modify provider trait | `traits/` — create or edit `<protocol>_provider.rs` |
| Understand execution flow | `executor.rs` (NodeExecutor trait) + `plugin.rs` (NodePlugin trait) |
| Add runtime event for frontend | `execution_event.rs` (ExecutionEvent) |
| Modify report structure | `report.rs` (FlowExecutionReport, NodeExecutionReport) |
| Add debug feature | `debug_types.rs` (DebugCommand, DebugEvent, DebugSessionState) |
| Add flow hook | `hook.rs` (HookTrigger, HookAction, FlowHook) |

## KEY TYPES

- **NodeExecutor** (`executor.rs`): `async fn execute(...)` — the per-node execution contract. Takes node + context + connections, returns next node UUIDs + optional output.
- **NodePlugin** (`plugin.rs`): Defines node metadata, ports, and validation. Used for registration, not execution.
- **NodeRegistry** (`registry.rs`): `HashMap<String, Box<dyn NodePlugin>>` — maps node type name to plugin instance.
- **ErrorKind** (`error.rs`): Two-tier classification — `Fatal` (stops flow) vs `Recoverable` (continues). Includes typed error codes.
- **ExecutionContext** (`context.rs`): Runtime variable store with scoped resolution. Holds flow inputs, node outputs, proxy state.
- **NodeData** (`node_config.rs`): 14-protocol enum with ~120 config struct variants. All protocol-specific configuration.
- **NodeType** (`node.rs`): `#[serde(rename_all = "snake_case")]` enum with 89+ variants. Serialized with frontend.

## CONVENTIONS

- **No workspace crate dependencies.** Only external crates + workspace-hack.
- **Serde derives** (`Serialize`/`Deserialize`, `rename_all = "snake_case"`) on all public types for IPC serialization.
- **Types must be kept in sync** with `src/types/*.ts` on the frontend. Adding/removing fields requires paired changes.
- **ProviderKey pattern** (`traits/provider_key.rs`): Zero-sized marker types for protocol identification.
- **Abstract providers** (`traits/*.rs`): All protocol-specific operations are trait-based, keeping this crate free of runtime client dependencies.
- **Re-exports** in `lib.rs`: Critical types re-exported at crate root for ergonomic access.

## COMPILE IMPACT

**Full workspace rebuild.** Because every other crate depends on this one, any change here invalidates the entire dependency graph. Plan Core changes during off-peak hours or batch them to minimize iteration cycles.
