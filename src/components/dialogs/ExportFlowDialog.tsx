/**
 * 流程导出固化弹窗 (ExportFlowDialog - Bake Environment)
 *
 * 将当前激活环境参数与流程默认变量合并固化，清空勾选的敏感凭据，
 * 导出为脱离外部环境即可独立执行的自包含 .mpf 文件。
 */

import { useEffect, useState, useMemo } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { FileDown, ShieldAlert, Loader2, KeyRound } from 'lucide-react';
import { useUIStore } from '@/stores/ui';
import { useEnvironmentStore } from '@/stores/environment';
import { flowStorage } from '@/services/storage';
import { isTauriEnv } from '@/lib/platform';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import { stringify as stringifyYaml } from 'yaml';
import type { Flow } from '@/types/flow';
import type { EnvironmentSet } from '@/types/environment';
import type { FlowMetadata } from '@/types';

const SENSITIVE_KEY_REGEX = /(key|secret|token|password|passwd|auth|cred|cert|private)/i;

/**
 * 构建导出用流程对象（纯函数；凭据清理属安全关键逻辑，单独抽出便于回归测试）
 *
 * - `bakeEnvironment` 为真：写入「流程内置变量 + 激活环境变量」的合并结果，
 *   并清空 `sanitizeKeys` 中的键。**未激活环境时同样必须清理**，
 *   否则弹窗已勾选的敏感变量会原样泄漏进导出文件。
 * - `bakeEnvironment` 为假：原样导出，不触碰 `initial_variables`。
 */
export function buildExportedFlow(params: {
  targetFlow: Flow;
  bakeEnvironment: boolean;
  activeEnvironmentData: EnvironmentSet | null;
  sanitizeKeys: ReadonlySet<string>;
}): Flow {
  const { targetFlow, bakeEnvironment, activeEnvironmentData, sanitizeKeys } = params;

  // 深拷贝，避免污染画布中正在编辑的流程对象
  const exported: Flow = JSON.parse(JSON.stringify(targetFlow));
  if (!bakeEnvironment) {
    return exported;
  }

  const merged: Record<string, unknown> = {
    ...(targetFlow.initial_variables ?? {}),
    ...(activeEnvironmentData?.variables ?? {}),
  };
  for (const key of sanitizeKeys) {
    if (key in merged) {
      merged[key] = '';
    }
  }
  exported.initial_variables = merged;
  return exported;
}

