# 断言节点功能需求

**创建时间**: 2026-03-11
**状态**: 待实现
**相关文档**: 无

## 一、功能概述

为流程执行器新增独立断言节点，用于验证 HTTP/WebSocket 响应是否符合预期。支持多种数据断言类型、多断言逻辑组合、结构化断言结果输出，并可配置失败时的处理行为。

### 1.1 背景与目标

当前流程执行中，用户只能通过条件节点进行简单的路由判断，存在以下问题：
- 无法同时验证多个条件
- 无法获取结构化的断言结果
- 无法清晰区分"验证失败"和"路由分支"

本功能旨在提供专业的断言能力，支持：
- 多种数据断言类型（状态码、响应时间、JSON Path、Header、Body 等）
- 多断言组合（AND/OR 模式）
- 两种失败模式（路由到 false 分支 / 抛出异常终止）
- 结构化断言结果输出
- 断言预设模板

### 1.2 执行模型

```
[上游节点] → [断言节点] → (true端口) → [下游节点]
                  ↓
            (false端口) → [错误处理分支] / [终止流程]
```

- 独立节点类型，单一职责
- 断言失败可配置：路由到 false 分支 或 抛出异常终止

---

## 二、核心功能

### 2.1 断言类型

| 断言类型 | 目标字段 | 说明 |
|---------|---------|------|
| 状态码 | `status` | HTTP 响应状态 |
| 响应时间 | `response_time` | 毫秒级性能断言 |
| JSON Path | `body.data.xxx` | JSON 字段存在性/值 |
| Header | `headers["X-Custom"]` | 响应头断言 |
| Body 文本 | `body` | 文本匹配 |
| 数组长度 | `body.items.length` | 数组检查 |

### 2.2 操作符

| 操作符 | 说明 | 适用类型 |
|--------|------|---------|
| `eq` | 等于 | 所有类型 |
| `ne` | 不等于 | 所有类型 |
| `gt` | 大于 | 数字 |
| `gte` | 大于等于 | 数字 |
| `lt` | 小于 | 数字 |
| `lte` | 小于等于 | 数字 |
| `contains` | 包含 | 字符串/数组 |
| `not_contains` | 不包含 | 字符串/数组 |
| `matches` | 正则匹配 | 字符串 |
| `exists` | 字段存在 | 任意 |
| `not_exists` | 字段不存在 | 任意 |
| `in` | 值在列表中 | 所有类型 |
| `not_in` | 值不在列表中 | 所有类型 |
| `type_is` | 类型检查 | 任意 |
| `between` | 范围检查 | 数字 |

### 2.3 断言组合模式

| 模式 | 说明 |
|------|------|
| `all` | 全部通过才算通过 (AND) |
| `any` | 任一通过即算通过 (OR) |

### 2.4 失败行为配置

| 配置项 | 选项 | 说明 |
|--------|------|------|
| `on_failure` | `route_false` | 路由到 false 分支，允许后续处理 |
| `on_failure` | `throw` | 抛出异常终止流程 |
| `continue_on_failure` | `true/false` | 失败后是否继续执行后续断言（用于收集所有失败） |

### 2.5 断言预设

内置预设模板，减少重复配置：

| 预设名称 | 说明 |
|---------|------|
| `http_success` | HTTP 状态码 2xx |
| `http_client_error` | HTTP 状态码 4xx |
| `http_server_error` | HTTP 状态码 5xx |
| `json_response` | Content-Type 包含 application/json |
| `response_fast` | 响应时间 < 500ms |

---

## 三、数据结构变更

### 3.1 TypeScript 类型定义

