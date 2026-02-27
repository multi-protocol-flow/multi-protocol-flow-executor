/**
 * 节点配置面板主组件
 * 根据节点类型显示不同的配置表单
 */
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
import { Settings2, MousePointerClick, Trash2 } from 'lucide-react';

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
  const handleDelete = () => {
    removeNode(selectedNode.uuid);
    selectNode(null);
  };

  return (
    <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none">
      <CardHeader className="shrink-0 border-b border-muted bg-white pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Settings2 className="h-4 w-4 text-slate-400" />
            <CardTitle className="text-base font-semibold text-black">
              {selectedNode.name}
            </CardTitle>
          </div>
          <div className="flex items-center gap-2">
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
        <ScrollArea className="h-full px-6 py-4">
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
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
