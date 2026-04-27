/**
 * IMAP Close 节点配置组件
 * 支持连接选择
 */
import { useEffect } from 'react';
import { Unplug, Info } from 'lucide-react';
import type { ImapCloseConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapClosePanelProps {
  data: ImapCloseConfig;
  onSave: (data: ImapCloseConfig) => void;
}

export function ImapClosePanel({ data, onSave }: ImapClosePanelProps) {
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
          <Unplug className="h-4 w-4" />
          IMAP 关闭连接节点
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">关闭 IMAP 连接并释放服务器资源。</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">关闭连接说明：</p>
          <p>关闭连接后将无法继续执行 IMAP 操作</p>
          <p>建议在流程结束时关闭所有 IMAP 连接</p>
          <p>关闭前会自动执行 Expunge（清除已删除邮件）</p>
        </div>
      </div>
    </div>
  );
}
