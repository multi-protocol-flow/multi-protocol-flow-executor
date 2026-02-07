# 执行器设计大框文档

**文档位置**：`docs/executor_architecture.md`
**语言**：中文
**版本**：v1.0

---

## TL;DR

> **快速摘要**：本文档描述 Multi-Protocol Flow Executor 后端执行引擎的整体架构设计，基于现有实现优化改进。
>
> **核心设计决策**：
> - **条件节点**：双分支设计（true/false）
> - **失败策略**：容错模式（节点可配置失败后继续）
> - **变量管理**：节点隔离（独立输出空间）
> - **循环检测**：允许重入（限制最大执行次数）
> - **执行模式**：顺序执行（单线程简单可靠）
> - **End 节点**：执行逻辑（可自定义最终处理）
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
│  │  ├─ 初始化执行上下文                           │   │
│  │  ├─ 验证流程（查找Start节点）                   │   │
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
┌────────────────┼────────────────┬────────────────┬────────────────┐
↓                ↓                ↓                ↓                ↓
StartNodeExecutor HttpNodeExecutor ConditionalNodeExecutor EndNodeExecutor LoopNodeExecutor
(开始节点)         (HTTP请求)        (条件判断)           (结束)           (循环)
```

### 2.2 数据流向图

```
Flow Definition (输入)
    ↓
初始化 ExecutionContext
    ├─ flow_id: String
    ├─ variables: Value (初始变量）
    ├─ execution_history: Vec<ExecutionRecord>
    └─ metadata: ExecutionMetadata
    ↓
┌─────────────────────────────────┐
│  执行循环 (顺序遍历）             │
│  ┌───────────────────────────┐  │
│  │ 1. 检测循环/限制重入      │  │
│  │ 2. 执行当前节点          │  │
│  │ 3. 处理结果/错误         │  │
│  │ 4. 更新上下文          │  │
│  │ 5. 查找下一个节点        │  │
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
    /// 流程 ID
    pub flow_id: String,
    /// 当前节点 ID
    pub current_node_id: Option<String>,
    /// 执行变量（用于变量替换）
    pub variables: VariableStorage,
    /// 执行历史
    pub execution_history: Vec<ExecutionRecord>,
    /// 执行元数据
    pub metadata: ExecutionMetadata,
}

/// 变量存储（节点隔离）
pub struct VariableStorage {
    /// 全局变量（初始变量 + 显式提升的变量）
    pub global: HashMap<String, serde_json::Value>,
    /// 节点局部变量（node_uuid -> 变量映射）
    pub nodes: HashMap<String, HashMap<String, serde_json::Value>>,
}

/// 执行历史记录
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    /// 节点 UUID
    pub node_uuid: String,
    /// 执行时间
    pub timestamp: u64,
    /// 执行结果
    pub result: ExecutionResultType,
}

/// 执行结果类型
#[derive(Debug, Clone)]
pub enum ExecutionResultType {
    Success,
    Failed(String),
    Skipped(String),
}

/// 执行元数据
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    /// 流程开始时间
    pub flow_start_time: u64,
    /// 已执行节点数
    pub executed_nodes: u32,
    /// 失败节点数
    pub failed_nodes: u32,
    /// 跳过节点数
    pub skipped_nodes: u32,
}
```

**设计要点：**
- **变量隔离**：`VariableStorage` 分离全局变量和节点局部变量
- **执行历史**：记录每个节点的执行记录，便于调试
- **元数据**：跟踪执行统计信息

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
    ///   - `next_node_ids`: 下一个节点的 UUID 列表（支持多分支）
    ///   - `output_data`: 节点输出数据
    /// - `Err(error)`: 执行失败
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String>;

    /// 检查节点是否支持容错
    ///
    /// 默认返回 false（不支持容错）
    fn is_fault_tolerant(&self, _node: &Node) -> bool {
        false
    }

    /// 获取最大重试次数
    ///
    /// 默认返回 0（不重试）
    fn max_retries(&self, _node: &Node) -> u32 {
        0
    }
}
```

**设计要点：**
- **多分支支持**：返回 `Vec<String>` 支持多个下一个节点
- **容错配置**：`is_fault_tolerant()` 判断节点是否支持容错
- **重试机制**：`max_retries()` 配置最大重试次数

### 3.3 FlowExecutor（流程执行器）

