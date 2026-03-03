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
    condition_type: data.advanced_expression ? 'advanced' : (data.condition_type || 'simple'),
    field_path: data.field_path || '',
    operator: (data.operator as ConditionalConfigFormData['operator']) || 'equal',
    expected_value: expectedValue || undefined,
    advanced_expression: data.advanced_expression || '',
    expected_value_type: data.expected_value_type,
  };
}

/**
 * 将表单数据转换为 ConditionalConfigType
 */
function fromFormData(formData: ConditionalConfigFormData): ConditionalConfigType {
  const isSimple = formData.condition_type === 'simple';

  let expectedValue: unknown = undefined;
  if (isSimple && formData.expected_value?.trim()) {
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
    field_path: isSimple ? formData.field_path : undefined,
    operator: isSimple ? (formData.operator as ConditionOperator) : undefined,
    expected_value: isSimple ? expectedValue : undefined,
    advanced_expression: !isSimple ? formData.advanced_expression : undefined,
    expected_value_type: isSimple ? formData.expected_value_type : undefined,
  };
}

export function ConditionalConfig({ data, onSave }: ConditionalConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
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
          if (om.enabled !== false) {
            vars.push({
              name: om.target,
              value: `{{${om.source}}}`,
              sourceNodeUuid: node.uuid,
              description: om.description,
            });
          }
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
        <Label className="text-sm font-medium text-black">条件类型</Label>
        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            onClick={() => handleConditionTypeChange('simple')}
            className={`flex items-center justify-center gap-2 rounded-lg border-2 p-3 transition-all ${
              conditionType === 'simple'
                ? 'border-sky-500 bg-sky-50 text-sky-700'
                : 'border-black hover:bg-slate-50'
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
                : 'border-black hover:bg-slate-50'
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
            <Label htmlFor="field_path" className="text-sm font-medium text-black">
              字段路径
            </Label>
            <VariableInput
              id="field_path"
              variables={variables}
              nodeLabels={nodeLabels}
              value={watch('field_path') || ''}
              onValueChange={(value) => setValue('field_path', value)}
              placeholder="http_response.status"
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground break-all">
              使用点号分隔的路径，如 <code className="rounded bg-muted px-1 break-all">http_response.data.id</code>
            </p>
            <VariableValidationWarning
              value={watch('field_path') || ''}
              availableVariables={availableVariableNames}
              compact
            />
          </div>

          {/* 操作符 */}
          <div className="space-y-2">
            <Label htmlFor="operator" className="text-sm font-medium text-black">
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
              <Label htmlFor="expected_value_type" className="text-sm font-medium text-black">
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
            <Label htmlFor="expected_value" className="text-sm font-medium text-black">
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
          <Label htmlFor="advanced_expression" className="text-sm font-medium text-black">
            高级表达式
          </Label>
          <Textarea
            id="advanced_expression"
            {...register('advanced_expression')}
            placeholder="http_response.status >= 200 && http_response.status < 300"
            className="min-h-[120px] resize-y font-mono text-sm"
            rows={4}
          />
          <div className="rounded-md bg-muted/50 p-3 text-xs text-muted-foreground">
            <p className="mb-1 font-medium text-foreground">支持的语法：</p>
            <ul className="ml-3 list-disc space-y-0.5">
              <li>比较运算符: <code className="rounded bg-muted px-1 break-all">{'>'} {'>='} {'<'} {'<='} {'=='} {'!='}</code></li>
              <li>逻辑运算符: <code className="rounded bg-muted px-1 break-all">{'&&'} {'||'} {'!'}</code></li>
              <li>字符串函数: <code className="rounded bg-muted px-1 break-all">contains() startsWith() endsWith()</code></li>
              <li>变量引用: <code className="rounded bg-muted px-1 break-all">http_response.status</code></li>
            </ul>
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
