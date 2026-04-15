/**
 * 压测报告存储服务
 * 通过 Tauri IPC 调用后端压测报告存储命令
 */

import { invoke } from '@tauri-apps/api/core';
import type { StressTestReport, StressTestReportMetadata } from '@/types/stressTest';

export const stressTestStorage = {
  /** 保存压测报告 */
  async saveReport(report: StressTestReport, reportsDir: string): Promise<string> {
    return invoke<string>('save_stress_test_report', { report, reportsDir });
  },

  /** 加载压测报告 */
  async loadReport(filePath: string): Promise<StressTestReport> {
    return invoke<StressTestReport>('load_stress_test_report', { filePath });
  },

  /** 列出所有压测报告 */
  async listReports(reportsDir: string): Promise<StressTestReportMetadata[]> {
    return invoke<StressTestReportMetadata[]>('list_stress_test_reports', { reportsDir });
  },

  /** 删除压测报告 */
  async deleteReport(filePath: string): Promise<void> {
    await invoke<void>('delete_stress_test_report', { filePath });
  },
};
