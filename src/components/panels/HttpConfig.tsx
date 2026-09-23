/**
 * HTTP 节点配置组件
 * 支持 URL、方法、Headers、Body、Auth 及高级设置配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { httpConfigSchema, type HttpConfigFormData, type HeaderEntry, type HttpBodyType } from './schemas';
import { Input } from '@/components/ui/input';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableTextarea } from '@/components/ui/VariableTextarea';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import {
  Plus,
  Trash2,
  X,
  Terminal,
  Shield,
  Copy,
  Sparkles,
  Repeat,
  Cookie,
  Compass,
} from 'lucide-react';
import type { HttpConfig as HttpConfigType } from '@/types';
import { useFlowVariables } from '@/hooks';
import { getLL } from '@/i18n/i18n-lazy';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { toast } from 'sonner';
import { parseCurlToFormData } from '@/lib/curlParser';
import { generateCurlCommand, type CurlExportOptions } from '@/lib/curlExporter';
import { isTauriEnv } from '@/lib/platform';

/** 动态导入 Tauri 文件对话框 */
async function openTauriDialog(options?: Record<string, unknown>) {
  try {
    const dialog = await import('@tauri-apps/plugin-dialog');
    return await dialog.open(options);
  } catch (error) {
    console.error('Failed to open Tauri dialog:', error);
    return null;
  }
}

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

  const rawBodyType = (data as HttpConfigType & { body_type?: HttpBodyType }).body_type;
  const inferredBodyType = (rawBodyType && rawBodyType !== 'none')
    ? rawBodyType
    : (body && body.trim() !== '' ? 'body' : (rawBodyType || 'none'));

  return {
    url: data.url || '',
    method: data.method || 'get',
    headers: headers.length > 0 ? headers : undefined,
    body_type: inferredBodyType,
    content_type: data.content_type ?? 'json',
    body: body || undefined,
    form_fields: data.form_fields || [],
    file_path: data.file_path || undefined,
    timeout_ms: data.timeout_ms ?? 30000,
    auth: data.auth ?? { auth_type: 'none' },
    follow_redirects: data.follow_redirects ?? true,
    max_redirects: data.max_redirects ?? 10,
    tls_skip_verify: data.tls_skip_verify ?? false,
    tls_ca_cert_path: data.tls_ca_cert_path ?? '',
    tls_client_cert_path: data.tls_client_cert_path ?? '',
    tls_client_key_path: data.tls_client_key_path ?? '',
    accept_encoding: data.accept_encoding ?? true,
    enable_cookies: data.enable_cookies ?? false,
    cookie_jar: data.cookie_jar ?? [],
    retry_count: data.retry_count ?? 0,
    retry_delay_ms: data.retry_delay_ms ?? 1000,
    retry_on_status: data.retry_on_status ?? [],
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
  if (formData.body !== undefined && formData.body !== '') {
    body = formData.body;
  }

  return {
    type: 'http:request',
    url: formData.url,
    method: formData.method,
    headers: headers && Object.keys(headers).length > 0 ? headers : undefined,
    body,
    body_type: formData.body_type,
    content_type: formData.content_type,
    form_fields: formFields && formFields.length > 0 ? formFields : undefined,
    file_path: formData.file_path || undefined,
    timeout_ms: formData.timeout_ms,
    auth: formData.auth?.auth_type !== 'none' ? formData.auth : undefined,
    follow_redirects: formData.follow_redirects === false ? false : undefined,
    max_redirects: formData.max_redirects !== 10 ? formData.max_redirects : undefined,
    tls_skip_verify: formData.tls_skip_verify || undefined,
    tls_ca_cert_path: formData.tls_ca_cert_path || undefined,
    tls_client_cert_path: formData.tls_client_cert_path || undefined,
    tls_client_key_path: formData.tls_client_key_path || undefined,
    accept_encoding: formData.accept_encoding === false ? false : undefined,
    enable_cookies: formData.enable_cookies || undefined,
    cookie_jar: formData.enable_cookies && formData.cookie_jar?.length ? formData.cookie_jar : undefined,
    retry_count: formData.retry_count || undefined,
    retry_delay_ms: formData.retry_delay_ms !== 1000 ? formData.retry_delay_ms : undefined,
    retry_on_status: formData.retry_on_status?.length ? formData.retry_on_status : undefined,
  };
}

const HTTP_METHODS = [
  { value: 'get', label: 'GET', color: 'text-emerald-600 dark:text-emerald-400 font-bold' },
  { value: 'post', label: 'POST', color: 'text-sky-600 dark:text-sky-400 font-bold' },
  { value: 'put', label: 'PUT', color: 'text-amber-600 dark:text-amber-400 font-bold' },
  { value: 'delete', label: 'DELETE', color: 'text-rose-600 dark:text-rose-400 font-bold' },
  { value: 'patch', label: 'PATCH', color: 'text-violet-600 dark:text-violet-400 font-bold' },
  { value: 'head', label: 'HEAD', color: 'text-cyan-600 dark:text-cyan-400 font-bold' },
  { value: 'options', label: 'OPTIONS', color: 'text-slate-600 dark:text-slate-400 font-bold' },
] as const;

/** 常用 Header 快速添加 */
const COMMON_HEADER_SUGGESTIONS = [
  { key: 'Content-Type', value: 'application/json' },
  { key: 'Authorization', value: 'Bearer {{token}}' },
  { key: 'Accept', value: 'application/json' },
  { key: 'User-Agent', value: 'MPE/1.0' },
];

/** 根据 Content-Type 返回对应的 Body 标签 */
function getBodyTypeLabel(contentType: string): string {
  const labels: Record<string, () => string> = {
    json: () => getLL().panels.http.bodyTypeJson(),
    text: () => getLL().panels.http.bodyTypeText(),
    html: () => getLL().panels.http.bodyTypeHtml(),
    xml: () => getLL().panels.http.bodyTypeXml(),
    javascript: () => getLL().panels.http.bodyTypeJavascript(),
  };
  return (labels[contentType] ?? labels.json)();
}

/** 根据 Content-Type 返回对应的占位提示 */
function getBodyPlaceholder(contentType: string): string {
  const placeholders: Record<string, string> = {
    json: '{\n  "key": "value"\n}',
    text: 'Enter text content...',
    html: '<div>...</div>',
    xml: '<root>\n  <item>value</item>\n</root>',
    javascript: '// JavaScript code\n',
  };
  return placeholders[contentType] ?? placeholders.json;
}

