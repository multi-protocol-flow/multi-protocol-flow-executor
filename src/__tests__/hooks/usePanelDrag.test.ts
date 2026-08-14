/**
 * usePanelDrag Hook 测试
 * 覆盖：初始状态、mouseDown 进入拖拽并设置 body 样式、
 * 拖拽移动按 sign 计算并 clamp 尺寸、vertical 轴取 clientY、
 * mouseUp 结束并清理、未拖拽不响应、卸载时移除全局监听
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { usePanelDrag } from '@/hooks/usePanelDrag';
import type { UsePanelDragOptions } from '@/hooks/usePanelDrag';

function mouseDownEvent(x: number, y: number): ReactMouseEvent {
  return {
    preventDefault: vi.fn(),
    clientX: x,
    clientY: y,
  } as unknown as ReactMouseEvent;
}

function baseOptions(overrides: Partial<UsePanelDragOptions> = {}): UsePanelDragOptions {
  return {
    axis: 'horizontal',
    min: 250,
    max: 500,
    sign: 1,
    initialSize: 300,
    onResize: vi.fn(),
    ...overrides,
  };
}

describe('usePanelDrag', () => {
  beforeEach(() => {
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  });

  it('初始状态未在拖拽', () => {
    const onResize = vi.fn();
    const { result } = renderHook(() => usePanelDrag(baseOptions({ onResize })));

    expect(result.current.isDragging).toBe(false);
    expect(document.body.style.cursor).toBe('');
    expect(onResize).not.toHaveBeenCalled();
  });

  it('mouseDown 进入拖拽状态并设置光标样式', () => {
    const { result } = renderHook(() => usePanelDrag(baseOptions()));

    act(() => {
      result.current.handleMouseDown(mouseDownEvent(150, 0));
    });

    expect(result.current.isDragging).toBe(true);
    expect(document.body.style.cursor).toBe('col-resize');
    expect(document.body.style.userSelect).toBe('none');
  });

  it('拖拽移动按 sign 计算新尺寸并 clamp 到 [min, max]', () => {
    const onResize = vi.fn();
    const { result } = renderHook(() => usePanelDrag(baseOptions({ onResize })));

    act(() => {
      result.current.handleMouseDown(mouseDownEvent(200, 0));
    });

    // 鼠标左移 40px → delta = 200 - 160 = 40 → 尺寸 340
    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 160 }));
    });
    expect(onResize).toHaveBeenLastCalledWith(340);

    // 超出上限：delta = 200 - (-1000) = 1200 → 1500 → clamp 到 500
    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: -1000 }));
    });
    expect(onResize).toHaveBeenLastCalledWith(500);

    // 超出下限：delta = 200 - 400 = -200 → 100 → clamp 到 250
    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 400 }));
    });
    expect(onResize).toHaveBeenLastCalledWith(250);
  });

  it('sign=-1 时鼠标右移使尺寸增大（方向反转）', () => {
    const onResize = vi.fn();
    const { result } = renderHook(() =>
      usePanelDrag(baseOptions({ sign: -1, onResize }))
    );

    act(() => {
      result.current.handleMouseDown(mouseDownEvent(200, 0));
    });

    // sign=-1：delta = -(200 - 240) = 40 → 尺寸 340
    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 240 }));
    });
    expect(onResize).toHaveBeenLastCalledWith(340);
  });

  it('vertical 轴使用 clientY 计算并设置行调整光标', () => {
    const onResize = vi.fn();
    const { result } = renderHook(() =>
      usePanelDrag(baseOptions({ axis: 'vertical', onResize }))
    );

    act(() => {
      result.current.handleMouseDown(mouseDownEvent(0, 100));
    });
    expect(document.body.style.cursor).toBe('row-resize');

    // 鼠标上移 40px → delta = 100 - 60 = 40 → 尺寸 340
    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientY: 60 }));
    });
    expect(onResize).toHaveBeenLastCalledWith(340);
  });

  it('mouseUp 结束拖拽并清理样式，后续移动不再触发 resize', () => {
    const onResize = vi.fn();
    const { result } = renderHook(() => usePanelDrag(baseOptions({ onResize })));

    act(() => {
      result.current.handleMouseDown(mouseDownEvent(200, 0));
    });
    act(() => {
      window.dispatchEvent(new MouseEvent('mouseup'));
    });

    expect(result.current.isDragging).toBe(false);
    expect(document.body.style.cursor).toBe('');
    expect(document.body.style.userSelect).toBe('');

    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 50 }));
    });
    expect(onResize).not.toHaveBeenCalled();
  });

  it('未开始拖拽时窗口移动事件不触发 resize', () => {
    const onResize = vi.fn();
    renderHook(() => usePanelDrag(baseOptions({ onResize })));

    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientX: 50 }));
    });
    expect(onResize).not.toHaveBeenCalled();
  });

  it('卸载时移除全局鼠标事件监听器', () => {
    const removeSpy = vi.spyOn(window, 'removeEventListener');
    const { result, unmount } = renderHook(() => usePanelDrag(baseOptions()));

    act(() => {
      result.current.handleMouseDown(mouseDownEvent(200, 0));
    });
    unmount();

    expect(removeSpy).toHaveBeenCalledWith('mousemove', expect.any(Function));
    expect(removeSpy).toHaveBeenCalledWith('mouseup', expect.any(Function));
    removeSpy.mockRestore();
  });
});
