/**
 * AssertionViewer - Assertion 节点执行详情查看器
 * 结构化展示断言执行结果、统计信息和失败详情
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  CheckCircle,
  XCircle,
  AlertTriangle,
  FileCheck,
} from 'lucide-react';
import type { NodeExecutionReport, ErrorDetail } from '@/types';
import type { AssertionOutput, AssertionResult, AssertionNodeConfig } from '@/types/node';
import { Badge } from '@/components/ui/badge';
import { VariableChangesCard } from './VariableChangesCard';
import { cn } from '@/lib/utils';

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
  children: React.ReactNode;
}

function CollapsibleSection({ title, icon, defaultOpen = false, badge, children }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full flex items-center gap-2 p-3 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
        aria-expanded={isOpen}
      >
        <div className="text-muted-foreground">
          {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        </div>
        <div className="text-muted-foreground">{icon}</div>
        <span className="font-medium text-foreground text-sm">{title}</span>
        {badge && <div className="ml-auto">{badge}</div>}
      </button>
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

  return (
    <div className="rounded-lg bg-red-50 border border-red-200 overflow-hidden">
      {/* 失败头部 */}
      <button
        type="button"
        onClick={() => setShowDetails(!showDetails)}
        className="w-full p-3 bg-red-100/50 hover:bg-red-100/70 transition-colors text-left"
      >
        <div className="flex items-center gap-2">
          <XCircle className="w-4 h-4 text-red-600 shrink-0" />
          <span className="font-medium text-red-700 text-sm">
            {result.description || `断言 #${index + 1}`}
          </span>
          <div className="ml-auto text-muted-foreground">
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
        <div className="p-3 border-t border-red-200 space-y-3">
          {/* 目标和操作符 */}
          <div className="space-y-1.5">
            <div className="flex items-start gap-2 text-sm">
              <span className="text-muted-foreground shrink-0 min-w-16">目标:</span>
              <code className="font-mono text-red-700 bg-red-100 px-1.5 py-0.5 rounded text-xs break-all">
                {result.target}
              </code>
            </div>
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground shrink-0 min-w-16">操作符:</span>
              <span className="font-mono text-red-700">
                {getOperatorDisplay(result.operator)}
              </span>
            </div>
          </div>

          {/* 期望值 vs 实际值 */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <div className="text-xs font-medium text-muted-foreground">期望值</div>
              <div className="bg-background/60 rounded p-2 font-mono text-sm text-foreground break-all">
                {formatValue(result.expected)}
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-xs font-medium text-muted-foreground">实际值</div>
              <div className="bg-background/60 rounded p-2 font-mono text-sm text-foreground break-all">
                {formatValue(result.actual)}
              </div>
            </div>
          </div>

          {/* 错误信息 */}
          {result.error && (
            <div className="rounded bg-red-100/70 p-2">
              <div className="flex items-start gap-2">
                <AlertTriangle className="w-4 h-4 text-red-600 shrink-0 mt-0.5" />
                <p className="text-sm text-red-700">{result.error}</p>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * 错误卡片组件
 */
function ErrorCard({ error }: { error: ErrorDetail }) {
  return (
    <div className="rounded-lg bg-red-50 border border-red-200 overflow-hidden">
      {/* 错误头部 */}
      <div className="p-3 bg-red-100/50 border-b border-red-200">
        <div className="flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-red-600" />
          <Badge className="font-mono text-xs bg-red-200 text-red-800 border-red-300">
            执行错误
          </Badge>
          {error.error_code && (
            <Badge variant="outline" className="font-mono text-xs text-red-600 border-red-300">
              {error.error_code}
            </Badge>
          )}
        </div>
      </div>
      
      {/* 错误消息 */}
      <div className="p-3">
        <p className="text-sm text-red-700 font-medium">{error.message}</p>
      </div>

      {/* 堆栈跟踪 */}
      {error.stack_trace && (
        <details className="border-t border-red-200">
          <summary className="p-2 text-xs text-red-500 cursor-pointer hover:bg-red-100/50">
            堆栈跟踪
          </summary>
          <pre className="p-2 text-xs text-red-600 bg-red-100/50 overflow-x-auto max-h-32">
            {error.stack_trace}
          </pre>
        </details>
      )}
    </div>
  );
}

export function AssertionViewer({ report }: AssertionViewerProps) {
  // 从 output_data 提取 Assertion 数据
  const outputData = report.output_data as AssertionOutput | undefined;
  
  // 从 resolved_config 提取断言配置
  const configData = report.resolved_config as AssertionNodeConfig | undefined;

  // 节点级错误（从 report.error 读取）
  const nodeError = report.error;

  // 如果有节点级错误，显示错误卡片
  if (nodeError) {
    return (
      <div className="space-y-3">
        <ErrorCard error={nodeError} />
        
        {/* 如果有 output_data，也显示部分信息 */}
        {outputData && (
          <CollapsibleSection
            title="断言结果"
            icon={<FileCheck className="w-4 h-4" />}
            defaultOpen={false}
          >
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <XCircle className="w-4 h-4 text-red-600" />
                <Badge className="font-mono text-sm px-3 py-1 text-red-600 bg-red-50">
                  执行失败
                </Badge>
              </div>
              {outputData.total !== undefined && (
                <div className="text-sm text-muted-foreground">
                  共 {outputData.total} 条断言，通过 {outputData.passed_count} 条
                </div>
              )}
            </div>
          </CollapsibleSection>
        )}

        {/* 变量变更 */}
        <VariableChangesCard changes={report.variable_changes} />
      </div>
    );
  }

  // 无数据情况
  if (!outputData) {
    return (
      <div className="space-y-3">
        <div className="text-sm text-muted-foreground italic p-3 bg-muted/30 rounded-lg">
          无断言执行数据
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
          ? 'bg-green-50 border-green-200'
          : 'bg-red-50 border-red-200'
      )}>
        <div className="flex items-center gap-3">
          {/* 通过/失败图标 */}
          {passed ? (
            <CheckCircle className="w-6 h-6 text-green-600" />
          ) : (
            <XCircle className="w-6 h-6 text-red-600" />
          )}
          
          {/* 统计信息 */}
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <span className={cn(
                'font-semibold',
                passed ? 'text-green-700' : 'text-red-700'
              )}>
                {passed ? '全部通过' : '断言失败'}
              </span>
              <Badge className={cn(
                'font-mono text-xs',
                passed
                  ? 'bg-green-100 text-green-700 border-green-200'
                  : 'bg-red-100 text-red-700 border-red-200'
              )}>
                {passed_count} / {total}
              </Badge>
            </div>
            <p className={cn(
              'text-sm mt-1',
              passed ? 'text-green-600' : 'text-red-600'
            )}>
              {hasFailures
                ? `${failed_count} 条断言失败`
                : '所有断言均已通过验证'
              }
            </p>
          </div>

          {/* 组合模式 */}
          {configData?.mode && (
            <Badge variant="outline" className="text-xs">
              {configData.mode === 'all' ? '全部满足' : '任一满足'}
            </Badge>
          )}
        </div>
      </div>

      {/* 失败详情区域 (可折叠) */}
      {hasFailures && failed_assertions && failed_assertions.length > 0 && (
        <CollapsibleSection
          title="失败详情"
          icon={<AlertTriangle className="w-4 h-4 text-red-500" />}
          defaultOpen={true}
          badge={
            <Badge className="bg-red-100 text-red-700 border-red-200 font-mono text-xs">
              {failed_count} 条
            </Badge>
          }
        >
          <div className="space-y-2">
            {failed_assertions.map((result, index) => (
              <FailedAssertionCard
                key={result.id || `failed-${index}`}
                result={result}
                index={results.indexOf(result)}
              />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 全部断言列表 (可折叠，默认关闭) */}
      {results && results.length > 0 && (
        <CollapsibleSection
          title="全部断言"
          icon={<FileCheck className="w-4 h-4" />}
          defaultOpen={false}
          badge={
            <Badge variant="outline" className="font-mono text-xs">
              {results.length} 条
            </Badge>
          }
        >
          <div className="space-y-1.5">
            {results.map((result, index) => (
              <div
                key={result.id || `assertion-${index}`}
                className={cn(
                  'flex items-center gap-2 p-2 rounded text-sm',
                  result.passed
                    ? 'bg-green-50/50 text-green-700'
                    : 'bg-red-50/50 text-red-700'
                )}
              >
                {result.passed ? (
                  <CheckCircle className="w-4 h-4 shrink-0" />
                ) : (
                  <XCircle className="w-4 h-4 shrink-0" />
                )}
                <span className="truncate flex-1">
                  {result.description || `断言 #${index + 1}`}
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
}