```typescript
// src/types/node.ts

/**
 * 断言操作符
 */
export type AssertionOperator =
  | 'eq' | 'ne'
  | 'gt' | 'gte' | 'lt' | 'lte'
  | 'contains' | 'not_contains'
  | 'matches'
  | 'exists' | 'not_exists'
  | 'in' | 'not_in'
  | 'type_is'
  | 'between';

/**
 * 单条断言配置
 */
export interface Assertion {
  id: string;                    // 断言唯一标识
  description?: string;          // 断言描述
  target: string;                // 断言目标 (JSONPath)
  operator: AssertionOperator;   // 操作符
  expected: unknown;             // 期望值
  enabled?: boolean;             // 是否启用，默认 true
}

/**
 * 断言组合模式
 */
export type AssertionMode = 'all' | 'any';

/**
 * 失败行为
 */
export type OnFailureMode = 'route_false' | 'throw';

/**
 * 断言节点配置
 */
export interface AssertionNodeConfig {
  /** 断言列表 */
  assertions: Assertion[];
  /** 组合模式：all=全部通过, any=任一通过 */
  mode: AssertionMode;
  /** 失败行为：route_false=路由到false分支, throw=抛出异常 */
  on_failure: OnFailureMode;
  /** 失败后是否继续执行后续断言（用于收集所有失败） */
  continue_on_failure: boolean;
  /** 预设模板名称（可选） */
  preset?: string;
}

/**
 * 单条断言执行结果
 */
export interface AssertionResult {
  id: string;              // 断言ID
  description?: string;    // 断言描述
  passed: boolean;         // 是否通过
  target: string;          // 断言目标
  operator: string;        // 操作符
  expected: unknown;       // 期望值
  actual: unknown;         // 实际值
  error?: string;          // 错误信息（如解析失败）
}

/**
 * 断言节点输出
 */
export interface AssertionOutput {
  /** 整体是否通过 */
  passed: boolean;
  /** 断言总数 */
  total: number;
  /** 通过数量 */
  passed_count: number;
  /** 失败数量 */
  failed_count: number;
  /** 每条断言的详情 */
  results: AssertionResult[];
  /** 失败的断言列表 */
  failed_assertions: AssertionResult[];
  /** 透传上游输入 */
  input: Record<string, unknown>;
}

/**
 * 断言节点类型
 */
export const NODE_TYPE_ASSERTION = 'assertion';
```

### 3.2 Rust 类型定义

```rust
// src-tauri/crates/flow-engine/src/node_config.rs

/// 断言操作符
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    NotContains,
    Matches,
    Exists,
    NotExists,
    In,
    NotIn,
    TypeIs,
    Between,
}

/// 单条断言配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub target: String,
    pub operator: AssertionOperator,
    pub expected: Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

/// 断言组合模式
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssertionMode {
    #[default]
    All,
    Any,
}

/// 失败行为
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnFailureMode {
    #[default]
    RouteFalse,
    Throw,
}

/// 断言节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionNodeConfig {
    pub assertions: Vec<Assertion>,
    #[serde(default)]
    pub mode: AssertionMode,
    #[serde(default)]
    pub on_failure: OnFailureMode,
    #[serde(default)]
    pub continue_on_failure: bool,
    #[serde(default)]
    pub preset: Option<String>,
}

/// 单条断言执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub passed: bool,
    pub target: String,
    pub operator: String,
    pub expected: Value,
    pub actual: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 断言节点输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionOutput {
    pub passed: bool,
    pub total: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub results: Vec<AssertionResult>,
    pub failed_assertions: Vec<AssertionResult>,
    pub input: Value,
}
```

### 3.3 错误类型扩展

```rust
// src-tauri/crates/flow-engine/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    // ... 现有错误 ...
    
    #[error("断言失败: {0}")]
    AssertionFailed(String, AssertionOutput),
}
```

---

## 四、后端实现

### 4.1 文件结构

```
src-tauri/crates/flow-engine/src/
├── assertion_executor.rs      # 断言执行器（新增）
├── assertion/
│   ├── mod.rs                 # 模块导出
│   ├── operator.rs            # 操作符实现
│   ├── evaluator.rs           # 断言求值器
│   └── preset.rs              # 预设模板
└── node_type.rs               # 添加 Assertion 类型
```

### 4.2 断言执行器核心逻辑

