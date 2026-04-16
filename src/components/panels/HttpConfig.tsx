/**
 * HTTP 节点配置组件
 * 支持 URL、方法、Headers、Body 配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef, useMemo } from 'react';
import { httpConfigSchema, type HttpConfigFormData, type HeaderEntry, type HttpBodyType } from './schemas';
import { open } from '@tauri-apps/plugin-dialog';
import { Input } from '@/components/ui/input';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Plus, Trash2, Globe, X, ArrowRightFromLine, Info } from 'lucide-react';
import type { HttpConfig as HttpConfigType } from '@/types';
import type { Variable } from '@/types/variable';
import { useFlowStore } from '@/stores';
import { isEntryConfig, isHttpConfig } from '@/types/node';
/** 检测是否在 Tauri 环境中运行 */
const isTauriEnv = () => {
  return typeof window !== 'undefined' && '__TAURI__' in window;
};
interface HttpConfigProps {
  /** 当前节点数据 */
  data: HttpConfigType;
  /** 保存回调 */
  onSave: (data: HttpConfigType) => void;
}

/**
 * 将 HttpConfigType 转换为表单数据
 */
function toFormData(data: HttpConfigType): HttpConfigFormData {
  const headers: HeaderEntry[] = data.headers
    ? Object.entries(data.headers).map(([key, value]) => ({ key, value }))
    : [];

  let body = '';
  if (data.body !== undefined && data.body !== null) {
    try {
      body = typeof data.body === 'string' 
        ? data.body 
        : JSON.stringify(data.body, null, 2);
    } catch {
      body = String(data.body);
    }
  }

  return {
    url: data.url || '',
    method: data.method || 'get',
    headers: headers.length > 0 ? headers : undefined,
    body_type: (data as HttpConfigType & { body_type?: HttpBodyType }).body_type || 'none',
    content_type: data.content_type ?? 'json',
    body: body || undefined,
    form_fields: data.form_fields || [],
    file_path: data.file_path || undefined,
    timeout_ms: data.timeout_ms ?? 30000,
    output_mappings: data.output_mappings || [],
  };
}

/**
 * 将表单数据转换为 HttpConfigType
 */
function fromFormData(formData: HttpConfigFormData): HttpConfigType {
  const headers = formData.headers?.reduce<Record<string, string>>((acc, { key, value }) => {
    if (key.trim()) {
      acc[key.trim()] = value;
    }
    return acc;
  }, {});

  // 过滤掉空的 form_fields
  const formFields = formData.form_fields?.filter(field => field.key.trim());

  let body: string | undefined = undefined;
  if (formData.body?.trim()) {
    body = formData.body.trim();
  }

  return {
    type: 'http',
    url: formData.url,
    method: formData.method,
    headers: headers && Object.keys(headers).length > 0 ? headers : undefined,
    body,
    body_type: formData.body_type,
    content_type: formData.content_type,
    form_fields: formFields && formFields.length > 0 ? formFields : undefined,
    file_path: formData.file_path,
    timeout_ms: formData.timeout_ms,
    output_mappings: formData.output_mappings && formData.output_mappings.length > 0 ? formData.output_mappings : undefined,
  };
}

const HTTP_METHODS = [
  { value: 'get', label: 'GET', color: 'text-emerald-600' },
  { value: 'post', label: 'POST', color: 'text-sky-600' },
  { value: 'put', label: 'PUT', color: 'text-amber-600' },
  { value: 'delete', label: 'DELETE', color: 'text-rose-600' },
  { value: 'patch', label: 'PATCH', color: 'text-violet-600' },
] as const;

