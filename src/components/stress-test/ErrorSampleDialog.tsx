/**
 * ErrorSampleDialog - 压测错误采样详情弹窗
 * 展示压测中采样的失败请求/节点执行完整快照（错误信息、配置参数、输入数据、响应输出等）
 */
import { useState, useEffect, useMemo } from 'react';
import {
  AlertCircle,
  Clock,
  Timer,
  Layers,
  FileCode,
  ArrowRight,
  CornerDownRight,
  X,
} from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ErrorSection } from '@/components/execution/viewers/ErrorSection';
import { RawJsonViewer } from '@/components/execution/viewers/RawJsonViewer';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
import type { ErrorSample } from '@/types/stressTest';

interface ErrorSampleDialogProps {
  /** 弹窗是否可见 */
  open: boolean;
  /** 可见性变更回调 */
  onOpenChange: (open: boolean) => void;
  /** 错误类型分类名称 */
  errorKind: string;
  /** 错误样本列表 */
  samples: ErrorSample[];
}

/** 格式化时间戳 */
function formatTime(timestamp?: number): string {
  if (!timestamp) return '-';
  const date = new Date(timestamp);
  const ms = String(date.getMilliseconds()).padStart(3, '0');
  return `${date.toLocaleTimeString(undefined, {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })}.${ms}`;
}

