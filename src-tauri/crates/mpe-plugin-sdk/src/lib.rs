//! MPE plugin SDK — the crate plugin authors link against.
//!
//! Depends on `mpe-protocol` (shared wire types) but deliberately NOT on
//! `flow-engine-core` (host types): plugins must not inherit host internals.
//! The runtime, execute context, connection pool, and the `mpe_plugin_main!`
//! macro are implemented in M5; this file declares the module skeleton.

/// Async JSON-RPC event loop: reads framed requests from stdin and writes
/// responses to stdout (pipelined, spawn-per-request).
pub mod runtime;
/// Per-execution context handed to plugin node implementations.
pub mod context;
/// Keyed pool of long-lived connections shared across executions.
pub mod pool;
/// `mpe_plugin_main!` entry-point macro.
pub mod macros;
/// Recommended re-exports for plugin authors
/// (`use mpe_plugin_sdk::prelude::*;`).
pub mod prelude;
