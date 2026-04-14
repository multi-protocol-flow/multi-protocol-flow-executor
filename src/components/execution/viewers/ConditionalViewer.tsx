/**
 * ConditionalViewer - Conditional 节点执行详情查看器
 * 结构化展示条件表达式、评估值和结果
 */
import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Code2,
  GitBranch,
  AlertCircle,
  AlertTriangle,
} from 'lucide-react';
import type { NodeExecutionReport, ErrorDetail } from '@/types';
import { Badge } from '@/components/ui/badge';
import { VariableChangesCard } from './VariableChangesCard';

interface ConditionalViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

/**
 * Conditional 输出数据结构
 */
interface ConditionalOutputData {
  success?: boolean;
  expression?: string;
  evaluated_value?: unknown;
  field_value?: unknown;
  result?: boolean;
  error?: string;
}

/**
 * 条件配置数据结构（从 resolved_config 提取）
 */
interface ConditionalConfigData {
  type?: string;
  condition_type?: string;
  field_path?: string;
  operator?: string;
  expected_value?: unknown;
  advanced_expression?: string;
  expected_value_type?: string;
}

/**
 * 折叠面板组件
 */
interface CollapsibleSectionProps {
  title: string;
  icon: React.ReactNode;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

function CollapsibleSection({ title, icon, defaultOpen = false, children }: CollapsibleSectionProps) {
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
      </button>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/**
 * 值展示组件
 */
interface ValueDisplayProps {
  label: string;
  value: unknown;
}

function ValueDisplay({ label, value }: ValueDisplayProps) {
  if (value === undefined || value === null) {
    return (
      <div className="flex items-start gap-2 text-sm">
        <span className="text-muted-foreground min-w-0 shrink-0">{label}:</span>
        <span className="text-muted-foreground italic">未设置</span>
      </div>
    );
  }

  const displayValue = typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);

  return (
    <div className="space-y-1">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div className="bg-muted rounded p-2 font-mono text-sm text-foreground whitespace-pre-wrap break-all">
        {displayValue}
      </div>
    </div>
  );
}

/**
 * 错误类型映射为中文
 */
function getErrorKindLabel(errorKind: string): string {
  const labels: Record<string, string> = {
    'condition_syntax': '条件语法错误',
    'condition_evaluation': '条件评估错误',
    'configuration': '配置错误',
    'execution': '执行错误',
    'connection': '连接错误',
    'timeout': '超时错误',
    'validation': '验证错误',
    'parse': '解析错误',
  };
  return labels[errorKind] || errorKind;
}

/**
 * 操作符映射为显示符号
 */
function getOperatorDisplay(operator?: string): string {
  if (!operator) return '-';
  const operators: Record<string, string> = {
    'equal': '==',
    'not_equal': '!=',
    'greater': '>',
    'greater_or_equal': '>=',
    'less': '<',
    'less_or_equal': '<=',
    'contains': 'contains',
    'starts_with': 'startsWith',
    'ends_with': 'endsWith',
    'exists': 'exists',
  };
  return operators[operator] || operator;
}

/**
 * 错误卡片组件
 */
function ErrorCard({ error, configData }: { error: ErrorDetail; configData?: ConditionalConfigData }) {
  return (
    <div className="rounded-lg bg-red-50 dark:bg-red-950/50 border border-red-200 dark:border-red-800 overflow-hidden">
      {/* 错误头部 */}
      <div className="p-3 bg-red-100/50 dark:bg-red-900/30 border-b border-red-200 dark:border-red-800">
        <div className="flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-red-600" />
          <Badge className="font-mono text-xs bg-red-200 text-red-800 border-red-300">
            {getErrorKindLabel(error.error_kind)}
          </Badge>
          {error.error_code && (
            <Badge variant="outline" className="font-mono text-xs text-red-600 border-red-300">
              {error.error_code}
            </Badge>
          )}
        </div>
      </div>
      
      {/* 错误消息 */}
      <div className="p-3 border-b border-red-200 dark:border-red-800">
        <p className="text-sm text-red-700 dark:text-red-300 font-medium">{error.message}</p>
      </div>

      {/* 原始条件配置 */}
      {configData && (
        <div className="p-3 bg-red-50/50 dark:bg-red-950/30">
          <div className="text-xs font-medium text-red-600 dark:text-red-400 mb-2">原始条件配置:</div>
          <div className="space-y-1.5 text-sm">
            {configData.condition_type && (
              <div className="flex gap-2">
                <span className="text-muted-foreground shrink-0">条件类型:</span>
                <span className="font-mono text-red-700 dark:text-red-300">
                  {configData.condition_type === 'simple' ? '简单条件' : '高级表达式'}
                </span>
              </div>
            )}
            {configData.condition_type === 'simple' && (
              <>
                {configData.field_path && (
                  <div className="flex gap-2">
                    <span className="text-muted-foreground shrink-0">字段路径:</span>
                    <code className="font-mono text-red-700 dark:text-red-300 bg-red-100 dark:bg-red-900/50 px-1 rounded text-xs">
                      {configData.field_path}
                    </code>
                  </div>
                )}
                {configData.operator && (
                  <div className="flex gap-2">
                    <span className="text-muted-foreground shrink-0">操作符:</span>
                    <span className="font-mono text-red-700 dark:text-red-300">
                      {getOperatorDisplay(configData.operator)}
                    </span>
                  </div>
                )}
                {configData.expected_value !== undefined && (
                  <div className="flex gap-2">
                    <span className="text-muted-foreground shrink-0">期望值:</span>
                    <code className="font-mono text-red-700 dark:text-red-300 bg-red-100 dark:bg-red-900/50 px-1 rounded text-xs">
                      {typeof configData.expected_value === 'object' 
                        ? JSON.stringify(configData.expected_value) 
                        : String(configData.expected_value)}
                    </code>
                  </div>
                )}
                {configData.expected_value_type && (
                  <div className="flex gap-2">
                    <span className="text-muted-foreground shrink-0">期望值类型:</span>
                    <span className="font-mono text-red-700 dark:text-red-300">
                      {configData.expected_value_type}
                    </span>
                  </div>
                )}
              </>
            )}
            {configData.condition_type === 'advanced' && configData.advanced_expression && (
              <div className="flex flex-col gap-1">
                <span className="text-muted-foreground">高级表达式:</span>
                <code className="font-mono text-red-700 dark:text-red-300 bg-red-100 dark:bg-red-900/50 px-2 py-1 rounded text-xs whitespace-pre-wrap">
                  {configData.advanced_expression}
                </code>
              </div>
            )}
          </div>
        </div>
      )}

      {/* 堆栈跟踪 */}
      {error.stack_trace && (
        <details className="border-t border-red-200 dark:border-red-800">
          <summary className="p-2 text-xs text-red-500 dark:text-red-400 cursor-pointer hover:bg-red-100/50 dark:hover:bg-red-900/30">
            堆栈跟踪
          </summary>
          <pre className="p-2 text-xs text-red-600 dark:text-red-400 bg-red-100/50 dark:bg-red-900/30 overflow-x-auto max-h-32">
            {error.stack_trace}
          </pre>
        </details>
      )}
    </div>
  );
}

export function ConditionalViewer({ report }: ConditionalViewerProps) {
  // 从 output_data 提取 Conditional 数据
  const outputData = report.output_data as ConditionalOutputData | undefined;
  
  // 从 resolved_config 提取原始条件配置（始终包含节点配置）
  const configData = report.resolved_config as ConditionalConfigData | undefined;
  
  const expression = outputData?.expression;
  const evaluatedValue = outputData?.evaluated_value;
  const fieldValue = outputData?.field_value;
  const result = outputData?.result;

  // 节点级错误（从 report.error 读取）
  const nodeError = report.error;
  
  // output_data 中的错误（向后兼容）
  const outputError = outputData?.error;

  // 判断是否有错误：
  // 1. report.status 为 failed
  // 2. 存在节点级错误 (report.error 不为 null/undefined)
  // 3. output_data 中明确标记 success: false
  // 4. output_data 中有 error 字段
  const hasError = report.status === 'failed' 
    || nodeError != null  // 使用 != null 同时检查 null 和 undefined
    || outputData?.success === false 
    || Boolean(outputError);

  // 如果有节点级错误，显示错误卡片（优先级最高）
  if (nodeError) {
    return (
      <div className="space-y-3">
        <ErrorCard error={nodeError} configData={configData} />
        
        {/* 如果有 output_data，也显示评估信息（可能部分可用） */}
        {outputData && (expression || fieldValue !== undefined) && (
          <>
            {expression && (
              <CollapsibleSection
                title="条件表达式"
                icon={<Code2 className="w-4 h-4" />}
                defaultOpen={false}
              >
                <div className="bg-muted rounded p-2 font-mono text-sm text-foreground">
                  {expression}
                </div>
              </CollapsibleSection>
            )}
            {fieldValue !== undefined && (
              <CollapsibleSection
                title="评估值"
                icon={<GitBranch className="w-4 h-4" />}
                defaultOpen={false}
              >
                <ValueDisplay label="字段值 (field_value)" value={fieldValue} />
              </CollapsibleSection>
            )}
          </>
        )}

        {/* 变量变更 */}
        <VariableChangesCard changes={report.variable_changes} />
      </div>
    );
  }

  // 正常情况：显示完整的条件评估信息
  return (
    <div className="space-y-3">
      {/* 条件表达式 */}
      <CollapsibleSection
        title="条件表达式"
        icon={<Code2 className="w-4 h-4" />}
        defaultOpen={true}
      >
        {expression ? (
          <div className="bg-muted rounded p-2 font-mono text-sm text-foreground">
            {expression}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground italic">未配置条件表达式</p>
        )}
      </CollapsibleSection>

      {/* 评估值 */}
      <CollapsibleSection
        title="评估值"
        icon={<GitBranch className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          <ValueDisplay label="字段值 (field_value)" value={fieldValue} />
          <ValueDisplay label="评估值 (evaluated_value)" value={evaluatedValue} />
        </div>
      </CollapsibleSection>

      {/* 评估结果 */}
      <CollapsibleSection
        title="评估结果"
        icon={<GitBranch className="w-4 h-4" />}
        defaultOpen={true}
      >
        {hasError ? (
          // output_data 中的错误（向后兼容）
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <AlertCircle className="w-4 h-4 text-red-600" />
              <Badge className="font-mono text-sm px-3 py-1 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50">
                评估失败
              </Badge>
            </div>
            {outputError && (
              <div className="bg-red-50 dark:bg-red-950/50 rounded p-2 text-sm text-red-700 dark:text-red-300">
                {outputError}
              </div>
            )}
          </div>
        ) : result !== undefined ? (
          // 正常结果
          <div className="flex items-center gap-2">
            {result === true ? (
              <>
                <Badge className="font-mono text-xs px-2 py-0.5 text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950/50">
                  true
                </Badge>
                <span className="text-sm text-muted-foreground">条件成立</span>
              </>
            ) : (
              <>
                <Badge className="font-mono text-xs px-2 py-0.5 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50">
                  false
                </Badge>
                <span className="text-sm text-muted-foreground">条件不成立</span>
              </>
            )}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground italic">无评估结果</p>
        )}
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
