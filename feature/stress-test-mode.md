# 压测模式功能需求

**创建时间**: 2026-03-13
**状态**: 待实现

## 一、功能概述

为流程执行器增加压测模式，支持对完整流程进行并发压力测试。用户可配置并发用户数、预热时长、总运行时间等参数，实时监控压测指标，结束后生成 JSON 格式的性能报告。

## 二、核心功能

### 2.1 压测配置

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| 并发用户数 | number | 50 | 同时执行的虚拟用户数量 |
| 预热时长 | number | 10 | 线性增加到目标并发的时长（秒） |
| 总运行时长 | number | 60 | 压测总时长（秒） |

### 2.2 压测控制

| 操作 | 说明 |
|------|------|
| 开始压测 | 按配置启动压测会话 |
| 停止压测 | 手动终止压测，生成已收集数据的报告 |

### 2.3 实时监控（每秒推送）

| 指标 | 说明 |
|------|------|
| 已执行次数 | 累计完成的流程执行次数 |
| 成功次数 | 执行成功的次数 |
| 失败次数 | 执行失败的次数 |
| 当前 RPS | 当前每秒请求数 |
| 平均响应时间 | 平均流程执行耗时（毫秒） |
| 当前状态 | 预热中/运行中/已停止 |

### 2.4 性能报告

**文件命名**: `performance_{flow_name}_{timestamp}.json`

**存储位置**: `{appDataDir}/reports/`

**报告内容**:
- 压测配置快照
- 执行环境信息（OS、时间）
- 汇总统计（总次数、成功/失败数、成功率）
- 响应时间分布（平均值、最小值、最大值、P50/P90/P95/P99）
- 实际 RPS
- 错误分类统计
- 失败错误信息摘要（前 10 条）

### 2.5 执行行为

- **迭代间隔**: 无间隔，一次执行完成立即开始下一次
- **超时控制**: 复用现有节点的 `timeout_ms` 配置
- **预热策略**: 线性增加并发用户（如 10 秒预热，目标 50 并发，每秒增加 5 个用户）
- **失败处理**: 单次执行失败不中断压测，计入失败统计
- **崩溃处理**: 压测崩溃/中断直接结束，输出已收集数据的报告

## 三、数据结构变更

### 3.1 新增类型（后端）

#### StressTestConfig

```rust
pub struct StressTestConfig {
    pub concurrent_users: u32,      // 并发用户数
    pub ramp_up_seconds: u32,       // 预热时长（秒）
    pub duration_seconds: u32,      // 总运行时长（秒）
    pub flow_uuid: String,          // 流程 UUID
}
```

#### StressTestProgress

```rust
pub struct StressTestProgress {
    pub status: StressTestStatus,   // 当前状态
    pub elapsed_seconds: u64,       // 已运行时长
    pub total_iterations: u64,      // 已执行次数
    pub successful_iterations: u64, // 成功次数
    pub failed_iterations: u64,     // 失败次数
    pub current_rps: f64,           // 当前 RPS
    pub avg_latency_ms: f64,        // 平均响应时间
}
```

#### StressTestStatus

```rust
pub enum StressTestStatus {
    RampUp,     // 预热中
    Running,    // 运行中
    Stopped,    // 已停止
}
```

#### StressTestReport

```rust
pub struct StressTestReport {
    // 元信息
    pub flow_uuid: String,
    pub flow_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: u64,
    
    // 配置快照
    pub config: StressTestConfig,
    
    // 汇总统计
    pub total_iterations: u64,
    pub successful_iterations: u64,
    pub failed_iterations: u64,
    pub success_rate: f64,
    pub actual_rps: f64,
    
    // 响应时间分布
    pub avg_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p90_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    
    // 错误统计
    pub error_summary: HashMap<String, u64>,  // 错误类型 -> 数量
    pub error_samples: Vec<ErrorSample>,      // 错误样例（前 10 条）
}

pub struct ErrorSample {
    pub iteration: u64,
    pub node_uuid: String,
    pub node_name: String,
    pub error_message: String,
}
```

#### StressTestCommand / StressTestEvent

```rust
pub enum StressTestCommand {
    Start { config: StressTestConfig },
    Stop,
}

pub enum StressTestEvent {
    Started { config: StressTestConfig },
    Progress { data: StressTestProgress },
    Ended { report: StressTestReport },
}
```

### 3.2 新增类型（前端）

#### StressTestStore

