# 画布撤销/重做功能技术设计文档

> 状态：待实现
> 创建时间：2026-03-02
> 最后更新：2026-03-02

## 1. 功能概述

### 1.1 需求描述

为流程画布编辑器添加撤销/重做（Undo/Redo）功能，支持用户：

- 通过 `Ctrl+Z` / `Ctrl+Y` 快捷键撤销/重做操作
- 通过工具栏按钮撤销/重做操作
- 撤销/重做以下操作：
  - 节点添加/删除/位置变更/配置修改
  - 连接创建/删除
  - 流程名称/初始变量修改

### 1.2 当前状态

| 项目 | 状态 |
|------|------|
| 现有撤销机制 | ❌ 无 |
| 状态管理 | Zustand v5 (`useFlowStore`) |
| 画布组件 | React Flow (`useNodesState` / `useEdgesState`) |
| 快捷键支持 | `useKeyboardShortcuts.ts` (仅 Delete/Escape) |

**注意**：现有 `ExecutionHistory.tsx` 是**执行历史**（记录每次运行结果），非编辑历史。

---

## 2. 技术方案对比

### 2.1 方案概览

| 方案 | 存储方式 | 内存效率 | 实现复杂度 | 适用场景 |
|------|----------|----------|------------|----------|
| **zundo** | 完整快照 | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 中小型流程图 |
| **zustand-travel** | JSON Patch | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 大型流程图 |
| **Command Pattern** | 操作对象 | ⭐⭐⭐⭐ | ⭐⭐ | 复杂业务逻辑 |
| **手动快照** | 完整快照 | ⭐⭐ | ⭐⭐⭐ | 简单场景 |

### 2.2 方案详情

#### 方案 A: zundo (快照模式)

**原理**：每次状态变更时存储完整快照到 `pastStates[]` 栈。

```typescript
import { temporal } from 'zundo';

export const useFlowStore = create<FlowState>()(
  temporal(
    (set, get) => ({ /* 现有代码 */ }),
    {
      partialize: ({ flow }) => ({ flow }), // 只追踪 flow
      limit: 50,                            // 限制历史栈大小
      equality: (a, b) => JSON.stringify(a) === JSON.stringify(b),
    }
  )
);
```

**优点**：
- 实现最简单，改动最小
- 社区成熟 (840+ stars)
- 类型安全

**缺点**：
- 内存占用较高（每次存储完整状态）
- 大型流程图可能性能问题

#### 方案 B: zustand-travel (JSON Patch 模式) ⭐ 推荐

**原理**：使用 JSON Patch 只存储状态变更差异，内存占用节省 80-90%。

```typescript
import { travel } from 'zustand-travel';

export const useFlowStore = create<FlowState>()(
  travel(
    (set) => ({
      flow: null,
      updateNode: (uuid, updates) => set((state) => {
        if (!state.flow) return;
        const node = state.flow.nodes.find(n => n.uuid === uuid);
        if (node) Object.assign(node, updates); // mutation 风格
      }),
    }),
    {
      maxHistory: 100,
      autoArchive: false, // 手动控制归档时机
    }
  )
);

// 在适当时机手动归档
onNodeDragStop: () => {
  useFlowStore.getControls().archive();
}
```

**优点**：
- 内存效率最高
- 适合流程图编辑器（频繁的小范围变更）
- 支持跳转到任意历史点

**缺点**：
- 需要使用 mutation 风格更新状态
- 相对较新，生态不如 zundo

#### 方案 C: Command Pattern (命令模式)

**原理**：将每个操作封装为可逆的命令对象。

```typescript
interface Command {
  execute(): void;
  undo(): void;
  description: string;
}

class MoveNodeCommand implements Command {
  constructor(
    private nodeId: string,
    private oldPosition: Position,
    private newPosition: Position,
  ) {}
  
  execute() { /* 移动到 newPosition */ }
  undo() { /* 移动到 oldPosition */ }
}

class CommandManager {
  private history: Command[] = [];
  private currentIndex = -1;
  
  execute(cmd: Command) {
    cmd.execute();
    this.history = this.history.slice(0, this.currentIndex + 1);
    this.history.push(cmd);
    this.currentIndex++;
  }
  
  undo() {
    if (this.currentIndex >= 0) {
      this.history[this.currentIndex--].undo();
    }
  }
}
```

