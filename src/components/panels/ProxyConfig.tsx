/**
 * 代理节点配置组件
 * 支持代理模式选择（清除/HTTP/SOCKS5）及代理地址、认证配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import { proxyConfigSchema, type ProxyConfigFormData } from './schemas';
import { Label } from '@/components/ui/label';
import type { ProxyConfig as ProxyConfigType } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';

interface ProxyConfigProps {
  /** 当前节点数据 */
  data: ProxyConfigType;
  /** 保存回调 */
  onSave: (data: ProxyConfigType) => void;
}

/** 代理模式选项 */
function getProxyModeOptions() {
  const ll = getLL().panels.proxy;
  return [
    { value: 'clear', label: ll.clearProxy() },
    { value: 'http', label: ll.httpProxy() },
    { value: 'socks5', label: ll.socks5Proxy() },
  ] as const;
}

/**
 * 将 ProxyConfigType 转换为表单数据
 */
function toFormData(data: ProxyConfigType): ProxyConfigFormData {
  return {
    type: 'proxy',
    mode: data.mode || 'clear',
    proxy_url: data.proxy_url || undefined,
    username: data.username || undefined,
    password: data.password || undefined,
  };
}

/**
 * 将表单数据转换为 ProxyConfigType
 */
function fromFormData(formData: ProxyConfigFormData): ProxyConfigType {
  return {
    type: 'proxy',
    mode: formData.mode,
    proxy_url: formData.proxy_url || undefined,
    username: formData.username || undefined,
    password: formData.password || undefined,
  };
}

export function ProxyConfig({ data, onSave }: ProxyConfigProps) {
  const ll = getLL().panels.proxy;
  const commonLL = getLL().panels;
  const {
    register,
    handleSubmit,
    watch,
    getValues,
    reset,
    setValue,
    formState: { errors, isDirty },
  } = useForm<ProxyConfigFormData>({
    resolver: zodResolver(proxyConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const currentMode = watch('mode');
  const proxyModeOptions = getProxyModeOptions();

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 组件卸载时同步保存
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
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存代理配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  // 监听 Ctrl+S 刷新事件
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * 处理代理模式变更
   */
  const handleModeChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    setValue('mode', e.target.value as 'clear' | 'http' | 'socks5', { shouldDirty: true });
    // 切换到清除模式时清空代理地址
    if (e.target.value === 'clear') {
      setValue('proxy_url', undefined, { shouldDirty: true });
      setValue('username', undefined, { shouldDirty: true });
      setValue('password', undefined, { shouldDirty: true });
    }
    setTimeout(saveData, 100);
  }, [setValue, saveData]);

  const showProxyFields = currentMode === 'http' || currentMode === 'socks5';

  return (
    <form onBlur={() => { setTimeout(() => { if (isActuallyDirtyRef.current) saveData(); }, 100); }} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 代理模式选择 */}
      <div className="space-y-2">
        <Label htmlFor="proxy-mode" className="text-sm font-medium text-foreground">
          {ll.mode()}
        </Label>
        <select
          id="proxy-mode"
          value={currentMode || 'clear'}
          onChange={handleModeChange}
          className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          {proxyModeOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>

      {/* 代理地址（仅 http/socks5 模式显示） */}
      {showProxyFields && (
        <div className="space-y-2">
          <Label htmlFor="proxy-url" className="text-sm font-medium text-foreground">
            {ll.proxyUrl()}
          </Label>
          <input
            id="proxy-url"
            type="text"
            placeholder={currentMode === 'http' ? 'http://proxy:8080' : 'socks5://proxy:1080'}
            {...register('proxy_url')}
            className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
          {errors.proxy_url && (
            <p className="text-xs text-destructive">{errors.proxy_url.message}</p>
          )}
        </div>
      )}

      {/* 用户名（仅 http/socks5 模式显示） */}
      {showProxyFields && (
        <div className="space-y-2">
          <Label htmlFor="proxy-username" className="text-sm font-medium text-foreground">
            {ll.username()}
          </Label>
          <input
            id="proxy-username"
            type="text"
            placeholder={commonLL.optional()}
            {...register('username')}
            className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
        </div>
      )}

      {/* 密码（仅 http/socks5 模式显示） */}
      {showProxyFields && (
        <div className="space-y-2">
          <Label htmlFor="proxy-password" className="text-sm font-medium text-foreground">
            {ll.password()}
          </Label>
          <input
            id="proxy-password"
            type="password"
            placeholder={commonLL.optional()}
            {...register('password')}
            className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
        </div>
      )}
    </form>
  );
}
