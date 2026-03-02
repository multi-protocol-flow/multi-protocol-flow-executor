/**
 * 报告存储服务
 * 通过 Tauri IPC 调用后端报告存储命令
 */

import { invoke } from '@tauri-apps/api/core';
import type { FlowExecutionReport } from '@/types/report';
import type { ReportMetadata } from '@/types/ipc';

/**
 * 保存执行报告到后端
 * @param report 流程执行报告
 */
export async function saveReport(report: FlowExecutionReport): Promise<void> {
  await invoke<void>('save_flow_report', { report });
}

/**
 * 从后端加载执行报告
 * @param uuid 报告 UUID
 * @returns 流程执行报告
 */
export async function loadReport(uuid: string): Promise<FlowExecutionReport> {
  return await invoke<FlowExecutionReport>('load_flow_report', { uuid });
}

/**
 * 列出所有报告元数据
 * @returns 报告元数据列表（按时间倒序）
 */
export async function listReports(): Promise<ReportMetadata[]> {
  return await invoke<ReportMetadata[]>('list_flow_reports');
}

/**
 * 删除指定报告
 * @param uuid 报告 UUID
 */
export async function deleteReport(uuid: string): Promise<void> {
  await invoke<void>('delete_flow_report', { uuid });
}
