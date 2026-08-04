//! Echo mock host — spawns the crate's `echo_mock` binary through the real
//! `process::spawn_plugin` path (stderr drain, Job Object, env injection),
//! exercising spawn hygiene end to end.
//!
//! The binary is located via `CARGO_BIN_EXE_echo_mock`, which cargo sets for
//! integration tests of this package.
//!
//! Each integration-test binary compiles this module separately and uses a
//! different subset of its helpers, so dead-code is expected per target.
#![allow(dead_code)]

use std::collections::HashMap;

use flow_engine_plugin::process::{spawn_plugin, PluginProcess};
use mpe_protocol::manifest::PluginEntry;

/// Env var that makes the echo mock flood its stderr before the loop
/// (proves the host drains stderr so the 64 KB pipe never stalls).
pub const FLOOD_STDERR_ENV: &str = "MPE_ECHO_MOCK_FLOOD_STDERR";
/// Env var selecting a one-shot / sleep mode.
pub const MODE_ENV: &str = "MPE_ECHO_MOCK_MODE";

/// Spawns the echo mock plugin process with no extra env.
pub fn spawn_echo_mock() -> Result<PluginProcess, flow_engine_plugin::error::PluginError> {
    spawn_echo_mock_with_env(&HashMap::new())
}

/// Spawns the echo mock plugin process with extra environment overrides.
pub fn spawn_echo_mock_with_env(
    env_overrides: &HashMap<String, String>,
) -> Result<PluginProcess, flow_engine_plugin::error::PluginError> {
    let entry = PluginEntry {
        command: env!("CARGO_BIN_EXE_echo_mock").to_string(),
        args: Vec::new(),
    };
    spawn_plugin(&entry, env_overrides)
}
