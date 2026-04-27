/**
 * 数据集执行 IPC 服务层
 * 封装数据集执行相关的 Tauri IPC 调用
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type { Flow } from '@/types/flow';
import type {
  DatasetExecutionConfig,
  CsvInfo,
  DatasetExecutionReport,
  DatasetReportSummary,
  DatasetEvent,
} from '@/types/dataset';

// ============================================================================
// 数据集执行服务
// ============================================================================

export const datasetService = {
  /**
   * 验证 CSV 文件元信息
   * @param path CSV 文件路径
   * @param encoding 指定编码（可选）
   * @param delimiter 分隔符（可选）
   */
  validateCsv: (path: string, encoding?: string, delimiter?: string): Promise<CsvInfo> =>
    invoke<CsvInfo>('validate_dataset_csv', { path, encoding, delimiter }),

  /**
   * 启动数据驱动执行
   * 使用 Channel API 接收执行事件
   * @param flow 要执行的流程
   * @param config 数据集执行配置
   * @param onEvent 事件回调函数
   */
  executeWithDataset: (
    flow: Flow,
    config: DatasetExecutionConfig,
    onEvent: (event: DatasetEvent) => void
  ): Promise<void> => {
    const eventChannel = new Channel<DatasetEvent>();
    eventChannel.onmessage = (event) => {
      onEvent(event);
    };
    return invoke<void>('execute_flow_with_dataset', {
      input: { flow, config },
      onDatasetEvent: eventChannel,
    });
  },

  /**
   * 取消数据驱动执行
   */
  cancelExecution: (): Promise<void> => invoke<void>('cancel_dataset_execution'),

  /**
   * 列出数据集报告
   * @param reportsDir 报告存储目录
   */
  listReports: (reportsDir: string): Promise<DatasetReportSummary[]> =>
    invoke<DatasetReportSummary[]>('list_dataset_reports', { reportsDir }),

  /**
   * 加载数据集报告
   * @param reportId 报告 ID
   * @param reportsDir 报告存储目录
   */
  loadReport: (reportId: string, reportsDir: string): Promise<DatasetExecutionReport> =>
    invoke<DatasetExecutionReport>('load_dataset_report', { reportId, reportsDir }),

  /**
   * 加载失败行详情
   * @param reportId 报告 ID
   * @param rowIndex 行索引
   * @param reportsDir 报告存储目录
   */
  loadRowDetail: (
    reportId: string,
    rowIndex: number,
    reportsDir: string
  ): Promise<unknown | null> =>
    invoke<unknown | null>('load_dataset_row_detail', { reportId, rowIndex, reportsDir }),

  /**
   * 删除数据集报告
   * @param reportId 报告 ID
   * @param reportsDir 报告存储目录
   */
  deleteReport: (reportId: string, reportsDir: string): Promise<void> =>
    invoke<void>('delete_dataset_report', { reportId, reportsDir }),

  /**
   * 保存数据集报告到磁盘
   * @param report 数据集执行报告
   * @param reportsDir 报告存储目录
   */
  saveReport: (report: DatasetExecutionReport, reportsDir: string): Promise<string> =>
    invoke<string>('save_dataset_report', { report, reportsDir }),
};