```rust
// src-tauri/crates/flow-engine/src/assertion_executor.rs

use crate::{Node, NodeExecutor, ExecutionContext, ExecutionResult, ExecutionError};
use crate::assertion::{AssertionEvaluator, AssertionPreset};
use async_trait::async_trait;

pub struct AssertionExecutor;

#[async_trait]
impl NodeExecutor for AssertionExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
    ) -> ExecutionResult {
        let config: AssertionNodeConfig = node.parse_config()?;
        
        // 获取上游输入
        let input = context.get_input().clone();
        
        // 应用预设（如果有）
        let mut assertions = config.assertions.clone();
        if let Some(preset_name) = &config.preset {
            let preset_assertions = AssertionPreset::get(preset_name)?;
            assertions = [preset_assertions, assertions].concat();
        }
        
        // 过滤启用的断言
        let enabled_assertions: Vec<_> = assertions
            .into_iter()
            .filter(|a| a.enabled)
            .collect();
        
        // 执行断言
        let evaluator = AssertionEvaluator::new(&input);
        let mut results = Vec::new();
        
        for assertion in &enabled_assertions {
            let result = evaluator.evaluate(assertion)?;
            results.push(result.clone());
            
            // 检查是否需要继续
            if !result.passed && !config.continue_on_failure {
                break;
            }
        }
        
        // 计算统计
        let passed_count = results.iter().filter(|r| r.passed).count();
        let failed_count = results.len() - passed_count;
        let total = results.len();
        
        // 判断整体是否通过
        let passed = match config.mode {
            AssertionMode::All => failed_count == 0,
            AssertionMode::Any => passed_count > 0,
        };
        
        // 收集失败的断言
        let failed_assertions: Vec<_> = results
            .iter()
            .filter(|r| !r.passed)
            .cloned()
            .collect();
        
        // 构建输出
        let output = AssertionOutput {
            passed,
            total,
            passed_count,
            failed_count,
            results,
            failed_assertions,
            input,
        };
        
        // 处理失败
        if !passed {
            match config.on_failure {
                OnFailureMode::RouteFalse => {
                    return Ok(ExecutionResult::with_output_and_route(
                        serde_json::to_value(output)?,
                        "false".to_string(),
                    ));
                }
                OnFailureMode::Throw => {
                    let error_msg = format!(
                        "断言失败: {}/{} 未通过",
                        failed_count, total
                    );
                    return Err(ExecutionError::AssertionFailed(error_msg, output));
                }
            }
        }
        
        Ok(ExecutionResult::with_output(
            serde_json::to_value(output)?
        ))
    }
}
```

### 4.3 断言求值器

```rust
// src-tauri/crates/flow-engine/src/assertion/evaluator.rs

use super::operator::evaluate_operator;
use serde_json::Value;

pub struct AssertionEvaluator<'a> {
    input: &'a Value,
}

impl<'a> AssertionEvaluator<'a> {
    pub fn new(input: &'a Value) -> Self {
        Self { input }
    }
    
    /// 从输入中提取目标值
    fn extract_target(&self, path: &str) -> Result<Value, String> {
        // 支持: status, response_time, body, body.data.id, headers["X-Custom"]
        if path == "status" {
            return Ok(self.input.get("status").cloned().unwrap_or(Value::Null));
        }
        if path == "response_time" {
            return Ok(self.input.get("response_time").cloned().unwrap_or(Value::Null));
        }
        
        // JSONPath 解析 (简化版，支持点号路径和数组索引)
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self.input;
        
        for part in parts {
            // 处理数组索引: items[0]
            if let Some(idx_start) = part.find('[') {
                let field = &part[..idx_start];
                let idx_str = &part[idx_start+1..part.len()-1];
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if let Some(arr) = current.get(field).and_then(|v| v.as_array()) {
                        current = arr.get(idx).unwrap_or(&Value::Null);
                    } else {
                        return Ok(Value::Null);
                    }
                }
            } else if let Some(val) = current.get(part) {
                current = val;
            } else {
                return Ok(Value::Null);
            }
        }
        
        Ok(current.clone())
    }
    
    /// 执行单条断言
    pub fn evaluate(&self, assertion: &Assertion) -> Result<AssertionResult, String> {
        let actual = self.extract_target(&assertion.target)?;
        
        let passed = evaluate_operator(
            &assertion.operator,
            &actual,
            &assertion.expected,
        )?;
        
        Ok(AssertionResult {
            id: assertion.id.clone(),
            description: assertion.description.clone(),
            passed,
            target: assertion.target.clone(),
            operator: format!("{:?}", assertion.operator).to_lowercase(),
            expected: assertion.expected.clone(),
            actual,
            error: None,
        })
    }
}
```

### 4.4 操作符实现

