/**
 * NewFlowDialog - 新建流程对话框组件
 * 用于创建新的流程或从模板创建（entry 节点由 createFlow 自动创建）
 */
import { useState, useEffect, useMemo } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { getLL } from '@/i18n/i18n-lazy';
import { useUIStore, useFlowStore, useFlowListStore } from '@/stores';
import { createFlow } from '@/services/tauri';
import { flowStorage } from '@/services/storage';
import { FLOW_TEMPLATES, type FlowTemplate, type TemplateCategory } from '@/templates/builtinFlows';
import type { FolderInfo } from '@/services/storage';
import type { Flow } from '@/types/flow';
import { Check, Search } from 'lucide-react';

/** 分类颜色映射（静态） */
const CATEGORY_COLORS: Record<string, string> = {
  all: '',
  http: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  websocket: 'bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200',
  database: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200',
  mqtt: 'bg-teal-100 text-teal-800 dark:bg-teal-900 dark:text-teal-200',
  grpc: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200',
  kafka: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
  amqp: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
  graphql: 'bg-pink-100 text-pink-800 dark:bg-pink-900 dark:text-pink-200',
  tcp: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
  udp: 'bg-lime-100 text-lime-800 dark:bg-lime-900 dark:text-lime-200',
  smtp: 'bg-sky-100 text-sky-800 dark:bg-sky-900 dark:text-sky-200',
  imap: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  sse: 'bg-violet-100 text-violet-800 dark:bg-violet-900 dark:text-violet-200',
  mcp: 'bg-cyan-100 text-cyan-800 dark:bg-cyan-900 dark:text-cyan-200',
  script: 'bg-fuchsia-100 text-fuchsia-800 dark:bg-fuchsia-900 dark:text-fuchsia-200',
  combination: 'bg-rose-100 text-rose-800 dark:bg-rose-900 dark:text-rose-200',
};

const DIFFICULTY_VARIANT: Record<string, 'default' | 'secondary' | 'outline'> = {
  beginner: 'secondary',
  intermediate: 'outline',
  advanced: 'default',
};

