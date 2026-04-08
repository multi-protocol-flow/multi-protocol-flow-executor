//! 断言模块 - 提供响应断言功能
//!
//! 本模块实现响应断言功能，支持：
//! - 多种断言操作符（等于、包含、匹配等）
//! - 预设断言模板
//! - 断言求值器

pub mod evaluator;
pub mod operator;
pub mod preset;

// Re-export 公共类型
pub use evaluator::AssertionEvaluator;
// AssertionOperator 定义在 node_config.rs 中，这里仅 re-export 操作符求值函数
pub use operator::{evaluate_operator, get_json_type};
pub use preset::AssertionPreset;
