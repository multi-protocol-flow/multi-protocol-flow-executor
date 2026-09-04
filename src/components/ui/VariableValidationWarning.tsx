/**
 * 变量引用失效警告组件
 * 检测输入值中的 {{变量名}} 引用，如果变量不存在则显示警告提示
 */
import * as React from "react";
import { cn } from "@/lib/utils";
import { AlertCircle } from "lucide-react";
import { isDynamicVariable } from "@/types/dynamicVariable";
import { getLL } from '@/i18n/i18n-lazy';

// ============================================================================
// 辅助函数
// ============================================================================

/**
 * 检测字符串中的变量引用
 * @param value 要检测的字符串
 * @returns 提取的变量名数组
 */
export function detectVariableRefs(value: string): string[] {
  if (!value || typeof value !== "string") return [];
  
  const regex = /\{\{([^}]+)\}\}/g;
  const matches: string[] = [];
  let match;
  
  while ((match = regex.exec(value)) !== null) {
    matches.push(match[1].trim());
  }
  
  return matches;
}

/**
 * 检查变量引用是否有效
 * @param refs 变量引用列表
 * @param availableVariables 可用变量名列表
 * @returns 失效的变量名数组
 */
export function findInvalidRefs(
  refs: string[],
  availableVariables: string[]
): string[] {
  return refs.filter((ref) => !availableVariables.includes(ref) && !isDynamicVariable(ref));
}

// ============================================================================
// 类型定义
// ============================================================================

export interface VariableValidationWarningProps {
  /** 要检测的输入值 */
  value: string;
  /** 可用变量名列表 */
  availableVariables: string[];
  /** 自定义类名 */
  className?: string;
  /** 是否显示图标 */
  showIcon?: boolean;
  /** 是否紧凑模式（更小的 padding） */
  compact?: boolean;
}

// ============================================================================
// VariableValidationWarning 组件
// ============================================================================

/**
 * 变量引用失效警告
 * 
 * @example
 * ```tsx
 * <VariableValidationWarning
 *   value="{{nonexistent_var}}"
 *   availableVariables={["status", "body"]}
 * />
 * ```
 */
export function VariableValidationWarning({
  value,
  availableVariables,
  className,
  showIcon = true,
  compact = false,
}: VariableValidationWarningProps) {
  const LL = getLL();

  // 检测变量引用
  const invalidRefs = React.useMemo(() => {
    const refs = detectVariableRefs(value);
    return findInvalidRefs(refs, availableVariables);
  }, [value, availableVariables]);

  // 没有失效变量，不显示警告
  if (invalidRefs.length === 0) return null;

  return (
    <div
      className={cn(
        "flex items-center gap-2 rounded-md border",
        "bg-amber-50 dark:bg-amber-950/50 border-amber-200 dark:border-amber-800 text-amber-700 dark:text-amber-300",
        compact ? "px-2 py-1" : "px-3 py-2",
        className
      )}
    >
      {showIcon && (
        <AlertCircle className={cn("shrink-0", compact ? "h-3.5 w-3.5" : "h-4 w-4")} />
      )}
      <div className="flex-1 min-w-0 flex items-center gap-1.5 flex-wrap">
        <span className="text-xs font-medium shrink-0">
          {LL.flow.var_ref_invalid()}
        </span>
        {invalidRefs.map((ref, index) => (
          <code
            key={`${ref}-${index}`}
            className={cn(
              "inline-flex items-center rounded px-1.5 py-0.5",
              "bg-amber-100 dark:bg-amber-900/50 text-amber-800 dark:text-amber-200 font-mono text-xs",
              "break-all"
            )}
          >
            {`{{${ref}}}`}
          </code>
        ))}
      </div>
    </div>
  );
}

// ============================================================================
// Hook: useVariableValidation
// ============================================================================

export interface UseVariableValidationOptions {
  /** 要检测的值 */
  value: string;
  /** 可用变量名列表 */
  availableVariables: string[];
}

export interface VariableValidationResult {
  /** 检测到的所有变量引用 */
  refs: string[];
  /** 失效的变量引用 */
  invalidRefs: string[];
  /** 是否有失效变量 */
  hasInvalid: boolean;
}

/**
 * 变量验证 Hook
 * 用于在组件中获取变量验证状态
 */
export function useVariableValidation({
  value,
  availableVariables,
}: UseVariableValidationOptions): VariableValidationResult {
  return React.useMemo(() => {
    const refs = detectVariableRefs(value);
    const invalidRefs = findInvalidRefs(refs, availableVariables);
    
    return {
      refs,
      invalidRefs,
      hasInvalid: invalidRefs.length > 0,
    };
  }, [value, availableVariables]);
}

export default VariableValidationWarning;
