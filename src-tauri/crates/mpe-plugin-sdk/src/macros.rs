//! The `mpe_plugin_main!` entry-point macro.
//!
//! Generates a `fn main` that builds a tokio runtime, runs the SDK event
//! loop with the plugin, and maps the outcome to the process exit code.
//! The plugin type must implement [`Plugin`](crate::runtime::Plugin) and
//! `Default` (the macro constructs it via `Default::default()`).

/// Wires a plugin's node implementations into the runtime event loop.
///
/// # Example
///
/// ```ignore
/// use mpe_plugin_sdk::prelude::*;
///
/// #[derive(Default)]
/// struct MyPlugin;
///
/// impl Plugin for MyPlugin {
///     fn describe(&self) -> Vec<NodeDescription> { /* ... */ }
///     fn execute(&self, ctx: &mut ExecuteContext) -> impl Future<Output = ExecuteResult> + Send {
///         /* ... */
///     }
/// }
///
/// mpe_plugin_main!(MyPlugin);
/// ```
///
/// The generated `main` exits with code `0` on a clean shutdown and `1` on
/// a fatal runtime error.
#[macro_export]
macro_rules! mpe_plugin_main {
    ($plugin_type:ty) => {
        fn main() {
            std::process::exit($crate::__plugin_main::<$plugin_type>());
        }
    };
}
