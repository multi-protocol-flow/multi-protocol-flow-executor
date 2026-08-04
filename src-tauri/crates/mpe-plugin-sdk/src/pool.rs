//! Keyed connection pool for plugins.
//!
//! Maps a `key` to a long-lived `Arc<dyn Any + Send + Sync>` connection
//! shared across executions within one plugin process.
//! Implementation lands in M5.
