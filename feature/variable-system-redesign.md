# 变量系统重设计方案

> 状态：待实现
> 创建时间：2026-03-03
> 最后更新：2026-03-03

## 1. 功能概述

### 1.1 背景

当前变量系统存在以下问题：
- 节点输出自动合并到全局变量，用户无法控制
- 变量命名空间不隔离，同名变量会被覆盖
- 用户无法直观知道有哪些变量可用
- 变量引用缺乏提示和辅助

### 1.2 设计目标

1. **显式控制**：用户明确配置每个节点输出哪些变量
2. **清晰可见**：用户能直观看到所有可用变量及其来源
3. **友好引用**：提供变量选择器，方便用户引用
4. **类型安全**：条件判断支持明确类型选择

---

## 2. 核心设计决策

| # | 设计点 | 决策 |
|---|--------|------|
| 1 | 变量存储 | 全局变量池 + 来源 UUID 追踪 |
| 2 | 变量命名 | 用户自定义，全局唯一 |
| 3 | 说明字段 | 可选，不校验唯一性 |
| 4 | 输出映射 | 显式配置，提示可用字段，用户按需添加 |
| 5 | 变量可见性 | 静态声明 + 执行顺序 |
| 6 | 引用语法 | `{{变量名}}` |
| 7 | 条件类型选择器 | 根据操作符自动显示/隐藏 |
| 8 | 节点删除 | 级联删除变量，引用处提示失效 |
| 9 | 变量面板 | 按来源分组 + 输入框选择 icon |
| 10 | 变量类型 | 保留原始 JSON 类型 |
| 11 | 初始变量 | Entry 节点配置 |

---

## 3. 数据结构设计

### 3.1 变量存储结构

```typescript
// 前端类型定义 - src/types/variable.ts

/**
 * 变量定义
 */
export interface Variable {
  /** 变量名（全局唯一） */
  name: string;
  /** 变量值（保留原始 JSON 类型） */
  value: unknown;
  /** 来源节点 UUID */
  sourceNodeUuid: string;
  /** 说明（可选） */
  description?: string;
}

/**
 * 全局变量池
 */
export interface VariablePool {
  /** 变量列表 */
  variables: Variable[];
}

/**
 * 输出映射配置
 */
export interface OutputMapping {
  /** 源字段路径（响应中的路径） */
  source: string;
  /** 目标变量名 */
  target: string;
  /** 说明（可选） */
  description?: string;
  /** 是否启用 */
  enabled: boolean;
}
```

```rust
// 后端类型定义 - src-tauri/src/variable.rs

/// 变量定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// 变量名（全局唯一）
    pub name: String,
    /// 变量值
    pub value: serde_json::Value,
    /// 来源节点 UUID
    pub source_node_uuid: String,
    /// 说明（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 全局变量池
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariablePool {
    /// 变量列表
    pub variables: Vec<Variable>,
}

/// 输出映射配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMapping {
    /// 源字段路径
    pub source: String,
    /// 目标变量名
    pub target: String,
    /// 说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }
```

### 3.2 节点配置扩展

```typescript
// 前端 - 扩展 HttpConfig
export interface HttpConfig {
  type: 'http';
  url: string;
  method?: HttpMethod;
  // ... 其他现有字段
  
  /** 输出变量映射 */
  output_mappings?: OutputMapping[];
}

// 前端 - 扩展 EntryConfig
export interface EntryConfig {
  type: 'entry';
  
  /** 初始变量 */
  initial_variables?: InitialVariable[];
}

export interface InitialVariable {
  /** 变量名 */
  name: string;
  /** 默认值 */
  value: unknown;
  /** 说明（可选） */
  description?: string;
}
```

### 3.3 条件配置扩展

```typescript
// 前端 - 扩展 ConditionalConfig
export interface ConditionalConfig {
  type: 'conditional';
  condition_type?: ConditionType;
  field_path?: string;
  operator?: ConditionOperator;
  expected_value?: unknown;
  
  /** 期望值类型（用于等于/不等于操作符） */
  expected_value_type?: ExpectedValueType;
}

export type ExpectedValueType = 'number' | 'string' | 'boolean' | 'null';
```

---

## 4. UI/UX 设计

### 4.1 Entry 节点配置面板

