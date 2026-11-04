# `flow:delay` 延时节点 — 设计与实现规范

**状态**：草案（待实现） | **影响面**：`flow-engine-core` / `flow-engine-executor` / 前端三注册表

**相关**：循环间隔见 [`docs/flow-loop.md`](./flow-loop.md) §3.6

---

## 1. 背景

当前无法在流程中"主动等一会儿"：

| 现有能力 | 局限 |
| :--- | :--- |
| 节点 `retry_delay_ms`（`flow-engine-core/src/node_config.rs:178`） | 仅在**失败重试**时生效，不能表达正常路径上的等待 |
| `script` 节点的 `fn.util.sleep(ms)` | 需要写脚本、跑 QuickJS，语义被"脚本"遮蔽，且无法被图形化表达 |
| `flow:loop` 的 `interval_ms` | 只作用于**迭代之间**，不能用于图的任意位置 |

典型需求：等外部系统就绪、接口限流、给异步任务留出落库时间、演示节奏。

---

## 2. 与 `flow:loop` 间隔的分工

| | `flow:delay` | `flow:loop.interval_ms` |
| :--- | :--- | :--- |
| 位置 | 图中任意位置 | 循环的迭代之间 |
| 次数 | 执行到就等一次 | 每轮之间自动等待 |
| 退避 | 无（单次等待，无需退避） | 支持 `interval_backoff` |
| 典型场景 | "发完通知等 2 秒再断言" | "每 3 秒轮询一次任务状态" |

两者不重叠：循环不需要在 body 里放 delay 节点（那会让 body 带着节奏耦合，换到别的循环就多等一次）。

---

## 3. 节点契约

- 类型 ID：`flow:delay`
- 分类：`logic`
- 端口：`out`（单出）
- 图标：`timer`，颜色沿用 `logic` 分类

### 3.1 配置

```rust
// flow-engine-core/src/node_config.rs

/// 延时节点配置
pub struct DelayConfig {
    /// 等待时长（毫秒），支持变量，如 "{{wait_ms}}"
    pub duration_ms: String,
    /// 随机抖动上限（毫秒），实际等待 = duration + rand(0..=jitter)，默认 0
    pub jitter_ms: u64,
}
```

`NodeData` 新增变体：

```rust
/// 延时节点配置
#[serde(rename = "flow:delay")]
Delay(DelayConfig),
```

### 3.2 默认值与限制

| 参数 | 默认 | 限制 |
| :--- | :--- | :--- |
| `duration_ms` | `1000` | 解析后须为 `0..=3_600_000`（1 小时），越界立即报错（校验期与执行期都会拦） |
| `jitter_ms` | `0` | `0..=60000`（校验期拦截；执行期用饱和加法，不会溢出） |

`duration_ms` 用变量时，解析失败（非数字 / 负数 / 越界）→ 节点返回错误，按节点 `on_error` 策略处理（`flow:delay` 只有 `out` 一个端口，没有 `false` 分支）。

---

## 4. 执行语义

```rust
// 1. 变量解析
let raw = replace_variables(&cfg.duration_ms, ctx)?;
let ms: u64 = raw.trim().parse().map_err(|_| ErrorDetail::invalid_config(...))?;
if ms > MAX_DELAY_MS {           // 3_600_000，超限直接报错而不是静默钳制
    return Err(ErrorDetail::invalid_config(...));
}

// 2. 抖动（workspace 为 rand 0.8，无 random_range）
let jitter = if cfg.jitter_ms > 0 {
    rand::thread_rng().gen_range(0..=cfg.jitter_ms)
} else {
    0
};

// 3. 可中断等待（ExecutionContext::wait_or_cancelled）
if !ctx.wait_or_cancelled(ms.saturating_add(jitter)).await {
    return Err(ErrorDetail::cancelled("flow:delay"));
}
```

| 规则 | 说明 |
| :--- | :--- |
| 可中断 | 取消信号即时响应，不睡满 |
| 计入流程耗时 | 等待即节点耗时，`duration_ms` 如实上报 |
| 与节点超时交互 | 若节点配置了 `timeout_ms` 且小于等待时长，会被 executor 的包装超时（`executor.rs:836`）中断 → 节点失败。**校验期给警告**："等待时长超出节点超时" |
| 不占用连接/资源 | 纯 `sleep`，不持有任何 provider |

### 4.1 输出

```jsonc
{ "requested_ms": 2000, "waited_ms": 2003, "jitter_ms": 0 }
```

`waited_ms` 用实测值（含调度误差），便于排查"为什么慢"。

---

## 5. 压测语义

`Main` 阶段出现 `flow:delay` 会直接拉低 QPS 且不属于被测服务的延迟。处理：

- 指标不排除该节点耗时（保持真实端到端），但报告标注 `delay_in_main: true`；
- 若启用"纯度模式"（P2 可选），压测时把 `duration_ms` 强制置 0 并在报告中记录被改写次数。

---

## 6. 前端

| 文件 | 内容 |
| :--- | :--- |
| `src/components/panels/DelayConfig.tsx` | 时长输入（数字 + 单位提示）+ 抖动输入 + 变量按钮 |
| `src/components/panels/schemas.ts` | `delaySchema`（字符串数字或变量模板） |
| `src/lib/NodeRegistry.ts` | `flow:delay` 元数据 |
| `src/lib/configPanelRegistry.ts` | `flow:delay` → `DelayConfigPanel` |
| `src/lib/viewerRegistry.tsx` | **不注册**，走 `GenericViewer` 兜底（只有一个等待时长，不值得专用 viewer） |

后端：`flow-engine-executor/src/delay_executor.rs` + `default_registry.rs` 注册。

---

## 7. 验证计划

| 用例 | 断言 |
| :--- | :--- |
| 固定时长 | `duration_ms=200`，耗时在 200~400ms |
| 抖动 | `duration=100, jitter=100`，重复 20 次命中区间且不全相等 |
| 变量时长 | `"{{wait_ms}}"` 解析生效 |
| 非法值 | `"abc"` / `-1` / `4000000` → 节点失败，错误含原值 |
| 取消 | `duration=60000` 时取消，1 秒内返回 |
| 超时冲突 | `duration=5000` + 节点 `timeout_ms=1000` → 超时失败 |
| 报告 | `output_data.waited_ms` 与 `duration_ms` 同量级 |

---

## 8. 非目标

- 定时调度 / cron（属于调度器领域，不是图内节点）
- 长于 1 小时的等待（应改用外部调度，避免长占用流程会话）
