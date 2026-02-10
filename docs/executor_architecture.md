# 执行器设计大框文档

**文档位置**：`docs/executor_architecture.md`
**语言**：中文
**版本**：v1.3

---

## TL;DR

> **快速摘要**：本文档描述 Multi-Protocol Flow Executor 后端执行引擎的整体架构设计，基于现有实现优化改进。
>
  > **核心设计决策**：
  > - **条件节点**：双分支设计（true/false），根据条件评估结果选择分支；匹配到的分支有连接则执行，无连接则流程终止（与普通节点无下游节点逻辑一致）
  > - **变量管理**：节点隔离（独立输出空间）
  > - **执行模式**：顺序执行（单线程简单可靠）
  > - **流程终止**：没有下一个节点时生成报告
  > - **执行上下文**：完整上下文（历史+元数据）
>
> **目标读者**：后端开发人员、系统架构师
> **相关文档**：`docs/node_structure.md`、`docs/report_structure.md`

---

## 1. 概述

### 1.1 设计目标

Multi-Protocol Flow Executor 的执行引擎负责：
1. 按照流程定义（Flow）按顺序执行节点
2. 支持条件分支、容错、变量管理
3. 生成完整的执行报告
4. 提供灵活的扩展性

### 1.2 设计原则

- **简单优先**：优先选择简单可靠的实现，避免过度设计
- **渐进增强**：从 MVP 开始，逐步添加高级功能
- **可扩展性**：架构支持未来添加新节点类型和新功能
- **可调试性**：完整的日志和报告，便于问题定位
- **类型安全**：利用 Rust 的类型系统防止运行时错误

---

## 2. 整体架构

### 2.1 组件架构图

```
┌─────────────────────────────────────────────────────────┐
│                    FlowExecutor (主执行引擎)              │
│  ┌─────────────────────────────────────────────────┐   │
│  │  execute_flow(flow, variables)                 │   │
│  │  ├─ 初始化执行上下文（flow_uuid, node_uuid）    │   │
│  │  ├─ 验证流程（查找Entry节点）                   │   │
│  │  ├─ 构建映射表（node_map, connection_map）      │   │
│  │  └─ 执行循环 (while let Some(node))           │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                           ↓
          ┌────────────────┼────────────────┐
          ↓                ↓                ↓
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ ExecutionContext │  │ NodeExecutor │  │   Reporter    │
│   (执行上下文)  │  │  (trait接口)  │  │  (报告生成)  │
└──────────────┘  └──────────────┘  └──────────────┘
           ↓
 ┌────────────────┼────────────────┐
  ↓                ↓                ↓
  EntryNodeExecutor HttpNodeExecutor ConditionalNodeExecutor
  (入口节点)         (HTTP请求)        (条件判断)

   **节点端口**：详见 `docs/node_structure.md` 端口类型定义
```

### 2.2 数据流向图

```
Flow Definition (输入)
    ↓
  初始化 ExecutionContext
      ├─ flow_uuid: String (流程 UUID)
      ├─ current_node_uuid: Option<String>
      ├─ variables: VariableStorage (初始变量 + 节点隔离）
      ├─ node_reports: Vec<NodeExecutionReport>
      └─ flow_trigger_time: u64
     ↓
  ┌─────────────────────────────────┐
  │  执行循环 (顺序遍历）             │
  │  ┌───────────────────────────┐  │
  │  │ 1. 执行当前节点          │  │
  │  │ 2. 处理结果/错误         │  │
  │  │ 3. 更新上下文          │  │
  │  │ 4. 查找下一个节点        │  │
  │  │ 5. 如果没有下一个节点，生成报告并终止 │  │
  │  └───────────────────────────┘  │
  └─────────────────────────────────┘
     ↓
 生成执行报告
     ├─ NodeExecutionReport (每个节点）
     └─ FlowExecutionReport (整体）
     ↓
 ExecutionResult (输出)
```