```typescript
interface StressTestState {
  status: 'idle' | 'ramp_up' | 'running' | 'stopped';
  config: StressTestConfig | null;
  progress: StressTestProgress | null;
  report: StressTestReport | null;
  isLoading: boolean;
  error: string | null;
}
```

## 四、后端架构设计

### 4.1 目录结构

```
src-tauri/crates/flow-engine/src/
├── stress_test/
│   ├── mod.rs           # 模块入口
│   ├── config.rs        # StressTestConfig 配置
│   ├── executor.rs      # StressTestExecutor 主执行器
│   ├── types.rs         # 类型定义 (Progress, Report, Status 等)
│   ├── metrics.rs       # 指标收集器
│   └── command.rs       # 命令/事件处理
```

### 4.2 核心组件

#### StressTestExecutor

**职责**: 协调压测执行，管理并发任务，收集指标

**核心流程**:
1. 接收 `Start` 命令，初始化压测会话
2. 预热阶段：按时间线性增加并发用户
3. 稳定阶段：维持目标并发数，持续执行
4. 每秒聚合指标，发送 `Progress` 事件
5. 接收 `Stop` 命令或超时，停止压测
6. 计算分位数，生成报告，发送 `Ended` 事件

#### MetricsCollector

**职责**: 线程安全地收集和聚合压测指标

**数据结构**:
- `AtomicU64` 计数器（成功/失败次数）
- `Mutex<Vec<u64>>` 延迟记录（用于计算分位数）
- `Mutex<HashMap<String, u64>>` 错误分类统计

### 4.3 并发模型

- 使用 Tokio task 模拟虚拟用户
- 每个虚拟用户独立循环执行流程
- 通过 Channel 接收停止信号

```
┌─────────────────────────────────────────────────────────────┐
│                    StressTestExecutor                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐         │
│  │ Worker 1│  │ Worker 2│  │ Worker 3│  │ Worker N│  ...    │
│  │         │  │         │  │         │  │         │         │
│  │  ┌───┐  │  │  ┌───┐  │  │  ┌───┐  │  │  ┌───┐  │         │
│  │  │循环│  │  │  │循环│  │  │  │循环│  │  │  │循环│  │         │
│  │  │执行│  │  │  │执行│  │  │  │执行│  │  │  │执行│  │         │
│  │  │流程│  │  │  │流程│  │  │  │流程│  │  │  │流程│  │         │
│  │  └───┘  │  │  └───┘  │  │  └───┘  │  │  └───┘  │         │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘         │
│       │            │            │            │               │
│       └────────────┼────────────┼────────────┘               │
│                    ▼                                         │
│           ┌─────────────────┐                                │
│           │MetricsCollector │                                │
│           │  (Arc<Mutex>)   │                                │
│           └────────┬────────┘                                │
│                    ▼                                         │
│         StressTestEvent::Progress ──────► 前端               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 4.4 预热算法

**线性预热**: 在 `ramp_up_seconds` 时间内，均匀增加并发用户

```
并发用户数 = floor(目标并发数 × 已过时间 / 预热时长)