**优点**：
- 最大灵活性
- 支持操作合并（连续拖拽合并为一个）
- 可添加操作描述

**缺点**：
- 实现复杂，需重构所有操作
- 每个操作都需要实现 undo 逻辑

### 2.3 推荐方案

考虑到项目特点（流程编排引擎，可能存在较多节点）：

| 预期规模 | 推荐方案 |
|----------|----------|
| 节点数 < 100 | zundo (实现更简单) |
| 节点数 > 100 | **zustand-travel** (内存更优) |

**建议采用 zustand-travel**，理由：
1. 流程图编辑器适合 JSON Patch 模式
2. 未来扩展性好
3. 与现有 Zustand 架构无缝集成

---

## 3. 详细设计 (基于 zustand-travel)

### 3.1 Store 改造

**文件**：`src/stores/flow.ts`

```typescript
import { create } from 'zustand';
import { travel, TravelState } from 'zustand-travel';
import type { Flow, Node, Connection } from '@/types';

interface FlowState {
  flow: Flow | null;
  selectedNodeId: string | null;
  isDirty: boolean;
  pendingAddNodeType: string | null;

  // Actions
  setFlow: (flow: Flow | null) => void;
  addNode: (node: Node) => void;
  updateNode: (uuid: string, updates: Partial<Node>) => void;
  removeNode: (uuid: string) => void;
  addConnection: (connection: Connection) => void;
  removeConnection: (id: string) => void;
  // ... 其他现有方法
}

export const useFlowStore = create<FlowState>()(
  travel(
    (set, get) => ({
      flow: null,
      selectedNodeId: null,
      isDirty: false,
      pendingAddNodeType: null,

      setFlow: (flow) => set((state) => {
        state.flow = flow;
        state.selectedNodeId = null;
        state.isDirty = false;
      }),

      addNode: (node) => set((state) => {
        if (state.flow) {
          state.flow.nodes.push(node);
          state.isDirty = true;
        }
      }),

      updateNode: (uuid, updates) => set((state) => {
        if (state.flow) {
          const node = state.flow.nodes.find(n => n.uuid === uuid);
          if (node) {
            Object.assign(node, updates);
            state.isDirty = true;
          }
        }
      }),

      removeNode: (uuid) => set((state) => {
        if (state.flow) {
          state.flow.nodes = state.flow.nodes.filter(n => n.uuid !== uuid);
          state.flow.connections = state.flow.connections.filter(
            c => c.source_node_uuid !== uuid && c.target_node_uuid !== uuid
          );
          if (state.selectedNodeId === uuid) {
            state.selectedNodeId = null;
          }
          state.isDirty = true;
        }
      }),

      addConnection: (connection) => set((state) => {
        if (state.flow) {
          state.flow.connections.push(connection);
          state.isDirty = true;
        }
      }),

      removeConnection: (id) => set((state) => {
        if (state.flow) {
          state.flow.connections = state.flow.connections.filter(c => c.id !== id);
          state.isDirty = true;
        }
      }),

      // ... 其他方法
    }),
    {
      maxHistory: 50,
      autoArchive: false, // 手动归档
    }
  )
);

// 导出撤销控制器类型
export type FlowStoreControls = ReturnType<typeof useFlowStore.getControls>;
```

### 3.2 撤销 Hook

**新建文件**：`src/hooks/useUndoRedo.ts`

```typescript
import { useCallback, useEffect } from 'react';
import { useFlowStore } from '@/stores';
import { useReactFlow } from '@xyflow/react';

export function useUndoRedo() {
  const controls = useFlowStore.getControls();
  const { setNodes, setEdges, getNodes, getEdges } = useReactFlow();
  const flow = useFlowStore((state) => state.flow);

  // 检查是否可撤销/重做
  const canUndo = controls.position > 0;
  const canRedo = controls.position < controls.patches.length;

  // 撤销
  const undo = useCallback(() => {
    if (!canUndo) return;
    controls.back();
    syncToReactFlow();
  }, [canUndo, controls]);

  // 重做
  const redo = useCallback(() => {
    if (!canRedo) return;
    controls.forward();
    syncToReactFlow();
  }, [canRedo, controls]);

  // 同步到 React Flow 画布
  const syncToReactFlow = useCallback(() => {
    const currentFlow = useFlowStore.getState().flow;
    if (!currentFlow) return;

    // 转换并同步节点
    const reactFlowNodes = currentFlow.nodes.map(node => ({
      id: node.uuid,
      type: node.type,
      position: node.position,
      data: node.data,
    }));
    setNodes(reactFlowNodes);

    // 转换并同步连接
    const reactFlowEdges = currentFlow.connections.map(conn => ({
      id: conn.id,
      source: conn.source_node_uuid,
      target: conn.target_node_uuid,
      sourceHandle: conn.source_port_id,
      targetHandle: conn.target_port_id,
      type: 'custom',
    }));
    setEdges(reactFlowEdges);
  }, [setNodes, setEdges]);

  // 手动归档（在适当时机调用）
  const archive = useCallback(() => {
    if ('archive' in controls) {
      (controls as any).archive();
    }
  }, [controls]);

  return {
    undo,
    redo,
    canUndo,
    canRedo,
    archive,
  };
}
```