```rust
// src-tauri/crates/flow-engine/src/assertion/operator.rs

use super::AssertionOperator;
use serde_json::Value;
use regex::Regex;

pub fn evaluate_operator(
    operator: &AssertionOperator,
    actual: &Value,
    expected: &Value,
) -> Result<bool, String> {
    match operator {
        AssertionOperator::Eq => Ok(actual == expected),
        AssertionOperator::Ne => Ok(actual != expected),
        
        AssertionOperator::Gt => {
            let a = actual.as_f64().ok_or("actual 不是数字")?;
            let e = expected.as_f64().ok_or("expected 不是数字")?;
            Ok(a > e)
        }
        
        AssertionOperator::Gte => {
            let a = actual.as_f64().ok_or("actual 不是数字")?;
            let e = expected.as_f64().ok_or("expected 不是数字")?;
            Ok(a >= e)
        }
        
        AssertionOperator::Lt => {
            let a = actual.as_f64().ok_or("actual 不是数字")?;
            let e = expected.as_f64().ok_or("expected 不是数字")?;
            Ok(a < e)
        }
        
        AssertionOperator::Lte => {
            let a = actual.as_f64().ok_or("actual 不是数字")?;
            let e = expected.as_f64().ok_or("expected 不是数字")?;
            Ok(a <= e)
        }
        
        AssertionOperator::Contains => {
            match (actual, expected) {
                (Value::String(a), Value::String(e)) => Ok(a.contains(e)),
                (Value::Array(a), e) => Ok(a.contains(e)),
                _ => Err("contains 仅支持字符串和数组".to_string()),
            }
        }
        
        AssertionOperator::NotContains => {
            match (actual, expected) {
                (Value::String(a), Value::String(e)) => Ok(!a.contains(e)),
                (Value::Array(a), e) => Ok(!a.contains(e)),
                _ => Err("not_contains 仅支持字符串和数组".to_string()),
            }
        }
        
        AssertionOperator::Matches => {
            let a = actual.as_str().ok_or("actual 不是字符串")?;
            let pattern = expected.as_str().ok_or("expected 不是字符串")?;
            let re = Regex::new(pattern).map_err(|e| format!("正则表达式无效: {}", e))?;
            Ok(re.is_match(a))
        }
        
        AssertionOperator::Exists => Ok(!actual.is_null()),
        AssertionOperator::NotExists => Ok(actual.is_null()),
        
        AssertionOperator::In => {
            let arr = expected.as_array().ok_or("expected 不是数组")?;
            Ok(arr.contains(actual))
        }
        
        AssertionOperator::NotIn => {
            let arr = expected.as_array().ok_or("expected 不是数组")?;
            Ok(!arr.contains(actual))
        }
        
        AssertionOperator::TypeIs => {
            let type_name = expected.as_str().ok_or("expected 不是字符串")?;
            let actual_type = get_json_type(actual);
            Ok(actual_type == type_name)
        }
        
        AssertionOperator::Between => {
            let arr = expected.as_array().ok_or("expected 不是数组")?;
            let a = actual.as_f64().ok_or("actual 不是数字")?;
            let min = arr.get(0).and_then(|v| v.as_f64()).ok_or("范围最小值无效")?;
            let max = arr.get(1).and_then(|v| v.as_f64()).ok_or("范围最大值无效")?;
            Ok(a >= min && a <= max)
        }
    }
}

fn get_json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
```

### 4.5 断言预设

```rust
// src-tauri/crates/flow-engine/src/assertion/preset.rs

use super::Assertion;
use crate::node_config::AssertionOperator;

pub struct AssertionPreset;

impl AssertionPreset {
    pub fn get(name: &str) -> Result<Vec<Assertion>, String> {
        match name {
            "http_success" => Ok(vec![
                Assertion {
                    id: "preset_http_status".to_string(),
                    description: Some("HTTP状态码应为2xx".to_string()),
                    target: "status".to_string(),
                    operator: AssertionOperator::Between,
                    expected: serde_json::json!([200, 299]),
                    enabled: true,
                },
            ]),
            
            "http_client_error" => Ok(vec![
                Assertion {
                    id: "preset_client_error".to_string(),
                    description: Some("HTTP状态码应为4xx".to_string()),
                    target: "status".to_string(),
                    operator: AssertionOperator::Between,
                    expected: serde_json::json!([400, 499]),
                    enabled: true,
                },
            ]),
            
            "http_server_error" => Ok(vec![
                Assertion {
                    id: "preset_server_error".to_string(),
                    description: Some("HTTP状态码应为5xx".to_string()),
                    target: "status".to_string(),
                    operator: AssertionOperator::Between,
                    expected: serde_json::json!([500, 599]),
                    enabled: true,
                },
            ]),
            
            "json_response" => Ok(vec![
                Assertion {
                    id: "preset_json_content".to_string(),
                    description: Some("Content-Type应为application/json".to_string()),
                    target: r#"headers["Content-Type"]"#.to_string(),
                    operator: AssertionOperator::Contains,
                    expected: serde_json::json!("application/json"),
                    enabled: true,
                },
            ]),
            
            "response_fast" => Ok(vec![
                Assertion {
                    id: "preset_fast_response".to_string(),
                    description: Some("响应时间应小于500ms".to_string()),
                    target: "response_time".to_string(),
                    operator: AssertionOperator::Lt,
                    expected: serde_json::json!(500),
                    enabled: true,
                },
            ]),
            
            _ => Err(format!("未知的断言预设: {}", name)),
        }
    }
    
    pub fn list() -> Vec<PresetInfo> {
        vec![
            PresetInfo { name: "http_success", description: "HTTP状态码2xx" },
            PresetInfo { name: "http_client_error", description: "HTTP状态码4xx" },
            PresetInfo { name: "http_server_error", description: "HTTP状态码5xx" },
            PresetInfo { name: "json_response", description: "Content-Type为application/json" },
            PresetInfo { name: "response_fast", description: "响应时间<500ms" },
        ]
    }
}

pub struct PresetInfo {
    pub name: &'static str,
    pub description: &'static str,
}
```

