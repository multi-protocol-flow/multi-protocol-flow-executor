//! MPE plugin SDK — the crate plugin authors link against.
//!
//! Depends on `mpe-protocol` (the shared wire contract) but deliberately
//! NOT on `flow-engine-core` or `flow-engine-plugin` (host types): plugins
//! are sidecar subprocesses that speak JSON-RPC 2.0 over stdio, and must
//! never inherit host internals.
//!
//! # Quick start
//!
//! ```ignore
//! use mpe_plugin_sdk::prelude::*;
//!
//! #[derive(Default)]
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn describe(&self) -> Vec<NodeDescription> {
//!         vec![NodeDescription::new("vendor:my-node", "My Node")]
//!     }
//!     fn execute(&self, ctx: &mut ExecuteContext) -> impl Future<Output = ExecuteResult> + Send {
//!         async move { ExecuteResult::ok(ctx.config().clone()) }
//!     }
//! }
//!
//! mpe_plugin_main!(MyPlugin);
//! ```
//!
//! `mpe_plugin_main!` generates `fn main`, which runs the async event loop:
//! framed requests/notifications from stdin, framed responses to stdout.
//! The plugin process is hosted by `flow-engine-plugin` (host side).

pub mod context;
pub mod macros;
pub mod pool;
pub mod prelude;
pub mod runtime;
pub mod types;

use crate::runtime::Plugin;

/// Builds a multi-threaded tokio runtime and runs the plugin event loop,
/// returning the process exit code (`0` = clean shutdown). Invoked by
/// [`mpe_plugin_main!`]; hidden from the public API surface. The plugin
/// type must implement [`Plugin`] and `Default`.
#[doc(hidden)]
pub fn __plugin_main<P: Plugin + Default>() -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("mpe-plugin-sdk: failed to build tokio runtime: {err}");
            return 1;
        }
    };
    match runtime.block_on(runtime::run(P::default())) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("mpe-plugin-sdk: runtime error: {err}");
            1
        }
    }
}
