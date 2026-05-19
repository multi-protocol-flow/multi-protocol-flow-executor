// Vitest 测试设置文件
import '@testing-library/jest-dom'
import { configure } from '@testing-library/react'
import { vi } from 'vitest'

/**
 * Mock typesafe-i18n util: make i18nObject(baseLocale) return real zh-CN translations.
 *
 * Both @/i18n/i18n-lazy (used by components) and @/i18n/i18n-util (used by
 * nodeMeta.ts / constants.ts) are covered because the lazy module imports from util.
 */
vi.mock('@/i18n/i18n-util', async () => {
  const zhCN = await import('@/i18n/zh-CN/index');
  const translations = (zhCN.default || zhCN) as Record<string, unknown>;

  function buildLLProxy(obj: unknown, seen = new WeakSet()): unknown {
    if (typeof obj === 'string') {
      const text: string = obj;
      // Support {param} template substitution for typesafe-i18n parameters
      return (params?: Record<string, unknown>) => {
        if (!params) return text;
        let result = text;
        for (const [key, value] of Object.entries(params)) {
          result = result.replace(new RegExp(`\\{${key}\\}`, 'g'), String(value ?? ''));
        }
        return result;
      };
    }
    if (obj === null || obj === undefined) return () => String(obj);
    if (seen.has(obj)) return () => '[Circular]';
    seen.add(obj);

    return new Proxy({} as Record<string, unknown>, {
      get: (target, key) => {
        if (typeof key === 'symbol') return undefined;
        if (key === 'then' || key === 'toJSON') return undefined;

        if (key in target) return target[key];

        const val = (obj as Record<string, unknown>)[key];
        if (val === undefined) {
          target[key] = () => key;
          return target[key];
        }

        const resolved = buildLLProxy(val, seen);
        target[key] = resolved;
        return resolved;
      },
    });
  }

  const LL = buildLLProxy(translations);

  return {
    baseLocale: 'zh-CN' as const,
    locales: ['zh-CN', 'en-US'],
    isLocale: (l: string) => ['zh-CN', 'en-US'].includes(l),
    detectLocale: () => 'zh-CN' as const,
    i18nObject: () => LL,
    i18n: () => ({ 'zh-CN': LL, 'en-US': LL }),
    i18nString: () => (s: string) => s,
    extendDictionary: (d: unknown) => d,
    loadedLocales: {},
    loadedFormatters: {},
  };
});

// 配置 testing-library 行为
configure({
  // 在 jsdom 环境中支持 act()
  reactStrictMode: true,
})

// ResizeObserver polyfill for React Flow
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

// 确保 global 和 window 存在后再设置
if (typeof global !== 'undefined') {
  global.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver
}

// Mock window.matchMedia for components that check media queries
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {}, // deprecated
    removeListener: () => {}, // deprecated
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})

// Mock window.getComputedStyle for React Flow (仅在 window 存在时)
if (typeof window !== 'undefined') {
  const originalGetComputedStyle = window.getComputedStyle
  window.getComputedStyle = (element: Element) => {
    return originalGetComputedStyle(element)
  }
}
