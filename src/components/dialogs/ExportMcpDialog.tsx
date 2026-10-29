/**
 * ExportMcpDialog - 发布流程为 MCP Server 弹窗
 *
 * 支持未安装插件一键引导安装、入参变量静态推导展示、
 * Stdio 模式 Claude Desktop / Cursor 配置一键复制生成，
 * 以及 Streamable HTTP 模式本地一键启动。
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
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import {
  Bot,
  Copy,
  Check,
  Play,
  Loader2,
  DownloadCloud,
  Terminal,
  Activity,
  AlertCircle,
} from 'lucide-react';
import { useUIStore } from '@/stores/ui';
import { useMcpStore } from '@/stores/mcp';
import { useFlowStore } from '@/stores/flow';
import { useSettingsStore } from '@/stores/settings';
import { flowStorage } from '@/services/storage';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import type { Flow } from '@/types/flow';
import type { FlowMetadata } from '@/types';
import { DEFAULT_MCP_PORT } from '@/types/mcp';

interface JsonSchemaProperty {
  type?: string;
  default?: unknown;
}

function getSchemaProperty(
  schema: Record<string, unknown>,
  varName: string,
): JsonSchemaProperty | undefined {
  if ('properties' in schema && typeof schema.properties === 'object' && schema.properties !== null) {
    const props = schema.properties as Record<string, JsonSchemaProperty>;
    return props[varName];
  }
  return undefined;
}

export function ExportMcpDialog() {
  const ll = getLL();
  const showExportMcpDialog = useUIStore((s) => s.showExportMcpDialog);
  const exportMcpTarget = useUIStore((s) => s.exportMcpTarget);
  const closeExportMcpDialog = useUIStore((s) => s.closeExportMcpDialog);

  const {
    pluginStatus,
    checkPluginStatus,
    installMcpPlugin,
    inspectFlow,
    activeInspection,
    startServer,
    servers,
    loadServers,
    loading,
  } = useMcpStore();

  const [targetFlow, setTargetFlow] = useState<Flow | null>(null);
  const [flowFilePath, setFlowFilePath] = useState<string>('');
  const [activeTab, setActiveTab] = useState<'stdio' | 'streamable'>('stdio');
  const [clientConfigTab, setClientConfigTab] = useState<'claude' | 'cursor'>('claude');
  const [copied, setCopied] = useState(false);

  const defaultMcpPort = useSettingsStore((s) => s.defaultMcpPort) ?? DEFAULT_MCP_PORT;

  // Streamable parameters. Shared mode joins the global gateway on the
  // configurable default port and is isolated by flow slug path.
  const [sharedGateway, setSharedGateway] = useState(true);
  const [customPort, setCustomPort] = useState(defaultMcpPort);
  const [host, setHost] = useState('127.0.0.1');
  const [timeoutMs, setTimeoutMs] = useState(28000);
  const [isStarting, setIsStarting] = useState(false);

  const effectivePort = sharedGateway ? defaultMcpPort : customPort;

  // Resolve concrete flow file path
  useEffect(() => {
    if (!showExportMcpDialog) return;

    void checkPluginStatus();
    void loadServers();

    if (exportMcpTarget) {
      if ('nodes' in exportMcpTarget && Array.isArray(exportMcpTarget.nodes)) {
        const flow = exportMcpTarget as Flow;
        setTargetFlow(flow);

        void (async () => {
          let resolved = '';
          const tab = useFlowStore.getState().tabs.find((t) => t.uuid === flow.uuid);
          const tabPath = tab?.filePath || (tab as unknown as { file_path?: string })?.file_path;
          if (tabPath) {
            resolved = tabPath;
          } else {
            const list = await flowStorage.listFlows();
            const meta = list.find((m) => m.uuid === flow.uuid);
            const pathFromMeta = meta?.filePath || (meta as unknown as { file_path?: string })?.file_path;
            if (pathFromMeta) {
              resolved = pathFromMeta;
            } else {
              // Ensure flow is persisted to disk so sidecar can read it
              try {
                await flowStorage.saveFlow(flow);
                const updatedList = await flowStorage.listFlows();
                const updatedMeta = updatedList.find((m) => m.uuid === flow.uuid);
                resolved = updatedMeta?.filePath || (updatedMeta as unknown as { file_path?: string })?.file_path || '';
              } catch (e) {
                console.error('[ExportMcpDialog] Auto-save flow error:', e);
              }
            }
          }
          if (resolved) {
            setFlowFilePath(resolved);
            void inspectFlow(resolved);
          }
        })();
      } else {
        const meta = exportMcpTarget as FlowMetadata;
        const filePath = meta.file_path || (meta as unknown as { filePath?: string })?.filePath || '';
        if (filePath) {
          setFlowFilePath(filePath);
          void inspectFlow(filePath);
        } else {
          void flowStorage.listFlows().then((list) => {
            const m = list.find((item) => item.uuid === meta.uuid);
            const path = m?.filePath || (m as unknown as { file_path?: string })?.file_path || '';
            setFlowFilePath(path);
            if (path) void inspectFlow(path);
          });
        }
        void flowStorage.loadFlow(meta.uuid).then((flow) => {
          if (flow) {
            setTargetFlow(flow);
          }
        });
      }
    }
  }, [showExportMcpDialog, exportMcpTarget, checkPluginStatus, loadServers, inspectFlow]);

  const sanitizedToolName = useMemo(() => {
    if (activeInspection?.tool_name) {
      return activeInspection.tool_name;
    }
    const raw = targetFlow?.name || 'mpe_flow';
    return raw
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, '_')
      .replace(/^_+|_+$/g, '') || 'mpe_flow';
  }, [activeInspection, targetFlow]);

  // Route slug used by the shared MCP gateway (`/{slug}/mcp`).
  const flowSlug = useMemo(() => {
    const slug = sanitizedToolName
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
    return slug || 'mcp';
  }, [sanitizedToolName]);

  // Streamable endpoint preview. Both modes expose `/mcp`: a shared gateway
  // aggregates every published flow there, so one registration discovers all
  // tools; `/{slug}/mcp` stays available for per-flow scoping.
  const mcpEndpoint = useMemo(
    () => `http://${host}:${effectivePort}/mcp`,
    [host, effectivePort],
  );

  const scopedEndpoint = useMemo(
    () => `http://${host}:${effectivePort}/${flowSlug}/mcp`,
    [host, effectivePort, flowSlug],
  );

  // Generate client configurations.
  //
  // Both Claude Desktop and Cursor connect over **stdio** here: the client
  // spawns the host binary and speaks JSON-RPC over stdin/stdout, so no port is
  // opened. Streamable HTTP clients use the URL snippet instead.
  const stdioConfig = useMemo(() => {
    const isPluginBin = Boolean(
      pluginStatus?.binary_path && pluginStatus.binary_path.includes('mpe-plugin-mcp'),
    );
    const command = isPluginBin ? pluginStatus?.binary_path || 'mpe-plugin-mcp' : 'mpe';
    const targetFlow = flowFilePath || `${sanitizedToolName}.mpf`;
    const args = isPluginBin
      ? ['--server', '--flow', targetFlow, '--transport', 'stdio']
      : ['mcp', 'serve', '--flow', targetFlow, '--transport', 'stdio'];

    return {
      mcpServers: {
        [sanitizedToolName]: {
          command,
          args,
        },
      },
    };
  }, [sanitizedToolName, pluginStatus?.binary_path, flowFilePath]);

  const claudeConfig = stdioConfig;
  const cursorConfig = stdioConfig;

  const copyConfig = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      toast.success(ll.mcp.copy_success());
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error(ll.mcp.copy_failed());
    }
  };

  const handleStartStreamable = async () => {
    let targetPath = flowFilePath;
    if (!targetPath && targetFlow) {
      try {
        await flowStorage.saveFlow(targetFlow);
        const list = await flowStorage.listFlows();
        const meta = list.find((m) => m.uuid === targetFlow.uuid);
        targetPath = meta?.filePath || (meta as unknown as { file_path?: string })?.file_path || '';
        if (targetPath) {
          setFlowFilePath(targetPath);
        }
      } catch (e) {
        console.error('[ExportMcpDialog] Save error before start:', e);
      }
    }

    if (!targetPath) {
      toast.error(ll.mcp.no_flow_path());
      return;
    }

    setIsStarting(true);
    try {
      await startServer({
        name: sanitizedToolName,
        flow_target: targetPath,
        transport: 'streamable',
        host,
        port: effectivePort,
        timeout_ms: timeoutMs,
        managed_by: 'gui',
        shared: sharedGateway,
      });
    } finally {
      setIsStarting(false);
    }
  };

  const activeServer = useMemo(() => {
    return (servers ?? []).find(
      (s) => s.flow_target === flowFilePath && s.transport === 'streamable',
    );
  }, [servers, flowFilePath]);

  const activeServerEndpoint = useMemo(() => {
    if (!activeServer) return '';
    return `http://${activeServer.host}:${activeServer.port}/mcp`;
  }, [activeServer]);

  if (!showExportMcpDialog) return null;

  return (
    <Dialog open={showExportMcpDialog} onOpenChange={(open) => !open && closeExportMcpDialog()}>
      <DialogContent className="w-full max-w-[620px] p-0 overflow-hidden border-border bg-card">
        <DialogHeader className="px-6 pt-5 pb-3 border-b border-border bg-muted/20">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-amber-500/10 text-amber-500 border border-amber-500/20">
              <Bot className="w-5 h-5" />
            </div>
            <div>
              <DialogTitle className="text-base font-semibold">
                {ll.mcp.dialog_title()}
              </DialogTitle>
              <DialogDescription className="text-xs text-muted-foreground mt-0.5">
                {ll.mcp.dialog_desc()}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="p-6 space-y-5 max-h-[75vh] overflow-y-auto">
          {/* Plugin uninstalled warning card */}
          {pluginStatus && !pluginStatus.installed && (
            <div className="rounded-xl p-4 border border-amber-500/30 bg-amber-500/5 space-y-3">
              <div className="flex items-start gap-3">
                <AlertCircle className="w-5 h-5 text-amber-500 shrink-0 mt-0.5" />
                <div className="space-y-1">
                  <h4 className="text-sm font-medium text-foreground">
                    {ll.mcp.plugin_required_title()}
                  </h4>
                  <p className="text-xs text-muted-foreground leading-relaxed">
                    {ll.mcp.plugin_required_desc()}
                  </p>
                </div>
              </div>
              <div className="flex items-center justify-between pt-1">
                <span className="text-xs text-muted-foreground flex items-center gap-1.5">
                  <Terminal className="w-3.5 h-3.5" />
                  {ll.mcp.cli_hint()}
                </span>
                <Button
                  size="sm"
                  className="bg-amber-600 hover:bg-amber-700 text-white gap-1.5 h-8 text-xs cursor-pointer"
                  disabled={loading}
                  onClick={() => void installMcpPlugin()}
                >
                  {loading ? (
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  ) : (
                    <DownloadCloud className="w-3.5 h-3.5" />
                  )}
                  {loading ? ll.mcp.installing() : ll.mcp.install_plugin()}
                </Button>
              </div>
            </div>
          )}

          {/* Flow information banner */}
          <div className="flex items-center justify-between p-3 rounded-lg border border-border bg-muted/30">
            <div className="space-y-0.5">
              <div className="text-xs text-muted-foreground">{ll.mcp.target_flow()}</div>
              <div className="text-sm font-medium">
                {targetFlow?.name || ll.mcp.current_flow()}
              </div>
            </div>
            <div className="text-right space-y-0.5">
              <div className="text-xs text-muted-foreground">{ll.mcp.tool_name()}</div>
              <Badge
                variant="outline"
                className="font-mono text-xs text-amber-500 border-amber-500/30 bg-amber-500/10"
              >
                {sanitizedToolName}
              </Badge>
            </div>
          </div>

          {/* Transport mode selector */}
          <div className="space-y-3">
            <div className="flex items-center gap-2 border-b border-border">
              <button
                type="button"
                onClick={() => setActiveTab('stdio')}
                className={`pb-2 text-xs font-medium border-b-2 transition-colors cursor-pointer ${
                  activeTab === 'stdio'
                    ? 'border-amber-500 text-amber-500 font-semibold'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }`}
              >
                {ll.mcp.tab_stdio()}
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('streamable')}
                className={`pb-2 text-xs font-medium border-b-2 transition-colors cursor-pointer ${
                  activeTab === 'streamable'
                    ? 'border-amber-500 text-amber-500 font-semibold'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }`}
              >
                {ll.mcp.tab_streamable()}
              </button>
            </div>

            {activeTab === 'stdio' && (
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5 bg-muted p-1 rounded-md">
                    <button
                      type="button"
                      onClick={() => setClientConfigTab('claude')}
                      className={`px-2.5 py-1 text-xs rounded transition-colors cursor-pointer ${
                        clientConfigTab === 'claude'
                          ? 'bg-card text-foreground font-medium shadow-sm'
                          : 'text-muted-foreground hover:text-foreground'
                      }`}
                    >
                      {ll.mcp.tab_claude()}
                    </button>
                    <button
                      type="button"
                      onClick={() => setClientConfigTab('cursor')}
                      className={`px-2.5 py-1 text-xs rounded transition-colors cursor-pointer ${
                        clientConfigTab === 'cursor'
                          ? 'bg-card text-foreground font-medium shadow-sm'
                          : 'text-muted-foreground hover:text-foreground'
                      }`}
                    >
                      {ll.mcp.tab_cursor()}
                    </button>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs gap-1.5 cursor-pointer"
                    onClick={() => {
                      const text = JSON.stringify(
                        clientConfigTab === 'claude' ? claudeConfig : cursorConfig,
                        null,
                        2,
                      );
                      void copyConfig(text);
                    }}
                  >
                    {copied ? (
                      <Check className="w-3.5 h-3.5 text-emerald-500" />
                    ) : (
                      <Copy className="w-3.5 h-3.5" />
                    )}
                    {copied ? ll.mcp.copied() : ll.mcp.copy_config_json()}
                  </Button>
                </div>

                <div className="relative rounded-lg border border-border bg-muted/40 p-3 font-mono text-xs overflow-x-auto">
                  <pre className="text-foreground/90 whitespace-pre">
                    {JSON.stringify(
                      clientConfigTab === 'claude' ? claudeConfig : cursorConfig,
                      null,
                      2,
                    )}
                  </pre>
                </div>

                <p className="text-[11px] text-muted-foreground leading-relaxed">
                  {ll.mcp.stdio_hint()}
                </p>
              </div>
            )}

            {activeTab === 'streamable' && (
              <div className="space-y-4">
                <div className="grid grid-cols-3 gap-3">
                  <div className="space-y-1">
                    <label className="text-xs text-muted-foreground">
                      {ll.mcp.host_label()}
                    </label>
                    <Input
                      size={1}
                      value={host}
                      onChange={(e) => setHost(e.target.value)}
                      className="h-8 text-xs font-mono"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs text-muted-foreground">
                      {ll.mcp.port_label()}
                    </label>
                    <Input
                      type="number"
                      value={effectivePort}
                      disabled={sharedGateway}
                      onChange={(e) => setCustomPort(Number(e.target.value) || DEFAULT_MCP_PORT)}
                      className="h-8 text-xs font-mono disabled:opacity-60"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs text-muted-foreground">
                      {ll.mcp.timeout_label()}
                    </label>
                    <Input
                      type="number"
                      value={timeoutMs}
                      onChange={(e) => setTimeoutMs(Number(e.target.value) || 28000)}
                      className="h-8 text-xs font-mono"
                    />
                  </div>
                </div>

                <label className="flex items-start gap-2 p-2.5 rounded-lg border border-border bg-muted/30 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    checked={sharedGateway}
                    onChange={(e) => setSharedGateway(e.target.checked)}
                    className="mt-0.5 accent-amber-500"
                  />
                  <span className="space-y-0.5">
                    <span className="block text-xs font-medium text-foreground">
                      {ll.mcp.shared_port_mode()}
                    </span>
                    <span className="block text-[11px] text-muted-foreground leading-relaxed">
                      {ll.mcp.shared_port_hint()}
                    </span>
                  </span>
                </label>

                <div className="space-y-1">
                  <div className="text-[11px] text-muted-foreground">{ll.mcp.endpoint_label()}</div>
                  <div className="rounded-md border border-border bg-muted/40 px-2.5 py-1.5 font-mono text-[11px] text-amber-500 truncate">
                    {mcpEndpoint}
                  </div>
                  {sharedGateway && (
                    <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                      <span>{ll.mcp.scoped_endpoint_label()}</span>
                      <span className="font-mono truncate">{scopedEndpoint}</span>
                    </div>
                  )}
                </div>

                {activeServer ? (
                  <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 p-3 flex items-center justify-between">
                    <div className="space-y-0.5">
                      <div className="flex items-center gap-1.5 text-xs text-emerald-500 font-medium">
                        <Activity className="w-3.5 h-3.5 animate-pulse" />
                        {ll.mcp.service_running()} (PID: {activeServer.pid})
                      </div>
                      <div className="text-xs font-mono text-muted-foreground">
                        {activeServerEndpoint}
                      </div>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-7 text-xs gap-1.5 cursor-pointer"
                      onClick={() => void copyConfig(activeServerEndpoint)}
                    >
                      <Copy className="w-3.5 h-3.5" />
                      {ll.mcp.copy_url()}
                    </Button>
                  </div>
                ) : (
                  <Button
                    className="w-full bg-amber-600 hover:bg-amber-700 text-white gap-2 h-9 text-xs cursor-pointer"
                    disabled={isStarting}
                    onClick={() => void handleStartStreamable()}
                  >
                    {isStarting ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <Play className="w-4 h-4" />
                    )}
                    {isStarting ? ll.mcp.starting() : ll.mcp.start_streamable()}
                  </Button>
                )}
              </div>
            )}
          </div>

          {/* Detected flow parameters */}
          <div className="space-y-2 pt-2 border-t border-border">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-foreground">
                {ll.mcp.input_params()}
              </span>
              <span className="text-xs text-muted-foreground">
                {ll.mcp.var_count({ count: activeInspection?.variables.length || 0 })}
              </span>
            </div>

            {activeInspection && activeInspection.variables.length > 0 ? (
              <div className="rounded-lg border border-border overflow-hidden">
                <table className="w-full text-xs">
                  <thead className="bg-muted/50 border-b border-border text-muted-foreground">
                    <tr>
                      <th className="py-1.5 px-3 text-left font-medium">{ll.mcp.var_name()}</th>
                      <th className="py-1.5 px-3 text-left font-medium">{ll.mcp.var_type()}</th>
                      <th className="py-1.5 px-3 text-right font-medium">{ll.mcp.var_default()}</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {activeInspection.variables.map((v) => {
                      const prop = getSchemaProperty(activeInspection.input_schema, v);
                      return (
                        <tr key={v} className="hover:bg-muted/20">
                          <td className="py-1.5 px-3 font-mono font-medium">{v}</td>
                          <td className="py-1.5 px-3 text-muted-foreground font-mono">
                            {prop?.type || 'string'}
                          </td>
                          <td className="py-1.5 px-3 text-right font-mono text-muted-foreground">
                            {prop?.default !== undefined ? String(prop.default) : '-'}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="p-3 text-center text-xs text-muted-foreground border border-dashed rounded-lg">
                {ll.mcp.no_input_params()}
              </div>
            )}
          </div>
        </div>

        <DialogFooter className="px-6 py-3 border-t border-border bg-muted/20 flex justify-between">
          <div className="text-[11px] text-muted-foreground flex items-center gap-1.5">
            <span>{ll.mcp.list_changed_hint()}</span>
          </div>
          <Button variant="outline" size="sm" onClick={closeExportMcpDialog} className="cursor-pointer">
            {ll.mcp.close()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
