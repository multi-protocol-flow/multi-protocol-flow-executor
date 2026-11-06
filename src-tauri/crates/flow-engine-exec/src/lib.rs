//! Flow Engine Exec - 本地命令（`local:exec`）与本机信息（`local:sysinfo`）节点执行器
//!
//! 两个节点都运行在宿主进程内（而非 sidecar 插件），因为只有宿主能在变量替换**之前**
//! 看到模板原文：`local:exec` 的防注入语义（`args` 逐参数传递、shell 模式禁止 `{{ }}`）
//! 依赖这一点。
//!
//! 模块职责：
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`policy`] | 执行策略 `ExecPolicy`（总开关 / 白名单 / shell 许可 / 审批模式） |
//! | [`approval`] | 审批请求、信任记录与命令哈希 |
//! | [`spec`] | 配置 → 可执行命令的解析（变量替换、环境清洗、路径解析） |
//! | [`process`] | 子进程启动、输出捕获、超时与取消（进程组终止） |
//! | [`media`] | 产物捕获（复用媒体预览的 `media` 契约） |
//! | [`sysinfo`] | 本机信息探测（只读，硬编码探针） |
//! | [`exec_plugin`] / [`sysinfo_plugin`] | 两个 `NodePlugin` 实现 |

// `ErrorDetail` 是节点插件的统一错误类型（`NodePlugin::execute` 的签名决定），
// 本 crate 的每个解析/执行函数都以它为 `Err`，逐个函数加 `#[allow]` 只会变成噪音。
#![allow(clippy::result_large_err)]

pub mod approval;
pub mod exec_plugin;
pub mod media;
pub mod policy;
pub mod process;
pub mod spec;
pub mod sysinfo;
pub mod sysinfo_plugin;

pub use approval::{ExecApprovalProvider, ExecApprovalRecord, ExecApprovalRequest};
pub use exec_plugin::LocalExecPlugin;
pub use policy::{
    policy_providers, ApprovalMode, ExecPolicy, EXEC_POLICY_PROVIDER, MAX_TIMEOUT_MS,
};
pub use sysinfo_plugin::LocalSysinfoPlugin;
