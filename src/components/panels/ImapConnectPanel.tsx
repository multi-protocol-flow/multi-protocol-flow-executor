/**
 * IMAP Connect 节点配置组件
 * 支持服务器地址、端口、TLS 模式、认证配置
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Mail, Info, Shield } from 'lucide-react';
import type { ImapConnectConfig } from '@/types';

interface ImapConnectPanelProps {
  data: ImapConnectConfig;
  onSave: (data: ImapConnectConfig) => void;
}

export function ImapConnectPanel({ data, onSave }: ImapConnectPanelProps) {
  // 追踪脏状态和定时器
  const dirtyRef = useRef(false);
  const timersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      timersRef.current.forEach(id => clearTimeout(id));
      timersRef.current.clear();
    };
  }, []);

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      timersRef.current.delete(id);
      fn();
    }, ms);
    timersRef.current.add(id);
  }, []);

  /** 构建当前配置并保存 */
  const save = useCallback(() => {
    // 从 DOM 读取最新值
    const host = (document.getElementById('imap-host') as HTMLInputElement)?.value ?? data.host ?? '';
    const port = Number((document.getElementById('imap-port') as HTMLInputElement)?.value ?? data.port ?? 993);
    const tlsMode = (document.getElementById('imap-tls-mode') as HTMLSelectElement)?.value ?? data.tls_mode ?? 'implicit';
    const username = (document.getElementById('imap-username') as HTMLInputElement)?.value ?? '';
    const password = (document.getElementById('imap-password') as HTMLInputElement)?.value ?? '';

    const config: ImapConnectConfig = {
      type: 'imap_connect',
      host: host || data.host || '',
      port: port || 993,
      tls_mode: (tlsMode || 'implicit') as 'none' | 'starttls' | 'implicit',
      auth: username ? {
        auth_type: 'plain',
        username,
        password: password || undefined,
      } : undefined,
    };
    onSave(config);
  }, [data, onSave]);

  const handleChange = useCallback(() => {
    dirtyRef.current = true;
    safeSetTimeout(() => {
      if (!dirtyRef.current) return;
      dirtyRef.current = false;
      save();
    }, 300);
  }, [safeSetTimeout, save]);

  // 卸载时保存
  useEffect(() => {
    return () => {
      if (!dirtyRef.current) return;
      try { save(); } catch { /* 静默 */ }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 监听 Ctrl+S
  useEffect(() => {
    const handleFlush = () => { save(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [save]);

  const currentTls = data.tls_mode ?? 'implicit';

  return (
    <div className="w-full space-y-5" onBlur={handleChange}>
      {/* 说明卡片 */}
      <div className="rounded-lg border border-blue-200/50 bg-gradient-to-br from-blue-50/80 to-indigo-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-blue-900 flex items-center gap-2">
            <Mail className="h-4 w-4" />
            IMAP 连接节点
          </h4>
          <p className="text-sm leading-relaxed text-blue-700/90 whitespace-normal break-words">
            建立 IMAP 邮箱服务器连接，支持 TLS 加密和多种认证方式。
          </p>
        </div>
      </div>

      {/* 服务器地址 */}
      <div className="space-y-2">
        <Label htmlFor="imap-host" className="text-sm font-medium text-foreground">
          服务器地址 <span className="text-rose-500">*</span>
        </Label>
        <div className="relative">
          <Mail className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
          <Input
            id="imap-host"
            defaultValue={data.host ?? ''}
            placeholder="imap.gmail.com"
            className="pl-9 font-mono text-sm"
          />
        </div>
      </div>

      {/* 端口 */}
      <div className="space-y-2">
        <Label htmlFor="imap-port" className="text-sm font-medium text-foreground">
          端口
        </Label>
        <Input
          id="imap-port"
          type="number"
          defaultValue={data.port ?? 993}
          placeholder="993"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          常用端口: 143（明文）、993（TLS）
        </p>
      </div>

      {/* TLS 模式 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">TLS 模式</Label>
        <Select
          value={currentTls}
          onValueChange={(value) => {
            const config: ImapConnectConfig = {
              ...data,
              type: 'imap_connect',
              tls_mode: value as 'none' | 'starttls' | 'implicit',
            };
            onSave(config);
          }}
        >
          <SelectTrigger id="imap-tls-mode" className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="implicit">隐式 TLS (IMAPS)</SelectItem>
            <SelectItem value="starttls">STARTTLS</SelectItem>
            <SelectItem value="none">无加密</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          推荐使用隐式 TLS（端口 993）
        </p>
      </div>

      {/* 认证配置 */}
      <div className="border rounded-lg overflow-hidden">
        <div className="flex items-center gap-2 px-3 py-2.5 bg-muted/30">
          <Shield className="h-4 w-4 text-blue-600" />
          <span className="text-sm font-medium text-foreground">认证配置</span>
        </div>
        <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
          <div className="space-y-2">
            <Label htmlFor="imap-username" className="text-xs font-medium text-foreground">用户名</Label>
            <Input
              id="imap-username"
              defaultValue={data.auth?.username ?? ''}
              placeholder="user@example.com"
              className="h-8 text-xs font-mono"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="imap-password" className="text-xs font-medium text-foreground">密码</Label>
            <Input
              id="imap-password"
              type="password"
              defaultValue={data.auth?.password ?? ''}
              placeholder="应用专用密码"
              className="h-8 text-xs font-mono"
            />
            <p className="text-[10px] text-muted-foreground">
              Gmail/Outlook 等需使用应用专用密码
            </p>
          </div>
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">IMAP 连接说明：</p>
          <p>IMAP 用于读取和管理邮箱中的邮件</p>
          <p>Gmail 端口 993，需启用 IMAP 并使用应用专用密码</p>
          <p>Outlook 端口 993，TLS 模式选择隐式 TLS</p>
        </div>
      </div>
    </div>
  );
}
