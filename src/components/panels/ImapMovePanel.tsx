/**
 * IMAP Move 节点配置组件
 * 支持连接选择、UID 集合、目标邮箱
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ArrowRightLeft, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapMoveConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapMovePanelProps {
  data: ImapMoveConfig;
  onSave: (data: ImapMoveConfig) => void;
}

export function ImapMovePanel({ data, onSave }: ImapMovePanelProps) {
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
    const uidSet = (document.getElementById('imap-move-uid') as HTMLInputElement)?.value ?? data.uid_set ?? '';
    const dest = (document.getElementById('imap-move-dest') as HTMLInputElement)?.value ?? data.destination_mailbox ?? '';
    const config: ImapMoveConfig = {
      type: 'imap_move',
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
          <ArrowRightLeft className="h-4 w-4" />
          {getLL().panels.imapMove.title()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapMove.description()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-move-uid" className="text-sm font-medium text-foreground">
          {getLL().panels.imapMove.uidSet()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-move-uid"
          defaultValue={data.uid_set ?? ''}
          placeholder={getLL().panels.partitionRangeFormat()}
          className="font-mono text-sm"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="imap-move-dest" className="text-sm font-medium text-foreground">
          {getLL().panels.imapMove.destinationMailbox()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-move-dest"
          defaultValue={data.destination_mailbox ?? ''}
          placeholder="Archive"
          className="font-mono text-sm"
        />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapMove.helpTitle()}</p>
          <p>{getLL().panels.imapMove.helpLine1()}</p>
          <p>{getLL().panels.imapMove.helpLine2()}</p>
        </div>
      </div>
    </div>
  );
}