export function NewFlowDialog() {
  const LL = getLL();
  const W = LL.welcome;
  const showNewFlowDialog = useUIStore((state) => state.showNewFlowDialog);
  const newFlowDialogFolderPath = useUIStore((state) => state.newFlowDialogFolderPath);
  const newFlowDialogDefaultTemplate = useUIStore((state) => state.newFlowDialogDefaultTemplate);
  const closeNewFlowDialog = useUIStore((state) => state.closeNewFlowDialog);
  const setFlow = useFlowStore((state) => state.setFlow);
  const addFlow = useFlowListStore((state) => state.addFlow);

  const [flowName, setFlowName] = useState('');
  const [selectedFolder, setSelectedFolder] = useState('');
  const [folders, setFolders] = useState<FolderInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<'blank' | 'template'>('blank');
  const [selectedTemplate, setSelectedTemplate] = useState<FlowTemplate | null>(null);
  const [templateCategory, setTemplateCategory] = useState<TemplateCategory | 'all'>('all');
  const [templateSearch, setTemplateSearch] = useState('');

  // 国际化分类标签
  const categories = useMemo(() => [
    { value: 'all' as const, label: W.template_category_all?.() || '全部', color: CATEGORY_COLORS.all },
    { value: 'http' as const, label: W.template_category_http?.() || 'HTTP', color: CATEGORY_COLORS.http },
    { value: 'websocket' as const, label: W.template_category_websocket?.() || 'WebSocket', color: CATEGORY_COLORS.websocket },
    { value: 'database' as const, label: W.template_category_database?.() || '数据库', color: CATEGORY_COLORS.database },
    { value: 'mqtt' as const, label: W.template_category_mqtt?.() || 'MQTT', color: CATEGORY_COLORS.mqtt },
    { value: 'grpc' as const, label: W.template_category_grpc?.() || 'gRPC', color: CATEGORY_COLORS.grpc },
    { value: 'kafka' as const, label: W.template_category_kafka?.() || 'Kafka', color: CATEGORY_COLORS.kafka },
    { value: 'amqp' as const, label: W.template_category_amqp?.() || 'AMQP', color: CATEGORY_COLORS.amqp },
    { value: 'graphql' as const, label: W.template_category_graphql?.() || 'GraphQL', color: CATEGORY_COLORS.graphql },
    { value: 'tcp' as const, label: W.template_category_tcp?.() || 'TCP', color: CATEGORY_COLORS.tcp },
    { value: 'udp' as const, label: W.template_category_udp?.() || 'UDP', color: CATEGORY_COLORS.udp },
    { value: 'smtp' as const, label: W.template_category_smtp?.() || 'SMTP', color: CATEGORY_COLORS.smtp },
    { value: 'imap' as const, label: W.template_category_imap?.() || 'IMAP', color: CATEGORY_COLORS.imap },
    { value: 'sse' as const, label: W.template_category_sse?.() || 'SSE', color: CATEGORY_COLORS.sse },
    { value: 'mcp' as const, label: W.template_category_mcp?.() || 'MCP', color: CATEGORY_COLORS.mcp },
    { value: 'script' as const, label: W.template_category_script?.() || '脚本', color: CATEGORY_COLORS.script },
    { value: 'combination' as const, label: W.template_category_combination?.() || '组合', color: CATEGORY_COLORS.combination },
  ], [W]);

  const difficultyLabel: Record<string, string> = useMemo(() => ({
    beginner: W.template_difficulty_beginner?.() || '入门',
    intermediate: W.template_difficulty_intermediate?.() || '进阶',
    advanced: W.template_difficulty_advanced?.() || '高级',
  }), [W]);

  // 筛选后的模板列表
  const filteredTemplates = FLOW_TEMPLATES.filter((t) => {
    if (templateCategory !== 'all' && t.category !== templateCategory) return false;
    if (templateSearch.trim()) {
      const kw = templateSearch.toLowerCase();
      return t.name.toLowerCase().includes(kw) || t.description.toLowerCase().includes(kw) || t.tags.some((tag) => tag.toLowerCase().includes(kw));
    }
    return true;
  });

  // 对话框打开时加载文件夹列表，并设置预选文件夹
  useEffect(() => {
    if (showNewFlowDialog) {
      setSelectedFolder(newFlowDialogFolderPath);
      flowStorage.listFolders().then(setFolders).catch(() => setFolders([]));
      setActiveTab(newFlowDialogDefaultTemplate ? 'template' : 'blank');
      setSelectedTemplate(null);
      setTemplateCategory('all');
      setTemplateSearch('');
    }
  }, [showNewFlowDialog, newFlowDialogFolderPath]);

  const handleClose = () => {
    setFlowName('');
    setSelectedFolder('');
    setSelectedTemplate(null);
    setTemplateCategory('all');
    setTemplateSearch('');
    closeNewFlowDialog();
  };

  const handleCreate = async () => {
    if (!flowName.trim()) return;

    setIsLoading(true);
    try {
      const newFlow = await createFlow(flowName.trim());

      await flowStorage.saveFlow(newFlow);

      if (selectedFolder) {
        await flowStorage.moveFlowToFolder(newFlow.uuid, selectedFolder);
      }

      setFlow(newFlow);

      addFlow({
        uuid: newFlow.uuid,
        name: newFlow.name,
        updated_at: Date.now(),
        folder_path: selectedFolder,
      });

      handleClose();
    } finally {
      setIsLoading(false);
    }
  };

  const handleUseTemplate = async () => {
    if (!selectedTemplate) return;

    setIsLoading(true);
    try {
      const templateFlow = selectedTemplate.flow;

      // 使用 createFlow 创建基础结构，然后覆盖模板的节点和连接
      const newFlowBase = await createFlow(selectedTemplate.name);
      const templatedFlow: Flow = {
        ...newFlowBase,
        nodes: templateFlow.nodes,
        connections: templateFlow.connections,
        initial_variables: templateFlow.initial_variables,
      };

      await flowStorage.saveFlow(templatedFlow);

      if (selectedFolder) {
        await flowStorage.moveFlowToFolder(templatedFlow.uuid, selectedFolder);
      }

      setFlow(templatedFlow);

      addFlow({
        uuid: templatedFlow.uuid,
        name: templatedFlow.name,
        updated_at: Date.now(),
        folder_path: selectedFolder,
      });

      handleClose();
    } finally {
      setIsLoading(false);
    }
  };

  if (!showNewFlowDialog) return null;

  return (
    <Dialog open={showNewFlowDialog} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent className="w-full max-w-[640px]">
        <DialogHeader>
          <DialogTitle>{LL.flow.new_flow_dialog_title()}</DialogTitle>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={(v) => { setActiveTab(v as 'blank' | 'template'); setSelectedTemplate(null); }}>
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="blank">{W.blank_tab_title?.() || '空白'}</TabsTrigger>
            <TabsTrigger value="template">{W.template_tab_title?.() || '模板'}</TabsTrigger>
          </TabsList>

          {/* ── 空白创建 Tab ── */}
          <TabsContent value="blank" className="py-4 space-y-3">
            <Input
              placeholder={LL.flow.flow_name_placeholder()}
              value={flowName}
              onChange={(e) => setFlowName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
              disabled={isLoading}
              autoFocus
            />
            <div className="space-y-1.5">
              <label htmlFor="folder-select" className="text-sm text-muted-foreground">
                {LL.flow.parent_folder()}
              </label>
              <select
                id="folder-select"
                value={selectedFolder}
                onChange={(e) => setSelectedFolder(e.target.value)}
                disabled={isLoading}
                className="w-full h-9 rounded-md border border-border bg-background px-3 py-1 text-sm transition-colors focus:outline-none focus:ring-1 focus:ring-primary disabled:cursor-not-allowed disabled:opacity-50"
              >
                <option value="">{LL.flow.root_directory()}</option>
                {folders.map(folder => (
                  <option key={folder.path} value={folder.path}>
                    {folder.name}
                  </option>
                ))}
              </select>
            </div>
          </TabsContent>

          {/* ── 从模板创建 Tab ── */}
          <TabsContent value="template" className="py-4 space-y-3">
            {/* 搜索框 */}
            <div className="relative">
              <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder={W.template_search_placeholder?.() || '搜索模板...'}
                value={templateSearch}
                onChange={(e) => setTemplateSearch(e.target.value)}
                className="pl-8"
              />
            </div>

            {/* 分类筛选行 - 自动换行 */}
            <div className="flex flex-wrap gap-1.5">
              {categories.map((cat) => (
                <button
                  key={cat.value}
                  onClick={() => setTemplateCategory(cat.value)}
                  className={`inline-flex items-center rounded-full px-2.5 py-1 text-xs font-medium transition-colors ${
                    templateCategory === cat.value
                      ? 'bg-primary text-primary-foreground'
                      : `${cat.color} hover:opacity-80`
                  }`}
                >
                  {cat.label}
                </button>
              ))}
            </div>

            {/* 模板网格 */}
            {filteredTemplates.length === 0 ? (
              <div className="py-8 text-center text-sm text-muted-foreground">
                {W.template_no_matches?.() || '没有找到匹配的模板'}
              </div>
            ) : (
              <ScrollArea className="h-[320px] rounded-md border">
                <div className="grid grid-cols-2 gap-2 p-3">
                  {filteredTemplates.map((tmpl) => (
                    <button
                      key={tmpl.id}
                      onClick={() => setSelectedTemplate(tmpl)}
                      className={`relative text-left rounded-lg border p-3 transition-all hover:bg-accent ${
                        selectedTemplate?.id === tmpl.id
                          ? 'border-primary ring-1 ring-primary bg-accent'
                          : 'border-border'
                      }`}
                    >
                      {/* 选中标记 */}
                      {selectedTemplate?.id === tmpl.id && (
                        <div className="absolute top-2 right-2 h-5 w-5 rounded-full bg-primary flex items-center justify-center">
                          <Check className="h-3 w-3 text-primary-foreground" />
                        </div>
                      )}

                      {/* 模板名称 */}
                      <div className="font-medium text-sm mb-1 pr-5 truncate">{tmpl.name}</div>

                      {/* 模板描述 */}
                      <div className="text-xs text-muted-foreground line-clamp-2 mb-2 min-h-[2em]">
                        {tmpl.description}
                      </div>

                      {/* 标签行 */}
                      <div className="flex flex-wrap gap-1">
                        <Badge variant={DIFFICULTY_VARIANT[tmpl.difficulty]} className="text-[10px] px-1.5 py-0">
                          {difficultyLabel[tmpl.difficulty]}
                        </Badge>
                        {tmpl.tags.slice(0, 2).map((tag) => (
                          <span key={tag} className="text-[10px] text-muted-foreground bg-muted rounded px-1 py-0">
                            {tag}
                          </span>
                        ))}
                      </div>
                    </button>
                  ))}
                </div>
              </ScrollArea>
            )}
          </TabsContent>
        </Tabs>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={isLoading}>
            {LL.flow.cancel()}
          </Button>

          {activeTab === 'blank' ? (
            <Button onClick={handleCreate} disabled={isLoading || !flowName.trim()}>
              {isLoading ? LL.flow.creating() : LL.flow.create()}
            </Button>
          ) : (
            <Button onClick={handleUseTemplate} disabled={isLoading || !selectedTemplate}>
              {isLoading ? (W.template_creating?.() || '创建中...') : (W.template_use?.() || '使用此模板')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
