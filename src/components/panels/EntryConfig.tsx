/**
 * Entry 节点配置组件
 * 支持初始变量配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect } from 'react';
import { entryConfigSchema, type EntryConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { PlayCircle, Plus, Trash2, Variable } from 'lucide-react';
import type { EntryConfig as EntryConfigType } from '@/types';

interface EntryConfigProps {
  /** 当前节点数据 */
  data: EntryConfigType;
  /** 保存回调 */
  onSave: (data: EntryConfigType) => void;
}

/**
 * 将 EntryConfigType 转换为表单数据
 */
function toFormData(data: EntryConfigType): EntryConfigFormData {
  return {
    type: 'entry',
    initial_variables: data.initial_variables || [],
  };
}

/**
 * 将表单数据转换为 EntryConfigType
 */
function fromFormData(formData: EntryConfigFormData): EntryConfigType {
  return {
    type: 'entry',
    initial_variables: formData.initial_variables && formData.initial_variables.length > 0
      ? formData.initial_variables
      : undefined,
  };
}

/**
 * 将 JSON 值转换为字符串（用于显示）
 */
function valueToString(value: unknown): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  };
}

/**
 * 将字符串解析为 JSON 值
 */
function stringToValue(str: string): unknown {
  if (!str.trim()) return '';
  try {
    return JSON.parse(str);
  } catch {
    return str;
  };
}

export function EntryConfig({ data, onSave }: EntryConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    formState: { errors },
  } = useForm<EntryConfigFormData>({
    resolver: zodResolver(entryConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'initial_variables' as never,
  });

  // 当外部数据变化时，重置表单
  useEffect(() => {
    const formData = toFormData(data);
    setValue('initial_variables', formData.initial_variables || []);
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
        console.error('保存 Entry 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    // 延迟保存，确保表单校验完成
    setTimeout(saveData, 100);
  }, [saveData]);

  /**
   * 添加变量
   */
  const handleAddVariable = useCallback(() => {
    append({ name: '', value: '', description: '' });
  }, [append]);

  /**
   * 删除变量
   */
  const handleRemoveVariable = useCallback((index: number) => {
    remove(index);
    setTimeout(saveData, 100);
  }, [remove, saveData]);

  /**
   * 处理变量值变更（转换为 JSON）
   */
  const handleValueChange = useCallback((index: number, valueString: string) => {
    const value = stringToValue(valueString);
    setValue(`initial_variables.${index}.value`, value);
  }, [setValue]);

  // 监听所有变量名，用于唯一性检查
  const variables = watch('initial_variables') || [];

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-emerald-200/50 bg-gradient-to-br from-emerald-50/80 to-teal-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-emerald-900 flex items-center gap-2">
            <PlayCircle className="h-4 w-4" />
            入口节点
          </h4>
          <p className="text-sm leading-relaxed text-emerald-700/90 whitespace-normal break-words">
            入口节点是流程的起点。每个流程必须有且仅有一个入口节点，流程执行将从这里开始，沿着连接线依次执行各个节点。
          </p>
        </div>
      </div>

      {/* 初始变量配置 */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Variable className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium text-black">初始变量</Label>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddVariable}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-black"
          >
            <Plus className="h-3 w-3" />
            添加变量
          </Button>
        </div>

        {/* 变量列表 */}
        <div className="space-y-2">
          {fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无初始变量</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                点击「添加变量」按钮创建初始变量
              </p>
            </div>
          ) : (
            fields.map((field, index) => {
              const nameError = errors.initial_variables?.[index]?.name;
              const valueString = valueToString(variables[index]?.value);
              
              // 检查变量名是否重复
              const currentName = variables[index]?.name || '';
              const duplicateNames = variables
                .map((v, i) => (v.name && i !== index && v.name === currentName ? i : -1))
                .filter(i => i !== -1);
              const hasDuplicate = duplicateNames.length > 0 && currentName.trim() !== '';
              
              return (
                <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                  {/* 第一行：变量名 + 删除按钮 */}
                  <div className="flex items-center gap-2">
                    <div className="flex-1 min-w-0">
                      <Input
                        {...register(`initial_variables.${index}.name`)}
                        placeholder="变量名（如：api_base）"
                        className={`h-8 font-mono text-xs ${nameError || hasDuplicate ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                      />
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveVariable(index)}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-violet-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                  
                  {/* 错误提示 */}
                  {nameError && (
                    <p className="text-xs text-rose-500">{nameError.message}</p>
                  )}
                  {hasDuplicate && !nameError && (
                    <p className="text-xs text-rose-500">变量名已存在</p>
                  )}
                  
                  {/* 第二行：默认值 */}
                  <div>
                    <Input
                      value={valueString}
                      onChange={(e) => handleValueChange(index, e.target.value)}
                      onBlur={handleBlur}
                      placeholder="默认值（支持 JSON：字符串、数字、对象、数组）"
                      className="h-8 font-mono text-xs"
                    />
                  </div>
                  
                  {/* 第三行：说明 */}
                  <div>
                    <Input
                      {...register(`initial_variables.${index}.description`)}
                      placeholder="说明（可选）"
                      className="h-8 text-xs"
                    />
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* 提示信息 */}
        {fields.length > 0 && (
          <div className="rounded-md bg-muted/30 p-2">
            <p className="text-xs text-muted-foreground whitespace-normal break-words">
              变量名只能包含字母、数字和下划线，长度 1-64 个字符。默认值支持 JSON 格式（字符串需加引号）。
            </p>
          </div>
        )}
      </div>

      {/* 执行流程说明 */}
      <div className="rounded-lg border border-blue-200/50 bg-gradient-to-br from-blue-50/80 to-indigo-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-blue-900 flex items-center gap-2">
            <PlayCircle className="h-4 w-4" />
            执行流程
          </h4>
          <p className="text-sm leading-relaxed text-blue-700/90 whitespace-normal break-words">
            点击工具栏中的「执行」按钮即可开始执行整个流程。
          </p>
        </div>
      </div>
    </form>
  );
}
