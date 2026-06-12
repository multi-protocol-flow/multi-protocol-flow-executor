# flow-engine-executor/src

**Generated:** 2026-06-12 | **Layer 2 (Orchestration)**

## OVERVIEW

Layer 2 orchestrator crate. 28 source files, ~15,717 lines. Largest crate in the workspace. Hosts the main flow execution loop, conditional branching, flow validation, file storage, debug mode with breakpoints, stress testing engine, and business command handlers. Depends on `flow-engine-core` (type system, traits) and optionally on all 16 protocol crates via `protocol-*` feature flags.

## STRUCTURE

```
src/
├── executor.rs                  # FlowExecutor: main execution loop
├── conditional_executor.rs      # Condition evaluation and branch routing
├── entry_executor.rs            # Entry node: flow start logic
├── variable_extractor_executor.rs # Variable extraction from responses
├── validator.rs                 # Flow structure and config validation
├── storage.rs                   # Flow file read/write and persistence
├── provider_builder.rs          # DRY: protocol provider construction
├── hook_runner.rs               # Webhook/Script/Notify hook execution
├── csv_parser.rs                # Multi-encoding CSV parser for datasets
├── command.rs                   # Shared I/O types for CLI and IPC
├── debug_event_adapter.rs       # Debug event translation layer
├── default_registry.rs          # Registers all 14+ built-in node plugins
├── plugin_entry.rs              # Entry node plugin (NodePlugin trait)
├── plugin_conditional.rs        # Conditional node plugin
├── plugin_end.rs                # End node plugin
├── plugin_variable_extractor.rs # Variable extractor plugin
├── plugin_proxy.rs              # Proxy settings plugin
├── commands/                    # Business command handlers (6 files)
│   ├── execute_flow.rs, create_flow.rs, update_node.rs
│   ├── delete_node.rs, report_commands.rs
├── debug/                       # Debug execution (3 files)
│   ├── executor.rs, types.rs
├── stress_test/                 # Stress testing (7 files)
│   ├── executor.rs, config.rs, types.rs, command.rs
│   ├── metrics.rs, report.rs
└── dataset_executor/            # Dataset-driven execution (5 files)
    ├── executor.rs, session.rs, types.rs, events.rs
```

## WHERE TO LOOK

| Task | File |
|------|------|
| Modify execution loop (traversal, error handling) | `executor.rs` |
| Add or change node plugin registration | `default_registry.rs` |
| Add new business command (IPC/CLI handler) | `commands/` (new file + `mod.rs`) |
| Debug mode: breakpoints, step, continue, inspect | `debug/executor.rs` |
| Stress test: concurrency, ramp-up, metrics | `stress_test/` |
| Data-driven execution (CSV rows into flow) | `dataset_executor/` |
| Change condition evaluation or branching logic | `conditional_executor.rs` |
| Flow validation rules | `validator.rs` |
| Flow file I/O format or storage paths | `storage.rs` |
| CSV parsing or encoding detection | `csv_parser.rs` |
| Protocol provider construction | `provider_builder.rs` |

## KEY LOGIC

**Plugin architecture.** All node types backed by `NodePlugin` trait (defined in `flow-engine-core/src/plugin.rs`). `NodeRegistry` maps `type_id` strings to plugin instances. The old match-based dispatch is replaced by registry lookup: `registry.get(node_type).execute(...)`. To add a new node type: implement `NodePlugin`, register it in `default_registry.rs`.

**FlowExecutor.** Main loop (`executor.rs`): starts at entry node, follows connections by port ID, executes each node via plugin registry, passes context with variable storage and log sender. Handles conditional branching, end node termination, and error propagation. Produces `ExecutionResult` with per-node reports.

**DebugFlowExecutor.** (`debug/executor.rs`) Wraps the same plugin architecture with debug primitives: set breakpoints on node UUIDs, step through execution one node at a time, continue to next breakpoint, inspect variables mid-execution. Communicates via `mpsc` channels for `DebugCommand`/`DebugEvent`. Sends heartbeats every 5 seconds.

**Stress testing.** (`stress_test/executor.rs`) Runs a flow concurrently across N virtual users. Linear ramp-up: `concurrent_users = floor(target * elapsed / ramp_up)`. Collects per-request latency samples, computes p50/p99/p999 via sorted slice percentile (`stress_test/metrics.rs`). Generates `StressTestReport` with error samples and response time distribution.

**Conditional executor.** Evaluates expressions via `evalexpr` against the current variable context. Supports `equals`, `contains`, `regex`, `gt`/`gte`/`lt`/`lte`, `exists`, `true`/`false` operators. Routes to the matching branch port.

## COMPILE IMPACT

| Modified | Rebuilds |
|----------|----------|
| `flow-engine-executor/src/` | `flow-engine` + `app` (~2 crates) |
| `default_registry.rs` | Same scope (registry is internal wiring, no downstream deps) |
| `debug/executor.rs` or `stress_test/` | Same scope |
| Adding a new command in `commands/` | Same scope |
