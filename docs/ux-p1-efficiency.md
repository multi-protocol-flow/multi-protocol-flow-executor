# UX 优化路线图 — P1：效率优化 (High Priority)

> 核心体验（P0）完成后，立即投入效率优化。目标：用户完成常见操作的效率提升 2 倍以上。

---

## 目录

1. [节点配置面板体验升级](#1-节点配置面板体验升级)
2. [快捷键系统重大改进](#2-快捷键系统重大改进)
3. [搜索与发现系统](#3-搜索与发现系统)
4. [变量系统可视化](#4-变量系统可视化)
5. [实现计划](#5-实现计划)

---

## 1. 节点配置面板体验升级

### 1.1 预设配置模板

**目标**：每个节点类型提供 2-3 个"一键填充"的配置预设，减少重复劳动。

#### 实现方案

```typescript
// 新文件: src/lib/nodePresets.ts
interface NodePreset {
  id: string;
  name: string;
  description: string;
  nodeType: NodeType;
  data: Record<string, unknown>;  // 完整的 node.data 内容
}

// HTTP 节点预设示例
const HTTP_PRESETS: NodePreset[] = [
  {
    id: 'http-get-users',
    name: 'GET /api/users',
    description: '获取用户列表 - REST API 典型 GET 请求',
    nodeType: 'http',
    data: {
      method: 'get',
      url: 'https://jsonplaceholder.typicode.com/users',
      headers: [{ key: 'Accept', value: 'application/json' }],
      body_type: 'none',
      timeout_ms: 30000,
    },
  },
  {
    id: 'http-post-login',
    name: 'POST /auth/login',
    description: '用户登录 - JSON body 认证请求',
    nodeType: 'http',
    data: {
      method: 'post',
      url: '{{base_url}}/auth/login',
      body_type: 'body',
      body_content: '{"username":"{{username}}","password":"{{password}}"}',
      headers: [{ key: 'Content-Type', value: 'application/json' }],
    },
  },
  {
    id: 'http-health-check',
    name: '健康检查',
    description: '简单 GET 探活，超时 5s',
    nodeType: 'http',
    data: {
      method: 'get',
      url: '{{health_endpoint}}',
      timeout_ms: 5000,
    },
  },
];
```

#### UI 呈现

在 `HttpConfig.tsx`（及其他所有 Config 面板）顶部增加"预设"下拉：

```tsx
// 每个 Config 面板增加预设选择器
<Select onValueChange={applyPreset}>
  <SelectTrigger className="w-full">
    <SelectValue placeholder="选择一个预设配置..." />
  </SelectTrigger>
  <SelectContent>
    {PRESETS_BY_TYPE[nodeType].map(p => (
      <SelectItem key={p.id} value={p.id}>
        {p.name} — {p.description}
      </SelectItem>
    ))}
  </SelectContent>
</Select>
```

### 1.2 智能默认值

**目标**：根据上下文（上游节点类型、已有配置）自动推断新节点的合理默认值。

#### 实现方案

```typescript
// 新文件: src/lib/nodeDefaults.ts
// 根据上游节点推断下游节点参数

function inferDefaults(
  upstreamNode: Node | null,
  targetType: NodeType
): Record<string, unknown> {
  if (!upstreamNode) return {};

  switch (targetType) {
    case 'kafka_produce':
      // 如果上游是 kafka_connect，自动使用相同的 brokers
      if (upstreamNode.type === 'kafka_connect') {
        return { brokers: upstreamNode.data.brokers };
      }
      break;

    case 'db_query':
      // 如果上游是 db_connect，自动使用相同的 connection_id
      if (upstreamNode.type === 'db_connect') {
        return { connection_id: upstreamNode.data.connection_id };
      }
      break;

    // 其他协议类推...
  }

  return {};
}
```

#### 调用时机

在 `addNode()` 或拖拽添加节点时调用：

```typescript
// FlowCanvas.tsx - onDrop 回调中
const inferred = inferDefaults(upstreamNode, nodeType);
addNode({
  ...newNode,
  data: { ...createDefaultNodeData(nodeType), ...inferred },
});
```

### 1.3 配置面板折叠分组

**目标**：长配置表单（Kafka、gRPC、AMQP 等）按逻辑分组折叠，默认收起"高级"区域。

#### 实现方案

```tsx
// 通用折叠区组件
// 新建: src/components/ui/ConfigSection.tsx
interface ConfigSectionProps {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
  badge?: string;   // 可选：标注"必填""高级"等
}
```

使用方式：

```tsx
// KafkaProduceConfig.tsx 示例
<ConfigSection title="基础配置" defaultOpen>
  <FormField ... broker />
  <FormField ... topic />
</ConfigSection>

<ConfigSection title="消息配置" defaultOpen>
  <FormField ... key />
  <FormField ... value />
  <FormField ... headers />
</ConfigSection>

<ConfigSection title="高级选项" badge="可选">
  <FormField ... acks />
  <FormField ... timeout />
  <FormField ... compression />
</ConfigSection>
```

#### 实现细节

- 使用 `Collapsible` 来自 `@radix-ui/react-collapsible`
- 折叠状态存储在 `localStorage`，key 为 `config-section-${nodeType}-${sectionKey}`
- 添加展开/折叠全部按钮
- 带 `badge="必填"` 的 section 默认展开

### 1.4 配置复制/粘贴

**目标**：在同类型节点间快速复制配置。

#### 实现方案

```typescript
// 节点右键菜单新增项
// 使用 Clipboard API 存储/读取配置 JSON

interface NodeConfigClipboard {
  nodeType: string;          // 节点类型，用于校验兼容性
  data: Record<string, unknown>;  // 完整配置
  copiedAt: number;          // 复制时间
  sourceNodeName: string;    // 来源节点名称（显示用）
}
```

- 复制：`localStorage.setItem('node-config-clipboard', JSON.stringify(config))`
- 粘贴：检查类型兼容（同类型节点间可以粘贴，跨类型显示警告）
- UI：右键菜单 → `复制配置` / `粘贴配置`（仅在剪贴板有兼容配置时启用）

---

## 2. 快捷键系统重大改进

### 2.1 快捷键面板

**目标**：`Cmd+/` 或 `Ctrl+/` 弹出完整快捷键列表，可搜索。

#### 实现方案

```tsx
// 新建: src/components/dialogs/ShortcutsDialog.tsx
// 由 useKeyboardShortcuts 触发

const SHORTCUT_GROUPS = [
  {
    group: '编辑',
    shortcuts: [
      { keys: ['Ctrl+Z'], description: '撤销' },
      { keys: ['Ctrl+Shift+Z', 'Ctrl+Y'], description: '重做' },
      { keys: ['Ctrl+D'], description: '复制选中节点' },
      { keys: ['Delete', 'Backspace'], description: '删除选中节点/边' },
      { keys: ['Ctrl+A'], description: '全选节点' },
    ],
  },
  {
    group: '执行',
    shortcuts: [
      { keys: ['Ctrl+Enter'], description: '执行流程' },
      { keys: ['Ctrl+Shift+Enter'], description: '从当前节点开始执行' },
      { keys: ['Ctrl+B'], description: '切换断点' },
      { keys: ['F5'], description: '继续调试' },
      { keys: ['F10'], description: '单步执行' },
      { keys: ['Shift+F5'], description: '停止调试' },
    ],
  },
  {
    group: '导航',
    shortcuts: [
      { keys: ['Ctrl+K', 'Ctrl+F'], description: '全文搜索（节点 + 配置）' },
      { keys: ['Ctrl+L'], description: '聚焦到节点列表' },
      { keys: ['Ctrl+1' ], description: '切换左侧面板' },
      { keys: ['Ctrl+2'], description: '切换底部面板' },
      { keys: ['Ctrl+3'], description: '切换右侧面板' },
      { keys: ['Ctrl+,'] , description: '打开设置' },
    ],
  },
  {
    group: '画布',
    shortcuts: [
      { keys: ['Space + 拖拽'], description: '平移画布' },
      { keys: ['Ctrl+滚轮'], description: '缩放画布' },
      { keys: ['Ctrl+0'], description: '重置缩放' },
      { keys: ['Ctrl+ - / ='], description: '缩小/放大' },
      { keys: ['右键'], description: '画布右键打开节点选择器' },
      { keys: ['F'], description: '适应画布到窗口' },
    ],
  },
];
```

#### 关联文件

| 文件 | 修改内容 |
|------|---------|
| `src/` 新建 `components/dialogs/ShortcutsDialog.tsx` | 快捷键面板 |
| `src/hooks/useKeyboardShortcuts.ts` | 注册 `Ctrl+/` 触发面板 |
| `src/stores/ui.ts` | 新增 `showShortcutsDialog` |

### 2.2 快捷键可视化

**目标**：按钮 tooltip 和菜单项中显示快捷键提示。

#### 实现方案

```tsx
// 工具函数: src/lib/hotkeyLabels.ts
// 根据平台显示 Cmd/Ctrl
export function hotkeyLabel(keys: string[]): string {
  const mod = navigator.platform.includes('Mac') ? '⌘' : 'Ctrl+';
  return keys
    .map(k => k.replace(/^Ctrl\+/i, mod))
    .join(' 或 ');
}
```

在所有按钮的 `title` 属性中追加快捷键：

```tsx
// CanvasToolbar.tsx 中的撤销按钮
<button
  title={`${ll.flow.undo()} (${hotkeyLabel(['Ctrl+Z'])})`}
  ...
>
```

### 2.3 画布关键快捷键实现

对照快捷键列表，逐一确认 `useKeyboardShortcuts.ts` 中的实现：

| 快捷键 | 状态 | 实现位置 |
|--------|------|---------|
| `Ctrl+Z` / `Ctrl+Shift+Z` | ✅ 已有 | `useKeyboardShortcuts.ts` |
| `Ctrl+D` 复制节点 | ❌ 缺失 | 需新增 |
| `Ctrl+A` 全选 | ❌ 缺失 | 需新增 |
| `Ctrl+Enter` 执行 | ❌ 缺失 | 需新增 |
| `Ctrl+,` 设置 | ❌ 缺失 | 需新增 |
| `F` 适应画布 | ❌ 缺失 | 需新增 |
| `Ctrl+L` 聚焦节点列表 | ❌ 缺失 | 需新增 |

---

## 3. 搜索与发现系统

### 3.1 全流程搜索

**目标**：`Ctrl+Shift+F` 搜索当前流程所有节点的名称、配置字段、变量引用。

#### 实现方案

```typescript
// 新建: src/lib/flowSearch.ts
interface SearchResult {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  field: string;         // 匹配的字段路径
  value: string;         // 匹配的值（含上下文）
  matchType: 'name' | 'config' | 'variable';
}

function searchFlow(flow: Flow, query: string): SearchResult[] {
  const results: SearchResult[] = [];
  const lowerQuery = query.toLowerCase();

  for (const node of flow.nodes) {
    // 1. 匹配节点名称
    if (node.name.toLowerCase().includes(lowerQuery)) {
      results.push({ nodeUuid: node.uuid, nodeName: node.name, ... });
    }

    // 2. 深度递归匹配节点配置
    const configMatches = deepSearchConfig(node.data, lowerQuery, '');
    results.push(...configMatches);

    // 3. 匹配变量引用 {{...}}
    const variableMatches = searchVariableRefs(node.data, lowerQuery);
    results.push(...variableMatches);
  }

  return results;
}
```

#### UI

```tsx
// 搜索面板: SearchOverlay.tsx
// 类似 VS Code 的搜索界面
// 结果按节点分组，点击跳转到对应节点并高亮匹配字段

<CommandDialog open={isOpen} onOpenChange={setOpen}>
  <CommandInput placeholder="搜索节点名称、配置字段、变量引用..." />
  <CommandList>
    {groupedResults.map(group => (
      <CommandGroup heading={group.nodeName}>
        {group.matches.map(m => (
          <CommandItem onSelect={() => jumpToNode(m)}>
            <Search className="w-4 h-4" />
            {m.field}: {highlightMatch(m.value, query)}
          </CommandItem>
        ))}
      </CommandGroup>
    ))}
  </CommandList>
</CommandDialog>
```

### 3.2 全局跨流程搜索

**目标**：搜索所有已保存流程中的内容。

#### 实现方案

在 `FlowListPanel.tsx` 中搜索模式增强：

```typescript
// 搜索时不仅搜索名称，还加载并搜索流程内容
async function globalSearch(query: string): Promise<GlobalSearchResult[]> {
  const metas = await flowStorage.listFlows();
  const results: GlobalSearchResult[] = [];

  // 仅当搜索词较长时搜索内容，避免性能问题
  if (query.length >= 3) {
    await Promise.all(
      metas.map(async meta => {
        try {
          const flow = await flowStorage.loadFlow(meta.uuid);
          const matches = searchFlow(flow, query);
          if (matches.length > 0) {
            results.push({ flowMeta: meta, matches });
          }
        } catch { /* ignore */ }
      })
    );
  }

  return results;
}
```

### 3.3 NodePalette 搜索增强

**问题**：当前 NodePalette 仅在当前超级分类下搜索，且只搜索名称。

**改进**：

- 跨所有超级分类搜索
- 支持模糊匹配（`hhtp` → 仍匹配 `http`）
- 支持关键词别名（`rest` → `http`, `pubsub` → `mqtt/kafka`）
- 最近/最常用节点置顶

---

## 4. 变量系统可视化

### 4.1 变量选择器升级

**目标**：在配置输入中插入 `{{variable}}` 时，提供带预览值的可视化树形选择器。

#### 现状

`VariableSelector.tsx` 实现了基本的树形选择。需要增强：

- **值预览**：变量名旁显示当前值（如果有初始值或上游产生值）
- **类型标注**：显示值的类型（`string | number | object | array`）
- **来源追踪**：标注变量来自哪个节点（`body.user.name ← HTTP-1 响应`）
- **搜索过滤**：在长列表中搜索变量名

#### 实现

```tsx
// 增强 VariableSelector.tsx
interface VariableEntry {
  path: string;         // body.user.name
  displayName: string;  // user.name
  value: unknown;       // 当前值（预览）
  type: string;         // 类型标注
  sourceNode?: string;  // 来源节点名称
  sourceNodeId?: string; // 来源节点 UUID
}

// 变量来源分析：遍历流程中已配置的变量
function extractVariables(flow: Flow): VariableEntry[] {
  const vars: VariableEntry[] = [];

  // 1. 初始变量
  for (const [key, value] of Object.entries(flow.initial_variables ?? {})) {
    vars.push({
      path: `initial_variables.${key}`,
      displayName: key,
      value,
      type: typeof value,
      sourceNode: '流程初始变量',
    });
  }

  // 2. 各节点的输出变量
  for (const node of flow.nodes) {
    const outputs = getNodeOutputVariables(node);
    for (const out of outputs) {
      vars.push({
        ...out,
        sourceNode: node.name,
        sourceNodeId: node.uuid,
      });
    }
  }

  return vars;
}
```

### 4.2 变量引用着色

**目标**：在配置输入框中，`{{variable}}` 引用以特殊颜色高亮显示，与普通文本区分。

#### 实现方案

使用 `VariableInput.tsx` 组件增强：

```tsx
// 修改: VariableInput.tsx
// 使用 contentEditable 或 正则分词 + 染色渲染

// 方案A: 纯 CSS + 伪元素（简单但效果有限）
const highlightedValue = value.replace(
  /\{\{([^}]+)\}\}/g,
  (match, path) => {
    // 根据 path 是否有效着色
    const isValid = variablePaths.has(path);
    return `<span class="${cn(
      'text-violet-600 dark:text-violet-400 font-medium',
      !isValid && 'text-red-500 dark:text-red-400 line-through'
    )}">${match}</span>`;
  }
);

// 方案B: contentEditable + 词法解析（更好交互）
// 类似 VS Code 的 token 着色，支持点击跳转
```

### 4.3 变量定义-使用关系图

**目标**：显示"哪些节点使用了某变量"和"某节点使用了哪些变量"。

#### 实现

```tsx
// 新建: components/execution/VariableDependencyGraph.tsx
// 在 VariablePanel 中增加"变量关系"Tab

// 解析流程中所有 {{variable}} 引用
function buildVariableDependencyGraph(flow: Flow): VariableDependencyGraph {
  const graph: VariableDependencyGraph = { definitions: [], usages: [] };

  for (const node of flow.nodes) {
    // 1. 找出该节点定义的变量（输出变量）
    const outputs = getNodeOutputVariables(node);
    for (const out of outputs) {
      graph.definitions.push({
        nodeId: node.uuid,
        nodeName: node.name,
        variablePath: out.path,
      });
    }

    // 2. 找出该节点使用的变量（配置中的 {{}} 引用）
    const refs = extractVariableRefs(node.data);
    for (const ref of refs) {
      graph.usages.push({
        nodeId: node.uuid,
        nodeName: node.name,
        variablePath: ref,
      });
    }
  }

  return graph;
}
```

### 4.4 实时变量监视器

**目标**：执行过程中左侧面板实时显示变量值的变化，类似开发工具的 Watch 面板。

#### 实现

```tsx
// 修改: execution/VariableTreeView.tsx
// 在执行模式中切换为"监视模式"

// 通过 ExecutionEvent 通道接收变量变更
interface VariableChangedEvent {
  type: 'variable_changed';
  variable_path: string;
  old_value: unknown;
  new_value: unknown;
  node_uuid: string;
  timestamp: number;
}
```

- 变量值变化时以绿色闪烁动画高亮变化的行
- 显示变化时间和来源节点
- 支持固定关注的变量到顶部（pin）
- 支持搜索过滤变量名

---

## 5. 实现计划

### 里程碑 M1.1 — 配置体验（5 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 预设模板系统 | `nodePresets.ts` + 各 Config 面板 | 2d |
| 智能默认值 | `nodeDefaults.ts` + `FlowCanvas.tsx` | 1d |
| 配置折叠分组 | `ConfigSection.tsx` + 各 Config 面板 | 1.5d |
| 配置复制/粘贴 | `NodeContextMenu.tsx` | 0.5d |

### 里程碑 M1.2 — 快捷键系统（4 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 快捷键面板 | `ShortcutsDialog.tsx` | 1d |
| 所有按钮快捷键提示 | `hotkeyLabels.ts` + 各组件 tooltip | 1d |
| 缺失快捷键实现 | `useKeyboardShortcuts.ts` | 2d |

### 里程碑 M1.3 — 搜索系统（4 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 全流程搜索 | `flowSearch.ts` + `SearchOverlay.tsx` | 2d |
| 全局跨流程搜索 | `FlowListPanel.tsx` | 1d |
| NodePalette 搜索增强 | `NodePalette.tsx` | 1d |

### 里程碑 M1.4 — 变量系统（4 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 变量选择器增强 | `VariableSelector.tsx` | 1d |
| 变量引用着色 | `VariableInput.tsx` | 1d |
| 变量依赖关系图 | `VariableDependencyGraph.tsx` | 1d |
| 实时变量监视器 | `VariableTreeView.tsx` + `execution.ts` | 1d |

### 验证标准

- [ ] HTTP/Kafka/gRPC 等高频节点有 3+ 预设配置，一键填充
- [ ] 上游节点配置自动传递到下游（如 kafka_connect → kafka_produce 自动填充 brokers）
- [ ] 长配置面板按基础/高级折叠，状态持久化
- [ ] 同类型节点间可复制/粘贴配置
- [ ] `Ctrl+/` 打开快捷键面板，所有功能可搜索
- [ ] 所有常用操作有快捷键且按钮上有提示
- [ ] `Ctrl+Shift+F` 搜索节点名称 + 配置内容 + 变量引用
- [ ] 跨流程全文搜索（搜索词 ≥3 字符时）
- [ ] NodePalette 搜索跨所有分类
- [ ] 变量选择器显示预览值、类型、来源
- [ ] 配置输入框内 `{{var}}` 紫色高亮
- [ ] 变量依赖关系图可交互操作
- [ ] 执行中实时监视变量变化
