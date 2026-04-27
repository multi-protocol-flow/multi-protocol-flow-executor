/**
 * IMAP Search 节点配置组件
 * 支持连接选择、搜索条件（带常用条件快捷标签）、UID 模式
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Search, Info } from 'lucide-react';
import type { ImapSearchConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';
import { cn } from '@/lib/utils';

interface ImapSearchPanelProps {
  data: ImapSearchConfig;
  onSave: (data: ImapSearchConfig) => void;
}

/** 常用搜索条件快捷标签 */
const CRITERIA_PRESETS = [
  { label: 'UNSEEN', value: 'UNSEEN' },
  { label: 'SEEN', value: 'SEEN' },
  { label: 'FLAGGED', value: 'FLAGGED' },
  { label: 'FROM', value: 'FROM ' },
  { label: 'TO', value: 'TO ' },
  { label: 'SUBJECT', value: 'SUBJECT ' },
  { label: 'SINCE', value: 'SINCE ' },
  { label: 'BEFORE', value: 'BEFORE ' },
  { label: 'LARGER', value: 'LARGER ' },
  { label: 'SMALLER', value: 'SMALLER ' },
];

export function ImapSearchPanel({ data, onSave }: ImapSearchPanelProps) {
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
    const criteria = (document.getElementById('imap-criteria') as HTMLInputElement)?.value ?? data.criteria ?? 'ALL';
    const mailbox = (document.getElementById('imap-search-mailbox') as HTMLInputElement)?.value ?? '';
    const config: ImapSearchConfig = {
      type: 'imap_search',
      connection_id: data.connection_id || '',
      mailbox: mailbox || undefined,
      criteria: criteria || 'ALL',
      uid: data.uid,
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

  const handlePreset = useCallback((value: string) => {
    const el = document.getElementById('imap-criteria') as HTMLInputElement;
    if (el) {
      el.value = el.value ? `${el.value} ${value}` : value;
      handleChange();
    }
  }, [handleChange]);

  return (
    <div className="w-full space-y-5" onBlur={handleChange}>
      <div className="rounded-lg border border-blue-200/50 bg-gradient-to-br from-blue-50/80 to-indigo-50/50 p-4">
        <h4 className="text-sm font-semibold text-blue-900 flex items-center gap-2">
          <Search className="h-4 w-4" />
          IMAP 搜索节点
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">在邮箱中搜索匹配条件的邮件。</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-search-mailbox" className="text-sm font-medium text-foreground">
          邮箱（可选）
        </Label>
        <Input
          id="imap-search-mailbox"
          defaultValue={data.mailbox ?? ''}
          placeholder="留空使用当前已选择的邮箱"
          className="font-mono text-sm"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="imap-criteria" className="text-sm font-medium text-foreground">
          搜索条件 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-criteria"
          defaultValue={data.criteria ?? 'ALL'}
          placeholder="ALL"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          如: <code className="rounded bg-muted px-1">UNSEEN FROM &quot;bob&quot; SINCE 01-Jan-2024</code>
        </p>
        {/* 快捷标签 */}
        <div className="flex flex-wrap gap-1.5">
          {CRITERIA_PRESETS.map((preset) => (
            <button
              key={preset.label}
              type="button"
              onClick={() => handlePreset(preset.value)}
              className={cn(
                'px-2 py-0.5 rounded-md border text-[10px] font-mono',
                'border-blue-200 dark:border-blue-800/60 text-blue-700 dark:text-blue-400',
                'bg-blue-50 dark:bg-blue-950/30 hover:bg-blue-100 dark:hover:bg-blue-950/50',
                'transition-colors cursor-pointer'
              )}
            >
              {preset.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2.5">
        <div>
          <Label className="text-sm font-medium text-foreground">UID 模式</Label>
          <p className="text-xs text-muted-foreground">使用 UID 而非序列号标识邮件</p>
        </div>
        <Switch
          checked={data.uid ?? false}
          onCheckedChange={(checked) => onSave({ ...data, uid: checked || undefined })}
        />
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">搜索条件说明：</p>
          <p>多个条件以空格分隔，默认使用 AND 逻辑</p>
          <p>点击上方快捷标签可快速追加常用条件</p>
          <p>UID 模式下返回的结果为邮件的唯一标识符</p>
        </div>
      </div>
    </div>
  );
}
