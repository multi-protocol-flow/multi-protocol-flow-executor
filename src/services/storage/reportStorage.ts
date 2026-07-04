/**
 * 报告存储服务
 * 通过 Tauri IPC 调用后端报告存储命令
 */

import { invoke } from '@tauri-apps/api/core';
import type { FlowExecutionReport } from '@/types/report';
import type { ReportMetadata } from '@/types/ipc';
import { isTauriEnv } from '@/lib/platform';

/**
 * 保存执行报告到后端
 * @param report 流程执行报告
 * @param reportsDir 报告存储目录
 */
export async function saveReport(report: FlowExecutionReport, reportsDir: string): Promise<void> {
  if (isTauriEnv()) {
    await invoke<void>('save_flow_report', { report, reportsDir });
    return;
  }
  const { saveReport: mock } = await import('./reportStorage.mock');
  return mock(report, reportsDir);
}

/**
 * 从后端加载执行报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 * @returns 流程执行报告
 */
export async function loadReport(uuid: string, reportsDir: string): Promise<FlowExecutionReport> {
  if (isTauriEnv()) {
    return await invoke<FlowExecutionReport>('load_flow_report', { uuid, reportsDir });
  }
  const { loadReport: mock } = await import('./reportStorage.mock');
  return mock(uuid, reportsDir);
}

/**
 * 列出所有报告元数据
 * @param reportsDir 报告存储目录
 * @returns 报告元数据列表（按时间倒序）
 */
export async function listReports(reportsDir: string): Promise<ReportMetadata[]> {
  if (isTauriEnv()) {
    return await invoke<ReportMetadata[]>('list_flow_reports', { reportsDir });
  }
  const { listReports: mock } = await import('./reportStorage.mock');
  return mock(reportsDir);
}

/**
 * 删除指定报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 */
export async function deleteReport(uuid: string, reportsDir: string): Promise<void> {
  if (isTauriEnv()) {
    await invoke<void>('delete_flow_report', { uuid, reportsDir });
    return;
  }
  const { deleteReport: mock } = await import('./reportStorage.mock');
  return mock(uuid, reportsDir);
}
