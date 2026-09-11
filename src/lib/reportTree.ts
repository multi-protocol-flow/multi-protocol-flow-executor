/**
 * 节点报告树查询工具
 *
 * 子流程的嵌套执行报告挂在 `NodeExecutionReport.sub_flow_report.node_reports` 上，
 * 层级不限（子流程内可再调用子流程）。报告 UI 需要在整棵树中定位 / 判断节点，
 * 因此统一在此提供递归查询，避免各处手写一层查找。
 */
import type { NodeExecutionReport } from '@/types';

/** 递归查找指定 UUID 的节点报告（含任意层级子流程），未命中返回 null */
export function findNodeReportInTree(
  reports: NodeExecutionReport[] | undefined,
  nodeUuid: string
): NodeExecutionReport | null {
  if (!reports) return null;
  for (const report of reports) {
    if (report.node_uuid === nodeUuid) return report;
    const nested = findNodeReportInTree(report.sub_flow_report?.node_reports, nodeUuid);
    if (nested) return nested;
  }
  return null;
}

/** 判断节点报告树中是否存在指定 UUID（含任意层级子流程） */
export function containsNodeReport(
  reports: NodeExecutionReport[] | undefined,
  nodeUuid: string | null | undefined
): boolean {
  if (!nodeUuid) return false;
  return findNodeReportInTree(reports, nodeUuid) !== null;
}
