# UX 优化路线图 — P1：效率优化 (High Priority)

> 核心体验（P0）完成后，立即投入效率优化。目标：用户完成常见操作的效率提升 2 倍以上。
>
> ⚠️ 本文档已同步代码库实际状态。修正记录：同步了 30+ 节点类型、101 个配置面板、已有的 Variable 系列组件、面板快捷键（B/H/J 而非 1/2/3）、i18n 规范。

---

## 目录

1. [节点配置面板体验升级](#1-节点配置面板体验升级)
2. [快捷键系统重大改进](#2-快捷键系统重大改进)
3. [搜索与发现系统](#3-搜索与发现系统)
4. [变量系统可视化增强](#4-变量系统可视化增强)
5. [实现计划](#5-实现计划)

---

## 1. 节点配置面板体验升级

> **现状**：代码库已有 30+ 节点类型、101 个配置面板文件（`src/components/panels/`），覆盖 HTTP、WebSocket、数据库、gRPC、MCP、TCP/UDP、MQTT、AMQP、GraphQL、IMAP、Kafka、SMTP、SSE 等 15+ 协议。新增功能需对齐三重注册表（`NodeRegistry.ts` + `configPanelRegistry.ts` + `viewerRegistry.tsx`）。

### 1.1 预设配置模板

**目标**：高频节点类型提供 2-3 个"一键填充"的配置预设，减少重复劳动。

**优先覆盖节点**（按使用频率排序）：
- HTTP (`http`) — 最高频，预设 3 个
- Kafka (`kafka_produce`, `kafka_consume`)
- MQTT (`mqtt_publish`, `mqtt_subscribe`)
- gRPC (`grpc_call`)
- AMQP (`amqp_publish`)
- 数据库查询 (`db_query`)

其余节点类型根据后续数据追加 1-2 个预设。

#### 实现方案

```typescript
// 新文件: src/lib/nodePresets.ts
// ⚠️ 所有用户可见文本必须通过 getLL() 获取，不可硬编码中文
import { getLL } from '@/i18n/i18n-lazy';
import type { NodeType } from '@/types';

interface NodePreset {
  id: string;
  name: string;          // getLL() 获取
  description: string;   // getLL() 获取
  nodeType: NodeType;
  data: Record<string, unknown>;  // 完整的 node.data 内容
}

// HTTP 节点预设示例
const HTTP_PRESETS: NodePreset[] = [
  {
    id: 'http-get-users',
    name: 'GET /api/users',
    description: ll.nodes.preset.getUsers(), // '获取用户列表 - REST API 典型 GET 请求'
    nodeType: 'http',
    data: {
      method: 'get',
      url: 'https://jsonplaceholder.typicode.com/users',
      headers: [{ key: 'Accept', value: 'application/json' }],
      body_type: 'none',
      timeout_ms: 30000,
    },
  },
  // ... 更多预设
];
```

#### UI 呈现

在 `HttpConfig.tsx`（及优先覆盖的 Config 面板）顶部增加"预设"下拉：

```tsx
const ll = getLL();
// 每个 Config 面板增加预设选择器
<Select onValueChange={applyPreset}>
  <SelectTrigger className="w-full">
    <SelectValue placeholder={ll.nodes.preset.placeholder()} />
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

#### 注册方式

在 `configPanelRegistry.ts` 中新增可选 `presets` 字段，配置面板通过该字段自动读取预设列表。

### 1.2 智能默认值

**目标**：根据上下文（上游节点类型、已有配置）自动推断新节点的合理默认值。

#### 实现方案

```typescript
// 新文件: src/lib/nodeDefaults.ts
// 根据上游节点推断下游节点参数
// ⚠️ 必须覆盖实际存在的协议连接-操作对

import type { NodeType } from '@/types';

function inferDefaults(
  upstreamNode: Node | null,
  targetType: NodeType
): Record<string, unknown> {
  if (!upstreamNode) return {};

  switch (targetType) {
    // HTTP → ...
    case 'http':
      // 如果上游也是 http，继承 base_url
      if (upstreamNode.type === 'http') {
        const baseUrl = extractBaseUrl(upstreamNode.data.url as string);
        if (baseUrl) return { url: baseUrl };
      }
      break;

    // Kafka: kafka_connect → kafka_produce/kafka_consume 自动填充 brokers
    case 'kafka_produce':
    case 'kafka_consume':
      if (upstreamNode.type === 'kafka_connect') {
        return { brokers: upstreamNode.data.brokers };
      }
      break;

    // MQTT: mqtt_connect → mqtt_publish/mqtt_subscribe
    case 'mqtt_publish':
    case 'mqtt_subscribe':
    case 'mqtt_unsubscribe':
      if (upstreamNode.type === 'mqtt_connect') {
        return { host: upstreamNode.data.host, port: upstreamNode.data.port };
      }
      break;

    // 数据库: db_connect → db_query/db_begin
    case 'db_query':
    case 'db_begin':
      if (upstreamNode.type === 'db_connect') {
        return { connection_id: upstreamNode.data.connection_id };
      }
      break;

    // gRPC: grpc_connect → grpc_call
    case 'grpc_call':
      if (upstreamNode.type === 'grpc_connect') {
        return {
          proto_file: upstreamNode.data.proto_file,
          service_name: upstreamNode.data.service_name,
          connection_id: upstreamNode.data.connection_id,
        };
      }
      break;

    // MCP: mcp_connect → mcp_call_tool/mcp_read_resource/mcp_get_prompt
    case 'mcp_call_tool':
    case 'mcp_read_resource':
    case 'mcp_get_prompt':
      if (upstreamNode.type === 'mcp_connect') {
        return { server_url: upstreamNode.data.server_url };
      }
      break;

    // AMQP: amqp_connect → amqp_publish/amqp_consume
    case 'amqp_publish':
    case 'amqp_consume':
      if (upstreamNode.type === 'amqp_connect') {
        return { connection_id: upstreamNode.data.connection_id };
      }
      break;

    // TCP: tcp_connect → tcp_send
    case 'tcp_send':
      if (upstreamNode.type === 'tcp_connect') {
        return { host: upstreamNode.data.host, port: upstreamNode.data.port };
      }
      break;

    // SMTP: smtp_connect → smtp_send
    case 'smtp_send':
      if (upstreamNode.type === 'smtp_connect') {
        return { host: upstreamNode.data.host, port: upstreamNode.data.port };
      }
      break;

    // IMAP: imap_connect → imap_fetch/imap_search
    case 'imap_fetch':
    case 'imap_search':
      if (upstreamNode.type === 'imap_connect') {
        return { host: upstreamNode.data.host, port: upstreamNode.data.port };
      }
      break;

    // GraphQL: graphql_connect → graphql_query/graphql_subscribe
    case 'graphql_query':
    case 'graphql_subscribe':
      if (upstreamNode.type === 'graphql_connect') {
        return { url: upstreamNode.data.url };
      }
      break;

    // SSE: sse_connect → sse_listen
    case 'sse_listen':
      if (upstreamNode.type === 'sse_connect') {
        return { url: upstreamNode.data.url };
      }
      break;
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

**目标**：长配置表单（Kafka、gRPC、AMQP、IMAP 等）按逻辑分组折叠，默认收起"高级"区域。

> **已有基础设施**：`src/components/ui/CollapsiblePanel.tsx` — 面板级折叠（VSCode 风格，带头部拖拽调高）。
> 本文档提出的 `ConfigSection` 是表单内折叠区，作用于配置表单内部，两者用途不同。

#### 实现方案

```tsx
// 新建: src/components/ui/ConfigSection.tsx
// 使用 @radix-ui/react-collapsible
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@radix-ui/react-collapsible';
import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';

interface ConfigSectionProps {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
  badge?: string;      // 可选：标注"必填""高级"等
  storageKey?: string;  // localStorage 持久化 key，格式 `config-section-${nodeType}-${sectionKey}`
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
- **仅改造高频节点**：Kafka、gRPC、IMAP、AMQP、MQTT、HTTP 的长表单，非一次性覆盖全部 101 面板

### 1.4 配置复制/粘贴

**目标**：在同类型节点间快速复制配置。

#### 实现方案

```typescript
// 节点右键菜单新增项（文件: src/components/canvas/FlowCanvas.tsx 的右键菜单部分）
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

> **快捷键说明**：本文档及代码库统一使用 `Ctrl+` 表示修饰键。
> - **macOS** 上实际对应 `⌘`（Command 键），例如 `Ctrl+Z` 在 Mac 上是 `⌘Z`
> - **Windows/Linux** 上对应 `Ctrl` 键
> - 极少数 Mac 特有的快捷键会单独标注（如 `⌥` Option 键）
> - 实现中使用 `hotkeyLabel()` 工具函数（见 2.2 节）自动根据平台转换显示

### 2.1 快捷键面板

**目标**：`Ctrl+/` 或 `Cmd+/` 弹出完整快捷键列表，可搜索。

> ⚠️ 注意对齐：代码库现有快捷键使用字母键（`B`/`H`/`J`），而非文档旧版提案的数字键（`1`/`2`/`3`）。

#### 现有快捷键（快捷键面板需全部收录）

| 快捷键 | 功能 | 实现位置 |
|--------|------|---------|
| `Ctrl+Z` | 撤销 | ✅ `useKeyboardShortcuts.ts` |
| `Ctrl+Shift+Z` / `Ctrl+Y` | 重做 | ✅ |
| `Ctrl+S` | 保存当前流程 | ✅ |
| `Ctrl+B` | 切换左侧面板（流程列表） | ✅ |
| `Ctrl+H` | 切换历史面板 | ✅ |
| `Ctrl+J` | 切换配置面板 | ✅ |
| `Ctrl+=/+` | 放大 | ✅ |
| `Ctrl+-` | 缩小 | ✅ |
| `Ctrl+0` | 重置缩放 | ✅ |
| `Delete` / `Backspace` | 删除选中节点/边 | ✅ |
| `Escape` | 取消选择节点 | ✅ |

#### 新增快捷键（需要实现）

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+/` | 打开快捷键面板 |
| `Ctrl+D` | 复制选中节点 |
| `Ctrl+A` | 全选节点 |
| `Ctrl+Enter` | 执行流程 |
| `Ctrl+,` | 打开设置 |
| `F` | 适应画布到窗口（fitView） |
| `Ctrl+L` | 聚焦到节点列表 |

#### 实现方案

```tsx
// 新建: src/components/dialogs/ShortcutsDialog.tsx
// 由 useKeyboardShortcuts 注册 Ctrl+/ 触发
// ⚠️ 所有快捷键描述文本必须通过 getLL() 获取

const ll = getLL();
const SHORTCUT_GROUPS = [
  {
    group: ll.shortcuts.group.edit(), // '编辑'
    shortcuts: [
      { keys: ['Ctrl+Z'], description: ll.shortcuts.undo() },
      { keys: ['Ctrl+Shift+Z', 'Ctrl+Y'], description: ll.shortcuts.redo() },
      { keys: ['Ctrl+S'], description: ll.shortcuts.save() },
      { keys: ['Ctrl+D'], description: ll.shortcuts.duplicateNode() },
      { keys: ['Delete', 'Backspace'], description: ll.shortcuts.delete() },
      { keys: ['Ctrl+A'], description: ll.shortcuts.selectAll() },
    ],
  },
  {
    group: ll.shortcuts.group.execute(), // '执行'
    shortcuts: [
      { keys: ['Ctrl+Enter'], description: ll.shortcuts.executeFlow() },
      { keys: ['Ctrl+B'], description: ll.shortcuts.togglePanel('left') },
      { keys: ['Ctrl+H'], description: ll.shortcuts.togglePanel('history') },
      { keys: ['Ctrl+J'], description: ll.shortcuts.togglePanel('config') },
      { keys: ['Ctrl+,'] , description: ll.shortcuts.openSettings() },
    ],
  },
  {
    group: ll.shortcuts.group.canvas(), // '画布'
    shortcuts: [
      { keys: ['Space + 拖拽'], description: ll.shortcuts.panCanvas() },
      { keys: ['Ctrl+滚轮'], description: ll.shortcuts.zoomCanvas() },
      { keys: ['Ctrl+0'], description: ll.shortcuts.resetZoom() },
      { keys: ['Ctrl + - / ='], description: ll.shortcuts.zoomInOut() },
      { keys: ['F'], description: ll.shortcuts.fitView() },
      { keys: ['Ctrl+L'], description: ll.shortcuts.focusNodeList() },
    ],
  },
];
```

#### 关联文件

| 文件 | 修改内容 |
|------|---------|
| `src/components/dialogs/ShortcutsDialog.tsx` **(新建)** | 快捷键面板 |
| `src/hooks/useKeyboardShortcuts.ts` | 注册 `Ctrl+/` 触发面板 |
| `src/stores/ui.ts` | 新增 `showShortcutsDialog` 状态 |
| `src/i18n/zh-CN/shortcuts.ts` **(新建模块)** | 中文快捷键文本 |
| `src/i18n/en-US/shortcuts.ts` **(新建模块)** | 英文快捷键文本 |
| `src/i18n/i18n-types.ts` | 同步新增类型 |

### 2.2 快捷键可视化

**目标**：按钮 tooltip 和菜单项中显示快捷键提示。

#### 实现方案

```typescript
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
| `Ctrl+S` (保存) | ✅ 已有 | `useKeyboardShortcuts.ts` |
| `Ctrl+B` (左侧面板) | ✅ 已有 | `useKeyboardShortcuts.ts` |
| `Ctrl+H` (历史面板) | ✅ 已有 | `useKeyboardShortcuts.ts` |
| `Ctrl+J` (配置面板) | ✅ 已有 | `useKeyboardShortcuts.ts` |
| `Ctrl+=/-/0` (缩放) | ✅ 已有 | `useKeyboardShortcuts.ts` |
| `Ctrl+/` (快捷键面板) | ❌ 缺失 | 需新增 |
| `Ctrl+D` (复制节点) | ❌ 缺失 | 需新增 |
| `Ctrl+A` (全选) | ❌ 缺失 | 需新增 |
| `Ctrl+Enter` (执行) | ❌ 缺失 | 需新增 + `flowStore.executeFlow()` |
| `Ctrl+,` (设置) | ❌ 缺失 | 需新增 |
| `F` (适应画布) | ❌ 缺失 | 需新增 + `reactFlowInstance.fitView()` |
| `Ctrl+L` (聚焦节点列表) | ❌ 缺失 | 需新增 |

---

## 3. 搜索与发现系统

### 3.1 全流程搜索

**目标**：`Ctrl+Shift+F` 搜索当前流程所有节点的名称、配置字段、变量引用。

#### 实现方案

```typescript
// 新建: src/lib/flowSearch.ts
// ⚠️ 搜索结果中的节点名称、匹配值等需标记 i18n key，但不直接翻译（搜索是原始文本匹配）

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
// 搜索面板: SearchOverlay.tsx (新建)
// 类似 VS Code 的搜索界面
// 结果按节点分组，点击跳转到对应节点并高亮匹配字段

<CommandDialog open={isOpen} onOpenChange={setOpen}>
  <CommandInput placeholder={ll.search.placeholder()} />
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

> **现状**：`FlowListPanel.tsx` 已有搜索框，但仅搜索流程名称（`flow.name`），不搜索流程内容。

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

**现状**（`src/components/canvas/NodePalette.tsx`）：
- **搜索范围**：仅在当前超级分类（network/services/messaging/tools/other）下搜索 ❌
- **搜索字段**：仅匹配 `item.name` 和 `item.type`，不搜索别名 ❌
- **无模糊匹配**：`hhtp` 无法匹配 `http` ❌
- **无关键词别名**：`rest` → 无法找到 `http` ❌

**改进**：

- 搜索时合并 **所有** 超级分类的结果，不再限制当前 Tab
- 支持模糊匹配（编辑距离 ≤ 2，使用 `fuzzysort` 或类似库）
- 支持关键词别名映射表（`rest` → `http`，`pubsub` → `mqtt/kafka`）
- 最近/最常用节点置顶（基于 `localStorage` 计数）

```typescript
// 关键词别名映射
const SEARCH_ALIASES: Record<string, string[]> = {
  rest: ['http'],
  pubsub: ['mqtt', 'kafka'],
  stream: ['kafka', 'sse', 'ws_connect'],
  queue: ['mqtt', 'amqp', 'kafka'],
  trigger: ['mqtt', 'kafka'],
  sql: ['db_query'],
  nosql: ['db_query'],
  rpc: ['grpc', 'mcp'],
  tool: ['mcp', 'script'],
  email: ['smtp', 'imap'],
};
```

---

## 4. 变量系统可视化增强

> **现状**：变量相关组件已全部存在，且功能比文档旧版描述的基线更强：
> - `VariableInput.tsx` — 带变量选择器的输入框，支持光标位置插入、受控/非受控模式
> - `VariableSelector.tsx` — Popover 弹出变量列表，支持搜索过滤、节点分组
> - `VariableTreeView.tsx` — 树形变量查看器，支持复制、编辑、展开折叠
> - `VariablePanel.tsx` — 完整变量面板，支持搜索、类型徽标、按节点分组折叠
>
> 以下为增量增强方案。

### 4.1 变量选择器升级

**现状**：`VariableSelector.tsx` 显示变量名和描述，按节点分组，支持搜索。

**增强目标**：
- **值预览**：变量名旁显示当前值（如果有初始值或上游产生值）
- **类型标注**：显示值的类型（`string | number | object | array`）
- **来源追踪**：标注变量来自哪个节点（`body.user.name ← HTTP-1 响应`）

#### 实现

```typescript
// 增强 VariableSelector.tsx

// Variable 类型已有 sourceNodeUuid 字段
// 新增以下显示增强：
interface VariableEntry {
  path: string;         // body.user.name
  displayName: string;  // user.name
  value: unknown;       // 当前值（预览）— 新增
  type: string;         // 类型标注 — 新增
  sourceNodeName?: string;  // 来源节点名称 — 新增（从 nodeLabels 推断）
}
```

### 4.2 变量引用着色

**目标**：在配置输入框中，`{{variable}}` 引用以特殊颜色高亮显示，与普通文本区分。

**现状**：`VariableInput.tsx` 是纯文本输入框，无高亮渲染。

#### 实现方案

```tsx
// 增强 VariableInput.tsx
// 在输入框下方或内部展示 {{var}} 的着色预览

// 方案：在输入框下方添加一个高亮预览区域
const highlightedPreview = value.replace(
  /\{\{([^}]+)\}\}/g,
  (match, path) => {
    const isValid = variablePaths.has(path);
    return `<span class="${cn(
      'text-violet-600 dark:text-violet-400 font-medium',
      !isValid && 'text-red-500 dark:text-red-400 line-through'
    )}">${match}</span>`;
  }
);
```

### 4.3 变量依赖关系图

**目标**：显示"哪些节点使用了某变量"和"某节点使用了哪些变量"。

#### 实现

```tsx
// 新建: src/components/execution/VariableDependencyGraph.tsx
// 在 VariablePanel 中增加"变量关系"Tab
// ⚠️ 使用 getLL() 获取 Tab 标题等文本

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

**现状**：`VariableTreeView.tsx` 仅支持静态变量查看（复制/编辑/展开折叠），不支持实时跟踪。

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

### 里程碑 M1.1 — 配置体验（调整：7 天，原估算低估了节点数量）

| 任务 | 文件 | 估时 | 备注 |
|------|------|------|------|
| 预设模板系统 | `nodePresets.ts` + 优先覆盖 6 种高频节点的 Config 面板 | 3d | 原 2d，因 30+ 节点类型需优先排序 |
| 智能默认值 | `nodeDefaults.ts` + `FlowCanvas.tsx` | 2d | 需覆盖 12 个协议连接-操作对 |
| 配置折叠分组 | `ConfigSection.tsx` + 优先改造 6 个长表单面板 | 2d | 原 1.5d |
| 配置复制/粘贴 | 嵌入 `FlowCanvas.tsx` 节点右键菜单 | 0.5d | — |
| i18n 同步 | 在对应模块翻译文件中新增 key | 0.5d | 贯穿所有任务 |

### 里程碑 M1.2 — 快捷键系统（调整：5 天）

| 任务 | 文件 | 估时 | 备注 |
|------|------|------|------|
| 快捷键面板 | `ShortcutsDialog.tsx` + `ui.ts` 状态 + `useKeyboardShortcuts.ts` 注册 `Ctrl+/` | 1.5d | 含 i18n 新模块 |
| 所有按钮快捷键提示 | `hotkeyLabels.ts` + CanvasToolbar 等组件 tooltip 追加 | 1d | — |
| 缺失快捷键实现 | `useKeyboardShortcuts.ts` + React Flow API (`fitView`) + Store Actions | 2.5d | 含 `Ctrl+Enter` 触发执行、`F` 触发 fitView |

### 里程碑 M1.3 — 搜索系统（4 天）

| 任务 | 文件 | 估时 | 备注 |
|------|------|------|------|
| 全流程搜索 | `flowSearch.ts` + `SearchOverlay.tsx` | 2d | — |
| 全局跨流程搜索 | 增强 `FlowListPanel.tsx` 搜索模式 | 1d | 现仅搜索名称，需扩展为可选内容搜索 |
| NodePalette 搜索增强 | `NodePalette.tsx` | 1d | 跨分类搜索 + 模糊匹配 + 别名 + 高频置顶 |

### 里程碑 M1.4 — 变量系统（4 天）

| 任务 | 文件 | 估时 | 备注 |
|------|------|------|------|
| 变量选择器增强 | `VariableSelector.tsx` | 1d | 追加值预览、类型标注、来源追踪 |
| 变量引用着色 | `VariableInput.tsx` | 1d | 新增高亮预览 |
| 变量依赖关系图 | `VariableDependencyGraph.tsx` (新建) | 1d | — |
| 实时变量监视器 | `VariableTreeView.tsx` + `execution.ts` | 1d | 监听 `ExecutionEvent` 通道 |

### 验证标准

- [ ] HTTP/Kafka/MQTT/gRPC/AMQP/DB 共 6 种高频节点有 3+ 预设配置，一键填充
- [ ] 上游节点配置自动传递到下游（12 种协议连接-操作对）
- [ ] 长配置面板按基础/高级折叠，状态持久化（优先 6 种）
- [ ] 同类型节点间可复制/粘贴配置
- [ ] `Ctrl+/` 打开快捷键面板，所有功能可搜索，涵盖已有 + 新增快捷键
- [ ] 所有常用操作有快捷键且按钮上有提示
- [ ] `Ctrl+Shift+F` 搜索节点名称 + 配置内容 + 变量引用
- [ ] 跨流程全文搜索（搜索词 ≥3 字符时）
- [ ] NodePalette 搜索跨所有分类、支持模糊匹配和别名
- [ ] 变量选择器显示预览值、类型、来源
- [ ] 配置输入框内 `{{var}}` 紫色高亮
- [ ] 变量依赖关系图可交互操作
- [ ] 执行中实时监视变量变化
