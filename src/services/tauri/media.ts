/**
 * Tauri IPC 媒体保存调用
 *
 * 媒体下载/解码由宿主完成：webview 里直接用 `fetch` 下载跨域媒体会被 CORS 阻塞，
 * 用 `<a download>` 又会被跨域 `Content-Disposition` 忽略。因此统一走后端命令。
 */

import { invoke } from '@tauri-apps/api/core';
import { isTauriEnv } from '@/lib/platform';
import { formatError } from './flow';

/** 媒体内容来源 */
export type MediaSaveSource =
  | { kind: 'url'; url: string }
  | { kind: 'data_url'; data_url: string }
  | { kind: 'body'; body: string };

/**
 * 把媒体内容写入用户选择好的路径
 *
 * @param source - 内容来源（远程 URL / data URL / 原始文本）
 * @param filePath - 目标路径；已存在时后端会自动去重为 `stem-1.ext`
 * @returns 实际写入路径
 * @throws IpcError 如果写入失败
 */
export async function saveResponseMedia(
  source: MediaSaveSource,
  filePath: string
): Promise<string> {
  if (!isTauriEnv()) {
    throw new Error('save_response_media is only available in the desktop app');
  }
  try {
    return await invoke<string>('save_response_media', { source, filePath });
  } catch (error) {
    throw formatError(error, 'save_response_media');
  }
}
