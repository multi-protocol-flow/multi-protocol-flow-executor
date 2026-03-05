/**
 * 带变量选择器的输入框组件
 * 在输入框右侧显示变量选择按钮，点击后弹出 VariableSelector
 * 选择变量后，将 {{变量名}} 插入到光标位置
 */
import * as React from "react";
import { Variable } from "@/types/variable";
import { cn } from "@/lib/utils";
import { Input } from "./input";
import { VariableSelector } from "./VariableSelector";
import { Variable as VariableIcon } from "lucide-react";

// ============================================================================
// 类型定义
// ============================================================================

export interface VariableInputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> {
  /** 所有可用变量 */
  variables: Variable[];
  /** 节点 ID -> 显示名称的映射 */
  nodeLabels?: Record<string, string>;
  /** 输入值（受控模式） */
  value?: string;
  /** 默认值（非受控模式） */
  defaultValue?: string;
  /** 值变更回调 */
  onValueChange?: (value: string) => void;
  /** 占位文本 */
  placeholder?: string;
  /** 是否禁用变量选择器 */
  disableVariableSelector?: boolean;
  /** 自定义变量按钮类名 */
  variableButtonClassName?: string;
  /** 输入框容器类名 */
  containerClassName?: string;
}

// ============================================================================
// VariableInput 组件
// ============================================================================

/**
 * 带变量选择器的输入框
 *
 * @example
 * ```tsx
 * // 受控模式
 * <VariableInput
 *   variables={variables}
 *   nodeLabels={{ "node-1": "Entry 节点" }}
 *   value={url}
 *   onValueChange={setUrl}
 *   placeholder="输入 URL..."
 * />
 *
 * // 非受控模式
 * <VariableInput
 *   variables={variables}
 *   defaultValue="https://api.example.com"
 *   onValueChange={(value) => console.log(value)}
 * />
 * ```
 */
export const VariableInput = React.forwardRef<
  HTMLInputElement,
  VariableInputProps
>(
  (
    {
      variables,
      nodeLabels,
      value: controlledValue,
      defaultValue = "",
      onValueChange,
      placeholder,
      disableVariableSelector = false,
      variableButtonClassName,
      containerClassName,
      className,
      disabled,
      ...props
    },
    forwardedRef
  ) => {
    // 内部 ref 用于获取光标位置
    const internalRef = React.useRef<HTMLInputElement>(null);

    // 合并 ref
    const setInputRef = React.useCallback(
      (el: HTMLInputElement | null) => {
        (internalRef as React.MutableRefObject<HTMLInputElement | null>).current =
          el;
        if (typeof forwardedRef === "function") {
          forwardedRef(el);
        } else if (forwardedRef) {
          forwardedRef.current = el;
        }
      },
      [forwardedRef]
    );

    // 非受控模式状态
    const [uncontrolledValue, setUncontrolledValue] =
      React.useState(defaultValue);

    // 当前值（受控/非受控）
    const value = controlledValue ?? uncontrolledValue;

    // 更新值
    const updateValue = React.useCallback(
      (newValue: string) => {
        if (controlledValue === undefined) {
          setUncontrolledValue(newValue);
        }
        onValueChange?.(newValue);
      },
      [controlledValue, onValueChange]
    );

    // 处理输入变更
    const handleInputChange = React.useCallback(
      (e: React.ChangeEvent<HTMLInputElement>) => {
        updateValue(e.target.value);
      },
      [updateValue]
    );

    // 处理变量选择 - 插入到光标位置
    const handleVariableSelect = React.useCallback(
      (variableName: string) => {
        const input = internalRef.current;
        if (!input) {
          // 如果没有 input 引用，直接追加到末尾
          const insertion = `{{${variableName}}}`;
          updateValue(value + insertion);
          return;
        }

        const start = input.selectionStart ?? value.length;
        const end = input.selectionEnd ?? value.length;
        const insertion = `{{${variableName}}}`;

        // 构建新值：光标前 + 插入内容 + 光标后
        const newValue = value.slice(0, start) + insertion + value.slice(end);
        updateValue(newValue);

        // 延迟设置光标位置（等待 DOM 更新）
        requestAnimationFrame(() => {
          const newCursorPos = start + insertion.length;
          input.setSelectionRange(newCursorPos, newCursorPos);
          input.focus();
        });
      },
      [value, updateValue]
    );

    // 变量选择器是否可用
    const isVariableSelectorEnabled =
      !disableVariableSelector && variables.length > 0 && !disabled;

    return (
      <div className={cn("relative flex items-center", containerClassName)}>
        <Input
          {...props}
          ref={setInputRef}
          type={props.type ?? "text"}
          value={value}
          onChange={handleInputChange}
          placeholder={placeholder}
          disabled={disabled}
          className={cn(
            // 右侧预留空间给变量按钮
            isVariableSelectorEnabled && "pr-10",
            className
          )}
        />

        {/* 变量选择按钮 */}
        {isVariableSelectorEnabled && (
          <VariableSelector
            variables={variables}
            nodeLabels={nodeLabels}
            onSelect={handleVariableSelect}
            triggerClassName={cn(
              // 绝对定位在输入框右侧内部
              "absolute right-1 top-1/2 -translate-y-1/2",
              "p-1.5 rounded-sm",
              "border-0 bg-violet-500/10",
              "hover:bg-violet-500/20 focus:bg-violet-500/20",
              "text-violet-600 hover:text-violet-700 focus:text-violet-700",
              "transition-colors duration-150",
              variableButtonClassName
            )}
            triggerChildren={<VariableIcon className="h-4 w-4" />}
          />
        )}
      </div>
    );
  }
);

VariableInput.displayName = "VariableInput";

export default VariableInput;
