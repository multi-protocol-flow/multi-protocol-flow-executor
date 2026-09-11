/**
 * @vitest-environment jsdom
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { EmptyCanvasTips, CANVAS_TIPS } from '@/components/canvas/EmptyCanvasTips';
import { switchLocale } from '@/i18n';
import { getLL } from '@/i18n/i18n-lazy';

describe('EmptyCanvasTips 组件', () => {
  beforeEach(() => {
    switchLocale('zh-CN');
    vi.useRealTimers();
  });

  afterEach(() => {
    switchLocale('zh-CN');
    vi.useRealTimers();
  });

  it('默认渲染一个随机提示且包含提示徽章', () => {
    render(<EmptyCanvasTips autoRotateInterval={0} />);
    const container = screen.getByTestId('empty-canvas-tips');
    expect(container).toBeDefined();

    const btn = screen.getByTestId('empty-canvas-tips-btn');
    expect(btn).toBeDefined();
    expect(btn.textContent).toContain('小技巧:');
  });

  it('指定 initialIndex 时渲染对应的提示 (zh-CN)', () => {
    // 索引 0: right_click
    render(<EmptyCanvasTips initialIndex={0} autoRotateInterval={0} />);
    const btn = screen.getByTestId('empty-canvas-tips-btn');
    expect(btn.textContent).toContain('右键');
    expect(btn.textContent).toContain('画布空白处打开节点选择器');
  });

  it('支持多语言切换 (en-US)', () => {
    switchLocale('en-US');
    render(<EmptyCanvasTips initialIndex={0} autoRotateInterval={0} />);
    const btn = screen.getByTestId('empty-canvas-tips-btn');
    expect(btn.textContent).toContain('Tip:');
    expect(btn.textContent).toContain('Right-click');
    expect(btn.textContent).toContain('canvas to open node selector');
  });

  it('AI 快捷指令在不同语言下的显示', () => {
    // 索引 1: spotlight
    const { unmount } = render(<EmptyCanvasTips initialIndex={1} autoRotateInterval={0} />);
    expect(screen.getByTestId('empty-canvas-tips-btn').textContent).toContain('唤起 AI 快捷指令弹窗');
    unmount();

    switchLocale('en-US');
    render(<EmptyCanvasTips initialIndex={1} autoRotateInterval={0} />);
    expect(screen.getByTestId('empty-canvas-tips-btn').textContent).toContain('open AI Spotlight command palette');
  });

  it('点击按钮切换到另一个不同的提示', () => {
    render(<EmptyCanvasTips initialIndex={0} autoRotateInterval={0} />);
    const btn = screen.getByTestId('empty-canvas-tips-btn');
    const initialText = btn.textContent;

    fireEvent.click(btn);

    const newText = screen.getByTestId('empty-canvas-tips-btn').textContent;
    expect(newText).not.toEqual(initialText);
  });

  it('定时自动轮播提示', () => {
    vi.useFakeTimers();
    render(<EmptyCanvasTips initialIndex={0} autoRotateInterval={5000} />);
    const btn = screen.getByTestId('empty-canvas-tips-btn');
    const initialText = btn.textContent;

    act(() => {
      vi.advanceTimersByTime(5000);
    });

    const newText = screen.getByTestId('empty-canvas-tips-btn').textContent;
    expect(newText).not.toEqual(initialText);
  });

  it('鼠标悬停时暂停轮播，移开后恢复', () => {
    vi.useFakeTimers();
    render(<EmptyCanvasTips initialIndex={0} autoRotateInterval={5000} />);
    const btn = screen.getByTestId('empty-canvas-tips-btn');
    const initialText = btn.textContent;

    // 悬停
    fireEvent.mouseEnter(btn);

    act(() => {
      vi.advanceTimersByTime(10000);
    });

    // 悬停中不应轮播
    expect(btn.textContent).toEqual(initialText);

    // 移开
    fireEvent.mouseLeave(btn);

    act(() => {
      vi.advanceTimersByTime(5000);
    });

    // 移开后恢复轮播
    expect(screen.getByTestId('empty-canvas-tips-btn').textContent).not.toEqual(initialText);
  });

  it('验证所有预置提示在 zh-CN 与 en-US 下均有非空按键与描述', () => {
    expect(CANVAS_TIPS.length).toBeGreaterThanOrEqual(15);

    // 检查 zh-CN
    switchLocale('zh-CN');
    const zhTips = getLL().welcome.empty_tips;
    expect(zhTips.tip_label()).toBe('小技巧');
    for (const tip of CANVAS_TIPS) {
      const keys = typeof tip.keys === 'function' ? tip.keys(zhTips) : tip.keys;
      const desc = tip.getDesc(zhTips);
      expect(keys.length).toBeGreaterThan(0);
      for (const k of keys) {
        expect(k).toBeTruthy();
      }
      expect(desc).toBeTruthy();
    }

    // 检查 en-US
    switchLocale('en-US');
    const enTips = getLL().welcome.empty_tips;
    expect(enTips.tip_label()).toBe('Tip');
    for (const tip of CANVAS_TIPS) {
      const keys = typeof tip.keys === 'function' ? tip.keys(enTips) : tip.keys;
      const desc = tip.getDesc(enTips);
      expect(keys.length).toBeGreaterThan(0);
      for (const k of keys) {
        expect(k).toBeTruthy();
      }
      expect(desc).toBeTruthy();
    }
  });
});
