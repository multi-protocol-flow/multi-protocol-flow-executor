/**
 * NodeLibrary - 节点库面板
 * 显示可用的节点类型，支持点击添加或拖拽到画布
 */
import { Globe, GitBranch, GripVertical, Sparkles, Wifi, Send, Plug } from 'lucide-react';
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
  {
    type: 'ws_connect',
    name: 'WS 连接',
    description: '建立 WebSocket 连接',
    icon: Wifi,
    color: 'bg-amber-500',
    colorHover: 'hover:bg-amber-400',
    iconColor: 'text-white',
  },
  {
    type: 'ws_send_wait',
    name: 'WS 发送等待',
    description: '发送 WebSocket 消息并等待响应',
    icon: Send,
    color: 'bg-teal-500',
    colorHover: 'hover:bg-teal-400',
    iconColor: 'text-white',
  },
  {
    type: 'ws_close',
    name: 'WS 关闭',
    description: '关闭 WebSocket 连接',
    icon: Plug,
    color: 'bg-rose-500',
    colorHover: 'hover:bg-rose-400',
    iconColor: 'text-white',
  },
] as const;

export function NodeLibrary() {
  const { setPendingAddNodeType, flow } = useFlowStore();

  const handleDragStart = (e: React.DragEvent, type: string) => {
    e.dataTransfer.setData('application/reactflow/nodeType', type);
    e.dataTransfer.effectAllowed = 'move';
  };

  // 点击添加节点 - 设置待添加类型，由 FlowCanvas 在视口中心处理
  const handleAddNode = (type: string) => {
    if (!flow) return;
    setPendingAddNodeType(type);
  };

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* 标题栏 */}
      <div className="px-3 py-2 border-b border-border">
        <span className="text-sm font-medium text-black">节点库</span>
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
              'w-full flex items-center gap-3 px-3 h-14 rounded-lg',
              'bg-white border border-black',
              'hover:bg-slate-50 transition-all duration-150 cursor-pointer',
              'group text-left'
            )}
          >
            {/* 节点图标 */}
            <div
              className={cn(
                'w-9 h-9 rounded-lg flex items-center justify-center shrink-0',
                'bg-white border border-black'
              )}
            >
              <nodeType.icon className="w-5 h-5 text-black" />
            </div>

            {/* 节点信息 */}
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-black truncate">
                {nodeType.name}
              </p>
              <p className="text-xs text-slate-500 truncate">
                {nodeType.description}
              </p>
            </div>

            {/* 拖拽手柄 */}
            <GripVertical
              className={cn(
                'w-4 h-4 text-slate-400 shrink-0',
                'opacity-0 group-hover:opacity-100 transition-opacity'
              )}
            />
          </button>
        ))}
      </div>

      {/* 底部提示 */}
      <div className="px-3 py-2.5 border-t border-border bg-slate-50">
        <div className="flex items-center gap-2 text-xs text-slate-400">
          <Sparkles className="w-3.5 h-3.5 text-amber-500" />
          <span>点击添加节点或拖拽到画布</span>
        </div>
      </div>
    </div>
  );
}
