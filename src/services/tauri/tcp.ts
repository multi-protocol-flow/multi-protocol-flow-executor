/**
 * TCP 服务层 — TCP 协议模板相关的 Tauri IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';
import type { ProtocolTemplate } from '@/types/node';

/**
 * 获取所有 TCP 协议模板列表
 *
 * @returns 协议模板数组
 */
export async function listTcpTemplates(): Promise<ProtocolTemplate[]> {
  return invoke<ProtocolTemplate[]>('tcp_list_templates');
}

/**
 * 获取指定 TCP 协议模板
 *
 * @param id - 模板 ID
 * @returns 协议模板，未找到时返回 null
 */
export async function getTcpTemplate(id: string): Promise<ProtocolTemplate | null> {
  return invoke<ProtocolTemplate | null>('tcp_get_template', { id });
}
