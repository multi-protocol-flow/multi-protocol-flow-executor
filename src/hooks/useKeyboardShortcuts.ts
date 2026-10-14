/**
 * useKeyboardShortcuts - 键盘快捷键 Hook
 * 提供全局键盘快捷键支持
 */
import { useEffect, useCallback } from 'react';
import { toast } from 'sonner';
import { useFlowStore, useUIStore, useAIStore, getStageGraph } from '@/stores';
import { flowStorage } from '@/services/storage';
import { useZoomStore } from '@/stores/zoom';
import { getLL } from '@/i18n/i18n-lazy';
import { isEntryNodeType } from '@/types';

function getFlowStoreState() {
  return typeof useFlowStore.getState === 'function'
    ? useFlowStore.getState()
    : (useFlowStore as unknown as () => ReturnType<typeof useFlowStore.getState>)();
}

function getUIStoreState() {
  return typeof useUIStore.getState === 'function'
    ? useUIStore.getState()
    : (useUIStore as unknown as () => ReturnType<typeof useUIStore.getState>)();
}

/**
 * 键盘快捷键 Hook
 * 
 * 支持的快捷键：
 * - Delete / Backspace: 删除选中的节点（带确认弹窗）或选中的边
 * - Escape: 取消选择节点
 * - Ctrl/Cmd + S: 保存当前流程
 * - Ctrl/Cmd + I: 唤起/切换 AI Copilot 侧边栏
 * - Ctrl/Cmd + K: 唤起 AI Spotlight 快捷指令弹窗
 * - Ctrl/Cmd + B: 切换左侧面板
 * - Ctrl/Cmd + H: 切换历史面板
 * - Ctrl/Cmd + J: 切换配置面板
 * - Ctrl/Cmd + Z: 撤销
 * - Ctrl/Cmd + Shift + Z / Ctrl/Cmd + Y: 重做
 * - Ctrl/Cmd + =: 放大
 * - Ctrl/Cmd + -: 缩小
 * - Ctrl/Cmd + 0: 重置缩放
 */
