/**
 * hotkeyLabels - 快捷键标签工具
 * 根据平台显示 Cmd/Ctrl 等修饰键
 */

/** 检测当前平台是否为 macOS */
function isMac(): boolean {
  if (typeof navigator === 'undefined') return false;
  return navigator.platform.toUpperCase().includes('MAC');
}

/** macOS 修饰键映射 */
const MAC_MODIFIERS: Record<string, string> = {
  'Ctrl+': '⌘',
  'Ctrl': '⌘',
  'Shift+': '⇧',
  'Shift': '⇧',
  'Alt+': '⌥',
  'Alt': '⌥',
};

/**
 * 将 Ctrl+ 跨平台标签转换为平台特定的显示文本
 *
 * @example
 * hotkeyLabel(['Ctrl+Z'])           // → '⌘Z' (macOS) / 'Ctrl+Z' (Win/Linux)
 * hotkeyLabel(['Ctrl+Shift+Z'])     // → '⌘⇧Z' (macOS) / 'Ctrl+Shift+Z' (Win/Linux)
 * hotkeyLabel(['Ctrl+D', 'Ctrl+E']) // → '⌘D 或 ⌘E' (macOS)
 */
export function hotkeyLabel(keys: string[]): string {
  if (keys.length === 0) return '';

  const platformKey = (key: string): string => {
    if (!isMac()) return key;
    let result = key;
    for (const [mod, symbol] of Object.entries(MAC_MODIFIERS)) {
      // 替换修饰键前缀
      if (result.startsWith(mod)) {
        result = symbol + result.slice(mod.length);
        break;
      }
    }
    return result;
  };

  return keys.map(platformKey).join(' 或 ');
}

/**
 * 获取当前平台修饰键名称
 */
export function getModifierKey(): string {
  return isMac() ? '⌘' : 'Ctrl';
}
