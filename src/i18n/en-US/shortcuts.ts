import type { Translation } from '../i18n-types';

const shortcuts: Translation['shortcuts'] = {
  title: 'Keyboard Shortcuts',
  searchPlaceholder: 'Search shortcuts...',
  noResults: 'No matching shortcuts found',
  hint: 'Press Esc to close · Ctrl modifier (⌘ on Mac)',
  group: {
    edit: 'Edit',
    execute: 'Execution & Panels',
    ai: 'AI Assistant',
    canvas: 'Canvas',
  },
  undo: 'Undo',
  redo: 'Redo',
  save: 'Save current flow',
  duplicateNode: 'Duplicate selected node',
  delete: 'Delete selected node/edge',
  selectAll: 'Select all nodes',
  executeFlow: 'Execute flow',
  togglePanelLeft: 'Toggle left panel',
  togglePanelHistory: 'Toggle history panel',
  togglePanelConfig: 'Toggle config panel',
  openSettings: 'Open settings',
  panCanvas: 'Pan canvas',
  zoomCanvas: 'Zoom canvas',
  resetZoom: 'Reset zoom',
  zoomInOut: 'Zoom in/out',
  fitView: 'Fit canvas to window',
  focusNodeList: 'Focus on node list',
  openShortcuts: 'Open shortcuts panel',
  toggleCopilot: 'Toggle AI Copilot sidebar',
  toggleSpotlight: 'Toggle AI Spotlight modal',
  searchOverlay: 'Global search',
};

export default shortcuts;
