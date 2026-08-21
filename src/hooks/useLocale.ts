/**
 * useCurrentLocale - 获取当前 locale 的 hook
 *
 * 响应式订阅 useSettingsStore 中的 locale 设置，并回退到 localStorage / 默认 'en-US'。
 * SSR 安全：使用 typeof window !== 'undefined' 守卫。
 */

import { useSettingsStore } from '@/stores/settings';

export function useCurrentLocale(): string {
  const storeLocale = useSettingsStore((state) => state.locale);
  if (typeof window !== 'undefined') {
    try {
      const saved = localStorage.getItem('locale');
      if (saved) return saved;
    } catch {
      // localStorage 不可用时静默降级
    }
  }
  return storeLocale || 'en-US';
}
