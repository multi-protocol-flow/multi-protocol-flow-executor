/**
 * IMAP Fetch 节点配置组件
 * 支持连接选择、UID 集合、获取项、最大消息数
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Download, Info } from 'lucide-react';
import type { ImapFetchConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapFetchPanelProps {
  data: ImapFetchConfig;
  onSave: (data: ImapFetchConfig) => void;
}

export function ImapFetchPanel({ data, onSave }: ImapFetchPanelProps) {
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
    const uidSet = (document.getElementById('imap-uid-set') as HTMLInputElement)?.value ?? data.uid_set ?? '';
    const maxMsg = (document.getElementById('imap-max-msg') as HTMLInputElement)?.value ?? '';
    const config: ImapFetchConfig = {
      type: 'imap_fetch',
      connection_id: data.connection_id || '',
      uid_set: uidSet || '1:*',
      items: data.items || '(ENVELOPE BODY.PEEK[] FLAGS)',
      max_messages: maxMsg ? Number(maxMsg) : undefined,
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
          <Download className="h-4 w-4" />
          IMAP 获取邮件节点
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">根据 UID 集合获取邮件内容。</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-uid-set" className="text-sm font-medium text-foreground">
          UID 集合 <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-uid-set"
          defaultValue={data.uid_set ?? ''}
          placeholder="1:10 或 1,3,5,7"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          格式: <code className="rounded bg-muted px-1">1:10</code>（范围）或 <code className="rounded bg-muted px-1">1,3,5</code>（列表）或 <code className="rounded bg-muted px-1">1:*</code>（全部）
        </p>
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">获取项</Label>
        <Select
          value={data.items || '(ENVELOPE BODY.PEEK[] FLAGS)'}
          onValueChange={(value) => onSave({ ...data, items: value })}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="(ENVELOPE BODY.PEEK[] FLAGS)">完整内容（信封+正文+标志）</SelectItem>
            <SelectItem value="(ENVELOPE)">仅信封（主题/发件人/日期）</SelectItem>
            <SelectItem value="(BODY.PEEK[])">仅正文</SelectItem>
            <SelectItem value="(FLAGS)">仅标志</SelectItem>
            <SelectItem value="(BODYSTRUCTURE)">邮件结构</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="imap-max-msg" className="text-sm font-medium text-foreground">
          最大消息数
        </Label>
        <Input
          id="imap-max-msg"
          type="number"
          defaultValue={data.max_messages ?? ''}
          placeholder="不限"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">限制获取的消息数量，留空表示不限制</p>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">获取邮件说明：</p>
          <p>UID 集合通常由搜索节点返回</p>
          <p>使用 BODY.PEEK[] 不会将邮件标记为已读</p>
          <p>大量邮件获取建议设置最大消息数限制</p>
        </div>
      </div>
    </div>
  );
}