---

## 五、前端实现

### 5.1 文件结构

```
src/
├── components/
│   ├── canvas/nodes/
│   │   └── AssertionNode.tsx          # 画布节点组件
│   └── panels/
│       └── AssertionConfigPanel.tsx   # 配置面板
└── types/
    └── assertion.ts                   # 断言相关类型（或合并到 node.ts）
```

### 5.2 配置面板组件

**UI 设计**：表单模式，每个断言一行，下拉选择操作符。

```tsx
// src/components/panels/AssertionConfigPanel.tsx

import { useState } from 'react';
import { useNodeStore } from '@/stores/node';
import { 
  Assertion, 
  AssertionOperator, 
  AssertionMode, 
  OnFailureMode,
  AssertionNodeConfig 
} from '@/types/node';
import { Plus, Trash2, GripVertical } from 'lucide-react';

const OPERATOR_OPTIONS: { value: AssertionOperator; label: string }[] = [
  { value: 'eq', label: '等于' },
  { value: 'ne', label: '不等于' },
  { value: 'gt', label: '大于' },
  { value: 'gte', label: '大于等于' },
  { value: 'lt', label: '小于' },
  { value: 'lte', label: '小于等于' },
  { value: 'contains', label: '包含' },
  { value: 'not_contains', label: '不包含' },
  { value: 'matches', label: '正则匹配' },
  { value: 'exists', label: '存在' },
  { value: 'not_exists', label: '不存在' },
  { value: 'in', label: '在列表中' },
  { value: 'not_in', label: '不在列表中' },
  { value: 'type_is', label: '类型为' },
  { value: 'between', label: '在范围内' },
];

const PRESET_OPTIONS = [
  { value: '', label: '无预设' },
  { value: 'http_success', label: 'HTTP 成功响应 (2xx)' },
  { value: 'http_client_error', label: 'HTTP 客户端错误 (4xx)' },
  { value: 'http_server_error', label: 'HTTP 服务端错误 (5xx)' },
  { value: 'json_response', label: 'JSON 响应' },
  { value: 'response_fast', label: '快速响应 (<500ms)' },
];

interface Props {
  nodeId: string;
}

export function AssertionConfigPanel({ nodeId }: Props) {
  const { nodes, updateNode } = useNodeStore();
  const node = nodes.find(n => n.id === nodeId);
  const config = node?.config as AssertionNodeConfig | undefined;

  const [assertions, setAssertions] = useState<Assertion[]>(
    config?.assertions ?? []
  );
  const [mode, setMode] = useState<AssertionMode>(config?.mode ?? 'all');
  const [onFailure, setOnFailure] = useState<OnFailureMode>(
    config?.on_failure ?? 'route_false'
  );
  const [continueOnFailure, setContinueOnFailure] = useState(
    config?.continue_on_failure ?? true
  );
  const [preset, setPreset] = useState(config?.preset ?? '');

  const addAssertion = () => {
    const newAssertion: Assertion = {
      id: `assert_${Date.now()}`,
      description: '',
      target: 'status',
      operator: 'eq',
      expected: 200,
      enabled: true,
    };
    setAssertions([...assertions, newAssertion]);
  };

  const updateAssertion = (index: number, updates: Partial<Assertion>) => {
    const updated = [...assertions];
    updated[index] = { ...updated[index], ...updates };
    setAssertions(updated);
  };

  const removeAssertion = (index: number) => {
    setAssertions(assertions.filter((_, i) => i !== index));
  };

  const saveConfig = () => {
    updateNode(nodeId, {
      config: {
        assertions,
        mode,
        on_failure: onFailure,
        continue_on_failure: continueOnFailure,
        preset: preset || undefined,
      },
    });
  };

  return (
    <div className="space-y-4 p-4">
      {/* 预设选择 */}
      <div>
        <label className="text-sm font-medium">断言预设</label>
        <select
          value={preset}
          onChange={(e) => setPreset(e.target.value)}
          className="w-full mt-1 border rounded px-3 py-2"
        >
          {PRESET_OPTIONS.map(opt => (
            <option key={opt.value} value={opt.value}>{opt.label}</option>
          ))}
        </select>
      </div>

      {/* 全局配置 */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="text-sm font-medium">组合模式</label>
          <select
            value={mode}
            onChange={(e) => setMode(e.target.value as AssertionMode)}
            className="w-full mt-1 border rounded px-3 py-2"
          >
            <option value="all">全部通过 (AND)</option>
            <option value="any">任一通过 (OR)</option>
          </select>
        </div>
        <div>
          <label className="text-sm font-medium">失败行为</label>
          <select
            value={onFailure}
            onChange={(e) => setOnFailure(e.target.value as OnFailureMode)}
            className="w-full mt-1 border rounded px-3 py-2"
          >
            <option value="route_false">路由到 false 分支</option>
            <option value="throw">抛出异常终止</option>
          </select>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          id="continueOnFailure"
          checked={continueOnFailure}
          onChange={(e) => setContinueOnFailure(e.target.checked)}
        />
        <label htmlFor="continueOnFailure" className="text-sm">
          失败后继续执行（收集所有失败）
        </label>
      </div>

      {/* 断言列表 */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium">断言列表</label>
          <button
            onClick={addAssertion}
            className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1"
          >
            <Plus size={14} /> 添加断言
          </button>
        </div>

        <div className="space-y-2">
          {assertions.map((assertion, index) => (
            <AssertionItem
              key={assertion.id}
              assertion={assertion}
              onChange={(updates) => updateAssertion(index, updates)}
              onRemove={() => removeAssertion(index)}
            />
          ))}
        </div>

        {assertions.length === 0 && (
          <div className="text-center text-gray-400 py-4 border rounded">
            暂无断言，点击上方按钮添加
          </div>
        )}
      </div>

      {/* 保存按钮 */}
      <button
        onClick={saveConfig}
        className="w-full bg-blue-600 text-white py-2 rounded hover:bg-blue-700"
      >
        保存配置
      </button>
    </div>
  );
}

// 单条断言配置项
function AssertionItem({
  assertion,
  onChange,
  onRemove,
}: {
  assertion: Assertion;
  onChange: (updates: Partial<Assertion>) => void;
  onRemove: () => void;
}) {
  return (
    <div className="border rounded p-3 space-y-2">
      <div className="flex items-center gap-2">
        <GripVertical size={14} className="text-gray-400 cursor-move" />
        <input
          type="checkbox"
          checked={assertion.enabled ?? true}
          onChange={(e) => onChange({ enabled: e.target.checked })}
          title="启用/禁用"
        />
        <input
          type="text"
          placeholder="断言描述（可选）"
          value={assertion.description ?? ''}
          onChange={(e) => onChange({ description: e.target.value })}
          className="flex-1 text-sm border-b border-transparent focus:border-blue-500 outline-none"
        />
        <button onClick={onRemove} className="text-red-500 hover:text-red-700">
          <Trash2 size={14} />
        </button>
      </div>

      <div className="grid grid-cols-12 gap-2">
        {/* 目标字段 */}
        <input
          type="text"
          placeholder="目标字段 (如: body.data.id)"
          value={assertion.target}
          onChange={(e) => onChange({ target: e.target.value })}
          className="col-span-4 text-sm border rounded px-2 py-1"
        />

        {/* 操作符 */}
        <select
          value={assertion.operator}
          onChange={(e) => onChange({ operator: e.target.value as AssertionOperator })}
          className="col-span-3 text-sm border rounded px-2 py-1"
        >
          {OPERATOR_OPTIONS.map(opt => (
            <option key={opt.value} value={opt.value}>{opt.label}</option>
          ))}
        </select>

        {/* 期望值 */}
        <input
          type="text"
          placeholder="期望值"
          value={String(assertion.expected)}
          onChange={(e) => {
            // 尝试解析为 JSON，否则作为字符串
            try {
              const parsed = JSON.parse(e.target.value);
              onChange({ expected: parsed });
            } catch {
              onChange({ expected: e.target.value });
            }
          }}
          className="col-span-5 text-sm border rounded px-2 py-1"
        />
      </div>
    </div>
  );
}
```