---

## 3. 核心组件设计

### 3.1 ExecutionContext（执行上下文）

```rust
/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// 流程 UUID（前端生成）
    pub flow_uuid: String,
    /// 当前执行节点的 UUID
    pub current_node_uuid: Option<String>,
    /// 执行变量（用于变量替换）
    pub variables: VariableStorage,
    /// 执行历史记录（每个节点的详细执行报告）
    pub node_reports: Vec<NodeExecutionReport>,
    /// 流程触发时间（用于记录触发和开始执行的时间差）
    pub flow_trigger_time: u64,
}

/// 变量存储（节点隔离）
pub struct VariableStorage {
    /// 全局变量（初始变量 + 显式提升的变量）
    pub global: HashMap<String, serde_json::Value>,
    /// 节点局部变量（node_uuid -> 变量映射）
    pub nodes: HashMap<String, HashMap<String, serde_json::Value>>,
}

/// 节点执行报告（详见 `docs/report_structure.md`）
/// 定义位于 `docs/report_structure.md`，此处仅引用以说明上下文关系
pub type NodeExecutionReport = crate::report_structure::NodeExecutionReport;

**职责**：在流程执行期间维护所有必要的状态信息。

**设计要点**：
- **变量隔离**：`VariableStorage` 分离全局变量和节点局部变量
- **完整报告结构**：使用 `NodeExecutionReport` 统一记录所有节点执行信息（详见 `docs/report_structure.md`），无需区分内部记录和外部报告
- **统一状态**：使用 `ExecutionStatus` 作为统一的状态枚举（详见 `docs/report_structure.md`）
- **时间区分**：`flow_trigger_time` 是流程触发时间（外部请求到达时间），`FlowExecutionReport.start_time` 是真正开始执行的时间（完成初始化后），两者可能有一定误差

### 3.2 NodeExecutor Trait（节点执行器接口）

```rust
#[async_trait::async_trait]
pub trait NodeExecutor {
    /// 执行节点
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文（包含当前节点 ID）
    /// - `connections`: 流程的所有连接
    /// - `node_map`: 节点 ID 到节点的映射
    ///
    /// # 返回
    /// - `Ok((next_node_ids, output_data))`:
    ///   - `next_node_ids`: 下一个节点的 UUID 列表
    ///     - 返回 0 或 1 个节点 UUID（空列表表示流程终止）
    ///   - `output_data`: 节点输出数据
    ///     - Entry 节点：必须为 `None`（不产生数据，仅作为流程入口）
    ///     - Http 节点：HTTP 响应数据
    ///     - Conditional 节点：条件评估结果的布尔值
    /// - `Err(error)`: 执行失败
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String>;
}
```

**职责**：定义节点的执行行为接口，所有节点类型必须实现此 trait。

**设计要点**：
- **统一返回类型**：所有节点类型使用统一的 `Vec<String>` 返回类型，避免为条件节点单独使用特殊类型（如 `Option<String>`），保持接口一致性
- **返回值语义**：
  - 空列表 `[]`：没有下一个节点，流程终止
  - 单元素列表 `[node_uuid]`：有下一个节点，继续执行
  - `Vec<String>` 类型为未来可能的并行执行预留扩展空间
- **条件节点行为**：根据条件评估结果选择分支：
  - 条件为 true 时返回 true 端口连接的节点（true 端口无连接时返回空列表，流程终止）
  - 条件为 false 时返回 false 端口连接的节点（false 端口无连接时返回空列表，流程终止）
  - 与普通节点保持一致：匹配到的分支（true/false）无下游连接时流程终止

### 3.3 FlowExecutor（流程执行器）

```rust
pub struct FlowExecutor;
```

**职责**：管理整个流程的执行生命周期，从初始化到报告生成。

**执行流程**：

**1. 初始化阶段**
- 记录流程触发时间
- 创建执行上下文（包括变量存储、执行报告等）
- 验证流程完整性（查找 Entry 节点）
- 构建节点映射表和连接映射表

**2. 执行循环**
- 从 Entry 节点开始，顺序执行节点
- 节点执行成功：记录执行报告、更新变量、继续下一个节点
- 节点执行失败：记录失败状态，立即终止流程

**3. 报告生成**
- 当没有下一个节点时，根据节点报告生成完整的流程报告
- 计算统计信息（成功数、失败数等）
- 返回执行结果和报告

**错误处理策略**：
- 节点不存在：立即终止
- 节点执行失败：立即终止

---

## 4. 关键设计决策摘要

| 设计领域 | 决策 | 说明 |
|---------|------|------|
| **条件分支** | 双分支 | 条件节点有两个输出端口（true/false），根据条件评估结果选择分支；匹配到的分支有连接则执行，无连接则流程终止（与普通节点无下游节点逻辑一致） |
| **失败策略** | 严格模式 | 节点失败时立即终止流程 |
| **变量管理** | 节点隔离 | 每个节点有独立的输出空间，通过 `nodes.{uuid}.{key}` 访问 |
| **执行模式** | 顺序执行 | 单线程顺序执行，简单可靠 |
| **流程终止** | 无下游节点 | 没有下一个节点时生成报告并终止 |
| **ID 命名** | UUID 规范 | 流程和节点使用 UUID v4 作为唯一标识，由前端生成 |
| **执行上下文** | 完整上下文 | 包含执行历史和元数据，便于调试和监控 |

---

## 5. 扩展性设计

### 5.1 添加新节点类型

**步骤：**
1. 在 `NodeType` 枚举中添加新类型
2. 在 `NodeData` 枚举中添加配置结构
3. 创建新的执行器实现 `NodeExecutor` trait
4. 在 `FlowExecutor::execute_node()` 中注册新执行器

详见 `docs/node_structure.md` 扩展性设计。

### 5.2 添加新功能

**并行执行（未来）**：修改 `NodeExecutor` 返回值支持并行执行模式。

---

## 6. 相关文档

- **节点结构设计**：`docs/node_structure.md`
- **报告结构设计**：`docs/report_structure.md`
- **执行流程详细设计**：`docs/executor/execution_flow.md`（待创建）
- **条件节点详细设计**：`docs/executor/conditional_node.md`（待创建）
- **错误处理详细设计**：`docs/executor/error_handling.md`（待创建）
- **变量管理详细设计**：`docs/executor/variable_management.md`（待创建）
- **循环检测详细设计**：`docs/executor/loop_detection.md`（待创建）
- **报告生成详细设计**：`docs/executor/reporting.md`（待创建）
- **上下文管理详细设计**：`docs/executor/context_management.md`（待创建）

---

## 7. 附录

### 7.1 术语表

| 术语 | 说明 |
|-----|------|
| Flow | 流程定义，包含节点和连接 |
| Node | 节点，流程的基本执行单元，使用 UUID v4 作为唯一标识 |
| Connection | 连接，定义节点之间的依赖关系 |
| ExecutionContext | 执行上下文，包含流程执行期间的所有状态 |
| NodeExecutor | 节点执行器接口，定义节点的执行行为 |
| VariableStorage | 变量存储，管理全局变量和节点局部变量 |

**数据结构定义详见**：
- `docs/node_structure.md`：Flow、Node、Connection、Port
- `docs/report_structure.md`：ExecutionStatus、NodeExecutionReport、FlowExecutionReport、ExecutionResult

### 7.2 版本历史

**当前版本**：v1.9 (2026-02-10)

**主要变更**：
- v1.8-v1.9：条件节点双分支设计，统一报告结构
- v1.4-v1.7：重构节点类型（Start→Entry），移除End节点
- v1.1-v1.3：统一 UUID 命名，简化文档结构
- v1.0：初始版本

---

**文档结束**
