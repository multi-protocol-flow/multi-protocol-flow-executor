//! Sample plugin exercising the SDK end-to-end.
//!
//! Implements `describe` (one `demo:echo` node) and `execute` (echoes the
//! resolved config back as `output_data`). Used by the `sdk_plugin_roundtrip`
//! integration test, which spawns this binary and speaks JSON-RPC over stdio
//! exactly as the host would.

use std::future::Future;

use mpe_plugin_sdk::prelude::*;

/// Echo plugin: returns the host-resolved config as `output_data`.
#[derive(Default)]
struct DemoPlugin;

impl Plugin for DemoPlugin {
    fn describe(&self) -> Vec<NodeDescription> {
        vec![NodeDescription::new("demo:echo", "Demo Echo")]
    }

    // The runtime spawns execute futures, so the future must be Send; an
    // `async fn` cannot express that bound on stable (see `Plugin`).
    #[allow(clippy::manual_async_fn)]
    fn execute(&self, ctx: &mut ExecuteContext) -> impl Future<Output = ExecuteResult> + Send {
        async move {
            ctx.log("info", "demo:echo executing");
            ExecuteResult::ok(ctx.config().clone())
        }
    }
}

mpe_plugin_main!(DemoPlugin);
