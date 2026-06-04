# UX 优化路线图 — P0：核心体验 (Critical)

> 优先级最高，直接影响用户能否顺利上手并完成基本任务。未完成 P0 前不应推进 P1。

---

## 目录

1. [新手引导系统](#1-新手引导系统)
2. [执行可视化增强](#2-执行可视化增强)
3. [实现计划](#3-实现计划)

---

## 1. 新手引导系统

### 1.1 首次启动向导

**目标**：新用户在 3 分钟内完成"创建 → 配置 → 执行"的完整闭环。

**实现方案**：

#### 欢迎遮罩层 (`App.tsx` + 新建组件 `WelcomeWizard.tsx`)

```tsx
// 新文件: src/components/welcome/WelcomeWizard.tsx
// 使用 localStorage 标记是否首次启动，非首次直接跳过
```

- 3 步引导流程：
  - **Step 1**: "欢迎使用 MPE" → 简要介绍什么是多协议流程编排
  - **Step 2**: 高亮左侧面板「新建流程」按钮 + 画布区域 → "点击新建流程，开始你的第一个编排"
  - **Step 3**: 引导拖拽一个 HTTP 节点到画布 → 配置 URL → 执行 → 查看结果
- 实现方式：使用 Portal 渲染遮罩层，通过 `data-guide` 属性定位目标元素
- 跳过条件：`localStorage.setItem('mpe-welcome-shown', 'true')`

#### 关联文件

| 文件 | 修改内容 |
|------|---------|
| `src/App.tsx` | 在 `MainLayout` 外包裹 `WelcomeWizard` 组件 |
| `src/` 新建 `components/welcome/WelcomeWizard.tsx` | 引导流程主体 |
| `src/` 新建 `components/welcome/GuideHighlighter.tsx` | 元素高亮遮罩工具组件 |
| `src/stores/ui.ts` | 新增 `welcomeStep` 状态 |
| `src/i18n/zh-CN/welcome.ts` + `en-US/welcome.ts` | 引导文案 |

### 1.2 上下文帮助系统

**目标**：每个节点类型可获取协议说明、参数含义、最佳实践、常见错误排错。

#### 实现方案

- 在 `NodeConfigPanel.tsx` 标题栏增加 `?` 图标按钮
- 点击后弹出 `HelpDrawer` 从右侧滑入，包含：
  - 协议概述（1-2 句话）
  - 参数表格（字段名 → 类型 → 必填 → 说明 → 示例值）
  - 使用场景示例（2-3 个典型流程片段截图或文字描述）
  - 常见错误（FAQ 格式）

#### 数据结构

```typescript
// 新文件: src/lib/nodeHelpRegistry.ts
interface NodeHelpContent {
  typeId: string;
  summary: string;           // 一句话概述
  params: Array<{
    name: string;
    type: string;
    required: boolean;
    description: string;
    example: string;
  }>;
  useCases: Array<{
    title: string;
    description: string;
    flowExample?: string;    // 示例流程JSON
  }>;
  faq: Array<{
    question: string;
    answer: string;
  }>;
}
```

#### 关联文件

| 文件 | 修改内容 |
|------|---------|
| `src/lib/` 新建 `nodeHelpRegistry.ts` | 注册所有节点的帮助内容 |
| `src/components/panels/NodeConfigPanel.tsx` | 标题栏增加帮助按钮 |
| `src/components/panels/` 新建 `HelpDrawer.tsx` | 帮助面板组件 |
| `src/context/` 无需 | 使用 `getLL()` 翻译 |

### 1.3 模板库增强

**目标**：每个协议至少有一个完整的、可立即运行的示例流程。

#### 现状

`src/templates/builtinFlows.ts` 仅有 2 个模板（HTTP DEMO、WS DEMO）。

#### 改造方案

```typescript
// 修改: src/templates/builtinFlows.ts
// 结构改为分类模板数组
export interface FlowTemplate {
  id: string;
  name: string;           // 模板名称
  description: string;    // 一句话描述
  category: TemplateCategory;
  icon: string;           // lucide icon name
  tags: string[];
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  flow: Flow;             // 完整的 Flow 结构
}

export type TemplateCategory =
  | 'http' | 'websocket' | 'database' | 'grpc' | 'mqtt'
  | 'kafka' | 'amqp' | 'graphql' | 'imap' | 'smtp'
  | 'sse' | 'tcp' | 'udp' | 'mcp' | 'script' | 'combination';
```

- 新增 15+ 模板，每个协议至少 1 个
- 新增 3 个组合模板（HTTP → Conditional → Email、Kafka → Script → DB 等）
- 新增 `NewFlowDialog.tsx` 中的"从模板创建"Tab

#### 关联文件

| 文件 | 修改内容 |
|------|---------|
| `src/templates/builtinFlows.ts` | 扩展为 15+ 模板并分类 |
| `src/components/dialogs/NewFlowDialog.tsx` | 增加"从模板创建"Tab |
| `src/i18n/` | 模板相关翻译 |

### 1.4 空画布智能建议

**目标**：空画布不再是冷冰冰的"还没有流程"，而是引导用户快速开始。

#### 现状

```tsx
// FlowCanvas.tsx - 当前空状态
<div className="text-6xl mb-4">📋</div>
<h2>{ll.flow.no_flow_title()}</h2>
<p>{ll.flow.no_flow_desc()}</p>
```

#### 改造方案

```tsx
// 空状态改为交互式引导面板
<div className="...">
  <div className="flex gap-8">
    {/* 三个快速入口卡片 */}
    <ActionCard
      icon={Plus}
      title="新建空白流程"
      description="从零开始搭建你的流程"
      onClick={openNewFlowDialog}
    />
    <ActionCard
      icon={Copy}
      title="从模板创建"
      description="选择预置模板快速开始"
      onClick={showTemplates}
    />
    <ActionCard
      icon={Upload}
      title="导入流程文件"
      description="拖入或选择 .mpf 文件"
      onDrop={handleImport}
    />
  </div>

  {/* 热门模板快捷入口 */}
  <div className="...">
    <h3>快速开始</h3>
    <div className="flex gap-4">
      {POPULAR_TEMPLATES.map(t => (
        <TemplateChip key={t.id} {...t} />
      ))}
    </div>
  </div>

  {/* 快捷键提示 */}
  <div className="...">
    <kbd>右键</kbd> 画布打开节点选择器 ·
    <kbd>Ctrl+K</kbd> 全局搜索
  </div>
</div>
```

---

## 2. 执行可视化增强

### 2.1 实时节点高亮动画

**目标**：执行过程中，正在运行的节点在画布上清晰可见，无需切换到控制台查看进度。

#### 现状

- 调试模式有 `debug-breathing-sky` CSS 类
- 普通执行模式无任何画布反馈

#### 实现方案

**CSS 动画**（`src/index.css` 或全局样式）：

```css
@keyframes node-executing-pulse {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.5);
    border-color: rgb(59, 130, 246);
  }
  50% {
    box-shadow: 0 0 0 8px rgba(59, 130, 246, 0);
    border-color: rgb(37, 99, 235);
  }
}

.node-executing {
  animation: node-executing-pulse 1s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  z-index: 10;
}

.node-succeeded {
  border-color: rgb(34, 197, 94) !important;
  background: rgba(34, 197, 94, 0.05);
}

.node-failed {
  border-color: rgb(239, 68, 68) !important;
  background: rgba(239, 68, 68, 0.05);
}
```

**Store 对接**（`src/stores/execution.ts`）：

```typescript
// 新增状态
executingNodeUuid: string | null;     // 当前正在执行的节点
executedNodes: Map<string, 'success' | 'fail' | 'skipped'>;  // 已执行节点结果

// 新增 action
setExecutingNode(uuid: string | null): void;
setNodeResult(uuid: string, result: 'success' | 'fail' | 'skipped'): void;
```

**节点组件对接**（每个节点的基座）：

在 `FlowCanvas.tsx` 层统一处理，通过 `nodeTypes` wrapper 注入执行状态：

```tsx
// 在 FlowCanvas.tsx 的 nodeTypes 外层包裹
function withExecutionState(WrappedNode: ComponentType<NodeProps>) {
  return function ExecutionAwareNode(props: NodeProps) {
    const executingNodeUuid = useExecutionStore(s => s.executingNodeUuid);
    const executedNodes = useExecutionStore(s => s.executedNodes);
    const result = executedNodes.get(props.id);
    const isExecuting = executingNodeUuid === props.id;

    return (
      <div className={cn(
        isExecuting && 'node-executing',
        result === 'success' && 'node-succeeded',
        result === 'fail' && 'node-failed',
      )}>
        <WrappedNode {...props} />
      </div>
    );
  };
}
```

### 2.2 连接边数据流动画

**目标**：数据在节点间传递时，边上有动态流动粒子效果。

#### 现状

所有边都设置了 `animated: true`，统一使用默认的虚线动画，无法区分"有数据流过"和"静态连接"。

#### 实现方案

修改 `CustomEdge.tsx`，在执行期间激活更丰富的动画：

```tsx
// src/components/canvas/edges/CustomEdge.tsx
// 接收 isActive 属性
interface CustomEdgeData {
  isActive?: boolean;
  dataType?: string;   // 'http' | 'variable' | 'control'
}
```

- 非执行状态：静态虚线（当前样式）
- 执行中激活的边：流动光点粒子 + 颜色渐变
- 数据类连接（数据传输）：蓝色流动粒子
- 控制类连接（条件分支）：橙色流动粒子

#### 实现参考

```tsx
// SVG 动画路径
<svg>
  <defs>
    <linearGradient id="active-edge-gradient">
      <stop offset="0%" stopColor="#3b82f6" />
      <stop offset="100%" stopColor="#8b5cf6" />
    </linearGradient>
  </defs>
  {/* 激活时叠加流动光点 */}
  {isActive && (
    <circle r="3" fill="#3b82f6">
      <animateMotion dur="1s" repeatCount="indefinite" path={pathData} />
    </circle>
  )}
</svg>
```

### 2.3 节点执行结果即时着色

**目标**：每个节点执行完毕后立即在画布上变色，无需等待最终报告。

#### 实现方案

结合 `2.1` 的 Store 设计，后端通过 `ExecutionEvent` 通道实时推送节点完成事件：

```typescript
// src/types/execution_event.ts - 已有类型
// 确保包含以下字段
interface NodeCompletedEvent {
  type: 'node_completed';
  node_uuid: string;
  status: 'success' | 'failed' | 'skipped';
  timestamp: number;
}
```

前端监听后即刻更新 Store → 节点 CSS 类切换。

### 2.4 实时变量浮窗

**目标**：执行时或执行后，悬停在节点上可查看该节点的输入/输出变量。

#### 实现方案

```tsx
// 在 withExecutionState wrapper 中增加 hover 浮窗
function NodeVariableTooltip({ nodeId }: { nodeId: string }) {
  const nodeReports = useExecutionStore(s => s.report?.node_reports ?? []);

  return (
    <HoverCard>
      <HoverCardContent className="w-80">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold">节点变量</h4>
          {/* 输入变量 */}
          <div>
            <span className="text-xs text-muted-foreground">输入</span>
            <VariableTreeView data={inputVars} />
          </div>
          {/* 输出变量 */}
          <div>
            <span className="text-xs text-muted-foreground">输出</span>
            <VariableTreeView data={outputVars} />
          </div>
        </div>
      </HoverCardContent>
    </HoverCard>
  );
}
```

---

## 3. 实现计划

### 里程碑 M0.1 — 引导系统（3 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| WelcomeWizard 组件 | 新建 + `App.tsx` | 1d |
| 空画布改进 | `FlowCanvas.tsx` | 0.5d |
| 帮助内容注册 | `nodeHelpRegistry.ts` | 0.5d |
| 帮助面板 | `HelpDrawer.tsx` + `NodeConfigPanel.tsx` | 0.5d |
| 模板扩展 | `builtinFlows.ts` + `NewFlowDialog.tsx` | 0.5d |

### 里程碑 M0.2 — 执行可视化（4 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| Store 扩展 | `execution.ts` | 0.5d |
| 节点执行 wrapper | `FlowCanvas.tsx` | 1d |
| 边流动画增强 | `CustomEdge.tsx` | 1d |
| 即时着色 | `index.css` + Store 对接 | 0.5d |
| 变量浮窗 | `NodeVariableTooltip.tsx` | 1d |

### 验证标准

- [ ] 新用户首次启动可见 3 步引导，可顺利完成创建 → 执行
- [ ] 每个节点配置面板有 ? 帮助按钮，点击展示帮助内容
- [ ] 15+ 模板可用，每个协议至少 1 个
- [ ] 空画布显示 3 个快捷操作卡片 + 模板推荐
- [ ] 执行中正在运行的节点有脉冲高亮动画
- [ ] 节点执行完毕即时变色（绿/红）
- [ ] 执行中连接边有流动粒子效果
- [ ] 悬停节点可查看变量输入/输出
