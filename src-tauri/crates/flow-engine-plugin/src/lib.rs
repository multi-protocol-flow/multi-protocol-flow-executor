//! Flow Engine Plugin — host-side sidecar plugin manager.
//!
//! Implements the host half of the MPE plugin protocol (see
//! `docs/plugin-protocol.md`): discovery, process lifecycle, JSON-RPC
//! transport over stdio, and the `NodePlugin` wrapper (`SidecarPlugin`).
//!
//! L1-style crate: depends on `flow-engine-core` for host types and on
//! `mpe-protocol` for the shared wire contract. Transport, lifecycle, and
//! manager logic land in M1+; this file declares the module skeleton only.

/// Describe RPC handshake and P0 frontend normalization.
pub mod describe;
/// Plugin directory scanning and discovery.
pub mod discovery;
/// Plugin error types.
pub mod error;
/// Line framing (`LF` delimiter, `2 MiB` cap) over child stdio.
pub mod framing;
/// Plugin process lifecycle (start, recycle, shutdown).
pub mod lifecycle;
/// `PluginManager`: scan + describe + quarantine + lifecycle + registration.
pub mod manager;
/// Plugin manifest (`plugin.json`) parsing and validation.
pub mod manifest;
/// Child process spawning hygiene (stderr drain, Job Object, CLOEXEC).
pub mod process;
/// JSON-RPC wire types shared with plugins (via `mpe-protocol`).
pub mod protocol;
/// Registration of describe-produced nodes into the host registry (collision
/// policy, M2c).
pub mod registration;
/// Reserved built-in node type ids (collision policy).
pub mod reserved;
/// Minimal semver parsing and `min_host_version` gating.
pub mod semver;
/// `SidecarPlugin`: the `NodePlugin` wrapper around a plugin connection.
pub mod sidecar;
/// Pipelined JSON-RPC transport over a child process's stdin/stdout.
pub mod transport;

#[cfg(test)]
mod registration_tests;
