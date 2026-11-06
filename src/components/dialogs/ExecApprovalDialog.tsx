/**
 * 本地命令审批弹窗（`local:exec`）
 *
 * 监听后端 `exec-approval-request` 事件，展示完整命令行、工作目录、将注入的
 * 环境变量**名**与风险提示，由用户批准或拒绝。命令哈希与流程路径由后端固化，
 * 修改命令后必然重新弹窗。
 */
import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Terminal, AlertTriangle } from 'lucide-react';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { getLL } from '@/i18n/i18n-lazy';
import { isTauriEnv } from '@/lib/platform';
import { createLogger } from '@/lib/logger';
import { useExecApprovalStore } from '@/stores/execApproval';
import {
  EXEC_APPROVAL_EVENT,
  respondExecApproval,
  type ExecApprovalRequest,
} from '@/services/tauri/execTrust';

/**
 * 本地命令审批弹窗
 *
 * 队列为空时不渲染；一次只展示队首请求，`respond_exec_approval` 失败
 * （请求已超时/会话结束）时仅提示并关闭，不阻塞后续审批。
 */
export function ExecApprovalDialog() {
  const ll = getLL();
  const requests = useExecApprovalStore((state) => state.requests);
  const push = useExecApprovalStore((state) => state.push);
  const remove = useExecApprovalStore((state) => state.remove);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!isTauriEnv()) return;
    // 事件订阅失败（WebView 事件总线未就绪等）必须捕获，否则冒泡为未处理拒绝
    const unlistenPromise = listen<ExecApprovalRequest>(EXEC_APPROVAL_EVENT, (event) => {
      if (event.payload) push(event.payload);
    }).catch((err: unknown) => {
      createLogger('ExecApproval').error('Failed to subscribe exec-approval-request', err);
      return null;
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, [push]);

  const current = requests[0];

  const respond = async (approved: boolean) => {
    if (!current || submitting) return;
    setSubmitting(true);
    try {
      await respondExecApproval(current.requestId, approved);
    } catch {
      // 请求已失效（超时或会话结束）：只提示，不阻塞后续审批
      toast.error(ll.settings.exec_approval_unknown());
    } finally {
      remove(current.requestId);
      setSubmitting(false);
    }
  };

  if (!current) return null;

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) void respond(false);
      }}
    >
      <DialogContent className="max-w-2xl" hideCloseButton>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Terminal className="h-4 w-4" />
            {ll.settings.exec_approval_title()}
          </DialogTitle>
          <DialogDescription className="flex items-start gap-2 text-amber-600">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{ll.settings.exec_approval_warning()}</span>
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3 text-sm">
          <div className="space-y-1">
            <div className="text-xs font-medium text-muted-foreground">
              {current.nodeName} · {current.flowName}
            </div>
            <pre className="max-h-56 overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono text-xs whitespace-pre-wrap break-all">
              {current.commandDisplay}
            </pre>
          </div>

          <div className="space-y-1">
            <div className="text-xs font-medium text-muted-foreground">
              {ll.settings.exec_approval_cwd()}
            </div>
            <div className="font-mono text-xs break-all">{current.cwd}</div>
          </div>

          {current.inheritEnvNames.length > 0 && (
            <div className="space-y-1">
              <div className="text-xs font-medium text-muted-foreground">
                {ll.settings.exec_approval_inherit_names()}
              </div>
              <div className="flex flex-wrap gap-1">
                {current.inheritEnvNames.map((name) => (
                  <span
                    key={name}
                    className="rounded bg-amber-500/10 px-1.5 py-0.5 font-mono text-xs text-amber-700 dark:text-amber-400"
                  >
                    {name}
                  </span>
                ))}
              </div>
            </div>
          )}

          {current.envNames.length > 0 && (
            <div className="space-y-1">
              <div className="text-xs font-medium text-muted-foreground">
                {ll.settings.exec_approval_env_names()}
              </div>
              <div className="flex flex-wrap gap-1">
                {current.envNames.map((name) => (
                  <span
                    key={name}
                    className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground"
                  >
                    {name}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => void respond(false)} disabled={submitting}>
            {ll.settings.exec_approval_deny()}
          </Button>
          <Button onClick={() => void respond(true)} disabled={submitting}>
            {ll.settings.exec_approval_approve()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
