/**
 * HTTP 节点配置组件
 * 支持 URL、方法、Headers、Body 配置
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import { httpConfigSchema, type HttpConfigFormData, type HeaderEntry, type HttpBodyType } from './schemas';
import { open } from '@tauri-apps/plugin-dialog';
import { Input } from '@/components/ui/input';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { Plus, Trash2, Globe, X, Terminal, ChevronDown, ChevronRight, Shield, Settings2 } from 'lucide-react';
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
    type: 'http',
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
  { value: 'get', label: 'GET', color: 'text-emerald-600' },
  { value: 'post', label: 'POST', color: 'text-sky-600' },
  { value: 'put', label: 'PUT', color: 'text-amber-600' },
  { value: 'delete', label: 'DELETE', color: 'text-rose-600' },
  { value: 'patch', label: 'PATCH', color: 'text-violet-600' },
  { value: 'head', label: 'HEAD', color: 'text-cyan-600' },
  { value: 'options', label: 'OPTIONS', color: 'text-gray-500' },
] as const;

/** 根据 Content-Type 返回对应的 Body 标签和占位提示 */
const BODY_CONTENT_TYPE_META: Record<string, { label: string; placeholder: string }> = {
  json: { label: 'Body (JSON)', placeholder: '{"key": "value"}' },
  text: { label: 'Body (Text)', placeholder: 'Enter text content...' },
  html: { label: 'Body (HTML)', placeholder: '<div>...</div>' },
  xml: { label: 'Body (XML)', placeholder: '<root>\n  <item>value</item>\n</root>' },
  javascript: { label: 'Body (JavaScript)', placeholder: '// JavaScript code\n' },
};

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

  // 高级设置各子区域的折叠状态
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [authOpen, setAuthOpen] = useState(false);
  const [redirectOpen, setRedirectOpen] = useState(false);
  const [tlsOpen, setTlsOpen] = useState(false);
  const [compressionOpen, setCompressionOpen] = useState(false);
  const [cookieOpen, setCookieOpen] = useState(false);
  const [retryOpen, setRetryOpen] = useState(false);

  // 获取流程变量
  const { variables, availableVariableNames, nodeLabels } = useFlowVariables();
  // 监听 body_type 变化
  const watchedBodyType = watch('body_type');

  // 监听 content_type 变化
  const watchedContentType = watch('content_type') || 'json';

  // 监听表单值变化
  const watchedMethod = watch('method');

  // 高级设置监听
  const watchedAuthType = watch('auth.auth_type');
  const watchedFollowRedirects = watch('follow_redirects');
  const watchedEnableCookies = watch('enable_cookies');
  const watchedRetryCount = watch('retry_count');

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
        console.error('Failed to save HTTP config:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    // 延迟保存，确保表单校验完成且 isDirty ref 已更新
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [saveData]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
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
   * 添加 Cookie
   */
  const handleAddCookie = useCallback(() => {
    cookieJar.append({ name: '', value: '', domain: '', path: '' });
  }, [cookieJar]);

  /**
   * 删除 Cookie
   */
  const handleRemoveCookie = useCallback((index: number) => {
    cookieJar.remove(index);
    safeSetTimeout(saveData, 100);
  }, [cookieJar, saveData]);

  /**
   * 添加重试状态码
   */
  const handleAddRetryStatus = useCallback((status?: number) => {
    retryOnStatus.append(status ?? 0);
  }, [retryOnStatus]);

  /**
   * 删除重试状态码
   */
  const handleRemoveRetryStatus = useCallback((index: number) => {
    retryOnStatus.remove(index);
    safeSetTimeout(saveData, 100);
  }, [retryOnStatus, saveData]);
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

    // 用解析结果填充表单
    reset(formData);

    toast.success(getLL().panels.http.curlImportSuccess());
    setCurlText('');
    setCurlError(null);
    setIsCurlDialogOpen(false);

    // 延迟保存，确保表单值已更新
    safeSetTimeout(saveData, 100);
  }, [curlText, getValues, reset, safeSetTimeout, saveData]);

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
        setValue('file_path', selected as string, { shouldDirty: true });
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
      setValue('file_path', file.name, { shouldDirty: true });
      // 延迟保存，确保表单值已更新
      safeSetTimeout(saveData, 100);
    }
    // 重置 input 以允许重复选择同一文件
    e.target.value = '';
  }, [setValue, saveData]);

  /**
   * 选择 TLS CA 证书文件
   */
  const handleSelectTlsCaCert = useCallback(async () => {
    if (isTauriEnv()) {
      const selected = await open({
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
  }, [setValue, saveData]);

  /**
   * 选择 TLS 客户端证书文件
   */
  const handleSelectTlsClientCert = useCallback(async () => {
    if (isTauriEnv()) {
      const selected = await open({
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
  }, [setValue, saveData]);

  /**
   * 选择 TLS 客户端密钥文件
   */
  const handleSelectTlsClientKey = useCallback(async () => {
    if (isTauriEnv()) {
      const selected = await open({
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
  }, [setValue, saveData]);

  /**
   * 处理非 Tauri 环境的 TLS 证书文件选择
   */
  const handleTlsCertInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>, field: 'tls_ca_cert_path' | 'tls_client_cert_path' | 'tls_client_key_path') => {
    const file = e.target.files?.[0];
    if (file) {
      setValue(field, file.name, { shouldDirty: true });
      safeSetTimeout(saveData, 100);
    }
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
        setValue(`form_fields.${index}.file_path`, selected as string, { shouldDirty: true });
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
          setValue(`form_fields.${index}.file_path`, file.name, { shouldDirty: true });
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

      {/* cURL 导入按钮 */}
      <div className="flex justify-end">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-7 gap-1.5 text-xs text-muted-foreground hover:text-foreground"
          onClick={() => setIsCurlDialogOpen(true)}
        >
          <Terminal className="h-3 w-3" />
          {getLL().panels.http.curlImportButton()}
        </Button>
      </div>

      {/* cURL 导入弹窗 */}
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
              variant="outline"
              onClick={() => {
                setIsCurlDialogOpen(false);
                setCurlText('');
                setCurlError(null);
              }}
            >
              {getLL().panels.cancel()}
            </Button>
            <Button onClick={handleCurlImport}>
              {getLL().panels.import()}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
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
            onValueChange={(value) => setValue('url', value, { shouldDirty: true })}
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
          {getLL().panels.http.requestMethod()}
        </Label>
        <Select
          value={watchedMethod}
          onValueChange={(value) => {
            setValue('method', value as HttpConfigFormData['method'], { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="method" className="font-mono">
            <SelectValue placeholder={getLL().panels.http.selectMethod()} />
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
          {getLL().panels.http.timeout()}
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">{getLL().panels.http.milliseconds()}</p>
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
             {getLL().panels.add()}
          </Button>
        </div>
        <div className="space-y-2">
          {fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.http.noCustomHeaders()}</p>
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
                  onValueChange={(value) => setValue(`headers.${index}.value`, value, { shouldDirty: true })}
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
          {getLL().panels.http.bodyType()}
        </Label>
        <Select
          value={watchedBodyType || 'none'}
          onValueChange={(value) => {
            setValue('body_type', value as HttpConfigFormData['body_type'], { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="body_type" className="font-mono">
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

      {/* Raw Body 编辑器 (仅 body 类型时显示) */}
      {watchedBodyType === 'body' && (
        <div className="space-y-2">
          <Label htmlFor="body" className="text-sm font-medium text-foreground">
            {(BODY_CONTENT_TYPE_META[watchedContentType] ?? BODY_CONTENT_TYPE_META.json).label}
          </Label>
          <Textarea
            id="body"
            {...register('body')}
            placeholder={(BODY_CONTENT_TYPE_META[watchedContentType] ?? BODY_CONTENT_TYPE_META.json).placeholder}
            className="min-h-[120px] resize-y font-mono text-sm"
            rows={5}
          />
          <p className="text-xs text-muted-foreground break-all">
            {getLL().panels.http.variableRefHelp('{{var}}')}
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
              setValue('content_type', value as HttpConfigFormData['content_type'], { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger id="content_type" className="font-mono">
              <SelectValue placeholder={getLL().panels.http.selectContentType()} />
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
               {getLL().panels.addField()}
            </Button>
          </div>
          <div className="space-y-2">
            {formFields.fields.length === 0 ? (
              <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
                <p className="text-xs text-muted-foreground">{getLL().panels.http.noFormFields()}</p>
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
                              setValue(`form_fields.${index}.type`, value as 'text' | 'file', { shouldDirty: true });
                              if (value === 'file') {
                                setValue(`form_fields.${index}.value`, '', { shouldDirty: true });
                              } else {
                                setValue(`form_fields.${index}.file_path`, '', { shouldDirty: true });
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
                            placeholder={getLL().panels.http.clickToSelectFile()}
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
                                setValue(`form_fields.${index}.file_path`, '', { shouldDirty: true });
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
                          onValueChange={(value) => setValue(`form_fields.${index}.value`, value, { shouldDirty: true })}
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
            {getLL().panels.http.variableRefHelp('{{var}}')}
          </p>
        </div>
      )}

      {/* Binary File 选择器 */}
      {watchedBodyType === 'binary' && (
        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.http.filePath()}</Label>
          <div className="flex items-center gap-2">
            <Input
              {...register('file_path')}
              placeholder={getLL().panels.http.clickToSelectFile()}
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

      {/* ==================== 高级设置 ==================== */}
      <div className="space-y-1">
        <button
          type="button"
          onClick={() => setAdvancedOpen(!advancedOpen)}
          className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-foreground hover:bg-muted/50 transition-colors"
        >
          {advancedOpen ? <ChevronDown className="h-4 w-4 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
          <Settings2 className="h-4 w-4 text-muted-foreground" />
          {getLL().panels.advancedSettings()}
        </button>

        {advancedOpen && (
          <div className="space-y-1 pl-2 border-l-2 border-muted">
            {/* 认证配置 */}
            <div className="space-y-0">
              <button
                type="button"
                onClick={() => setAuthOpen(!authOpen)}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-foreground hover:bg-muted/50 transition-colors"
              >
                {authOpen ? <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" /> : <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />}
                {getLL().panels.http.authConfig()}
                {watchedAuthType && watchedAuthType !== 'none' && (
                  <span className="ml-auto rounded-full bg-primary/10 px-2 py-0.5 text-xs text-primary">
                    {watchedAuthType}
                  </span>
                )}
              </button>

              {authOpen && (
                <div className="space-y-3 px-2 pb-3">
                  {/* 认证类型 */}
                  <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.authType()}</Label>
                    <Select
                      value={watchedAuthType || 'none'}
                      onValueChange={(value) => {
                        setValue('auth.auth_type', value as 'none' | 'basic' | 'bearer' | 'api_key_header' | 'api_key_query' | 'digest', { shouldDirty: true });
                        safeSetTimeout(saveData, 100);
                      }}
                    >
                      <SelectTrigger className="h-8 text-xs">
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
                    <div className="space-y-2">
                      <div className="space-y-1">
                        <Label className="text-xs text-muted-foreground">{getLL().panels.http.username()}</Label>
                        <VariableInput
                          variables={variables}
                          nodeLabels={nodeLabels}
                          value={watch('auth.username') || ''}
                          onValueChange={(v) => { setValue('auth.username', v, { shouldDirty: true }); }}
                          placeholder="username"
                          className="h-8 text-xs"
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs text-muted-foreground">{getLL().panels.http.password()}</Label>
                        <Input
                          type="password"
                          {...register('auth.password')}
                          placeholder="password"
                          className="h-8 text-xs"
                        />
                      </div>
                    </div>
                  )}

                  {/* Bearer Token */}
                  {watchedAuthType === 'bearer' && (
                    <div className="space-y-1">
                      <Label className="text-xs text-muted-foreground">{getLL().panels.http.token()}</Label>
                      <VariableInput
                        variables={variables}
                        nodeLabels={nodeLabels}
                        value={watch('auth.token') || ''}
                        onValueChange={(v) => { setValue('auth.token', v, { shouldDirty: true }); }}
                        placeholder="Bearer token"
                        className="h-8 text-xs"
                      />
                    </div>
                  )}

                  {/* API Key Header */}
                  {watchedAuthType === 'api_key_header' && (
                    <div className="space-y-2">
                      <div className="space-y-1">
                        <Label className="text-xs text-muted-foreground">{getLL().panels.http.headerName()}</Label>
                        <Input
                          {...register('auth.api_key_name')}
                          placeholder="X-API-Key"
                          className="h-8 text-xs font-mono"
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
                          className="h-8 text-xs"
                        />
                      </div>
                    </div>
                  )}

                  {/* API Key Query */}
                  {watchedAuthType === 'api_key_query' && (
                    <div className="space-y-2">
                      <div className="space-y-1">
                        <Label className="text-xs text-muted-foreground">{getLL().panels.http.paramName()}</Label>
                        <Input
                          {...register('auth.api_key_name')}
                          placeholder="api_key"
                          className="h-8 text-xs font-mono"
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
                          className="h-8 text-xs"
                        />
                      </div>
                    </div>
                  )}

                  {/* Digest */}
                  {watchedAuthType === 'digest' && (
                    <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-3 text-center">
                      <p className="text-xs text-muted-foreground">{getLL().panels.http.digestNotSupported()}</p>
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* 重定向配置 */}
            <div className="space-y-0">
              <button
                type="button"
                onClick={() => setRedirectOpen(!redirectOpen)}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-foreground hover:bg-muted/50 transition-colors"
              >
                {redirectOpen ? <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" /> : <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />}                {getLL().panels.http.redirectConfig()}
                {watchedFollowRedirects === false && (
                  <span className="ml-auto rounded-full bg-amber-500/10 px-2 py-0.5 text-xs text-amber-600">{getLL().panels.http.disabled()}</span>
                )}
              </button>

              {redirectOpen && (
                <div className="space-y-3 px-2 pb-3">
                  <div className="flex items-center justify-between">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.followRedirects()}</Label>
                    <Switch
                      checked={watch('follow_redirects') ?? true}
                      onCheckedChange={(checked) => {
                        setValue('follow_redirects', checked, { shouldDirty: true });
                        safeSetTimeout(saveData, 100);
                      }}
                    />
                  </div>
                  {(watchedFollowRedirects !== false) && (
                    <div className="space-y-1">
                      <Label className="text-xs text-muted-foreground">{getLL().panels.http.maxRedirects()}</Label>
                      <Input
                        type="number"
                        min={1}
                        max={20}
                        {...register('max_redirects', { valueAsNumber: true })}
                        className="h-8 text-xs"
                      />
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* TLS 配置 */}
            <div className="space-y-0">
              <button
                type="button"
                onClick={() => setTlsOpen(!tlsOpen)}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-foreground hover:bg-muted/50 transition-colors"
              >
                {tlsOpen ? <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" /> : <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />}
                <Shield className="h-3.5 w-3.5 text-muted-foreground" />                {getLL().panels.http.tlsConfig()}
                {watch('tls_skip_verify') && (
                  <span className="ml-auto rounded-full bg-rose-500/10 px-2 py-0.5 text-xs text-rose-500">{getLL().panels.http.skipVerify()}</span>
                )}
              </button>

              {tlsOpen && (
                <div className="space-y-3 px-2 pb-3">
                  <div className="flex items-center justify-between">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.skipCertVerify()}</Label>
                    <Switch
                      checked={watch('tls_skip_verify') ?? false}
                      onCheckedChange={(checked) => {
                        setValue('tls_skip_verify', checked, { shouldDirty: true });
                        safeSetTimeout(saveData, 100);
                      }}
                    />
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.caCertPath()}</Label>
                    <div className="flex items-center gap-2">
                      <Input
                        {...register('tls_ca_cert_path')}
                        placeholder="/path/to/ca-cert.pem"
                        className="h-8 flex-1 font-mono text-xs bg-muted/50 cursor-pointer"
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
                          className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                        >
                          <X className="h-3.5 w-3.5" />
                        </Button>
                      )}
                    </div>
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.clientCertPath()}</Label>
                    <div className="flex items-center gap-2">
                      <Input
                        {...register('tls_client_cert_path')}
                        placeholder="/path/to/client-cert.pem"
                        className="h-8 flex-1 font-mono text-xs bg-muted/50 cursor-pointer"
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
                          className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                        >
                          <X className="h-3.5 w-3.5" />
                        </Button>
                      )}
                    </div>
                  </div>
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.clientKeyPath()}</Label>
                    <div className="flex items-center gap-2">
                      <Input
                        {...register('tls_client_key_path')}
                        placeholder="/path/to/client-key.pem"
                        className="h-8 flex-1 font-mono text-xs bg-muted/50 cursor-pointer"
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
                          className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                        >
                          <X className="h-3.5 w-3.5" />
                        </Button>
                      )}
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* 压缩配置 */}
            <div className="space-y-0">
              <button
                type="button"
                onClick={() => setCompressionOpen(!compressionOpen)}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-foreground hover:bg-muted/50 transition-colors"
              >
                {compressionOpen ? <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" /> : <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />}                {getLL().panels.http.compressionConfig()}
              </button>

              {compressionOpen && (
                <div className="space-y-3 px-2 pb-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <Label className="text-xs text-muted-foreground">{getLL().panels.http.autoDecompress()}</Label>
                      <p className="text-xs text-muted-foreground/60 mt-0.5">gzip / deflate / brotli</p>
                    </div>
                    <Switch
                      checked={watch('accept_encoding') ?? true}
                      onCheckedChange={(checked) => {
                        setValue('accept_encoding', checked, { shouldDirty: true });
                        safeSetTimeout(saveData, 100);
                      }}
                    />
                  </div>
                </div>
              )}
            </div>

            {/* Cookie 配置 */}
            <div className="space-y-0">
              <button
                type="button"
                onClick={() => setCookieOpen(!cookieOpen)}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-foreground hover:bg-muted/50 transition-colors"
              >
                {cookieOpen ? <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" /> : <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />}
                {getLL().panels.http.cookieConfig()}
                {watchedEnableCookies && (
                  <span className="ml-auto rounded-full bg-emerald-500/10 px-2 py-0.5 text-xs text-emerald-600">
                    {getLL().panels.http.cookieCount({ count: cookieJar.fields.length })}
                  </span>
                )}
              </button>

              {cookieOpen && (
                <div className="space-y-3 px-2 pb-3">
                  <div className="flex items-center justify-between">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.enableCookieManagement()}</Label>
                    <Switch
                      checked={watchedEnableCookies ?? false}
                      onCheckedChange={(checked) => {
                        setValue('enable_cookies', checked, { shouldDirty: true });
                        safeSetTimeout(saveData, 100);
                      }}
                    />
                  </div>

                  {watchedEnableCookies && (
                    <div className="space-y-2">
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
                        <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-3 text-center">
                          <p className="text-xs text-muted-foreground">{getLL().panels.http.noCookies()}</p>
                        </div>
                      ) : (
                        cookieJar.fields.map((field, index) => (
                          <div key={field.id} className="space-y-2 p-2.5 rounded-md border bg-muted/30">
                            <div className="flex items-center gap-2">
                              <Input
                                {...register(`cookie_jar.${index}.name`)}
                                placeholder={getLL().panels.http.cookieName()}
                                className="h-7 flex-1 font-mono text-xs"
                              />
                              <Input
                                {...register(`cookie_jar.${index}.value`)}
                                placeholder={getLL().panels.http.value()}
                                className="h-7 flex-1 font-mono text-xs"
                              />
                              <Button
                                type="button"
                                variant="ghost"
                                size="icon"
                                onClick={() => handleRemoveCookie(index)}
                                className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                              >
                                <Trash2 className="h-3 w-3" />
                              </Button>
                            </div>
                            <div className="flex items-center gap-2">
                              <Input
                                {...register(`cookie_jar.${index}.domain`)}
                                placeholder={getLL().panels.http.cookieDomainOptional()}
                                className="h-7 flex-1 text-xs"
                              />
                              <Input
                                {...register(`cookie_jar.${index}.path`)}
                                placeholder={getLL().panels.http.cookiePathOptional()}
                                className="h-7 flex-1 text-xs"
                              />
                            </div>
                          </div>
                        ))
                      )}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* 重试配置 */}
            <div className="space-y-0">
              <button
                type="button"
                onClick={() => setRetryOpen(!retryOpen)}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-foreground hover:bg-muted/50 transition-colors"
              >
                {retryOpen ? <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" /> : <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />}
                {getLL().panels.http.retryConfig()}
                {watchedRetryCount && watchedRetryCount > 0 && (
                  <span className="ml-auto rounded-full bg-primary/10 px-2 py-0.5 text-xs text-primary">
                    {getLL().panels.http.retryCountBadge({ count: watchedRetryCount })}
                  </span>
                )}
              </button>

              {retryOpen && (
                <div className="space-y-3 px-2 pb-3">
                  <div className="space-y-1">
                    <Label className="text-xs text-muted-foreground">{getLL().panels.http.retryCount()}</Label>
                    <Input
                      type="number"
                      min={0}
                      max={10}
                      {...register('retry_count', { valueAsNumber: true })}
                      className="h-8 text-xs"
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
                          className="h-8 text-xs"
                        />
                      </div>

                      <div className="space-y-2">
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

                        {/* 快捷状态码标签 */}
                        <div className="flex flex-wrap gap-1.5">
                          {[429, 502, 503, 504].map((status) => {
                            const existing = (watch('retry_on_status') || []) as number[];
                            const alreadyAdded = existing.includes(status);
                            return (
                              <button
                                key={status}
                                type="button"
                                disabled={alreadyAdded}
                                onClick={() => handleAddRetryStatus(status)}
                                className={`rounded-md px-2 py-0.5 text-xs font-mono transition-colors ${
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
                          <div className="space-y-1.5">
                            {retryOnStatus.fields.map((field, index) => (
                              <div key={field.id} className="flex items-center gap-2">
                                <Input
                                  type="number"
                                  min={100}
                                  max={599}
                                  {...register(`retry_on_status.${index}` as never, { valueAsNumber: true })}
                                  className="h-7 w-24 text-xs font-mono"
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
              )}
            </div>
          </div>
        )}
      </div>

    </form>
  );
}
