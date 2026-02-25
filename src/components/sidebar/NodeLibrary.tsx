/**
 * NodeLibrary - 节点库面板
 * 显示可用的节点类型，支持点击添加或拖拽到画布
 */
import { Globe, GitBranch, GripVertical, Sparkles } from 'lucide-react';
import { useFlowStore } from '@/stores';
import { cn } from '@/lib/utils';

/** 节点类型定义 */
const NODE_TYPES = [
  {
    type: 'http',
    name: 'HTTP 请求',
    description: '发送 HTTP 请求',
    icon: Globe,
    color: 'bg-sky-500',
    colorHover: 'hover:bg-sky-400',
    iconColor: 'text-white',
  },
  {
    type: 'conditional',
    name: '条件判断',
    description: '根据条件分支',
    icon: GitBranch,
    color: 'bg-amber-500',
    colorHover: 'hover:bg-amber-400',
    iconColor: 'text-white',
  },
] as const;

export function NodeLibrary() {
  const { addNode, flow } = useFlowStore();

  const handleDragStart = (e: React.DragEvent, type: string) => {
    e.dataTransfer.setData('application/reactflow/nodeType', type);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleAddNode = (type: string) => {
    if (!flow) return;

    // 计算节点位置，避免重叠
    const existingNodes = flow.nodes.length;
    const baseX = 250 + existingNodes * 30;
    const baseY = 150 + existingNodes * 30;

    if (type === 'http') {
      addNode({
        uuid: crypto.randomUUID(),
        type: 'http',
        name: `HTTP 请求 ${existingNodes + 1}`,
        position: { x: baseX, y: baseY },
        data: {
          type: 'http',
          url: '',
          method: 'get',
          headers: {},
          body: '',
        },
      });
    } else if (type === 'conditional') {
      addNode({
        uuid: crypto.randomUUID(),
        type: 'conditional',
        name: `条件判断 ${existingNodes + 1}`,
        position: { x: baseX, y: baseY },
        data: {
          type: 'conditional',
          condition_type: 'simple',
          field_path: '',
          operator: 'equal',
          expected_value: '',
        },
      });
    }
  };

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* 标题栏 */}
      <div className="px-3 py-2 border-b border-border/30">
        <span className="text-sm font-medium text-foreground/90">节点库</span>
      </div>

      {/* 节点列表 */}
      <div className="p-2 space-y-1.5 flex-1 overflow-auto">
        {NODE_TYPES.map((nodeType) => (
          <button
            type="button"
            key={nodeType.type}
            onClick={() => handleAddNode(nodeType.type)}
            draggable
            onDragStart={(e) => handleDragStart(e, nodeType.type)}
            className={cn(
              'w-full flex items-center gap-3 px-3 py-2.5 rounded-xl border border-border/40',
              'hover:bg-muted/50 hover:border-border/60 transition-all duration-150 cursor-pointer',
              'group text-left'
            )}
          >
            {/* 节点图标 */}
            <div
              className={cn(
                'w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors',
                nodeType.color,
                nodeType.colorHover
              )}
            >
              <nodeType.icon className={cn('w-4.5 h-4.5', nodeType.iconColor)} />
            </div>

            {/* 节点信息 */}
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-foreground/90 truncate">
                {nodeType.name}
              </p>
              <p className="text-xs text-muted-foreground truncate">
                {nodeType.description}
              </p>
            </div>

            {/* 拖拽手柄 */}
            <GripVertical
              className={cn(
                'w-4 h-4 text-muted-foreground/50 shrink-0',
                'opacity-0 group-hover:opacity-100 transition-opacity'
              )}
            />
          </button>
        ))}
      </div>

      {/* 底部提示 */}
      <div className="px-3 py-2.5 border-t border-border/30 bg-muted/20">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Sparkles className="w-3.5 h-3.5 text-amber-500" />
          <span>点击添加节点或拖拽到画布</span>
        </div>
      </div>
    </div>
  );
}
