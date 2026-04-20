/**
 * 节点类型元数据常量
 * 提供 14 种节点类型的图标、颜色、名称映射，供 NodePalette / FlowNodeList 等组件共享
 */
import type { LucideIcon } from 'lucide-react';
import {
  Play,
  Globe,
  GitBranch,
  Code,
  CheckCircle,
  Wifi,
  Send,
  Plug,
  Database,
  CirclePlay,
  Check,
  RotateCcw,
} from 'lucide-react';
import type { NodeType } from '@/types';

/** 节点类型元数据 */
export interface NodeTypeMeta {
  /** 显示名称 */
  name: string;
  /** lucide-react 图标组件 */
  icon: LucideIcon;
  /** 图标背景色 class (如 'bg-blue-500') */
  iconBg: string;
  /** 标签颜色 class (如 'text-blue-600') */
  labelColor: string;
  /** 徽章颜色 class (如 'bg-blue-100 text-blue-700') */
  badge: string;
}

/** 14 种节点类型的元数据映射 */
export const NODE_META: Record<NodeType, NodeTypeMeta> = {
  entry: {
    name: '入口',
    icon: Play,
    iconBg: 'bg-emerald-500',
    labelColor: 'text-emerald-600',
    badge: 'bg-emerald-100 text-emerald-700',
  },
  http: {
    name: 'HTTP 请求',
    icon: Globe,
    iconBg: 'bg-blue-500',
    labelColor: 'text-blue-600',
    badge: 'bg-blue-100 text-blue-700',
  },
  conditional: {
    name: '条件判断',
    icon: GitBranch,
    iconBg: 'bg-orange-500',
    labelColor: 'text-orange-600',
    badge: 'bg-orange-100 text-orange-700',
  },
  script: {
    name: '脚本',
    icon: Code,
    iconBg: 'bg-indigo-500',
    labelColor: 'text-indigo-600',
    badge: 'bg-indigo-100 text-indigo-700',
  },
  assertion: {
    name: '断言',
    icon: CheckCircle,
    iconBg: 'bg-purple-500',
    labelColor: 'text-purple-600',
    badge: 'bg-purple-100 text-purple-700',
  },
  ws_connect: {
    name: 'WS-连接',
    icon: Wifi,
    iconBg: 'bg-amber-500',
    labelColor: 'text-amber-600',
    badge: 'bg-amber-100 text-amber-700',
  },
  ws_send_collect: {
    name: 'WS-发送采集',
    icon: Send,
    iconBg: 'bg-teal-500',
    labelColor: 'text-teal-600',
    badge: 'bg-teal-100 text-teal-700',
  },
  ws_close: {
    name: 'WS-关闭',
    icon: Plug,
    iconBg: 'bg-rose-500',
    labelColor: 'text-rose-600',
    badge: 'bg-rose-100 text-rose-700',
  },
  db_connect: {
    name: 'DB-连接',
    icon: Database,
    iconBg: 'bg-violet-500',
    labelColor: 'text-violet-600',
    badge: 'bg-violet-100 text-violet-700',
  },
  db_query: {
    name: 'DB-查询',
    icon: Database,
    iconBg: 'bg-violet-500',
    labelColor: 'text-violet-600',
    badge: 'bg-violet-100 text-violet-700',
  },
  db_begin: {
    name: 'DB-开始事务',
    icon: CirclePlay,
    iconBg: 'bg-violet-500',
    labelColor: 'text-violet-600',
    badge: 'bg-violet-100 text-violet-700',
  },
  db_commit: {
    name: 'DB-提交',
    icon: Check,
    iconBg: 'bg-violet-500',
    labelColor: 'text-violet-600',
    badge: 'bg-violet-100 text-violet-700',
  },
  db_rollback: {
    name: 'DB-回滚',
    icon: RotateCcw,
    iconBg: 'bg-violet-500',
    labelColor: 'text-violet-600',
    badge: 'bg-violet-100 text-violet-700',
  },
  db_close: {
    name: 'DB-关闭',
    icon: Plug,
    iconBg: 'bg-violet-500',
    labelColor: 'text-violet-600',
    badge: 'bg-violet-100 text-violet-700',
  },
};