export function ErrorSampleDialog({
  open,
  onOpenChange,
  errorKind,
  samples,
}: ErrorSampleDialogProps) {
  const [selectedIndex, setSelectedIndex] = useState(0);

  // 当 samples 改变或弹窗打开时重置选中项
  useEffect(() => {
    if (open) {
      setSelectedIndex(0);
    }
  }, [open, samples]);

  const currentSample = useMemo(() => {
    if (samples.length === 0) return null;
    return samples[selectedIndex] ?? samples[0];
  }, [samples, selectedIndex]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-4xl h-[80vh] flex flex-col overflow-hidden p-6 gap-4"
        hideCloseButton
      >
        {/* 头部 */}
        <DialogHeader className="flex-shrink-0">
          <DialogTitle className="flex items-center gap-3">
            <div className="p-2 rounded-full bg-red-100 dark:bg-red-950/50">
              <AlertCircle className="w-5 h-5 text-red-600 dark:text-red-400" />
            </div>
            <div className="flex items-center gap-2">
              <span>{getLL().flow.stress_result_sample_dialog_title()}</span>
              <Badge variant="outline" className="font-mono text-xs text-red-600 dark:text-red-400 border-red-200">
                {errorKind}
              </Badge>
              <Badge variant="secondary" className="text-xs">
                {getLL().flow.stress_result_error_samples_count({ count: samples.length })}
              </Badge>
            </div>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 ml-auto cursor-pointer"
              onClick={() => onOpenChange(false)}
            >
              <X className="h-4 w-4" />
            </Button>
          </DialogTitle>
        </DialogHeader>

        {samples.length === 0 ? (
          <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
            {getLL().flow.stress_result_no_samples()}
          </div>
        ) : (
          <div className="flex-1 flex min-h-0 gap-4">
            {/* 左侧样本列表选择（当样本 > 1 时展示导航） */}
            {samples.length > 1 && (
              <div className="w-48 flex-shrink-0 flex flex-col border border-border/50 rounded-xl overflow-hidden bg-muted/20">
                <div className="px-3 py-2 border-b border-border/50 text-xs font-semibold text-muted-foreground">
                  {getLL().flow.stress_result_error_samples()}
                </div>
                <ScrollArea className="flex-1">
                  <div className="p-1.5 flex flex-col gap-1">
                    {samples.map((sample, idx) => (
                      <button
                        key={`${sample.iteration}-${idx}`}
                        type="button"
                        className={cn(
                          'w-full text-left px-2.5 py-2 rounded-lg text-xs transition-colors cursor-pointer flex flex-col gap-1',
                          selectedIndex === idx
                            ? 'bg-background shadow-xs font-medium text-foreground border border-border/60'
                            : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
                        )}
                        onClick={() => setSelectedIndex(idx)}
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-mono font-medium text-foreground">
                            {getLL().flow.stress_result_sample_iteration({ iteration: sample.iteration })}
                          </span>
                          {sample.durationMs !== undefined && (
                            <span className="text-[10px] text-muted-foreground">
                              {sample.durationMs}ms
                            </span>
                          )}
                        </div>
                        {sample.nodeName && (
                          <span className="truncate text-[11px] text-muted-foreground">
                            {sample.nodeName}
                          </span>
                        )}
                      </button>
                    ))}
                  </div>
                </ScrollArea>
              </div>
            )}

            {/* 右侧样本详细快照 */}
            {currentSample && (
              <div className="flex-1 flex flex-col min-h-0 border border-border/50 rounded-xl overflow-hidden bg-background">
                {/* 样本摘要栏 */}
                <div className="px-4 py-2.5 bg-muted/40 border-b border-border/50 flex flex-wrap items-center gap-3 text-xs flex-shrink-0">
                  <div className="flex items-center gap-1.5 font-medium text-foreground font-mono">
                    <span>{getLL().flow.stress_result_sample_iteration({ iteration: currentSample.iteration })}</span>
                  </div>

                  {currentSample.nodeName && (
                    <div className="flex items-center gap-1.5 text-muted-foreground">
                      <Layers className="w-3.5 h-3.5" />
                      <span>{getLL().flow.stress_result_sample_node()}:</span>
                      <span className="font-medium text-foreground truncate max-w-[160px]">
                        {currentSample.nodeName}
                      </span>
                      {currentSample.nodeType && (
                        <Badge variant="outline" className="text-[10px] px-1.5 py-0 h-4">
                          {currentSample.nodeType}
                        </Badge>
                      )}
                    </div>
                  )}

                  {currentSample.durationMs !== undefined && (
                    <div className="flex items-center gap-1.5 text-muted-foreground">
                      <Timer className="w-3.5 h-3.5" />
                      <span>{getLL().flow.stress_result_sample_duration()}:</span>
                      <span className="font-medium text-foreground">
                        {currentSample.durationMs}ms
                      </span>
                    </div>
                  )}

                  {currentSample.timestamp && (
                    <div className="flex items-center gap-1.5 text-muted-foreground ml-auto">
                      <Clock className="w-3.5 h-3.5" />
                      <span className="tabular-nums">
                        {formatTime(currentSample.timestamp)}
                      </span>
                    </div>
                  )}
                </div>

                {/* 滚动详情 */}
                <ScrollArea className="flex-1">
                  <div className="p-4 flex flex-col gap-4">
                    {/* 错误详情卡片 */}
                    <div className="space-y-1.5">
                      <div className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                        <AlertCircle className="w-3.5 h-3.5 text-red-500" />
                        <span>{getLL().flow.stress_result_sample_error_info()}</span>
                      </div>
                      {currentSample.errorDetail ? (
                        <ErrorSection error={currentSample.errorDetail} />
                      ) : (
                        <div className="rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 p-3 text-sm text-red-700 dark:text-red-300 font-mono whitespace-pre-wrap break-all">
                          {currentSample.errorMessage}
                        </div>
                      )}
                    </div>

                    {/* 解析后配置 */}
                    {currentSample.resolvedConfig !== undefined && currentSample.resolvedConfig !== null && (
                      <div className="space-y-1.5">
                        <div className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                          <FileCode className="w-3.5 h-3.5 text-blue-500" />
                          <span>{getLL().flow.stress_result_sample_config()}</span>
                        </div>
                        <div className="rounded-lg border border-border/50 bg-muted/20 p-2.5 overflow-hidden">
                          <RawJsonViewer data={currentSample.resolvedConfig} />
                        </div>
                      </div>
                    )}

                    {/* 请求 / 输入数据 */}
                    {currentSample.inputData !== undefined && currentSample.inputData !== null && (
                      <div className="space-y-1.5">
                        <div className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                          <ArrowRight className="w-3.5 h-3.5 text-emerald-500" />
                          <span>{getLL().flow.stress_result_sample_input()}</span>
                        </div>
                        <div className="rounded-lg border border-border/50 bg-muted/20 p-2.5 overflow-hidden">
                          <RawJsonViewer data={currentSample.inputData} />
                        </div>
                      </div>
                    )}

                    {/* 响应 / 输出数据 */}
                    {currentSample.outputData !== undefined && currentSample.outputData !== null && (
                      <div className="space-y-1.5">
                        <div className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                          <CornerDownRight className="w-3.5 h-3.5 text-violet-500" />
                          <span>{getLL().flow.stress_result_sample_output()}</span>
                        </div>
                        <div className="rounded-lg border border-border/50 bg-muted/20 p-2.5 overflow-hidden">
                          <RawJsonViewer data={currentSample.outputData} />
                        </div>
                      </div>
                    )}
                  </div>
                </ScrollArea>
              </div>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
