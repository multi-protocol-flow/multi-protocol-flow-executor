/**
 * ExecSettingsTab - 全局设置中的「本地命令」面板
 *
 * 与后端策略一一对应（`MPE_ALLOW_EXEC` 等环境变量优先级更高）：
 * 总开关 / 是否允许 shell 模式 / 程序白名单 / 已批准列表（逐条撤销、清空全部）。
 */
import { useEffect, useState } from 'react';
import { Terminal, ShieldAlert, Trash2, RefreshCw, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { getLL } from '@/i18n/i18n-lazy';
import { createLogger } from '@/lib/logger';
import { useCurrentLocale } from '@/hooks/useLocale';
import { useSettingsStore } from '@/stores/settings';
import {
  listExecApprovals,
  revokeExecApproval,
  clearExecApprovals,
  type ExecApprovalRecord,
} from '@/services/tauri/execTrust';

/** 时间戳（Unix 秒）→ 本地可读时间 */
function formatCreatedAt(seconds: number): string {
  if (!seconds) return '-';
  const date = new Date(seconds * 1000);
  if (Number.isNaN(date.getTime())) return '-';
  return date.toLocaleString();
}

/**
 * 本地命令设置面板
 */
export function ExecSettingsTab() {
  useCurrentLocale();
  const ll = getLL().settings;
  const execEnabled = useSettingsStore((s) => s.execEnabled);
  const execAllowShell = useSettingsStore((s) => s.execAllowShell);
  const execAllowedPrograms = useSettingsStore((s) => s.execAllowedPrograms);
  const setExecEnabled = useSettingsStore((s) => s.setExecEnabled);
  const setExecAllowShell = useSettingsStore((s) => s.setExecAllowShell);
  const setExecAllowedPrograms = useSettingsStore((s) => s.setExecAllowedPrograms);

  const [programsDraft, setProgramsDraft] = useState((execAllowedPrograms ?? []).join(', '));
  const [approvals, setApprovals] = useState<ExecApprovalRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  useEffect(() => {
    setProgramsDraft((execAllowedPrograms ?? []).join(', '));
  }, [execAllowedPrograms]);

  const refreshApprovals = async () => {
    setLoading(true);
    try {
      setApprovals(await listExecApprovals());
    } catch (err) {
      // 读取失败（信任库损坏 / IPC 不可用）只记日志，不抛给调用方造成未处理拒绝
      createLogger('ExecTrust').error('Failed to load exec approvals', err);
      setApprovals([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refreshApprovals();
  }, []);

  const savePrograms = async (raw: string) => {
    const programs = raw
      .split(',')
      .map((item) => item.trim())
      .filter((item) => item !== '');
    try {
      await setExecAllowedPrograms(programs);
    } catch (err) {
      createLogger('ExecTrust').error('Failed to save exec allowlist', err);
    }
  };

  const revoke = async (record: ExecApprovalRecord) => {
    try {
      await revokeExecApproval(record.flowPath, record.commandHash);
    } catch (err) {
      createLogger('ExecTrust').error('Failed to revoke exec approval', err);
    }
    await refreshApprovals();
  };

  const clearAll = async () => {
    try {
      await clearExecApprovals();
    } catch (err) {
      createLogger('ExecTrust').error('Failed to clear exec approvals', err);
    }
    setConfirmClear(false);
    await refreshApprovals();
  };

  return (
    <div className="space-y-6">
      {/* 总开关 */}
      <section className="space-y-3 rounded-lg border border-border bg-card p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-1">
            <Label className="flex items-center gap-2 text-sm font-medium">
              <Terminal className="h-4 w-4" />
              {ll.exec_enabled()}
            </Label>
            <p className="text-xs text-muted-foreground">{ll.exec_enabled_desc()}</p>
          </div>
          <Switch
            checked={execEnabled}
            onCheckedChange={(checked) => void setExecEnabled(checked)}
          />
        </div>

        <div className="flex items-start justify-between gap-4 border-t border-border/60 pt-3">
          <div className="space-y-1">
            <Label className="flex items-center gap-2 text-sm font-medium">
              <ShieldAlert className="h-4 w-4" />
              {ll.exec_allow_shell()}
            </Label>
            <p className="text-xs text-muted-foreground">{ll.exec_allow_shell_desc()}</p>
          </div>
          <Switch
            checked={execAllowShell}
            onCheckedChange={(checked) => void setExecAllowShell(checked)}
          />
        </div>
      </section>

      {/* 白名单 */}
      <section className="space-y-2 rounded-lg border border-border bg-card p-4">
        <Label className="text-sm font-medium" htmlFor="exec-allowed-programs">
          {ll.exec_allowed_programs()}
        </Label>
        <p className="text-xs text-muted-foreground">{ll.exec_allowed_programs_desc()}</p>
        <div className="flex items-center gap-2">
          <Input
            id="exec-allowed-programs"
            className="font-mono text-sm"
            value={programsDraft}
            placeholder="ffmpeg, /usr/bin/python3"
            onChange={(event) => setProgramsDraft(event.target.value)}
            onBlur={() => void savePrograms(programsDraft)}
          />
          <Button
            variant="outline"
            size="icon"
            title={ll.saved_badge()}
            onClick={() => void savePrograms(programsDraft)}
          >
            <Check className="h-4 w-4" />
          </Button>
        </div>
      </section>

      {/* 已批准列表 */}
      <section className="space-y-3 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between gap-2">
          <Label className="text-sm font-medium">{ll.exec_approvals()}</Label>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="icon" title={ll.exec_approvals()} onClick={() => void refreshApprovals()}>
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            </Button>
            {approvals.length > 0 &&
              (confirmClear ? (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">{ll.exec_clear_confirm()}</span>
                  <Button variant="destructive" size="sm" onClick={() => void clearAll()}>
                    {ll.exec_clear_all()}
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => setConfirmClear(false)}>
                    {ll.exec_approval_deny()}
                  </Button>
                </div>
              ) : (
                <Button variant="outline" size="sm" onClick={() => setConfirmClear(true)}>
                  {ll.exec_clear_all()}
                </Button>
              ))}
          </div>
        </div>

        {approvals.length === 0 ? (
          <p className="text-xs text-muted-foreground">{ll.exec_approvals_empty()}</p>
        ) : (
          <ul className="space-y-2">
            {approvals.map((record) => (
              <li
                key={`${record.flowPath ?? 'session'}:${record.commandHash}`}
                className="flex items-start justify-between gap-3 rounded-md border border-border/60 bg-muted/20 p-2"
              >
                <div className="min-w-0 space-y-1">
                  <div className="truncate font-mono text-xs">{record.display || record.commandHash}</div>
                  <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                    <span className="truncate font-mono">{record.flowPath ?? 'session'}</span>
                    {record.nodeName && <Badge variant="outline">{record.nodeName}</Badge>}
                    <span>{formatCreatedAt(record.createdAt)}</span>
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  title={ll.exec_revoke()}
                  onClick={() => void revoke(record)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
