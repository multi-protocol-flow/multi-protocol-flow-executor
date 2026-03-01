/**
 * 节点配置面板主组件
 * 根据节点类型显示不同的配置表单
 */
import { useState, useRef, useEffect } from 'react';
import { useFlowStore } from '@/stores';
import { EntryConfig } from './EntryConfig';
import { HttpConfig } from './HttpConfig';
import { ConditionalConfig } from './ConditionalConfig';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { NodeData } from '@/types';
import { isHttpConfig, isConditionalConfig } from '@/types';
import { Settings2, MousePointerClick, Trash2, Pencil } from 'lucide-react';

/**
 * 节点类型配置映射
 */
const NODE_TYPE_CONFIG = {
  entry: {
    label: '入口',
    badgeVariant: 'default' as const,
    badgeClass: 'bg-emerald-50 text-emerald-700',
  },
  http: {
    label: 'HTTP',
    badgeVariant: 'secondary' as const,
    badgeClass: 'bg-sky-50 text-sky-700',
  },
  conditional: {
    label: '条件',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-violet-200 text-violet-700',
  },
};

export function NodeConfigPanel() {
  const { getSelectedNode, updateNode, removeNode, selectNode } = useFlowStore();
  const selectedNode = getSelectedNode();
  
  // 节点名称编辑状态
  const [isEditingName, setIsEditingName] = useState(false);
  const [editingName, setEditingName] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  // 当选中节点变化时，重置编辑状态
  useEffect(() => {
    setIsEditingName(false);
  }, [selectedNode?.uuid]);

  // 未选中节点时的空状态
  if (!selectedNode) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-6 text-center">
        <div className="rounded-full bg-slate-100 p-4 mb-4">
          <MousePointerClick className="h-8 w-8 text-slate-400" />
        </div>
        <h3 className="text-lg font-medium text-black">未选中节点</h3>
        <p className="mt-2 text-sm text-slate-400">
          请在画布上点击选择一个节点进行配置
        </p>
      </div>
    );
  }

  const typeConfig = NODE_TYPE_CONFIG[selectedNode.type];

  /**
   * 处理配置保存
   */
  const handleSave = (data: NodeData) => {
    updateNode(selectedNode.uuid, { data });
  };

  /**
   * 处理删除节点
   */
  /**
   * 处理删除节点
   */
  const handleDelete = () => {
    removeNode(selectedNode.uuid);
    selectNode(null);
  };

  /**
   * 开始编辑节点名称
   */
  const handleStartEdit = () => {
    setEditingName(selectedNode.name);
    setIsEditingName(true);
  };

  /**
   * 保存节点名称
   */
  const handleSaveName = () => {
    const trimmedName = editingName.trim();
    if (trimmedName && trimmedName !== selectedNode.name) {
      updateNode(selectedNode.uuid, { name: trimmedName });
    }
    setIsEditingName(false);
  };

  /**
   * 取消编辑节点名称
   */
  const handleCancelEdit = () => {
    setIsEditingName(false);
    setEditingName(selectedNode.name);
  };

  /**
   * 处理键盘事件
   */
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSaveName();
    } else if (e.key === 'Escape') {
      handleCancelEdit();
    }
  };

  return (
    <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none w-full min-w-0">
      <CardHeader className="shrink-0 border-b border-muted bg-white pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 flex-1 min-w-0">
            <Settings2 className="h-4 w-4 text-slate-400 shrink-0" />
            {isEditingName ? (
              <input
                ref={inputRef}
                type="text"
                value={editingName}
                onChange={(e) => setEditingName(e.target.value)}
                onBlur={handleSaveName}
                onKeyDown={handleKeyDown}
                className="text-base font-semibold text-black bg-white border-b-2 border-black outline-none flex-1 min-w-0 px-0.5"
                autoFocus
              />
            ) : (
              <>
                <CardTitle className="text-base font-semibold text-black truncate cursor-default">
                  {selectedNode.name}
                </CardTitle>
                <button
                  onClick={handleStartEdit}
                  className="shrink-0 p-1 rounded hover:bg-slate-100 text-slate-400 hover:text-slate-600 transition-colors"
                  title="编辑节点名称"
                >
                  <Pencil className="h-3.5 w-3.5" />
                </button>
              </>
            )}
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <Badge 
              variant={typeConfig.badgeVariant}
              className={typeConfig.badgeClass}
            >
              {typeConfig.label}
            </Badge>
            {selectedNode.type !== 'entry' && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-slate-400 hover:text-red-500 hover:bg-red-50"
                onClick={handleDelete}
                title="删除节点"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      
      <CardContent className="flex-1 overflow-hidden p-0">
        <ScrollArea className="h-full">
          <div className="w-full min-w-0 max-w-full overflow-hidden px-6 py-4">
            {/* Entry 节点：仅显示说明 */}
            {selectedNode.type === 'entry' && <EntryConfig />}

            {/* HTTP 节点：URL/方法/Headers/Body 配置 */}
            {selectedNode.type === 'http' && isHttpConfig(selectedNode.data) && (
              <HttpConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* Conditional 节点：条件配置 */}
            {selectedNode.type === 'conditional' && isConditionalConfig(selectedNode.data) && (
              <ConditionalConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
