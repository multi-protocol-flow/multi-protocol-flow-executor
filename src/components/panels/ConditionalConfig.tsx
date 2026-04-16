/**
 * Conditional 节点配置组件
 * 支持简单条件和高级表达式两种模式
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo } from 'react';
import {
  conditionalConfigSchema,
  type ConditionalConfigFormData,
  OPERATOR_OPTIONS,
} from './schemas';
import { Label } from '@/components/ui/label';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Code2, Split } from 'lucide-react';
import type { ConditionalConfig as ConditionalConfigType, ConditionOperator } from '@/types';
import type { Variable } from '@/types/variable';
import { useFlowStore } from '@/stores';
import { isEntryConfig, isHttpConfig } from '@/types/node';

interface ConditionalConfigProps {
  /** 当前节点数据 */
  data: ConditionalConfigType;
  /** 保存回调 */
  onSave: (data: ConditionalConfigType) => void;
}

/**
 * 将 ConditionalConfigType 转换为表单数据
 */
function toFormData(data: ConditionalConfigType): ConditionalConfigFormData {
  let expectedValue = '';
  if (data.expected_value !== undefined && data.expected_value !== null) {
    try {
      expectedValue = typeof data.expected_value === 'string'
        ? data.expected_value
        : JSON.stringify(data.expected_value);
    } catch {
      expectedValue = String(data.expected_value);
    }
  }

  return {
    // 优先使用 condition_type 字段，不再根据 advanced_expression 推断
    // 这样可以支持两种模式的数据同时存在
    condition_type: data.condition_type || 'simple',
    field_path: data.field_path || '',
    operator: (data.operator as ConditionalConfigFormData['operator']) || 'equal',
    expected_value: expectedValue || undefined,
    advanced_expression: data.advanced_expression || '',
    expected_value_type: data.expected_value_type,
  };
}

/**
 * 将表单数据转换为 ConditionalConfigType
 * 注意：保留两种模式的数据，切换模式时不会丢失已输入的内容
 */
function fromFormData(formData: ConditionalConfigFormData): ConditionalConfigType {
  let expectedValue: unknown = undefined;
  if (formData.expected_value?.trim()) {
    try {
      expectedValue = JSON.parse(formData.expected_value);
    } catch {
      // 如果不是有效的 JSON，保留为字符串
      expectedValue = formData.expected_value;
    }
  }

  return {
    type: 'conditional',
    condition_type: formData.condition_type,
    // 保留简单条件数据，即使当前是高级模式
    field_path: formData.field_path || undefined,
    operator: (formData.operator as ConditionOperator) || undefined,
    expected_value: expectedValue,
    // 保留高级表达式数据，即使当前是简单模式
    advanced_expression: formData.advanced_expression || undefined,
    expected_value_type: formData.expected_value_type || undefined,
  };
}

