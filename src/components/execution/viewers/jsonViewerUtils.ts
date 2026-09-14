/**
 * JSON & 响应体展示相关的工具函数与阈值定义
 */

/** 字节大小阈值 */
export const JSON_VIEWER_THRESHOLDS = {
  /** 小体积（< 50 KB）：默认全部展开 */
  SMALL: 50 * 1024,
  /** 中体积（1 MB）：大于此值时默认优先以 CodeMirror 视图打开，树仅展开 1 层 */
  MEDIUM: 1024 * 1024,
  /** 极端大体积（5 MB）：树视图展示警示屏障，需用户手动确认才渲染 */
  EXTREME: 5 * 1024 * 1024,
} as const;

/**
 * 格式化字节大小
 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0 || !Number.isFinite(bytes)) {
    return '0 B';
  }
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${i === 0 ? value : value.toFixed(value >= 100 ? 0 : 1)} ${units[i]}`;
}

/**
 * 估算数据的字节大小
 */
export function estimateDataSize(data: unknown, rawString?: string): number {
  if (rawString !== undefined) {
    return rawString.length;
  }
  if (typeof data === 'string') {
    return data.length;
  }
  if (data === null || data === undefined) {
    return 0;
  }
  if (typeof data === 'number' || typeof data === 'boolean') {
    return String(data).length;
  }
  try {
    return JSON.stringify(data).length;
  } catch {
    return 0;
  }
}

/**
 * 根据数据大小及用户手动指令生成树展开策略
 */
export function getAdaptiveExpandStrategy(
  byteSize: number,
  userOverride: 'all' | 'collapse' | null
): (level: number) => boolean {
  if (userOverride === 'all') {
    return () => true;
  }
  if (userOverride === 'collapse') {
    return () => false;
  }
  // 自适应：小数据全展，中等数据展 2 层，大数据仅展 1 层
  if (byteSize < JSON_VIEWER_THRESHOLDS.SMALL) {
    return () => true;
  }
  if (byteSize < JSON_VIEWER_THRESHOLDS.MEDIUM) {
    return (level: number) => level < 2;
  }
  return (level: number) => level < 1;
}

/**
 * 将文本内容触发为本地文件下载
 */
export function downloadContentAsFile(content: string, filename: string, mimeType = 'application/json'): void {
  try {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  } catch (error) {
    console.error('Failed to download file:', error);
  }
}
