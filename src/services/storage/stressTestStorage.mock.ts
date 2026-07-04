/**
 * 压测报告存储 Mock 服务
 * 用于非 Tauri 环境的测试
 */

import type { StressTestReport, StressTestReportMetadata } from '@/types/stressTest';

// 内存存储
const reportStore = new Map<string, StressTestReport>();

/** 从 filePath 提取 UUID */
function getUuidFromPath(filePath: string): string {
  return filePath.replace('mock://stress/', '').replace('.json', '');
}

/**
 * 保存压测报告
 * @param report 压测报告
 * @param _reportsDir 报告存储目录（Mock 中忽略）
 * @returns 报告文件路径
 */
export async function saveReport(report: StressTestReport, _reportsDir?: string): Promise<string> {
  reportStore.set(report.uuid, report);
  return `mock://stress/${report.uuid}.json`;
}

/**
 * 加载压测报告
 * @param filePath 报告文件路径
 * @returns 压测报告
 */
export async function loadReport(filePath: string): Promise<StressTestReport> {
  const uuid = getUuidFromPath(filePath);
  const report = reportStore.get(uuid);
  if (!report) {
    throw new Error(`Stress test report not found: ${filePath}`);
  }
  return report;
}

/**
 * 列出所有压测报告
 * @param _reportsDir 报告存储目录（Mock 中忽略）
 * @returns 压测报告元数据列表
 */
export async function listReports(_reportsDir?: string): Promise<StressTestReportMetadata[]> {
  return Array.from(reportStore.values()).map((report) => ({
    uuid: report.uuid,
    flowUuid: report.flowUuid,
    flowName: report.flowName,
    startTime: report.startTime,
    filePath: `mock://stress/${report.uuid}.json`,
  }));
}

/**
 * 删除压测报告
 * @param filePath 报告文件路径
 */
export async function deleteReport(filePath: string): Promise<void> {
  const uuid = getUuidFromPath(filePath);
  if (!reportStore.has(uuid)) {
    throw new Error(`Stress test report not found: ${filePath}`);
  }
  reportStore.delete(uuid);
}

/**
 * 清空所有压测报告（测试辅助函数）
 */
export function clearMockStressTests(): void {
  reportStore.clear();
}

/**
 * 获取存储的压测报告数量（测试辅助函数）
 */
export function getMockStressTestCount(): number {
  return reportStore.size;
}
