/**
 * 带变量选择器的多行文本输入组件 (VariableTextarea)
 * 专为支持变量插值的多行场景（如 HTTP Body、TCP/UDP 数据、WS 消息、GraphQL 查询等）设计。
 * 支持：
 * - 输入 {{ 主动唤起变量选择
 * - 键盘快捷键 (Ctrl+I / Cmd+I) 主动唤起变量选择
 * - 右上角常驻紧凑变量选择图标按钮
 * - 光标处精准智能插值，自动合并光标前可能已输入的 {{
 * - 支持所有 43+ 动态变量与流程变量
 */
import * as React from 'react';
import { Variable } from '@/types/variable';
import { cn } from '@/lib/utils';
import { Textarea } from './textarea';
import { VariableSelector } from './VariableSelector';
import { Variable as VariableIcon } from 'lucide-react';

export interface VariableTextareaProps
  extends Omit<React.TextareaHTMLAttributes<HTMLTextAreaElement>, 'value' | 'onChange'> {
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
  /** 原生 onChange 兼容（react-hook-form 等） */
  onChange?: React.ChangeEventHandler<HTMLTextAreaElement>;
  /** 是否禁用变量选择器 */
  disableVariableSelector?: boolean;
  /** 是否支持动态变量（默认 true） */
  enableDynamicVariables?: boolean;
  /** 自定义变量按钮类名 */
  variableButtonClassName?: string;
  /** 容器类名 */
  containerClassName?: string;
}

export const VariableTextarea = React.forwardRef<
  HTMLTextAreaElement,
  VariableTextareaProps
>(
  (
    {
      variables,
      nodeLabels,
      value: controlledValue,
      defaultValue = '',
      onValueChange,
      onChange,
      disableVariableSelector = false,
      enableDynamicVariables = true,
      variableButtonClassName,
      containerClassName,
      className,
      disabled,
      ...props
    },
    forwardedRef
  ) => {
    // 内部 ref 用于获取光标位置
    const internalRef = React.useRef<HTMLTextAreaElement>(null);

    // 合并 ref
    const setTextareaRef = React.useCallback(
      (el: HTMLTextAreaElement | null) => {
        (internalRef as React.MutableRefObject<HTMLTextAreaElement | null>).current = el;
        if (typeof forwardedRef === 'function') {
          forwardedRef(el);
        } else if (forwardedRef) {
          forwardedRef.current = el;
        }
      },
      [forwardedRef]
    );

    // 非受控模式状态
    const [uncontrolledValue, setUncontrolledValue] = React.useState(defaultValue);

    // 变量选择器受控开关状态
    const [selectorOpen, setSelectorOpen] = React.useState(false);

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

    // 变量选择器是否可用
    const isVariableSelectorEnabled =
      !disableVariableSelector &&
      !disabled &&
      (enableDynamicVariables ? true : variables.length > 0);

    // 处理输入变更
    const handleTextareaChange = React.useCallback(
      (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        const newVal = e.target.value;
        const cursor = e.target.selectionStart;
        updateValue(newVal);
        onChange?.(e);

        // 检测光标前刚好输入了 {{ 时主动唤起变量选择
        if (
          isVariableSelectorEnabled &&
          cursor !== null &&
          newVal.slice(0, cursor).endsWith('{{')
        ) {
          setSelectorOpen(true);
        }
      },
      [updateValue, isVariableSelectorEnabled, onChange]
    );

    // 键盘快捷键监听：Ctrl+I 或 Cmd+I 主动唤起/收起变量选择
    const handleKeyDown = React.useCallback(
      (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
        props.onKeyDown?.(e);
        if (e.defaultPrevented) return;

        if (
          isVariableSelectorEnabled &&
          (e.ctrlKey || e.metaKey) &&
          (e.key === 'i' || e.key === 'I')
        ) {
          e.preventDefault();
          setSelectorOpen((prev) => !prev);
        }
      },
      [props, isVariableSelectorEnabled]
    );

    // 处理变量选择 - 插入到光标位置
    const handleVariableSelect = React.useCallback(
      (variableName: string) => {
        const textarea = internalRef.current;
        if (!textarea) {
          const insertion = `{{${variableName}}}`;
          updateValue(value + insertion);
          setSelectorOpen(false);
          return;
        }

        let start = textarea.selectionStart ?? value.length;
        let end = textarea.selectionEnd ?? value.length;
        if (start === 0 && end === 0 && value.endsWith('{{')) {
          start = value.length;
          end = value.length;
        }
        // 如果光标前刚好是 {{，则替换掉它，避免产生 {{{{var}}}}
        const beforeCursor = value.slice(0, start);
        let replaceStart = start;
        if (beforeCursor.endsWith('{{')) {
          replaceStart = start - 2;
        }

        const insertion = `{{${variableName}}}`;
        const newValue = value.slice(0, replaceStart) + insertion + value.slice(end);
        updateValue(newValue);
        setSelectorOpen(false);

        // 延迟设置光标位置（等待 DOM 更新）
        requestAnimationFrame(() => {
          const newCursorPos = replaceStart + insertion.length;
          textarea.setSelectionRange(newCursorPos, newCursorPos);
          textarea.focus();
        });
      },
      [value, updateValue]
    );

    return (
      <div className={cn('relative flex flex-col group', containerClassName)}>
        <div className="relative">
          <Textarea
            {...props}
            ref={setTextareaRef}
            value={value}
            onChange={handleTextareaChange}
            onKeyDown={handleKeyDown}
            disabled={disabled}
            className={cn(
              'font-mono text-sm',
              isVariableSelectorEnabled && 'pr-9',
              className
            )}
          />

          {isVariableSelectorEnabled && (
            <div className="absolute right-2 top-2 z-10">
              <VariableSelector
                open={selectorOpen}
                onOpenChange={setSelectorOpen}
                variables={variables}
                nodeLabels={nodeLabels}
                onSelect={handleVariableSelect}
                triggerClassName={cn(
                  'h-6 w-6 p-0 rounded-md',
                  'border border-border/80 bg-background/80 backdrop-blur-xs',
                  'hover:bg-accent hover:text-foreground',
                  'text-muted-foreground shadow-xs',
                  'flex items-center justify-center transition-colors',
                  variableButtonClassName
                )}
                triggerChildren={
                  <VariableIcon className="h-3.5 w-3.5 text-violet-600 dark:text-violet-400" />
                }
              />
            </div>
          )}
        </div>
      </div>
    );
  }
);

VariableTextarea.displayName = 'VariableTextarea';

export default VariableTextarea;
