/**
 * 后端 icon 字段到 lucide-react 组件的映射
 *
 * 供 NodePalette 与 DynamicPluginNode 共享：
 * 后端 NodeMetadata.icon 是 kebab-case 字符串标识，
 * 此映射将其解析为可渲染的 Lucide 图标组件。
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
  RotateCcw,
  Wrench,
  Network,
  Puzzle,
  Cable,
  PhoneOutgoing,
  PhoneOff,
  FileSearch,
  FileText,
  Unplug,
  XCircle,
  Download,
  Search,
  Eye,
  EyeOff,
  Mail,
  Layers,
  Settings,
  Ear,
  RadioTower,
  Shuffle,
  Trash2,
  List,
  Trash,
  Link2,
  Inbox,
  Braces,
  Timer,
  ArrowDownLeft,
  MessageSquare,
  Copy,
  FolderPlus,
  Plus,
  RefreshCw,
} from 'lucide-react';

/** 后端 icon 字段到 lucide-react 组件的映射 */
export const BACKEND_ICON_MAP: Record<string, LucideIcon> = {
  play: Play,
  globe: Globe,
  'git-branch': GitBranch,
  code: Code,
  'check-circle': CheckCircle,
  wifi: Wifi,
  send: Send,
  plug: Plug,
  database: Database,
  'circle-play': CirclePlay,
  check: CheckCircle,
  'rotate-ccw': RotateCcw,
  wrench: Wrench,
  network: Network,
  puzzle: Puzzle,
  cable: Cable,
  'phone-outgoing': PhoneOutgoing,
  'phone-off': PhoneOff,
  'file-search': FileSearch,
  'file-text': FileText,
  'file-code': Code,
  unplug: Unplug,
  'x-circle': XCircle,
  download: Download,
  search: Search,
  eye: Eye,
  'eye-off': EyeOff,
  mail: Mail,
  layers: Layers,
  settings: Settings,
  ear: Ear,
  'radio-tower': RadioTower,
  shuffle: Shuffle,
  'trash-2': Trash2,
  list: List,
  trash: Trash,
  link: Link2,
  inbox: Inbox,
  braces: Braces,
  timer: Timer,
  'arrow-down-to-line': ArrowDownLeft,
  undo: RotateCcw,
  x: Unplug,
  'message-square': MessageSquare,
  copy: Copy,
  'folder-plus': FolderPlus,
  plus: Plus,
  'refresh-cw': RefreshCw,
  plugin: Puzzle,
};

/**
 * 将后端 icon 字符串解析为 Lucide 图标组件
 *
 * @param icon - 后端元数据中的 kebab-case 图标标识
 * @returns 对应的 Lucide 图标组件；未识别时返回 undefined
 */
export function resolveBackendIcon(icon: string): LucideIcon | undefined {
  return BACKEND_ICON_MAP[icon];
}
