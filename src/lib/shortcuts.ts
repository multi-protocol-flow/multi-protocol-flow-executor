/**
 * shortcuts - 快捷键数据定义与分组
 */
import { getLL } from '@/i18n/i18n-lazy';

/** 快捷键单项定义 */
export interface ShortcutItem {
  keys: string[];
  description: string;
}

/** 快捷键分组定义 */
export interface ShortcutGroup {
  group: string;
  shortcuts: ShortcutItem[];
}

/** 获取所有快捷键分组数据 */
export function getShortcutGroups(): ShortcutGroup[] {
  const ll = getLL().shortcuts;
  return [
    {
      group: ll.group.edit(),
      shortcuts: [
        { keys: ['Ctrl+Z'], description: ll.undo() },
        { keys: ['Ctrl+Shift+Z', 'Ctrl+Y'], description: ll.redo() },
        { keys: ['Ctrl+S'], description: ll.save() },
        { keys: ['Ctrl+D'], description: ll.duplicateNode() },
        { keys: ['Delete', 'Backspace'], description: ll.delete() },
        { keys: ['Ctrl+A'], description: ll.selectAll() },
      ],
    },
    {
      group: ll.group.execute(),
      shortcuts: [
        { keys: ['Ctrl+Enter'], description: ll.executeFlow() },
        { keys: ['Ctrl+B'], description: ll.togglePanelLeft() },
        { keys: ['Ctrl+H'], description: ll.togglePanelHistory() },
        { keys: ['Ctrl+J'], description: ll.togglePanelConfig() },
        { keys: ['Ctrl+Shift+F'], description: ll.searchOverlay() },
        { keys: ['Ctrl+,'], description: ll.openSettings() },
      ],
    },
    {
      group: ll.group.ai(),
      shortcuts: [
        { keys: ['Ctrl+I'], description: ll.toggleCopilot() },
        { keys: ['Ctrl+K'], description: ll.toggleSpotlight() },
      ],
    },
    {
      group: ll.group.canvas(),
      shortcuts: [
        { keys: [getLL().common.space_drag()], description: ll.panCanvas() },
        { keys: [getLL().common.ctrl_scroll()], description: ll.zoomCanvas() },
        { keys: ['Ctrl+0'], description: ll.resetZoom() },
        { keys: [getLL().common.ctrl_zoom()], description: ll.zoomInOut() },
        { keys: ['F'], description: ll.fitView() },
        { keys: ['Ctrl+L'], description: ll.focusNodeList() },
        { keys: ['Ctrl+/'], description: ll.openShortcuts() },
      ],
    },
  ];
}
