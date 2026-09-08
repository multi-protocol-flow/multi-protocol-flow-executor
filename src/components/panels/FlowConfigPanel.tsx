/**
 * FlowConfigPanel - 流程属性面板
 * 当画布未选中任何节点时展示，提供流程级元数据与全局变量的检查与配置
 */
import { useState, useCallback, useEffect, useId } from 'react';
import {
  Pencil,
  Eye,
  Plus,
  Trash2,
  Copy,
  Check,
  Settings2,
  Workflow,
  Layers,
  Network,
  Variable as VariableIcon,
} from 'lucide-react';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';
import { useFlowStore, useUIStore } from '@/stores';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';

interface FlowConfigPanelProps {
  /** 只读模式（调试或运行中） */
  readOnly?: boolean;
}

export function FlowConfigPanel({ readOnly = false }: FlowConfigPanelProps) {
  const flowLl = getLL().flow;
  const flow = useFlowStore((s) => s.flow);
  const updateFlowName = useFlowStore((s) => s.updateFlowName);
  const updateFlowDescription = useFlowStore((s) => s.updateFlowDescription);
  const updateInitialVariables = useFlowStore((s) => s.updateInitialVariables);
  const openEnvironmentManagerDialog = useUIStore((s) => s.openEnvironmentManagerDialog);

  const nameInputId = useId();

  // 本地名称状态（失焦或回车保存）
  const [nameInput, setNameInput] = useState(flow?.name ?? '');
  useEffect(() => {
    setNameInput(flow?.name ?? '');
  }, [flow?.name]);

  const handleNameSave = useCallback(() => {
    if (!flow) return;
    const trimmed = nameInput.trim();
    if (trimmed && trimmed !== flow.name) {
      updateFlowName(trimmed);
    } else {
      setNameInput(flow.name);
    }
  }, [flow, nameInput, updateFlowName]);

  // 描述编辑/预览模式
  const [descMode, setDescMode] = useState<'edit' | 'preview'>('edit');

  // 新增变量输入状态
  const [newVarKey, setNewVarKey] = useState('');
  const [newVarValue, setNewVarValue] = useState('');
  const [isAddingVar, setIsAddingVar] = useState(false);

  // 复制 Flow ID 状态
  const [copiedId, setCopiedId] = useState(false);
  const handleCopyUuid = useCallback(() => {
    if (!flow?.uuid) return;
    navigator.clipboard.writeText(flow.uuid).then(() => {
      setCopiedId(true);
      toast.success(`${flowLl.flow_uuid()}: ${flow.uuid}`);
      setTimeout(() => setCopiedId(false), 1500);
    });
  }, [flow?.uuid, flowLl]);

  // 添加初始变量
  const handleAddVariable = useCallback(() => {
    if (!flow) return;
    const key = newVarKey.trim();
    if (!key) return;

    let parsedValue: unknown = newVarValue;
    try {
      parsedValue = JSON.parse(newVarValue);
    } catch {
      // 保持原始字符串
    }

    const updated = { ...flow.initial_variables, [key]: parsedValue };
    updateInitialVariables(updated);
    setNewVarKey('');
    setNewVarValue('');
    setIsAddingVar(false);
  }, [flow, newVarKey, newVarValue, updateInitialVariables]);

  // 删除初始变量
  const handleRemoveVariable = useCallback(
    (keyToRemove: string) => {
      if (!flow?.initial_variables) return;
      const { [keyToRemove]: _, ...rest } = flow.initial_variables;
      updateInitialVariables(rest);
    },
    [flow, updateInitialVariables]
  );

  // 空状态
  if (!flow) {
    return (
      <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none w-full min-w-0">
        <CardContent className="flex flex-1 items-center justify-center p-6 text-center">
          <p className="text-sm text-muted-foreground">{flowLl.no_flow_title()}</p>
        </CardContent>
      </Card>
    );
  }

  const initialVars = flow.initial_variables ?? {};
  const varEntries = Object.entries(initialVars);

  return (
    <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none w-full min-w-0">
      {/* 顶部标题栏：与 NodeConfigPanel 完全一致的尺寸、间距与排版 */}
      <CardHeader className="shrink-0 border-b border-muted bg-background pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center flex-1 min-w-0">
            <CardTitle
              className="text-base font-semibold text-foreground truncate cursor-default"
              title={flowLl.flow_properties()}
            >
              {flowLl.flow_properties()}
            </CardTitle>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <span className="inline-flex items-center gap-1 text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-md font-mono">
              <Layers className="h-3.5 w-3.5" />
              {flow.nodes.length} {flowLl.node_count_label()}
            </span>
            {readOnly && (
              <span className="inline-flex items-center rounded-md border border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-950/50 px-2 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-300">
                {getLL().panels.nodeConfig.readOnly()}
              </span>
            )}
          </div>
        </div>
      </CardHeader>

      {/* 内容区域：与 NodeConfigPanel 保持相同的 px-6 py-4 内边距 */}
      <CardContent className="flex-1 overflow-hidden p-0 relative">
        <ScrollArea className="h-full">
          <div className="w-full min-w-0 max-w-full overflow-hidden px-6 py-4 space-y-4">
            <div className="space-y-3 min-w-0">
              <div className="min-w-0">
                <Label
                  htmlFor={nameInputId}
                  className="text-xs font-medium text-muted-foreground block mb-1.5"
                >
                  {flowLl.flow_name()}
                </Label>
                {readOnly ? (
                  <div className="text-xs font-medium px-2.5 py-1.5 rounded-md bg-muted/40 border border-muted text-foreground truncate">
                    {flow.name}
                  </div>
                ) : (
                  <Input
                    id={nameInputId}
                    value={nameInput}
                    onChange={(e) => setNameInput(e.target.value)}
                    onBlur={handleNameSave}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.currentTarget.blur();
                      }
                    }}
                    placeholder={flowLl.flow_name_placeholder()}
                    className="h-8 text-xs w-full"
                  />
                )}
              </div>

              {/* 流程描述 */}
              <div className="min-w-0">
                <div className="flex items-center justify-between mb-1 gap-2">
                  <Label className="text-xs font-medium text-muted-foreground shrink-0">
                    {flowLl.flow_info_description()}
                  </Label>
                  {!readOnly && (
                    <button
                      type="button"
                      onClick={() => setDescMode(descMode === 'edit' ? 'preview' : 'edit')}
                      className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer shrink-0"
                    >
                      {descMode === 'edit' ? (
                        <>
                          <Eye className="h-3 w-3" />
                          {flowLl.flow_info_preview()}
                        </>
                      ) : (
                        <>
                          <Pencil className="h-3 w-3" />
                          {flowLl.flow_info_edit()}
                        </>
                      )}
                    </button>
                  )}
                </div>

                {descMode === 'edit' && !readOnly ? (
                  <Textarea
                    value={flow.description ?? ''}
                    onChange={(e) => updateFlowDescription(e.target.value)}
                    placeholder={flowLl.flow_info_description_placeholder()}
                    className="min-h-[80px] text-xs resize-y font-normal w-full"
                  />
                ) : (
                  <div className="min-h-[80px] max-h-56 overflow-y-auto w-full min-w-0 rounded-md border border-input bg-background px-2.5 py-2 break-words">
                    {flow.description ? (
                      <MarkdownViewer content={flow.description} compact />
                    ) : (
                      <p className="text-xs text-muted-foreground italic">
                        {flowLl.flow_info_description_placeholder()}
                      </p>
                    )}
                  </div>
                )}
              </div>
            </div>

            <div className="border-t border-border/60" />

            {/* 2. 流程初始变量 */}
            <div className="space-y-2 min-w-0">
              <div className="flex items-center justify-between gap-2 min-w-0">
                <div className="flex items-center gap-1.5 min-w-0 flex-1">
                  <VariableIcon className="h-3.5 w-3.5 text-violet-500 shrink-0" />
                  <span className="text-xs font-medium text-foreground truncate">
                    {flowLl.flow_variables()}
                  </span>
                  <span className="text-[10px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded font-mono shrink-0">
                    {varEntries.length}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => openEnvironmentManagerDialog?.()}
                  className="h-6 px-1.5 text-xs text-muted-foreground hover:text-foreground gap-1 shrink-0"
                  title={flowLl.manage_environments_btn()}
                >
                  <Settings2 className="h-3.5 w-3.5 shrink-0" />
                  <span className="text-[11px]">{flowLl.manage_environments_btn()}</span>
                </Button>
              </div>
              {/* 添加变量内联表单：采用上下堆叠结构，避免窄屏下两个 Input 挤压变形 */}
              {isAddingVar && !readOnly && (
                <div className="p-2 rounded-lg border border-violet-200 dark:border-violet-800 bg-violet-50/40 dark:bg-violet-950/20 space-y-1.5 min-w-0">
                  <Input
                    value={newVarKey}
                    onChange={(e) => setNewVarKey(e.target.value)}
                    placeholder={flowLl.var_name_placeholder()}
                    className="h-7 text-xs font-mono w-full"
                    autoFocus
                  />
                  <Input
                    value={newVarValue}
                    onChange={(e) => setNewVarValue(e.target.value)}
                    placeholder={flowLl.var_value_placeholder()}
                    className="h-7 text-xs font-mono w-full"
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') handleAddVariable();
                    }}
                  />
                  <div className="flex justify-end gap-1.5 pt-0.5">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        setIsAddingVar(false);
                        setNewVarKey('');
                        setNewVarValue('');
                      }}
                      className="h-6 px-2 text-xs"
                    >
                      {getLL().common.CANCEL()}
                    </Button>
                    <Button
                      size="sm"
                      onClick={handleAddVariable}
                      disabled={!newVarKey.trim()}
                      className="h-6 px-2.5 text-xs"
                    >
                      {flowLl.add_variable_btn()}
                    </Button>
                  </div>
                </div>
              )}

              {/* 变量列表 */}
              {varEntries.length === 0 ? (
                <div className="p-3 text-center rounded-md border border-dashed border-border bg-muted/20 space-y-2">
                  <p className="text-xs text-muted-foreground">{flowLl.flow_variables_empty()}</p>
                  {!readOnly && !isAddingVar && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setIsAddingVar(true)}
                      className="h-7 text-xs gap-1"
                      title={flowLl.add_variable_btn()}
                    >
                      <Plus className="h-3.5 w-3.5" />
                      {flowLl.add_variable_btn()}
                    </Button>
                  )}
                </div>
              ) : (
                <div className="space-y-1.5 min-w-0">
                  <div className="space-y-1 min-w-0">
                    {varEntries.map(([key, val]) => (
                      <div
                        key={key}
                        className="flex items-center justify-between p-1.5 rounded-md border border-border bg-muted/10 hover:bg-muted/30 transition-colors text-xs font-mono group min-w-0"
                      >
                        <div className="flex items-center gap-1.5 min-w-0 flex-1 mr-1.5 overflow-hidden">
                          <span
                            className="font-semibold text-foreground truncate shrink-0 max-w-[45%] text-[11px]"
                            title={key}
                          >
                            {key}:
                          </span>
                          <span
                            className="text-muted-foreground truncate flex-1 min-w-0 text-[11px]"
                            title={String(val)}
                          >
                            {typeof val === 'string' ? `"${val}"` : JSON.stringify(val)}
                          </span>
                        </div>
                        {!readOnly && (
                          <button
                            type="button"
                            onClick={() => handleRemoveVariable(key)}
                            className="text-muted-foreground hover:text-destructive p-0.5 rounded cursor-pointer shrink-0 transition-colors"
                            title={getLL().common.DELETE()}
                          >
                            <Trash2 className="h-3 w-3" />
                          </button>
                        )}
                      </div>
                    ))}
                  </div>

                  {!readOnly && !isAddingVar && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setIsAddingVar(true)}
                      className="w-full h-7 text-xs border-dashed text-muted-foreground hover:text-foreground gap-1"
                      title={flowLl.add_variable_btn()}
                    >
                      <Plus className="h-3.5 w-3.5" />
                      {flowLl.add_variable_btn()}
                    </Button>
                  )}
                </div>
              )}
            </div>

            <div className="border-t border-border/60" />

            {/* 3. 流程概况 */}
            <div className="space-y-2 min-w-0">
              <span className="text-xs font-medium text-muted-foreground block">
                {flowLl.flow_summary()}
              </span>

              <div className="rounded-lg border border-border bg-muted/10 p-2.5 space-y-2 text-xs min-w-0">
                {/* 节点 & 连线统计 */}
                <div className="flex items-center justify-between gap-2 text-xs">
                  <div className="flex items-center gap-1.5 min-w-0">
                    <Workflow className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <span className="text-muted-foreground truncate">{flowLl.node_count_label()}:</span>
                    <span className="font-medium text-foreground shrink-0">{flow.nodes.length}</span>
                  </div>
                  <div className="flex items-center gap-1.5 min-w-0">
                    <Network className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <span className="text-muted-foreground truncate">{flowLl.connection_count_label()}:</span>
                    <span className="font-medium text-foreground shrink-0">{flow.connections.length}</span>
                  </div>
                </div>

                {/* Flow UUID：文字截断并靠右对齐 */}
                <div className="flex items-center justify-between gap-1.5 pt-1.5 border-t border-border/40 min-w-0">
                  <span className="text-muted-foreground shrink-0">{flowLl.flow_uuid()}:</span>
                  <div className="flex items-center gap-1 min-w-0 flex-1 justify-end">
                    <code
                      className="text-[10px] font-mono text-muted-foreground truncate"
                      title={flow.uuid}
                    >
                      {flow.uuid}
                    </code>
                    <button
                      type="button"
                      onClick={handleCopyUuid}
                      className="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer shrink-0"
                      title={flowLl.copy_details()}
                    >
                      {copiedId ? (
                        <Check className="h-3 w-3 text-green-500" />
                      ) : (
                        <Copy className="h-3 w-3" />
                      )}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