export function ExportFlowDialog() {
  const ll = getLL().flow;
  const showExportFlowDialog = useUIStore((s) => s.showExportFlowDialog);
  const exportFlowTarget = useUIStore((s) => s.exportFlowTarget);
  const closeExportFlowDialog = useUIStore((s) => s.closeExportFlowDialog);

  const { activeEnvironmentName, activeEnvironmentData, loadEnvironments } = useEnvironmentStore();

  const [bakeEnvironment, setBakeEnvironment] = useState<boolean>(true);
  const [selectedSanitizeKeys, setSelectedSanitizeKeys] = useState<Set<string>>(new Set());
  const [isLoadingFlow, setIsLoadingFlow] = useState<boolean>(false);
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [targetFlow, setTargetFlow] = useState<Flow | null>(() => {
    if (exportFlowTarget && 'nodes' in exportFlowTarget && Array.isArray(exportFlowTarget.nodes)) {
      return exportFlowTarget as Flow;
    }
    return null;
  });
  useEffect(() => {
    if (!showExportFlowDialog || !exportFlowTarget) {
      setTargetFlow(null);
      setIsLoadingFlow(false);
      return;
    }

    void loadEnvironments();

    // 判断是完整 Flow 还是 FlowMetadata
    if ('nodes' in exportFlowTarget && Array.isArray(exportFlowTarget.nodes)) {
      setTargetFlow(exportFlowTarget as Flow);
    } else {
      setIsLoadingFlow(true);
      const meta = exportFlowTarget as FlowMetadata;
      flowStorage
        .loadFlow(meta.uuid)
        .then((f) => {
          setTargetFlow(f);
        })
        .catch((e) => {
          toast.error(e instanceof Error ? e.message : String(e));
          closeExportFlowDialog();
        })
        .finally(() => {
          setIsLoadingFlow(false);
        });
    }
  }, [showExportFlowDialog, exportFlowTarget, loadEnvironments, closeExportFlowDialog]);

  // 合并后的候选变量（当勾选 Bake 时）
  const mergedCandidateVariables = useMemo<Record<string, unknown>>(() => {
    const base = { ...(targetFlow?.initial_variables ?? {}) };
    if (!bakeEnvironment || !activeEnvironmentData) {
      return base;
    }
    return { ...base, ...activeEnvironmentData.variables };
  }, [targetFlow, bakeEnvironment, activeEnvironmentData]);

  // 敏感凭据键识别
  const detectedSensitiveKeys = useMemo<string[]>(() => {
    return Object.keys(mergedCandidateVariables).filter((k) => SENSITIVE_KEY_REGEX.test(k));
  }, [mergedCandidateVariables]);

  // 默认全部勾选敏感变量清理
  useEffect(() => {
    setSelectedSanitizeKeys(new Set(detectedSensitiveKeys));
  }, [detectedSensitiveKeys]);

  const toggleSanitizeKey = (key: string) => {
    setSelectedSanitizeKeys((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const handleExport = async () => {
    if (!targetFlow) return;
    setIsExporting(true);
    try {
      // 1. 深拷贝 + 固化环境参数 + 清空勾选的敏感键（纯函数，见 buildExportedFlow）
      const exported = buildExportedFlow({
        targetFlow,
        bakeEnvironment,
        activeEnvironmentData,
        sanitizeKeys: selectedSanitizeKeys,
      });

      // 2. 构建 FlowFile 包装格式
      const flowFile = {
        _version: 1,
        flow: exported,
      };

      const yamlContent = stringifyYaml(flowFile);

      if (isTauriEnv()) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const defaultFilename = `${exported.name.replace(/[/\\:*?"<>|]/g, '_')}.mpf`;
        const selectedPath = await save({
          defaultPath: defaultFilename,
          filters: [{ name: 'MPE Flow File (*.mpf)', extensions: ['mpf', 'yaml', 'yml'] }],
        });

        if (!selectedPath) {
          setIsExporting(false);
          return; // 用户取消
        }

        await flowStorage.saveFlow(exported, selectedPath);
        toast.success(ll.export_success());
        closeExportFlowDialog();
      } else {
        // 浏览器环境触发下载
        const blob = new Blob([yamlContent], { type: 'text/yaml;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${exported.name}.mpf`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        toast.success(ll.export_success());
        closeExportFlowDialog();
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setIsExporting(false);
    }
  };

  return (
    <Dialog open={showExportFlowDialog} onOpenChange={(open) => !open && closeExportFlowDialog()}>
      <DialogContent className="w-full max-w-[500px]">
        <DialogHeader>
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
              <FileDown className="w-4 h-4" />
            </div>
            <div>
              <DialogTitle className="text-base font-semibold">
                {ll.export_flow_dialog_title()}
              </DialogTitle>
              <DialogDescription className="text-xs text-muted-foreground mt-0.5">
                {ll.export_flow_target({ name: targetFlow?.name ?? '...' })}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        {isLoadingFlow ? (
          <div className="py-8 flex items-center justify-center gap-2 text-xs text-muted-foreground">
            <Loader2 className="w-4 h-4 animate-spin text-primary" />
            <span>{ll.loading_flow_data()}</span>
          </div>
        ) : (
          <div className="space-y-4 py-2 text-xs">
            {/* Bake Environment 复选框 */}
            <div className="p-3 rounded-lg border border-border bg-muted/20 space-y-2">
              <label className="flex items-start gap-2.5 cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={bakeEnvironment}
                  onChange={(e) => setBakeEnvironment(e.target.checked)}
                  className="mt-0.5 rounded border-border text-primary focus:ring-primary h-4 w-4"
                />
                <div className="space-y-1">
                  <span className="font-medium text-foreground block">
                    {ll.bake_env_checkbox()}
                  </span>
                  <p className="text-[11px] text-muted-foreground leading-relaxed">
                    {activeEnvironmentName
                      ? ll.bake_env_desc({
                          name: activeEnvironmentName,
                          count: Object.keys(activeEnvironmentData?.variables ?? {}).length,
                        })
                      : ll.bake_no_env_desc()}
                  </p>
                </div>
              </label>
            </div>

            {/* 敏感凭据扫描 */}
            {bakeEnvironment && detectedSensitiveKeys.length > 0 && (
              <div className="p-3 rounded-lg border border-amber-500/30 bg-amber-500/5 space-y-2.5">
                <div className="flex items-center gap-2 text-amber-600 dark:text-amber-400 font-medium">
                  <ShieldAlert className="w-4 h-4 shrink-0" />
                  <span>{ll.sensitive_vars_title()}</span>
                </div>
                <p className="text-[11px] text-muted-foreground leading-relaxed">
                  {ll.sensitive_vars_desc()}
                </p>

                <div className="space-y-1.5 pt-1 max-h-36 overflow-y-auto pr-1 scrollbar-thin">
                  {detectedSensitiveKeys.map((key) => {
                    const isChecked = selectedSanitizeKeys.has(key);
                    return (
                      <label
                        key={key}
                        className="flex items-center justify-between px-2 py-1.5 rounded-md hover:bg-amber-500/10 transition-colors cursor-pointer select-none text-[11px]"
                      >
                        <div className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={isChecked}
                            onChange={() => toggleSanitizeKey(key)}
                            className="rounded border-amber-500/40 text-amber-600 focus:ring-amber-500 h-3.5 w-3.5"
                          />
                          <KeyRound className="w-3.5 h-3.5 text-amber-500 shrink-0" />
                          <span className="font-mono font-medium text-foreground">{key}</span>
                        </div>
                        <span className="text-[10px] text-muted-foreground">
                          {isChecked ? ll.clear_value_on_export() : ll.keep_original_value()}
                        </span>
                      </label>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        )}

        <DialogFooter className="gap-2 sm:gap-0 pt-2 border-t border-border">
          <Button variant="outline" size="sm" onClick={closeExportFlowDialog} disabled={isExporting}>
            {getLL().common.CANCEL()}
          </Button>
          <Button
            size="sm"
            onClick={handleExport}
            disabled={!targetFlow || isLoadingFlow || isExporting}
            className="gap-1.5 cursor-pointer"
          >
            {isExporting ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>{ll.exporting()}</span>
              </>
            ) : (
              <>
                <FileDown className="w-3.5 h-3.5" />
                <span>{ll.export_btn()}</span>
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
