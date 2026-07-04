/**
 * 平台环境检测工具
 * 集中管理 Tauri 运行时环境检测逻辑
 */

export function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}
