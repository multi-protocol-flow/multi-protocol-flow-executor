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
  Search,
  X,
  Plus,
  Pencil,
  Trash2,
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
  DatasourceConnection,
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

/** 单个数据源插件：管理该插件下的多个数据源连接实例及各连接的浏览树 */
function DatasourceEntry({
  ds,
  refreshToken,
  onSelectItem,
  searchKeyword,
}: {
  ds: DatasourceDescription;
  refreshToken: number;
  onSelectItem: (ds: DatasourceDescription, item: DataItem, conn: DatasourceConnection) => void;
  searchKeyword?: string;
}) {
  const LL = getLL();
  const connections = useDatasourceStore((s) => s.connections);
  const dsConnections = useMemo(
    () => connections.filter((c) => c.datasource_id === ds.id),
    [connections, ds.id],
  );
  const connected = dsConnections.length > 0;

  // 正在编辑或新建的连接状态
  // null: 未打开表单; '__new__': 新建连接; UUID: 编辑已有连接
  const [activeFormMode, setActiveFormMode] = useState<string | null>(null);
  const [connName, setConnName] = useState('');
  const [configValues, setConfigValues] = useState<Record<string, string>>({});
  const [configError, setConfigError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // 每个连接的折叠/展开状态
  const [expandedConns, setExpandedConns] = useState<Record<string, boolean>>({});

  const isConnExpanded = useCallback(
    (connId: string) => {
      return expandedConns[connId] ?? true;
    },
    [expandedConns],
  );

  const toggleConnExpanded = useCallback((connId: string) => {
    setExpandedConns((prev) => ({
      ...prev,
      [connId]: !(prev[connId] ?? true),
    }));
  }, []);

  // 打开新建连接表单
  const handleStartAdd = useCallback(() => {
    setActiveFormMode('__new__');
    setConnName('');
    setConfigValues({});
    setConfigError(null);
  }, []);

  // 打开编辑已有连接表单
  const handleStartEdit = useCallback((conn: DatasourceConnection) => {
    setActiveFormMode(conn.id);
    setConnName(conn.name ?? '');
    const stringified: Record<string, string> = {};
    if (conn.config) {
      for (const [k, v] of Object.entries(conn.config)) {
        if (v != null) stringified[k] = String(v);
      }
    }
    setConfigValues(stringified);
    setConfigError(null);
  }, []);

  const handleCancelForm = useCallback(() => {
    setActiveFormMode(null);
    setConfigValues({});
    setConfigError(null);
    setConnName('');
  }, []);

  const handleConfigChange = useCallback((key: string, value: string) => {
    setConfigValues((prev) => ({ ...prev, [key]: value }));
    setConfigError(null);
  }, []);

  // 删除连接
  const handleDeleteConnection = useCallback(
    async (connId: string) => {
      await useDatasourceStore.getState().deleteConnection(connId);
      if (activeFormMode === connId) {
        setActiveFormMode(null);
      }
    },
    [activeFormMode],
  );

  // 保存连接
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

      const isNew = activeFormMode === '__new__';
      const editingConn = isNew ? null : dsConnections.find((c) => c.id === activeFormMode);
      const connId = isNew ? crypto.randomUUID() : (editingConn?.id ?? crypto.randomUUID());

      await useDatasourceStore.getState().saveConnection({
        id: connId,
        datasource_id: ds.id,
        name: connName.trim() || undefined,
        display_name: ds.display_name,
        config,
        created_at: editingConn?.created_at ?? new Date().toISOString(),
      });

      setActiveFormMode(null);
      setExpandedConns((prev) => ({ ...prev, [connId]: true }));
    } catch (e) {
      setConfigError(formatErrorMessage(e));
    } finally {
      setSaving(false);
    }
  }, [LL, activeFormMode, configValues, connName, ds, dsConnections]);

  // 顶层根资源
  const childIds = useMemo(
    () => new Set(ds.resources.map((r) => r.children).filter(Boolean)),
    [ds.resources],
  );
  const rootResources = useMemo(
    () => ds.resources.filter((r) => !childIds.has(r.id)),
    [ds.resources, childIds],
  );

  return (
    <div className="border-b border-border last:border-b-0">
      {/* 插件级头部行 */}
      <div className="flex items-center gap-2 px-3 py-1.5">
        <span className="flex-1 min-w-0 truncate text-xs font-semibold text-foreground">
          {ds.display_name}
        </span>
        {connected ? (
          <Badge variant="outline" className="text-emerald-600 shrink-0 text-[11px] h-5 px-1.5">
            {dsConnections.length > 1
              ? LL.datasource.connections_count({ count: dsConnections.length })
              : LL.datasource.connected()}
          </Badge>
        ) : (
          <Badge variant="outline" className="text-muted-foreground shrink-0 text-[11px] h-5 px-1.5">
            {LL.datasource.not_connected()}
          </Badge>
        )}
        {connected ? (
          <div className="flex items-center gap-1 shrink-0">
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-1.5 text-xs gap-1 text-muted-foreground hover:text-foreground"
              onClick={handleStartAdd}
              title={LL.datasource.add_connection()}
            >
              <Plus className="w-3 h-3" />
              <span className="hidden sm:inline">{LL.datasource.add_connection()}</span>
            </Button>
            {dsConnections.length === 1 && (
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-xs"
                onClick={() => handleStartEdit(dsConnections[0])}
              >
                {LL.datasource.edit()}
              </Button>
            )}
          </div>
        ) : (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2 text-xs shrink-0"
            onClick={handleStartAdd}
          >
            {LL.datasource.configure()}
          </Button>
        )}
      </div>

      {/* 新建/编辑配置表单 */}
      {activeFormMode != null && (
        <div className="px-3 py-2 bg-muted/20 border-t border-b border-border space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-foreground">
              {activeFormMode === '__new__' ? LL.datasource.add_connection() : LL.datasource.edit()}
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="w-4 h-4 p-0 text-muted-foreground hover:text-foreground"
              onClick={handleCancelForm}
            >
              <X className="w-3 h-3" />
            </Button>
          </div>

          {/* 连接名称输入 */}
          <label className="block space-y-1">
            <span className="text-xs text-muted-foreground">
              {LL.datasource.connection_name()}
            </span>
            <Input
              type="text"
              value={connName}
              placeholder={LL.datasource.connection_name_placeholder()}
              onChange={(e) => setConnName(e.target.value)}
              className="h-7 text-xs"
            />
          </label>

          {/* 动态 Schema 字段 */}
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
            {activeFormMode !== '__new__' && (
              <Button
                size="sm"
                variant="destructive"
                className="h-6 px-2 text-xs mr-auto"
                onClick={() => handleDeleteConnection(activeFormMode)}
                disabled={saving}
              >
                {LL.datasource.disconnect()}
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="h-6 px-2 text-xs ml-auto"
              onClick={handleCancelForm}
            >
              {LL.datasource.cancel()}
            </Button>
          </div>
        </div>
      )}

      {/* 连接列表与浏览树 */}
      {connected && (
        <div className="pb-1">
          {dsConnections.map((conn, index) => {
            const isSingle = dsConnections.length === 1;
            const expanded = isConnExpanded(conn.id);
            const title = conn.name || (isSingle ? ds.display_name : `${ds.display_name} (${index + 1})`);

            return (
              <div key={conn.id} className={cn(!isSingle && 'border-t border-border/50 first:border-t-0')}>
                {/* 多连接时显示独立的连接头部行；单连接且有自定义名称时也显示连接名 */}
                {(!isSingle || conn.name) && (
                  <div
                    className={cn(
                      'flex items-center gap-1.5 h-8 px-2.5 rounded-md cursor-pointer select-none text-xs transition-colors',
                      expanded ? 'bg-accent/20 hover:bg-accent/40 font-medium text-foreground' : 'hover:bg-accent/30 text-muted-foreground',
                    )}
                    onClick={() => toggleConnExpanded(conn.id)}
                  >
                    {!isSingle && (
                      <ChevronRight
                        className={cn(
                          'w-3.5 h-3.5 shrink-0 text-muted-foreground transition-transform',
                          expanded && 'rotate-90',
                        )}
                      />
                    )}
                    <span className="truncate flex-1">{title}</span>
                    <div className="flex items-center gap-1 shrink-0" onClick={(e) => e.stopPropagation()}>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="w-5 h-5 text-muted-foreground hover:text-foreground"
                        title={LL.datasource.edit()}
                        onClick={() => handleStartEdit(conn)}
                      >
                        <Pencil className="w-3 h-3" />
                      </Button>
                      {!isSingle && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="w-5 h-5 text-muted-foreground hover:text-destructive"
                          title={LL.datasource.delete_connection()}
                          onClick={() => handleDeleteConnection(conn.id)}
                        >
                          <Trash2 className="w-3 h-3" />
                        </Button>
                      )}
                    </div>
                  </div>
                )}

                {/* 展开的资源树 */}
                {expanded && (
                  <div className={cn(!isSingle && 'pl-2')}>
                    {rootResources.map((resource) => (
                      <ResourceLevel
                        key={resource.id}
                        ds={ds}
                        conn={conn}
                        resource={resource}
                        parentId={null}
                        refreshToken={refreshToken}
                        onSelectItem={onSelectItem}
                        searchKeyword={searchKeyword}
                      />
                    ))}
                  </div>
                )}
              </div>
            );
          })}
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

/**
 * 检查数据项是否匹配搜索词
 * 支持匹配：名称(name)、路径(path)、目录/分组(folderPath)、HTTP方法(method)、描述(summary)、extra 属性
 */
export function matchItem(
  item: DataItem,
  query: string,
  folderPath?: string,
): boolean {
  if (!query) return true;
  const kw = query.toLowerCase().trim();
  if (!kw) return true;

  // 1. 名字 (name)
  if (item.name.toLowerCase().includes(kw)) return true;

  // 2. 路径 / 端点 (path)
  if (item.path && item.path.toLowerCase().includes(kw)) return true;

  // 3. 目录 / 分组路径 (folderPath)
  if (folderPath && folderPath.toLowerCase().includes(kw)) return true;

  // 4. HTTP 方法 (method)
  if (item.method && item.method.toLowerCase().includes(kw)) return true;

  // 5. 简介描述 (summary)
  if (item.summary && item.summary.toLowerCase().includes(kw)) return true;

  // 6. 额外属性 (extra)
  if (
    item.extra &&
    item.extra.some(
      (e) =>
        e.label.toLowerCase().includes(kw) ||
        e.value.toLowerCase().includes(kw),
    )
  ) {
    return true;
  }

  // 7. raw 中的 group 信息兜底
  const raw = item.raw as Record<string, unknown> | undefined;
  if (raw) {
    const rawGroupName = raw.group_name ?? raw.groupName;
    if (
      typeof rawGroupName === 'string' &&
      rawGroupName.toLowerCase().includes(kw)
    ) {
      return true;
    }
    const rawGroupPath = raw.group_path ?? raw.groupPath ?? raw.group_name_path;
    if (Array.isArray(rawGroupPath)) {
      if (rawGroupPath.some((s) => String(s).toLowerCase().includes(kw))) {
        return true;
      }
    } else if (
      typeof rawGroupPath === 'string' &&
      rawGroupPath.toLowerCase().includes(kw)
    ) {
      return true;
    }
  }

  return false;
}

/**
 * 根据搜索词递归过滤分组树
 * - 若目录/分组名匹配，则保留该目录及其下全部子目录与条目
 * - 若目录名不匹配，则递归过滤子项，仅保留命中的子目录与条目，并更新 totalCount
 */
export function filterGroupTree(
  nodes: GroupTreeNode[],
  query: string,
): GroupTreeNode[] {
  const kw = query.toLowerCase().trim();
  if (!kw) return nodes;

  const filtered: GroupTreeNode[] = [];

  for (const node of nodes) {
    const isFolderMatch =
      node.name.toLowerCase().includes(kw) ||
      node.fullPath.toLowerCase().includes(kw);

    if (isFolderMatch) {
      // 目录本身匹配，保留该目录下所有子目录及条目
      filtered.push(node);
      continue;
    }

    // 递归过滤子目录
    const matchingChildren = filterGroupTree(node.children, kw);

    // 过滤当前目录中的条目
    const matchingItems = node.items.filter((item) =>
      matchItem(item, kw, node.fullPath),
    );

    if (matchingChildren.length > 0 || matchingItems.length > 0) {
      const childrenCount = matchingChildren.reduce(
        (sum, c) => sum + c.totalCount,
        0,
      );
      filtered.push({
        ...node,
        children: matchingChildren,
        items: matchingItems,
        totalCount: matchingItems.length + childrenCount,
      });
    }
  }

  return filtered;
}

/** 递归渲染分组目录项 */
function GroupFolderItem({
  node,
  ds,
  conn,
  depth,
  expanded,
  onToggle,
  onSelectItem,
  isSearching,
}: {
  node: GroupTreeNode;
  ds: DatasourceDescription;
  conn: DatasourceConnection;
  depth: number;
  expanded: Record<string, boolean>;
  onToggle: (path: string) => void;
  onSelectItem: (ds: DatasourceDescription, item: DataItem, conn: DatasourceConnection) => void;
  isSearching?: boolean;
}) {
  const isOpen =
    expanded[node.fullPath] !== undefined
      ? expanded[node.fullPath]
      : isSearching
      ? true
      : false;
  return (
    <div>
      <button
        type="button"
        className={cn(
          'w-full flex items-center gap-1.5 h-8.5 px-2.5 rounded-md text-left text-xs transition-colors select-none',
          isOpen
            ? 'bg-accent/40 hover:bg-accent/60 text-foreground font-semibold'
            : 'hover:bg-accent/50 text-foreground/85 font-medium',
        )}
        onClick={() => onToggle(node.fullPath)}
      >
        {isOpen ? (
          <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        )}
        {isOpen ? (
          <FolderOpen className="w-4 h-4 text-amber-500 shrink-0" />
        ) : (
          <Folder className="w-4 h-4 text-amber-500 shrink-0" />
        )}
        <span className="truncate flex-1">{node.name}</span>
        <span className="text-[11px] text-muted-foreground/75 font-mono shrink-0 tabular-nums">
          {`(${node.totalCount})`}
        </span>
      </button>
      {isOpen && (
        <div className="pl-3.5 ml-2 border-l border-border/40 space-y-0.5 mt-0.5">
          {node.children.map((child) => (
            <GroupFolderItem
              key={child.id}
              node={child}
              ds={ds}
              conn={conn}
              depth={depth + 1}
              expanded={expanded}
              onToggle={onToggle}
              onSelectItem={onSelectItem}
              isSearching={isSearching}
            />
          ))}
          {node.items.map((item) => (
            <ItemRow key={item.id} ds={ds} conn={conn} item={item} onSelect={onSelectItem} />
          ))}
        </div>
      )}
    </div>
  );
}
function ResourceLevel({
  ds,
  conn,
  resource,
  parentId,
  refreshToken,
  onSelectItem,
  searchKeyword,
}: {
  ds: DatasourceDescription;
  conn: DatasourceConnection;
  resource: DatasourceResource;
  parentId: string | null;
  refreshToken: number;
  onSelectItem: (ds: DatasourceDescription, item: DataItem, conn: DatasourceConnection) => void;
  searchKeyword?: string;
}) {
  const LL = getLL();
  const cacheKey = `${conn.id}/${resource.id}/${parentId ?? ''}`;
  const legacyCacheKey = `${ds.id}/${resource.id}/${parentId ?? ''}`;
  const cached = useDatasourceStore(
    (s) => s.browseCache[cacheKey] ?? s.browseCache[legacyCacheKey],
  );
  const browseCache = useDatasourceStore((s) => s.browseCache);
  const loading = useDatasourceStore((s) => s.loading);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [groupExpanded, setGroupExpanded] = useState<Record<string, boolean>>({});

  // 首次挂载 / 刷新时拉取本层条目（缓存命中直接复用）。
  useEffect(() => {
    if (cached) return;
    useDatasourceStore
      .getState()
      .browse(ds.id, resource.id, parentId, false, conn.id)
      .catch(() => {
        /* 错误已写入 store.error，面板统一展示 */
      });
  }, [ds.id, conn.id, resource.id, parentId, refreshToken]); // eslint-disable-line react-hooks/exhaustive-deps
  const items = cached ?? [];
  const isSearching = Boolean(searchKeyword?.trim());

  // 自动根据 item 中的 group_name / extra 提取多级嵌套分组树
  const groupTree = useMemo(() => {
    if (resource.children != null) return null;
    return buildGroupTree(items, LL.datasource.default_group());
  }, [items, resource.children, LL]);

  // 根据搜索关键字过滤分组树
  const displayedGroupTree = useMemo(() => {
    if (!groupTree) return null;
    if (!isSearching) return groupTree;
    return filterGroupTree(groupTree, searchKeyword!);
  }, [groupTree, isSearching, searchKeyword]);

  // 平铺项搜索过滤
  const displayedFlatItems = useMemo(() => {
    if (groupTree != null || resource.children != null) return items;
    if (!isSearching) return items;
    return items.filter((item) => matchItem(item, searchKeyword!));
  }, [items, groupTree, resource.children, isSearching, searchKeyword]);

  const handleToggleGroup = useCallback((groupPath: string) => {
    setGroupExpanded((prev) => ({
      ...prev,
      [groupPath]: !(prev[groupPath] !== undefined ? prev[groupPath] : isSearching ? true : false),
    }));
  }, [isSearching]);

  return (
    <div>
      {/* 资源层级头（仅顶层展示；子级作为分组内容渲染） */}
      {parentId === null && (
        <div className="px-2.5 pt-2 pb-1 text-xs font-semibold text-muted-foreground/80 uppercase tracking-wider flex items-center gap-1">
          <span className="truncate">{resource.label}</span>
          {loading && !cached && <Loader2 className="w-3 h-3 animate-spin shrink-0" />}
        </div>
      )}
      {/* 叶子节点按多级嵌套分组展示 */}
      {displayedGroupTree ? (
        displayedGroupTree.map((node) => (
          <GroupFolderItem
            key={node.id}
            node={node}
            ds={ds}
            conn={conn}
            depth={0}
            expanded={groupExpanded}
            onToggle={handleToggleGroup}
            onSelectItem={onSelectItem}
            isSearching={isSearching}
          />
        ))
      ) : resource.children != null ? (
        items.map((item) => {
          const childCacheKey = `${conn.id}/${resource.children}/${item.id}`;
          const legacyChildCacheKey = `${ds.id}/${resource.children}/${item.id}`;
          const cachedChildren = browseCache[childCacheKey] ?? browseCache[legacyChildCacheKey];
          const hasMatchingChildren = cachedChildren?.some((child) =>
            matchItem(child, searchKeyword ?? '', item.name),
          );
          const isItemMatch =
            !isSearching ||
            matchItem(item, searchKeyword ?? '') ||
            Boolean(hasMatchingChildren);
          if (!isItemMatch) return null;

          const isExpanded =
            expanded[item.id] !== undefined
              ? expanded[item.id]
              : isSearching
              ? true
              : false;
          return (
            <div key={item.id}>
              <button
                type="button"
                title={item.extra.length > 0 ? `${item.name}\n${item.extra.map((kv) => `${kv.label}: ${kv.value}`).join(' · ')}` : item.name}
                className={cn(
                  'w-full flex items-center gap-1.5 h-8.5 px-2.5 rounded-md text-left text-xs transition-colors select-none',
                  isExpanded
                    ? 'bg-accent/40 hover:bg-accent/60 text-foreground font-semibold'
                    : 'hover:bg-accent/50 text-foreground/85 font-medium',
                )}
                onClick={() => {
                  const next = !isExpanded;
                  setExpanded((prev) => ({ ...prev, [item.id]: next }));
                  if (next) {
                    useDatasourceStore
                      .getState()
                      .browse(ds.id, resource.children!, item.id, false, conn.id)
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
                  <FolderOpen className="w-4 h-4 text-primary shrink-0" />
                ) : (
                  <Folder className="w-4 h-4 text-primary shrink-0" />
                )}
                <span className="truncate flex-1">{item.name}</span>
              </button>
              {isExpanded && (
                <div className="pl-3.5 ml-2 border-l border-border/40 space-y-0.5 mt-0.5">
                  <ResourceLevel
                    ds={ds}
                    conn={conn}
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
                    searchKeyword={searchKeyword}
                  />
                </div>
              )}
            </div>
          );
        })
      ) : (
        displayedFlatItems.map((item) => (
          <ItemRow key={item.id} ds={ds} conn={conn} item={item} onSelect={onSelectItem} />
        ))
      )}

      {!loading && cached && (
        isSearching ? (
          (displayedGroupTree ? displayedGroupTree.length === 0 : displayedFlatItems.length === 0) && (
            <div className="px-3 py-2 text-xs text-muted-foreground">
              {LL.datasource.no_search_results()}
            </div>
          )
        ) : (
          items.length === 0 && (
            <div className="px-3 py-1 text-xs text-muted-foreground">
              {LL.datasource.empty()}
            </div>
          )
        )
      )}
    </div>
  );
}

/** 叶子条目行：协议徽标 + method + 名称；可拖拽，点击打开详情弹窗，鼠标 hover 展示 url 及详细信息 */
function ItemRow({
  ds,
  conn,
  item,
  onSelect,
}: {
  ds: DatasourceDescription;
  conn: DatasourceConnection;
  item: DataItem;
  onSelect: (ds: DatasourceDescription, item: DataItem, conn: DatasourceConnection) => void;
}) {
  const LL = getLL();
  const Icon = PROTOCOL_ICON_MAP[item.protocol] ?? Network;

  const handleDragStart = useCallback(
    (e: React.DragEvent) => {
      e.dataTransfer.setData(
        'application/reactflow/datasource',
        JSON.stringify({
          datasource_id: ds.id,
          connection_id: conn.id,
          item_ids: [item.id],
        }),
      );
      e.dataTransfer.effectAllowed = 'move';
    },
    [ds.id, conn.id, item.id],
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
      onClick={() => onSelect(ds, item, conn)}
      title={tooltipText}
      className="group flex items-center gap-2 h-8 px-2.5 cursor-pointer hover:bg-accent/60 rounded-md select-none transition-colors"
    >
      <Icon className={cn('w-4 h-4 shrink-0', PROTOCOL_COLOR_MAP[item.protocol] ?? 'text-slate-500')} />
      {item.method && (
        <Badge variant="secondary" className="h-4.5 px-1.5 text-[10px] font-mono shrink-0 leading-none">
          {item.method}
        </Badge>
      )}
      <span className="min-w-0 truncate text-xs text-foreground/90 group-hover:text-foreground flex-1">{item.name}</span>
      <Button
        variant="ghost"
        size="icon"
        className="h-6 w-6 shrink-0 text-muted-foreground hover:text-foreground opacity-0 group-hover:opacity-100 transition-opacity"
        onClick={(e) => {
          e.stopPropagation();
          onSelect(ds, item, conn);
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
  const [searchKeyword, setSearchKeyword] = useState('');

  // 弹窗状态：选中的数据源与条目
  const [selectedItemState, setSelectedItemState] = useState<{
    ds: DatasourceDescription;
    item: DataItem;
    conn: DatasourceConnection;
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

  const handleSelectItem = useCallback(
    (ds: DatasourceDescription, item: DataItem, conn: DatasourceConnection) => {
      setSelectedItemState({ ds, item, conn });
    },
    [],
  );

  return (
    <div className="flex flex-col h-full min-h-0" data-testid="datasource-panel">
      {/* 搜索栏 + 刷新按钮 */}
      <div className="flex items-center gap-1.5 px-3 py-1.5 border-b border-border">
        <div className="relative flex-1">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
          <input
            type="text"
            value={searchKeyword}
            onChange={(e) => setSearchKeyword(e.target.value)}
            placeholder={LL.datasource.search_placeholder()}
            className="w-full h-7 pl-7 pr-6 text-xs rounded-md border border-border bg-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            data-testid="datasource-search-input"
          />
          {searchKeyword && (
            <button
              onClick={() => setSearchKeyword('')}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 text-muted-foreground hover:text-foreground rounded"
              title="Clear"
              data-testid="datasource-search-clear"
            >
              <X className="w-3 h-3" />
            </button>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 rounded-md hover:bg-accent shrink-0 text-muted-foreground hover:text-foreground"
          title={LL.datasource.refresh()}
          onClick={handleRefresh}
        >
          <RefreshCw className={cn('w-3.5 h-3.5', loading && 'animate-spin')} />
        </Button>
      </div>

      {/* 提示条（拖拽提示） */}
      <div className="px-3 py-1 border-b border-border flex items-center justify-between text-[11px] text-muted-foreground">
        <span>{LL.datasource.drag_hint()}</span>
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
            searchKeyword={searchKeyword}
          />
        ))}
      </div>
      <DatasourceItemDetailDialog
        open={Boolean(selectedItemState)}
        onOpenChange={(open) => {
          if (!open) setSelectedItemState(null);
        }}
        datasource={selectedItemState?.ds ?? null}
        item={selectedItemState?.item ?? null}
        connection={selectedItemState?.conn ?? null}
      />
    </div>
  );
}
