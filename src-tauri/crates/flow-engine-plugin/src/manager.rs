//! `PluginManager`: discovery + lifecycle + registry registration.
//!
//! Owns the plugin directory, the per-plugin connections, crash/restart
//! policy, and the pre-registration collision check against reserved
//! built-in ids. Implementation lands in M2b/M2c.