```
┌─────────────────────────────────────────────────────────────┐
│ 入口节点配置                                                 │
├─────────────────────────────────────────────────────────────┤
│ 节点名称: [开始________________]                             │
│                                                             │
│ ─────────────────────────────────────────────────────────── │
│ 📥 初始变量                                                  │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ 变量名              默认值                说明           │ │
│ │ api_base       →   [https://api...]     [API地址]       │ │
│ │ api_key        →   [xxxxx]              [密钥]          │ │
│ │                                                         │ │
│ │ [➕ 添加变量]                                            │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ 💡 提示：初始变量将直接暴露给下游节点使用                    │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 HTTP 节点输出映射配置

```
┌─────────────────────────────────────────────────────────────┐
│ HTTP 请求配置                                                │
├─────────────────────────────────────────────────────────────┤
│ URL:     [https://api.example.com/users] [📋]              │
│ Method:  [GET ▼]                                            │
│                                                             │
│ ─────────────────────────────────────────────────────────── │
│ 📤 输出变量映射                                              │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ 可用返回字段: status, body, headers, request            │ │
│ │                                                         │ │
│ │ 源字段            变量名              说明               │ │
│ │ ☑ status      →  [user_status]     [登录状态码]        │ │
│ │ ☑ body.data   →  [user_list]       [用户列表]          │ │
│ │ ☐ body.total  →  [____________]    [___________]       │ │
│ │                                                         │ │
│ │ [➕ 添加映射]                                            │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ 💡 提示：                                                   │
│ • 源字段支持嵌套路径，如 body.data.id                       │
│ • 变量名全局唯一，不可与已有变量重名                         │
│ • 说明字段可选，用于描述变量含义                             │
└─────────────────────────────────────────────────────────────┘
```

### 4.3 变量面板

```
┌─────────────────────────────────────────────────────────────┐
│ 📋 变量面板                                         [折叠 ▼] │
├─────────────────────────────────────────────────────────────┤
│ 🔍 [搜索变量...                              ]              │
│                                                             │
│ 📦 初始变量 (开始)                                          │
│   ├── api_base: API 地址                                    │
│   └── api_key: 密钥                                         │
│                                                             │
│ 📦 获取用户列表 (http_1)                                    │
│   ├── user_status: 登录状态码                               │
│   └── user_list: 用户列表                                   │
│                                                             │
│ 📦 创建订单 (http_2)                                        │
│   └── order_id: 订单号                                      │
└─────────────────────────────────────────────────────────────┘
```

### 4.4 变量选择器

```
输入框旁边显示变量选择图标：

URL: [https://api.example.com/users?id={{____}}] [📋]
                                        ↓ 点击图标
                              ┌─────────────────────┐
                              │ 🔍 搜索...          │
                              ├─────────────────────┤
                              │ 📦 初始变量         │
                              │   ├── api_base      │
                              │   └── api_key       │
                              │ 📦 获取用户列表     │
                              │   ├── user_status   │
                              │   └── user_list     │
                              └─────────────────────┘
```

### 4.5 条件节点配置

```
┌─────────────────────────────────────────────────────────────┐
│ 条件配置                                                     │
├─────────────────────────────────────────────────────────────┤
│ 条件类型: [简单条件 ▼]                                       │
│                                                             │
│ 字段路径: [user_status      ] [📋]                          │
│ 操作符:   [等于 ▼]                                          │
│ 期望值:   [200]              类型: [数字 ▼]                 │
│                                                             │
│ ─────────────────────────────────────────────────────────── │
│ 💡 类型选择说明：                                           │
│ • 数字: 输入值将解析为数字                                   │
│ • 字符串: 输入值将作为字符串                                 │
│ • 布尔: 输入 true 或 false                                  │
│ • null: 表示空值                                            │
└─────────────────────────────────────────────────────────────┘
```

**操作符与类型选择器关系**：

| 操作符 | 类型选择器 | 固定类型 |
|--------|-----------|---------|
| 等于 | ✅ 显示 | - |
| 不等于 | ✅ 显示 | - |
| 大于 | ❌ 隐藏 | 数字 |
| 大于等于 | ❌ 隐藏 | 数字 |
| 小于 | ❌ 隐藏 | 数字 |
| 小于等于 | ❌ 隐藏 | 数字 |
| 包含 | ❌ 隐藏 | 字符串 |
| 开头是 | ❌ 隐藏 | 字符串 |
| 结尾是 | ❌ 隐藏 | 字符串 |
| 存在 | ❌ 隐藏 | 无需期望值 |

### 4.6 变量重名提示

```
┌─────────────────────────────────────────────────────────────┐
│ 📤 输出变量映射                                              │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ 源字段            变量名              说明               │ │
│ │ ☑ status      →  [user_status  ]    [___________]      │ │
│ │                      ↓ 实时检测                         │ │
│ │              ┌────────────────────────────┐             │ │
│ │              │ ⚠️ 变量名已存在              │             │ │
│ │              │                            │             │ │
│ │              │ "user_status" 已被使用：    │             │ │
│ │              │   • 获取用户 (http_1)       │             │ │
│ │              │                            │             │ │
│ │              │ 请使用其他名称              │             │ │
│ │              └────────────────────────────┘             │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 4.7 变量失效提示

```
当节点被删除后，引用该节点变量的配置面板显示：

┌─────────────────────────────────────────────────────────────┐
│ 条件配置                                                     │
├─────────────────────────────────────────────────────────────┤
│ ⚠️ 变量引用失效                                              │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ 以下变量已失效（来源节点已删除）：                        │ │
│ │   • user_status                                          │ │
│ │   • user_list                                            │ │
│ │                                                         │ │
│ │ 请重新选择有效的变量或删除此条件                          │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ 字段路径: [user_status ⚠️] [📋]                             │
│           └── 红色高亮，提示失效                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. 交互流程

### 5.1 配置 Entry 节点初始变量

1. 用户点击 Entry 节点
2. 右侧显示配置面板
3. 用户在"初始变量"区域添加变量
4. 填写变量名、默认值、说明（可选）
5. 保存后变量立即对下游可见

### 5.2 配置 HTTP 节点输出映射

1. 用户点击 HTTP 节点
2. 右侧显示配置面板
3. 系统提示可用返回字段：status, body, headers, request
4. 用户点击"添加映射"
5. 填写源字段（如 `body.data.id`）、变量名、说明
6. 系统实时检测变量名唯一性
7. 保存后变量对下游可见

### 5.3 引用变量

1. 用户在输入框中输入或点击变量选择图标
2. 弹出变量选择器，显示所有可用变量
3. 变量按来源节点分组
4. 用户选择变量，自动插入 `{{变量名}}`

### 5.4 删除节点

1. 用户删除节点
2. 系统检测该节点输出的变量
3. 从全局变量池删除这些变量
4. 标记引用这些变量的配置为"失效"
5. 用户打开失效配置面板时显示提示

---

## 6. 技术实现要点

### 6.1 后端改动

| 文件 | 改动内容 |
|------|----------|
| `variable.rs` (新建) | Variable, VariablePool, OutputMapping 类型定义 |
| `context.rs` | 重构 VariableStorage，改为 VariablePool |
| `executor.rs` | 修改变量合并逻辑，按 output_mappings 配置 |
| `node_config.rs` | 添加 output_mappings 字段到各节点配置 |
| `http_executor.rs` | 解析 output_mappings，按配置提取并设置变量 |
| `conditional_executor.rs` | 支持类型选择器，按类型解析期望值 |
| `flow.rs` | 移除 initial_variables（迁移到 Entry 节点） |

### 6.2 前端改动

| 文件 | 改动内容 |
|------|----------|
| `types/variable.ts` (新建) | Variable, VariablePool, OutputMapping 类型定义 |
| `types/node.ts` | 扩展各节点配置，添加 output_mappings |
| `stores/variable.ts` (新建) | 变量池状态管理 |
| `components/panels/EntryConfig.tsx` | 添加初始变量配置 UI |
| `components/panels/HttpConfig.tsx` | 添加输出映射配置 UI |
| `components/panels/ConditionalConfig.tsx` | 添加类型选择器 |
| `components/panels/VariablePanel.tsx` (新建) | 变量面板组件 |
| `components/ui/VariableSelector.tsx` (新建) | 变量选择器组件 |
| `components/ui/VariableInput.tsx` (新建) | 带变量选择器的输入框 |

### 6.3 变量解析逻辑

```rust
// executor.rs

/// 按输出映射配置提取变量
fn apply_output_mappings(
    response_data: &serde_json::Value,
    mappings: &[OutputMapping],
    node_uuid: &str,
    pool: &mut VariablePool,
) -> Result<(), String> {
    for mapping in mappings.iter().filter(|m| m.enabled) {
        // 从响应中提取值
        let value = extract_value(response_data, &mapping.source)?;
        
        // 检查变量名唯一性
        if pool.variables.iter().any(|v| v.name == mapping.target) {
            return Err(format!("变量名 '{}' 已存在", mapping.target));
        }
        
        // 添加到变量池
        pool.variables.push(Variable {
            name: mapping.target.clone(),
            value,
            source_node_uuid: node_uuid.to_string(),
            description: mapping.description.clone(),
        });
    }
    Ok(())
}

/// 从响应数据中提取值
fn extract_value(data: &serde_json::Value, path: &str) -> Result<serde_json::Value, String> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = data;
    
    for part in parts {
        current = current.get(part)
            .ok_or_else(|| format!("路径 '{}' 不存在", path))?;
    }
    
    Ok(current.clone())
}
```

### 6.4 条件类型解析

```rust
// conditional_executor.rs

/// 根据类型解析期望值
fn parse_expected_value(
    value_str: &str,
    value_type: ExpectedValueType,
) -> Result<serde_json::Value, String> {
    match value_type {
        ExpectedValueType::Number => {
            value_str.parse::<f64>()
                .map(|n| json!(n))
                .map_err(|_| format!("'{}' 不是有效的数字", value_str))
        }
        ExpectedValueType::String => {
            Ok(json!(value_str))
        }
        ExpectedValueType::Boolean => {
            match value_str.to_lowercase().as_str() {
                "true" => Ok(json!(true)),
                "false" => Ok(json!(false)),
                _ => Err(format!("'{}' 不是有效的布尔值", value_str))
            }
        }
        ExpectedValueType::Null => {
            Ok(json!(null))
        }
    }
}
```

---

## 7. 实现步骤

### Phase 1: 类型定义与存储 (1-2 天)

1. 后端：创建 `variable.rs`，定义 Variable, VariablePool, OutputMapping
2. 后端：修改 `context.rs`，重构 VariableStorage
3. 前端：创建 `types/variable.ts`
4. 前端：扩展 `types/node.ts` 各节点配置

### Phase 2: Entry 节点初始变量 (1 天)

1. 后端：Entry 执行器读取初始变量配置
2. 前端：EntryConfig 组件添加初始变量配置 UI
3. 测试：初始变量传递到下游

### Phase 3: 输出映射配置 (2-3 天)

1. 后端：修改 executor.rs，按 output_mappings 设置变量
2. 后端：修改各执行器支持输出映射
3. 前端：HttpConfig 组件添加输出映射 UI
4. 前端：变量名唯一性校验
5. 测试：输出映射功能

### Phase 4: 变量面板与选择器 (2 天)

1. 前端：创建 VariablePanel 组件
2. 前端：创建 VariableSelector 组件
3. 前端：创建 VariableInput 组件（带选择器图标）
4. 前端：集成到各配置面板

### Phase 5: 条件类型选择器 (1 天)

1. 后端：支持类型选择器解析
2. 前端：ConditionalConfig 添加类型选择器
3. 前端：根据操作符自动显示/隐藏

### Phase 6: 节点删除处理 (1 天)

1. 后端：级联删除变量
2. 前端：变量失效提示 UI
3. 前端：失效变量高亮

### Phase 7: 测试与优化 (1-2 天)

1. 单元测试
2. 集成测试
3. UX 优化

---

## 8. 迁移策略

### 8.1 数据迁移

旧流程文件可能使用 `flow.initial_variables`，需要迁移：
- 后端兼容：读取时自动迁移到 Entry 节点配置
- 前端兼容：显示迁移提示

### 8.2 向后兼容

- 变量引用语法 `{{变量名}}` 保持不变
- 旧流程执行时自动转换

---

## 9. 测试用例

### 9.1 变量配置

- [ ] Entry 节点添加初始变量，下游可见
- [ ] HTTP 节点配置输出映射，变量正确设置
- [ ] 变量名重名时提示错误
- [ ] 变量说明可选，不影响功能

### 9.2 变量引用

- [ ] 变量选择器正确显示所有可用变量
- [ ] 选择变量后正确插入 `{{变量名}}`
- [ ] 变量面板按来源节点分组显示

### 9.3 条件判断

- [ ] 等于操作符显示类型选择器
- [ ] 大于操作符隐藏类型选择器，固定数字
- [ ] 类型解析正确（数字/字符串/布尔/null）

### 9.4 节点删除

- [ ] 删除节点后变量从池中移除
- [ ] 引用失效变量时显示提示
- [ ] 失效变量高亮显示

---

## 10. 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-03-03 | v1.0 | 初始设计文档 |
