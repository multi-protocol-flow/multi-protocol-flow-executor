/**
 * useKeyboardShortcuts - 键盘快捷键 Hook
 * 提供全局键盘快捷键支持
 */
import { useEffect, useCallback } from 'react';
import { toast } from 'sonner';
import { useFlowStore, useUIStore } from '@/stores';
import { flowStorage } from '@/services/storage';
import { useZoomStore } from '@/stores/zoom';

/**
 * 键盘快捷键 Hook
 * 
 * 支持的快捷键：
 * - Delete / Backspace: 删除选中的节点（带确认弹窗）或选中的边
 * - Escape: 取消选择节点
 * - Ctrl/Cmd + S: 保存当前流程
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
  const { selectedNodeId, selectNode } = useFlowStore();
  const { toggleFlowListPanel, toggleHistoryPanel, toggleConfigPanel } = useUIStore();

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

      // 检查是否有选中的非 entry 节点
      const { getSelectedNode, removeNode } = useFlowStore.getState();
      const selectedNode = getSelectedNode();

      if (selectedNode && selectedNode.type !== 'entry') {
        // 拦截事件，阻止 ReactFlow 的默认删除行为
        e.preventDefault();
        e.stopPropagation();

        // 弹出确认对话框，与 NodeConfigPanel 的删除按钮行为一致
        const { showConfirm } = useUIStore.getState();
        showConfirm({
          title: '删除节点',
          description: `确定要删除节点「${selectedNode.name}」吗？`,
          confirmText: '删除',
          onConfirm: () => {
            removeNode(selectedNode.uuid);
            selectNode(null);
          },
        });
      }
      // 无节点选中时，不拦截，让 ReactFlow 处理边的删除
    },
    [selectNode]
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
        if (selectedNodeId) {
          selectNode(null);
        }
      }

      // 快捷键：面板切换（仅在非输入状态）
      if (isInputting) return;

      // Ctrl/Cmd + Shift + Z / Ctrl/Cmd + Y: 重做
      if ((e.ctrlKey || e.metaKey) && (e.key === 'Z' || e.key === 'z') && e.shiftKey) {
        e.preventDefault();
        useFlowStore.getState().redo();
        return;
      }

      // Ctrl/Cmd + Y: 重做
      if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || e.key === 'Y')) {
        e.preventDefault();
        useFlowStore.getState().redo();
        return;
      }

      // Ctrl/Cmd + Z: 撤销
      if ((e.ctrlKey || e.metaKey) && (e.key === 'z' || e.key === 'Z') && !e.shiftKey) {
        e.preventDefault();
        useFlowStore.getState().undo();
        return;
      }

      // Ctrl/Cmd + S: 保存当前流程
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        const { flow, isDirty, markSaved } = useFlowStore.getState();
        if (!flow || !isDirty) {
          toast.info('无需保存', { description: '流程没有未保存的修改' });
          return;
        }
        try {
          await flowStorage.saveFlow(flow);
          markSaved();
          toast.success('保存成功', { description: flow.name });
        } catch (err) {
          toast.error('保存失败', { description: (err as Error).message });
        }
        return;
      }

      // Ctrl/Cmd + B: 切换左侧面板
      if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault();
        toggleFlowListPanel();
      }

      // Ctrl/Cmd + H: 切换历史面板
      if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
        e.preventDefault();
        toggleHistoryPanel();
      }

      // Ctrl/Cmd + J: 切换配置面板
      if ((e.ctrlKey || e.metaKey) && e.key === 'j') {
        e.preventDefault();
        toggleConfigPanel();
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
    },
    [selectedNodeId, selectNode, toggleFlowListPanel, toggleHistoryPanel, toggleConfigPanel]
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
