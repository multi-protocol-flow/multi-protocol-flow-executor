/**
 * IMAP Expunge 节点配置组件
 * 支持连接选择、确认提示
 */
import { useEffect } from 'react';
import { Trash2, Info, AlertTriangle } from 'lucide-react';
import type { ImapExpungeConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapExpungePanelProps {
  data: ImapExpungeConfig;
  onSave: (data: ImapExpungeConfig) => void;
}

export function ImapExpungePanel({ data, onSave }: ImapExpungePanelProps) {
  const connections = useImapConnections();

  useEffect(() => {
    const h = () => { onSave(data); };
    window.addEventListener('flush-config-panel', h);
    return () => { window.removeEventListener('flush-config-panel', h); };
  }, [data, onSave]);

  return (
    <div className="w-full space-y-5">
      <div className="rounded-lg border border-blue-200/50 bg-gradient-to-br from-blue-50/80 to-indigo-50/50 p-4">
        <h4 className="text-sm font-semibold text-blue-900 flex items-center gap-2">
          <Trash2 className="h-4 w-4" />
          IMAP 清除已删除节点
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">永久删除标记为 \Deleted 的邮件。</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="flex items-start gap-2 rounded-md border border-amber-200 dark:border-amber-800/60 bg-amber-50 dark:bg-amber-950/30 p-3">
        <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
        <div className="text-xs text-amber-700 dark:text-amber-400 space-y-1">
          <p className="font-medium">危险操作</p>
          <p>此操作将永久删除所有标记为 \Deleted 的邮件，无法撤销。</p>
          <p>请确保已正确选择目标邮箱。</p>
        </div>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">清除说明：</p>
          <p>仅清除当前选中邮箱中标记为 \Deleted 的邮件</p>
          <p>需要先使用 Store Flags 节点将邮件标记为 \Deleted</p>
        </div>
      </div>
    </div>
  );
}