```rust
pub struct FlowExecutor {
    /// 最大执行次数（防止无限重入）
    max_execution_count: HashMap<String, u32>,
    /// 每个节点的当前执行次数
    execution_count: HashMap<String, u32>,
}

impl FlowExecutor {
    /// 执行流程
    pub async fn execute_flow(
        &self,
        flow: &Flow,
        variables: Option<serde_json::Value>,
    ) -> ExecutionResult {
        // 1. 初始化执行上下文
        let mut context = ExecutionContext {
            flow_id: flow.uuid.clone(),
            current_node_id: None,
            variables: VariableStorage::from_initial_variables(variables),
            execution_history: Vec::new(),
            metadata: ExecutionMetadata::new(),
        };

        // 2. 验证流程（查找 Start 节点）
        let start_node = self.validate_and_find_start_node(&flow)?;

        // 3. 构建映射表
        let node_map = self.build_node_map(&flow);
        let connection_map = self.build_connection_map(&flow);

        // 4. 从 Start 节点开始遍历
        let current_node_ids = vec![start_node.uuid.clone()];

        // 5. 执行循环
        while !current_node_ids.is_empty() {
            // 取出第一个节点（顺序执行）
            let current_node_id = current_node_ids[0].clone();

            // 检测重入限制
            if !self.check_execution_limit(&current_node_id) {
                return ExecutionResult {
                    success: false,
                    error: Some(format!("节点 {} 超过最大执行次数", current_node_id)),
                    // ...
                };
            }

            // 查找节点
            let node = match node_map.get(&current_node_id) {
                Some(n) => n,
                None => {
                    return ExecutionResult {
                        success: false,
                        error: Some(format!("节点 {} 不存在", current_node_id)),
                        // ...
                    };
                }
            };

            // 执行节点
            match self.execute_node(node, &context, &flow.connections, &node_map).await {
                Ok((next_node_ids, output_data)) => {
                    // 执行成功

                    // 记录执行历史
                    context.execution_history.push(ExecutionRecord {
                        node_uuid: node.uuid.clone(),
                        timestamp: /* 当前时间戳 */,
                        result: ExecutionResultType::Success,
                    });

                    // 更新变量
                    context.variables.insert_node_output(&node.uuid, output_data);

                    // 更新元数据
                    context.metadata.executed_nodes += 1;

                    // 处理下一个节点
                    if next_node_ids.is_empty() {
                        // 没有下一个节点，终止执行
                        break;
                    } else {
                        // 设置当前节点 ID 为第一个下一个节点
                        context.current_node_id = Some(next_node_ids[0].clone());
                        // 更新 current_node_ids（这里简化处理，实际应该处理多个）
                    }
                }
                Err(error) => {
                    // 执行失败

                    // 检查是否支持容错
                    if node.executor().is_fault_tolerant(node) {
                        // 容错模式：标记为失败但继续执行

                        // 记录执行历史
                        context.execution_history.push(ExecutionRecord {
                            node_uuid: node.uuid.clone(),
                            timestamp: /* 当前时间戳 */,
                            result: ExecutionResultType::Failed(error.clone()),
                        });

                        // 更新元数据
                        context.metadata.failed_nodes += 1;

                        // 继续执行下一个节点
                        continue;
                    } else {
                        // 严格模式：立即终止

                        return ExecutionResult {
                            success: false,
                            error: Some(error),
                            // ...
                        };
                    }
                }
            }
        }

        // 6. 生成报告
        let flow_report = self.generate_flow_report(&context, &flow);

        // 7. 返回结果
        ExecutionResult {
            success: true,
            data: Some(json!({
                "flow_id": flow.uuid,
                "executed_nodes": context.metadata.executed_nodes,
                "failed_nodes": context.metadata.failed_nodes,
                "skipped_nodes": context.metadata.skipped_nodes,
            })),
            error: None,
            executed_nodes: context.execution_history
                .iter()
                .map(|r| r.node_uuid.clone())
                .collect(),
            flow_report: Some(flow_report),
        }
    }
}
```

---

## 4. 关键设计决策摘要

