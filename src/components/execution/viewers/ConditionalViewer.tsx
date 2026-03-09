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
  CheckCircle2,
  XCircle,
  AlertCircle,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';

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
        <span className="text-muted-foreground italic">未设置</span>
      </div>
    );
  }

  const displayValue = typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);

  return (
    <div className="space-y-1">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div className="bg-slate-50 rounded p-2 font-mono text-sm text-slate-700 whitespace-pre-wrap break-all">
        {displayValue}
      </div>
    </div>
  );
}

export function ConditionalViewer({ report }: ConditionalViewerProps) {
  // 从 output_data 提取 Conditional 数据
  const outputData = report.output_data as ConditionalOutputData | undefined;
  
  const success = outputData?.success;
  const expression = outputData?.expression;
  const evaluatedValue = outputData?.evaluated_value;
  const fieldValue = outputData?.field_value;
  const result = outputData?.result;
  const error = outputData?.error;

  // 判断是否评估失败
  const hasError = success === false || error;

  return (
    <div className="space-y-3">
      {/* 条件表达式 */}
      <CollapsibleSection
        title="条件表达式"
        icon={<Code2 className="w-4 h-4" />}
        defaultOpen={true}
      >
        {expression ? (
          <div className="bg-slate-50 rounded p-2 font-mono text-sm text-slate-700">
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
        icon={result === true ? <CheckCircle2 className="w-4 h-4" /> : <XCircle className="w-4 h-4" />}
        defaultOpen={true}
      >
        {hasError ? (
          // 错误状态
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <AlertCircle className="w-4 h-4 text-red-600" />
              <Badge className="font-mono text-sm px-3 py-1 text-red-600 bg-red-50">
                评估失败
              </Badge>
            </div>
            {error && (
              <div className="bg-red-50 rounded p-2 text-sm text-red-700">
                {error}
              </div>
            )}
          </div>
        ) : result !== undefined ? (
          // 正常结果
          <div className="flex items-center gap-2">
            {result === true ? (
              <>
                <CheckCircle2 className="w-5 h-5 text-green-600" />
                <Badge className="font-mono text-base px-3 py-1 text-green-600 bg-green-50">
                  true
                </Badge>
                <span className="text-sm text-muted-foreground">条件成立</span>
              </>
            ) : (
              <>
                <XCircle className="w-5 h-5 text-red-600" />
                <Badge className="font-mono text-base px-3 py-1 text-red-600 bg-red-50">
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
    </div>
  );
}
