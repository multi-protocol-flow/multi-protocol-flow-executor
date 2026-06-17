//! mpe-cli — Multi-Protocol Executor CLI 库入口
//!
//! 供 `app` crate 的 main.rs 在 GUI + CLI 二合一模式下调用。
//! 也作为 `mpe-cli` 纯 CLI 二进制的主入口。

pub mod cli;

pub use cli::run_cli;
