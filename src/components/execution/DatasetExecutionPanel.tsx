/**
 * DatasetExecutionPanel - 数据驱动执行面板
 * CSV 文件选择 + 执行配置 + 实时进度监控
 */
import { useState, useMemo, useCallback, useEffect } from 'react';
import {
  Database,
  FileSpreadsheet,
  Play,
  Square,
  X,
  Check,
  AlertTriangle,
  Clock,
  Loader2,
  ChevronDown,
  ChevronUp,
  Download,
} from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'sonner';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useFlowStore } from '@/stores';
import {
  useDatasetExecutionStore,
  selectProgressPercent,
} from '@/stores/datasetExecution';
import { validateDatasetCsv } from '@/services/tauri';
import type { CsvInfo, FailureStrategy } from '@/types/dataset';
import type { EntryConfig, NodeData } from '@/types/node';

// ============================================================================
// 类型定义
// ============================================================================

interface DatasetExecutionPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** 变量匹配结果 */
interface VariableMatch {
  /** CSV 列名 */
  columnName: string;
  /** 是否在流程变量中找到匹配 */
  matched: boolean;
}

// ============================================================================
// 辅助函数
// ============================================================================

/**
 * 根据流程的 Entry 初始变量生成 CSV 模板内容
 * 使用初始变量名作为列头，示例值填充前 3 行
 */
function generateCsvTemplate(flow: {
  nodes: { type: string; data: NodeData }[];
}): string {
  const variableNames = getEntryVariableNames(flow);

  if (variableNames.length === 0) {
    // 没有初始变量时，提供一个通用模板示例
    const LL = getLL();
    const t = (idx: number) => LL.execution.dataset.template_example_prefix({ index: idx });
    return [
      LL.execution.dataset.template_header_no_vars(),
      `${t(1)},${t(1)},${t(1)}`,
      `${t(2)},${t(2)},${t(2)}`,
      `${t(3)},${t(3)},${t(3)}`,
    ].join('\n');
  }

  // 用变量名作为列头，生成 3 行示例占位数据
  const header = variableNames.join(',');
  const LL = getLL();
  const rows = Array.from({ length: 3 }, (_, i) =>
    variableNames.map(() => LL.execution.dataset.template_example_prefix({ index: i + 1 })).join(',')
  );
  return [header, ...rows].join('\n');
}

/** 触发 CSV 模板下载（通过 Tauri 后端写入文件） */
async function downloadCsvTemplate(content: string, filename: string): Promise<string> {
  // 动态检测环境，选择对应的 saveCsvTemplate 实现
  const isTauriEnv = typeof window !== 'undefined' && '__TAURI__' in window;
  if (isTauriEnv) {
    const { saveCsvTemplate } = await import('@/services/tauri/settings');
    return saveCsvTemplate(filename, content);
  }
  const { saveCsvTemplate } = await import('@/services/tauri/settings.mock');
  return saveCsvTemplate(filename, content);
}

/** 格式化毫秒为可读时间 */
function formatDuration(ms: number): string {
  const LL = getLL();
  if (ms < 1000) return LL.execution.dataset.duration_ms({ ms });
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return LL.execution.dataset.duration_sec({ seconds });
  const minutes = Math.floor(seconds / 60);
  const remainSeconds = seconds % 60;
  return LL.execution.dataset.duration_min_sec({ minutes, seconds: remainSeconds });
}

/** 从 Entry 节点获取关联的 CSV 路径 */
function getEntryDatasetPath(flow: {
  nodes: { type: string; data: NodeData }[];
}): string | null {
  const entryNode = flow.nodes.find((n) => n.type === 'entry');
  if (!entryNode) return null;
  const data = entryNode.data as EntryConfig;
  return data.dataset_path ?? null;
}

/** 从 Entry 节点获取初始变量名列表 */
function getEntryVariableNames(flow: {
  nodes: { type: string; data: NodeData }[];
  initial_variables?: Record<string, unknown>;
}): string[] {
  // 变量已统一到流程级 initial_variables（环境管理 -> 默认变量）
  if (flow.initial_variables) {
    return Object.keys(flow.initial_variables);
  }
  return [];
}

