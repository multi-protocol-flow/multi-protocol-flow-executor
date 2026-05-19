/**
 * IMAP Folder Delete 节点配置组件
 * 支持连接选择、文件夹名称、确认提示
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { FolderMinus, Info, AlertTriangle } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapFolderDeleteConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapFolderDeletePanelProps {
  data: ImapFolderDeleteConfig;
  onSave: (data: ImapFolderDeleteConfig) => void;
}

export function ImapFolderDeletePanel({ data, onSave }: ImapFolderDeletePanelProps) {
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
    const folderName = (document.getElementById('imap-del-folder') as HTMLInputElement)?.value ?? data.folder_name ?? '';
    const config: ImapFolderDeleteConfig = {
      type: 'imap_folder_delete',
      connection_id: data.connection_id || '',
      folder_name: folderName,
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
          <FolderMinus className="h-4 w-4" />
          {getLL().panels.imapFolder.titleDelete()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapFolder.descriptionDelete()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-del-folder" className="text-sm font-medium text-foreground">
          {getLL().panels.imapFolder.folderName()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-del-folder"
          defaultValue={data.folder_name ?? ''}
          placeholder={getLL().panels.imapFolder.folderNameDeletePlaceholder()}
          className="font-mono text-sm"
        />
      </div>

      <div className="flex items-start gap-2 rounded-md border border-amber-200 dark:border-amber-800/60 bg-amber-50 dark:bg-amber-950/30 p-3">
        <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
        <div className="text-xs text-amber-700 dark:text-amber-400 space-y-1">
          <p className="font-medium">{getLL().panels.imapFolder.dangerWarning()}</p>
          <p>{getLL().panels.imapFolder.dangerDesc()}</p>
        </div>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapFolder.helpTitleDelete()}</p>
          <p>{getLL().panels.imapFolder.helpLine1Delete()}</p>
          <p>{getLL().panels.imapFolder.helpLine2Delete()}</p>
        </div>
      </div>
    </div>
  );
}