# flow-engine-assertion 源码知识库

**更新:** 2026-06-12 | **Layer:** 1 (协议) | **依赖:** flow-engine-core

## 概述

Assertion 协议 crate。7 个源文件，约 3,000 行。提供断言节点的执行、15 种操作符求值、预设模板和 all/any 组合模式。

## 文件结构

| 文件 | 行数 | 职责 |
|------|------|------|
| `lib.rs` | 7 | 模块声明 |
| `plugin.rs` | 98 | `NodePlugin` trait 实现，端口路由 (true/false)，默认配置 |
| `assertion_executor.rs` | 1,226 | `NodeExecutor` 实现，预设合并、数据源解析、失败行为 (Throw/RouteFalse)、调试事件 |
| `assertion/operator.rs` | 981 | 15 个操作符求值 + 正则 ReDoS 防护 (100ms) |
| `assertion/evaluator.rs` | 818 | `AssertionEvaluator`，JSONPath 提取 (RFC 9535)，all/any 模式判断 |
| `assertion/preset.rs` | 392 | 5 个预设模板 (http_success/client_error/server_error, json_response, response_fast) |
| `assertion/mod.rs` | 16 | 子模块 re-export |

## 核心逻辑

- **操作符 (operator.rs):** `eq, ne, gt, gte, lt, lte, contains, not_contains, matches, exists, not_exists, in, not_in, type_is, between`。严格类型比较，无隐式转换。数值操作要求 f64；正则匹配带 100ms 超时防 ReDoS。
- **求值器 (evaluator.rs):** `extract_target()` 支持完整 JSONPath 语法 (`$.a.b[0][*][?(@.x>1)]`)。`evaluate_all()` 按 mode 汇总：All 要求全部通过，Any 要求至少一个通过。`continue_on_failure` 控制失败后是否继续执行后续断言。
- **执行器 (assertion_executor.rs):** 读取 `AssertionConfig`，合并 preset 和自定义断言，路由到 true/false 端口。`OnFailureMode::Throw` 直接中断流程，`RouteFalse` 走 false 端口。
- **插件 (plugin.rs):** 从输出 `passed` 字段动态决定路由端口。
