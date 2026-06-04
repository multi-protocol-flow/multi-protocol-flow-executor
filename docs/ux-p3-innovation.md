# UX 优化路线图 — P3：创新差异化 (Future Priority)

> P0-P2 完成后，产品已经达到专业工具水准。P3 聚焦于 AI 能力、性能极致化、生态集成，形成竞争壁垒。

---

## 目录

1. [AI 辅助功能](#1-ai-辅助功能)
2. [性能极致优化](#2-性能极致优化)
3. [外部生态集成](#3-外部生态集成)
4. [开发者体验](#4-开发者体验)
5. [实现计划](#5-实现计划)

---

## 1. AI 辅助功能

### 1.1 AI 智能流程生成

**目标**：用户通过自然语言描述，AI 自动生成完整流程。

#### 核心交互

```
用户输入：
"每天早上 8 点检测 example.com 的健康状态，如果返回 200 则记录到日志，
如果失败则发送邮件告警到 admin@example.com"

AI 输出：
1. HTTP 节点 (GET https://example.com/health)
2. Conditional 节点 (判断 status === 200)
   ├─ true → Script 节点 (记录日志)
   └─ false → SMTP 节点 (发送告警邮件)
```

#### 实现方案

```typescript
// 新建: src/lib/ai/flowGenerator.ts
interface AIGenerationRequest {
  prompt: string;                 // 自然语言描述
  preferredProtocols?: string[];  // 倾向使用的协议
  existingFlow?: Flow;            // 可选：在现有流程上扩展
}

interface AIGenerationResponse {
  description: string;   // AI 对生成流程的说明
  flow: Flow;            // 生成的完整 Flow
  confidence: 'high' | 'medium' | 'low';
  alternatives?: Flow[]; // 备选方案
}
```

#### 技术选型

| 方案 | 优点 | 缺点 |
|------|------|------|
| 本地 LLM (llama.cpp / Ollama) | 离线可用、无 API 费用 | 需要本地计算资源 |
| 云端 API (OpenAI/Claude) | 质量更高 | 需要网络、有费用 |
| 混合方案 | 灵活 | 复杂度高 |

**推荐初始方案**：通过 Tauri 的 shell 插件调用本地 Ollama（用户自行安装），或直接通过 HTTP 调用 OpenAI 兼容 API（用户配置 endpoint + key）。

#### UI 入口

- 工具栏增加 AI ✨ 按钮
- 画布右键菜单 → "AI 辅助生成"
- 节点配置面板 → "AI 帮我填充"

### 1.2 智能节点建议

**目标**：在流程编辑过程中，AI 预测下一步最可能需要的节点。

#### 实现方案

```typescript
// 新建: src/lib/ai/nodeSuggestion.ts
interface SuggestionContext {
  currentNodes: Node[];
  connections: Connection[];
  selectedNodeId: string | null;
}

interface NodeSuggestion {
  type: NodeType;
  reason: string;    // 为什么推荐这个节点
  confidence: number; // 0-1
}

function suggestNextNodes(context: SuggestionContext): NodeSuggestion[] {
  const suggestions: NodeSuggestion[] = [];

  const selectedNode = context.currentNodes.find(n => n.id === context.selectedNodeId);

  if (!selectedNode) return [];

  switch (selectedNode.type) {
    case 'http':
      suggestions.push(
        { type: 'conditional', reason: '根据 HTTP 响应做分支判断', confidence: 0.9 },
        { type: 'assertion', reason: '验证响应数据是否符合预期', confidence: 0.7 },
        { type: 'script', reason: '处理/提取响应数据', confidence: 0.6 },
      );
      break;

    case 'kafka_consume':
      suggestions.push(
        { type: 'kafka_produce', reason: '处理并转发到另一个 Topic', confidence: 0.8 },
        { type: 'db_query', reason: '将消息写入数据库', confidence: 0.7 },
        { type: 'script', reason: '转换消息格式', confidence: 0.6 },
      );
      break;

    // 其他节点类型...
  }

  return suggestions.sort((a, b) => b.confidence - a.confidence);
}
```

#### 渲染方式

- 在 NodePalette 中置顶显示「智能推荐」区域
- 在节点输出端口旁显示推荐气泡
- 在 CanvasToolbar 中增加推荐面板按钮

### 1.3 智能配置自动填充

**目标**：AI 根据 URL/连接字符串/语义自动填充节点配置。

#### 使用场景

| 输入 | AI 推断 |
|------|---------|
| 粘贴 `jdbc:postgresql://localhost:5432/mydb` 到某个字段 | 自动选择 PostgreSQL、填充 host/port/dbname，创建 db_connect 节点 |
| 输入 `https://api.github.com/repos/owner/repo` | 推断 GET 请求、自动添加 Accept header |
| 输入 `kafka://broker1:9092,broker2:9092/topic` | 创建 kafka_connect 节点 + kafka_consume 节点 |
| 输入 `imaps://imap.gmail.com:993` | 创建 imap_connect 节点，自动设置 SSL/TLS |

#### 实现方案

```typescript
// 新建: src/lib/ai/smartDetect.ts
interface DetectedConfig {
  nodeType: NodeType;
  config: Record<string, unknown>;
  confidence: number;
  detectedFrom: string; // 用户输入的原始内容
}

// 基于规则的快速推断（无 AI 也可运行）
const DETECTORS: Array<{
  pattern: RegExp;
  detect: (match: RegExpMatchArray) => DetectedConfig;
}> = [
  {
    pattern: /^jdbc:(postgresql|mysql|sqlite):\/\/([^:]+):(\d+)\/(.+)$/i,
    detect: ([, type, host, port, db]) => ({
      nodeType: 'db_connect',
      config: { db_type: type === 'postgresql' ? 'postgresql' : 'mysql', host, port: parseInt(port), database: db },
      confidence: 0.95,
      detectedFrom: match[0],
    }),
  },
  // ... 更多检测规则
];
```

### 1.4 AI 调试助手

**目标**：执行失败时，AI 分析日志并提供根因分析和修复建议。

#### 工作流程

1. 流程执行失败 → 收集错误信息、节点报告、控制台日志
2. 将执行上下文打包发送给 AI
3. AI 返回分析报告：根因分析 + 修复步骤 + 代码示例（如果是脚本节点）
4. 显示在"执行报告"->"AI 分析"Tab 中

---

## 2. 性能极致优化

### 2.1 画布虚拟化

**目标**：50+ 节点场景下保持 60fps 交互流畅度。

#### 现状

React Flow 默认渲染所有节点，大量节点时拖拽和缩放会出现卡顿。

#### 优化方案

```tsx
// FlowCanvas.tsx — 启用 React Flow 的内置虚拟化
<ReactFlow
  onlyRenderVisibleNodes={true}  // 仅渲染视口内的节点
  nodes={nodes}
  // ...
>
```

#### 进一步优化

| 优化项 | 说明 | 预期效果 |
|--------|------|---------|
| `onlyRenderVisibleNodes` | React Flow 内置 | 只渲染可视区域节点 |
| `nodesDraggable` 仅当前编辑模式启用 | 减少事件监听 | 拖拽更流畅 |
| 使用 `React.memo` 包裹节点组件 | 减少不必要的重渲染 | 节点数 100+ 时有效 |
| 节点 data 使用浅引用 | 配合 memo 生效 | 减少重渲染判断 |
| 大型流程懒加载 | 首次加载 > 50 节点时分批渲染 | 首屏加载速度 |

#### 节点组件 Memo 包裹

```tsx
// 对所有节点组件包裹 React.memo
export const HttpNode = React.memo(function HttpNode({ id, data, selected }: NodeProps) {
  // ...
});
```

### 2.2 流程列表虚拟化

**目标**：500+ 流程时不卡顿。

#### 实现

使用 `react-window` 或 `@tanstack/react-virtual`：

```tsx
import { useVirtualizer } from '@tanstack/react-virtual';

// FlowListPanel.tsx — 虚拟滚动
const parentRef = useRef<HTMLDivElement>(null);
const virtualizer = useVirtualizer({
  count: flows.length + folders.length,
  getScrollElement: () => parentRef.current,
  estimateSize: () => 56,  // 每项高度 56px
});

return (
  <div ref={parentRef} className="overflow-y-auto">
    <div style={{ height: `${virtualizer.getTotalSize()}px` }}>
      {virtualizer.getVirtualItems().map((virtualItem) => (
        <div
          key={virtualItem.key}
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: `${virtualItem.size}px`,
            transform: `translateY(${virtualItem.start}px)`,
          }}
        >
          {renderItem(flows[virtualItem.index])}
        </div>
      ))}
    </div>
  </div>
);
```

### 2.3 配置面板懒加载

**目标**：每个节点配置面板按需加载，减少首屏 JS 体积。

#### 实现

```tsx
// NodeConfigPanel.tsx — React.lazy 按协议分块加载
const HttpConfig = React.lazy(() => import('@/components/panels/HttpConfig'));
const KafkaProduceConfig = React.lazy(() => import('@/components/panels/KafkaProduceConfig'));
// ... 每个 Config 面板单独 lazy

function NodeConfigPanel({ nodeType, ...props }) {
  // 按 nodeType 动态选取组件
  const ConfigComponent = CONFIG_MAP[nodeType];

  return (
    <Suspense fallback={<ConfigSkeleton />}>
      <ConfigComponent {...props} />
    </Suspense>
  );
}
```

#### 收益

- 首屏 JS 体积减少 ~60%（100+ 面板不会一次性加载）
- 每个协议配置面板仅在用户首次编辑该类型节点时加载
- 配合 Vite 的代码分割（`manualChunks` 按协议分组）

### 2.4 脚本身份隔离

**目标**：`flow-engine-script` 的 QuickJS 执行不阻塞 UI。

#### 现状

QuickJS 在 Rust 后端执行，但复杂脚本可能消耗大量 CPU 时间，阻塞 Tauri IPC。

#### 优化方案

- 在 Rust 后端使用 `tokio::task::spawn_blocking` 将脚本执行隔离到独立线程池
- 设置超时（默认 30s，可配置），超时后自动终止脚本
- 防止无限循环：注入执行步数计数器

```rust
// Rust 端
let result = tokio::task::spawn_blocking(move || {
    // QuickJS 执行
    let mut ctx = QuickJSContext::new();
    ctx.set_timeout(30_000); // 30s 超时
    ctx.set_step_limit(100_000); // 最大执行步数
    ctx.eval(&script)
}).await?;
```

---

## 3. 外部生态集成

### 3.1 流程导出为外部命令

**目标**：一键将流程片段导出为 curl/HTTPie/gRPCurl 等价命令行。

```typescript
// 新建: src/lib/exporter/commandExporter.ts
interface CommandExportOptions {
  target: 'curl' | 'httpie' | 'grpcurl' | 'kcat' | 'mqttx';
  includeAuth?: boolean;
  format?: 'pretty' | 'compact';
}

function exportToCurl(httpNode: Node): string {
  const config = httpNode.data as HttpConfig;
  const parts = ['curl'];

  if (config.method?.toLowerCase() !== 'get') {
    parts.push(`-X ${config.method.toUpperCase()}`);
  }

  if (config.headers) {
    for (const h of config.headers) {
      parts.push(`-H '${h.key}: ${h.value}'`);
    }
  }

  if (config.body_content && config.body_type !== 'none') {
    parts.push(`-d '${config.body_content}'`);
  }

  parts.push(`'${config.url}'`);

  return parts.join(' \\\n  ');
}
```

#### UI 入口

- 节点右键菜单 → "导出为 Curl/gRPCurl"
- 右键菜单 → "复制为 cURL 命令"
- 执行报告 → "查看等价命令"

### 3.2 流程分享与导入

**目标**：导出为可分享的格式，支持从 URL/剪贴板导入。

#### 实现

```typescript
// 新建: src/services/sharing/flowShareService.ts

interface SharePayload {
  version: number;
  name: string;
  description: string;
  nodes: Node[];
  connections: Connection[];
  exportedAt: string;
  appVersion: string;
}

// 导出为压缩 JSON（URL 安全 base64）
async function exportShareableFlow(flow: Flow): Promise<string> {
  const payload: SharePayload = {
    version: 2,
    name: flow.name,
    description: '',
    nodes: flow.nodes,
    connections: flow.connections,
    exportedAt: new Date().toISOString(),
    appVersion: await getAppVersion(),
  };

  const json = JSON.stringify(payload);
  // 使用 pako 或内置 CompressionStream 压缩
  const compressed = await compressData(json);
  return base64UrlEncode(compressed);
}

// 分享菜单
<ShareDialog>
  <Tabs>
    <Tab label="复制链接">
      <Button onClick={copyShareLink}>复制分享链接</Button>
      <p className="text-xs text-muted-foreground">链接包含完整流程数据，可直接在 MPE 中打开</p>
    </Tab>
    <Tab label="导出文件">
      <Button onClick={exportFlowFile}>导出 .mpf 文件</Button>
    </Tab>
    <Tab label="Postman 格式">
      <Button onClick={exportPostman}>导出 Postman Collection v2.1</Button>
    </Tab>
  </Tabs>
</ShareDialog>
```

### 3.3 与主流工具兼容

**目标**：支持导入 Postman Collection、Insomnia、OpenAPI 规范。

```typescript
// 新建: src/services/import/postmanImporter.ts
async function importPostmanCollection(file: File): Promise<Flow> {
  const json = await file.text();
  const collection = JSON.parse(json);

  const flow: Flow = {
    uuid: crypto.randomUUID(),
    name: collection.info.name,
    nodes: [],
    connections: [],
  };

  // 将每个 Postman Request 转换为 HTTP 节点 + 变量
  for (const item of collection.item) {
    const node = convertPostmanItemToNode(item);
    flow.nodes.push(node);
  }

  // 按顺序连接节点
  for (let i = 0; i < flow.nodes.length - 1; i++) {
    flow.connections.push({
      id: `auto-${i}`,
      source_node_uuid: flow.nodes[i].uuid,
      target_node_uuid: flow.nodes[i + 1].uuid,
      source_port_id: 'out',
      target_port_id: 'in',
    });
  }

  return flow;
}
```

---

## 4. 开发者体验

### 4.1 插件/节点 SDK

**目标**：社区可以开发第三方节点类型。

#### 概念设计

```typescript
// 前端插件注册接口 (构想)
interface PluginNodeDefinition {
  typeId: string;
  displayName: string;
  category: string;
  icon: string;
  color: string;
  defaultConfig: Record<string, unknown>;
  configPanel: React.ComponentType<ConfigPanelProps>;
  nodeComponent: React.ComponentType<NodeProps>;
  viewerComponent?: React.ComponentType<ViewerProps>;
  schema: ZodSchema;
}

// 后端插件注册 (Rust trait - 已有 NodePlugin trait)
// 使第三方 crate 可以实现 PluginNode trait 并注册到 registry
```

### 4.2 诊断与调试工具

**目标**：高级用户和开发者可通过内置工具诊断流程问题。

```typescript
// 诊断面板
interface DiagnosticsInfo {
  flowValidation: ValidationResult[];
  nodeConnections: ConnectionGraph;
  variableResolution: VariableTrace[];
  performanceMetrics: {
    renderTime: number;
    nodeCount: number;
    fps: number;
  };
}
```

- `Ctrl+Shift+I` 打开诊断面板
- 显示流程验证详情、连接状态、变量引用链
- 性能监控（FPS、节点数、渲染耗时）

---

## 5. 实现计划

### 里程碑 M3.1 — AI 基础（2 周）

| 任务 | 文件 | 估时 |
|------|------|------|
| AI 流程生成引擎 | `ai/flowGenerator.ts` | 3d |
| 智能节点建议 | `ai/nodeSuggestion.ts` + UI 集成 | 2d |
| 智能配置检测 | `ai/smartDetect.ts` + 各 Config 面板 | 2d |
| AI 调试助手 | `ai/debugAnalyzer.ts` + 报告视图 | 3d |

### 里程碑 M3.2 — 性能优化（1 周）

| 任务 | 文件 | 估时 |
|------|------|------|
| 画布虚拟化 | `FlowCanvas.tsx` (onlyRenderVisibleNodes) | 0.5d |
| 节点组件 Memo | 所有节点组件 | 1d |
| 流程列表虚拟化 | `FlowListPanel.tsx` | 1d |
| 配置面板懒加载 | `NodeConfigPanel.tsx` + Vite 配置 | 1d |
| 脚本执行隔离 | Rust 端 `spawn_blocking` | 0.5d |

### 里程碑 M3.3 — 生态集成（1 周）

| 任务 | 文件 | 估时 |
|------|------|------|
| 命令导出 | `commandExporter.ts` + UI 入口 | 1.5d |
| 流程分享 | `flowShareService.ts` + `ShareDialog.tsx` | 2d |
| Postman/OpenAPI 导入 | `postmanImporter.ts` + 导入对话框 | 1.5d |

### 里程碑 M3.4 — 开发者体验（持续）

| 任务 | 文件 | 估时 |
|------|------|------|
| 插件 SDK 设计 | 架构文档 + 示例 | 3d |
| 诊断面板 | `DevDiagnostics.tsx` | 2d |

### 验证标准

- [ ] 自然语言描述可生成正确流程（10 种常见场景）
- [ ] 节点推荐在 10 种常见节点类型后工作
- [ ] URL/连接字符串自动检测并填充配置
- [ ] 执行失败时 AI 分析根因并给出修复建议
- [ ] 50+ 节点画布拖拽仍然流畅（≥ 50fps）
- [ ] 500+ 流程列表滚动流畅无延迟
- [ ] 首屏 JS 体积减少 ≥ 50%
- [ ] 复杂脚本执行不阻塞 UI
- [ ] 流程可导出为 curl/gRPCurl/kcat 命令
- [ ] 分享链接可导入回 MPE
- [ ] Postman Collection 导入成功率 ≥ 90%
- [ ] 诊断面板可显示完整流程验证和性能指标