### 5.3 画布节点组件

```tsx
// src/components/canvas/nodes/AssertionNode.tsx

import { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { CheckCircle, XCircle } from 'lucide-react';
import { AssertionNodeConfig } from '@/types/node';

export const AssertionNode = memo(({ data, selected }: NodeProps) => {
  const config = data.config as AssertionNodeConfig;
  const assertionCount = config?.assertions?.filter(a => a.enabled !== false).length ?? 0;
  const mode = config?.mode ?? 'all';
  const onFailure = config?.on_failure ?? 'route_false';

  return (
    <div
      className={`
        min-w-[180px] bg-white border-2 rounded-lg shadow-sm
        ${selected ? 'border-blue-500' : 'border-gray-200'}
      `}
    >
      {/* 头部 */}
      <div className="flex items-center gap-2 px-3 py-2 border-b bg-amber-50 rounded-t-lg">
        <CheckCircle size={16} className="text-amber-600" />
        <span className="font-medium text-sm">断言</span>
        <span className="ml-auto text-xs text-gray-500">
          {assertionCount} 条
        </span>
      </div>

      {/* 内容 */}
      <div className="px-3 py-2 text-xs text-gray-600 space-y-1">
        <div>模式: {mode === 'all' ? '全部通过' : '任一通过'}</div>
        <div className="flex items-center gap-1">
          失败时: 
          {onFailure === 'route_false' ? (
            <span className="text-orange-600">路由 false</span>
          ) : (
            <span className="text-red-600">抛出异常</span>
          )}
        </div>
      </div>

      {/* 输入句柄 */}
      <Handle
        type="target"
        position={Position.Left}
        className="w-3 h-3 bg-gray-400"
      />

      {/* True 输出句柄 */}
      <Handle
        type="source"
        position={Position.Right}
        id="true"
        className="w-3 h-3 bg-green-500"
        style={{ top: '30%' }}
      />
      <span className="absolute right-6 text-xs text-green-600" style={{ top: '28%' }}>
        ✓
      </span>

      {/* False 输出句柄 */}
      {onFailure === 'route_false' && (
        <>
          <Handle
            type="source"
            position={Position.Right}
            id="false"
            className="w-3 h-3 bg-red-500"
            style={{ top: '70%' }}
          />
          <span className="absolute right-6 text-xs text-red-600" style={{ top: '68%' }}>
            ✗
          </span>
        </>
      )}
    </div>
  );
});

AssertionNode.displayName = 'AssertionNode';
```