### 3.3 快捷键支持

**修改文件**：`src/hooks/useKeyboardShortcuts.ts`

```typescript
import { useEffect } from 'react';
import { useFlowStore } from '@/stores';

export function useKeyboardShortcuts() {
  const { removeNode, selectedNodeId, flow } = useFlowStore();
  const controls = useFlowStore.getControls();

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // 检查是否在输入框中
      const activeElement = document.activeElement;
      const isInputFocused = 
        activeElement instanceof HTMLInputElement ||
        activeElement instanceof HTMLTextAreaElement ||
        activeElement?.hasAttribute('contenteditable');

      // Ctrl+Z: 撤销
      if ((event.ctrlKey || event.metaKey) && event.key === 'z' && !event.shiftKey) {
        if (isInputFocused) return; // 输入框中不拦截
        event.preventDefault();
        controls.back();
        return;
      }

      // Ctrl+Y 或 Ctrl+Shift+Z: 重做
      if ((event.ctrlKey || event.metaKey) && 
          (event.key === 'y' || (event.key === 'z' && event.shiftKey))) {
        if (isInputFocused) return;
        event.preventDefault();
        controls.forward();
        return;
      }

      // Delete: 删除选中节点（现有逻辑）
      if (event.key === 'Delete' && selectedNodeId) {
        if (isInputFocused) return;
        const node = flow?.nodes.find(n => n.uuid === selectedNodeId);
        if (node && node.type !== 'entry') {
          removeNode(selectedNodeId);
          // 归档删除操作
          if ('archive' in controls) {
            (controls as any).archive();
          }
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [removeNode, selectedNodeId, flow, controls]);
}
```

### 3.4 UI 组件

**新建文件**：`src/components/toolbar/UndoRedoButtons.tsx`

```tsx
import { Undo2, Redo2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useUndoRedo } from '@/hooks/useUndoRedo';

export function UndoRedoButtons() {
  const { undo, redo, canUndo, canRedo } = useUndoRedo();

  return (
    <div className="flex items-center gap-1">
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            onClick={undo}
            disabled={!canUndo}
            className="h-8 w-8"
          >
            <Undo2 className="h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          <p>撤销 (Ctrl+Z)</p>
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            onClick={redo}
            disabled={!canRedo}
            className="h-8 w-8"
          >
            <Redo2 className="h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          <p>重做 (Ctrl+Y)</p>
        </TooltipContent>
      </Tooltip>
    </div>
  );
}
```

### 3.5 归档时机

在以下时机调用 `archive()` 创建历史记录点：

| 时机 | 文件位置 | 说明 |
|------|----------|------|
| 节点拖拽结束 | `FlowCanvas.tsx` → `onNodeDragStop` | 合并连续拖拽 |
| 节点添加 | `FlowCanvas.tsx` → `onDrop` | 新节点创建 |
| 节点删除 | `useKeyboardShortcuts.ts` | Delete 键删除 |
| 连接创建 | `FlowCanvas.tsx` → `onConnect` | 连线完成 |
| 连接删除 | `FlowCanvas.tsx` → `handleEdgesChange` | 边删除 |
| 节点配置更新 | `NodeConfigPanel.tsx` | 配置保存时 |

```typescript
// FlowCanvas.tsx 示例
const onNodeDragStop = useCallback((_: unknown, node: Node) => {
  updateNode(node.id, { position: node.position });
  // 拖拽结束后归档
  const controls = useFlowStore.getControls();
  if ('archive' in controls) {
    (controls as any).archive();
  }
}, [updateNode]);
```

