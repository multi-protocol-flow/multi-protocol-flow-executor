// 钩子执行器 trait 定义
//
// 定义钩子执行器接口，解耦核心逻辑与具体执行实现。
// 默认提供 NoopHookExecutor 用于测试和禁用钩子场景。

use crate::hook::{FlowHook, HookContext};

/// 钩子执行器 trait
///
/// 所有钩子执行实现必须实现此接口。
/// 提供 execute 单个钩子和 `execute_all` 批量执行的默认实现。
pub trait HookExecutor: Send + Sync {
    /// 执行单个钩子
    ///
    /// # 参数
    /// - `hook`: 钩子配置
    /// - `context`: 钩子执行上下文（流程 UUID、状态、耗时等）
    fn execute(&self, hook: &FlowHook, context: &HookContext);

    /// 批量执行所有匹配的钩子
    ///
    /// 遍历钩子列表，对每个启用且匹配当前状态的钩子调用 execute。
    /// 钩子执行Failed不应影响流程结果，仅记录Error日志。
    fn execute_all(&self, hooks: &[FlowHook], context: &HookContext) {
        for hook in hooks {
            if hook.should_trigger(&context.status) {
                self.execute(hook, context);
            }
        }
    }
}

/// 空操作钩子执行器（用于测试或禁用钩子）
pub struct NoopHookExecutor;

impl HookExecutor for NoopHookExecutor {
    fn execute(&self, _hook: &FlowHook, _context: &HookContext) {
        // 空操作，不执行任何钩子
    }
}