export function useKeyboardShortcuts() {
  const ll = getLL();
  /**
   * Delete / Backspace 处理（capture 阶段）
   * 在 ReactFlow 处理之前拦截，实现节点删除确认弹窗
   */
  const handleDeleteKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key !== 'Delete' && e.key !== 'Backspace') return;

      // 忽略在输入框中的按键
      const target = e.target as HTMLElement;
      const isInputting =
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable;
      if (isInputting) return;

      // 检查是否有选中的可删除节点（单例入口节点 entry / http:ingress 仅在存在其他可用入口时允许删除）
      const flowState = getFlowStoreState();
      const selectedNode = flowState.getSelectedNode();
      if (!selectedNode) return;
      const stageNodes = flowState.flow ? getStageGraph(flowState.flow, flowState.activeStage).nodes : [];
      const hasAlternativeEntry = stageNodes.some(
        (n) => n.uuid !== selectedNode.uuid && isEntryNodeType(n.type)
      );
      const canDelete = !isEntryNodeType(selectedNode.type) || hasAlternativeEntry;

      if (canDelete) {
        // 拦截事件，阻止 ReactFlow 的默认删除行为
        e.preventDefault();
        e.stopPropagation();

        // 弹出确认对话框，与 NodeConfigPanel 的删除按钮行为一致
        const { showConfirm } = getUIStoreState();
        const LL = getLL();
        showConfirm({
          title: LL.flow.delete_node_title(),
          description: LL.flow.delete_node_desc({ name: selectedNode.name }),
          confirmText: LL.flow.delete_node_confirm(),
          onConfirm: () => {
            flowState.removeNode(selectedNode.uuid);
            getFlowStoreState().selectNode(null);
          },
        });
      }
      // 无节点选中时，不拦截，让 ReactFlow 处理边的删除
    },
    []
  );

  const handleKeyDown = useCallback(
    async (e: KeyboardEvent) => {
      // 忽略在输入框中的按键
      const target = e.target as HTMLElement;
      const isInputting =
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable;

      // Escape: 取消选择
      if (e.key === 'Escape') {
        const { selectedNodeId, selectNode } = getFlowStoreState();
        if (selectedNodeId) {
          selectNode(null);
        }
      }
      // Ctrl/Cmd + K: 切换 AI Spotlight 快捷指令弹窗
      if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K' || e.code === 'KeyK')) {
        e.preventDefault();
        useAIStore.getState().toggleSpotlight();
        return;
      }

      // Ctrl/Cmd + I: 切换 AI Copilot 侧边栏
      if ((e.ctrlKey || e.metaKey) && (e.key === 'i' || e.key === 'I' || e.code === 'KeyI')) {
        e.preventDefault();
        useAIStore.getState().toggleCopilot();
        return;
      }

      // 快捷键：面板切换（仅在非输入状态）
      if (isInputting) return;
      // Ctrl/Cmd + Shift + Z / Ctrl/Cmd + Y: 重做
      if ((e.ctrlKey || e.metaKey) && (e.key === 'Z' || e.key === 'z') && e.shiftKey) {
        e.preventDefault();
        getFlowStoreState().redo();
        return;
      }

      // Ctrl/Cmd + Y: 重做
      if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || e.key === 'Y')) {
        e.preventDefault();
        getFlowStoreState().redo();
        return;
      }

      // Ctrl/Cmd + Z: 撤销
      if ((e.ctrlKey || e.metaKey) && (e.key === 'z' || e.key === 'Z') && !e.shiftKey) {
        e.preventDefault();
        getFlowStoreState().undo();
        return;
      }

      // Ctrl/Cmd + S: 保存当前流程
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        
        // 先刷新配置面板的表单数据到 store
        window.dispatchEvent(new CustomEvent('flush-config-panel'));
        
        // 等待一个微任务，让面板的 saveData -> handleSave -> updateNode 完成同步更新
        await new Promise(resolve => setTimeout(resolve, 0));
        
        const { flow, isDirty, markSaved } = getFlowStoreState();
        if (!flow || !isDirty) {
          toast.info(ll.execution.toast.no_changes(), { description: ll.execution.toast.no_unsaved_changes() });
          return;
        }
        try {
          await flowStorage.saveFlow(flow);
          markSaved();
          toast.success(ll.execution.toast.save_success(), { description: flow.name });
        } catch (err) {
          toast.error(ll.execution.toast.save_failed(), { description: (err as Error).message });
        }
        return;
      }

      // Ctrl/Cmd + B: 切换左侧面板
      if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault();
        getUIStoreState().toggleFlowListPanel();
      }

      // Ctrl/Cmd + H: 切换底部面板展开/收起
      if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
        e.preventDefault();
        getUIStoreState().toggleBottomPanel();
      }

      // Ctrl/Cmd + J: 切换配置面板
      if ((e.ctrlKey || e.metaKey) && e.key === 'j') {
        e.preventDefault();
        getUIStoreState().toggleConfigPanel();
      }

      // Ctrl/Cmd + =/+: 放大
      if ((e.ctrlKey || e.metaKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        useZoomStore.getState().zoomIn();
      }

      // Ctrl/Cmd + -: 缩小
      if ((e.ctrlKey || e.metaKey) && e.key === '-') {
        e.preventDefault();
        useZoomStore.getState().zoomOut();
      }

      // Ctrl/Cmd + 0: 重置缩放
      if ((e.ctrlKey || e.metaKey) && e.key === '0') {
        e.preventDefault();
        useZoomStore.getState().resetZoom();
      }

      // Ctrl/Cmd + /: 打开快捷键面板
      if ((e.ctrlKey || e.metaKey) && e.key === '/') {
        e.preventDefault();
        getUIStoreState().openShortcutsDialog();
      }

      // Ctrl/Cmd + Shift + F: 打开全流程搜索
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'f' || e.key === 'F')) {
        e.preventDefault();
        getUIStoreState().openSearchOverlay();
      }

      // Ctrl/Cmd + ,: 打开设置
      if ((e.ctrlKey || e.metaKey) && e.key === ',') {
        e.preventDefault();
        getUIStoreState().openSettingsDialog();
      }

      // Ctrl/Cmd + Enter: 执行流程
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        // 派发自定义事件，由 CanvasToolbar 监听
        window.dispatchEvent(new CustomEvent('trigger-execute-flow'));
      }

      // Ctrl/Cmd + D: 复制选中节点
      if ((e.ctrlKey || e.metaKey) && e.key === 'd') {
        e.preventDefault();
        const state = getFlowStoreState();
        const selectedNode = state.getSelectedNode();
        if (selectedNode && selectedNode.type !== 'entry') {
          const newUuid = crypto.randomUUID();
          const offset = 50;
          state.addNode({
            uuid: newUuid,
            type: selectedNode.type,
            name: `${selectedNode.name}${ll.common.node_copy_suffix()}`,
            position: {
              x: selectedNode.position.x + offset,
              y: selectedNode.position.y + offset,
            },
            data: JSON.parse(JSON.stringify(selectedNode.data)),
          });
          state.selectNode(newUuid);
        }
      }

      // Ctrl/Cmd + L: 聚焦到节点列表
      if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
        e.preventDefault();
        getUIStoreState().setFlowListPanelOpen(true);
      }

      // F: 适应画布到窗口（非输入状态）
      if (!isInputting && e.key === 'f') {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent('trigger-fit-view'));
      }
    },
    []
  );

  // Delete/Backspace: capture 阶段拦截，优先于 ReactFlow 处理
  useEffect(() => {
    window.addEventListener('keydown', handleDeleteKeyDown, true);
    return () => window.removeEventListener('keydown', handleDeleteKeyDown, true);
  }, [handleDeleteKeyDown]);

  // 其他快捷键: bubble 阶段处理
  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}
