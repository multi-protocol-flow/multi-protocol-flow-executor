/**
 * 动态插件节点组件 - 未知节点类型的通用回退渲染
 * 当流程中包含未在 nodeTypes 注册的节点类型时，
 * React Flow 会自动使用 'default' key 指向此组件
 */
import { memo, type CSSProperties } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { Box, Wrench, FileText, MessageSquare, Server, Zap, Radio } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ICON_MAP, getNodeDisplayName } from '@/lib/nodeMeta';
import { resolveBackendIcon } from '@/lib/nodeIcons';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';
import type { PortDef } from '@/types/nodeRegistry';
import { useFlowStore, useDebugStore, useExecutionStore, selectIsNodeRunning, selectIsDebugging, selectIsNodeDebugPaused, selectIsIngressDebugWaiting } from '@/stores';
import { BreakpointMarker } from '../BreakpointMarker';
import { NodeTitle } from './NodeTitle';

/** 动态节点数据类型 */
interface DynamicNodeData {
  label?: string;
  config?: {
    type?: string;
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

/** mcp:connect 发现快照数量（tools/resources/prompts）。插件节点 config 平铺在
 *  data 顶层，旧格式可能嵌套在 data.config 下，两者都兼容。 */
interface McpDiscoveryCounts {
  tools: number;
  resources: number;
  prompts: number;
}

function getMcpDiscoveryCounts(data: DynamicNodeData): McpDiscoveryCounts | null {
  const cfg =
    data.config && typeof data.config === 'object' && !Array.isArray(data.config)
      ? data.config
      : (data as unknown as Record<string, unknown>);
  const hasAny =
    'discovered_tools' in cfg || 'discovered_resources' in cfg || 'discovered_prompts' in cfg;
  if (!hasAny) return null;
  const count = (v: unknown): number => (Array.isArray(v) ? v.length : 0);
  const counts = {
    tools: count(cfg.discovered_tools),
    resources: count(cfg.discovered_resources),
    prompts: count(cfg.discovered_prompts),
  };
  return counts.tools + counts.resources + counts.prompts > 0 ? counts : null;
}

/** grpc:connect 发现快照数量（services/methods/streaming）。与
 *  getMcpDiscoveryCounts 同款平铺/嵌套 config 兼容。 */
interface GrpcDiscoveryCounts {
  services: number;
  methods: number;
  streaming: number;
}

function getGrpcDiscoveryCounts(data: DynamicNodeData): GrpcDiscoveryCounts | null {
  const cfg =
    data.config && typeof data.config === 'object' && !Array.isArray(data.config)
      ? data.config
      : (data as unknown as Record<string, unknown>);
  const services = Array.isArray(cfg.discovered_services) ? cfg.discovered_services : [];
  if (services.length === 0) return null;
  let methods = 0;
  let streaming = 0;
  for (const service of services) {
    if (!service || typeof service !== 'object' || !('methods' in service)) continue;
    if (!Array.isArray(service.methods)) continue;
    for (const m of service.methods) {
      if (!m || typeof m !== 'object') continue;
      methods += 1;
      const isServerStreaming =
        'is_server_streaming' in m && m.is_server_streaming === true;
      const isClientStreaming =
        'is_client_streaming' in m && m.is_client_streaming === true;
      if (isServerStreaming || isClientStreaming) streaming += 1;
    }
  }
  return { services: services.length, methods, streaming };
}

/** 端口配色：true → emerald，false → rose，其余 → slate */
type HandleColor = 'slate' | 'emerald' | 'rose';

function getHandleColor(portId: string): HandleColor {
  if (portId === 'true') return 'emerald';
  if (portId === 'false') return 'rose';
  return 'slate';
}

const TARGET_DOT_COLORS: Record<HandleColor, string> = {
  slate: 'before:bg-slate-500',
  emerald: 'before:bg-emerald-600 dark:before:bg-emerald-500',
  rose: 'before:bg-rose-600 dark:before:bg-rose-500',
};

const SOURCE_BG_COLORS: Record<HandleColor, string> = {
  slate: '![background:radial-gradient(circle,_theme(colors.slate.500)_35%,_transparent_35%)]',
  emerald: '![background:radial-gradient(circle,_theme(colors.emerald.500)_35%,_transparent_35%)]',
  rose: '![background:radial-gradient(circle,_theme(colors.rose.500)_35%,_transparent_35%)]',
};

const BORDER_COLORS: Record<HandleColor, string> = {
  slate: '!border-border',
  emerald: '!border-emerald-500 dark:!border-emerald-400',
  rose: '!border-rose-500 dark:!border-rose-400',
};

/** 同侧多端口垂直分布：index+1 / (count+1) 位置；单端口居中 */
function getPortPositionStyle(count: number, index: number): CSSProperties | undefined {
  if (count <= 1) return undefined;
  return { top: `calc(${index + 1} * 100% / ${count + 1})` };
}

/**
 * 动态插件节点 - 未知节点类型的通用渲染
 * 从 data.config.type 或 React Flow node type 获取类型标识，
 * 通过 nodeMeta 查找图标/颜色，找不到时使用 Box 图标 + 灰色主题。
 * 端口按 getCachedNodeMeta 返回的 meta.ports 动态渲染：
 * position === "left" → target handle，position === "right" → source handle，
 * 无 meta.ports 时回退为单个 in/out。
 */
function DynamicPluginNode({ id, type, data, selected }: NodeProps) {
  const nodeData = data as DynamicNodeData;
  const nodeType = nodeData.config?.type || type || 'unknown';
  const meta = getCachedNodeMeta(nodeType);
  const iconData = ICON_MAP[nodeType];
  const IconComponent = iconData?.icon || (meta?.icon ? resolveBackendIcon(meta.icon) : undefined) || Box;
  const iconBg = iconData?.iconBg || 'bg-slate-500';
  const labelColor = iconData?.labelColor || 'text-slate-600';
  const pluginColor = meta?.color;

  const metaPorts = getCachedNodeMeta(nodeType)?.ports ?? [];
  const hasMetaPorts = metaPorts.length > 0;
  const leftPorts = hasMetaPorts ? metaPorts.filter((port) => port.position === 'left') : [];
  const rightPorts = hasMetaPorts ? metaPorts.filter((port) => port.position === 'right') : [];

  const hasBreakpoint = useFlowStore((state) => state.hasBreakpoint(id));
  const toggleBreakpoint = useFlowStore((state) => state.toggleBreakpoint);
  const isRunning = useExecutionStore(selectIsNodeRunning(id));
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugPaused = useDebugStore(selectIsNodeDebugPaused(id));
  const isIngressDebugWaiting = useDebugStore(selectIsIngressDebugWaiting);
  const isIngressWaitingNode = isIngressDebugWaiting && nodeType === 'http:ingress';
  const isDebugPausedAtThisNode = isRunning || isDebugPaused;

  const renderDynamicHandle = (port: PortDef, index: number, count: number, side: 'left' | 'right') => {
    const color = getHandleColor(port.id);
    const topClass = count <= 1 ? '!top-1/2' : undefined;
    return (
      <Handle
        key={port.id}
        type={side === 'left' ? 'target' : 'source'}
        position={side === 'left' ? Position.Left : Position.Right}
        id={port.id}
        style={getPortPositionStyle(count, index)}
        className={cn(
          '!w-3.5 !h-3.5',
          topClass,
          '!bg-background !border-2',
          BORDER_COLORS[color],
          'hover:!scale-125',
          'transition-transform duration-150',
          side === 'left'
            ? `before:content-[''] before:absolute before:inset-[3px] before:rounded-full ${TARGET_DOT_COLORS[color]}`
            : SOURCE_BG_COLORS[color]
        )}
      />
    );
  };

  return (
    <div
      className={cn(
        'relative px-4 py-2 rounded-xl',
        'border-2 border-border',
        'bg-background',
        'w-fit min-w-[180px] max-w-[400px]',
        'flex items-center',
        selected && 'border-[3px] border-slate-500',
        isIngressWaitingNode && 'border-indigo-500/80 shadow-[0_0_15px_rgba(99,102,241,0.3)]',
        isDebugPausedAtThisNode && 'debug-breathing-sky'
      )}
    >
      {isDebugging && (
        <BreakpointMarker
          hasBreakpoint={hasBreakpoint}
          onToggle={() => toggleBreakpoint(id)}
        />
      )}

      {hasMetaPorts ? (
        <>
          {leftPorts.map((port, index) => renderDynamicHandle(port, index, leftPorts.length, 'left'))}
          {rightPorts.map((port, index) => renderDynamicHandle(port, index, rightPorts.length, 'right'))}
        </>
      ) : (
        <>
          <Handle
            type="target"
            position={Position.Left}
            id="in"
            className={cn(
              '!w-3.5 !h-3.5',
              '!bg-background !border-2 !border-border',
              'hover:!scale-125',
              'transition-transform duration-150',
              "before:content-[''] before:absolute before:inset-[3px] before:bg-slate-500 before:rounded-full"
            )}
          />

          <Handle
            type="source"
            position={Position.Right}
            id="out"
            className={cn(
              '!w-3.5 !h-3.5',
              '!bg-background !border-2 !border-border',
              '![background:radial-gradient(circle,_theme(colors.slate.500)_35%,_transparent_35%)]',
              'hover:!scale-125',
              'transition-transform duration-150'
            )}
          />
        </>
      )}

      <div className="flex items-center gap-2.5 min-w-0">
        <div
          className={cn(
            'shrink-0 p-1.5 rounded-lg',
            pluginColor ? 'text-white' : `${iconBg} text-white`
          )}
          style={pluginColor ? { backgroundColor: pluginColor } : undefined}
        >
          <IconComponent className="w-3.5 h-3.5" />
        </div>
        <div className="flex flex-col min-w-0">
          <span
            className={cn('text-xs font-medium uppercase tracking-wide whitespace-nowrap leading-tight', !pluginColor && labelColor)}
            style={pluginColor ? { color: pluginColor } : undefined}
          >
            {nodeType}
          </span>
          <NodeTitle
            nodeId={id}
            label={nodeData.label}
            defaultLabel={getNodeDisplayName(nodeType)}
            className="max-w-[110px] leading-snug"
          />
          {nodeType === 'mcp:connect' &&
            (() => {
              const counts = getMcpDiscoveryCounts(nodeData);
              if (!counts) return null;
              return (
                <div className="flex items-center gap-1 flex-wrap mt-0.5">
                  {counts.tools > 0 && (
                    <span
                      data-testid="mcp-tools-count"
                      className="inline-flex items-center gap-0.5 rounded-full bg-amber-100 dark:bg-amber-950/50 px-1.5 py-0 text-[9px] font-medium text-amber-700 dark:text-amber-300"
                    >
                      <Wrench className="h-2.5 w-2.5" />
                      {counts.tools}
                    </span>
                  )}
                  {counts.resources > 0 && (
                    <span
                      data-testid="mcp-resources-count"
                      className="inline-flex items-center gap-0.5 rounded-full bg-blue-100 dark:bg-blue-950/50 px-1.5 py-0 text-[9px] font-medium text-blue-700 dark:text-blue-300"
                    >
                      <FileText className="h-2.5 w-2.5" />
                      {counts.resources}
                    </span>
                  )}
                  {counts.prompts > 0 && (
                    <span
                      data-testid="mcp-prompts-count"
                      className="inline-flex items-center gap-0.5 rounded-full bg-purple-100 dark:bg-purple-950/50 px-1.5 py-0 text-[9px] font-medium text-purple-700 dark:text-purple-300"
                    >
                      <MessageSquare className="h-2.5 w-2.5" />
                      {counts.prompts}
                    </span>
                  )}
                </div>
              );
            })()}
          {nodeType === 'grpc:connect' &&
            (() => {
              const counts = getGrpcDiscoveryCounts(nodeData);
              if (!counts) return null;
              return (
                <div className="flex items-center gap-1 flex-wrap mt-0.5">
                  {counts.services > 0 && (
                    <span
                      data-testid="grpc-services-count"
                      title="Services"
                      className="inline-flex items-center gap-0.5 rounded-full bg-violet-100 dark:bg-violet-950/50 px-1.5 py-0 text-[9px] font-medium text-violet-700 dark:text-violet-300"
                    >
                      <Server className="h-2.5 w-2.5" />
                      {counts.services}
                    </span>
                  )}
                  {counts.methods > 0 && (
                    <span
                      data-testid="grpc-methods-count"
                      title="Methods"
                      className="inline-flex items-center gap-0.5 rounded-full bg-indigo-100 dark:bg-indigo-950/50 px-1.5 py-0 text-[9px] font-medium text-indigo-700 dark:text-indigo-300"
                    >
                      <Zap className="h-2.5 w-2.5" />
                      {counts.methods}
                    </span>
                  )}
                  {counts.streaming > 0 && (
                    <span
                      data-testid="grpc-streaming-count"
                      title="Streaming Methods"
                      className="inline-flex items-center gap-0.5 rounded-full bg-purple-100 dark:bg-purple-950/50 px-1.5 py-0 text-[9px] font-medium text-purple-700 dark:text-purple-300"
                    >
                      <Radio className="h-2.5 w-2.5" />
                      {counts.streaming}
                    </span>
                  )}
                </div>
              );
            })()}
        </div>
      </div>
    </div>
  );
}

export default memo(DynamicPluginNode);
