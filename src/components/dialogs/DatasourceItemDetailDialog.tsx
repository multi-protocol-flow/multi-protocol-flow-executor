/**
 * DatasourceItemDetailDialog - 数据源条目原始数据与转换详情弹窗
 *
 * 用户在数据源面板中点击某个数据条目时弹出：
 * 1. 原始数据（Raw Data）：完整展示从平台获取的原始 JSON 对象，支持折叠与一键复制
 * 2. 概览信息（Overview）：展示名称、协议、方法、路径、状态、分组、说明与扩展属性
 * 3. 转换预览（Template Preview）：展示转成画布节点后的 NodeTemplate 配置，并支持一键导入到画布
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Copy,
  Check,
  Plus,
  Loader2,
  FileCode,
  Info,
  Layers,
  Globe,
  Wifi,
  RadioTower,
  Cable,
  Braces,
  Send,
  Hash,
  Network,
  type LucideIcon,
} from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { RawJsonViewer } from '@/components/execution/viewers/RawJsonViewer';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { datasourceGetItem, datasourceGetTemplates } from '@/services/tauri/datasource';
import { connectionFor } from '@/stores/datasource';
import { useFlowStore } from '@/stores';
import { createDefaultNodeData, createDefaultNodeName } from '@/components/canvas/utils';
import type {
  DataItem,
  DataItemProtocol,
  DatasourceConnection,
  DatasourceDescription,
  NodeTemplate,
} from '@/types/datasource';
import type { NodeType, NodeData } from '@/types';

/** 协议 → 图标 */
const PROTOCOL_ICON_MAP: Record<DataItemProtocol, LucideIcon> = {
  http: Globe,
  websocket: Wifi,
  sse: RadioTower,
  tcp: Cable,
  graphql: Braces,
  mqtt: Send,
  kafka: Hash,
  other: Network,
};

/** 协议 → 颜色 */
const PROTOCOL_COLOR_MAP: Record<DataItemProtocol, string> = {
  http: 'text-sky-500',
  websocket: 'text-emerald-500',
  sse: 'text-violet-500',
  tcp: 'text-amber-500',
  graphql: 'text-fuchsia-500',
  mqtt: 'text-rose-500',
  kafka: 'text-orange-500',
  other: 'text-slate-500',
};

export interface DatasourceItemDetailDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  datasource: DatasourceDescription | null;
  item: DataItem | null;
  connection?: DatasourceConnection | null;
  resourceId?: string;
}