---

## 4. 实现步骤

### Phase 1: 基础实现 (2-3 天)

1. **安装依赖**
   ```bash
   npm install zustand-travel
   ```

2. **改造 flowStore**
   - 引入 `travel` 中间件
   - 将所有 `set()` 改为 mutation 风格
   - 配置 `autoArchive: false`

3. **创建 useUndoRedo Hook**
   - 实现 undo/redo 逻辑
   - 处理 React Flow 状态同步

4. **添加快捷键支持**
   - 修改 `useKeyboardShortcuts.ts`
   - 处理输入框冲突

### Phase 2: UI 集成 (1-2 天)

1. **创建撤销按钮组件**
   - 工具栏位置
   - 禁用状态处理

2. **集成到现有工具栏**
   - 修改 `Header.tsx` 或工具栏组件

3. **添加归档调用**
   - 在各操作结束点调用 `archive()`

### Phase 3: 测试与优化 (1 天)

1. **单元测试**
   - 测试 undo/redo 逻辑
   - 测试边界情况

2. **集成测试**
   - 测试 React Flow 同步
   - 测试快捷键

3. **性能优化**
   - 验证内存占用
   - 调整 `maxHistory`

---

## 5. 注意事项

### 5.1 React Flow 状态同步

撤销操作会直接修改 Zustand store，需要手动同步到 React Flow：

```typescript
// 方案 1: 在 undo/redo 后强制刷新
const undo = () => {
  controls.back();
  // 触发 React Flow 重新渲染
  const flow = useFlowStore.getState().flow;
  setNodes(convertNodes(flow.nodes));
  setEdges(convertEdges(flow.connections));
};

// 方案 2: 修改 useEffect 依赖，监听 flow 变化
useEffect(() => {
  if (flow) {
    setNodes(convertNodes(flow.nodes));
    setEdges(convertEdges(flow.connections));
  }
}, [flow, setNodes, setEdges]);
```

### 5.2 输入框快捷键冲突

在输入框中 `Ctrl+Z` 应该是原生输入撤销，不应被全局拦截：

```typescript
const isInputFocused = 
  activeElement instanceof HTMLInputElement ||
  activeElement instanceof HTMLTextAreaElement ||
  activeElement?.hasAttribute('contenteditable');

if (isInputFocused) return;
```

### 5.3 批量操作合并

节点拖拽过程中会触发大量 `updateNode`，应在 `onNodeDragStop` 时才归档：

```typescript
// ❌ 错误：每次位置变化都记录
updateNode: (uuid, updates) => {
  set(...);
  archive(); // 会导致历史栈爆炸
},

// ✅ 正确：在适当时机手动归档
onNodeDragStop: () => {
  updateNode(...);
  archive(); // 只在拖拽结束时记录
},
```

### 5.4 内存管理

- 设置合理的 `maxHistory` (建议 30-50)
- 定期清理历史栈（流程切换时调用 `reset()`）

---

## 6. 测试用例

### 6.1 基础功能

- [ ] 添加节点后撤销，节点消失
- [ ] 删除节点后撤销，节点恢复
- [ ] 移动节点后撤销，位置恢复
- [ ] 创建连接后撤销，连接消失
- [ ] 删除连接后撤销，连接恢复
- [ ] 连续撤销后重做，状态正确

### 6.2 边界情况

- [ ] 无历史时撤销按钮禁用
- [ ] 无 future 时重做按钮禁用
- [ ] 输入框中 Ctrl+Z 不触发全局撤销
- [ ] 入口节点不可删除（现有逻辑不受影响）
- [ ] 撤销后 isDirty 状态正确

### 6.3 性能

- [ ] 100 节点流程撤销响应时间 < 100ms
- [ ] 历史栈内存占用在合理范围

---

## 7. 参考资料

- [zustand-travel GitHub](https://github.com/mutativejs/zustand-travel)
- [zundo GitHub](https://github.com/charkour/zundo)
- [React Flow Undo/Redo Example](https://reactflow.dev/examples/interaction/undo-redo) (Pro)
- [JSON Patch RFC 6902](https://datatracker.ietf.org/doc/html/rfc6902)

---

## 8. 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-03-02 | v0.1 | 初始设计文档 |
