/**
 * HTTP 节点配置组件
 * 支持 URL、方法、Headers、Body 配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect } from 'react';
import { httpConfigSchema, type HttpConfigFormData, type HeaderEntry } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Plus, Trash2, Globe } from 'lucide-react';
import type { HttpConfig as HttpConfigType } from '@/types';

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
    body: body || undefined,
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

  let body: unknown = undefined;
  if (formData.body?.trim()) {
    try {
      body = JSON.parse(formData.body);
    } catch {
      // 如果不是有效的 JSON，保留为字符串
      body = formData.body;
    }
  }

  return {
    type: 'http',
    url: formData.url,
    method: formData.method,
    headers: headers && Object.keys(headers).length > 0 ? headers : undefined,
    body,
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
    setValue,
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

  // 监听表单值变化
  const watchedMethod = watch('method');

  // 当外部数据变化时，重置表单
  useEffect(() => {
    const formData = toFormData(data);
    setValue('url', formData.url);
    setValue('method', formData.method);
    setValue('body', formData.body);
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
        console.error('保存 HTTP 配置失败:', error);
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
   * 添加 Header
   */
  const handleAddHeader = useCallback(() => {
    append({ key: '', value: '' });
  }, [append]);

  /**
   * 删除 Header
   */
  const handleRemoveHeader = useCallback((index: number) => {
    remove(index);
    // 删除后立即保存
    setTimeout(saveData, 100);
  }, [remove, saveData]);

  return (
    <form onBlur={handleBlur} className="w-full space-y-5">
      {/* URL 输入 */}
      <div className="space-y-2">
        <Label htmlFor="url" className="text-sm font-medium text-black">
          URL <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Globe className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            id="url"
            {...register('url')}
            placeholder="https://api.example.com/endpoint"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.url && (
          <p className="text-xs text-rose-500">{errors.url.message}</p>
        )}
      </div>

      {/* 请求方法选择 */}
      <div className="space-y-2">
        <Label htmlFor="method" className="text-sm font-medium text-black">
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

      {/* Headers 编辑器 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-black">Headers</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-black"
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
              <div key={field.id} className="flex items-center gap-2">
                <Input
                  {...register(`headers.${index}.key`)}
                  placeholder="Header Name"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  {...register(`headers.${index}.value`)}
                  placeholder="Value"
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
            ))
          )}
        </div>
      </div>

      {/* Body 编辑器 */}
      <div className="space-y-2">
        <Label htmlFor="body" className="text-sm font-medium text-black">
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
      </div>
    </form>
  );
}
