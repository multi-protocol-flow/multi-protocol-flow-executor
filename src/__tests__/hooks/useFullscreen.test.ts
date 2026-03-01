/**
 * useFullscreen Hook 测试
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFullscreen } from '@/hooks/useFullscreen';

describe('useFullscreen', () => {
  // 保存原始方法
  const originalFullscreenElement = Object.getOwnPropertyDescriptor(
    Document.prototype,
    'fullscreenElement'
  );
  const originalRequestFullscreen = Object.getOwnPropertyDescriptor(
    Element.prototype,
    'requestFullscreen'
  );
  const originalExitFullscreen = Object.getOwnPropertyDescriptor(
    Document.prototype,
    'exitFullscreen'
  );

  // Mock 函数
  let mockRequestFullscreen: ReturnType<typeof vi.fn>;
  let mockExitFullscreen: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    // 创建 mock 函数
    mockRequestFullscreen = vi.fn().mockResolvedValue(undefined);
    mockExitFullscreen = vi.fn().mockResolvedValue(undefined);

    // Mock document.fullscreenElement
    Object.defineProperty(document, 'fullscreenElement', {
      configurable: true,
      get: vi.fn(() => null),
    });

    // Mock document.documentElement.requestFullscreen
    Object.defineProperty(document.documentElement, 'requestFullscreen', {
      configurable: true,
      value: mockRequestFullscreen,
    });

    // Mock document.exitFullscreen
    Object.defineProperty(document, 'exitFullscreen', {
      configurable: true,
      value: mockExitFullscreen,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();

    // 恢复原始方法
    if (originalFullscreenElement) {
      Object.defineProperty(Document.prototype, 'fullscreenElement', originalFullscreenElement);
    }
    if (originalRequestFullscreen) {
      Object.defineProperty(Element.prototype, 'requestFullscreen', originalRequestFullscreen);
    }
    if (originalExitFullscreen) {
      Object.defineProperty(Document.prototype, 'exitFullscreen', originalExitFullscreen);
    }
  });

  describe('初始状态', () => {
    it('初始时 isFullscreen 为 false', () => {
      const { result } = renderHook(() => useFullscreen());
      expect(result.current.isFullscreen).toBe(false);
    });

    it('返回 toggleFullscreen 和 exitFullscreen 函数', () => {
      const { result } = renderHook(() => useFullscreen());
      expect(typeof result.current.toggleFullscreen).toBe('function');
      expect(typeof result.current.exitFullscreen).toBe('function');
    });
  });

  describe('toggleFullscreen', () => {
    it('非全屏时调用 requestFullscreen', async () => {
      const { result } = renderHook(() => useFullscreen());

      await act(async () => {
        result.current.toggleFullscreen();
      });

      expect(mockRequestFullscreen).toHaveBeenCalledTimes(1);
      expect(mockExitFullscreen).not.toHaveBeenCalled();
    });

    it('全屏时调用 exitFullscreen', async () => {
      // 模拟全屏状态
      Object.defineProperty(document, 'fullscreenElement', {
        configurable: true,
        get: vi.fn(() => document.documentElement),
      });

      const { result } = renderHook(() => useFullscreen());

      await act(async () => {
        result.current.toggleFullscreen();
      });

      expect(mockExitFullscreen).toHaveBeenCalledTimes(1);
      expect(mockRequestFullscreen).not.toHaveBeenCalled();
    });
  });

  describe('exitFullscreen', () => {
    it('非全屏时不调用 exitFullscreen', async () => {
      const { result } = renderHook(() => useFullscreen());

      await act(async () => {
        result.current.exitFullscreen();
      });

      expect(mockExitFullscreen).not.toHaveBeenCalled();
    });

    it('全屏时调用 exitFullscreen', async () => {
      // 模拟全屏状态
      Object.defineProperty(document, 'fullscreenElement', {
        configurable: true,
        get: vi.fn(() => document.documentElement),
      });

      const { result } = renderHook(() => useFullscreen());

      await act(async () => {
        result.current.exitFullscreen();
      });

      expect(mockExitFullscreen).toHaveBeenCalledTimes(1);
    });
  });

  describe('fullscreenchange 事件', () => {
    it('监听 fullscreenchange 事件', () => {
      const addEventListenerSpy = vi.spyOn(document, 'addEventListener');

      renderHook(() => useFullscreen());

      expect(addEventListenerSpy).toHaveBeenCalledWith(
        'fullscreenchange',
        expect.any(Function)
      );

      addEventListenerSpy.mockRestore();
    });

    it('组件卸载时移除事件监听', () => {
      const removeEventListenerSpy = vi.spyOn(document, 'removeEventListener');

      const { unmount } = renderHook(() => useFullscreen());
      unmount();

      expect(removeEventListenerSpy).toHaveBeenCalledWith(
        'fullscreenchange',
        expect.any(Function)
      );

      removeEventListenerSpy.mockRestore();
    });
  });

  describe('ESC 键退出全屏', () => {
    it('监听 keydown 事件', () => {
      const addEventListenerSpy = vi.spyOn(document, 'addEventListener');

      renderHook(() => useFullscreen());

      expect(addEventListenerSpy).toHaveBeenCalledWith(
        'keydown',
        expect.any(Function)
      );

      addEventListenerSpy.mockRestore();
    });

    it('组件卸载时移除 keydown 监听', () => {
      const removeEventListenerSpy = vi.spyOn(document, 'removeEventListener');

      const { unmount } = renderHook(() => useFullscreen());
      unmount();

      expect(removeEventListenerSpy).toHaveBeenCalledWith(
        'keydown',
        expect.any(Function)
      );

      removeEventListenerSpy.mockRestore();
    });
  });

  describe('使用自定义元素引用', () => {
    it('使用传入的元素引用进行全屏', async () => {
      const mockElement = {
        requestFullscreen: vi.fn().mockResolvedValue(undefined),
      } as unknown as HTMLElement;

      const mockRef = {
        current: mockElement,
      };

      const { result } = renderHook(() => useFullscreen(mockRef));

      await act(async () => {
        result.current.toggleFullscreen();
      });

      expect(mockElement.requestFullscreen).toHaveBeenCalledTimes(1);
      expect(mockRequestFullscreen).not.toHaveBeenCalled();
    });

    it('ref.current 为 null 时使用 document.documentElement', async () => {
      const mockRef = {
        current: null,
      };

      const { result } = renderHook(() => useFullscreen(mockRef));

      await act(async () => {
        result.current.toggleFullscreen();
      });

      expect(mockRequestFullscreen).toHaveBeenCalledTimes(1);
    });
  });
});