| 设计领域 | 决策 | 说明 |
|---------|------|------|
| **条件分支** | 双分支 | 条件节点有两个输出端口（true/false），根据评估结果选择分支 |
| **失败策略** | 容错模式 | 节点可配置"失败时是否继续"，支持非关键节点的容错 |
| **变量管理** | 节点隔离 | 每个节点有独立的输出空间，通过 `nodes.{uuid}.{key}` 访问 |
| **循环检测** | 允许重入 | 限制每个节点的最大执行次数，允许 Loop 节点重复执行 |
| **执行模式** | 顺序执行 | 单线程顺序执行，简单可靠 |
| **End 节点** | 执行逻辑 | 可以执行自定义逻辑，返回最终结果 |
| **执行上下文** | 完整上下文 | 包含执行历史和元数据，便于调试和监控 |

---

## 5. 扩展性设计

### 5.1 添加新节点类型

**步骤：**
1. 在 `NodeType` 枚举中添加新类型
2. 在 `NodeData` 枚举中添加配置结构
3. 创建新的执行器实现 `NodeExecutor` trait
4. 在 `FlowExecutor::execute_node()` 中注册新执行器

**示例：**
```rust
// 1. 添加节点类型
pub enum NodeType {
    Start,
    Http,
    Conditional,
    End,
    Loop,  // 新增
}

// 2. 添加节点配置
pub enum NodeData {
    Start(StartConfig),
    Http(HttpConfig),
    Conditional(ConditionalConfig),
    End(EndConfig),
    Loop(LoopConfig),  // 新增
}

// 3. 实现执行器
pub struct LoopNodeExecutor;

#[async_trait::async_trait]
impl NodeExecutor for LoopNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 实现 Loop 节点的执行逻辑
    }
}

// 4. 注册执行器
NodeType::Loop => {
    let executor = crate::loop_executor::LoopNodeExecutor::new();
    executor.execute(node, context, connections, node_map).await
}
```

### 5.2 添加新功能

**并行执行（未来）：**
```rust
// 修改 NodeExecutor 返回值，支持并行执行
async fn execute(
    &self,
    node: &Node,
    context: &ExecutionContext,
    connections: &[Connection],
    node_map: &HashMap<String, &Node>,
) -> Result<ExecutionOutput, String>;

pub enum ExecutionOutput {
    /// 顺序执行（下一个节点串行执行）
    Sequential(Vec<String>),
    /// 并行执行（多个节点并行执行）
    Parallel(Vec<String>),
    /// 无下一个节点
    None,
}
```

**重试机制（未来）：**
```rust
// 在 FlowExecutor 中实现重试逻辑
async fn execute_node_with_retry(
    &self,
    node: &Node,
    context: &ExecutionContext,
    connections: &[Connection],
    node_map: &HashMap<String, &Node>,
) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
    let max_retries = node.executor().max_retries(node);
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match node.executor().execute(node, context, connections, node_map).await {
            Ok(result) => return Ok(result),
            Err(error) => {
                last_error = Some(error);
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_millis(1000 * (attempt as u64 + 1))).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "重试次数耗尽".to_string()))
}
```

---

## 6. 相关文档

- **节点结构设计**：`docs/node_structure.md`
- **报告结构设计**：`docs/report_structure.md`
- **执行流程详细设计**：`docs/executor/execution_flow.md`（待创建）
- **条件节点详细设计**：`docs/executor/conditional_node.md`（待创建）
- **错误处理详细设计**：`docs/executor/error_handling.md`（待创建）
- **变量管理详细设计**：`docs/executor/variable_management.md`（待创建）
- **循环检测详细设计**：`docs/executor/loop_detection.md`（待创建）
- **End 节点详细设计**：`docs/executor/end_node.md`（待创建）
- **报告生成详细设计**：`docs/executor/reporting.md`（待创建）
- **上下文管理详细设计**：`docs/executor/context_management.md`（待创建）

---

## 7. 附录

### 7.1 术语表

| 术语 | 说明 |
|-----|------|
| Flow | 流程定义，包含节点和连接 |
| Node | 节点，流程的基本执行单元 |
| Connection | 连接，定义节点之间的依赖关系 |
| ExecutionContext | 执行上下文，包含流程执行期间的所有状态 |
| NodeExecutor | 节点执行器接口，定义节点的执行行为 |
| VariableStorage | 变量存储，管理全局变量和节点局部变量 |
| ExecutionRecord | 执行记录，记录节点的执行历史 |
| ExecutionMetadata | 执行元数据，包含执行统计信息 |

### 7.2 设计演进历史

| 版本 | 日期 | 主要变更 |
|-----|------|---------|
| v1.0 | 2025-02-07 | 初始版本，定义整体架构和关键设计决策 |

---

**文档结束**
