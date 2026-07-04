/**
 * 平台环境检测测试
 * 测试 isTauriEnv() 在 jsdom (浏览器) 环境中的行为
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { isTauriEnv } from '@/lib/platform';

describe('isTauriEnv', () => {
  const originalWindow = globalThis.window;

  beforeEach(() => {
    // 确保 __TAURI__ 不在 window 上 (jsdom 默认)
    delete (globalThis as Record<string, unknown>).__TAURI__;
  });

  afterEach(() => {
    // 清理，不影响其他测试
    delete (globalThis as Record<string, unknown>).__TAURI__;
  });

  it('在 jsdom（浏览器）环境中返回 false', () => {
    expect(isTauriEnv()).toBe(false);
  });

  it('当 window.__TAURI__ 存在时返回 true', () => {
    (globalThis as Record<string, unknown>).__TAURI__ = {};
    expect(isTauriEnv()).toBe(true);
  });

  it('当 window.__TAURI__ 为不同值类型时仍返回 true', () => {
    (globalThis as Record<string, unknown>).__TAURI__ = true;
    expect(isTauriEnv()).toBe(true);
  });

  it('当 window.__TAURI__ 为字符串时仍返回 true', () => {
    (globalThis as Record<string, unknown>).__TAURI__ = '1.0.0';
    expect(isTauriEnv()).toBe(true);
  });

  it('当 globalThis 上无 __TAURI__ 时返回 false', () => {
    expect('__TAURI__' in globalThis).toBe(false);
    expect(isTauriEnv()).toBe(false);
  });

  it('还原后再次返回 false', () => {
    (globalThis as Record<string, unknown>).__TAURI__ = {};
    expect(isTauriEnv()).toBe(true);

    delete (globalThis as Record<string, unknown>).__TAURI__;
    expect(isTauriEnv()).toBe(false);
  });
});