---

## 六、调试模式集成

### 6.1 后端调试事件

```rust
// src-tauri/crates/flow-engine/src/debug/assertion_debug.rs

impl DebugExecutor {
    pub async fn execute_assertion(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
    ) -> ExecutionResult {
        // 发送断言开始事件
        self.send_event(DebugEvent::AssertionStarted {
            node_id: node.id.clone(),
        });

        let result = AssertionExecutor.execute(node, context).await;

        // 发送断言完成事件（包含详细结果）
        if let Ok(ref output) = result {
            if let Ok(assertion_output) = serde_json::from_value::<AssertionOutput>(
                output.output.clone()
            ) {
                self.send_event(DebugEvent::AssertionCompleted {
                    node_id: node.id.clone(),
                    passed: assertion_output.passed,
                    total: assertion_output.total,
                    passed_count: assertion_output.passed_count,
                    failed_count: assertion_output.failed_count,
                    failed_assertions: assertion_output.failed_assertions,
                });
            }
        }

        result
    }
}
```

### 6.2 前端调试日志显示

```tsx
// src/components/debug/AssertionResultView.tsx

export function AssertionResultView({ result }: { result: AssertionOutput }) {
  return (
    <div className="space-y-2">
      <div className={cn(
        "flex items-center gap-2 font-medium",
        result.passed ? "text-green-600" : "text-red-600"
      )}>
        {result.passed ? <CheckCircle size={16} /> : <XCircle size={16} />}
        断言结果: {result.passed ? '通过' : '失败'}
        <span className="text-gray-400 text-sm">
          ({result.passed_count}/{result.total})
        </span>
      </div>

      {result.failed_assertions.length > 0 && (
        <div className="bg-red-50 border border-red-200 rounded p-2 space-y-1">
          <div className="text-sm font-medium text-red-700">失败详情:</div>
          {result.failed_assertions.map(failed => (
            <div key={failed.id} className="text-sm text-red-600">
              • {failed.description || failed.target}
              <div className="ml-4 text-gray-500">
                期望: {JSON.stringify(failed.expected)}
              </div>
              <div className="ml-4 text-gray-500">
                实际: {JSON.stringify(failed.actual)}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

---

## 七、测试策略

### 7.1 后端单元测试

| 测试项 | 测试内容 |
|--------|---------|
| 操作符测试 | 所有操作符的正确性 |
| JSONPath 提取 | 简单路径、嵌套路径、数组索引 |
| 组合模式 | all 模式、any 模式 |
| 失败行为 | route_false 分支、throw 异常 |
| 预设加载 | 各预设的正确加载 |

### 7.2 前端单元测试

| 测试项 | 测试内容 |
|--------|---------|
| 配置面板渲染 | 空状态、已有配置 |
| 断言增删改 | 添加、删除、修改断言 |
| 表单交互 | 下拉选择、输入框 |

### 7.3 CLI 集成测试

包含断言节点的完整流程执行测试。

---

## 八、配置示例

```json
{
  "id": "assert_login_response",
  "type": "assertion",
  "name": "验证登录响应",
  "config": {
    "assertions": [
      {
        "id": "status_ok",
        "description": "登录接口应返回200",
        "target": "status",
        "operator": "eq",
        "expected": 200,
        "enabled": true
      },
      {
        "id": "has_token",
        "description": "响应应包含token",
        "target": "body.data.token",
        "operator": "exists",
        "expected": true,
        "enabled": true
      },
      {
        "id": "token_format",
        "description": "token应为JWT格式",
        "target": "body.data.token",
        "operator": "matches",
        "expected": "^eyJ[A-Za-z0-9-_=]+\\.[A-Za-z0-9-_=]+\\.[A-Za-z0-9-_.+/=]*$",
        "enabled": true
      },
      {
        "id": "response_time",
        "description": "响应时间应小于1秒",
        "target": "response_time",
        "operator": "lt",
        "expected": 1000,
        "enabled": true
      }
    ],
    "mode": "all",
    "on_failure": "throw",
    "continue_on_failure": true
  }
}
```

---

## 九、实现步骤

| 阶段 | 任务 | 预估工时 |
|------|------|---------|
| **Phase 1: 类型定义** | TypeScript + Rust 类型对齐 | 2h |
| **Phase 2: 后端核心** | 断言执行器 + 操作符实现 | 4h |
| **Phase 3: 预设系统** | 断言预设 + 列表 API | 2h |
| **Phase 4: 前端组件** | 配置面板 + 画布节点 | 4h |
| **Phase 5: 调试集成** | 调试事件 + 前端展示 | 2h |
| **Phase 6: 测试** | 单元测试 + 集成测试 | 3h |
| **Phase 7: 回归测试** | 全量回归测试 | 1h |
| **总计** | | **18h** |

---

## 十、风险与依赖

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| JSONPath 解析复杂 | 中 | 先实现简化版点号路径，后续可引入 jsonpath crate |
| 正则表达式安全 | 低 | 使用 regex crate，已做 DoS 防护 |
| 前端表单交互复杂 | 中 | 拆分为独立子组件，逐个实现 |

---

## 十一、后续版本规划

| 功能 | 说明 |
|------|------|
| JSON Schema 验证 | 支持完整的 JSON Schema 验证 |
| 断言模板管理 | 用户自定义断言模板，持久化保存 |
| 断言结果导出 | 导出断言报告为独立文件 |
| 高级 JSONPath | 支持完整的 JSONPath 语法（过滤器、通配符等） |
