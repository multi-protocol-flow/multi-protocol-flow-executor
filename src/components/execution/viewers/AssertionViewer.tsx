/**
 * AssertionViewer - Assertion 节点执行详情查看器
 * 结构化展示断言执行结果、统计信息和失败详情
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  CheckCircle,
  XCircle,
  AlertTriangle,
  FileCheck,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import type { AssertionOutput, AssertionResult, AssertionNodeConfig } from '@/types/node';
import { Badge } from '@/components/ui/badge';
import { VariableChangesCard } from './VariableChangesCard';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { CopyButton } from '@/components/common';
interface AssertionViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

/**
 * 折叠面板组件
 */
interface CollapsibleSectionProps {
  title: string;
  icon: React.ReactNode;
  defaultOpen?: boolean;
  badge?: React.ReactNode;
  extra?: React.ReactNode;
  children: React.ReactNode;
}

function CollapsibleSection({ title, icon, defaultOpen = false, badge, extra, children }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <div className="flex flex-wrap items-center justify-between w-full bg-muted/30 hover:bg-muted/50 transition-colors gap-2 pr-3">
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex-1 min-w-0 flex items-center gap-2 p-3 text-left"
          aria-expanded={isOpen}
        >
          <div className="text-muted-foreground shrink-0">
            {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </div>
          <div className="text-muted-foreground shrink-0">{icon}</div>
          <span className="font-medium text-foreground text-sm truncate min-w-0">{title}</span>
        </button>
        {(badge || extra) && (
          <div className="flex items-center gap-2 shrink-0 pl-3 sm:pl-0 py-1">
            {badge}
            {extra}
          </div>
        )}
      </div>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/**
 * 操作符映射为显示符号
 */
function getOperatorDisplay(operator: string): string {
  const operators: Record<string, string> = {
    'eq': '==',
    'ne': '!=',
    'gt': '>',
    'gte': '>=',
    'lt': '<',
    'lte': '<=',
    'contains': 'contains',
    'not_contains': '!contains',
    'matches': 'matches',
    'exists': 'exists',
    'not_exists': '!exists',
    'in': 'in',
    'not_in': '!in',
    'type_is': 'typeIs',
    'between': 'between',
  };
  return operators[operator] || operator;
}

/**
 * 格式化值用于显示
 */
function formatValue(value: unknown): string {
  if (value === null) return 'null';
  if (value === undefined) return 'undefined';
  if (typeof value === 'string') {
    return value.length > 100 ? `${value.slice(0, 100)}...` : value;
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

/**
 * 单条失败断言卡片
 */
function FailedAssertionCard({ result, index }: { result: AssertionResult; index: number }) {
  const [showDetails, setShowDetails] = useState(false);
  const ll = getLL();
  return (
    <div className="rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 overflow-hidden">
      {/* 失败头部 */}
      <button
        type="button"
        onClick={() => setShowDetails(!showDetails)}
        className="w-full p-3 bg-red-100/50 dark:bg-red-900/30 hover:bg-red-100/70 dark:hover:bg-red-900/50 transition-colors text-left"
      >
        <div className="flex items-center gap-2 min-w-0">
          <XCircle className="w-4 h-4 text-red-600 dark:text-red-400 shrink-0" />
          <span className="font-medium text-red-700 dark:text-red-300 text-sm truncate min-w-0">
            {result.description || ll.execution.viewers.assertion_item({ index: index + 1 })}
          </span>
          <div className="ml-auto text-muted-foreground shrink-0">
            {showDetails ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </div>
        </div>
      </button>

      {/* 失败详情 */}
      {showDetails && (
        <div className="p-3 border-t border-red-200 dark:border-red-800 space-y-3">
          {/* 目标和操作符 */}
          <div className="space-y-1.5">
            <div className="flex items-start gap-2 text-sm">
              <span className="text-muted-foreground shrink-0 min-w-16">{getLL().execution.viewers.target_label()}:</span>
              <code className="font-mono text-red-700 dark:text-red-300 bg-red-100 dark:bg-red-900/50 px-1.5 py-0.5 rounded text-xs break-all select-text">
                {result.target}
              </code>
              <CopyButton
                value={result.target}
                title={ll.execution.viewers.copy_target()}
                size="xs"
                className="h-5 p-0.5"
              />
            </div>
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground shrink-0 min-w-16">{getLL().execution.viewers.operator_label()}:</span>
              <span className="font-mono text-red-700 dark:text-red-300">
                {getOperatorDisplay(result.operator)}
              </span>
            </div>
          </div>

          {/* 期望值 vs 实际值 */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <div className="flex flex-wrap items-center justify-between gap-1">
                <span className="text-xs font-medium text-muted-foreground truncate min-w-0">{getLL().execution.viewers.assertion_expected()}</span>
                <CopyButton
                  value={result.expected}
                  format="json"
                  title={ll.execution.viewers.copy_expected()}
                  size="xs"
                  className="h-5 p-0.5 shrink-0"
                />
              </div>
              <div className="bg-background/60 rounded p-2 font-mono text-sm text-foreground break-all select-text cursor-text">
                {formatValue(result.expected)}
              </div>
            </div>
            <div className="space-y-1">
              <div className="flex flex-wrap items-center justify-between gap-1">
                <span className="text-xs font-medium text-muted-foreground truncate min-w-0">{getLL().execution.viewers.assertion_actual()}</span>
                <CopyButton
                  value={result.actual}
                  format="json"
                  title={ll.execution.viewers.copy_actual()}
                  size="xs"
                  className="h-5 p-0.5 shrink-0"
                />
              </div>
              <div className="bg-background/60 rounded p-2 font-mono text-sm text-foreground break-all select-text cursor-text">
                {formatValue(result.actual)}
              </div>
            </div>
          </div>
          {/* 错误信息 */}
          {result.error && (
            <div className="rounded bg-red-100/70 dark:bg-red-900/40 p-2">
              <div className="flex items-start justify-between gap-2">
                <div className="flex items-start gap-2 flex-1">
                  <AlertTriangle className="w-4 h-4 text-red-600 dark:text-red-400 shrink-0 mt-0.5" />
                  <p className="text-sm text-red-700 dark:text-red-300 select-text cursor-text">{result.error}</p>
                </div>
                <CopyButton
                  value={result.error}
                  title={ll.execution.viewers.copy_error_message()}
                  size="xs"
                  className="h-5 p-0.5 shrink-0"
                />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export const AssertionViewer = memo(function AssertionViewer({ report }: AssertionViewerProps) {
  const ll = getLL();
  // 从 output_data 提取 Assertion 数据
  const outputData = report.output_data as AssertionOutput | undefined;
  
  // 从 resolved_config 提取断言配置
  const configData = report.resolved_config as AssertionNodeConfig | undefined;

  // 无数据情况
  if (!outputData) {
    return (
      <div className="space-y-3">
        <div className="text-sm text-muted-foreground italic p-3 bg-muted/30 rounded-lg">
          {getLL().execution.viewers.no_assertion_data()}
        </div>
        <VariableChangesCard changes={report.variable_changes} />
      </div>
    );
  }

  const { passed, total, passed_count, failed_count, results, failed_assertions } = outputData;
  const hasFailures = failed_count > 0;

  return (
    <div className="space-y-3">
      {/* 整体结果区域 */}
      <div className={cn(
        'rounded-lg p-4 border',
        passed
          ? 'bg-green-50 dark:bg-green-950/30 border-green-200 dark:border-green-800'
          : 'bg-red-50 dark:bg-red-950/30 border-red-200 dark:border-red-800'
      )}>
        <div className="flex items-center gap-3">
          {/* 通过/失败图标 */}
          {passed ? (
            <CheckCircle className="w-6 h-6 text-green-600 dark:text-green-400" />
          ) : (
            <XCircle className="w-6 h-6 text-red-600 dark:text-red-400" />
          )}
          
          {/* 统计信息 */}
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <span className={cn(
                'font-semibold',
                passed ? 'text-green-700 dark:text-green-300' : 'text-red-700 dark:text-red-300'
              )}>
                {passed ? getLL().execution.viewers.assertion_all_passed() : getLL().execution.viewers.assertion_fail()}
              </span>
              <Badge className={cn(
                'font-mono text-xs',
                passed
                  ? 'bg-green-100 dark:bg-green-900/50 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800'
                  : 'bg-red-100 dark:bg-red-900/50 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800'
              )}>
                {passed_count} / {total}
              </Badge>
            </div>
            <p className={cn(
              'text-sm mt-1',
              passed ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'
            )}>
              {hasFailures
                ? getLL().execution.viewers.assertion_failed({ count: failed_count })
                : getLL().execution.viewers.assertion_all_passed_desc()
              }
            </p>
          </div>

          {/* 组合模式 */}
          {configData?.mode && (
            <Badge variant="outline" className="text-xs">
              {configData.mode === 'all' ? getLL().execution.viewers.assertion_mode_all() : getLL().execution.viewers.assertion_mode_any()}
            </Badge>
          )}
        </div>
      </div>

      {/* 失败详情区域 (可折叠) */}
      {hasFailures && failed_assertions && failed_assertions.length > 0 && (
        <CollapsibleSection
          title={getLL().execution.viewers.assertion_fail_details()}
          icon={<AlertTriangle className="w-4 h-4 text-red-500" />}
          defaultOpen={true}
          badge={
            <Badge className="bg-red-100 dark:bg-red-900/50 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800 font-mono text-xs">
              {getLL().viewer.assert.failed_items({count: failed_count})}
            </Badge>
          }
        >
          <div className="space-y-2">
            {failed_assertions.map((result, index) => {
              const matchedIndex = results?.findIndex(
                (r) => (result.id && r.id === result.id) || r === result
              );
              const resolvedIndex = matchedIndex !== undefined && matchedIndex >= 0 ? matchedIndex : index;
              return (
                <FailedAssertionCard
                  key={result.id || `failed-${index}`}
                  result={result}
                  index={resolvedIndex}
                />
              );
            })}
          </div>
        </CollapsibleSection>
      )}

      {/* 全部断言列表 (可折叠，默认关闭) */}
      {results && results.length > 0 && (
        <CollapsibleSection
          title={getLL().execution.viewers.assertion_all_list()}
          icon={<FileCheck className="w-4 h-4" />}
          defaultOpen={!hasFailures}
          badge={
            <Badge variant="outline" className="font-mono text-xs">
              {getLL().viewer.assert.total_items({count: results.length})}
            </Badge>
          }
          extra={
            <CopyButton
              value={results}
              format="json"
              label={ll.execution.report.copy_all()}
              size="xs"
            />
          }
        >
          <div className="space-y-1.5">
            {results.map((result, index) => (
              <div
                key={result.id || `assertion-${index}`}
                className={cn(
                  'flex items-center gap-2 p-2 rounded text-sm',
                  result.passed
                    ? 'bg-green-50/50 dark:bg-green-950/30 text-green-700 dark:text-green-300'
                    : 'bg-red-50/50 dark:bg-red-950/30 text-red-700 dark:text-red-300'
                )}
              >
                {result.passed ? (
                  <CheckCircle className="w-4 h-4 shrink-0" />
                ) : (
                  <XCircle className="w-4 h-4 shrink-0" />
                )}
                <span className="truncate flex-1">
                  {result.description || ll.execution.viewers.assertion_item({ index: index + 1 })}
                </span>
                <code className="font-mono text-xs opacity-70">
                  {result.target}
                </code>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
});
