/**
 * layoutTransition 测试
 *
 * 覆盖可观察契约：
 * - 非过渡期 ResizeObserver 回调立即透传；
 * - 过渡期内多次尺寸变化被压缩为结束后的单次刷新，且携带最后一次条目；
 * - 时长驱动的过渡在到期后自动刷新；
 * - 观察者 disconnect 后不再执行挂起回调；
 * - 单个回调抛错不影响同批次其余回调。
 */
import { describe, it, expect, vi, beforeEach, afterEach, beforeAll, afterAll } from 'vitest';
import {
  beginLayoutTransition,
  endLayoutTransition,
  startLayoutTransition,
  initLayoutTransitionObserver,
} from '@/lib/layoutTransition';

/** 可手动驱动的原生 ResizeObserver 替身 */
class FakeResizeObserver {
  static instances: FakeResizeObserver[] = [];
  private readonly callback: ResizeObserverCallback;

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
    FakeResizeObserver.instances.push(this);
  }

  observe() {}
  unobserve() {}
  disconnect() {}

  emit(entries: ResizeObserverEntry[]) {
    this.callback(entries, this as unknown as ResizeObserver);
  }
}

/** 构造最小可用条目（仅测试回调透传，不参与真实布局计算） */
function entryOf(name: string): ResizeObserverEntry {
  return { target: document.body, name } as unknown as ResizeObserverEntry;
}

const originalResizeObserver = window.ResizeObserver;

/** 创建已接入保护层的观察者，返回回调 spy、patched 实例与其底层驱动实例 */
function createObserver() {
  const callback = vi.fn<ResizeObserverCallback>();
  const observer = new window.ResizeObserver(callback);
  const driver = FakeResizeObserver.instances[FakeResizeObserver.instances.length - 1];
  return { callback, driver, observer };
}

describe('layoutTransition', () => {
  beforeAll(() => {
    window.ResizeObserver = FakeResizeObserver as unknown as typeof ResizeObserver;
    initLayoutTransitionObserver();
  });

  afterAll(() => {
    window.ResizeObserver = originalResizeObserver;
  });

  beforeEach(() => {
    FakeResizeObserver.instances = [];
    endLayoutTransition();
  });

  it('非过渡期回调立即透传', () => {
    const { callback, driver } = createObserver();

    driver.emit([entryOf('a')]);

    expect(callback).toHaveBeenCalledTimes(1);
  });

  it('过渡期内多次尺寸变化只刷新一次，并携带最后一次条目', () => {
    const { callback, driver } = createObserver();

    beginLayoutTransition();
    driver.emit([entryOf('a')]);
    driver.emit([entryOf('b')]);
    expect(callback).not.toHaveBeenCalled();

    endLayoutTransition();

    expect(callback).toHaveBeenCalledTimes(1);
    expect(callback.mock.calls[0][0]).toEqual([entryOf('b')]);
  });

  it('时长驱动的过渡在结束后自动刷新一次', () => {
    vi.useFakeTimers();
    try {
      const { callback, driver } = createObserver();

      startLayoutTransition(220);
      driver.emit([entryOf('a')]);
      driver.emit([entryOf('b')]);
      expect(callback).not.toHaveBeenCalled();

      vi.advanceTimersByTime(220);

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback.mock.calls[0][0]).toEqual([entryOf('b')]);
    } finally {
      vi.useRealTimers();
    }
  });

  it('连续调用 startLayoutTransition 重置计时，只刷新一次', () => {
    vi.useFakeTimers();
    try {
      const { callback, driver } = createObserver();

      startLayoutTransition(220);
      vi.advanceTimersByTime(100);
      driver.emit([entryOf('a')]);
      startLayoutTransition(220);
      vi.advanceTimersByTime(100);
      expect(callback).not.toHaveBeenCalled();

      vi.advanceTimersByTime(120);

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback.mock.calls[0][0]).toEqual([entryOf('a')]);
    } finally {
      vi.useRealTimers();
    }
  });

  it('disconnect 后挂起的回调不再执行', () => {
    const { callback, driver, observer } = createObserver();

    beginLayoutTransition();
    driver.emit([entryOf('a')]);
    observer.disconnect();
    endLayoutTransition();

    expect(callback).not.toHaveBeenCalled();
  });

  it('单个回调抛错不影响同批次其余回调', () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { callback: failing, driver: failingDriver } = createObserver();
    const { callback: healthy, driver: healthyDriver } = createObserver();
    failing.mockImplementation(() => {
      throw new Error('boom');
    });

    beginLayoutTransition();
    failingDriver.emit([entryOf('a')]);
    healthyDriver.emit([entryOf('b')]);
    endLayoutTransition();

    expect(healthy).toHaveBeenCalledTimes(1);
    expect(errorSpy).toHaveBeenCalled();
    errorSpy.mockRestore();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });
});
