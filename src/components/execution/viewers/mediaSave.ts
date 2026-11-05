/**
 * mediaSave - 媒体候选的保存编排
 *
 * Tauri 环境走系统保存对话框 + 后端写盘（跨域下载与 `<a download>` 都不可靠）；
 * 浏览器环境退化为 Blob + `<a download>`。
 */

import { toast } from 'sonner';
import { save } from '@tauri-apps/plugin-dialog';
import { getLL } from '@/i18n/i18n-lazy';
import { isTauriEnv } from '@/lib/platform';
import { saveResponseMedia, type MediaSaveSource } from '@/services/tauri/media';
import { revealItemInDir } from '@/services/tauri/settings';
import type { MediaCandidate } from './mediaDetection';

/** 取文件名扩展名（不含点），兜底 `bin` */
function extensionOf(filename: string): string {
  const match = /\.([A-Za-z0-9]{1,5})$/.exec(filename);
  return match ? match[1].toLowerCase() : 'bin';
}

/** 浏览器下载：URL 直接跳转，base64/text 走 Blob */
function downloadInBrowser(source: MediaSaveSource, filename: string, mime: string): void {
  if (source.kind === 'url') {
    const link = document.createElement('a');
    link.href = source.url;
    link.download = filename;
    link.target = '_blank';
    link.rel = 'noreferrer';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    return;
  }

  let blob: Blob;
  if (source.kind === 'body') {
    blob = new Blob([source.body], { type: mime || 'text/plain;charset=utf-8' });
  } else {
    const comma = source.data_url.indexOf(',');
    const payload = comma >= 0 ? source.data_url.slice(comma + 1) : source.data_url;
    const binary = atob(payload);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    blob = new Blob([bytes], { type: mime || 'application/octet-stream' });
  }

  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/** 统一写盘/下载并给出 toast 反馈 */
async function persist(
  source: MediaSaveSource,
  defaultName: string,
  mime: string
): Promise<void> {
  const ll = getLL();
  try {
    if (isTauriEnv()) {
      const ext = extensionOf(defaultName);
      const path = await save({
        defaultPath: defaultName,
        filters: [{ name: `${ext.toUpperCase()} File`, extensions: [ext] }],
      });
      if (!path) return;
      const written = await saveResponseMedia(source, path);
      toast.success(ll.execution.toast.save_success(), {
        action: {
          label: getLL().flow.reveal_in_file_manager(),
          onClick: () => {
            revealItemInDir(written).catch(() => {});
          },
        },
      });
    } else {
      downloadInBrowser(source, defaultName, mime);
      toast.success(ll.execution.toast.save_success());
    }
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
}

/**
 * 保存一个媒体候选
 *
 * @param candidate - 媒体候选（`omitted` 候选无内容可存，直接忽略）
 * @param defaultName - 默认文件名（含扩展名）
 */
export async function saveMediaCandidate(
  candidate: MediaCandidate,
  defaultName: string
): Promise<void> {
  if (!candidate.source) return;
  const source: MediaSaveSource =
    candidate.source.type === 'url'
      ? { kind: 'url', url: candidate.source.value }
      : { kind: 'data_url', data_url: candidate.source.value };
  await persist(source, defaultName, candidate.mime);
}

/**
 * 保存原始响应体为文本文件
 *
 * @param body - 响应体（字符串原样，其余格式化为 JSON）
 * @param defaultName - 默认文件名（含扩展名）
 */
export async function saveRawBody(body: unknown, defaultName: string): Promise<void> {
  const text = typeof body === 'string' ? body : JSON.stringify(body, null, 2);
  const isJson = typeof body !== 'string';
  // 调用方只给基底名；这里统一决定后缀，避免节点名自带 .json/.txt 时出现双扩展名
  const base = defaultName.replace(/\.(txt|json)$/i, '');
  await persist(
    { kind: 'body', body: text },
    `${base}${isJson ? '.json' : '.txt'}`,
    isJson ? 'application/json' : 'text/plain'
  );
}
