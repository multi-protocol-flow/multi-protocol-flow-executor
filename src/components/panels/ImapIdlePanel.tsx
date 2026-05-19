/**
 * IMAP Idle 节点配置组件
 * 支持连接选择、邮箱名称、超时时间
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Eye, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapIdleConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapIdlePanelProps {
  data: ImapIdleConfig;
  onSave: (data: ImapIdleConfig) => void;
}

export function ImapIdlePanel({ data, onSave }: ImapIdlePanelProps) {
  const connections = useImapConnections();
  const timersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());
  const dirtyRef = useRef(false);

  useEffect(() => {
    return () => { timersRef.current.forEach(id => clearTimeout(id)); timersRef.current.clear(); };
  }, []);

  const safeTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => { timersRef.current.delete(id); fn(); }, ms);
    timersRef.current.add(id);
  }, []);

  const save = useCallback(() => {
    const mailbox = (document.getElementById('imap-idle-mailbox') as HTMLInputElement)?.value ?? data.mailbox ?? 'INBOX';
    const timeout = (document.getElementById('imap-idle-timeout') as HTMLInputElement)?.value ?? '';
    const config: ImapIdleConfig = {
      type: 'imap_idle',
      connection_id: data.connection_id || '',
      mailbox: mailbox || 'INBOX',
      timeout_secs: timeout ? Number(timeout) : undefined,
    };
    onSave(config);
  }, [data, onSave]);

  const handleChange = useCallback(() => {
    dirtyRef.current = true;
    safeTimeout(() => { if (!dirtyRef.current) return; dirtyRef.current = false; save(); }, 300);
  }, [safeTimeout, save]);

  useEffect(() => { return () => { if (!dirtyRef.current) return; try { save(); } catch {} }; }, [save]);

  useEffect(() => {
    const h = () => { save(); };
    window.addEventListener('flush-config-panel', h);
    return () => { window.removeEventListener('flush-config-panel', h); };
  }, [save]);

  return (
    <div className="w-full space-y-5" onBlur={handleChange}>
      <div className="rounded-lg border border-blue-200/50 bg-gradient-to-br from-blue-50/80 to-indigo-50/50 p-4">
        <h4 className="text-sm font-semibold text-blue-900 flex items-center gap-2">
          <Eye className="h-4 w-4" />
          {getLL().panels.imapIdle.title()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapIdle.description()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-idle-mailbox" className="text-sm font-medium text-foreground">
          {getLL().panels.imapIdle.mailbox()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-idle-mailbox"
          defaultValue={data.mailbox ?? 'INBOX'}
          placeholder="INBOX"
          className="font-mono text-sm"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="imap-idle-timeout" className="text-sm font-medium text-foreground">
          {getLL().panels.imapIdle.timeoutSecs()}
        </Label>
        <Input
          id="imap-idle-timeout"
          type="number"
          defaultValue={data.timeout_secs ?? ''}
          placeholder={getLL().panels.imapIdle.timeoutPlaceholder()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.imapIdle.idleTimeoutHint()}
        </p>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapIdle.helpTitle()}</p>
          <p>{getLL().panels.imapIdle.helpLine1()}</p>
          <p>{getLL().panels.imapIdle.helpLine2()}</p>
          <p>{getLL().panels.imapIdle.helpLine3()}</p>
        </div>
      </div>
    </div>
  );
}