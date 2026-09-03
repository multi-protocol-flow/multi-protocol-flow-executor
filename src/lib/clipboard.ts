/**
 * 安全剪贴板与序列化工具
 * 1. safeStringify: 防止循环引用导致 JSON.stringify 崩溃，正确序列化 Error 对象
 * 2. copyToClipboard: 兼具现代 Clipboard API 与 execCommand 回退，带超时保护，杜绝 Linux WebKitGTK / Windows 剪贴板挂起卡死
 */

/**
 * 安全 JSON 序列化，处理循环引用与 Error 对象
 */
export function safeStringify(obj: unknown, space?: number): string {
  if (obj === undefined || obj === null) return '';
  if (typeof obj === 'string') return obj;
  if (typeof obj !== 'object') return String(obj);

  const seen = new WeakSet();
  try {
    return JSON.stringify(
      obj,
      (_key, value) => {
        if (typeof value === 'object' && value !== null) {
          if (seen.has(value)) {
            return '[Circular]';
          }
          seen.add(value);
        }
        if (value instanceof Error) {
          return {
            name: value.name,
            message: value.message,
            stack: value.stack,
          };
        }
        return value;
      },
      space
    );
  } catch {
    try {
      return String(obj);
    } catch {
      return '[Unserializable Object]';
    }
  }
}

/**
 * 复制文本到系统剪贴板（带超时和回退机制）
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  // 1. 尝试现代 navigator.clipboard.writeText（带 1000ms 超时限制，防止主线程卡死）
  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    try {
      const copyPromise = navigator.clipboard.writeText(text);
      const timeoutPromise = new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error('Clipboard writeText timeout')), 1000)
      );
      await Promise.race([copyPromise, timeoutPromise]);
      return true;
    } catch {
      // 发生超时或拒绝时降级到 fallback
    }
  }

  // 2. 降级方案：document.execCommand('copy')
  if (typeof document !== 'undefined') {
    try {
      const textArea = document.createElement('textarea');
      textArea.value = text;
      textArea.style.position = 'fixed';
      textArea.style.left = '-9999px';
      textArea.style.top = '-9999px';
      textArea.style.opacity = '0';
      textArea.setAttribute('readonly', '');
      document.body.appendChild(textArea);
      textArea.focus();
      textArea.select();
      const successful = document.execCommand('copy');
      document.body.removeChild(textArea);
      if (successful) {
        return true;
      }
    } catch {
      // 忽略 fallback 异常
    }
  }

  return false;
}
