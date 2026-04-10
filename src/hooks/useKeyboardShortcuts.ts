/**
 * useKeyboardShortcuts - 键盘快捷键 Hook
 * 提供全局键盘快捷键支持
 */
import { useEffect, useCallback } from 'react';
import { useFlowStore, useUIStore } from '@/stores';

/**
 * 键盘快捷键 Hook
 * 
 * 支持的快捷键：
 * - Delete / Backspace: 删除选中的节点（带确认弹窗）或选中的边
 * - Escape: 取消选择节点
 * - Ctrl/Cmd + B: 切换左侧面板
 * - Ctrl/Cmd + H: 切换历史面板
 * - Ctrl/Cmd + J: 切换配置面板
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
    (e: KeyboardEvent) => {
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
