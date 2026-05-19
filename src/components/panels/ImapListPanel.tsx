/**
 * IMAP List 节点配置组件
 * 支持连接选择、引用名称、邮箱匹配模式
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { List, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapListConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapListPanelProps {
  data: ImapListConfig;
  onSave: (data: ImapListConfig) => void;
}

export function ImapListPanel({ data, onSave }: ImapListPanelProps) {
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
    const ref = (document.getElementById('imap-list-ref') as HTMLInputElement)?.value ?? '';
    const pattern = (document.getElementById('imap-list-pattern') as HTMLInputElement)?.value ?? '';
    const config: ImapListConfig = {
      type: 'imap_list',
      connection_id: data.connection_id || '',
      reference_name: ref || undefined,
      mailbox_pattern: pattern || undefined,
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
          <List className="h-4 w-4" />
          {getLL().panels.imapList.title()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapList.description()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-list-ref" className="text-sm font-medium text-foreground">
          {getLL().panels.imapList.referenceName()}
        </Label>
        <Input
          id="imap-list-ref"
          defaultValue={data.reference_name ?? ''}
          placeholder={getLL().panels.imapList.referencePlaceholder()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.imapList.referenceHint()}
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="imap-list-pattern" className="text-sm font-medium text-foreground">
          {getLL().panels.imapList.mailboxPattern()}
        </Label>
        <Input
          id="imap-list-pattern"
          defaultValue={data.mailbox_pattern ?? ''}
          placeholder={getLL().panels.imapList.patternPlaceholder()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.imapList.patternHint()}
        </p>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapList.helpTitle()}</p>
          <p>{getLL().panels.imapList.helpLine1()}</p>
          <p>{getLL().panels.imapList.helpLine2()}</p>
        </div>
      </div>
    </div>
  );
}