export function HttpConfig({ data, onSave }: HttpConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    getValues,
    setValue,
    reset,
    formState: { errors, isDirty },
  } = useForm<HttpConfigFormData>({
    resolver: zodResolver(httpConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪表单实际修改状态（useRef 用于卸载时读取）
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'headers' as never,
  });

  const formFields = useFieldArray({
    control,
    name: 'form_fields' as never,
  });

  const cookieJar = useFieldArray({
    control,
    name: 'cookie_jar' as never,
  });

  const retryOnStatus = useFieldArray({
    control,
    name: 'retry_on_status' as never,
  });

  // 当前激活的 Tab
  const [activeTab, setActiveTab] = useState<'headers' | 'body' | 'auth' | 'settings'>('headers');

  // 获取流程变量
  const { variables, availableVariableNames, nodeLabels, hasOpenEndedSubflow } = useFlowVariables();

  // 监听核心字段变化
  const watchedMethod = watch('method');
  const watchedBodyType = watch('body_type');
  const watchedContentType = watch('content_type') || 'json';
  const watchedAuthType = watch('auth.auth_type');
  const watchedFollowRedirects = watch('follow_redirects');
  const watchedEnableCookies = watch('enable_cookies');
  const watchedRetryCount = watch('retry_count');
  const watchedTimeout = watch('timeout_ms');
  const watchedTlsSkipVerify = watch('tls_skip_verify');

  // 是否有定制的高级配置项（用于高级设置 Tab 标记小圆点）
  const hasCustomSettings = useMemo(() => {
    return Boolean(
      (watchedTimeout && watchedTimeout !== 30000) ||
      watchedFollowRedirects === false ||
      watchedTlsSkipVerify ||
      watchedEnableCookies ||
      (watchedRetryCount && watchedRetryCount > 0)
    );
  }, [watchedTimeout, watchedFollowRedirects, watchedTlsSkipVerify, watchedEnableCookies, watchedRetryCount]);

  // 当外部数据变化时，完整重置表单（防止节点切换时残留旧数据）
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<number>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => window.clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 组件卸载时同步保存（仅当用户实际修改过表单时）
  useEffect(() => {
    return () => {
      if (!isActuallyDirtyRef.current) return;
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

  /** 安全的延迟执行包装器，注册定时器以便卸载时清理 */
  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = window.setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  /** 保存表单数据 */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('Failed to save HTTP config:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /** 处理失焦事件 */
  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [saveData, safeSetTimeout]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /** 添加 Header */
  const handleAddHeader = useCallback((initialKey = '', initialValue = '') => {
    append({ key: initialKey, value: initialValue });
  }, [append]);

  /** 删除 Header */
  const handleRemoveHeader = useCallback((index: number) => {
    remove(index);
    safeSetTimeout(saveData, 100);
  }, [remove, safeSetTimeout, saveData]);

  /** 添加 Form Field */
  const handleAddFormField = useCallback(() => {
    formFields.append({ key: '', value: '', field_type: 'text' });
  }, [formFields]);

  /** 删除 Form Field */
  const handleRemoveFormField = useCallback((index: number) => {
    formFields.remove(index);
    safeSetTimeout(saveData, 100);
  }, [formFields, safeSetTimeout, saveData]);

  /** 添加 Cookie */
  const handleAddCookie = useCallback(() => {
    cookieJar.append({ name: '', value: '', domain: '', path: '' });
  }, [cookieJar]);

  /** 删除 Cookie */
  const handleRemoveCookie = useCallback((index: number) => {
    cookieJar.remove(index);
    safeSetTimeout(saveData, 100);
  }, [cookieJar, safeSetTimeout, saveData]);

  /** 添加重试状态码 */
  const handleAddRetryStatus = useCallback((status?: number) => {
    retryOnStatus.append(status ?? 0);
  }, [retryOnStatus]);

  /** 删除重试状态码 */
  const handleRemoveRetryStatus = useCallback((index: number) => {
    retryOnStatus.remove(index);
    safeSetTimeout(saveData, 100);
  }, [retryOnStatus, safeSetTimeout, saveData]);

  /** 隐藏的文件输入框 ref (用于非 Tauri 环境) */
  const fileInputRef = useRef<HTMLInputElement>(null);
  const tlsCertInputRef = useRef<HTMLInputElement>(null);
  const tlsClientCertInputRef = useRef<HTMLInputElement>(null);
  const tlsClientKeyInputRef = useRef<HTMLInputElement>(null);

  /** cURL 导入弹窗状态 */
  const [isCurlDialogOpen, setIsCurlDialogOpen] = useState(false);
  const [curlText, setCurlText] = useState('');
  const [curlError, setCurlError] = useState<string | null>(null);

  /** 处理 cURL 导入 */
  const handleCurlImport = useCallback(() => {
    if (!curlText.trim()) {
      setCurlError(getLL().panels.http.curlErrorRequired());
      return;
    }

    const formData = parseCurlToFormData(curlText);
    if (!formData) {
      setCurlError(getLL().panels.http.curlErrorParseFailed());
      return;
    }

    reset(formData);
    toast.success(getLL().panels.http.curlImportSuccess());
    setCurlText('');
    setCurlError(null);
    setIsCurlDialogOpen(false);
    safeSetTimeout(saveData, 100);
  }, [curlText, reset, safeSetTimeout, saveData]);

  /** 复制为 cURL 命令 */
  const handleCopyCurl = useCallback(() => {
    const formValues = getValues();
    const options: CurlExportOptions = {
      url: formValues.url,
      method: formValues.method,
      headers: formValues.headers?.filter(h => h.key.trim()) || [],
      bodyType: formValues.body_type || 'none',
      contentType: formValues.content_type,
    };

    switch (formValues.body_type) {
      case 'body':
        options.bodyContent = formValues.body;
        break;
      case 'form-data':
      case 'x-www-form-urlencoded':
        options.formData = (formValues.form_fields || [])
          .filter(f => f.key.trim())
          .map(f => ({
            key: f.key,
            value: f.field_type === 'file' ? (f.file_path || '') : f.value,
            type: (f.field_type || 'text') as 'text' | 'file',
          }));
        break;
      case 'binary':
        options.bodyContent = formValues.file_path;
        break;
    }

    if (formValues.auth && formValues.auth.auth_type !== 'none') {
      const auth = formValues.auth;
      switch (auth.auth_type) {
        case 'basic': {
          const encoded = btoa(`${auth.username || ''}:${auth.password || ''}`);
          (options.headers as { key: string; value: string }[]).push({
            key: 'Authorization',
            value: `Basic ${encoded}`,
          });
          break;
        }
        case 'bearer':
          (options.headers as { key: string; value: string }[]).push({
            key: 'Authorization',
            value: `Bearer ${auth.token || ''}`,
          });
          break;
        case 'api_key_header':
          (options.headers as { key: string; value: string }[]).push({
            key: auth.api_key_name || 'X-API-Key',
            value: auth.api_key_value || '',
          });
          break;
        case 'api_key_query': {
          const separator = options.url.includes('?') ? '&' : '?';
          options.url = `${options.url}${separator}${encodeURIComponent(auth.api_key_name || 'api_key')}=${encodeURIComponent(auth.api_key_value || '')}`;
          break;
        }
      }
    }

    const command = generateCurlCommand(options);
    navigator.clipboard.writeText(command).then(() => {
      toast.success(getLL().panels.http.curlCopySuccess());
    }).catch(() => {
      toast.error(getLL().panels.http.curlCopyFailed());
    });
  }, [getValues]);

  /** 格式化 JSON Body */
  const handleFormatJson = useCallback(() => {
    const rawBody = getValues('body');
    if (!rawBody?.trim()) return;
    try {
      const parsed = JSON.parse(rawBody);
      const formatted = JSON.stringify(parsed, null, 2);
      setValue('body', formatted, { shouldDirty: true });
      safeSetTimeout(saveData, 100);
      toast.success(getLL().panels.http.formatJsonSuccess());
    } catch {
      toast.error(getLL().panels.http.formatJsonError());
    }
  }, [getValues, setValue, safeSetTimeout, saveData]);

  /** 选择 Binary 文件 */
  const handleSelectFile = useCallback(async () => {
    if (isTauriEnv()) {
      const selected = await openTauriDialog({
        multiple: false,
        filters: [{ name: '*', extensions: ['*'] }],
      });
      if (selected) {
        setValue('file_path', selected as string, { shouldDirty: true });
        safeSetTimeout(saveData, 100);
      }
    } else {
      fileInputRef.current?.click();
    }
  }, [setValue, safeSetTimeout, saveData]);

  /** 处理非 Tauri 环境的 Binary 文件选择 */
  const handleFileInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setValue('file_path', file.name, { shouldDirty: true });
      safeSetTimeout(saveData, 100);
    }
    e.target.value = '';
  }, [setValue, safeSetTimeout, saveData]);

  /** 选择 TLS CA 证书文件 */
  const handleSelectTlsCaCert = useCallback(async () => {
    if (isTauriEnv()) {
      const selected = await openTauriDialog({
        multiple: false,
        filters: [{ name: 'Certificate files', extensions: ['pem', 'crt', 'cer', 'cert', 'p12', 'pfx'] }],
      });
      if (selected) {
        setValue('tls_ca_cert_path', selected as string, { shouldDirty: true });
        safeSetTimeout(saveData, 100);
      }
    } else {
      tlsCertInputRef.current?.click();
    }
  }, [setValue, safeSetTimeout, saveData]);

  /** 选择 TLS 客户端证书文件 */
  const handleSelectTlsClientCert = useCallback(async () => {
    if (isTauriEnv()) {
      const selected = await openTauriDialog({
        multiple: false,
        filters: [{ name: 'Certificate files', extensions: ['pem', 'crt', 'cer', 'cert'] }],
      });
      if (selected) {
        setValue('tls_client_cert_path', selected as string, { shouldDirty: true });
        safeSetTimeout(saveData, 100);
      }
    } else {
      tlsClientCertInputRef.current?.click();
    }
  }, [setValue, safeSetTimeout, saveData]);

  /** 选择 TLS 客户端密钥文件 */
  const handleSelectTlsClientKey = useCallback(async () => {
    if (isTauriEnv()) {
      const selected = await openTauriDialog({
        multiple: false,
        filters: [{ name: 'Key files', extensions: ['pem', 'key', 'pk8'] }],
      });
      if (selected) {
        setValue('tls_client_key_path', selected as string, { shouldDirty: true });
        safeSetTimeout(saveData, 100);
      }
    } else {
      tlsClientKeyInputRef.current?.click();
    }
  }, [setValue, safeSetTimeout, saveData]);

  /** 处理非 Tauri 环境的 TLS 证书文件选择 */
  const handleTlsCertInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>, field: 'tls_ca_cert_path' | 'tls_client_cert_path' | 'tls_client_key_path') => {
    const file = e.target.files?.[0];
    if (file) {
      setValue(field, file.name, { shouldDirty: true });
      safeSetTimeout(saveData, 100);
    }
    e.target.value = '';
  }, [setValue, safeSetTimeout, saveData]);

  /** 选择 Form Field 文件 */
  const handleSelectFormFieldFile = useCallback(async (index: number) => {
    if (isTauriEnv()) {
      const selected = await openTauriDialog({
        multiple: false,
        filters: [{ name: '*', extensions: ['*'] }],
      });
      if (selected) {
        setValue(`form_fields.${index}.file_path`, selected as string, { shouldDirty: true });
        safeSetTimeout(saveData, 100);
      }
    } else {
      const newInput = document.createElement('input');
      newInput.type = 'file';
      newInput.onchange = (e) => {
        const target = e.target as HTMLInputElement;
        const file = target.files?.[0];
        if (file) {
          setValue(`form_fields.${index}.file_path`, file.name, { shouldDirty: true });
          safeSetTimeout(saveData, 100);
        }
      };
      newInput.click();
    }
  }, [setValue, safeSetTimeout, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-4">
      {/* 隐藏的文件输入框 (用于非 Tauri 环境) */}
      <input
        ref={fileInputRef}
        type="file"
        className="hidden"
        onChange={handleFileInputChange}
      />
      <input
        ref={tlsCertInputRef}
        type="file"
        className="hidden"
        onChange={(e) => handleTlsCertInputChange(e, 'tls_ca_cert_path')}
        accept=".pem,.crt,.cer,.cert,.p12,.pfx"
      />
      <input
        ref={tlsClientCertInputRef}
        type="file"
        className="hidden"
        onChange={(e) => handleTlsCertInputChange(e, 'tls_client_cert_path')}
        accept=".pem,.crt,.cer,.cert"
      />
      <input
        ref={tlsClientKeyInputRef}
        type="file"
        className="hidden"
        onChange={(e) => handleTlsCertInputChange(e, 'tls_client_key_path')}
        accept=".pem,.key,.pk8"
      />

      {/* ==================================================================== */}
      {/* 1. 顶部两行式请求控制栏                                                 */}
      {/* ==================================================================== */}
      <div className="space-y-2 rounded-lg border border-border/60 bg-muted/20 p-2.5 shadow-xs">
        {/* 第一行：Method 选择器 + cURL 工具组 */}
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-1.5">
            <Label htmlFor="method" className="sr-only">
              {getLL().panels.http.requestMethod()}
            </Label>
            <Select
              value={watchedMethod}
              onValueChange={(value) => {
                setValue('method', value as HttpConfigFormData['method'], { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger
                id="method"
                className="h-7 w-[100px] font-mono text-xs font-semibold bg-background shadow-none border-border/70"
              >
                <SelectValue placeholder={getLL().panels.http.selectMethod()} />
              </SelectTrigger>
              <SelectContent>
                {HTTP_METHODS.map((method) => (
                  <SelectItem key={method.value} value={method.value}>
                    <span className={`font-mono ${method.color}`}>
                      {method.label}
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="flex items-center gap-1">
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-7 px-2 text-xs gap-1 text-muted-foreground hover:text-foreground bg-background shadow-none border-border/70"
              onClick={() => setIsCurlDialogOpen(true)}
              title={getLL().panels.http.curlImportButton()}
            >
              <Terminal className="h-3 w-3" />
              <span>cURL</span>
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-7 w-7 text-muted-foreground hover:text-foreground"
              onClick={handleCopyCurl}
              title={getLL().panels.http.curlCopyButton()}
            >
              <Copy className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

        {/* 第二行：全宽 URL 地址栏 */}
        <div className="space-y-1">
          <Label htmlFor="url" className="sr-only">
            {getLL().panels.url()}
          </Label>
          <VariableInput
            id="url"
            variables={variables}
            nodeLabels={nodeLabels}
            value={watch('url') || ''}
            onValueChange={(value) => setValue('url', value, { shouldDirty: true })}
            placeholder="https://api.example.com/endpoint"
            className="font-mono text-sm text-left flex bg-background"
          />
          {errors.url && (
            <p className="text-xs text-rose-500">{errors.url.message}</p>
          )}
          <VariableValidationWarning
            value={watch('url') || ''}
            availableVariables={availableVariableNames}
            hasOpenEndedSubflow={hasOpenEndedSubflow}
            compact
          />
        </div>
      </div>

      {/* ==================================================================== */}
      {/* 2. 状态感知 Tab 切换区                                                */}
      {/* ==================================================================== */}
      <Tabs
        value={activeTab}
        onValueChange={(val) => setActiveTab(val as typeof activeTab)}
        className="w-full space-y-3"
      >
        <TabsList className="w-full grid grid-cols-4 h-8 p-0.5 bg-muted/60 border border-border/40 select-none">
          <TabsTrigger
            value="headers"
            className="text-xs px-1 py-1 gap-0.5 truncate font-medium data-[state=active]:font-semibold cursor-pointer"
          >
            <span className="truncate">{getLL().panels.http.tabHeaders()}</span>
            {fields.length > 0 && (
              <span className="text-[10px] opacity-70 font-mono">({fields.length})</span>
            )}
          </TabsTrigger>

          <TabsTrigger
            value="body"
            className="text-xs px-1 py-1 gap-1 truncate font-medium data-[state=active]:font-semibold cursor-pointer"
          >
            <span className="truncate">{getLL().panels.http.tabBody()}</span>
            {watchedBodyType && watchedBodyType !== 'none' && (
              <span className="h-1.5 w-1.5 rounded-full bg-primary shrink-0" />
            )}
          </TabsTrigger>

          <TabsTrigger
            value="auth"
            className="text-xs px-1 py-1 gap-1 truncate font-medium data-[state=active]:font-semibold cursor-pointer"
          >
            <span className="truncate">{getLL().panels.http.tabAuth()}</span>
            {watchedAuthType && watchedAuthType !== 'none' && (
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" />
            )}
          </TabsTrigger>

          <TabsTrigger
            value="settings"
            className="text-xs px-1 py-1 gap-1 truncate font-medium data-[state=active]:font-semibold cursor-pointer"
          >
            <span className="truncate">{getLL().panels.http.tabSettings()}</span>
            {hasCustomSettings && (
              <span className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0" />
            )}
          </TabsTrigger>
        </TabsList>

        {/* ------------------------------------------------------------------ */}
        {/* Tab 1: Headers (请求头)                                           */}
        {/* ------------------------------------------------------------------ */}
        <TabsContent
          value="headers"
          forceMount
          className="data-[state=inactive]:hidden space-y-3 pt-1"
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-1.5">
              <span className="text-xs font-semibold text-foreground">
                {getLL().panels.http.headersLabel()}
              </span>
              <span className="text-[11px] text-muted-foreground font-mono">
                ({fields.length})
              </span>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => handleAddHeader()}
              className="h-6 px-2 text-xs gap-1"
            >
              <Plus className="h-3 w-3" />
              {getLL().panels.add()}
            </Button>
          </div>

          {/* 常用 Header 快速添加芯片 */}
          <div className="flex flex-wrap gap-1">
            {COMMON_HEADER_SUGGESTIONS.map((sug) => (
              <button
                key={sug.key}
                type="button"
                onClick={() => handleAddHeader(sug.key, sug.value)}
                className="inline-flex items-center gap-0.5 rounded-md border border-border/40 bg-muted/30 px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-muted hover:text-foreground transition-colors cursor-pointer"
              >
                <Plus className="h-2.5 w-2.5 opacity-60" />
                <span>{sug.key}</span>
              </button>
            ))}
          </div>

          {/* Headers 列表 (高密度微单元行) */}
          <div className="space-y-2">
            {fields.length === 0 ? (
              <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/10 px-3 py-6 text-center">
                <p className="text-xs text-muted-foreground">{getLL().panels.http.noCustomHeaders()}</p>
              </div>
            ) : (
              fields.map((field, index) => (
                <div
                  key={field.id}
                  className="rounded-md border border-border/60 bg-muted/15 p-2 space-y-1.5 transition-colors hover:border-border"
                >
                  <div className="flex items-center gap-1.5">
                    <Input
                      {...register(`headers.${index}.key`)}
                      placeholder={getLL().panels.http.headerNamePlaceholder()}
                      className="h-7 flex-1 font-mono text-xs bg-background"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveHeader(index)}
                      className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                  <VariableInput
                    variables={variables}
                    nodeLabels={nodeLabels}
                    value={watch(`headers.${index}.value`) || ''}
                    onValueChange={(value) => setValue(`headers.${index}.value`, value, { shouldDirty: true })}
                    placeholder={getLL().panels.http.value()}
                    className="h-7 flex-1 font-mono text-xs bg-background"
                  />
                  <VariableValidationWarning
                    value={watch(`headers.${index}.value`) || ''}
                    availableVariables={availableVariableNames}
                    hasOpenEndedSubflow={hasOpenEndedSubflow}
                    compact
                  />
                </div>
              ))
            )}
          </div>
        </TabsContent>

        {/* ------------------------------------------------------------------ */}
        {/* Tab 2: Body (请求体)                                              */}
        {/* ------------------------------------------------------------------ */}
        <TabsContent
          value="body"
          forceMount
          className="data-[state=inactive]:hidden space-y-3 pt-1"
        >
          {/* Body 类型选择器 */}
          <div className="flex items-center justify-between gap-2 rounded-md border border-border/50 bg-muted/20 px-2.5 py-1.5">
            <Label htmlFor="body_type" className="text-xs font-medium text-foreground shrink-0">
              {getLL().panels.http.bodyType()}
            </Label>
            <Select
              value={watchedBodyType || 'none'}
              onValueChange={(value) => {
                setValue('body_type', value as HttpConfigFormData['body_type'], { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger id="body_type" className="h-7 text-xs font-mono w-auto min-w-[130px] bg-background">
                <SelectValue placeholder={getLL().panels.http.selectBodyType()} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">{getLL().panels.http.noBody()}</SelectItem>
                <SelectItem value="body">{getLL().panels.http.rawBody()}</SelectItem>
                <SelectItem value="form-data">{getLL().panels.http.formData()}</SelectItem>
                <SelectItem value="x-www-form-urlencoded">{getLL().panels.http.xWwwFormUrlencoded()}</SelectItem>
                <SelectItem value="binary">{getLL().panels.http.binaryFile()}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 无 Body 提示 */}
          {(!watchedBodyType || watchedBodyType === 'none') && (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/10 px-3 py-8 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.http.noBodyHelp()}</p>
            </div>
          )}

          {/* Raw Body 编辑器 */}
          {watchedBodyType === 'body' && (
            <div className="space-y-2">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1.5">
                  <Label htmlFor="content_type" className="text-xs text-muted-foreground shrink-0">
                    {getLL().panels.http.contentType()}
                  </Label>
                  <Select
                    value={watch('content_type') || 'json'}
                    onValueChange={(value) => {
                      setValue('content_type', value as HttpConfigFormData['content_type'], { shouldDirty: true });
                      saveData();
                    }}
                  >
                    <SelectTrigger id="content_type" className="h-6 text-xs font-mono px-2 py-0 bg-background">
                      <SelectValue placeholder={getLL().panels.http.selectContentType()} />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="json">{getLL().panels.http.bodyTypeJson()}</SelectItem>
                      <SelectItem value="text">{getLL().panels.http.bodyTypeText()}</SelectItem>
                      <SelectItem value="html">{getLL().panels.http.bodyTypeHtml()}</SelectItem>
                      <SelectItem value="xml">{getLL().panels.http.bodyTypeXml()}</SelectItem>
                      <SelectItem value="javascript">{getLL().panels.http.bodyTypeJavascript()}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {watchedContentType === 'json' && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={handleFormatJson}
                    className="h-6 px-1.5 text-[11px] gap-1 text-muted-foreground hover:text-foreground"
                    title={getLL().panels.http.formatJsonTitle()}
                  >
                    <Sparkles className="h-3 w-3 text-amber-500" />
                    <span>{getLL().panels.http.formatJson()}</span>
                  </Button>
                )}
              </div>

              <div className="space-y-1">
                <Label htmlFor="body" className="sr-only">
                  {getBodyTypeLabel(watchedContentType)}
                </Label>
                <VariableTextarea
                  id="body"
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={watch('body') ?? ''}
                  onValueChange={(val) => {
                    setValue('body', val, { shouldDirty: true });
                  }}
                  {...register('body')}
                  placeholder={getBodyPlaceholder(watchedContentType)}
                  className="min-h-[140px] resize-y font-mono text-xs"
                  rows={6}
                />
                <p className="text-[11px] text-muted-foreground break-all">
                  {getLL().panels.http.variableRefHelp('{{var}}')}
                </p>
                <VariableValidationWarning
                  value={watch('body') || ''}
                  availableVariables={availableVariableNames}
                  hasOpenEndedSubflow={hasOpenEndedSubflow}
                />
              </div>
            </div>
          )}

          {/* Form Fields (form-data 和 x-www-form-urlencoded) */}
          {['form-data', 'x-www-form-urlencoded'].includes(watchedBodyType || '') && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-foreground">
                  {getLL().panels.http.formFieldsLabel()}
                </span>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleAddFormField}
                  className="h-6 px-2 text-xs gap-1"
                >
                  <Plus className="h-3 w-3" />
                  {getLL().panels.addField()}
                </Button>
              </div>

              <div className="space-y-2">
                {formFields.fields.length === 0 ? (
                  <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/10 px-3 py-6 text-center">
                    <p className="text-xs text-muted-foreground">{getLL().panels.http.noFormFields()}</p>
                  </div>
                ) : (
                  formFields.fields.map((field, index) => {
                    const fieldType = watch(`form_fields.${index}.field_type`) || 'text';
                    const isFileField = fieldType === 'file' && watchedBodyType === 'form-data';
                    const isFormData = watchedBodyType === 'form-data';

                    return (
                      <div
                        key={field.id}
                        className="rounded-md border border-border/60 bg-muted/15 p-2 space-y-1.5 transition-colors hover:border-border"
                      >
                        <div className="flex items-center gap-1.5">
                          <Input
                            {...register(`form_fields.${index}.key`)}
                            placeholder={getLL().panels.http.formFieldKeyPlaceholder()}
                            className="h-7 flex-1 font-mono text-xs bg-background"
                          />
                          {isFormData && (
                            <Select
                              value={fieldType}
                              onValueChange={(val) => {
                                setValue(`form_fields.${index}.field_type`, val as 'text' | 'file', { shouldDirty: true });
                                saveData();
                              }}
                            >
                              <SelectTrigger className="h-7 w-[75px] text-xs px-2 bg-background">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="text">{getLL().panels.http.formFieldTypeText()}</SelectItem>
                                <SelectItem value="file">{getLL().panels.http.formFieldTypeFile()}</SelectItem>
                              </SelectContent>
                            </Select>
                          )}
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon"
                            onClick={() => handleRemoveFormField(index)}
                            className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </div>

                        {isFileField ? (
                          <div className="flex items-center gap-1.5">
                            <Input
                              {...register(`form_fields.${index}.file_path`)}
                              placeholder={getLL().panels.http.clickToSelectFile()}
                              className="h-7 flex-1 font-mono text-xs bg-background cursor-pointer"
                              readOnly
                              onClick={() => handleSelectFormFieldFile(index)}
                            />
                            {watch(`form_fields.${index}.file_path`) && (
                              <Button
                                type="button"
                                variant="ghost"
                                size="icon"
                                onClick={() => {
                                  setValue(`form_fields.${index}.file_path`, '', { shouldDirty: true });
                                  saveData();
                                }}
                                className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                              >
                                <X className="h-3.5 w-3.5" />
                              </Button>
                            )}
                          </div>
                        ) : (
                          <>
                            <VariableInput
                              variables={variables}
                              nodeLabels={nodeLabels}
                              value={watch(`form_fields.${index}.value`) || ''}
                              onValueChange={(value) => setValue(`form_fields.${index}.value`, value, { shouldDirty: true })}
                              placeholder={getLL().panels.http.value()}
                              className="h-7 flex-1 font-mono text-xs bg-background"
                            />
                            <VariableValidationWarning
                              value={watch(`form_fields.${index}.value`) || ''}
                              availableVariables={availableVariableNames}
                              hasOpenEndedSubflow={hasOpenEndedSubflow}
                              compact
                            />
                          </>
                        )}
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          )}

          {/* Binary File 选择器 */}
          {watchedBodyType === 'binary' && (
            <div className="space-y-1.5">
              <Label className="text-xs font-medium text-foreground">
                {getLL().panels.http.filePath()}
              </Label>
              <div className="flex items-center gap-1.5">
                <Input
                  {...register('file_path')}
                  placeholder={getLL().panels.http.clickToSelectFile()}
                  className="h-8 flex-1 font-mono text-xs bg-muted/40 cursor-pointer"
                  readOnly
                  onClick={handleSelectFile}
                />
                {watch('file_path') && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => {
                      setValue('file_path', '', { shouldDirty: true });
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
        </TabsContent>

        {/* ------------------------------------------------------------------ */}
        {/* Tab 3: Auth (认证鉴权)                                             */}
        {/* ------------------------------------------------------------------ */}
        <TabsContent
          value="auth"
          forceMount
          className="data-[state=inactive]:hidden space-y-3 pt-1"
        >
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-foreground">
              {getLL().panels.http.authType()}
            </Label>
            <Select
              value={watchedAuthType || 'none'}
              onValueChange={(value) => {
                setValue('auth.auth_type', value as 'none' | 'basic' | 'bearer' | 'api_key_header' | 'api_key_query' | 'digest', { shouldDirty: true });
                safeSetTimeout(saveData, 100);
              }}
            >
              <SelectTrigger className="h-8 text-xs bg-background">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">{getLL().panels.http.noAuth()}</SelectItem>
                <SelectItem value="basic">{getLL().panels.http.basicAuth()}</SelectItem>
                <SelectItem value="bearer">{getLL().panels.http.bearerToken()}</SelectItem>
                <SelectItem value="api_key_header">{getLL().panels.http.apiKeyHeader()}</SelectItem>
                <SelectItem value="api_key_query">{getLL().panels.http.apiKeyQuery()}</SelectItem>
                <SelectItem value="digest">{getLL().panels.http.digest()}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Basic Auth */}
          {watchedAuthType === 'basic' && (
            <div className="space-y-2 rounded-md border border-border/60 bg-muted/15 p-2.5">
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.username()}</Label>
                <VariableInput
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={watch('auth.username') || ''}
                  onValueChange={(v) => { setValue('auth.username', v, { shouldDirty: true }); }}
                  placeholder="username"
                  className="h-8 text-xs bg-background"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.password()}</Label>
                <Input
                  type="password"
                  {...register('auth.password')}
                  placeholder="password"
                  className="h-8 text-xs bg-background"
                />
              </div>
            </div>
          )}

          {/* Bearer Token */}
          {watchedAuthType === 'bearer' && (
            <div className="space-y-1 rounded-md border border-border/60 bg-muted/15 p-2.5">
              <Label className="text-xs text-muted-foreground">{getLL().panels.http.token()}</Label>
              <VariableInput
                variables={variables}
                nodeLabels={nodeLabels}
                value={watch('auth.token') || ''}
                onValueChange={(v) => { setValue('auth.token', v, { shouldDirty: true }); }}
                placeholder="Bearer token"
                className="h-8 text-xs bg-background font-mono"
              />
              <p className="text-[11px] text-muted-foreground pt-1">
                {getLL().panels.http.authBearerNotice()}
              </p>
            </div>
          )}

          {/* API Key Header */}
          {watchedAuthType === 'api_key_header' && (
            <div className="space-y-2 rounded-md border border-border/60 bg-muted/15 p-2.5">
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.headerName()}</Label>
                <Input
                  {...register('auth.api_key_name')}
                  placeholder="X-API-Key"
                  className="h-8 text-xs font-mono bg-background"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.value()}</Label>
                <VariableInput
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={watch('auth.api_key_value') || ''}
                  onValueChange={(v) => { setValue('auth.api_key_value', v, { shouldDirty: true }); }}
                  placeholder="API key value"
                  className="h-8 text-xs bg-background"
                />
              </div>
            </div>
          )}

          {/* API Key Query */}
          {watchedAuthType === 'api_key_query' && (
            <div className="space-y-2 rounded-md border border-border/60 bg-muted/15 p-2.5">
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.paramName()}</Label>
                <Input
                  {...register('auth.api_key_name')}
                  placeholder="api_key"
                  className="h-8 text-xs font-mono bg-background"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.value()}</Label>
                <VariableInput
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={watch('auth.api_key_value') || ''}
                  onValueChange={(v) => { setValue('auth.api_key_value', v, { shouldDirty: true }); }}
                  placeholder="API key value"
                  className="h-8 text-xs bg-background"
                />
              </div>
            </div>
          )}

          {/* Digest */}
          {watchedAuthType === 'digest' && (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.http.digestNotSupported()}</p>
            </div>
          )}

          {/* 无认证提示 */}
          {(!watchedAuthType || watchedAuthType === 'none') && (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/10 px-3 py-6 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.http.noAuthHelp()}</p>
            </div>
          )}
        </TabsContent>

        {/* ------------------------------------------------------------------ */}
        {/* Tab 4: Settings (设置与高级参数)                                    */}
        {/* ------------------------------------------------------------------ */}
        <TabsContent
          value="settings"
          forceMount
          className="data-[state=inactive]:hidden space-y-3.5 pt-1"
        >
          {/* 1. 请求超时 */}
          <div className="rounded-md border border-border/60 bg-muted/15 p-2.5 space-y-1.5">
            <div className="flex items-center justify-between">
              <Label htmlFor="timeout_ms" className="text-xs font-medium text-foreground">
                {getLL().panels.http.timeout()}
              </Label>
              <span className="text-[11px] text-muted-foreground font-mono">
                {getLL().panels.http.milliseconds()}
              </span>
            </div>
            <Input
              id="timeout_ms"
              type="number"
              min={1000}
              max={300000}
              {...register('timeout_ms', { valueAsNumber: true })}
              className="h-8 text-xs font-mono bg-background"
            />
          </div>

          {/* 2. 重定向配置 */}
          <div className="rounded-md border border-border/60 bg-muted/15 p-2.5 space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Compass className="h-3.5 w-3.5 text-muted-foreground" />
                <Label className="text-xs font-medium text-foreground">
                  {getLL().panels.http.followRedirects()}
                </Label>
              </div>
              <Switch
                checked={watch('follow_redirects') ?? true}
                onCheckedChange={(checked) => {
                  setValue('follow_redirects', checked, { shouldDirty: true });
                  safeSetTimeout(saveData, 100);
                }}
              />
            </div>
            {watchedFollowRedirects !== false && (
              <div className="space-y-1 pt-1 border-t border-border/40">
                <Label className="text-xs text-muted-foreground">
                  {getLL().panels.http.maxRedirects()}
                </Label>
                <Input
                  type="number"
                  min={1}
                  max={20}
                  {...register('max_redirects', { valueAsNumber: true })}
                  className="h-7 text-xs bg-background font-mono w-24"
                />
              </div>
            )}
          </div>

          {/* 3. TLS 与安全配置 */}
          <div className="rounded-md border border-border/60 bg-muted/15 p-2.5 space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Shield className="h-3.5 w-3.5 text-muted-foreground" />
                <Label className="text-xs font-medium text-foreground">
                  {getLL().panels.http.skipCertVerify()}
                </Label>
              </div>
              <Switch
                checked={watch('tls_skip_verify') ?? false}
                onCheckedChange={(checked) => {
                  setValue('tls_skip_verify', checked, { shouldDirty: true });
                  safeSetTimeout(saveData, 100);
                }}
              />
            </div>

            <div className="space-y-2 pt-1 border-t border-border/40">
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.caCertPath()}</Label>
                <div className="flex items-center gap-1.5">
                  <Input
                    {...register('tls_ca_cert_path')}
                    placeholder="/path/to/ca-cert.pem"
                    className="h-7 flex-1 font-mono text-xs bg-background cursor-pointer"
                    readOnly
                    onClick={() => handleSelectTlsCaCert()}
                  />
                  {watch('tls_ca_cert_path') && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        setValue('tls_ca_cert_path', '', { shouldDirty: true });
                        saveData();
                      }}
                      className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <X className="h-3.5 w-3.5" />
                    </Button>
                  )}
                </div>
              </div>

              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.clientCertPath()}</Label>
                <div className="flex items-center gap-1.5">
                  <Input
                    {...register('tls_client_cert_path')}
                    placeholder="/path/to/client-cert.pem"
                    className="h-7 flex-1 font-mono text-xs bg-background cursor-pointer"
                    readOnly
                    onClick={() => handleSelectTlsClientCert()}
                  />
                  {watch('tls_client_cert_path') && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        setValue('tls_client_cert_path', '', { shouldDirty: true });
                        saveData();
                      }}
                      className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <X className="h-3.5 w-3.5" />
                    </Button>
                  )}
                </div>
              </div>

              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.clientKeyPath()}</Label>
                <div className="flex items-center gap-1.5">
                  <Input
                    {...register('tls_client_key_path')}
                    placeholder="/path/to/client-key.pem"
                    className="h-7 flex-1 font-mono text-xs bg-background cursor-pointer"
                    readOnly
                    onClick={() => handleSelectTlsClientKey()}
                  />
                  {watch('tls_client_key_path') && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        setValue('tls_client_key_path', '', { shouldDirty: true });
                        saveData();
                      }}
                      className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <X className="h-3.5 w-3.5" />
                    </Button>
                  )}
                </div>
              </div>
            </div>
          </div>

          {/* 4. 传输压缩 */}
          <div className="rounded-md border border-border/60 bg-muted/15 p-2.5 flex items-center justify-between">
            <div className="space-y-0.5">
              <Label className="text-xs font-medium text-foreground">
                {getLL().panels.http.autoDecompress()}
              </Label>
              <p className="text-[11px] text-muted-foreground">
                {getLL().panels.http.gzipDeflateBrotli()}
              </p>
            </div>
            <Switch
              checked={watch('accept_encoding') ?? true}
              onCheckedChange={(checked) => {
                setValue('accept_encoding', checked, { shouldDirty: true });
                safeSetTimeout(saveData, 100);
              }}
            />
          </div>

          {/* 5. Cookie 管理 */}
          <div className="rounded-md border border-border/60 bg-muted/15 p-2.5 space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Cookie className="h-3.5 w-3.5 text-muted-foreground" />
                <Label className="text-xs font-medium text-foreground">
                  {getLL().panels.http.enableCookieManagement()}
                </Label>
              </div>
              <Switch
                checked={watchedEnableCookies ?? false}
                onCheckedChange={(checked) => {
                  setValue('enable_cookies', checked, { shouldDirty: true });
                  safeSetTimeout(saveData, 100);
                }}
              />
            </div>

            {watchedEnableCookies && (
              <div className="space-y-2 pt-1 border-t border-border/40">
                <div className="flex items-center justify-between">
                  <Label className="text-xs text-muted-foreground">{getLL().panels.http.initialCookies()}</Label>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={handleAddCookie}
                    className="h-6 gap-1 text-xs text-muted-foreground hover:text-foreground"
                  >
                    <Plus className="h-3 w-3" />
                    {getLL().panels.add()}
                  </Button>
                </div>
                {cookieJar.fields.length === 0 ? (
                  <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/10 px-2 py-3 text-center">
                    <p className="text-xs text-muted-foreground">{getLL().panels.http.noCookies()}</p>
                  </div>
                ) : (
                  cookieJar.fields.map((field, index) => (
                    <div key={field.id} className="space-y-1.5 p-2 rounded-md border border-border/50 bg-background">
                      <div className="flex items-center gap-1.5">
                        <Input
                          {...register(`cookie_jar.${index}.name`)}
                          placeholder={getLL().panels.http.cookieName()}
                          className="h-6 flex-1 font-mono text-xs"
                        />
                        <Input
                          {...register(`cookie_jar.${index}.value`)}
                          placeholder={getLL().panels.http.value()}
                          className="h-6 flex-1 font-mono text-xs"
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          onClick={() => handleRemoveCookie(index)}
                          className="h-6 w-6 shrink-0 text-muted-foreground hover:text-rose-500"
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <Input
                          {...register(`cookie_jar.${index}.domain`)}
                          placeholder={getLL().panels.http.cookieDomainOptional()}
                          className="h-6 flex-1 text-xs"
                        />
                        <Input
                          {...register(`cookie_jar.${index}.path`)}
                          placeholder={getLL().panels.http.cookiePathOptional()}
                          className="h-6 flex-1 text-xs"
                        />
                      </div>
                    </div>
                  ))
                )}
              </div>
            )}
          </div>

          {/* 6. 失败重试 */}
          <div className="rounded-md border border-border/60 bg-muted/15 p-2.5 space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Repeat className="h-3.5 w-3.5 text-muted-foreground" />
                <Label className="text-xs font-medium text-foreground">
                  {getLL().panels.http.retryConfig()}
                </Label>
              </div>
              <span className="text-xs font-mono text-muted-foreground">
                {getLL().panels.http.retryCountBadge({ count: watchedRetryCount || 0 })}
              </span>
            </div>

            <div className="space-y-2 pt-1 border-t border-border/40">
              <div className="space-y-1">
                <Label className="text-xs text-muted-foreground">{getLL().panels.http.retryCount()}</Label>
                <Input
                  type="number"
                  min={0}
                  max={10}
                  {...register('retry_count', { valueAsNumber: true })}
                  className="h-7 text-xs bg-background font-mono w-24"
                />
              </div>

              {(watchedRetryCount ?? 0) > 0 && (
                <>
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.retryDelayMs()}</Label>
                    <Input
                      type="number"
                      min={0}
                      {...register('retry_delay_ms', { valueAsNumber: true })}
                      className="h-7 text-xs bg-background font-mono w-28"
                    />
                  </div>

                  <div className="space-y-1.5">
                    <div className="flex items-center justify-between">
                      <Label className="text-xs text-muted-foreground">{getLL().panels.http.retryOnStatusCodes()}</Label>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => handleAddRetryStatus()}
                        className="h-6 gap-1 text-xs text-muted-foreground hover:text-foreground"
                      >
                        <Plus className="h-3 w-3" />
                        {getLL().panels.add()}
                      </Button>
                    </div>

                    <div className="flex flex-wrap gap-1">
                      {[429, 502, 503, 504].map((status) => {
                        const existing = (watch('retry_on_status') || []) as number[];
                        const alreadyAdded = existing.includes(status);
                        return (
                          <button
                            key={status}
                            type="button"
                            disabled={alreadyAdded}
                            onClick={() => handleAddRetryStatus(status)}
                            className={`rounded-md px-1.5 py-0.5 text-[10px] font-mono transition-colors ${
                              alreadyAdded
                                ? 'bg-muted/50 text-muted-foreground/50 cursor-not-allowed'
                                : 'bg-muted text-muted-foreground hover:bg-primary/10 hover:text-primary cursor-pointer'
                            }`}
                          >
                            {status}
                          </button>
                        );
                      })}
                    </div>

                    {retryOnStatus.fields.length > 0 && (
                      <div className="space-y-1 pt-1">
                        {retryOnStatus.fields.map((field, index) => (
                          <div key={field.id} className="flex items-center gap-1.5">
                            <Input
                              type="number"
                              min={100}
                              max={599}
                              {...register(`retry_on_status.${index}` as never, { valueAsNumber: true })}
                              className="h-7 w-20 text-xs font-mono bg-background"
                            />
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              onClick={() => handleRemoveRetryStatus(index)}
                              className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                            >
                              <Trash2 className="h-3 w-3" />
                            </Button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          </div>
        </TabsContent>
      </Tabs>

      {/* ==================================================================== */}
      {/* 3. cURL 导入弹窗                                                      */}
      {/* ==================================================================== */}
      <Dialog open={isCurlDialogOpen} onOpenChange={(open) => {
        if (!open) {
          setCurlText('');
          setCurlError(null);
        }
        setIsCurlDialogOpen(open);
      }}>
        <DialogContent className="max-w-[560px]">
          <DialogHeader>
            <DialogTitle>{getLL().panels.http.curlDialogTitle()}</DialogTitle>
            <DialogDescription>
              {getLL().panels.http.curlDialogDesc()}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 py-2">
            <Textarea
              value={curlText}
              onChange={(e) => {
                setCurlText(e.target.value);
                setCurlError(null);
              }}
              placeholder={'curl -X POST https://api.example.com/users \\\n  -H \'Content-Type: application/json\' \\\n  -H \'Authorization: Bearer token\' \\\n  -d \'{"name": "test"}\''}
              className="min-h-[160px] resize-y font-mono text-xs"
              rows={7}
              autoFocus
            />
            {curlError && (
              <p className="text-xs text-rose-500">{curlError}</p>
            )}
            <p className="text-xs text-muted-foreground">
              {getLL().panels.http.curlSupportedParams()}
            </p>
          </div>
          <DialogFooter className="gap-2 sm:gap-0">
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setIsCurlDialogOpen(false);
                setCurlText('');
                setCurlError(null);
              }}
            >
              {getLL().common.CANCEL()}
            </Button>
            <Button
              type="button"
              onClick={handleCurlImport}
            >
              {getLL().panels.http.curlImportButton()}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </form>
  );
}
