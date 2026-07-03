/**
 * ConditionalViewer - Conditional 节点执行详情查看器
 * 结构化展示条件表达式、评估值和结果
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Code2,
  GitBranch,
  AlertCircle,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';

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
        <span className="text-muted-foreground italic">{getLL().execution.viewers.no_data()}</span>
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

export const ConditionalViewer = memo(function ConditionalViewer({ report }: ConditionalViewerProps) {
  const ll = getLL();
  // 从 output_data 提取 Conditional 数据
  const outputData = report.output_data as ConditionalOutputData | undefined;
  
  const expression = outputData?.expression;
  const evaluatedValue = outputData?.evaluated_value;
  const fieldValue = outputData?.field_value;
  const result = outputData?.result;

  // ErrorSection (在 NodeExecutionDetails 中) 已经负责渲染结构化错误信息
  const hasError = report.status === 'failed';

  // 显示条件评估信息
  return (
    <div className="space-y-3">
      {/* 条件表达式 */}
      <CollapsibleSection
        title={ll.execution.viewers.condition_expression()}
        icon={<Code2 className="w-4 h-4" />}
        defaultOpen={true}
      >
        {expression ? (
          <div className="bg-muted rounded p-2 font-mono text-sm text-foreground">
            {expression}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground italic">{ll.execution.viewers.condition_not_configured()}</p>
        )}
      </CollapsibleSection>

      {/* 评估值 */}
      <CollapsibleSection
        title={ll.execution.viewers.eval_value()}
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
        title={ll.execution.viewers.eval_result()}
        icon={<GitBranch className="w-4 h-4" />}
        defaultOpen={true}
      >
        {hasError ? (
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-red-600" />
            <Badge className="font-mono text-sm px-3 py-1 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50">
              {ll.execution.viewers.evaluation_failed()}
            </Badge>
          </div>
        ) : result !== undefined ? (
          // 正常结果
          <div className="flex items-center gap-2">
            {result === true ? (
              <>
                <Badge className="font-mono text-xs px-2 py-0.5 text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950/50">
                  true
                </Badge>
                <span className="text-sm text-muted-foreground">{ll.execution.viewers.condition_match()}</span>
              </>
            ) : (
              <>
                <Badge className="font-mono text-xs px-2 py-0.5 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50">
                  false
                </Badge>
                <span className="text-sm text-muted-foreground">{ll.execution.viewers.condition_no_match()}</span>
              </>
            )}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground italic">{ll.execution.viewers.no_evaluation_result()}</p>
        )}
      </CollapsibleSection>

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
});
