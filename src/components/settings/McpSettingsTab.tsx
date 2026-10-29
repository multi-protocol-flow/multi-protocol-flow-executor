/**
 * McpSettingsTab - 全局设置中的 MCP 服务治理管理面板
 *
 * 展示受宿主监管的 MCP 运行实例，支持僵尸自愈、状态指示灯、
 * 端口、PID、启停、重启、重载以及快速查看客户端配置。
 */

import { useEffect, useMemo, useState } from 'react';
import {
  Server,
  RefreshCw,
  Play,
  Square,
  ExternalLink,
  RotateCcw,
  Copy,
  Check,
  Code,
  Activity,
  AlertCircle,
  DownloadCloud,
  Loader2,
  Trash2,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { useMcpStore } from '@/stores/mcp';
import { useUIStore } from '@/stores/ui';
import { useFlowStore } from '@/stores/flow';
import { flowStorage } from '@/services/storage';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import type { McpServerInstance, McpClientSnippets } from '@/types/mcp';

/**
 * Client-facing endpoint. Published flows share the gateway endpoint (`/mcp`),
 * where one registration discovers every tool; `/{slug}/mcp` scopes to one flow.
 */
function serverEndpoint(server: McpServerInstance): string {
  if (server.transport !== 'streamable') {
    return server.flow_target || server.dir_target || '-';
  }
  return `http://${server.host}:${server.port}/mcp`;
}

interface EndpointGroup {
  key: string;
  endpoint: string;
  shared: boolean;
  instances: McpServerInstance[];
  toolsTotal: number;
  stopped: number;
}

/** Slug-scoped endpoint (per-flow isolation), if this instance has one. */
function serverScopedEndpoint(server: McpServerInstance): string | undefined {
  if (server.transport !== 'streamable' || !server.slug) return undefined;
  return `http://${server.host}:${server.port}/${server.slug}/mcp`;
}

export function McpSettingsTab() {
  const ll = getLL();
  const {
    pluginStatus,
    checkPluginStatus,
    installMcpPlugin,
    servers,
    loadServers,
    stopServer,
    removeServer,
    reloadServer,
    restoreServers,
    loadClientConfig,
    loading,
  } = useMcpStore();

  const [selectedSnippetServer, setSelectedSnippetServer] = useState<McpServerInstance | null>(null);
  const [snippets, setSnippets] = useState<McpClientSnippets | null>(null);
  const [copiedTab, setCopiedTab] = useState<'claude' | 'cursor' | null>(null);
  const [activeSnippetTab, setActiveSnippetTab] = useState<'claude' | 'cursor'>('claude');

  useEffect(() => {
    void checkPluginStatus();
    void loadServers();
  }, [checkPluginStatus, loadServers]);

  const handleOpenSnippets = async (server: McpServerInstance) => {
    setSelectedSnippetServer(server);
    const snips = await loadClientConfig(server);
    if (snips) {
      setSnippets(snips);
    }
  };

  const handleCopy = async (text: string, tab: 'claude' | 'cursor') => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedTab(tab);
      toast.success(ll.mcp.copy_success());
      setTimeout(() => setCopiedTab(null), 2000);
    } catch {
      toast.error(ll.mcp.copy_failed());
    }
  };

  /**
   * Published instances grouped by the endpoint a client actually configures.
   * Every flow on the shared gateway maps to the same `/mcp` endpoint, so the
   * endpoint is rendered once per group instead of repeated on each row.
   */
  const groups = useMemo<EndpointGroup[]>(() => {
    const byEndpoint = new Map<string, EndpointGroup>();
    for (const server of servers) {
      const endpoint = serverEndpoint(server);
      const key = server.transport === 'streamable' ? endpoint : `id:${server.id}`;
      const group = byEndpoint.get(key) ?? {
        key,
        endpoint,
        shared: Boolean(server.shared),
        instances: [],
        toolsTotal: 0,
        stopped: 0,
      };
      group.instances.push(server);
      group.toolsTotal += server.tools_count;
      if (server.status !== 'running') group.stopped += 1;
      byEndpoint.set(key, group);
    }
    return Array.from(byEndpoint.values());
  }, [servers]);

  /** Jump from a published instance to its flow on the canvas. */
  const handleOpenFlow = async (server: McpServerInstance) => {
    const target = server.flow_target;
    if (!target) {
      toast.error(ll.mcp.no_flow_path());
      return;
    }

    const ui = useUIStore.getState();
    const flowState = useFlowStore.getState();
    const existing = flowState.tabs.find(
      (tab) => tab.filePath === target || tab.uuid === target,
    );
    if (existing) {
      ui.closeSettingsDialog();
      flowState.switchTab(existing.uuid);
      ui.revealFlowInList({ uuid: existing.uuid, filePath: existing.filePath });
      return;
    }

    try {
      const flows = await flowStorage.listFlows();
      const meta = flows.find(
        (m) =>
          m.filePath === target ||
          (m as unknown as { file_path?: string }).file_path === target,
      );
      const flow = await flowStorage.loadFlow(meta?.uuid || target);
      if (!flow) {
        toast.error(ll.mcp.no_flow_path());
        return;
      }
      ui.closeSettingsDialog();
      const opened = flowState.openTab(flow, target);
      if (opened) {
        ui.revealFlowInList({ uuid: flow.uuid, filePath: target });
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="space-y-6">
      {/* Plugin health / status card */}
      <div className="rounded-xl border border-border bg-card p-4 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-lg bg-amber-500/10 text-amber-500 border border-amber-500/20">
              <Server className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-sm font-medium text-foreground">{ll.mcp.status_title()}</h3>
              <p className="text-xs text-muted-foreground">{ll.mcp.status_desc()}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {pluginStatus?.installed ? (
              <Badge className="bg-emerald-500/10 text-emerald-500 border-emerald-500/20 text-xs gap-1">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                {ll.mcp.status_ready({ version: pluginStatus.version || '0.1.3' })}
              </Badge>
            ) : (
              <Badge variant="outline" className="text-amber-500 border-amber-500/30 text-xs gap-1">
                <AlertCircle className="w-3 h-3" />
                {ll.mcp.status_not_installed()}
              </Badge>
            )}
            <Button
              variant="outline"
              size="sm"
              className="h-8 text-xs gap-1.5 cursor-pointer"
              disabled={loading}
              onClick={() => {
                void checkPluginStatus();
                void loadServers();
              }}
            >
              <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
              {ll.mcp.refresh()}
            </Button>
          </div>
        </div>

        {pluginStatus && !pluginStatus.installed && (
          <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 flex items-center justify-between">
            <div className="text-xs text-muted-foreground">
              {ll.mcp.plugin_missing_tip()}
            </div>
            <Button
              size="sm"
              className="bg-amber-600 hover:bg-amber-700 text-white gap-1.5 h-7 text-xs cursor-pointer"
              disabled={loading}
              onClick={() => void installMcpPlugin()}
            >
              {loading ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <DownloadCloud className="w-3.5 h-3.5" />
              )}
              {loading ? ll.mcp.installing() : ll.mcp.install_plugin_btn()}
            </Button>
          </div>
        )}

        {pluginStatus?.binary_path && (
          <div className="text-xs font-mono text-muted-foreground truncate bg-muted/40 px-2.5 py-1.5 rounded border border-border">
            {ll.mcp.binary_path()}: {pluginStatus.binary_path}
          </div>
        )}
      </div>

      {/* Managed instances table */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium text-foreground">
            {ll.mcp.active_instances({ count: servers.length })}
          </h3>
          <span className="text-xs text-muted-foreground">{ll.mcp.zombie_clean_hint()}</span>
        </div>

        {groups.length > 0 ? (
          <div className="space-y-3">
            {groups.map((group) => (
              <div
                key={group.key}
                className="rounded-xl border border-border bg-card overflow-hidden"
              >
                {/* Endpoint header: one client registration covers every flow here */}
                <div className="flex items-center justify-between gap-3 px-3 py-2 bg-muted/50 border-b border-border">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="text-[10px] uppercase tracking-wide text-muted-foreground shrink-0">
                      {group.shared ? ll.mcp.group_shared() : ll.mcp.group_dedicated()}
                    </span>
                    <span
                      className="font-mono text-[11px] text-amber-500 truncate"
                      title={group.endpoint}
                    >
                      {group.endpoint}
                    </span>
                    <Badge variant="outline" className="text-[10px] shrink-0">
                      {ll.mcp.group_flows({ count: group.instances.length })}
                    </Badge>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="w-7 h-7 cursor-pointer shrink-0"
                      title={ll.mcp.copy_url()}
                      onClick={() => void handleCopy(group.endpoint, 'claude')}
                    >
                      <Copy className="w-3.5 h-3.5 text-muted-foreground" />
                    </Button>
                    {group.stopped > 0 && (
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-7 text-[11px] gap-1 cursor-pointer shrink-0"
                        disabled={loading}
                        onClick={() =>
                          void restoreServers(
                            group.instances
                              .filter((inst) => inst.status !== 'running')
                              .map((inst) => inst.id),
                          )
                        }
                      >
                        <Play className="w-3 h-3" />
                        {ll.mcp.restore_all()}
                      </Button>
                    )}
                  </div>
                </div>

                <div className="overflow-x-auto">
                  <table className="w-full text-xs">
                    <thead className="border-b border-border text-muted-foreground">
                      <tr>
                        <th className="py-2 px-3 text-left font-medium min-w-[130px] max-w-[180px] whitespace-nowrap">
                          {ll.mcp.col_name()}
                        </th>
                        <th className="py-2 px-3 text-left font-medium min-w-[150px] max-w-[220px] whitespace-nowrap">
                          {ll.mcp.col_tool_name()}
                        </th>
                        <th className="py-2 px-3 text-left font-medium w-16 whitespace-nowrap">
                          {ll.mcp.col_pid()}
                        </th>
                        <th className="py-2 px-3 text-center font-medium w-24 whitespace-nowrap">
                          {ll.mcp.col_status()}
                        </th>
                        <th className="py-2 px-3 text-right font-medium w-36 min-w-[144px] whitespace-nowrap">
                          {ll.mcp.col_actions()}
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y border-border">
                      {group.instances.map((srv) => (
                        <tr key={srv.id} className="hover:bg-muted/20">
                          <td className="py-2 px-3 max-w-[180px]">
                            <button
                              type="button"
                              className="group flex items-center gap-1 max-w-[160px] text-left cursor-pointer"
                              title={ll.mcp.action_open_flow()}
                              onClick={() => void handleOpenFlow(srv)}
                            >
                              <span className="font-medium text-foreground truncate group-hover:text-primary group-hover:underline">
                                {srv.name}
                              </span>
                              <ExternalLink className="w-3 h-3 shrink-0 text-muted-foreground opacity-0 group-hover:opacity-100" />
                            </button>
                            <div
                              className="font-mono text-[10px] text-muted-foreground truncate max-w-[160px]"
                              title={srv.id}
                            >
                              {srv.id}
                            </div>
                          </td>
                          <td className="py-2 px-3 max-w-[220px]">
                            <div
                              className="font-mono text-[11px] text-foreground truncate max-w-[200px]"
                              title={srv.tool_name || '-'}
                            >
                              {srv.tool_name || '-'}
                            </div>
                            {serverScopedEndpoint(srv) && (
                              <div
                                className="font-mono text-[10px] text-muted-foreground truncate max-w-[200px]"
                                title={serverScopedEndpoint(srv)}
                              >
                                {serverScopedEndpoint(srv)}
                              </div>
                            )}
                          </td>
                          <td className="py-2 px-3 font-mono whitespace-nowrap">
                            {srv.status === 'running' ? srv.pid : '-'}
                          </td>
                          <td className="py-2 px-3 text-center whitespace-nowrap">
                            <span
                              className={`inline-block w-2 h-2 rounded-full ${
                                srv.status === 'running'
                                  ? 'bg-emerald-500 animate-pulse'
                                  : 'bg-muted-foreground/40'
                              }`}
                              title={srv.status}
                              aria-label={srv.status}
                            />
                          </td>
                          <td className="py-2 px-3 text-right whitespace-nowrap w-36 min-w-[144px]">
                            <div className="flex items-center justify-end gap-1 flex-nowrap shrink-0">
                              <Button
                                variant="ghost"
                                size="icon"
                                className="w-7 h-7 cursor-pointer shrink-0"
                                title={ll.mcp.action_snippets()}
                                onClick={() => void handleOpenSnippets(srv)}
                              >
                                <Code className="w-3.5 h-3.5 text-muted-foreground" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="w-7 h-7 cursor-pointer shrink-0 disabled:opacity-40"
                                title={ll.mcp.action_reload()}
                                disabled={srv.status !== 'running'}
                                onClick={() => void reloadServer(srv.id)}
                              >
                                <RotateCcw className="w-3.5 h-3.5 text-muted-foreground" />
                              </Button>
                              {srv.status === 'running' ? (
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="w-7 h-7 cursor-pointer shrink-0"
                                  title={ll.mcp.action_stop()}
                                  onClick={() => void stopServer(srv.id)}
                                >
                                  <Square className="w-3.5 h-3.5 text-amber-500" fill="currentColor" />
                                </Button>
                              ) : (
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="w-7 h-7 cursor-pointer shrink-0"
                                  title={ll.mcp.action_restore()}
                                  disabled={loading}
                                  onClick={() => void restoreServers([srv.id])}
                                >
                                  <Play className="w-3.5 h-3.5 text-emerald-500" />
                                </Button>
                              )}
                              <Button
                                variant="ghost"
                                size="icon"
                                className="w-7 h-7 cursor-pointer shrink-0"
                                title={ll.mcp.action_remove()}
                                disabled={loading}
                                onClick={() => void removeServer(srv.id)}
                              >
                                <Trash2 className="w-3.5 h-3.5 text-rose-500" />
                              </Button>
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ))}
            <p className="text-[11px] text-muted-foreground px-1">{ll.mcp.restore_hint()}</p>
          </div>
        ) : (
          <div className="p-8 text-center rounded-xl border border-dashed border-border bg-card/50 space-y-2">
            <Activity className="w-8 h-8 text-muted-foreground/40 mx-auto" />
            <div className="text-sm font-medium text-foreground">
              {ll.mcp.empty_instances()}
            </div>
            <p className="text-xs text-muted-foreground max-w-sm mx-auto">
              {ll.mcp.empty_instances_hint()}
            </p>
          </div>
        )}
      </div>

      {/* Snippet preview modal */}
      {selectedSnippetServer && snippets && (
        <Dialog open={!!selectedSnippetServer} onOpenChange={() => setSelectedSnippetServer(null)}>
          <DialogContent className="w-full max-w-[540px]">
            <DialogHeader>
              <DialogTitle className="text-base font-semibold">
                {ll.mcp.client_config_title({ name: selectedSnippetServer.name })}
              </DialogTitle>
              <DialogDescription className="text-xs text-muted-foreground">
                {ll.mcp.client_config_desc()}
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-3 pt-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 bg-muted p-1 rounded-md">
                  <button
                    type="button"
                    onClick={() => setActiveSnippetTab('claude')}
                    className={`px-2.5 py-1 text-xs rounded transition-colors cursor-pointer ${
                      activeSnippetTab === 'claude'
                        ? 'bg-card text-foreground font-medium shadow-sm'
                        : 'text-muted-foreground hover:text-foreground'
                    }`}
                  >
                    {ll.mcp.tab_claude()}
                  </button>
                  <button
                    type="button"
                    onClick={() => setActiveSnippetTab('cursor')}
                    className={`px-2.5 py-1 text-xs rounded transition-colors cursor-pointer ${
                      activeSnippetTab === 'cursor'
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
                      activeSnippetTab === 'claude' ? snippets.claude_desktop : snippets.cursor,
                      null,
                      2,
                    );
                    void handleCopy(text, activeSnippetTab);
                  }}
                >
                  {copiedTab === activeSnippetTab ? (
                    <Check className="w-3.5 h-3.5 text-emerald-500" />
                  ) : (
                    <Copy className="w-3.5 h-3.5" />
                  )}
                  {copiedTab === activeSnippetTab ? ll.mcp.copied() : ll.mcp.copy_json_btn()}
                </Button>
              </div>

              <div className="rounded-lg border border-border bg-muted/40 p-3 font-mono text-xs overflow-x-auto max-h-[300px]">
                <pre className="text-foreground/90 whitespace-pre">
                  {JSON.stringify(
                    activeSnippetTab === 'claude' ? snippets.claude_desktop : snippets.cursor,
                    null,
                    2,
                  )}
                </pre>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}
