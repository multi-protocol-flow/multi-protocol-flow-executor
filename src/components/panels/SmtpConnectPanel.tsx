/**
 * SMTP Connect 节点配置组件
 * 支持服务器地址、端口、TLS 模式、认证配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef } from 'react';
import {
  smtpConnectConfigSchema,
  type SmtpConnectConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Mail, Info, ChevronDown, Shield, Loader2 } from 'lucide-react';
import type { SmtpConnectConfig as SmtpConnectConfigType } from '@/types';
import { cn } from '@/lib/utils';

interface SmtpConnectConfigProps {
  /** 当前节点数据 */
  data: SmtpConnectConfigType;
  /** 保存回调 */
  onSave: (data: SmtpConnectConfigType) => void;
}

/**
 * 将 SmtpConnectConfigType 转换为表单数据
 */
function toFormData(data: SmtpConnectConfigType): SmtpConnectConfigFormData {
  return {
    type: 'smtp_connect',
    connection_id: data.connection_id || '',
    host: data.host || '',
    port: data.port ?? 25,
    tls_mode: data.tls_mode || 'starttls',
    auth_type: data.auth?.auth_type || 'none',
    username: data.auth?.username || '',
    password: data.auth?.password || '',
    oauth2_access_token: data.auth?.oauth2_tokens?.access_token || '',
    oauth2_refresh_token: data.auth?.oauth2_tokens?.refresh_token || '',
    oauth2_expires_at: data.auth?.oauth2_tokens?.expires_at || '',
    oauth2_provider: data.auth?.oauth2_tokens?.provider || '',
  };
}

/**
 * 将表单数据转换为 SmtpConnectConfigType
 */
function fromFormData(formData: SmtpConnectConfigFormData): SmtpConnectConfigType {
  const authType = formData.auth_type || 'none';
  const hasAuth = authType !== 'none';

  return {
    type: 'smtp_connect',
    connection_id: formData.connection_id || undefined,
    host: formData.host,
    port: formData.port,
    tls_mode: formData.tls_mode,
    auth: hasAuth ? {
      auth_type: authType as 'plain' | 'login' | 'xoauth2',
      username: formData.username || '',
      password: authType !== 'xoauth2' ? (formData.password || undefined) : undefined,
      oauth2_tokens: authType === 'xoauth2' && formData.oauth2_access_token ? {
        access_token: formData.oauth2_access_token,
        refresh_token: formData.oauth2_refresh_token || undefined,
        expires_at: formData.oauth2_expires_at || undefined,
        provider: (formData.oauth2_provider as 'gmail' | 'outlook' | 'yahoo' | 'custom') || undefined,
      } : undefined,
    } : undefined,
  };
}

