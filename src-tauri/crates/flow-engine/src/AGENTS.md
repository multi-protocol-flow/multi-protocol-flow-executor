# flow-engine (src/)

**Generated:** 2026-06-12 | **Branch:** dev | **Layer:** L3 (Facade)

## OVERVIEW

Layer 3 facade crate. Pure re-export layer with zero business logic. The `app` crate (src-tauri/) depends only on this crate. All sub-crates are re-exported here so consumers never import Layer 0/1/2 directly.

## STRUCTURE

- **`lib.rs`** — Massive re-export file (~150 lines). Re-exports from `flow-engine-core` (17 modules), `flow-engine-executor` (commands, executor, subgraph_runner, storage, validator, debug, stress_test, dataset, default_registry, test_utils), and the 9 protocol crates (feature-gated behind `protocol-*`). Also conditionally re-exports per-protocol types (Db, Ws, etc.) under the same feature gates.
- **`context.rs`** — Thin re-export of `ExecutionContext` and `VariableStorage` from `flow-engine-core::context` (2 lines, 2 types).

## CONVENTIONS

- No business logic. No new types, traits, or implementations.
- Every `pub use` must originate from a sub-crate (core, crypto, or executor).
- Adding a new protocol crate? Add a feature-gated re-export block in `lib.rs` using the `protocol-{name}` feature flag pattern.
- `#[cfg(test)]` blocks are acceptable for test utilities (see `test_utils`).

## COMPILE IMPACT

| Modified | Rebuilt |
|----------|---------|
| `flow-engine/src/lib.rs` or `context.rs` | app only (~1 crate) |

Changing this crate never triggers recompilation of executor, protocol crates, core, or crypto. Fastest possible rebuild surface.
