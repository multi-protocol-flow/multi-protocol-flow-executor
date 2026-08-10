//! Recommended prelude for plugin authors.
//!
//! `use mpe_plugin_sdk::prelude::*;` brings in everything a typical plugin
//! needs: the [`Plugin`] trait, the entry-point macro, the execution
//! context/result types, the node description types, the connection pool,
//! and the shared capability type.

pub use crate::context::ExecuteContext;
pub use crate::mpe_plugin_main;
pub use crate::pool::ConnectionPool;
pub use crate::runtime::Plugin;
pub use crate::types::{ExecuteResult, NodeDescription, PortDescription, PORT_KIND_IN, PORT_KIND_OUT};
pub use mpe_protocol::manifest::PluginCapabilities;
