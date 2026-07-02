/**
 * ErrorSection - 节点执行错误展示组件
 *
 * 在所有 Viewer 组件中复用的错误卡片。
 * 从 ConditionalViewer / AssertionViewer 的 ErrorCard 模式提取，
 * 统一展示 error_kind、error_code、message、stack_trace。
 */
import { AlertTriangle } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { getLL } from '@/i18n/i18n-lazy';
import type { ErrorDetail } from '@/types';

interface ErrorSectionProps {
  /** 错误详情 */
  error: ErrorDetail;
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

export function ErrorSection({ error }: ErrorSectionProps) {
  return (
    <div className="rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 overflow-hidden">
      {/* 错误头部 */}
      <div className="p-3 bg-red-100/50 dark:bg-red-900/30 border-b border-red-200 dark:border-red-800">
        <div className="flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-red-600 dark:text-red-400 shrink-0" />
          <Badge className="font-mono text-xs bg-red-200 dark:bg-red-800 text-red-800 dark:text-red-200 border-red-300 dark:border-red-700">
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
      </div>

      {/* 错误消息 */}
      <div className="p-3">
        <p className="text-sm text-red-700 dark:text-red-300 font-medium whitespace-pre-wrap break-words">
          {error.message}
        </p>
      </div>

      {/* 堆栈跟踪（可折叠） */}
      {error.stack_trace && (
        <details className="border-t border-red-200 dark:border-red-800">
          <summary className="p-2 text-xs text-red-500 dark:text-red-400 cursor-pointer hover:bg-red-100/50 dark:hover:bg-red-900/30 select-none">
            {getLL().execution.viewers.stack_trace()}
          </summary>
          <pre className="p-2 text-xs text-red-600 dark:text-red-300 bg-red-100/50 dark:bg-red-900/30 overflow-x-auto max-h-48 whitespace-pre-wrap break-all">
            {error.stack_trace}
          </pre>
        </details>
      )}
    </div>
  );
}
