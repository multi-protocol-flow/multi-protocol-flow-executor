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
 * - Delete / Backspace: 删除选中的节点（不能删除 Entry 节点）
 * - Escape: 取消选择节点
 */
export function useKeyboardShortcuts() {
  const { selectedNodeId, removeNode, flow, selectNode } = useFlowStore();
  const { toggleFlowListPanel, toggleHistoryPanel, toggleConfigPanel } = useUIStore();

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // 忽略在输入框中的按键
      const target = e.target as HTMLElement;
      const isInputting =
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable;

      // Delete / Backspace: 删除选中节点
      if ((e.key === 'Delete' || e.key === 'Backspace') && selectedNodeId) {
        if (isInputting) return;

        const node = flow?.nodes.find((n) => n.uuid === selectedNodeId);
        // 不能删除 Entry 节点
        if (node && node.type !== 'entry') {
          e.preventDefault();
          removeNode(selectedNodeId);
        }
      }

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
    [selectedNodeId, removeNode, flow, selectNode, toggleFlowListPanel, toggleHistoryPanel, toggleConfigPanel]
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}
