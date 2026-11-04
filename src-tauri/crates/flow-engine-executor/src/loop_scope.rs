//! 单次流程执行内所有 `flow:loop` 共享的运行时状态
//!
//! 迭代预算与嵌套层数都挂在 [`LoopScope`] 上，通过 `ExecutionContext` 的
//! provider 注册表以 [`LOOP_SCOPE_PROVIDER`] 为键共享。顶层流程执行创建根作用域
//! （见 `FlowExecutor::execute_flow`），loop 每进入一层 body 就派生子作用域，
//! 所有子作用域共享同一份剩余迭代额度。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// 单次流程执行内所有 loop 节点累计迭代次数上限
pub const GLOBAL_ITERATION_BUDGET: u32 = 1000;
/// body 内再放 loop 的最大嵌套层数
pub const MAX_LOOP_NESTING: u32 = 3;
/// provider 键：`Arc<LoopScope>`
pub const LOOP_SCOPE_PROVIDER: &str = "loop_scope";

/// 循环运行时作用域
#[derive(Clone, Debug)]
pub struct LoopScope {
    /// 剩余可用迭代次数（同一次流程执行内共享）
    remaining: Arc<AtomicU32>,
    /// 当前所处嵌套层数：顶层为 0，body 内为 1
    pub depth: u32,
}

impl LoopScope {
    /// 根作用域（每次流程执行创建一次）
    pub fn root() -> Self {
        Self {
            remaining: Arc::new(AtomicU32::new(GLOBAL_ITERATION_BUDGET)),
            depth: 0,
        }
    }

    /// 进入一层 body
    pub fn child(&self) -> Self {
        Self {
            remaining: Arc::clone(&self.remaining),
            depth: self.depth + 1,
        }
    }

    /// 申请一次迭代额度；用尽返回 `false`
    pub fn try_acquire(&self) -> bool {
        self.remaining
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_sub(1))
            .is_ok()
    }

    /// 剩余可申请额度（诊断/测试用）
    pub fn remaining(&self) -> u32 {
        self.remaining.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_starts_with_full_budget() {
        let root = LoopScope::root();
        assert_eq!(root.remaining(), GLOBAL_ITERATION_BUDGET);
        assert_eq!(root.depth, 0);
    }

    #[test]
    fn child_shares_remaining_budget() {
        let root = LoopScope::root();
        let child = root.child();
        assert_eq!(child.depth, 1);
        assert!(child.try_acquire());
        assert_eq!(root.remaining(), GLOBAL_ITERATION_BUDGET - 1);

        let grandchild = child.child();
        assert_eq!(grandchild.depth, 2);
        assert!(grandchild.try_acquire());
        assert_eq!(root.remaining(), GLOBAL_ITERATION_BUDGET - 2);
    }

    #[test]
    fn budget_exhausts() {
        let scope = LoopScope::root();
        for _ in 0..GLOBAL_ITERATION_BUDGET {
            assert!(scope.try_acquire());
        }
        assert!(!scope.try_acquire());
        assert_eq!(scope.remaining(), 0);
    }
}