export function DatasourceItemDetailDialog({
  open,
  onOpenChange,
  datasource,
  item,
  connection,
  resourceId = 'api',
}: DatasourceItemDetailDialogProps) {
  const LL = getLL();
  const [detailItem, setDetailItem] = useState<DataItem | null>(item);
  const [template, setTemplate] = useState<NodeTemplate | null>(null);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);
  const [activeTab, setActiveTab] = useState<'raw' | 'overview' | 'template'>('raw');

  const flow = useFlowStore((s) => s.flow);
  const addNode = useFlowStore((s) => s.addNode);

  const activeConn = useMemo(() => {
    if (connection) return connection;
    if (datasource) return connectionFor(datasource.id);
    return undefined;
  }, [connection, datasource]);

  // 打开弹窗或条目变化时，拉取完整最新详情及转换模板
  useEffect(() => {
    if (!open || !datasource || !item || !activeConn) {
      setDetailItem(null);
      setTemplate(null);
      return;
    }

    setDetailItem(item);
    setActiveTab('raw');
    let cancelled = false;
    setLoading(true);

    Promise.allSettled([
      datasourceGetItem(datasource.id, resourceId, item.id, activeConn.config),
      datasourceGetTemplates(datasource.id, [item.id], activeConn.config),
    ]).then(([itemRes, tplRes]) => {
      if (cancelled) return;
      if (itemRes.status === 'fulfilled') {
        setDetailItem(itemRes.value);
      }
      if (tplRes.status === 'fulfilled' && tplRes.value.length > 0) {
        setTemplate(tplRes.value[0]);
      }
      setLoading(false);
    });

    return () => {
      cancelled = true;
    };
  }, [open, datasource, item, activeConn, resourceId]);

  const rawData = detailItem?.raw ?? detailItem;

  // 复制 JSON 数据到剪贴板
  const handleCopyJson = useCallback(async () => {
    if (!rawData) return;
    try {
      const jsonStr = typeof rawData === 'string' ? rawData : JSON.stringify(rawData, null, 2);
      await navigator.clipboard.writeText(jsonStr);
      setCopied(true);
      toast.success(LL.datasource.copy_success());
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error(LL.datasource.copy_failed());
    }
  }, [rawData, LL]);

  // 添加转换后的节点到画布
  const handleAddToCanvas = useCallback(() => {
    if (!template || !flow) return;
    try {
      const defaultData = createDefaultNodeData(template.type_id as NodeType);
      const nodeData = {
        ...defaultData,
        ...template.data,
        type: template.type_id,
      };
      const nodeCount = flow.nodes.length + 1;
      const name = template.name || createDefaultNodeName(template.type_id as NodeType, nodeCount);

      // 计算画布中心偏置位置
      const baseX = flow.nodes.length > 0
        ? Math.max(...flow.nodes.map((n) => n.position.x)) + 280
        : 200;
      const baseY = flow.nodes.length > 0
        ? flow.nodes[flow.nodes.length - 1].position.y
        : 200;

      addNode({
        uuid: crypto.randomUUID(),
        type: template.type_id as NodeType,
        name,
        position: { x: baseX, y: baseY },
        data: nodeData as unknown as NodeData,
      });

      toast.success(LL.datasource.add_success());
      onOpenChange(false);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, [template, flow, addNode, LL, onOpenChange]);

  if (!item || !datasource) return null;

  const Icon = PROTOCOL_ICON_MAP[item.protocol] ?? Network;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[85vh] flex flex-col p-0 gap-0 overflow-hidden sm:rounded-lg">
        {/* 头部标题与元信息 */}
        <DialogHeader className="p-4 pb-3 border-b border-border bg-muted/30">
          <div className="flex items-start justify-between gap-3 pr-6">
            <div className="flex items-start gap-2.5 min-w-0">
              <div className="p-1.5 rounded-md bg-background border border-border shrink-0 mt-0.5">
                <Icon className={cn('w-5 h-5', PROTOCOL_COLOR_MAP[item.protocol] ?? 'text-slate-500')} />
              </div>
              <div className="min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <DialogTitle className="text-base font-semibold text-foreground truncate">
                    {detailItem?.name || item.name}
                  </DialogTitle>
                  {item.method && (
                    <Badge variant="secondary" className="h-5 px-1.5 text-xs font-mono font-bold">
                      {item.method.toUpperCase()}
                    </Badge>
                  )}
                  <Badge variant="outline" className="text-[11px] text-muted-foreground">
                    {datasource.display_name}
                    {activeConn?.name ? ` · ${activeConn.name}` : ''}
                  </Badge>
                </div>
                <DialogDescription className="text-xs font-mono text-muted-foreground mt-1 truncate">
                  {detailItem?.path || item.path || `ID: ${item.id}`}
                </DialogDescription>
              </div>
            </div>
            {loading && (
              <div className="flex items-center gap-1.5 text-xs text-muted-foreground shrink-0 bg-background/80 px-2 py-1 rounded border border-border">
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>{LL.datasource.loading_detail()}</span>
              </div>
            )}
          </div>
        </DialogHeader>

        {/* 选项卡主体 */}
        <Tabs
          value={activeTab}
          onValueChange={(v) => setActiveTab(v as 'raw' | 'overview' | 'template')}
          className="flex-1 flex flex-col min-h-0"
        >
          {/* Tab 导航与操作栏 */}
          <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-muted/10">
            <TabsList className="h-8 p-0.5 bg-muted/60 border border-border/40 rounded-lg">
              <TabsTrigger value="raw" className="text-xs gap-1.5 px-3 h-7 rounded-md">
                <FileCode className="w-3.5 h-3.5" />
                {LL.datasource.raw_data()}
              </TabsTrigger>
              <TabsTrigger value="overview" className="text-xs gap-1.5 px-3 h-7 rounded-md">
                <Info className="w-3.5 h-3.5" />
                {LL.datasource.overview()}
              </TabsTrigger>
              <TabsTrigger value="template" className="text-xs gap-1.5 px-3 h-7 rounded-md">
                <Layers className="w-3.5 h-3.5" />
                {LL.datasource.template_preview()}
              </TabsTrigger>
            </TabsList>

            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-7 px-2 text-xs gap-1.5"
                onClick={handleCopyJson}
                title={LL.datasource.copy_json()}
              >
                {copied ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5" />}
                {copied ? LL.datasource.copy_success() : LL.datasource.copy_json()}
              </Button>
              {template && flow && (
                <Button
                  size="sm"
                  className="h-7 px-2.5 text-xs gap-1.5 bg-primary text-primary-foreground hover:bg-primary/90"
                  onClick={handleAddToCanvas}
                >
                  <Plus className="w-3.5 h-3.5" />
                  {LL.datasource.add_to_canvas()}
                </Button>
              )}
            </div>
          </div>

          {/* Tab 1: 原始数据 */}
          <TabsContent value="raw" className="flex-1 p-4 m-0 min-h-0 overflow-y-auto scrollbar-thin bg-background">
            <div className="p-3 rounded-lg border border-border bg-muted/15">
              <RawJsonViewer data={rawData} />
            </div>
          </TabsContent>

          {/* Tab 2: 概览信息 */}
          <TabsContent value="overview" className="flex-1 p-4 m-0 min-h-0 overflow-y-auto scrollbar-thin space-y-4">
            {/* 基本字段表 */}
            <div className="rounded-lg border border-border overflow-hidden">
              <table className="w-full text-xs text-left">
                <tbody>
                  <tr className="border-b border-border bg-muted/20">
                    <td className="w-28 px-3 py-2 font-medium text-muted-foreground">{LL.datasource.item_id()}</td>
                    <td className="px-3 py-2 font-mono text-foreground select-all">{item.id}</td>
                  </tr>
                  <tr className="border-b border-border">
                    <td className="px-3 py-2 font-medium text-muted-foreground">{LL.datasource.item_name()}</td>
                    <td className="px-3 py-2 text-foreground font-medium">{detailItem?.name || item.name}</td>
                  </tr>
                  <tr className="border-b border-border bg-muted/20">
                    <td className="px-3 py-2 font-medium text-muted-foreground">{LL.datasource.method()}</td>
                    <td className="px-3 py-2">
                      <Badge variant="secondary" className="font-mono text-[11px]">
                        {detailItem?.method?.toUpperCase() || item.method?.toUpperCase() || 'N/A'}
                      </Badge>
                    </td>
                  </tr>
                  <tr className="border-b border-border">
                    <td className="px-3 py-2 font-medium text-muted-foreground">{LL.datasource.path()}</td>
                    <td className="px-3 py-2 font-mono text-foreground select-all">
                      {detailItem?.path || item.path || 'N/A'}
                    </td>
                  </tr>
                  <tr className="border-b border-border bg-muted/20">
                    <td className="px-3 py-2 font-medium text-muted-foreground">{LL.datasource.source_datasource()}</td>
                    <td className="px-3 py-2 text-foreground">{datasource.display_name} ({datasource.id})</td>
                  </tr>
                  {detailItem?.summary && (
                    <tr className="border-b border-border">
                      <td className="px-3 py-2 font-medium text-muted-foreground">{LL.datasource.description()}</td>
                      <td className="px-3 py-2 text-foreground leading-relaxed">{detailItem.summary}</td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>

            {/* 扩展属性 */}
            {(detailItem?.extra ?? item.extra).length > 0 && (
              <div className="space-y-1.5">
                <span className="text-xs font-semibold text-muted-foreground">{LL.datasource.extra_properties()}</span>
                <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
                  {(detailItem?.extra ?? item.extra).map((ext, idx) => {
                    const localizedLabel =
                      ext.label === '分组' || ext.label === 'Group' || ext.label === 'group'
                        ? LL.datasource.group()
                        : ext.label === '规范版本' || ext.label === 'Spec Version'
                        ? LL.datasource.spec_version()
                        : ext.label === '已废弃' || ext.label === 'Deprecated'
                        ? LL.datasource.deprecated()
                        : ext.label;

                    const localizedValue =
                      ext.value === '是' || ext.value === 'Yes'
                        ? LL.datasource.yes()
                        : ext.value === '否' || ext.value === 'No'
                        ? LL.datasource.no()
                        : ext.value;

                    return (
                      <div key={idx} className="p-2 rounded-md border border-border bg-muted/20 text-xs">
                        <div className="text-[10px] text-muted-foreground">{localizedLabel}</div>
                        <div className="font-medium text-foreground mt-0.5 truncate" title={localizedValue}>
                          {localizedValue}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </TabsContent>

          {/* Tab 3: 转换节点模板预览 */}
          <TabsContent value="template" className="flex-1 p-4 m-0 min-h-0 overflow-y-auto scrollbar-thin space-y-3">
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>{LL.datasource.node_template_label()}</span>
              {template && (
                <span className="font-mono text-foreground">
                  type_id: <strong>{template.type_id}</strong>
                </span>
              )}
            </div>
            <div className="p-3 rounded-lg border border-border bg-muted/15">
              {template ? (
                <RawJsonViewer data={template} />
              ) : (
                <div className="flex items-center justify-center p-8 text-xs text-muted-foreground">
                  {loading ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
                  {loading ? LL.datasource.loading_detail() : LL.datasource.empty()}
                </div>
              )}
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
