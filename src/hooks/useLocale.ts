/**
 * useCurrentLocale - 获取当前 locale 的 hook
 *
 * 从 localStorage 读取用户存储的 locale 设置，无存储时默认 'zh-CN'。
 * SSR 安全：使用 typeof window !== 'undefined' 守卫。
 */

export function useCurrentLocale(): string {
  if (typeof window !== 'undefined') {
    try {
      const saved = localStorage.getItem('locale');
      if (saved) return saved;
    } catch {
      // localStorage 不可用时静默降级
    }
  }
  return 'zh-CN';
}
