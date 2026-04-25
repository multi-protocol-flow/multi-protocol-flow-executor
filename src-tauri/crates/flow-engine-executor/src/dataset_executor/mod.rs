// 数据驱动执行模块
//!
//! 提供 CSV 数据驱动的流程执行能力。逐行读取 CSV 数据，
//! 将每行作为 runtime_variables 注入流程执行，收集每行的执行结果。

mod events;
mod executor;
mod session;
mod types;

pub use events::{DatasetEvent, generate_report, save_report};
pub use executor::DatasetExecutor;
pub use session::DatasetSessionManager;
pub use types::{DatasetExecutionConfig, FailureStrategy};
