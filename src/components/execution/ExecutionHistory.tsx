/**
 * 统一执行历史列表组件
 * 合并普通执行、调试执行和压力测试历史，支持类型过滤
 */
import { toast } from 'sonner';
import { useState, useEffect, useCallback } from 'react';
import { CheckCircle2, XCircle, GitCompare, Clock, History, Loader2, Trash2, Bug, BarChart3, Database } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { loadReport, listReports, deleteReport } from '@/services/storage/reportStorage';
import { stressTestStorage } from '@/services/storage/stressTestStorage';
import type { ReportMetadata } from '@/types/ipc';
import type { StressTestReportMetadata, StressTestReport } from '@/types/stressTest';
import type { DatasetReportSummary } from '@/types/dataset';
import { useExecutionStore } from '@/stores/execution';
import { useStressTestStore } from '@/stores/stressTest';
import { useSettingsStore } from '@/stores/settings';
import { useUIStore } from '@/stores';
import { useDatasetExecutionStore } from '@/stores/datasetExecution';
import { listDatasetReports, deleteDatasetReport } from '@/services/tauri';
import { cn } from '@/lib/utils';

// ============================================================================
// 类型定义
// ============================================================================

/** 历史记录类型 */
type HistoryItemType = 'normal' | 'debug' | 'stress_test' | 'dataset';

/** 过滤类型 */
type FilterType = 'all' | 'normal' | 'debug' | 'stress_test' | 'dataset';

/** 统一历史记录项 */
interface UnifiedHistoryItem {
  /** 唯一标识 */
  uuid: string;
  /** 流程名称 */
  flowName: string;
  /** 时间戳 */
  timestamp: number;
  /** 执行时长（毫秒） */
  durationMs: number;
  /** 状态 */
  status: 'success' | 'failed' | 'completed';
  /** 记录类型 */
  type: HistoryItemType;
  /** 普通/调试报告元数据引用 */
  reportMetadata?: ReportMetadata;
  /** 压测报告元数据引用 */
  stressMetadata?: StressTestReportMetadata;
  /** 数据驱动报告元数据引用 */
  datasetMetadata?: DatasetReportSummary;
}

// ============================================================================
// 辅助函数
// ============================================================================

