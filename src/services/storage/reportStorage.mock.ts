/**
 * 报告存储 Mock 服务
 * 用于非 Tauri 环境的测试
 */

import type { FlowExecutionReport } from '@/types/report';
import type { ReportMetadata } from '@/types/ipc';

// 内存存储
const reportStore = new Map<string, FlowExecutionReport>();

/**
 * 保存执行报告
 * @param report 流程执行报告
 * @param _reportsDir 报告存储目录（Mock 中忽略）
 */
export async function saveReport(report: FlowExecutionReport, _reportsDir: string): Promise<void> {
  reportStore.set(report.uuid, report);
}

/**
 * 加载执行报告
 * @param uuid 报告 UUID
 * @param _reportsDir 报告存储目录（Mock 中忽略）
 * @returns 流程执行报告
 */
export async function loadReport(uuid: string, _reportsDir: string): Promise<FlowExecutionReport> {
  const report = reportStore.get(uuid);
  if (!report) {
    throw new Error(`Report not found: ${uuid}`);
  }
  return report;
}

/**
 * 列出所有报告元数据
 * @param _reportsDir 报告存储目录（Mock 中忽略）
 * @returns 报告元数据列表（按时间倒序）
 */
export async function listReports(_reportsDir: string): Promise<ReportMetadata[]> {
  const reports = Array.from(reportStore.values());
  // 按时间倒序排列
  reports.sort((a, b) => b.start_time - a.start_time);
  
  return reports
    .filter(report => report.status !== 'skipped')
    .map(report => ({
      uuid: report.uuid,
      flow_uuid: report.flow_uuid,
      flow_name: report.flow_name,
      status: report.status as 'success' | 'failed',
      timestamp: report.start_time,
      duration_ms: report.duration_ms,
    }));
}

/**
 * 删除指定报告
 * @param uuid 报告 UUID
 * @param _reportsDir 报告存储目录（Mock 中忽略）
 */
export async function deleteReport(uuid: string, _reportsDir: string): Promise<void> {
  if (!reportStore.has(uuid)) {
    throw new Error(`Report not found: ${uuid}`);
  }
  reportStore.delete(uuid);
}

/**
 * 清空所有报告（测试辅助函数）
 */
export function clearReports(): void {
  reportStore.clear();
}

/**
 * 获取存储的报告数量（测试辅助函数）
 */
export function getReportCount(): number {
  return reportStore.size;
}