示例：目标 50 并发，预热 10 秒
- 第 1 秒：5 个用户
- 第 2 秒：10 个用户
- 第 3 秒：15 个用户
- ...
- 第 10 秒：50 个用户（达到目标）
```

## 五、IPC 命令

### 5.1 新增命令

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `start_stress_test` | `StressTestConfig` | `()` | 启动压测 |
| `stop_stress_test` | - | `()` | 停止压测 |
| `get_stress_test_status` | - | `StressTestStatus` | 获取当前状态 |

### 5.2 事件推送

| 事件 | 数据 | 触发时机 |
|------|------|---------|
| `stress_test:started` | `StressTestConfig` | 压测开始 |
| `stress_test:progress` | `StressTestProgress` | 每秒推送 |
| `stress_test:ended` | `StressTestReport` | 压测结束 |

## 六、前端设计

### 6.1 入口位置

- 画布顶部工具栏，新增「压测」按钮
- 与「执行」「调试」并列

### 6.2 配置对话框

点击「压测」按钮弹出配置对话框：

```
┌─────────────────────────────────────┐
│         压测配置                     │
├─────────────────────────────────────┤
│  并发用户数:  [____50____]          │
│  预热时长(秒): [____10____]         │
│  总运行时长(秒): [____60____]       │
│                                     │
│  ⚠️ 压测期间流程将被锁定，无法编辑    │
│                                     │
│        [取消]    [开始压测]          │
└─────────────────────────────────────┘
```

### 6.3 实时监控面板

压测运行时，右侧面板切换为监控视图：

```
┌─────────────────────────────────────┐
│  压测监控                    [停止] │
├─────────────────────────────────────┤
│  状态: 运行中                       │
│  已运行: 25s / 60s                  │
├─────────────────────────────────────┤
│  执行次数:     1,234                │
│  成功:         1,200  (97.2%)       │
│  失败:         34    (2.8%)         │
├─────────────────────────────────────┤
│  当前 RPS:     48.5                 │
│  平均响应时间: 42ms                  │
├─────────────────────────────────────┤
│  [RPS 实时曲线图]                    │
│  [响应时间分布图]                    │
└─────────────────────────────────────┘
```

### 6.4 结果展示

压测结束后，显示结果摘要：

```
┌─────────────────────────────────────┐
│         压测结果                     │
├─────────────────────────────────────┤
│  总执行次数:   2,934                 │
│  成功率:       98.5%                 │
│  实际 RPS:     48.9                  │
├─────────────────────────────────────┤
│  响应时间分布:                       │
│    平均:  45ms                       │
│    最小:  12ms                       │
│    最大:  234ms                      │
│    P50:   38ms                       │
│    P90:   78ms                       │
│    P95:   102ms                      │
│    P99:   156ms                      │
├─────────────────────────────────────┤
│  错误统计:                           │
│    连接超时: 23次                    │
│    HTTP 500: 12次                   │
├─────────────────────────────────────┤
│  报告已保存:                         │
│  performance_api_test_20260313.json │
│                                     │
│        [关闭]    [查看报告]          │
└─────────────────────────────────────┘
```

### 6.5 UI 限制

压测运行期间：
- 流程画布锁定，禁止编辑节点、连接
- 禁止切换到其他流程
- 工具栏只显示「停止压测」按钮

## 七、技术约束

### 7.1 会话管理

- 同时只能有一个压测会话
- 压测会话期间，正常执行和调试按钮禁用
- 压测与调试互斥

### 7.2 资源限制

- 不限制用户配置的并发数（由用户自行负责）
- 崩溃时尽力保存已收集数据

### 7.3 超时控制

- 单次流程执行超时：复用节点的 `timeout_ms` 配置
- 无流程级超时限制

### 7.4 报告存储

- 使用 tauri-plugin-store 或直接文件写入
- 报告文件不自动清理

## 八、依赖

### 8.1 新增 Rust 依赖

无需新增外部依赖，复用现有：
- `tokio` - 并发运行时
- `serde` - 序列化
- `chrono` - 时间处理

### 8.2 前端依赖

可能需要图表库（用于实时曲线）：
- `recharts` 或 `echarts-for-react`

## 九、分批实现计划

### 批次 1：后端核心（2-3 天）

**目标**：实现压测执行器核心逻辑

1. 创建 `stress_test/` 模块目录结构
2. 实现数据类型定义（Config, Progress, Report, Status）
3. 实现 `MetricsCollector` 指标收集器
4. 实现 `StressTestExecutor` 核心执行器
   - 预热逻辑
   - 并发任务管理
   - 停止信号处理
5. 添加 IPC 命令（start, stop, get_status）
6. 实现事件推送（started, progress, ended）
7. 实现报告生成和持久化

### 批次 2：前端集成（2 天）

**目标**：实现前端压测 UI

1. 创建 `useStressTestStore` 状态管理
2. 实现配置对话框组件
3. 实现实时监控面板
4. 实现结果展示组件
5. 集成 IPC 通信和事件监听
6. 实现流程锁定逻辑

### 批次 3：图表可视化（1 天）

**目标**：实现实时图表

1. 集成图表库
2. 实现 RPS 实时曲线
3. 实现响应时间分布图

### 批次 4：测试和优化（1 天）

**目标**：确保功能稳定

1. 单元测试（指标收集、分位数计算）
2. 集成测试（完整压测流程）
3. 边界测试（0 并发、立即停止等）
4. 性能优化

## 十、风险和缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 高并发导致 OOM | 压测崩溃 | 文档说明合理配置范围 |
| 目标服务被压垮 | 服务不可用 | 用户自行负责 |
| 报告文件过大 | 磁盘占用 | 仅保留错误摘要，不保存每次响应 |
| 前端卡顿 | 体验差 | 使用虚拟列表，限制图表数据点 |

## 十一、后续扩展（不在本次范围）

- 自动性能检测和推荐配置
- 变量模板和随机函数
- 历史报告对比
- HTML 格式报告
- 分布式压测
- QPS 限流控制
- 思考时间（迭代间隔）
