//! Flow Engine Script sub-crate
//!
//! 脚本节点 API 和执行器模块，提供 `QuickJS` 运行时桥接和脚本节点执行功能。

pub mod script_api;
pub mod script_executor;

#[cfg(test)]
mod script_poc;
