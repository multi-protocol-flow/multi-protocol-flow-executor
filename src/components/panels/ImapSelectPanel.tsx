/**
 * IMAP Select 节点配置组件
 * 支持连接选择、邮箱名称、只读模式
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Inbox, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapSelectConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapSelectPanelProps {
  data: ImapSelectConfig;
  onSave: (data: ImapSelectConfig) => void;
}

export function ImapSelectPanel({ data, onSave }: ImapSelectPanelProps) {
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
    const mailbox = (document.getElementById('imap-mailbox') as HTMLInputElement)?.value ?? data.mailbox ?? 'INBOX';
    const config: ImapSelectConfig = {
      type: 'imap_select',
      connection_id: data.connection_id || '',
      mailbox: mailbox || 'INBOX',
      readonly: data.readonly,
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
          <Inbox className="h-4 w-4" />
          {getLL().panels.imapSelect.title()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapSelect.description()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => { const c = { ...data, connection_id: v }; onSave(c); }}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-mailbox" className="text-sm font-medium text-foreground">
          {getLL().panels.imapSelect.mailbox()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-mailbox"
          defaultValue={data.mailbox ?? 'INBOX'}
          placeholder="INBOX"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.imapSelect.mailboxHint()}
        </p>
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">{getLL().panels.imapSelect.readonlyMode()}</Label>
          <p className="text-xs text-muted-foreground">{getLL().panels.imapSelect.readonlyModeDesc()}</p>
        </div>
        <Switch
          checked={data.readonly ?? false}
          onCheckedChange={(checked) => {
            onSave({ ...data, readonly: checked || undefined });
          }}
        />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapSelect.helpTitle()}</p>
          <p>{getLL().panels.imapSelect.helpLine1()}</p>
          <p>{getLL().panels.imapSelect.helpLine2()}</p>
        </div>
      </div>
    </div>
  );
}