// ============================================================================
// 子组件：文件选择区域
// ============================================================================

function FileSelectionArea({
  csvPath,
  csvInfo,
  isLoading,
  fileNotFound,
  onFileSelect,
  onDownloadTemplate,
}: {
  csvPath: string | null;
  csvInfo: CsvInfo | null;
  isLoading: boolean;
  fileNotFound: boolean;
  onFileSelect: () => void;
  onDownloadTemplate: () => void;
}) {
  const ll = getLL();
  return (
    <div className="space-y-2">
      <Label className="text-sm font-medium">{ll.execution.dataset.data_file()}</Label>
      {csvPath && csvInfo ? (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-muted/50 border border-border/50">
          <FileSpreadsheet className="w-5 h-5 text-emerald-600 shrink-0" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium truncate" title={csvPath}>
              {csvPath.split(/[\\/]/).pop()}
            </p>
            <p className="text-xs text-muted-foreground">
              {csvInfo.row_count}{ll.execution.dataset.label_row()} × {csvInfo.column_names.length}{ll.execution.dataset.label_column()} · {csvInfo.detected_encoding}
            </p>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onFileSelect}
            disabled={isLoading}
            className="shrink-0"
          >
            {ll.execution.dataset.re_select()}
          </Button>
        </div>
      ) : fileNotFound ? (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-amber-50 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-800">
          <AlertTriangle className="w-5 h-5 text-amber-600 shrink-0" />
          <div className="flex-1">
<p className="text-sm text-amber-700 dark:text-amber-400">
               {ll.execution.dataset.csv_not_found()}
             </p>
             <p className="text-xs text-amber-600/70 dark:text-amber-500/70">
               {ll.execution.dataset.csv_not_found_hint()}
             </p>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={onFileSelect}
            disabled={isLoading}
            className="shrink-0"
          >
            {ll.execution.dataset.select_file()}
          </Button>
        </div>
      ) : (
        <div className="space-y-2">
          <Button
            variant="outline"
            onClick={onFileSelect}
            disabled={isLoading}
            className="w-full h-20 border-dashed gap-2"
          >
            {isLoading ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : (
              <FileSpreadsheet className="w-5 h-5" />
            )}
            {isLoading ? ll.execution.dataset.reading() : ll.execution.dataset.select_csv()}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={onDownloadTemplate}
            className="w-full gap-1.5 text-xs text-muted-foreground hover:text-foreground"
          >
<Download className="w-3.5 h-3.5" />
             {ll.execution.dataset.download_template()}
          </Button>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 子组件：数据预览表格
// ============================================================================

function DataPreviewTable({ csvInfo }: { csvInfo: CsvInfo }) {
  const ll = getLL();
  const [isExpanded, setIsExpanded] = useState(false);

  // Mock 前 20 行数据用于预览（真实数据在执行时从后端读取）
  // 这里只展示列头信息
  return (
    <div className="space-y-2">
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-1.5 text-sm font-medium cursor-pointer hover:text-foreground transition-colors"
      >
        {isExpanded ? (
          <ChevronUp className="w-4 h-4" />
        ) : (
          <ChevronDown className="w-4 h-4" />
        )}
        {ll.execution.dataset.column_preview()}
        <Badge variant="secondary" className="text-xs">
          {csvInfo.column_names.length}
        </Badge>
      </button>
      {isExpanded && (
        <div className="rounded-lg border border-border/50 overflow-hidden">
          <div className="flex flex-wrap gap-1.5 p-2 bg-muted/30">
            {csvInfo.column_names.map((col) => (
              <Badge
                key={col}
                variant="outline"
                className="text-xs font-mono"
              >
                {col}
              </Badge>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 子组件：变量匹配概览
// ============================================================================

function VariableMatchOverview({
  matches,
}: {
  matches: VariableMatch[];
}) {
  const ll = getLL();
  if (matches.length === 0) return null;

  const matchedCount = matches.filter((m) => m.matched).length;
  const unmatchedCount = matches.length - matchedCount;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label className="text-sm font-medium">{ll.execution.dataset.variable_match()}</Label>
        <span className="text-xs text-muted-foreground">
          {matchedCount}/{matches.length} {ll.execution.dataset.match()}
        </span>
      </div>
      <div className="rounded-lg border border-border/50 overflow-hidden">
        <ScrollArea className="max-h-[120px]">
          <div className="divide-y divide-border/30">
            {matches.map((m) => (
              <div
                key={m.columnName}
                className="flex items-center justify-between px-3 py-1.5"
              >
                <span className="text-xs font-mono">{m.columnName}</span>
                {m.matched ? (
                  <Badge
                    variant="outline"
                    className="text-xs gap-1 border-emerald-200 dark:border-emerald-800 text-emerald-700 dark:text-emerald-400"
                  >
                    <Check className="w-3 h-3" />
                    {ll.execution.dataset.matched()}
                  </Badge>
                ) : (
                  <Badge
                    variant="outline"
                    className="text-xs border-muted text-muted-foreground"
                  >
                    {ll.execution.dataset.unmatched()}
                  </Badge>
                )}
              </div>
            ))}
          </div>
        </ScrollArea>
      </div>
      {unmatchedCount > 0 && (
        <p className="text-xs text-muted-foreground">
          {ll.execution.dataset.unmatched_hint({ count: unmatchedCount })}
        </p>
      )}
    </div>
  );
}

// ============================================================================
// 子组件：执行进度
// ============================================================================

function ExecutionProgress() {
  const ll = getLL();
  const isExecuting = useDatasetExecutionStore((s) => s.isExecuting);
  const totalRows = useDatasetExecutionStore((s) => s.totalRows);
  const completedRows = useDatasetExecutionStore((s) => s.completedRows);
  const successCount = useDatasetExecutionStore((s) => s.successCount);
  const failCount = useDatasetExecutionStore((s) => s.failCount);
  const errorCount = useDatasetExecutionStore((s) => s.errorCount);
  const elapsedTimeMs = useDatasetExecutionStore((s) => s.elapsedTimeMs);
  const estimatedRemainingMs = useDatasetExecutionStore(
    (s) => s.estimatedRemainingMs
  );
  const progressPercent = useDatasetExecutionStore(selectProgressPercent);

  if (!isExecuting) return null;

  return (
    <div className="space-y-3">
      {/* 进度条 */}
      <div className="space-y-1.5">
        <div className="flex items-center justify-between text-sm">
          <span className="font-medium">
            {completedRows}/{totalRows} {ll.execution.dataset.label_row()}
          </span>
          <span className="text-muted-foreground">{progressPercent}%</span>
        </div>
        <div className="h-2 bg-muted rounded-full overflow-hidden">
          <div
            className="h-full bg-primary rounded-full transition-all duration-300 ease-out"
            style={{ width: `${progressPercent}%` }}
          />
        </div>
      </div>

      {/* 统计指标 */}
      <div className="grid grid-cols-3 gap-2">
        <div className="flex items-center gap-1.5 text-xs">
          <div className="w-2 h-2 rounded-full bg-emerald-500" />
<span className="text-muted-foreground">{ll.execution.dataset.success()}</span>
           <span className="font-medium tabular-nums">{successCount}</span>
        </div>
        <div className="flex items-center gap-1.5 text-xs">
          <div className="w-2 h-2 rounded-full bg-amber-500" />
<span className="text-muted-foreground">{ll.execution.dataset.fail()}</span>
           <span className="font-medium tabular-nums">{failCount}</span>
        </div>
        <div className="flex items-center gap-1.5 text-xs">
          <div className="w-2 h-2 rounded-full bg-red-500" />
<span className="text-muted-foreground">{ll.execution.dataset.error()}</span>
           <span className="font-medium tabular-nums">{errorCount}</span>
        </div>
      </div>

      {/* 时间信息 */}
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <div className="flex items-center gap-1">
          <Clock className="w-3 h-3" />
          <span>{ll.execution.dataset.elapsed()} {formatDuration(elapsedTimeMs)}</span>
        </div>
        {estimatedRemainingMs !== null && estimatedRemainingMs > 0 && (
          <span>{ll.execution.dataset.estimated_remaining()} {formatDuration(estimatedRemainingMs)}</span>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export function DatasetExecutionPanel({
  open: isOpen,
  onOpenChange,
}: DatasetExecutionPanelProps) {
  const ll = getLL();
  const flow = useFlowStore((s) => s.flow);

  // Store 状态
  const isExecuting = useDatasetExecutionStore((s) => s.isExecuting);
  const startExecution = useDatasetExecutionStore(
    (s) => s.startExecution
  );
  const cancelExecution = useDatasetExecutionStore(
    (s) => s.cancelExecution
  );
  const currentReport = useDatasetExecutionStore(
    (s) => s.currentReport
  );
  const error = useDatasetExecutionStore((s) => s.error);
  const reset = useDatasetExecutionStore((s) => s.reset);

  // 本地状态
  const [csvPath, setCsvPath] = useState<string | null>(null);
  const [csvInfo, setCsvInfo] = useState<CsvInfo | null>(null);
  const [isLoadingCsv, setIsLoadingCsv] = useState(false);
  const [fileNotFound, setFileNotFound] = useState(false);
  const [concurrency, setConcurrency] = useState(1);
  const [failureStrategy, setFailureStrategy] =
    useState<FailureStrategy>('continue');

  // 自动检测 Entry 节点关联的 CSV
  const entryDatasetPath = useMemo(
    () => (flow ? getEntryDatasetPath(flow) : null),
    [flow]
  );

  // 获取 Entry 节点的初始变量名
  const entryVariableNames = useMemo(
    () => (flow ? getEntryVariableNames(flow) : []),
    [flow]
  );

  // 下载 CSV 模板
  const handleDownloadTemplate = useCallback(async () => {
    if (!flow) return;
    try {
      const content = generateCsvTemplate(flow);
      const flowName = flow.name.replace(/[/\\?%*:|"<>]/g, '_');
      const ll = getLL();
      const filename = `${flowName}${ll.execution.dataset.template_file_suffix()}.csv`;
      const savedPath = await downloadCsvTemplate(content, filename);
      // 获取目录路径（去掉文件名）
      const dirPath = savedPath.replace(/[/\\][^/\\]+$/, '');
      toast.success(ll.execution.toast.csv_template_downloaded(), {
        description: filename,
        duration: 8000,
        action: {
          label: ll.execution.toast.open_dir(),
          onClick: async () => {
            try {
              const isTauriEnv = typeof window !== 'undefined' && '__TAURI__' in window;
              if (isTauriEnv) {
                const { openDirectory } = await import('@/services/tauri/settings');
                await openDirectory(dirPath);
              }
            } catch {
              toast.error(ll.execution.toast.open_dir_failed());
            }
          },
        },
      });
    } catch (err) {
      toast.error(ll.execution.toast.download_template_failed(), {
        description: (err as Error).message,
      });
    }
  }, [flow]);

  // 变量匹配计算
  const variableMatches: VariableMatch[] = useMemo(() => {
    if (!csvInfo) return [];
    return csvInfo.column_names.map((col) => ({
      columnName: col,
      matched: entryVariableNames.includes(col),
    }));
  }, [csvInfo, entryVariableNames]);

  // 自动加载 Entry 节点关联的 CSV
  useEffect(() => {
    if (!isOpen) return;
    if (csvPath) return; // 已有手动选择

    if (entryDatasetPath) {
      loadCsvInfo(entryDatasetPath);
    }
    // 仅在面板打开时且首次执行
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, entryDatasetPath]);

  // 加载 CSV 信息
  const loadCsvInfo = useCallback(async (path: string) => {
    setIsLoadingCsv(true);
    setFileNotFound(false);
    try {
      const info = await validateDatasetCsv(path);
      setCsvPath(path);
      setCsvInfo(info);
    } catch {
      setFileNotFound(true);
      setCsvPath(null);
      setCsvInfo(null);
    } finally {
      setIsLoadingCsv(false);
    }
  }, []);

  // 选择 CSV 文件
  const handleFileSelect = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: ll.execution.dataset.csv_filter(), extensions: ['csv'] },
          { name: ll.execution.dataset.all_files(), extensions: ['*'] },
        ],
      });
      if (selected && typeof selected === 'string') {
        await loadCsvInfo(selected);
      }
    } catch {
      // 用户取消选择，忽略
    }
  }, [loadCsvInfo]);

  // 启动执行
  const handleStartExecution = useCallback(async () => {
    if (!flow || !csvPath) return;

    await startExecution(flow, {
      flow_uuid: flow.uuid,
      dataset_path: csvPath,
      dataset_encoding: csvInfo?.detected_encoding,
      concurrency,
      failure_strategy: failureStrategy,
    });
  }, [flow, csvPath, csvInfo, concurrency, failureStrategy, startExecution]);

  // 取消执行
  const handleCancel = useCallback(async () => {
    await cancelExecution();
  }, [cancelExecution]);

  // 关闭面板
  const handleClose = useCallback(() => {
    if (isExecuting) return; // 执行中不允许关闭
    reset();
    setCsvPath(null);
    setCsvInfo(null);
    setFileNotFound(false);
    setConcurrency(1);
    setFailureStrategy('continue');
    onOpenChange(false);
  }, [isExecuting, reset, onOpenChange]);

  // 不渲染关闭状态的面板
  if (!isOpen) return null;

  // 执行是否已完成
  const isCompleted = !isExecuting && currentReport !== null;
  const canStart =
    !isExecuting && !isCompleted && csvPath !== null && csvInfo !== null;

  return (
    <Card className="fixed top-16 right-16 w-[400px] max-h-[calc(100vh-140px)] flex flex-col overflow-hidden z-50 shadow-xl">
      <CardContent className="p-4 flex-1 min-h-0 flex flex-col">
        {/* 头部 */}
        <div className="flex items-center justify-between mb-4 shrink-0">
          <div className="flex items-center gap-2">
            <Database className="w-5 h-5 text-violet-500" />
            <span className="font-semibold">{ll.execution.dataset.title()}</span>
            {isExecuting && (
              <Badge
                variant="outline"
                className="text-xs bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-950/20 dark:text-emerald-400 dark:border-emerald-800"
              >
                {ll.execution.dataset.status_running()}
              </Badge>
            )}
            {isCompleted && (
              <Badge
                variant="outline"
                className="text-xs bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-950/20 dark:text-blue-400 dark:border-blue-800"
              >
                {ll.execution.dataset.status_completed()}
              </Badge>
            )}
          </div>
          {!isExecuting && (
            <button
              type="button"
              onClick={handleClose}
              className="p-1 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
              title={ll.execution.dataset.close()}
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        {/* 可滚动内容区域 */}
        <ScrollArea className="flex-1 min-h-0">
          <div className="space-y-4 pr-1">
            {/* 错误提示 */}
            {error && (
              <div className="flex items-center gap-2 p-3 rounded-lg bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-400 text-sm">
                <AlertTriangle className="w-4 h-4 shrink-0" />
                <span>{error}</span>
              </div>
            )}

            {/* 文件选择区域 */}
            <FileSelectionArea
              csvPath={csvPath}
              csvInfo={csvInfo}
              isLoading={isLoadingCsv}
              fileNotFound={fileNotFound}
              onFileSelect={handleFileSelect}
              onDownloadTemplate={handleDownloadTemplate}
            />

            {/* 数据预览 */}
            {csvInfo && <DataPreviewTable csvInfo={csvInfo} />}

            {/* 变量匹配概览 */}
            {csvInfo && variableMatches.length > 0 && (
              <VariableMatchOverview matches={variableMatches} />
            )}

            {/* 执行进度 */}
            {(isExecuting || isCompleted) && <ExecutionProgress />}

            {/* 完成摘要 */}
            {isCompleted && currentReport && (
              <div className="rounded-lg border border-border/50 bg-muted/30 p-3 space-y-2">
                <p className="text-sm font-medium">{ll.execution.dataset.completed()}</p>
                <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
                  <span className="text-muted-foreground">{ll.execution.dataset.total_rows()}</span>
                  <span className="tabular-nums">{currentReport.total_rows}</span>
                  <span className="text-muted-foreground">{ll.execution.dataset.total_duration()}</span>
                  <span className="tabular-nums">{formatDuration(currentReport.total_duration_ms)}</span>
                  <span className="text-emerald-600">{ll.execution.dataset.success()}</span>
                  <span className="tabular-nums text-emerald-600">{currentReport.success_count}</span>
                  <span className="text-amber-600">{ll.execution.dataset.fail()}</span>
                  <span className="tabular-nums text-amber-600">{currentReport.fail_count}</span>
                  <span className="text-red-600">{ll.execution.dataset.error()}</span>
                  <span className="tabular-nums text-red-600">{currentReport.error_count}</span>
                  <span className="text-muted-foreground">{ll.execution.dataset.avg_duration()}</span>
                  <span className="tabular-nums">{currentReport.avg_duration_ms}ms</span>
                  <span className="text-muted-foreground">P95</span>
                  <span className="tabular-nums">{currentReport.p95_duration_ms}ms</span>
                </div>
              </div>
            )}
          </div>
        </ScrollArea>

        {/* 执行选项 - 固定在滚动区域外，始终可见 */}
        {!isExecuting && !isCompleted && csvInfo && (
          <div className="shrink-0 space-y-3 pt-3 border-t border-border mt-3">
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
<Label htmlFor="ds-concurrency" className="text-sm font-medium">
                   {ll.execution.dataset.concurrency()}
                 </Label>
                <Input
                  id="ds-concurrency"
                  type="number"
                  min={1}
                  max={100}
                  value={concurrency}
                  onChange={(e) => {
                    const val = parseInt(e.target.value, 10);
                    if (!isNaN(val) && val >= 1 && val <= 100) {
                      setConcurrency(val);
                    }
                  }}
                  className="h-8"
                />
<p className="text-xs text-muted-foreground">
                   {ll.execution.dataset.concurrency_hint()}
                 </p>
              </div>
              <div className="space-y-1.5">
                <Label className="text-sm font-medium">{ll.execution.dataset.failure_strategy()}</Label>
                <Select
                  value={failureStrategy}
                  onValueChange={(v) => setFailureStrategy(v as FailureStrategy)}
                >
                  <SelectTrigger className="h-8">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="continue">{ll.execution.dataset.strategy_continue()}</SelectItem>
                    <SelectItem value="stop_on_first">{ll.execution.dataset.strategy_stop()}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>
        )}

        {/* 底部按钮 */}
        <div className="flex justify-end gap-2 shrink-0 pt-3 border-t border-border mt-3">
          {isExecuting ? (
            <Button
              variant="destructive"
              size="sm"
              onClick={handleCancel}
              className="gap-1.5"
            >
<Square className="w-4 h-4" />
               {ll.execution.dataset.cancel_execution()}
            </Button>
          ) : isCompleted ? (
<Button variant="outline" size="sm" onClick={handleClose}>
               {ll.execution.dataset.close()}
             </Button>
          ) : (
            <Button
              size="sm"
              onClick={handleStartExecution}
              disabled={!canStart}
              className="gap-1.5"
            >
<Play className="w-4 h-4" />
               {ll.execution.dataset.start()}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
