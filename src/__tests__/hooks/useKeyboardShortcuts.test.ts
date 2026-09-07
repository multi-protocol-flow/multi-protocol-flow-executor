/**
 * useKeyboardShortcuts Hook 测试
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useKeyboardShortcuts } from '@/hooks/useKeyboardShortcuts';
import * as stores from '@/stores';

// Mock stores
const mockSelectNode = vi.fn();
const mockToggleFlowListPanel = vi.fn();
const mockToggleBottomPanel = vi.fn();
const mockToggleConfigPanel = vi.fn();
const mockToggleCopilot = vi.fn();
const mockToggleSpotlight = vi.fn();

vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(() => ({
    selectedNodeId: null,
    selectNode: mockSelectNode,
  })),
  useUIStore: vi.fn(() => ({
    toggleFlowListPanel: mockToggleFlowListPanel,
    toggleBottomPanel: mockToggleBottomPanel,
    toggleConfigPanel: mockToggleConfigPanel,
  })),
  useAIStore: {
    getState: vi.fn(() => ({
      toggleCopilot: mockToggleCopilot,
      toggleSpotlight: mockToggleSpotlight,
    })),
  },
}));

// Mock flowStorage (hook imports it for Ctrl+S)
vi.mock('@/services/storage', () => ({
  flowStorage: {
    saveFlow: vi.fn().mockResolvedValue(undefined),
  },
}));

// Mock zoom store (hook imports it for zoom shortcuts)
vi.mock('@/stores/zoom', () => ({
  useZoomStore: {
    getState: vi.fn(() => ({
      zoomIn: vi.fn(),
      zoomOut: vi.fn(),
      resetZoom: vi.fn(),
    })),
  },
}));

// Mock sonner (hook uses toast)
vi.mock('sonner', () => ({
  toast: { info: vi.fn(), success: vi.fn(), error: vi.fn() },
}));

describe('useKeyboardShortcuts', () => {
  let addEventListenerSpy: ReturnType<typeof vi.spyOn>;
  let removeEventListenerSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    addEventListenerSpy = vi.spyOn(window, 'addEventListener');
    removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('事件监听器', () => {
    it('挂载时添加 keydown 事件监听', () => {
      renderHook(() => useKeyboardShortcuts());

      expect(addEventListenerSpy).toHaveBeenCalledWith(
        'keydown',
        expect.any(Function)
      );
    });

    it('卸载时移除 keydown 事件监听', () => {
      const { unmount } = renderHook(() => useKeyboardShortcuts());
      unmount();

      expect(removeEventListenerSpy).toHaveBeenCalledWith(
        'keydown',
        expect.any(Function)
      );
    });
  });

  describe('Escape 键', () => {
    it('有选中节点时按 Escape 调用 selectNode(null)', () => {
      // 重新 mock 以返回选中的节点
      vi.mocked(stores.useFlowStore).mockReturnValue({
        selectedNodeId: 'node-123',
        selectNode: mockSelectNode,
      } as ReturnType<typeof stores.useFlowStore>);

      renderHook(() => useKeyboardShortcuts());

      // 获取冒泡阶段的事件处理函数（第二个 keydown 监听器，非 capture）
      const bubbleHandler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      // 模拟 Escape 键，设置 target 为 document.body
      const event = new KeyboardEvent('keydown', { key: 'Escape' });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });
      bubbleHandler(event);

      expect(mockSelectNode).toHaveBeenCalledWith(null);
    });

    it('无选中节点时按 Escape 不调用 selectNode', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'Escape' });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });
      handler(event);

      expect(mockSelectNode).not.toHaveBeenCalled();
    });
  });

  describe('Ctrl+B 切换流程列表面板', () => {
    it('Ctrl+B 调用 toggleFlowListPanel', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'b', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleFlowListPanel).toHaveBeenCalledTimes(1);
    });

    it('Cmd+B (metaKey) 也调用 toggleFlowListPanel', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'b', metaKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleFlowListPanel).toHaveBeenCalledTimes(1);
    });

    it('Ctrl+B 调用 preventDefault', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'b', ctrlKey: true });
      const preventDefaultSpy = vi.spyOn(event, 'preventDefault');
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(preventDefaultSpy).toHaveBeenCalled();
    });
  });

  describe('Ctrl+H 切换底部面板', () => {
    it('Ctrl+H 调用 toggleBottomPanel', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'h', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleBottomPanel).toHaveBeenCalledTimes(1);
    });

    it('Cmd+H (metaKey) 也调用 toggleBottomPanel', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'h', metaKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleBottomPanel).toHaveBeenCalledTimes(1);
    });
  });

  describe('Ctrl+J 切换配置面板', () => {
    it('Ctrl+J 调用 toggleConfigPanel', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'j', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleConfigPanel).toHaveBeenCalledTimes(1);
    });

    it('Cmd+J (metaKey) 也调用 toggleConfigPanel', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'j', metaKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleConfigPanel).toHaveBeenCalledTimes(1);
    });
  });

  describe('Ctrl+I 唤起 AI Copilot', () => {
    it('Ctrl+I 调用 toggleCopilot', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'i', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleCopilot).toHaveBeenCalledTimes(1);
    });

    it('Cmd+I (metaKey) 也调用 toggleCopilot', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'i', metaKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleCopilot).toHaveBeenCalledTimes(1);
    });

    it('通过 e.code === KeyI 也能唤起（兼容输入法按键）', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'Process', code: 'KeyI', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleCopilot).toHaveBeenCalledTimes(1);
    });
  });

  describe('忽略输入状态', () => {
    it('在 INPUT 中不响应快捷键', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const inputElement = document.createElement('input');
      const event = new KeyboardEvent('keydown', { key: 'b', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: inputElement, writable: false });

      handler(event);

      expect(mockToggleFlowListPanel).not.toHaveBeenCalled();
    });

    it('在 TEXTAREA 中不响应快捷键', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const textareaElement = document.createElement('textarea');
      const event = new KeyboardEvent('keydown', { key: 'h', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: textareaElement, writable: false });

      handler(event);

      expect(mockToggleBottomPanel).not.toHaveBeenCalled();
    });

    it('在 isContentEditable 元素中不响应快捷键', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const editableElement = document.createElement('div');
      // 在 jsdom 中需要直接设置 isContentEditable 属性
      Object.defineProperty(editableElement, 'isContentEditable', { value: true });
      const event = new KeyboardEvent('keydown', { key: 'j', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: editableElement, writable: false });

      handler(event);

      expect(mockToggleConfigPanel).not.toHaveBeenCalled();
    });

    it('在输入状态时 Escape 仍然可以取消选择', () => {
      // 重新 mock 以返回选中的节点
      vi.mocked(stores.useFlowStore).mockReturnValue({
        selectedNodeId: 'node-456',
        selectNode: mockSelectNode,
      } as ReturnType<typeof stores.useFlowStore>);

      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const inputElement = document.createElement('input');
      const event = new KeyboardEvent('keydown', { key: 'Escape' });
      Object.defineProperty(event, 'target', { value: inputElement, writable: false });

      handler(event);

      expect(mockSelectNode).toHaveBeenCalledWith(null);
    });
  });

  describe('其他按键', () => {
    it('单独按 b 不触发任何操作', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'b' });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleFlowListPanel).not.toHaveBeenCalled();
    });

    it('按其他字母键不触发任何操作', () => {
      renderHook(() => useKeyboardShortcuts());

      const handler = addEventListenerSpy.mock.calls
        .filter((call) => call[0] === 'keydown' && !call[2])
        [0]?.[1] as EventListener;

      const event = new KeyboardEvent('keydown', { key: 'x', ctrlKey: true });
      Object.defineProperty(event, 'target', { value: document.body, writable: false });

      handler(event);

      expect(mockToggleFlowListPanel).not.toHaveBeenCalled();
      expect(mockToggleBottomPanel).not.toHaveBeenCalled();
      expect(mockToggleConfigPanel).not.toHaveBeenCalled();
    });
  });
});
