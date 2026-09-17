# UX 优化路线图 — P2：体验打磨 (Medium Priority)

> 产品整体体验从"功能好用"提升到"使用优雅"。着重于布局灵活度、反馈质感、流程管理能力、动画细节。

---

## 目录

1. [布局自适应与工作区管理](#1-布局自适应与工作区管理)
2. [错误与状态反馈升级](#2-错误与状态反馈升级)
3. [流程管理增强](#3-流程管理增强)
4. [动画与微交互打磨](#4-动画与微交互打磨)
5. [辅助功能（a11y）](#5-辅助功能a11y)
6. [实现计划](#6-实现计划)

---

## 1. 布局自适应与工作区管理

### 1.1 全屏画布模式

**目标**：一键隐藏所有面板，让用户专注于流程编辑。

#### 实现方案

```typescript
// src/stores/ui.ts — 新增状态
interface UIState {
  // ...
  isFullscreenCanvas: boolean;
  toggleFullscreenCanvas: () => void;
}
```

```tsx
// MainLayout.tsx — 全屏模式下
{isFullscreenCanvas ? (
  // 只渲染画布 + 工具栏
  <div className="h-screen w-screen">
    <FlowCanvas />
    {/* 浮动小面板：点击显示/隐藏 */}
    <FloatingMiniToolbar />
  </div>
) : (
  // 正常布局（左中右+底部）
  ...
)}
```

#### 快捷键

- `F11` 或 `Ctrl+Shift+F` 切换全屏模式
- 全屏时右上角显示"退出全屏"按钮
- 底部显示小型进度条（不遮挡画布）

#### 关联文件

| 文件 | 修改内容 |
|------|---------|
| `src/stores/ui.ts` | 新增全屏状态 |
| `src/layouts/MainLayout.tsx` | 条件渲染全屏模式 |
| `src/components/canvas/` 新建 `FloatingMiniToolbar.tsx` | 全屏迷你工具栏 |
| `src/hooks/useKeyboardShortcuts.ts` | 注册 `F11` 快捷键 |

### 1.2 面板布局预设

**目标**：一键切换不同工作模式的面板组合。

#### 预设定义

```typescript
interface LayoutPreset {
  id: string;
  name: string;
  icon: string;
  panels: {
    left: boolean;
    right: boolean;
    bottom: boolean;
    bottomTab: BottomPanelTab;
    rightMode: ConfigPanelMode;
    leftWidth: number;
    rightWidth: number;
    bottomHeight: number;
  };
}

const LAYOUT_PRESETS: LayoutPreset[] = [
  {
    id: 'edit',
    name: '编辑模式',
    icon: 'Pencil',
    panels: { left: true, right: true, bottom: false, bottomTab: 'history', rightMode: 'node', leftWidth: 280, rightWidth: 320, bottomHeight: 208 },
  },
  {
    id: 'debug',
    name: '调试模式',
    icon: 'Bug',
    panels: { left: true, right: true, bottom: true, bottomTab: 'running', rightMode: 'debug', leftWidth: 320, rightWidth: 360, bottomHeight: 300 },
  },
  {
    id: 'execute',
    name: '执行模式',
    icon: 'Play',
    panels: { left: false, right: false, bottom: true, bottomTab: 'running', rightMode: 'node', leftWidth: 280, rightWidth: 320, bottomHeight: 400 },
  },
  {
    id: 'present',
    name: '展示模式',
    icon: 'Monitor',
    panels: { left: false, right: false, bottom: false, bottomTab: 'history', rightMode: 'node', leftWidth: 280, rightWidth: 320, bottomHeight: 208 },
  },
];
```

#### UI

在顶部工具栏增加预设选择器（位于环境选择器旁）：

```tsx
// CanvasToolbar.tsx
<Select value={currentPreset} onValueChange={applyPreset}>
  <SelectTrigger className="w-28 h-8">
    <SelectValue />
  </SelectTrigger>
  <SelectContent>
    {LAYOUT_PRESETS.map(p => (
      <SelectItem key={p.id} value={p.id}>
        <div className="flex items-center gap-2">
          <Icon name={p.icon} className="w-4 h-4" />
          {p.name}
        </div>
      </SelectItem>
    ))}
  </SelectContent>
</Select>
```

### 1.3 面板状态持久化

**目标**：用户调整的面板尺寸、折叠状态在重启后恢复。

#### 现状

`ui.ts` 中的面板尺寸（`sidebarWidth`, `configPanelWidth`, `bottomPanelHeight`）和面板开关状态仅在内存中，刷新后恢复默认值。

#### 实现方案

```typescript
// src/stores/ui.ts — 在初始化时从 localStorage 恢复
const UI_STORAGE_KEY = 'mpe-ui-layout';

function loadLayoutFromStorage(): Partial<UIState> {
  try {
    const saved = localStorage.getItem(UI_STORAGE_KEY);
    if (saved) return JSON.parse(saved);
  } catch { /* ignore */ }
  return {};
}

// 每次布局变化时保存
function saveLayoutToStorage(state: UIState) {
  const layout = {
    sidebarWidth: state.sidebarWidth,
    configPanelWidth: state.configPanelWidth,
    bottomPanelHeight: state.bottomPanelHeight,
    isFlowListPanelOpen: state.isFlowListPanelOpen,
    isHistoryPanelOpen: state.isHistoryPanelOpen,
    isConfigPanelOpen: state.isConfigPanelOpen,
    showMinimap: state.showMinimap,
  };
  localStorage.setItem(UI_STORAGE_KEY, JSON.stringify(layout));
}
```

#### 优化：使用 `zustand/middleware` 的 `persist`

```typescript
import { persist } from 'zustand/middleware';

export const useUIStore = create(
  persist<UIState>(
    (set) => ({
      // ... 现有代码
    }),
    {
      name: 'mpe-ui-store',
      partialize: (state) => ({
        // 只持久化布局相关字段
        sidebarWidth: state.sidebarWidth,
        configPanelWidth: state.configPanelWidth,
        bottomPanelHeight: state.bottomPanelHeight,
        isFlowListPanelOpen: state.isFlowListPanelOpen,
        isHistoryPanelOpen: state.isHistoryPanelOpen,
        isConfigPanelOpen: state.isConfigPanelOpen,
        showMinimap: state.showMinimap,
      }),
    }
  )
);
```

### 1.4 浮动节点配置面板

**目标**：将右侧固定配置面板改为可拖动的浮动弹窗（类似 Figma 的属性面板），释放画布空间。

#### 实现方案

```tsx
// 可选模式：用户可在固定面板/浮动面板间切换

// 使用 @radix-ui/react-popover 或 Portal
// 选中节点后，在节点旁边弹出浮动配置窗口
<DraggablePanel
  anchorEl={selectedNodeElement}
  side="right"
  offset={12}
  defaultOpen
  onClose={() => selectNode(null)}
>
  <NodeConfigPanel nodeId={selectedNodeId} />
</DraggablePanel>
```

#### 优势

- 画布可用空间大幅增加
- 配置面板跟随所选节点，视觉关联更清晰
- 可同时打开多个节点的配置（对比参数）

---

## 2. 错误与状态反馈升级

### 2.1 操作反馈中心

**目标**：右下角聚合显示操作结果，替代分散的 Toast 通知。

#### 设计方案

```tsx
// 新建: src/components/common/NotificationCenter.tsx
// 类似 VS Code 的通知中心

interface Notification {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  title: string;
  description?: string;
  timestamp: number;
  action?: { label: string; onClick: () => void };
  progress?: number;  // 0-100，有进度时为进度通知
  persistent?: boolean; // 不会自动消失
}
```

- 右下角通知图标（显示未读数量）
- 点击展开通知列表
- 可清除单条或全部
- 错误通知可展开查看详情（stack trace）
- 操作通知保留至少 30 条历史

### 2.2 错误智能修复建议

**目标**：验证错误不仅报告问题，还提供一键修复方案。

#### 当前情况

验证失败时仅显示错误文本，用户需要自行排查。

#### 方案

```typescript
// 新文件: src/lib/errorFixSuggestions.ts
interface FixSuggestion {
  errorPattern: RegExp;          // 匹配错误信息的正则
  suggestion: string;            // 修复建议（人类可读）
  autoFix?: () => Partial<Node>; // 可选：自动修复函数
  relatedDocs?: string;          // 相关文档链接
}

const FIX_SUGGESTIONS: FixSuggestion[] = [
  {
    errorPattern: /connection.*refused/i,
    suggestion: '目标服务未启动或端口错误。请检查主机和端口是否正确，以及目标服务是否在运行。',
    relatedDocs: 'tcp:connect',
  },
  {
    errorPattern: /no such host/i,
    suggestion: '域名解析失败。请检查 URL/主机名是否正确，或尝试使用 IP 地址代替域名。',
  },
  {
    errorPattern: /timeout/i,
    suggestion: '请求超时。目标服务响应过慢，可以增大超时时间（timeout_ms）或检查网络连接。',
    autoFix: () => ({ timeout_ms: 60000 }),
  },
  {
    errorPattern: /401|unauthorized/i,
    suggestion: '认证失败。请检查请求头中的认证凭据（Token/API Key/用户名密码）是否正确。',
  },
  {
    errorPattern: /ssl.*certificate|tls.*handshake/i,
    suggestion: 'TLS/SSL 握手失败。可能是证书过期、自签名证书或中间人证书。可尝试"跳过证书验证"选项。',
  },
  {
    errorPattern: /kafka.*broker.*unavailable/i,
    suggestion: 'Kafka Broker 不可达。请检查 brokers 地址列表，以及是否已建立 kafka_connect 连接。',
  },
];
```

#### UI

- 在执行报告的错误详情中显示"智能建议"卡片
- 错误旁边显示 🔧 图标，点击展开修复建议
- 如果有 `autoFix`，显示"一键修复"按钮

### 2.3 连接状态指示器

**目标**：全局显示后端服务连接状态。

```tsx
// 顶部工具栏添加
<ConnectionIndicator
  status={backendStatus}
  onClick={showStatusDetail}
/>

// 三种状态
// 🟢 Connected — 正常
// 🟡 Reconnecting — 断线重连
// 🔴 Disconnected — 断线
```

### 2.4 进度条精细化

**目标**：长时间操作（执行、压测、数据驱动）显示具体进度。

```typescript
// 执行进度事件
interface ExecutionProgressEvent {
  type: 'execution_progress';
  currentStep: number;
  totalSteps: number;
  currentNodeName: string;
  nodeType: string;
  elapsedMs: number;
  estimatedRemainingMs: number;
}

// UI
<ProgressBar
  value={currentStep / totalSteps * 100}
  label={`${currentStep}/${totalSteps}: ${currentNodeName}`}
  sublabel={`已用 ${formatTime(elapsedMs)} · 预计剩余 ${formatTime(estimatedRemainingMs)}`}
/>
```

---

## 3. 流程管理增强

### 3.1 流程图缩略图预览

**目标**：在流程列表中悬停流程项时，弹出微型画布预览。

```tsx
// FlowListPanel.tsx — 悬停时
<HoverCard>
  <HoverCardTrigger>
    <FlowListItem ... />
  </HoverCardTrigger>
  <HoverCardContent className="w-64 h-48">
    <FlowMinimap
      nodes={flow.nodes}
      connections={flow.connections}
      width={256}
      height={192}
    />
  </HoverCardContent>
</HoverCard>
```

#### 实现方式

- 使用 `<svg>` 绘制简化版的节点框图，无需加载完整 React Flow
- 节点简化为矩形 + 标签，连接为简化线条
- 缓存渲染结果到 `IndexedDB` 或 `localStorage`（base64）

### 3.2 批量操作

**目标**：支持批量选中、移动、删除、导出流程。

```tsx
// FlowListPanel.tsx — 增加选择模式
// 通过长按或 Ctrl+点击进入多选模式

interface BatchActionBar {
  selectedCount: number;
  onBatchDelete: () => void;
  onBatchExport: () => void;
  onBatchMove: () => void;
  onClearSelection: () => void;
}

// 操作栏
{batchMode && (
  <div className="sticky bottom-0 ...">
    <span>已选择 {selectedCount} 个流程</span>
    <Button onClick={onBatchExport}>导出</Button>
    <Button onClick={onBatchMove}>移动到</Button>
    <Button variant="destructive" onClick={onBatchDelete}>删除</Button>
  </div>
)}
```

### 3.3 流程版本历史

**目标**：自动保存流程快照，支持查看和回滚历史版本。

#### 设计方案

```typescript
// 新建: src/services/storage/flowSnapshotStorage.ts
interface FlowSnapshot {
  id: string;
  flowUuid: string;
  name: string;
  timestamp: number;
  version: number;
  description?: string;   // 自动生成："执行前自动快照"、"手动保存"
  flow: Flow;             // 完整快照
  reportUuid?: string;    // 关联的执行报告
}
```

#### 触发时机

- 每次执行前自动创建快照（描述："执行前自动快照 - 2026-06-04 10:30"）
- 每次手动保存时创建快照（描述："手动保存 - 2026-06-04 11:00"）
- 用户手动点击"创建快照"

#### UI

- `FlowListPanel.tsx` 中流程右键菜单 → "查看版本历史"
- 时间线视图展示所有版本，可预览差异（使用现有 `FlowCompareDialog.tsx`）
- 一键回滚到任意历史版本

### 3.4 流程标签/标记

**目标**：为流程打标签，支持按标签快速过滤。

```typescript
// Flow 类型扩展
interface Flow {
  // ... 现有字段
  tags?: string[];   // 新增
  color?: string;    // 新增：标记颜色
}

// 预定义标签
const BUILTIN_TAGS = [
  { name: '重要', color: '#ef4444' },
  { name: 'WIP', color: '#f59e0b' },
  { name: '废弃', color: '#6b7280' },
  { name: '精选', color: '#8b5cf6' },
];
```

- 流程列表项左侧显示彩色标签圆点
- 搜索栏旁增加标签筛选器
- 标签可自定义创建

---

## 4. 动画与微交互打磨

### 4.1 节点添加动画

**目标**：新建节点时有流畅的入场动画。

```css
@keyframes node-entrance {
  0% {
    opacity: 0;
    transform: scale(0.5) translateY(-20px);
  }
  60% {
    opacity: 1;
    transform: scale(1.05) translateY(0);
  }
  100% {
    transform: scale(1) translateY(0);
  }
}

.react-flow__node-adding {
  animation: node-entrance 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}
```

- 使用 React Flow 的 `nodeOrigin` 定位新节点
- 通过 `useEffect` 在节点出现时临时添加 `node-adding` 类，动画结束后移除

### 4.2 连接创建动画

**目标**：拖拽连线时的视觉反馈提升。

#### 改进方案

- 拖拽过程中连接线有发光拖尾效果
- 当拖到有效目标节点范围时，目标端口放大提示（已有 `hover:!scale-125`）
- 当拖到无效目标时，连接线变红闪烁
- 连接成功时，连接线从起点到终点流动扩散动画

```css
/* 连接创建成功的扩散动画 */
@keyframes edge-creation-pulse {
  0% {
    stroke-width: 4;
    stroke-opacity: 0.8;
  }
  100% {
    stroke-width: 1.5;
    stroke-opacity: 1;
  }
}
```

### 4.3 拖拽对齐参考线

**目标**：画布上拖拽节点时显示对齐参考线，帮助整齐排列。

#### 实现方案

- 使用 `react-flow__node-dragging` 类配合 mousemove 事件
- 当节点中心与附近节点中心对齐（X 或 Y 轴偏差 ≤ 5px）时，渲染对齐线
- 对齐线：贯穿整个画布的虚线

```tsx
// 在 FlowCanvas.tsx 中
// 使用 SVG overlay 渲染对齐线
const [snapLines, setSnapLines] = useState<SnapLine[]>([]);

function updateSnapLines(draggingNode: Node, allNodes: Node[]) {
  const lines: SnapLine[] = [];
  const threshold = 5;

  for (const other of allNodes) {
    if (other.id === draggingNode.id) continue;

    // 检查 X 轴对齐（左中右）
    if (Math.abs(draggingNode.position.x - other.position.x) < threshold) {
      lines.push({ type: 'vertical', position: other.position.x });
    }
    // 检查 Y 轴对齐（上中下）
    if (Math.abs(draggingNode.position.y - other.position.y) < threshold) {
      lines.push({ type: 'horizontal', position: other.position.y });
    }
  }

  setSnapLines(lines);
}

// 渲染
<svg className="absolute inset-0 pointer-events-none z-50">
  {snapLines.map((line, i) => (
    <line
      key={i}
      x1={line.type === 'vertical' ? line.position : 0}
      y1={line.type === 'horizontal' ? line.position : 0}
      x2={line.type === 'vertical' ? line.position : canvasWidth}
      y2={line.type === 'horizontal' ? line.position : canvasHeight}
      stroke="#3b82f6"
      strokeWidth={1}
      strokeDasharray="4 4"
      opacity={0.6}
    />
  ))}
</svg>
```

### 4.4 面板切换弹簧动画

**目标**：面板展开/收起使用更流畅的弹簧动画（spring animation）。

```css
/* 替换 transition-all duration-300 ease-out */
.panel-slide {
  transition: width 0.4s cubic-bezier(0.34, 1.56, 0.64, 1),
              transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}
```

或在 JS 层面使用 `framer-motion`（如果引入）的 spring 动画：

```tsx
// 示例：使用 framer-motion 动画
<motion.div
  animate={{
    width: isOpen ? sidebarWidth : 0,
    opacity: isOpen ? 1 : 0,
  }}
  transition={{
    type: "spring",
    stiffness: 300,
    damping: 30,
  }}
>
  {children}
</motion.div>
```

### 4.5 骨架屏（Skeleton Loading）

**目标**：流程列表加载时显示骨架屏，替代 spinner。

```tsx
<>
  {isLoading ? (
    <div className="space-y-2 p-2">
      {[1, 2, 3].map(i => (
        <div key={i} className="h-12 rounded-lg bg-muted animate-pulse" />
      ))}
      <div className="h-4 w-24 rounded bg-muted animate-pulse ml-2" />
    </div>
  ) : (
    <FlowList ... />
  )}
</>
```

---

## 5. 辅助功能（a11y）

### 5.1 焦点管理

**问题**：当前大量 `role="button"` 元素，但 Tab 键导航不够完整。

#### 改进清单

| 改进项 | 涉及文件 | 说明 |
|--------|---------|------|
| 画布节点 Tab 导航 | `FlowCanvas.tsx` + 各节点组件 | 节点可获得焦点，使用 Tab/方向键导航 |
| 配置表单 Tab 顺序 | 各 Config 面板 | 确保自然从左到右、从上到下的 Tab 顺序 |
| Esc 关闭弹窗/面板 | `NodePalette.tsx`, `ConfirmDialog.tsx` 等 | 确认所有弹窗都支持 Esc 关闭（部分已有） |
| 焦点陷阱 | `NodePalette.tsx`, `ShortcutsDialog.tsx` | 弹窗打开时焦点锁定在弹窗内 |

### 5.2 ARIA 标签

**问题**：大量图标按钮只有 `title`，没有 `aria-label`。

#### 批量修复

```tsx
// 对项目中所有纯图标按钮增加 aria-label
<Button
  variant="ghost"
  size="icon"
  onClick={...}
  title="撤销"
  aria-label="撤销 (Ctrl+Z)" // 新增
>
  <Undo2 className="w-4 h-4" />
</Button>
```

#### 检查工具

使用 `grep` 搜索所有 `<Button variant="ghost" size="icon">` 实例，对缺少 `aria-label` 的补充。

```bash
# 搜索所有图标按钮
grep -r 'variant="ghost".*size="icon"' src/components/ --include="*.tsx" -l
```

### 5.3 高对比度模式

**问题**：部分节点颜色在暗色模式下对比度不足。

#### 修复方案

```css
/* 全局高对比度媒体查询 */
@media (forced-colors: active) {
  .react-flow__node {
    border-width: 2px;
    border-color: CanvasText;
  }

  .bg-background {
    background: Canvas;
  }

  .text-muted-foreground {
    color: GrayText;
  }
}

/* Windows 高对比度主题 */
@media (-ms-high-contrast: active) {
  /* 同上 */
}
```

### 5.4 屏幕阅读器支持

| 改进项 | 说明 |
|--------|------|
| 流程画布 `role="application"` + `aria-label` | 描述当前流程名称和节点数 |
| 节点选中/取消选中状态 | `aria-selected` 或 `aria-current` |
| 拖拽操作说明 | `aria-dropeffect`, `aria-grabbed` |
| 执行进度播报 | `aria-live="polite"` 区域播报当前执行节点 |
| 错误提示播报 | `role="alert"` 播报执行错误 |

---

## 6. 实现计划

### 里程碑 M2.1 — 布局升级（5 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 全屏画布模式 | `MainLayout.tsx` + `FloatingMiniToolbar.tsx` | 1.5d |
| 面板布局预设 | `CanvasToolbar.tsx` + `ui.ts` | 1d |
| 面板状态持久化 | `ui.ts` (zustand persist) | 0.5d |
| 浮动配置面板 | `DraggablePanel.tsx` + `NodeConfigPanel.tsx` | 2d |

### 里程碑 M2.2 — 反馈系统（3 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 操作反馈中心 | `NotificationCenter.tsx` | 1d |
| 错误智能修复 | `errorFixSuggestions.ts` + 报告视图 | 1d |
| 连接状态指示器 + 进度条 | `ConnectionIndicator.tsx` + `ProgressBar.tsx` | 1d |

### 里程碑 M2.3 — 流程管理（4 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 缩略图预览 | `FlowMinimap.tsx` + `FlowListPanel.tsx` | 1d |
| 批量操作 | `FlowListPanel.tsx` | 1d |
| 版本历史 | `flowSnapshotStorage.ts` + 时间线 UI | 1.5d |
| 流程标签 | `flow.ts` 类型扩展 + `FlowListPanel.tsx` | 0.5d |

### 里程碑 M2.4 — 动画 & a11y（4 天）

| 任务 | 文件 | 估时 |
|------|------|------|
| 节点添加/连接创建动画 | `FlowCanvas.tsx` + `index.css` | 1d |
| 拖拽对齐参考线 | `FlowCanvas.tsx` | 1d |
| 骨架屏 | `FlowListPanel.tsx` + 其他 loading 区域 | 0.5d |
| ARIA 标签批量补充 | 全组件 | 0.5d |
| 焦点管理 + 高对比度 | 全组件 + `index.css` | 1d |

### 验证标准

- [ ] `F11` 切换全屏画布模式，所有面板隐藏，不影响操作
- [ ] 4 种布局预设可一键切换，面板状态匹配
- [ ] 调整面板尺寸后刷新/重启，布局状态恢复
- [ ] 右下角通知中心聚合所有操作结果，可展开查看
- [ ] HTTP 401/连接拒绝/kafka 超时等常见错误显示修复建议
- [ ] 连接状态指示器实时反映后端状态
- [ ] 执行中精确进度 `3/7: GET /api/users`
- [ ] 流程列表悬停显示缩略图预览
- [ ] 多选流程可批量导出/删除/移动
- [ ] 流程版本历史时间线 + 对比 + 回滚
- [ ] 流程标签创建 + 筛选
- [ ] 新建节点有弹入动画
- [ ] 拖拽节点显示对齐参考线
- [ ] 加载状态使用骨架屏替换 spinner
- [ ] Tab 键可导航所有交互元素
- [ ] 所有图标按钮有 `aria-label`
- [ ] Windows 高对比度模式正常显示
