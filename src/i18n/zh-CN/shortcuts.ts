import type { Translation } from '../i18n-types';

const shortcuts: Translation['shortcuts'] = {
  title: '快捷键',
  searchPlaceholder: '搜索快捷键...',
  noResults: '未找到匹配的快捷键',
  hint: '按 Esc 关闭 · 快捷键使用 Ctrl 修饰（Mac 上为 ⌘）',
  group: {
    edit: '编辑',
    execute: '执行',
    canvas: '画布',
  },
  undo: '撤销',
  redo: '重做',
  save: '保存当前流程',
  duplicateNode: '复制选中节点',
  delete: '删除选中节点/边',
  selectAll: '全选节点',
  executeFlow: '执行流程',
  togglePanelLeft: '切换左侧面板',
  togglePanelHistory: '切换历史面板',
  togglePanelConfig: '切换配置面板',
  openSettings: '打开设置',
  panCanvas: '平移画布',
  zoomCanvas: '缩放画布',
  resetZoom: '重置缩放',
  zoomInOut: '缩小/放大',
  fitView: '适应画布到窗口',
  focusNodeList: '聚焦到节点列表',
  openShortcuts: '打开快捷键面板',
};

export default shortcuts;
