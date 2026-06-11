/**
 * IMAP Store Flags 节点配置组件
 * 支持连接选择、UID 集合、操作类型、标志列表
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Flag, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapStoreFlagsConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';
import { cn } from '@/lib/utils';

interface ImapStoreFlagsPanelProps {
  data: ImapStoreFlagsConfig;
  onSave: (data: ImapStoreFlagsConfig) => void;
}

const FLAGS_PRESETS = ['\\Seen', '\\Answered', '\\Flagged', '\\Deleted', '\\Draft', '\\Recent'];

export function ImapStoreFlagsPanel({ data, onSave }: ImapStoreFlagsPanelProps) {
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
    const uidSet = (document.getElementById('imap-store-uid') as HTMLInputElement)?.value ?? data.uid_set ?? '';
    const config: ImapStoreFlagsConfig = {
      type: 'imap_store_flags',
      connection_id: data.connection_id || '',
      uid_set: uidSet || '1:*',
      operation: data.operation || 'set',
      flags: data.flags || [],
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

  const currentFlags = data.flags || [];

  const toggleFlag = useCallback((flag: string) => {
    const newFlags = currentFlags.includes(flag)
      ? currentFlags.filter(f => f !== flag)
      : [...currentFlags, flag];
    onSave({ ...data, flags: newFlags });
  }, [currentFlags, data, onSave]);

  return (
    <div className="w-full space-y-5" onBlur={handleChange}>
      <div className="rounded-lg border border-blue-200/50 bg-gradient-to-br from-blue-50/80 to-indigo-50/50 p-4">
        <h4 className="text-sm font-semibold text-blue-900 flex items-center gap-2">
          <Flag className="h-4 w-4" />
          {getLL().panels.imapStoreFlags.title()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapStoreFlags.description()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-store-uid" className="text-sm font-medium text-foreground">
          {getLL().panels.imapStoreFlags.uidSet()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-store-uid"
          defaultValue={data.uid_set ?? ''}
          placeholder={getLL().panels.partitionRangeFormat()}
          className="font-mono text-sm"
        />
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.imapStoreFlags.operationType()}</Label>
        <Select
          value={data.operation || 'set'}
          onValueChange={(value) => onSave({ ...data, operation: value as 'set' | 'add' | 'remove' })}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="set">{getLL().panels.imapStoreFlags.operationSet()}</SelectItem>
            <SelectItem value="add">{getLL().panels.imapStoreFlags.operationAdd()}</SelectItem>
            <SelectItem value="remove">{getLL().panels.imapStoreFlags.operationRemove()}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.imapStoreFlags.flags()}</Label>
        <div className="flex flex-wrap gap-1.5">
          {FLAGS_PRESETS.map((flag) => (
            <button
              key={flag}
              type="button"
              onClick={() => toggleFlag(flag)}
              className={cn(
                'px-2 py-0.5 rounded-md border text-[10px] font-mono transition-colors cursor-pointer',
                currentFlags.includes(flag)
                  ? 'border-blue-400 bg-blue-100 dark:bg-blue-950/50 text-blue-700 dark:text-blue-300'
                  : 'border-border bg-muted/50 text-muted-foreground hover:bg-muted'
              )}
            >
              {flag}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapStoreFlags.helpTitle()}</p>
          <p>{getLL().panels.imapStoreFlags.helpLine1()}</p>
          <p>{getLL().panels.imapStoreFlags.helpLine2()}</p>
          <p>{getLL().panels.imapStoreFlags.helpLine3()}</p>
        </div>
      </div>
    </div>
  );
}