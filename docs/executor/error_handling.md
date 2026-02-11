# 错误处理详细设计

**文档位置**：`docs/executor/error_handling.md`
**语言**：中文
**版本**：v2.0

---

## 概述

错误处理机制处理节点执行失败情况，采用**容错模式**。

---

## 错误处理模式

### 容错模式（当前设计）

**特点：**
- 节点失败时根据错误类型决定处理策略
- **执行错误/验证错误/解析错误**：记录错误信息，继续执行后续节点
- **配置错误**：立即终止流程，无法继续执行
- 流程完成后统计成功和失败的节点数量

**适用场景：**
- 所有节点默认行为（配置错误除外）
- 批量处理场景
- 可以容忍部分节点失败的流程

---

## 错误类型定义

### ErrorKind - 错误类型枚举

```rust
/// 错误类型枚举（决定错误处理策略，同时标识具体错误类型）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    // ===== 配置错误（立即终止流程） =====
    /// 通用配置错误
    ConfigurationError,

    /// 条件表达式语法错误（如表达式语法错误、字段配置缺失）
    ConditionSyntaxError,

    // ===== 执行错误（容错模式，继续执行） =====
    /// 通用执行错误
    ExecutionError,

    /// 网络连接失败
    ConnectionError,

    /// 操作超时
    TimeoutError,

    /// 条件评估运行时错误（如字段不存在、类型不匹配）
    ConditionEvaluationError,

    // ===== 验证错误（容错模式，继续执行） =====
    /// 参数验证失败
    ValidationError,

    // ===== 解析错误（容错模式，继续执行） =====
    /// 数据解析失败（如 JSON 解析失败、数据格式转换失败）
    ParseError,
}

impl ErrorKind {
    /// 判断是否为致命错误（需要立即终止流程）
    pub fn is_fatal(&self) -> bool {
        matches!(self,
            ErrorKind::ConfigurationError |
            ErrorKind::ConditionSyntaxError
        )
    }
}
```

**错误类型与处理策略映射：**

| ErrorKind | 示例场景 | 处理策略 |
|-----------|---------|----------|
| `ConfigurationError` | 流程配置错误 | 立即终止流程 |
| `ConditionSyntaxError` | 条件表达式语法错误、字段配置缺失 | 立即终止流程 |
| `ConnectionError` | 网络超时、连接失败 | 容错模式，记录失败，继续执行 |
| `TimeoutError` | 操作超时 | 容错模式，记录失败，继续执行 |
| `ExecutionError` | HTTP 请求失败 | 容错模式，记录失败，继续执行 |
| `ConditionEvaluationError` | 条件评估运行时错误（字段不存在、类型不匹配） | 容错模式，记录失败，继续执行 |
| `ValidationError` | 参数验证失败、数据格式不符合要求 | 容错模式，记录失败，继续执行 |
| `ParseError` | JSON 解析失败、数据格式转换失败 | 容错模式，记录失败，继续执行 |

---

### ErrorDetail

错误详情结构定义见 `docs/report_structure.md`：

```rust
/// 错误详情结构（用于执行调试）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorDetail {
    /// 错误类型（决定错误处理策略，同时标识具体错误类型）
    pub error_kind: ErrorKind,

    /// 错误代码（可选，用于程序化处理）
    pub error_code: Option<String>,

    /// 错误消息（人类可读）
    pub message: String,

    /// 堆栈跟踪（可选）
    pub stack_trace: Option<String>,

    /// 错误发生时的上下文快照（可选）
    pub context: Option<serde_json::Value>,
}
```

---

## 错误处理流程

```
节点执行失败
    ↓
判断错误类型 (ErrorKind)
    ↓
├─ ConfigurationError（配置错误）
│   ↓
│   记录失败状态到节点报告
│   ↓
│   立即终止流程（无法继续执行）
│
└─ ExecutionError / ValidationError / ParseError（执行/验证/解析错误）
    ↓
    记录失败状态到节点报告
    ↓
    尝试继续执行下一个节点或其他分支（容错模式）
    ↓
    流程完成后生成完整报告（包含失败节点统计）
```

**详细流程：**

**执行成功：**
- 记录成功状态到节点报告
- 更新上下文，继续执行下一个节点

**配置错误（ConfigurationError）：**
- 立即终止流程
- 返回错误，停止执行

**执行/验证/解析错误（ExecutionError/ValidationError/ParseError）：**
- 记录失败状态到节点报告
- 当前分支终止（失败节点无输出）
- 如果有其他分支（如条件节点的另一分支）则继续执行

---

## 常见错误类型

 | ErrorKind | 处理策略 | 说明 |
|-----------|---------|------|
| `ConfigurationError` | 立即终止 | 通用配置错误 |
| `ConditionSyntaxError` | 立即终止 | 条件表达式语法错误、字段配置缺失 |
| `ConnectionError` | 容错 | 网络连接失败 |
| `TimeoutError` | 容错 | 操作超时 |
| `ValidationError` | 容错 | 参数验证失败 |
| `ParseError` | 容错 | 数据解析失败、格式转换失败 |
| `ConditionEvaluationError` | 容错 | 条件评估运行时错误（字段不存在、类型不匹配等） |

**错误类型分类原则：**
- **语法/配置错误** → `ConfigurationError` / `ConditionSyntaxError`（立即终止）
- **运行时错误** → `ConnectionError` / `TimeoutError` / `ValidationError` / `ParseError` / `ConditionEvaluationError`（容错模式）
- 使用 `error_kind.is_fatal()` 判断是否需要立即终止流程

---

## 相关文档

- **执行器架构**：`docs/executor_architecture.md`
- **报告结构**：`docs/report_structure.md`

---

**文档结束**