export function ConditionalConfig({ data, onSave }: ConditionalConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    getValues,
  } = useForm<ConditionalConfigFormData>({
    resolver: zodResolver(conditionalConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const conditionType = watch('condition_type');
  const watchedOperator = watch('operator');

  // 获取 flow 并提取变量
  const flow = useFlowStore((state) => state.flow);

  // 提取变量列表 (参考 VariablePanel 的 extractVariableGroups 逻辑)
  const variables: Variable[] = useMemo(() => {
    if (!flow) return [];
    const vars: Variable[] = [];
    flow.nodes.forEach(node => {
      // Entry 节点：提取 initial_variables
      if (isEntryConfig(node.data) && node.data.initial_variables) {
        node.data.initial_variables.forEach(iv => {
          vars.push({
            name: iv.name,
            value: iv.value,
            sourceNodeUuid: node.uuid,
            description: iv.description,
          });
        });
      }
      // HTTP 节点：提取 output_mappings
      if (isHttpConfig(node.data) && node.data.output_mappings) {
        node.data.output_mappings.forEach(om => {
          vars.push({
            name: om.target,
            value: `{{${om.source}}}`,
            sourceNodeUuid: node.uuid,
            description: om.description,
          });
        });
      }
    });
    return vars;
  }, [flow]);

  // 提取可用变量名列表（用于验证）
  const availableVariableNames = useMemo(() => {
    return variables.map(v => v.name);
  }, [variables]);

  // 节点标签映射
  const nodeLabels = useMemo(() => {
    if (!flow) return {};
    const labels: Record<string, string> = {};
    flow.nodes.forEach(node => {
      labels[node.uuid] = node.name;
    });
    return labels;
  }, [flow]);

  // 当外部数据变化时，重置表单
  useEffect(() => {
    const formData = toFormData(data);
    setValue('condition_type', formData.condition_type);
    setValue('field_path', formData.field_path || '');
    setValue('operator', formData.operator);
    setValue('expected_value', formData.expected_value);
    setValue('advanced_expression', formData.advanced_expression || '');
    setValue('expected_value_type', formData.expected_value_type);
  }, [data, setValue]);

  // 组件卸载时同步保存，防止点击画布导致编辑丢失
  useEffect(() => {
    return () => {
      try {
        const formData = getValues();
        const config = fromFormData(formData);
        onSave(config);
      } catch {
        // 静默处理卸载时的保存错误
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存条件配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    setTimeout(saveData, 100);
  }, [saveData]);

  /**
   * 切换条件类型
   */
  const handleConditionTypeChange = useCallback((value: string) => {
    setValue('condition_type', value as 'simple' | 'advanced');
    saveData();
  }, [setValue, saveData]);

  return (
    <form onBlur={handleBlur} className="w-full space-y-5">
      {/* 条件类型选择 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">条件类型</Label>
        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            onClick={() => handleConditionTypeChange('simple')}
            className={`flex items-center justify-center gap-2 rounded-lg border-2 p-3 transition-all ${
              conditionType === 'simple'
                ? 'border-sky-500 bg-sky-50 text-sky-700'
                : 'border-border hover:bg-accent'
            }`}
>
            <Split className="h-4 w-4" />
            <span className="text-sm font-medium">简单条件</span>
          </button>
          <button
            type="button"
            onClick={() => handleConditionTypeChange('advanced')}
            className={`flex items-center justify-center gap-2 rounded-lg border-2 p-3 transition-all ${
              conditionType === 'advanced'
                ? 'border-violet-500 bg-violet-50 text-violet-700'
                : 'border-border hover:bg-accent'
            }`}
>
            <Code2 className="h-4 w-4" />
            <span className="text-sm font-medium">高级表达式</span>
          </button>
        </div>
      </div>

      {/* 简单条件配置 */}
      {conditionType === 'simple' && (
        <>
          {/* 字段路径 */}
          <div className="space-y-2">
            <Label htmlFor="field_path" className="text-sm font-medium text-foreground">
              字段路径
            </Label>
            <VariableInput
              id="field_path"
              variables={variables}
              nodeLabels={nodeLabels}
              value={watch('field_path') || ''}
              onValueChange={(value) => setValue('field_path', value)}
              placeholder="{{变量名}} 或 $.body.data.token"
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground break-all">
              支持两种格式：
              <br />
              • <code className="rounded bg-muted px-1">{"{{变量名}}"}</code> 从变量池获取
              <br />
              • <code className="rounded bg-muted px-1">$.json.path</code> JSONPath 从数据中提取
            </p>
            <VariableValidationWarning
              value={watch('field_path') || ''}
              availableVariables={availableVariableNames}
              compact
            />
          </div>

          {/* 操作符 */}
          <div className="space-y-2">
            <Label htmlFor="operator" className="text-sm font-medium text-foreground">
              操作符
            </Label>
            <Select
              value={watchedOperator}
              onValueChange={(value) => {
                setValue('operator', value as ConditionalConfigFormData['operator']);
                saveData();
              }}
            >
              <SelectTrigger id="operator">
                <SelectValue placeholder="选择操作符" />
              </SelectTrigger>
              <SelectContent>
                {OPERATOR_OPTIONS.map((op) => (
                  <SelectItem key={op.value} value={op.value}>
                    {op.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* 值类型选择器 - 仅当操作符为 equal 或 not_equal 时显示 */}
          {(watchedOperator === 'equal' || watchedOperator === 'not_equal') && (
            <div className="space-y-2">
              <Label htmlFor="expected_value_type" className="text-sm font-medium text-foreground">
                值类型
              </Label>
              <Select
                value={watch('expected_value_type') || 'string'}
                onValueChange={(value) => {
                  setValue('expected_value_type', value as 'number' | 'string' | 'boolean' | 'null');
                  saveData();
                }}
              >
                <SelectTrigger id="expected_value_type">
                  <SelectValue placeholder="选择类型" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="string">字符串 (String)</SelectItem>
                  <SelectItem value="number">数字 (Number)</SelectItem>
                  <SelectItem value="boolean">布尔值 (Boolean)</SelectItem>
                  <SelectItem value="null">空值 (Null)</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                选择期望值的类型，用于精确比较
              </p>
            </div>
          )}

          {/* 期望值 */}
          <div className="space-y-2">
            <Label htmlFor="expected_value" className="text-sm font-medium text-foreground">
              期望值
            </Label>
            <VariableInput
              id="expected_value"
              variables={variables}
              nodeLabels={nodeLabels}
              value={watch('expected_value') || ''}
              onValueChange={(value) => setValue('expected_value', value)}
              placeholder="200"
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground break-all">
              支持数字、字符串或 JSON 值，如 <code className="rounded bg-muted px-1">200</code> 或 <code className="rounded bg-muted px-1">"success"</code>
            </p>
            <VariableValidationWarning
              value={watch('expected_value') || ''}
              availableVariables={availableVariableNames}
              compact
            />
          </div>
        </>
      )}

      {/* 高级表达式配置 */}
      {conditionType === 'advanced' && (
        <div className="space-y-2">
          <Label htmlFor="advanced_expression" className="text-sm font-medium text-foreground">
            高级表达式
          </Label>
          <Textarea
            id="advanced_expression"
            {...register('advanced_expression')}
            placeholder="http_response.status >= 200 && http_response.status < 300"
            className="min-h-[120px] resize-y font-mono text-sm"
            rows={4}
          />
          <div className="rounded-md bg-muted/50 p-3 text-xs text-muted-foreground space-y-2">
            <div>
              <p className="mb-1 font-medium text-foreground">支持的运算符：</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li>比较运算符: <code className="rounded bg-muted px-1">{'>'} {'>='} {'<'} {'<='} {'=='} {'!='}</code></li>
                <li>逻辑运算符: <code className="rounded bg-muted px-1">{'&&'} {'||'} {'!'}</code></li>
              </ul>
            </div>
            <div>
              <p className="mb-1 font-medium text-foreground">变量引用方式：</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li>直接使用变量名，无需 <code className="rounded bg-muted px-1">{"{{}}"}</code> 或 <code className="rounded bg-muted px-1">{"$."}</code></li>
                <li>示例: <code className="rounded bg-muted px-1">status == 200 && code == 0</code></li>
              </ul>
            </div>
            <div className="border-t border-border pt-2 mt-2">
              <p className="mb-1 font-medium text-amber-600">⚠️ 注意事项：</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li><span className="font-medium">不支持 JSONPath</span>（如 <code className="rounded bg-muted px-1">$.body.data</code>），仅支持引用变量池中的变量</li>
                <li>如需使用 HTTP 响应字段，请先在 HTTP 节点配置 <span className="font-medium">输出变量映射</span></li>
                <li>字符串值需用双引号: <code className="rounded bg-muted px-1">name == "John"</code></li>
              </ul>
            </div>
          </div>
          <VariableValidationWarning
            value={watch('advanced_expression') || ''}
            availableVariables={availableVariableNames}
          />
        </div>
      )}
    </form>
  );
}