export function SmtpConnectConfig({ data, onSave }: SmtpConnectConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<SmtpConnectConfigFormData>({
    resolver: zodResolver(smtpConnectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 追踪待执行的 setTimeout，组件卸载时清除
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

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

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 SMTP Connect 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  // 监听 Ctrl+S 刷新事件
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  // 可折叠区域的展开状态
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());

  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

  // 当前选中的 auth_type
  const currentAuthType = watch('auth_type') || 'none';
  const currentTlsMode = watch('tls_mode') || 'starttls';

  // 测试连接状态
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  /**
   * 测试连接（预留，后端 test_smtp_connection 命令尚未实现时禁用）
   */
  const handleTestConnection = useCallback(async () => {
    const formData = getValues();
    const host = formData.host?.trim();
    if (!host) return;

    setIsTesting(true);
    setTestResult(null);

    try {
      // 动态导入，避免后端命令不存在时编译报错
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<{ success: boolean; server_info?: string; error?: string }>('test_smtp_connection', {
        config: {
          host: formData.host,
          port: formData.port,
          tls_mode: formData.tls_mode,
          auth: formData.auth_type && formData.auth_type !== 'none' ? {
            auth_type: formData.auth_type,
            username: formData.username || '',
            password: formData.password || undefined,
          } : undefined,
        },
      });

      if (result.success) {
        setTestResult({ success: true, message: `连接成功${result.server_info ? ` — ${result.server_info}` : ''}` });
      } else {
        setTestResult({ success: false, message: result.error || '连接失败，未知错误' });
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: typeof error === 'string' ? error : (error instanceof Error ? error.message : String(error)),
      });
    } finally {
      setIsTesting(false);
    }
  }, [getValues]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Mail className="h-4 w-4" />
            SMTP 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            建立 SMTP 服务器连接，支持 STARTTLS 和隐式 TLS。
          </p>
        </div>
      </div>

      {/* 服务器地址 */}
      <div className="space-y-2">
        <Label htmlFor="host" className="text-sm font-medium text-foreground">
          服务器地址 <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Mail className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="host"
            {...register('host')}
            placeholder="smtp.example.com"
            className="pl-9 font-mono text-sm"
          />
        </div>
        {errors.host && (
          <p className="text-xs text-rose-500">{errors.host.message}</p>
        )}
      </div>

      {/* 端口 */}
      <div className="space-y-2">
        <Label htmlFor="port" className="text-sm font-medium text-foreground">
          端口
        </Label>
        <Input
          id="port"
          type="number"
          {...register('port', { valueAsNumber: true })}
          placeholder="25"
          className="font-mono text-sm"
        />
        {errors.port && (
          <p className="text-xs text-rose-500">{errors.port.message}</p>
        )}
      </div>

      {/* TLS 模式 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">
          TLS 模式
        </Label>
        <Select
          value={currentTlsMode}
          onValueChange={(value) => {
            setValue('tls_mode', value as 'none' | 'starttls' | 'implicit', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">无加密 (None)</SelectItem>
            <SelectItem value="starttls">STARTTLS</SelectItem>
            <SelectItem value="implicit">隐式 TLS (SMTPS)</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {currentTlsMode === 'none' && '不使用加密，仅适用于受信任网络'}
          {currentTlsMode === 'starttls' && '先明文连接，再升级为 TLS（推荐）'}
          {currentTlsMode === 'implicit' && '从一开始就使用 TLS 加密（端口 465）'}
        </p>
      </div>

      {/* 认证配置 */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('auth')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-orange-600" />
            <span className="text-sm font-medium text-foreground">认证配置</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('auth') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('auth') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            {/* 认证类型 */}
            <div className="space-y-2">
              <Label className="text-xs font-medium text-foreground">认证类型</Label>
              <Select
                value={currentAuthType}
                onValueChange={(value) => {
                  setValue('auth_type', value as 'none' | 'plain' | 'login' | 'xoauth2', { shouldDirty: true });
                  saveData();
                }}
              >
                <SelectTrigger className="h-8 text-xs font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">无需认证</SelectItem>
                  <SelectItem value="plain">PLAIN</SelectItem>
                  <SelectItem value="login">LOGIN</SelectItem>
                  <SelectItem value="xoauth2">XOAUTH2</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {currentAuthType !== 'none' && currentAuthType !== 'xoauth2' && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="username" className="text-xs font-medium text-foreground">用户名</Label>
                  <Input
                    id="username"
                    {...register('username')}
                    placeholder="user@example.com"
                    className="h-8 text-xs font-mono"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="password" className="text-xs font-medium text-foreground">密码</Label>
                  <Input
                    id="password"
                    type="password"
                    {...register('password')}
                    placeholder="••••••••"
                    className="h-8 text-xs font-mono"
                  />
                </div>
              </>
            )}

            {currentAuthType === 'xoauth2' && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="username" className="text-xs font-medium text-foreground">用户名（邮箱）</Label>
                  <Input
                    id="username"
                    {...register('username')}
                    placeholder="user@gmail.com"
                    className="h-8 text-xs font-mono"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="oauth2_access_token" className="text-xs font-medium text-foreground">Access Token</Label>
                  <Input
                    id="oauth2_access_token"
                    type="password"
                    {...register('oauth2_access_token')}
                    placeholder="OAuth2 访问令牌"
                    className="h-8 text-xs font-mono"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="oauth2_provider" className="text-xs font-medium text-foreground">邮件服务商</Label>
                  <Select
                    value={watch('oauth2_provider') || ''}
                    onValueChange={(value) => {
                      setValue('oauth2_provider', value, { shouldDirty: true });
                      saveData();
                    }}
                  >
                    <SelectTrigger className="h-8 text-xs font-mono">
                      <SelectValue placeholder="选择服务商" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="gmail">Gmail</SelectItem>
                      <SelectItem value="outlook">Outlook</SelectItem>
                      <SelectItem value="yahoo">Yahoo</SelectItem>
                      <SelectItem value="custom">自定义</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </>
            )}
          </div>
        )}
      </div>

      {/* 连接 ID */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          连接 ID（可选）
        </Label>
        <Input
          id="connection_id"
          {...register('connection_id')}
          placeholder="留空自动生成"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          留空时执行器将自动生成唯一连接 ID
        </p>
      </div>

      {/* 测试连接 */}
      <div className="space-y-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full gap-2"
          disabled={isTesting || !watch('host')?.trim()}
          onClick={handleTestConnection}
        >
          {isTesting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Mail className="h-4 w-4" />
          )}
          {isTesting ? '正在测试连接...' : '测试连接'}
        </Button>

        {testResult && (
          <div className={cn(
            "rounded-md border px-3 py-2 text-xs",
            testResult.success
              ? "border-emerald-200 dark:border-emerald-800/60 bg-emerald-50 dark:bg-emerald-950/30 text-emerald-700 dark:text-emerald-400"
              : "border-rose-200 dark:border-rose-800/60 bg-rose-50 dark:bg-rose-950/30 text-rose-600 dark:text-rose-400"
          )}>
            {testResult.message}
          </div>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">SMTP 连接说明：</p>
          <p>STARTTLS 模式通常使用端口 587，隐式 TLS 使用端口 465</p>
          <p>PLAIN 和 LOGIN 是最常见的认证方式</p>
          <p>XOAUTH2 支持 Gmail、Outlook 等服务商的 OAuth2 令牌</p>
        </div>
      </div>
    </form>
  );
}