/** 格式化时间戳为可读字符串 */
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();
  const isToday = date.toDateString() === now.toDateString();

  if (isToday) {
    return date.toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  return date.toLocaleDateString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** 格式化耗时 */
function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

/** 获取过滤类型的显示名称 */
function getFilterLabel(filter: FilterType): string {
  switch (filter) {
    case 'all': return '全部';
    case 'normal': return '普通';
    case 'debug': return '调试';
    case 'stress_test': return '压测';
    case 'dataset': return '数据驱动';
  }
}

// ============================================================================
// 组件
// ============================================================================

interface ExecutionHistoryProps {
  /** 选择报告回调（点击历史项时触发） */
  onSelectReport?: (reportUuid: string) => void;
  /** 对比回调 */
  onCompare?: (reportUuid: string) => void;
  /** 重新执行回调（保留向后兼容） */
  onRerun?: (report: ReportMetadata) => void;
}

export function ExecutionHistory({ onSelectReport, onRerun: _onRerun, onCompare }: ExecutionHistoryProps) {
  const [unifiedHistory, setUnifiedHistory] = useState<UnifiedHistoryItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState<FilterType>('all');
  const [refreshKey, setRefreshKey] = useState(0);

  const openReportDialog = useExecutionStore((state) => state.openReportDialog);
  const reportDir = useSettingsStore((state) => state.reportDir);
  const showConfirm = useUIStore((state) => state.showConfirm);
  // 监听压测报告变化，压测完成后自动刷新历史列表
  const stressTestReport = useStressTestStore((state) => state.report);

  // 加载并合并历史列表
  useEffect(() => {
    if (!reportDir) return;

    let cancelled = false;

    const loadAllHistory = async () => {
      setLoading(true);
      try {
        // 并行加载普通报告、压测报告和数据驱动报告
        const [reports, stressReports, datasetReports] = await Promise.all([
          listReports(reportDir),
          stressTestStorage.listReports(reportDir),
          listDatasetReports(reportDir),
        ]);

        if (cancelled) return;

        // 映射普通/调试报告
        const normalItems: UnifiedHistoryItem[] = reports.map((r: ReportMetadata) => ({
          uuid: r.uuid,
          flowName: r.flow_name,
          timestamp: r.timestamp,
          durationMs: r.duration_ms,
          status: r.status as 'success' | 'failed',
          type: r.execution_mode === 'debug' ? 'debug' : 'normal',
          reportMetadata: r,
        }));

        // 映射压测报告
        const stressItems: UnifiedHistoryItem[] = stressReports.map((s: StressTestReportMetadata) => ({
          uuid: s.uuid,
          flowName: s.flowName,
          timestamp: s.startTime,
          durationMs: 0,
          status: 'completed' as const,
          type: 'stress_test' as const,
          stressMetadata: s,
        }));

        // 映射数据驱动报告
        const datasetItems: UnifiedHistoryItem[] = datasetReports.map((d: DatasetReportSummary) => ({
          uuid: d.report_id,
          flowName: d.flow_name,
          timestamp: d.executed_at,
          durationMs: 0,
          status: (d.fail_count + d.error_count === 0 ? 'success' : 'failed') as 'success' | 'failed',
          type: 'dataset' as const,
          datasetMetadata: d,
        }));

        // 合并并按时间戳降序排序（最新在前）
        const merged = [...normalItems, ...stressItems, ...datasetItems].sort(
          (a, b) => b.timestamp - a.timestamp
        );

        setUnifiedHistory(merged);
      } catch (error) {
        if (cancelled) return;
        console.error('加载历史列表失败:', error);
        toast.error('加载历史列表失败', {
          description: error instanceof Error ? error.message : '未知错误',
        });
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    loadAllHistory();

    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reportDir, refreshKey, stressTestReport]);

  // 重新加载历史
  const reloadHistory = useCallback(() => {
    setRefreshKey((prev) => prev + 1);
  }, []);

  // 过滤后的列表
  const filteredHistory = filter === 'all'
    ? unifiedHistory
    : unifiedHistory.filter((item) => item.type === filter);

  // 点击普通/调试报告 - 加载完整报告
  const handleSelectReport = useCallback(async (item: UnifiedHistoryItem) => {
    try {
      if (item.type === 'stress_test' && item.stressMetadata) {
        // 压测报告：加载后写入 stressTest store 并打开弹窗
        const report = await stressTestStorage.loadReport(item.stressMetadata.filePath);
        useStressTestStore.setState({
          report: report as StressTestReport,
          progressHistory: (report as StressTestReport).progressHistory ?? [],
        });
        useStressTestStore.getState().openReportDialog();
      } else if (item.type === 'dataset') {
        // 数据驱动报告：打开报告弹窗
        useDatasetExecutionStore.getState().openReportDialog(item.uuid);
      } else {
        // 普通/调试报告
        const report = await loadReport(item.uuid, reportDir);
        openReportDialog(report);
      }
      onSelectReport?.(item.uuid);
    } catch (error) {
      console.error('加载报告失败:', error);
      toast.error('加载报告失败', {
        description: error instanceof Error ? error.message : '未知错误',
      });
    }
  }, [openReportDialog, onSelectReport, reportDir]);

  // 删除单个报告（带确认）
  const handleDelete = useCallback((item: UnifiedHistoryItem) => {
    showConfirm({
      title: '删除报告',
      description: '确定要删除此执行报告吗？删除后无法恢复。',
      confirmText: '删除',
      onConfirm: async () => {
        try {
          if (item.type === 'stress_test' && item.stressMetadata) {
            await stressTestStorage.deleteReport(item.stressMetadata.filePath);
          } else if (item.type === 'dataset') {
            await deleteDatasetReport(item.uuid, reportDir);
          } else {
            await deleteReport(item.uuid, reportDir);
          }
          // 从本地状态移除
          setUnifiedHistory((prev) => prev.filter((h) => h.uuid !== item.uuid));
          toast.success('报告已删除');
        } catch (error) {
          console.error('删除报告失败:', error);
          const errorMessage = error instanceof Error ? error.message : '未知错误';
          if (errorMessage.includes('not found') || errorMessage.includes('不存在')) {
            toast.error('报告文件已删除', {
              description: '该报告文件可能已被手动删除',
            });
          } else if (errorMessage.includes('permission') || errorMessage.includes('权限')) {
            toast.error('删除报告失败', {
              description: '没有足够的权限删除该文件',
            });
          } else {
            toast.error('删除报告失败', {
              description: errorMessage,
            });
          }
        }
      },
    });
  }, [reportDir, showConfirm]);

  // 清空历史（带确认，按过滤类型操作）
  const handleClearHistory = useCallback(() => {
    const label = getFilterLabel(filter);
    const count = filteredHistory.length;

    showConfirm({
      title: `清空${label}执行历史`,
      description: `确定要清空所有${label}执行历史吗？共 ${count} 条记录，删除后无法恢复。`,
      confirmText: '清空',
      onConfirm: async () => {
        try {
          if (filter === 'stress_test') {
            // 仅删除压测报告
            const stressItems = filteredHistory.filter(
              (item) => item.type === 'stress_test' && item.stressMetadata
            );
            await Promise.all(
              stressItems.map((item) =>
                stressTestStorage.deleteReport(item.stressMetadata!.filePath)
              )
            );
          } else if (filter === 'normal') {
            // 仅删除非调试的普通报告
            const normalItems = filteredHistory.filter(
              (item) => item.reportMetadata && item.type === 'normal'
            );
            await Promise.all(
              normalItems.map((item) => deleteReport(item.uuid, reportDir))
            );
          } else if (filter === 'debug') {
            // 仅删除调试报告
            const debugItems = filteredHistory.filter(
              (item) => item.reportMetadata && item.type === 'debug'
            );
            await Promise.all(
              debugItems.map((item) => deleteReport(item.uuid, reportDir))
            );
          } else if (filter === 'dataset') {
            // 仅删除数据驱动报告
            const datasetItems = filteredHistory.filter(
              (item) => item.type === 'dataset'
            );
            await Promise.all(
              datasetItems.map((item) => deleteDatasetReport(item.uuid, reportDir))
            );
          } else {
            // 全部清空：同时删除普通报告、压测报告和数据驱动报告
            const allFlowItems = filteredHistory.filter((item) => item.reportMetadata);
            const allStressItems = filteredHistory.filter(
              (item) => item.type === 'stress_test' && item.stressMetadata
            );
            const allDatasetItems = filteredHistory.filter(
              (item) => item.type === 'dataset'
            );
            await Promise.all([
              ...allFlowItems.map((item) => deleteReport(item.uuid, reportDir)),
              ...allStressItems.map((item) =>
                stressTestStorage.deleteReport(item.stressMetadata!.filePath)
              ),
              ...allDatasetItems.map((item) => deleteDatasetReport(item.uuid, reportDir)),
            ]);
          }
          // 重新加载全部历史
          reloadHistory();
          toast.success(`${label}执行历史已清空`);
        } catch (error) {
          console.error('清空执行历史失败:', error);
          toast.error('清空执行历史失败', {
            description: error instanceof Error ? error.message : '未知错误',
          });
        }
      },
    });
  }, [filter, filteredHistory, reportDir, showConfirm, reloadHistory]);

  // 加载中状态（初次加载）
  if (loading && unifiedHistory.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-8 text-muted-foreground">
        <Loader2 className="w-10 h-10 mb-3 animate-spin" />
        <p className="text-sm">加载中...</p>
      </div>
    );
  }

  // 空状态
  if (unifiedHistory.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full py-8 text-muted-foreground">
        <History className="w-10 h-10 mb-3 opacity-40" />
        <p className="text-sm">暂无执行历史</p>
        <p className="text-xs mt-1 opacity-70">执行流程后记录将显示在这里</p>
      </div>
    );
  }

  // 过滤后空状态
  if (filteredHistory.length === 0) {
    const emptyTextMap: Record<FilterType, { title: string; subtitle: string }> = {
      all: { title: '暂无执行历史', subtitle: '执行流程后记录将显示在这里' },
      normal: { title: '暂无普通执行历史', subtitle: '' },
      debug: { title: '暂无调试执行历史', subtitle: '' },
      stress_test: { title: '暂无压测执行历史', subtitle: '执行压力测试后记录将显示在这里' },
      dataset: { title: '暂无数据驱动执行历史', subtitle: '执行数据驱动测试后记录将显示在这里' },
    };
    const { title, subtitle } = emptyTextMap[filter];

    return (
      <div className="flex flex-col h-full">
        {/* 头部 */}
        <div className="flex items-center justify-between px-3 py-2 border-b border-border">
          <div className="flex items-center gap-2">
            <Select value={filter} onValueChange={(v) => setFilter(v as FilterType)}>
              <SelectTrigger className="w-20 h-7 text-xs">
                <SelectValue placeholder="全部" />
              </SelectTrigger>
                <SelectContent>
                <SelectItem value="all">全部</SelectItem>
                <SelectItem value="normal">普通</SelectItem>
                <SelectItem value="debug">调试</SelectItem>
                <SelectItem value="stress_test">压测</SelectItem>
                <SelectItem value="dataset">数据驱动</SelectItem>
              </SelectContent>
            </Select>
            <span className="text-xs text-muted-foreground">共 0 条记录</span>
          </div>
        </div>
        {/* 空内容 */}
        <div className="flex flex-col items-center justify-center flex-1 py-8 text-muted-foreground">
          <History className="w-10 h-10 mb-3 opacity-40" />
          <p className="text-sm">{title}</p>
          {subtitle && <p className="text-xs mt-1 opacity-70">{subtitle}</p>}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 头部 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <div className="flex items-center gap-2">
          <Select value={filter} onValueChange={(v) => setFilter(v as FilterType)}>
            <SelectTrigger className="w-20 h-7 text-xs">
              <SelectValue placeholder="全部" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="normal">普通</SelectItem>
              <SelectItem value="debug">调试</SelectItem>
              <SelectItem value="stress_test">压测</SelectItem>
              <SelectItem value="dataset">数据驱动</SelectItem>
            </SelectContent>
          </Select>
          <span className="text-xs text-muted-foreground">
            共 {filteredHistory.length} 条记录
          </span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleClearHistory}
          className="h-7 text-xs text-muted-foreground hover:text-red-500"
        >
          清空
        </Button>
      </div>

      {/* 历史列表 */}
      <ScrollArea className="flex-1">
        <div className="space-y-1 p-2">
          {filteredHistory.map((item) => {
            const isStress = item.type === 'stress_test';
            const isDataset = item.type === 'dataset';
            const isSuccess = item.status === 'success' || item.status === 'completed';
            return (
              <div
                key={item.uuid}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    handleSelectReport(item);
                  }
                }}
                onClick={() => handleSelectReport(item)}
                className={cn(
                  'group flex items-center justify-between p-2.5 rounded-lg w-full',
                  'border border-transparent hover:border-border',
                  'hover:bg-accent cursor-pointer transition-all duration-150 text-left'
                )}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  {/* 状态图标 */}
                  <div
                    className={cn(
                      'shrink-0 w-7 h-7 rounded-full flex items-center justify-center',
                      isStress
                        ? 'bg-blue-100'
                        : isSuccess
                          ? 'bg-emerald-100'
                          : 'bg-red-100'
                    )}
                  >
                    {isStress ? (
                      <BarChart3 className="w-3.5 h-3.5 text-blue-600" />
                    ) : isSuccess ? (
                      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                    ) : (
                      <XCircle className="w-3.5 h-3.5 text-red-600" />
                    )}
                  </div>

                  {/* 流程信息 */}
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <p className="text-sm font-medium text-foreground truncate">
                        {item.flowName}
                      </p>
                      {/* 调试模式标识 */}
                      {item.type === 'debug' && (
                        <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 text-amber-700">
                          <Bug className="w-3 h-3" />
                          Debug
                        </span>
                      )}
                      {/* 压测标识 */}
                      {item.type === 'stress_test' && (
                        <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-xs font-medium bg-violet-100 text-violet-700">
                          <BarChart3 className="w-3 h-3" />
                          压测
                        </span>
                      )}
                      {/* 数据驱动标识 */}
                      {item.type === 'dataset' && (
                        <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-xs font-medium bg-teal-100 text-teal-700">
                          <Database className="w-3 h-3" />
                          数据驱动
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-xs text-muted-foreground mt-0.5">
                      <Clock className="w-3 h-3" />
                      <span>{formatTimestamp(item.timestamp)}</span>
                      {/* 压测和数据驱动不显示时长 */}
                      {!isStress && !isDataset && item.durationMs > 0 && (
                        <>
                          <span className="opacity-50">·</span>
                          <span>{formatDuration(item.durationMs)}</span>
                        </>
                      )}
                    </div>
                  </div>
                </div>

                {/* 操作按钮 */}
                <div className="flex items-center gap-1 shrink-0">
                  {/* 删除按钮 */}
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(item);
                    }}
                    className={cn(
                      'w-8 h-8 opacity-0 group-hover:opacity-100',
                      'transition-opacity duration-150 text-muted-foreground hover:text-red-500'
                    )}
                    title="删除"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </Button>

                  {/* 对比按钮 */}
                  {(item.reportMetadata || item.stressMetadata) && (
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={(e) => {
                        e.stopPropagation();
                        onCompare?.(item.uuid);
                      }}
                      className={cn(
                        'w-8 h-8 opacity-0 group-hover:opacity-100',
                        'transition-opacity duration-150 text-muted-foreground hover:text-blue-500'
                      )}
                      title="对比"
                    >
                      <GitCompare className="w-3.5 h-3.5" />
                    </Button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </ScrollArea>
    </div>
  );
}
