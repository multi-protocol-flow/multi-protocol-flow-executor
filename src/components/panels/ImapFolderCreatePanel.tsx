/**
 * IMAP Folder Create 节点配置组件
 * 支持连接选择、文件夹名称
 */
import { useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { FolderPlus, Info } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { ImapFolderCreateConfig } from '@/types';
import { ImapConnectionSelector, useImapConnections } from './shared/ImapShared';

interface ImapFolderCreatePanelProps {
  data: ImapFolderCreateConfig;
  onSave: (data: ImapFolderCreateConfig) => void;
}

export function ImapFolderCreatePanel({ data, onSave }: ImapFolderCreatePanelProps) {
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
    const folderName = (document.getElementById('imap-folder-name') as HTMLInputElement)?.value ?? data.folder_name ?? '';
    const config: ImapFolderCreateConfig = {
      type: 'imap_folder_create',
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
          <FolderPlus className="h-4 w-4" />
          {getLL().panels.imapFolder.titleCreate()}
        </h4>
        <p className="text-sm text-blue-700/90 mt-1">{getLL().panels.imapFolder.descriptionCreate()}</p>
      </div>

      <ImapConnectionSelector
        value={data.connection_id || ''}
        onChange={(v) => onSave({ ...data, connection_id: v })}
        connections={connections}
      />

      <div className="space-y-2">
        <Label htmlFor="imap-folder-name" className="text-sm font-medium text-foreground">
          {getLL().panels.imapFolder.folderName()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="imap-folder-name"
          defaultValue={data.folder_name ?? ''}
          placeholder="NewFolder"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.imapFolder.hierarchyHint()}
        </p>
      </div>

      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.imapFolder.helpTitleCreate()}</p>
          <p>{getLL().panels.imapFolder.helpLine1Create()}</p>
          <p>{getLL().panels.imapFolder.helpLine2Create()}</p>
        </div>
      </div>
    </div>
  );
}