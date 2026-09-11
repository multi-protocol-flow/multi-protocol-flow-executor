/**
 * ErrorSection - 节点执行错误展示组件
 *
 * 在所有 Viewer 组件中复用的错误卡片。
 * 从 ConditionalViewer / AssertionViewer 的 ErrorCard 模式提取，
 * 统一展示 error_kind、error_code、message、stack_trace。
 */
import { AlertTriangle, Sparkles } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { getLL } from '@/i18n/i18n-lazy';
import { resolveErrorCode } from '@/services/tauri/flow';
import { useAIStore } from '@/stores/ai';
import type { ErrorDetail } from '@/types';
import { CopyButton } from '@/components/common';

interface ErrorSectionProps {
  /** 错误详情 */
  error: ErrorDetail;
  /** 节点名称 */
  nodeName?: string;
  /** 节点类型 */
  nodeType?: string;
  /** 节点 UUID */
  nodeUuid?: string;
}

/**
 * 错误类型映射为可读标签
 */
function getErrorKindLabel(errorKind: string): string {
  const ll = getLL();
  const labels: Record<string, string> = {
    condition_syntax: ll.viewer.common.error_kind_condition_syntax(),
    condition_evaluation: ll.viewer.common.error_kind_condition_evaluation(),
    configuration: ll.viewer.common.error_kind_configuration(),
    execution: ll.viewer.common.error_kind_execution(),
    connection: ll.viewer.common.error_kind_connection(),
    timeout: ll.viewer.common.error_kind_timeout(),
    validation: ll.viewer.common.error_kind_validation(),
    parse: ll.viewer.common.error_kind_parse(),
  };
  return labels[errorKind] || errorKind;
}

export function ErrorSection({ error, nodeName, nodeType }: ErrorSectionProps) {
  const ll = getLL();
  const resolvedMsg = error.error_code
    ? resolveErrorCode(error.error_code, error.i18n_args)
    : error.message;

  const fullErrorReport = [
    `【${nodeName || '未知节点'}】(${nodeType || 'node'}) 执行失败`,
    `错误类型: ${error.error_kind}`,
    error.error_code ? `错误代码: ${error.error_code}` : null,
    `错误详情: ${resolvedMsg}`,
    error.stack_trace ? `\n堆栈跟踪:\n${error.stack_trace}` : null,
  ]
    .filter(Boolean)
    .join('\n');

  const handleDiagnoseWithAI = (e: React.MouseEvent) => {
    e.stopPropagation();
    useAIStore.getState().setCopilotOpen(true);
    const prompt = nodeName
      ? ll.ai.copilot.prompt_diagnose_node({
          nodeName,
          nodeType: nodeType || 'node',
          errorKind: error.error_kind,
          errorMessage: resolvedMsg,
        })
      : ll.ai.copilot.prompt_diagnose_current_node({
          errorKind: error.error_kind,
          errorMessage: resolvedMsg,
        });
    useAIStore.getState().sendMessage(prompt);
  };
  return (
    <div className="rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 overflow-hidden">
      {/* 错误头部 */}
      <div className="p-2.5 bg-red-100/50 dark:bg-red-900/30 border-b border-red-200 dark:border-red-800 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <AlertTriangle className="w-4 h-4 text-red-600 dark:text-red-400 shrink-0" />
          {nodeName && (
            <div className="flex items-center gap-1 min-w-0">
              <span className="text-xs font-semibold text-red-900 dark:text-red-200 truncate select-text cursor-text">
                {nodeName}
              </span>
              <CopyButton
                value={nodeName}
                title={ll.execution.viewers.copy_node_name()}
                size="xs"
                className="h-5 p-0.5 bg-background/80 text-red-700 dark:text-red-300 border-red-300/60 shrink-0"
              />
            </div>
          )}
          <Badge className="font-mono text-xs bg-red-200 dark:bg-red-800 text-red-800 dark:text-red-200 border-red-300 dark:border-red-700 shrink-0">
            {getErrorKindLabel(error.error_kind)}
          </Badge>
          {error.error_code && (
            <Badge
              variant="outline"
              className="font-mono text-xs text-red-600 dark:text-red-400 border-red-300 dark:border-red-700"
            >
              {error.error_code}
            </Badge>
          )}
        </div>

        {/* 操作区：复制错误报告 + AI 诊断 */}
        <div className="flex items-center gap-1.5 shrink-0">
          <CopyButton
            value={fullErrorReport}
            label={ll.execution.viewers.copy_error_report()}
            title={ll.execution.viewers.copy_error_report()}
            size="xs"
            className="bg-background/90 text-red-700 dark:text-red-300 border-red-300/80 dark:border-red-700/80 hover:bg-background"
          />
          <button
            type="button"
            onClick={handleDiagnoseWithAI}
            className="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium rounded border border-red-300/80 dark:border-red-700/80 bg-background/90 hover:bg-background text-red-700 dark:text-red-300 shadow-xs hover:shadow transition-all shrink-0 cursor-pointer"
            title={ll.ai.copilot.diagnose_and_fix_node()}
          >
            <Sparkles className="w-3.5 h-3.5 text-amber-500 fill-amber-500/20" />
            <span>{ll.ai.copilot.diagnose_and_fix_node()}</span>
          </button>
        </div>
      </div>
      <div className="p-3 flex items-start justify-between gap-2 group">
        <p className="text-sm text-red-700 dark:text-red-300 font-medium whitespace-pre-wrap break-words select-text cursor-text flex-1">
          {resolvedMsg}
        </p>
        <CopyButton
          value={resolvedMsg}
          title={ll.execution.viewers.copy_error_message()}
          size="xs"
          className="opacity-0 group-hover:opacity-100 transition-opacity bg-background/80 text-red-700 dark:text-red-300 border-red-300/60 shrink-0"
        />
      </div>
      {/* 堆栈跟踪（可折叠） */}
      {error.stack_trace && (
        <details className="border-t border-red-200 dark:border-red-800">
          <summary className="p-2 text-xs text-red-500 dark:text-red-400 cursor-pointer hover:bg-red-100/50 dark:hover:bg-red-900/30 select-none flex items-center justify-between">
            <span>{getLL().execution.viewers.stack_trace()}</span>
            <CopyButton
              value={error.stack_trace}
              label={ll.execution.viewers.copy_stack_trace()}
              title={ll.execution.viewers.copy_stack_trace()}
              size="xs"
              className="bg-background/80 text-red-700 dark:text-red-300 border-red-300/60 hover:bg-background"
            />
          </summary>
          <pre className="p-2 text-xs text-red-600 dark:text-red-300 bg-red-100/50 dark:bg-red-900/30 overflow-x-auto max-h-48 whitespace-pre-wrap break-all select-text cursor-text">
            {error.stack_trace}
          </pre>
        </details>
      )}
    </div>
  );
}
