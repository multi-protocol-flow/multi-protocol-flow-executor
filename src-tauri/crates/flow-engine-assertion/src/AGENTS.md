# flow-engine-assertion

**Layer 1** | 6 文件 | ~2,946 行 | 依赖 core

## 概述

15 种断言操作符 + 断言节点执行器。支持 All/Any 模式和预设断言。

## 结构

```
flow-engine-assertion/src/
├── lib.rs                  # 模块导出
├── assertion_executor.rs   # AssertionNodeExecutor (实现 NodeExecutor, ~850 行)
└── assertion/              # 断言引擎
    ├── mod.rs              # 导出
    ├── operator.rs         # 15 种操作符实现 (~750 行)
    ├── evaluator.rs        # AssertionEvaluator 求值器 (~680 行)
    └── preset.rs           # 预设断言模板
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 断言执行逻辑 | `assertion_executor.rs` |
| 添加新操作符 | `assertion/operator.rs` |
| 修改求值逻辑 | `assertion/evaluator.rs` |
| 预设断言 | `assertion/preset.rs` |

## 操作符 (15 种)

| 类别 | 操作符 |
|------|--------|
| 比较 | `eq`, `ne`, `gt`, `gte`, `lt`, `lte` |
| 字符串 | `contains`, `matches` (正则) |
| 存在性 | `exists`, `not_exists` |
| 集合 | `in`, `not_in` |
| 范围 | `between` |
| 类型 | `is_type` |
| JSON | `json_path` |

## All/Any 模式

- **All**: 所有断言必须通过
- **Any**: 任一断言通过即可
- 结果路由到 true/false 端口

## 关键实现

- **操作符分发**: `evaluate_operator()` 按 Operator 枚举分发到各实现
- **JSON 路径**: 支持 `json_path` 操作符提取嵌套值
- **类型推断**: `get_json_type()` 判断值类型用于类型断言

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-assertion       # 仅测断言
cargo clippy -p flow-engine-assertion     # 仅 Lint 断言
```
