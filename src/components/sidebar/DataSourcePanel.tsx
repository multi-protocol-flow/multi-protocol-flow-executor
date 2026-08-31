/**
 * DataSourcePanel - 左侧栏数据源面板
 *
 * 展示插件注册的数据源（describe 的 DatasourceDescription 列表）：
 * - 未连接 → "配置"：内联渲染 config_schema 表单（string 字段；password 用
 *   password input），保存前调 validate_config 校验（不通过不保存）。
 * - 已连接 → 浏览树：项目级 browse → 展开分组 → 子资源条目；leaf 条目可
 *   拖拽落画布（`application/reactflow/datasource` payload），点击可弹窗查阅原始数据与转换详情。
 * - 刷新：清空浏览缓存并重新拉取当前展开节点。
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  Globe,
  Wifi,
  RadioTower,
  Cable,
  Braces,
  Send,
  Hash,
  Network,
  RefreshCw,
  Loader2,
  Eye,
  type LucideIcon,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { useDatasourceStore } from '@/stores';
import { datasourceValidateConfig } from '@/services/tauri/datasource';
import { DatasourceItemDetailDialog } from '@/components/dialogs/DatasourceItemDetailDialog';
import type {
  DataItem,
  DataItemProtocol,
  DatasourceDescription,
  DatasourceResource,
} from '@/types/datasource';

/** 协议 → lucide 图标（与 nodeIcons.ts 同源图标集） */
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

/** 协议 → 颜色徽章（text-* 类） */
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

/** config_schema 的 properties 条目（JSON Schema object 子集） */
interface SchemaProperty {
  type?: string;
  format?: string;
  title?: string;
}

/** config_schema 的 object 形状（仅取面板需要字段） */
interface ConfigSchema {
  type?: string;
  properties?: Record<string, SchemaProperty>;
  required?: string[];
}

function formatErrorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** 单个数据源：连接状态 + 配置表单 / 浏览树 */
function DatasourceEntry({
  ds,
  refreshToken,
  onSelectItem,
}: {
  ds: DatasourceDescription;
  refreshToken: number;
  onSelectItem: (ds: DatasourceDescription, item: DataItem) => void;
}) {
  const LL = getLL();
  const connections = useDatasourceStore((s) => s.connections);
  const currentConn = useMemo(
    () => connections.find((c) => c.datasource_id === ds.id),
    [connections, ds.id],
  );
  const connected = currentConn != null;
  const [configOpen, setConfigOpen] = useState(false);
  const [configValues, setConfigValues] = useState<Record<string, string>>({});
  const [configError, setConfigError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const toggleConfig = useCallback(() => {
    if (!configOpen && currentConn?.config) {
      // 打开时自动回填已保存的配置
      const stringified: Record<string, string> = {};
      for (const [k, v] of Object.entries(currentConn.config)) {
        if (v != null) stringified[k] = String(v);
      }
      setConfigValues(stringified);
    }
    setConfigError(null);
    setConfigOpen((v) => !v);
  }, [configOpen, currentConn]);

  const handleConfigChange = useCallback((key: string, value: string) => {
    setConfigValues((prev) => ({ ...prev, [key]: value }));
    setConfigError(null);
  }, []);

  const handleDisconnect = useCallback(async () => {
    if (currentConn) {
      await useDatasourceStore.getState().deleteConnection(currentConn.id);
      setConfigOpen(false);
      setConfigValues({});
      setConfigError(null);
    }
  }, [currentConn]);

  const handleSave = useCallback(async () => {
    const schema = (ds.config_schema ?? null) as ConfigSchema | null;
    for (const field of schema?.required ?? []) {
      if (!(configValues[field] ?? '').trim()) {
        setConfigError(LL.datasource.required_field({ field }));
        return;
      }
    }
    setSaving(true);
    setConfigError(null);
    try {
      const config: Record<string, unknown> = { ...configValues };
      const result = await datasourceValidateConfig(ds.id, config);
      if (!result.valid) {
        setConfigError(result.message ?? LL.datasource.validate_fail());
        return;
      }
      await useDatasourceStore
        .getState()
        .saveConnection({
          id: currentConn?.id ?? crypto.randomUUID(),
          datasource_id: ds.id,
          display_name: ds.display_name,
          config,
          created_at: currentConn?.created_at ?? new Date().toISOString(),
        });
      setConfigOpen(false);
    } catch (e) {
      setConfigError(formatErrorMessage(e));
    } finally {
      setSaving(false);
    }
  }, [LL, configValues, currentConn, ds]);

  return (
    <div className="border-b border-border last:border-b-0">
      {/* 数据源行 */}
      <div className="flex items-center gap-2 px-3 py-1.5">
        <span className="flex-1 min-w-0 truncate text-sm font-medium text-foreground">
          {ds.display_name}
        </span>
        {connected ? (
          <Badge variant="outline" className="text-emerald-600 shrink-0">
            {LL.datasource.connected()}
          </Badge>
        ) : (
          <Badge variant="outline" className="text-muted-foreground shrink-0">
            {LL.datasource.not_connected()}
          </Badge>
        )}
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2 text-xs shrink-0"
          onClick={toggleConfig}
        >
          {connected ? LL.datasource.edit() : LL.datasource.configure()}
        </Button>
      </div>

      {/* 配置表单 */}
      {configOpen && (
        <div className="px-3 pb-2 space-y-2">
          {ds.config_schema
            ? (() => {
                const schema = ds.config_schema as ConfigSchema;
                const properties = schema.properties ?? {};
                return Object.entries(properties).map(([key, prop]) => (
                  <label key={key} className="block space-y-1">
                    <span className="text-xs text-muted-foreground">
                      {prop.title ?? key}
                      {schema.required?.includes(key) && (
                        <span className="text-destructive ml-0.5">*</span>
                      )}
                    </span>
                    <Input
                      type={prop.format === 'password' ? 'password' : 'text'}
                      value={configValues[key] ?? ''}
                      placeholder={prop.title ?? key}
                      onChange={(e) => handleConfigChange(key, e.target.value)}
                      className="h-7 text-xs"
                    />
                  </label>
                ));
              })()
            : null}
          {configError && (
            <p className="text-xs text-destructive">{configError}</p>
          )}
          <div className="flex gap-2 pt-1">
            <Button
              size="sm"
              className="h-6 px-2 text-xs"
              onClick={handleSave}
              disabled={saving}
            >
              {saving && <Loader2 className="w-3 h-3 animate-spin mr-1" />}
              {LL.datasource.save()}
            </Button>
            {connected && (
              <Button
                size="sm"
                variant="destructive"
                className="h-6 px-2 text-xs mr-auto"
                onClick={handleDisconnect}
                disabled={saving}
              >
                {LL.datasource.disconnect()}
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="h-6 px-2 text-xs"
              onClick={() => {
                setConfigOpen(false);
                setConfigError(null);
              }}
            >
              {LL.datasource.cancel()}
            </Button>
          </div>
        </div>
      )}
      {/* 浏览树（仅渲染顶层根资源；子资源在展开分组时按需递归渲染） */}
      {connected && (
        <div className="pb-1">
          {(() => {
            const childIds = new Set(
              ds.resources.map((r) => r.children).filter(Boolean),
            );
            const rootResources = ds.resources.filter(
              (r) => !childIds.has(r.id),
            );
            return rootResources.map((resource) => (
              <ResourceLevel
                key={resource.id}
                ds={ds}
                resource={resource}
                parentId={null}
                refreshToken={refreshToken}
                onSelectItem={onSelectItem}
              />
            ));
          })()}
        </div>
      )}
    </div>
  );
}

/**
 * 一个资源层级：根级 browse(resource.id, parentId) 渲染条目；若资源有
 * `children`，条目是可展开的分组，展开后 browse(children, item.id) 渲染子级
 * 条目（leaf）。`parentId` 为 null 表示顶层。
 */
/** 嵌套分组树节点 */
export interface GroupTreeNode {
  id: string;
  name: string;
  fullPath: string;
  children: GroupTreeNode[];
  items: DataItem[];
  totalCount: number;
}

/** 分隔路径为层级段落（支持 /、\、>、->） */
export function splitGroupPath(pathStr: string): string[] {
  return pathStr
    .split(/\/|\\|\s*->\s*|\s*>\s*/)
    .map((s) => s.trim())
    .filter(Boolean);
}

/** 提取数据项的分组层级路径 */
export function extractItemGroupPath(
  item: DataItem,
  idToName?: Map<string, string>,
): string[] | null {
  const extraGroup = item.extra?.find(
    (e) => e.label === '分组' || e.label === 'group' || e.label === 'Group',
  )?.value;

  const raw = item.raw as Record<string, unknown> | undefined;
  const rawGroupName =
    typeof raw?.group_name === 'string'
      ? raw.group_name.trim()
      : typeof raw?.groupName === 'string'
        ? raw.groupName.trim()
        : '';

  // 1. 优先从 extra 中提取
  if (extraGroup && typeof extraGroup === 'string' && extraGroup.trim()) {
    const trimmed = extraGroup.trim();
    const commaSegments = trimmed.split(',').map((s) => s.trim()).filter(Boolean);
    const isAllNumeric =
      commaSegments.length > 0 && commaSegments.every((s) => /^\d+$/.test(s));

    if (isAllNumeric) {
      if (idToName) {
        const resolved: string[] = [];
        for (let i = 0; i < commaSegments.length; i++) {
          const gid = commaSegments[i];
          if (idToName.has(gid)) {
            resolved.push(idToName.get(gid)!);
          } else if (i === commaSegments.length - 1 && rawGroupName) {
            resolved.push(rawGroupName);
          }
        }
        if (resolved.length > 0) return resolved;
      }
      if (rawGroupName) return splitGroupPath(rawGroupName);
    } else {
      const segments = splitGroupPath(trimmed);
      const cleanSegments = segments.filter((s) => !/^\d+$/.test(s));
      if (cleanSegments.length > 0) return cleanSegments;
      if (rawGroupName) return splitGroupPath(rawGroupName);
    }
  }

  // 2. 检查 raw?.group_path
  const rawPath = raw?.group_path || raw?.group_name_path || raw?.groupPath;
  if (rawPath != null) {
    let pathSegments: string[] = [];
    if (Array.isArray(rawPath)) {
      pathSegments = rawPath.map((s) => String(s).trim()).filter(Boolean);
    } else if (typeof rawPath === 'string' && rawPath.trim()) {
      pathSegments = rawPath.includes(',')
        ? rawPath.split(',').map((s) => s.trim()).filter(Boolean)
        : splitGroupPath(rawPath);
    }

    const isAllNumeric =
      pathSegments.length > 0 && pathSegments.every((s) => /^\d+$/.test(s));
    if (isAllNumeric) {
      if (idToName) {
        const resolved: string[] = [];
        for (let i = 0; i < pathSegments.length; i++) {
          const gid = pathSegments[i];
          if (idToName.has(gid)) {
            resolved.push(idToName.get(gid)!);
          } else if (i === pathSegments.length - 1 && rawGroupName) {
            resolved.push(rawGroupName);
          }
        }
        if (resolved.length > 0) return resolved;
      }
      if (rawGroupName) return splitGroupPath(rawGroupName);
    } else {
      const cleanSegments = pathSegments.filter((s) => !/^\d+$/.test(s));
      if (cleanSegments.length > 0) return cleanSegments;
    }
  }

  // 3. 最后使用 raw.group_name 兜底
  if (rawGroupName) {
    return splitGroupPath(rawGroupName);
  }

  return null;
}

/** 根据 items 构建多级嵌套分组树 */
export function buildGroupTree(
  items: DataItem[],
  defaultGroupName: string,
): GroupTreeNode[] | null {
  // 预扫描所有 items 构建 group_id -> group_name 字典
  const idToName = new Map<string, string>();
  for (const item of items) {
    const raw = item.raw as Record<string, unknown> | undefined;
    if (raw) {
      const gid = raw.group_id ?? raw.groupID ?? raw.api_group_id;
      const gname = raw.group_name ?? raw.groupName;
      if (gid != null && typeof gname === 'string' && gname.trim()) {
        idToName.set(String(gid), gname.trim());
      }
    }
  }

  let hasAnyGroup = false;
  for (const item of items) {
    if (extractItemGroupPath(item, idToName) != null) {
      hasAnyGroup = true;
      break;
    }
  }

  if (!hasAnyGroup) return null;

  interface MutableNode {
    id: string;
    name: string;
    fullPath: string;
    childrenMap: Map<string, MutableNode>;
    items: DataItem[];
  }

  const rootChildrenMap = new Map<string, MutableNode>();
  const ungroupedItems: DataItem[] = [];

  for (const item of items) {
    const pathSegments = extractItemGroupPath(item, idToName);
    if (!pathSegments || pathSegments.length === 0) {
      ungroupedItems.push(item);
      continue;
    }

    let currentMap = rootChildrenMap;
    let currentPath = '';
    let currentNode: MutableNode | null = null;
    for (let i = 0; i < pathSegments.length; i++) {
      const segName = pathSegments[i];
      currentPath = currentPath ? `${currentPath}/${segName}` : segName;

      let nextNode = currentMap.get(segName);
      if (!nextNode) {
        nextNode = {
          id: `group:${currentPath}`,
          name: segName,
          fullPath: currentPath,
          childrenMap: new Map(),
          items: [],
        };
        currentMap.set(segName, nextNode);
      }
      currentNode = nextNode;
      currentMap = nextNode.childrenMap;
    }

    if (currentNode) {
      currentNode.items.push(item);
    }
  }

  function finalizeNode(node: MutableNode): GroupTreeNode {
    const children = Array.from(node.childrenMap.values()).map(finalizeNode);
    const childrenCount = children.reduce((sum, c) => sum + c.totalCount, 0);
    return {
      id: node.id,
      name: node.name,
      fullPath: node.fullPath,
      children,
      items: node.items,
      totalCount: node.items.length + childrenCount,
    };
  }

  const result: GroupTreeNode[] = Array.from(rootChildrenMap.values()).map(finalizeNode);

  if (ungroupedItems.length > 0) {
    result.push({
      id: `group:${defaultGroupName}`,
      name: defaultGroupName,
      fullPath: defaultGroupName,
      children: [],
      items: ungroupedItems,
      totalCount: ungroupedItems.length,
    });
  }

  return result;
}

/** 递归渲染分组目录项 */
function GroupFolderItem({
  node,
  ds,
  depth,
  expanded,
  onToggle,
  onSelectItem,
}: {
  node: GroupTreeNode;
  ds: DatasourceDescription;
  depth: number;
  expanded: Record<string, boolean>;
  onToggle: (path: string) => void;
  onSelectItem: (ds: DatasourceDescription, item: DataItem) => void;
}) {
  const isOpen = expanded[node.fullPath] ?? false;

  return (
    <div>
      <button
        type="button"
        className="w-full flex items-center gap-1.5 px-3 py-1 text-left text-xs text-foreground hover:bg-accent font-medium select-none"
        onClick={() => onToggle(node.fullPath)}
      >
        {isOpen ? (
          <ChevronDown className="w-3 h-3 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3 h-3 text-muted-foreground shrink-0" />
        )}
        {isOpen ? (
          <FolderOpen className="w-3.5 h-3.5 text-amber-500 shrink-0" />
        ) : (
          <Folder className="w-3.5 h-3.5 text-amber-500 shrink-0" />
        )}
        <span className="truncate flex-1">{node.name}</span>
        <span className="text-[10px] text-muted-foreground font-mono">
          {`(${node.totalCount})`}
        </span>
      </button>
      {isOpen && (
        <div className="pl-3">
          {node.children.map((child) => (
            <GroupFolderItem
              key={child.id}
              node={child}
              ds={ds}
              depth={depth + 1}
              expanded={expanded}
              onToggle={onToggle}
              onSelectItem={onSelectItem}
            />
          ))}
          {node.items.map((item) => (
            <ItemRow key={item.id} ds={ds} item={item} onSelect={onSelectItem} />
          ))}
        </div>
      )}
    </div>
  );
}
function ResourceLevel({
  ds,
  resource,
  parentId,
  refreshToken,
  onSelectItem,
}: {
  ds: DatasourceDescription;
  resource: DatasourceResource;
  parentId: string | null;
  refreshToken: number;
  onSelectItem: (ds: DatasourceDescription, item: DataItem) => void;
}) {
  const LL = getLL();
  const cacheKey = `${ds.id}/${resource.id}/${parentId ?? ''}`;
  const cached = useDatasourceStore((s) => s.browseCache[cacheKey]);
  const loading = useDatasourceStore((s) => s.loading);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [groupExpanded, setGroupExpanded] = useState<Record<string, boolean>>({});

  // 首次挂载 / 刷新时拉取本层条目（缓存命中直接复用）。
  useEffect(() => {
    if (cached) return;
    useDatasourceStore
      .getState()
      .browse(ds.id, resource.id, parentId)
      .catch(() => {
        /* 错误已写入 store.error，面板统一展示 */
      });
  }, [ds.id, resource.id, parentId, refreshToken]); // eslint-disable-line react-hooks/exhaustive-deps

  const items = cached ?? [];

  // 自动根据 item 中的 group_name / extra 提取多级嵌套分组树
  const groupTree = useMemo(() => {
    if (resource.children != null) return null;
    return buildGroupTree(items, LL.datasource.default_group());
  }, [items, resource.children, LL]);

  const handleToggleGroup = useCallback((groupPath: string) => {
    setGroupExpanded((prev) => ({
      ...prev,
      [groupPath]: !prev[groupPath],
    }));
  }, []);

  return (
    <div>
      {/* 资源层级头（仅顶层展示；子级作为分组内容渲染） */}
      {parentId === null && (
        <div className="px-3 pt-1 pb-0.5 text-xs text-muted-foreground flex items-center gap-1">
          <span className="truncate">{resource.label}</span>
          {loading && !cached && <Loader2 className="w-3 h-3 animate-spin shrink-0" />}
        </div>
      )}

      {/* 叶子节点按多级嵌套分组展示 */}
      {groupTree ? (
        groupTree.map((node) => (
          <GroupFolderItem
            key={node.id}
            node={node}
            ds={ds}
            depth={0}
            expanded={groupExpanded}
            onToggle={handleToggleGroup}
            onSelectItem={onSelectItem}
          />
        ))
      ) : (
        items.map((item) => {
          const hasChildren = resource.children != null;
          if (hasChildren) {
            // 分组条目：点击展开 → 浏览子资源
            const isExpanded = expanded[item.id] ?? false;
            return (
              <div key={item.id}>
                <button
                  type="button"
                  title={item.extra.length > 0 ? `${item.name}\n${item.extra.map((kv) => `${kv.label}: ${kv.value}`).join(' · ')}` : item.name}
                  className="w-full flex items-center gap-1.5 px-3 py-1 text-left text-sm text-foreground hover:bg-accent font-medium select-none"
                  onClick={() => {
                    const next = !isExpanded;
                    setExpanded((prev) => ({ ...prev, [item.id]: next }));
                    if (next) {
                      useDatasourceStore
                        .getState()
                        .browse(ds.id, resource.children!, item.id)
                        .catch(() => {
                          /* 错误已写入 store.error */
                        });
                    }
                  }}
                >
                  {isExpanded ? (
                    <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
                  ) : (
                    <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
                  )}
                  {isExpanded ? (
                    <FolderOpen className="w-3.5 h-3.5 text-primary shrink-0" />
                  ) : (
                    <Folder className="w-3.5 h-3.5 text-primary shrink-0" />
                  )}
                  <span className="truncate flex-1">{item.name}</span>
                </button>
                {isExpanded && (
                  <div className="pl-3">
                    <ResourceLevel
                      ds={ds}
                      resource={
                        ds.resources.find((r) => r.id === resource.children) ?? {
                          id: resource.children!,
                          label: '',
                          children: null,
                          leaf: true,
                        }
                      }
                      parentId={item.id}
                      refreshToken={refreshToken}
                      onSelectItem={onSelectItem}
                    />
                  </div>
                )}
              </div>
            );
          }
          return <ItemRow key={item.id} ds={ds} item={item} onSelect={onSelectItem} />;
        })
      )}

      {!loading && cached && items.length === 0 && (
        <div className="px-3 py-1 text-xs text-muted-foreground">
          {LL.datasource.empty()}
        </div>
      )}
    </div>
  );
}

/** 叶子条目行：协议徽标 + method + 名称；可拖拽，点击打开详情弹窗，鼠标 hover 展示 url 及详细信息 */
function ItemRow({
  ds,
  item,
  onSelect,
}: {
  ds: DatasourceDescription;
  item: DataItem;
  onSelect: (ds: DatasourceDescription, item: DataItem) => void;
}) {
  const LL = getLL();
  const Icon = PROTOCOL_ICON_MAP[item.protocol] ?? Network;

  const handleDragStart = useCallback(
    (e: React.DragEvent) => {
      e.dataTransfer.setData(
        'application/reactflow/datasource',
        JSON.stringify({ datasource_id: ds.id, item_ids: [item.id] }),
      );
      e.dataTransfer.effectAllowed = 'move';
    },
    [ds.id, item.id],
  );

  const tooltipText = useMemo(() => {
    const lines: string[] = [];

    // 第一行：[方法/协议] 名称
    const typeLabel = item.method || item.protocol.toUpperCase();
    lines.push(`[${typeLabel}] ${item.name}`);

    // 第二行：URL/Path
    if (item.path) {
      lines.push(`${item.method ? `${item.method} ` : ''}${item.path}`);
    }

    // 摘要
    if (item.summary) {
      lines.push(item.summary);
    }

    // 附加信息（分组、标签等）
    if (item.extra && item.extra.length > 0) {
      lines.push(item.extra.map((kv) => `${kv.label}: ${kv.value}`).join(' · '));
    }

    // 操作提示
    lines.push(`${LL.datasource.view_detail()} (${LL.datasource.drag_hint()})`);

    return lines.join('\n');
  }, [item, LL]);

  return (
    <div
      draggable
      onDragStart={handleDragStart}
      onClick={() => onSelect(ds, item)}
      title={tooltipText}
      className="group flex items-center gap-1.5 px-3 py-1 cursor-pointer hover:bg-accent rounded-sm select-none transition-colors"
    >
      <Icon className={cn('w-3.5 h-3.5 shrink-0', PROTOCOL_COLOR_MAP[item.protocol] ?? 'text-slate-500')} />
      {item.method && (
        <Badge variant="secondary" className="h-4 px-1 text-[10px] font-mono shrink-0">
          {item.method}
        </Badge>
      )}
      <span className="min-w-0 truncate text-sm text-foreground flex-1">{item.name}</span>
      <Button
        variant="ghost"
        size="icon"
        className="w-5 h-5 opacity-0 group-hover:opacity-100 shrink-0 text-muted-foreground hover:text-foreground"
        onClick={(e) => {
          e.stopPropagation();
          onSelect(ds, item);
        }}
        title={LL.datasource.view_detail()}
      >
        <Eye className="w-3.5 h-3.5" />
      </Button>
    </div>
  );
}

export function DataSourcePanel() {
  const LL = getLL();
  const descriptions = useDatasourceStore((s) => s.descriptions);
  const error = useDatasourceStore((s) => s.error);
  const loading = useDatasourceStore((s) => s.loading);
  const [refreshToken, setRefreshToken] = useState(0);

  // 弹窗状态：选中的数据源与条目
  const [selectedItemState, setSelectedItemState] = useState<{
    ds: DatasourceDescription;
    item: DataItem;
  } | null>(null);

  // 挂载时加载描述列表 + 已保存连接
  useEffect(() => {
    useDatasourceStore.getState().loadDescriptions();
    useDatasourceStore.getState().loadConnections();
  }, []);

  /** 刷新：清空缓存并重新拉取当前展开节点（refreshToken 触发各层 effect） */
  const handleRefresh = useCallback(() => {
    useDatasourceStore.getState().clearCache();
    setRefreshToken((t) => t + 1);
  }, []);

  const handleSelectItem = useCallback((ds: DatasourceDescription, item: DataItem) => {
    setSelectedItemState({ ds, item });
  }, []);

  return (
    <div className="flex flex-col h-full min-h-0" data-testid="datasource-panel">
      {/* 面板工具栏（折叠面板标题下的内容头） */}
      <div className="flex items-center justify-between px-3 py-1 border-b border-border">
        <span className="text-xs text-muted-foreground">{LL.datasource.drag_hint()}</span>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 rounded-md hover:bg-accent"
          title={LL.datasource.refresh()}
          onClick={handleRefresh}
        >
          <RefreshCw className={cn('w-3.5 h-3.5', loading && 'animate-spin')} />
        </Button>
      </div>

      {/* 错误横幅（browse/validate 失败；认证失效提示重新配置） */}
      {error && (
        <div className="px-3 py-1.5 bg-destructive/10 border-b border-destructive/20">
          <p className="text-xs text-destructive">{error}</p>
          <p className="text-xs text-muted-foreground">{LL.datasource.auth_failed()}</p>
        </div>
      )}

      {/* 数据源列表 */}
      <div className="flex-1 min-h-0 overflow-y-auto scrollbar-thin">
        {descriptions.length === 0 && !loading && (
          <div className="px-3 py-3 text-xs text-muted-foreground">
            {LL.datasource.no_datasources()}
          </div>
        )}
        {descriptions.map((ds) => (
          <DatasourceEntry
            key={ds.id}
            ds={ds}
            refreshToken={refreshToken}
            onSelectItem={handleSelectItem}
          />
        ))}
      </div>

      {/* 条目原始数据与转换详情弹窗 */}
      <DatasourceItemDetailDialog
        open={Boolean(selectedItemState)}
        onOpenChange={(open) => {
          if (!open) setSelectedItemState(null);
        }}
        datasource={selectedItemState?.ds ?? null}
        item={selectedItemState?.item ?? null}
      />
    </div>
  );
}