export function HttpConfig({ data, onSave }: HttpConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    getValues,
    setValue,
    reset,
    formState: { errors },
  } = useForm<HttpConfigFormData>({
    resolver: zodResolver(httpConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'headers' as never,
  });

  const formFields = useFieldArray({
    control,
    name: 'form_fields' as never,
  });

  const outputMappings = useFieldArray({
    control,
    name: 'output_mappings' as never,
  });

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
  // 监听 body_type 变化
  const watchedBodyType = watch('body_type');

  // 监听表单值变化
  const watchedMethod = watch('method');

  // 当外部数据变化时，完整重置表单（防止节点切换时残留旧数据）
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      // 组件卸载时清除所有待执行的定时器
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

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
   * 安全的延迟执行包装器，注册定时器以便卸载时清理
   */
  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
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
        console.error('保存 HTTP 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    // 延迟保存，确保表单校验完成
    safeSetTimeout(saveData, 100);
  }, [saveData]);

  /**
   * 添加 Header
   */
  const handleAddHeader = useCallback(() => {
    append({ key: '', value: '' });
  }, [append]);

  /**
   * 添加 Form Field
   */
  const handleAddFormField = useCallback(() => {
    formFields.append({ key: '', value: '', type: 'text' });
  }, [formFields]);

  /**
   * 删除 Form Field
   */
  const handleRemoveFormField = useCallback((index: number) => {
    formFields.remove(index);
    safeSetTimeout(saveData, 100);
  }, [formFields, saveData]);

  /**
   * 删除 Header
   */
  const handleRemoveHeader = useCallback((index: number) => {
    remove(index);
    safeSetTimeout(saveData, 100);
  }, [remove, saveData]);

  /**
   * 添加输出映射
   */
  const handleAddOutputMapping = useCallback(() => {
    outputMappings.append({ source: '$.status', target: '', description: '' });
  }, [outputMappings]);

  /**
   * 删除输出映射
   */
  const handleRemoveOutputMapping = useCallback((index: number) => {
    outputMappings.remove(index);
    safeSetTimeout(saveData, 100);
  }, [outputMappings, saveData]);
  /** 隐藏的文件输入框 ref (用于非 Tauri 环境) */
  const fileInputRef = useRef<HTMLInputElement>(null);

  /**
   * 选择 Binary 文件
   */
  const handleSelectFile = useCallback(async () => {
    if (isTauriEnv()) {
      // Tauri 环境：使用原生文件对话框
      const selected = await open({
        multiple: false,
        filters: [{ name: '*', extensions: ['*'] }],
      });
      if (selected) {
        setValue('file_path', selected as string);
        // 延迟保存，确保表单值已更新
        safeSetTimeout(saveData, 100);
      }
    } else {
      // 非 Tauri 环境：触发隐藏的 input[type=file]
      fileInputRef.current?.click();
    }
  }, [setValue, saveData]);

  /**
   * 处理非 Tauri 环境的 Binary 文件选择
   */
  const handleFileInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      // 在浏览器环境中，只能获取文件名，无法获取完整路径
      setValue('file_path', file.name);
      // 延迟保存，确保表单值已更新
      safeSetTimeout(saveData, 100);
    }
    // 重置 input 以允许重复选择同一文件
    e.target.value = '';
  }, [setValue, saveData]);

  /**
   * 选择 Form Field 文件
   */
  const handleSelectFormFieldFile = useCallback(async (index: number) => {
    if (isTauriEnv()) {
      // Tauri 环境：使用原生文件对话框
      const selected = await open({
        multiple: false,
        filters: [{ name: '*', extensions: ['*'] }],
      });
      if (selected) {
        setValue(`form_fields.${index}.file_path`, selected as string);
        // 延迟保存，确保表单值已更新
        safeSetTimeout(saveData, 100);
      }
    } else {
      // 非 Tauri 环境：动态创建 input 元素
      const newInput = document.createElement('input');
      newInput.type = 'file';
      newInput.onchange = (e) => {
        const target = e.target as HTMLInputElement;
        const file = target.files?.[0];
        if (file) {
          setValue(`form_fields.${index}.file_path`, file.name);
          // 延迟保存，确保表单值已更新
          safeSetTimeout(saveData, 100);
        }
      };
      newInput.click();
    }
  }, [setValue, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 隐藏的文件输入框 (用于非 Tauri 环境) */}
      <input
        ref={fileInputRef}
        type="file"
        className="hidden"
        onChange={handleFileInputChange}
      />
      {/* URL 输入 */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-foreground">
          URL <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Globe className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <VariableInput
            id="url"
            variables={variables}
            nodeLabels={nodeLabels}
            value={watch('url') || ''}
            onValueChange={(value) => setValue('url', value)}
            placeholder="https://api.example.com/endpoint"
            className="pl-9 font-mono text-sm text-left"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
        <VariableValidationWarning
          value={watch('url') || ''}
          availableVariables={availableVariableNames}
          compact
        />
      </div>

      {/* 请求方法选择 */}
      <div className="space-y-2">
        <Label htmlFor="method" className="text-sm font-medium text-foreground">
          请求方法
        </Label>
        <Select
          value={watchedMethod}
          onValueChange={(value) => {
            setValue('method', value as HttpConfigFormData['method']);
            saveData();
          }}
        >
          <SelectTrigger id="method" className="font-mono">
            <SelectValue placeholder="选择方法" />
          </SelectTrigger>
          <SelectContent>
            {HTTP_METHODS.map((method) => (
              <SelectItem key={method.value} value={method.value}>
                <span className={`font-mono font-semibold ${method.color}`}>
                  {method.label}
                </span>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* 超时时间 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          超时时间
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)</p>
      </div>

      {/* Headers 编辑器 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">Headers</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加
          </Button>
        </div>
        <div className="space-y-2">
          {fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无自定义 Headers</p>
            </div>
          ) : (
            fields.map((field, index) => (
              <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                <div className="flex items-center gap-2">
                  <Input
                    {...register(`headers.${index}.key`)}
                    placeholder="Header Name"
                    className="h-8 flex-1 font-mono text-xs"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => handleRemoveHeader(index)}
                    className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
                <VariableInput
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={watch(`headers.${index}.value`) || ''}
                  onValueChange={(value) => setValue(`headers.${index}.value`, value)}
                  placeholder="Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <VariableValidationWarning
                  value={watch(`headers.${index}.value`) || ''}
                  availableVariables={availableVariableNames}
                  compact
                />
              </div>
            ))
          )}
        </div>
      </div>
      {/* Body 类型选择器 */}
      <div className="space-y-2">
        <Label htmlFor="body_type" className="text-sm font-medium text-foreground">
          Body 类型
        </Label>
        <Select
          value={watchedBodyType || 'none'}
          onValueChange={(value) => {
            setValue('body_type', value as HttpConfigFormData['body_type']);
            saveData();
          }}
        >
          <SelectTrigger id="body_type" className="font-mono">
            <SelectValue placeholder="选择 Body 类型" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">无 Body</SelectItem>
            <SelectItem value="body">Raw Body</SelectItem>
            <SelectItem value="form-data">Form-Data</SelectItem>
            <SelectItem value="x-www-form-urlencoded">X-WWW-Form-Urlencoded</SelectItem>
            <SelectItem value="binary">Binary File</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Raw Body 编辑器 (仅 body 类型时显示) */}
      {watchedBodyType === 'body' && (
        <div className="space-y-2">
          <Label htmlFor="body" className="text-sm font-medium text-foreground">
            Body (JSON)
          </Label>
          <Textarea
            id="body"
            {...register('body')}
            placeholder='{"key": "value"}'
            className="min-h-[120px] resize-y font-mono text-sm"
            rows={5}
          />
          <p className="text-xs text-muted-foreground break-all">
            支持使用 <code className="rounded bg-muted px-1 py-0.5 break-all">{'{{变量名}}'}</code> 引用变量
          </p>
          <VariableValidationWarning
            value={watch('body') || ''}
            availableVariables={availableVariableNames}
          />
        </div>
      )}

      {/* Content-Type 选择器 (仅 body 类型时显示) */}
      {watchedBodyType === 'body' && (
        <div className="space-y-2">
          <Label htmlFor="content_type" className="text-sm font-medium text-foreground">
            Content-Type
          </Label>
          <Select
            value={watch('content_type') || 'json'}
            onValueChange={(value) => {
              setValue('content_type', value as HttpConfigFormData['content_type']);
              saveData();
            }}
          >
            <SelectTrigger id="content_type" className="font-mono">
              <SelectValue placeholder="选择 Content-Type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="json">JSON</SelectItem>
              <SelectItem value="text">Text</SelectItem>
              <SelectItem value="html">HTML</SelectItem>
              <SelectItem value="xml">XML</SelectItem>
              <SelectItem value="javascript">JavaScript</SelectItem>
            </SelectContent>
          </Select>
        </div>
      )}

      {/* Form Fields 编辑器 (form-data 和 x-www-form-urlencoded) */}
      {['form-data', 'x-www-form-urlencoded'].includes(watchedBodyType || '') && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label className="text-sm font-medium text-foreground">Form Fields</Label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={handleAddFormField}
              className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
            >
              <Plus className="h-3 w-3" />
              添加字段
            </Button>
          </div>
          <div className="space-y-2">
            {formFields.fields.length === 0 ? (
              <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
                <p className="text-xs text-muted-foreground">暂无 Form Fields</p>
              </div>
            ) : (
              formFields.fields.map((field, index) => {
                  const fieldType = watch(`form_fields.${index}.type`) || 'text';
                  const isFileField = fieldType === 'file' && watchedBodyType === 'form-data';
                  const isFormData = watchedBodyType === 'form-data';
                  
                  return (
                    <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                      {/* 第一行：Key + 类型 + 删除 */}
                      <div className="flex items-center gap-2">
                        <Input
                          {...register(`form_fields.${index}.key`)}
                          placeholder="Key"
                          className="h-8 flex-1 font-mono text-xs"
                        />
                        {/* 类型选择器：仅 form-data 显示 */}
                        {isFormData && (
                          <Select
                            value={fieldType}
                            onValueChange={(value) => {
                              setValue(`form_fields.${index}.type`, value as 'text' | 'file');
                              if (value === 'file') {
                                setValue(`form_fields.${index}.value`, '');
                              } else {
                                setValue(`form_fields.${index}.file_path`, '');
                              }
                              saveData();
                            }}
                          >
                            <SelectTrigger className="h-8 w-20 font-mono text-xs">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="text">Text</SelectItem>
                              <SelectItem value="file">File</SelectItem>
                            </SelectContent>
                          </Select>
                        )}
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          onClick={() => handleRemoveFormField(index)}
                          className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                      
                      {/* 第二行：Value 或 文件选择 */}
                      {isFileField ? (
                        <div className="flex items-center gap-2">
                          <Input
                            {...register(`form_fields.${index}.file_path`)}
                            placeholder="点击选择文件..."
                            className="h-8 flex-1 font-mono text-xs bg-muted/50 cursor-pointer"
                            readOnly
                            onClick={() => handleSelectFormFieldFile(index)}
                          />
                          {watch(`form_fields.${index}.file_path`) && (
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              onClick={() => {
                                setValue(`form_fields.${index}.file_path`, '');
                                saveData();
                              }}
                              className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                            >
                              <X className="h-3.5 w-3.5" />
                            </Button>
                          )}
                        </div>
                      ) : (
                        <VariableInput
                          variables={variables}
                          nodeLabels={nodeLabels}
                          value={watch(`form_fields.${index}.value`) || ''}
                          onValueChange={(value) => setValue(`form_fields.${index}.value`, value)}
                          placeholder="Value"
                          className="h-8 flex-1 font-mono text-xs"
                        />
                      )}
                    </div>
                  );
                })
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            支持使用 <code className="rounded bg-muted px-1 py-0.5">{'{{变量名}}'}</code> 引用变量
          </p>
        </div>
      )}

      {/* Binary File 选择器 */}
      {watchedBodyType === 'binary' && (
        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">File Path</Label>
          <div className="flex items-center gap-2">
            <Input
              {...register('file_path')}
              placeholder="点击选择文件..."
              className="h-8 flex-1 font-mono text-xs bg-muted/50 cursor-pointer"
              readOnly
              onClick={handleSelectFile}
            />
            {watch('file_path') && (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() => {
                  setValue('file_path', '');
                  saveData();
                }}
                className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        </div>
      )}

      {/* 输出变量映射 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ArrowRightFromLine className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium text-foreground">输出变量映射</Label>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddOutputMapping}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加映射
          </Button>
        </div>

        {/* 映射列表 */}
        <div className="space-y-2">
          {outputMappings.fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无输出映射</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                点击「添加映射」配置变量提取规则
              </p>
            </div>
          ) : (
            outputMappings.fields.map((field, index) => {
              const targetError = errors.output_mappings?.[index]?.target;
              const sourceError = errors.output_mappings?.[index]?.source;
              
              // 检查变量名是否重复
              const outputMappingsList = watch('output_mappings') || [];
              const currentTarget = outputMappingsList[index]?.target || '';
              const duplicateTargets = outputMappingsList
                .map((m, i) => (m.target && i !== index && m.target === currentTarget ? i : -1))
                .filter(i => i !== -1);
              const hasDuplicate = duplicateTargets.length > 0 && currentTarget.trim() !== '';

              // JSONPath 常用预设
              const jsonPathPresets = [
                { value: '$.status', label: '状态码' },
                { value: '$.body', label: '响应体' },
                { value: '$.headers', label: '响应头' },
                { value: '$.request', label: '请求信息' },
                { value: '$.body.data', label: 'body.data' },
                { value: '$.body.data.token', label: 'body.data.token' },
                { value: '$.body.items[0]', label: 'body.items[0]' },
                { value: '$.headers.content-type', label: 'headers.content-type' },
              ];

              return (
                <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                  {/* 第一行：变量名 + 删除按钮 */}
                  <div className="flex items-center gap-2">
                    <Input
                      {...register(`output_mappings.${index}.target`)}
                      placeholder="变量名（字母、数字、下划线）"
                      className={`h-8 flex-1 font-mono text-xs ${targetError || hasDuplicate ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveOutputMapping(index)}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>

                  {/* 第二行：源字段 */}
                  <div className="relative">
                    <Input
                      {...register(`output_mappings.${index}.source`)}
                      placeholder="源字段（如 $.status、$.body.data.token）"
                      className={`h-8 w-full font-mono text-xs ${sourceError ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                      list={`jsonpath-presets-${index}`}
                    />
                    <datalist id={`jsonpath-presets-${index}`}>
                      {jsonPathPresets.map((preset) => (
                        <option key={preset.value} value={preset.value} />
                      ))}
                    </datalist>
                  </div>

                  {/* 错误提示 */}
                  {sourceError && (
                    <p className="text-xs text-rose-500">{sourceError.message}</p>
                  )}
                  {targetError && (
                    <p className="text-xs text-rose-500">{targetError.message}</p>
                  )}
                  {hasDuplicate && !targetError && (
                    <p className="text-xs text-rose-500">变量名已存在</p>
                  )}

                  {/* 第三行：说明 */}
                  <Input
                    {...register(`output_mappings.${index}.description`)}
                    placeholder="说明（可选）"
                    className="h-8 w-full text-xs"
                  />
                </div>
              );
            })
          )}
        </div>

        {/* 提示信息 - JSONPath 语法说明 */}
        {outputMappings.fields.length > 0 && (
          <div className="flex items-start gap-2 rounded-md bg-muted/30 p-2">
            <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
            <div className="text-xs text-muted-foreground space-y-1">
              <p className="font-medium text-foreground">JSONPath 语法：</p>
              <p><code className="rounded bg-muted px-1">$.status</code> 状态码</p>
              <p><code className="rounded bg-muted px-1">$.body.data.token</code> 嵌套字段</p>
              <p><code className="rounded bg-muted px-1">$.body.items[0]</code> 数组索引</p>
              <p><code className="rounded bg-muted px-1">$.headers.content-type</code> 响应头</p>
              <p className="text-muted-foreground/70 mt-1">变量名：字母、数字、下划线，1-64 字符</p>
            </div>
          </div>
        )}
      </div>

    </form>
  );
}
