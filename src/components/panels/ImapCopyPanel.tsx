/**
 * IMAP Copy 节点配置组件
 * 支持连接选择、UID 集合、目标邮箱
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Copy, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapCopyConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapCopyPanelProps {
  data: ImapCopyConfig;
  onSave: (data: ImapCopyConfig) => void;
}

export function ImapCopyPanel({ data, onSave }: ImapCopyPanelProps) {
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
    const uidSet = (document.getElementById('imap-copy-uid') as HTMLInputElement)?.value ?? data.uid_set ?? '';
    const dest = (document.getElementById('imap-copy-dest') as HTMLInputElement)?.value ?? data.destination_mailbox ?? '';
    const config: ImapCopyConfig = {
      type: 'imap_copy',
      connection_id: data.connection_id || '',
      uid_set: uidSet || '1:*',
      destination_mailbox: dest,
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
          <Copy className="h-4 w-4" />
          {getLL().panels.imapCopy.title()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapCopy.description()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-copy-uid" className="text-sm font-medium text-foreground">
          {getLL().panels.imapCopy.uidSet()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-copy-uid"
          defaultValue={data.uid_set ?? ''}
          placeholder={getLL().panels.partitionRangeFormat()}
          className="font-mono text-sm"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="imap-copy-dest" className="text-sm font-medium text-foreground">
          {getLL().panels.imapCopy.destinationMailbox()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-copy-dest"
          defaultValue={data.destination_mailbox ?? ''}
          placeholder="Archive"
          className="font-mono text-sm"
        />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapCopy.helpTitle()}</p>
          <p>{getLL().panels.imapCopy.helpLine1()}</p>
          <p>{getLL().panels.imapCopy.helpLine2()}</p>
        </div>
      </div>
    </div>
  );
}