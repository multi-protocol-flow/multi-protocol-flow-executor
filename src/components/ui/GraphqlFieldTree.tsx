/**
 * GraphQL Schema 字段树组件
 *
 * 可复用的树形浏览器，用于浏览和勾选 GraphQL Schema 字段，
 * 支持多选级联、展开/折叠、类型标签、必填标记、描述展示和查询生成。
 */
import { useCallback, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import {
  generateQueryFromSelection,
  type FieldNode,
} from '@/lib/graphqlSchema';

// ============================================================================
// 类型定义
// ============================================================================

export interface GraphqlFieldTreeProps {
  /** 操作类型（用于逻辑判断：query/mutation/subscription） */
  title: string;
  /** 显示用标题，不传则使用 title */
  displayTitle?: string;
  /** 根字段列表 */
  fields: FieldNode[];
  /** 查询生成回调，参数为生成的 GraphQL 查询字符串 */
  onGenerate: (query: string) => void;
  /** 最大显示高度，默认 16rem */
  maxHeight?: string;
  /** 额外 className */
  className?: string;
}

// ============================================================================
// 工具函数
// ============================================================================

/** 递归设置所有子节点的 checked 状态 */
function setAllChildren(node: FieldNode, checked: boolean): FieldNode {
  return {
    ...node,
    checked,
    children: node.children.map((c) => setAllChildren(c, checked)),
  };
}

/** 按路径索引更新节点（避免同名冲突） */
function setAtPath(
  nodes: FieldNode[],
  path: number[],
  fn: (n: FieldNode) => FieldNode,
): FieldNode[] {
  return nodes.map((n, i) => {
    if (path.length === 0) return fn(n);
    if (i === path[0]) {
      if (path.length === 1) return fn(n);
      return { ...n, children: setAtPath(n.children, path.slice(1), fn) };
    }
    return n;
  });
}

/** 判断是否有任何节点被勾选 */
function hasAnyChecked(nodes: FieldNode[]): boolean {
  return nodes.some(
    (n) => n.checked || (n.children.length > 0 && hasAnyChecked(n.children)),
  );
}

/** 收集所有含有已勾选后代的根级节点（用于查询生成，支持子节点勾选而父节点未勾选） */
function collectRootsWithChecked(nodes: FieldNode[]): FieldNode[] {
  return nodes.filter((n) => n.checked || (n.children.length > 0 && hasAnyChecked(n.children)));
}

// ============================================================================
// 主组件
// ============================================================================

export function GraphqlFieldTree({
  title,
  displayTitle,
  fields,
  onGenerate,
  maxHeight = '16rem',
  className,
}: GraphqlFieldTreeProps) {
  const [tree, setTree] = useState<FieldNode[]>(() =>
    fields.map((f) => ({ ...f, checked: false, expanded: false })),
  );

  const handleCheck = useCallback((path: number[], checked: boolean) => {
    setTree((prev) => setAtPath(prev, path, (n) => setAllChildren(n, checked)));
  }, []);

  const handleToggle = useCallback((path: number[]) => {
    setTree((prev) =>
      setAtPath(prev, path, (n) => ({ ...n, expanded: !n.expanded })),
    );
  }, []);

  const hasChecked = hasAnyChecked(tree);

  const handleGenerate = () => {
    if (!hasChecked) return;

    const operation = title.toLowerCase() as 'query' | 'mutation' | 'subscription';
    // 收集所有含已勾选后代的根节点（包括根节点本身勾选和仅子节点勾选的场景）
    const checkedRoots = collectRootsWithChecked(tree);
    const rootFieldName = checkedRoots[0]?.name ?? 'Query';
    // 传入全部根节点，renderFieldSelection 会自动过滤未勾选分支
    const query = generateQueryFromSelection(tree, operation, rootFieldName);
    if (query) {
      onGenerate(query);
    }
  };

  const isMutation = title.toLowerCase() === 'mutation';

  return (
    <div
      className={cn(
        'rounded-md border border-indigo-200/60 dark:border-indigo-800/40 bg-white dark:bg-gray-900',
        className,
      )}
    >
      {/* 操作类型头 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-indigo-100 dark:border-indigo-800/30 bg-gradient-to-r from-indigo-50/50 dark:from-indigo-950/30">
        <span className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider">
          <span
            className={cn(
              'px-1.5 py-0.5 rounded',
              isMutation
                ? 'bg-amber-100 text-amber-700 dark:bg-amber-900/50 dark:text-amber-300'
                : 'bg-blue-100 text-blue-700 dark:bg-blue-900/50 dark:text-blue-300',
            )}
          >
            {displayTitle || title}
          </span>
          <span className="text-muted-foreground font-normal normal-case text-[11px]">
            {getLL().panels.graphql_field.field_count({ count: tree.length })}
          </span>
        </span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!hasChecked}
          onClick={handleGenerate}
          className="h-7 text-[11px] px-2.5 gap-1"
        >
          <SendIcon />
          {getLL().panels.graphql_field.generate_query()}
        </Button>
      </div>

      {/* 字段列表 */}
      <div className="overflow-y-auto p-1.5 space-y-0.5" style={{ maxHeight }}>
        {tree.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center py-4">
            {getLL().panels.graphql_field.no_fields()}
          </p>
        ) : (
          tree.map((node, idx) => (
            <FieldTreeNode
              key={`${node.name}-${idx}`}
              node={node}
              depth={0}
              path={[idx]}
              onCheck={handleCheck}
              onToggle={handleToggle}
            />
          ))
        )}
      </div>
    </div>
  );
}

// ============================================================================
// 子组件：字段树节点
// ============================================================================

interface FieldTreeNodeProps {
  node: FieldNode;
  depth: number;
  path: number[];
  onCheck: (path: number[], checked: boolean) => void;
  onToggle: (path: number[]) => void;
}

function FieldTreeNode({
  node,
  depth,
  path,
  onCheck,
  onToggle,
}: FieldTreeNodeProps) {
  const [showDesc, setShowDesc] = useState(false);
  const hasChildren = node.children.length > 0;
  const partialChecked =
    hasChildren &&
    node.children.some((c) => c.checked) &&
    !node.children.every((c) => c.checked);

  return (
    <div>
      <div
        className="flex items-start gap-1.5 px-1 py-1 rounded hover:bg-indigo-50/50 dark:hover:bg-indigo-950/30 transition-colors group"
        style={{ paddingLeft: `${8 + depth * 16}px` }}
      >
        {/* 展开/折叠 */}
        {hasChildren ? (
          <button
            type="button"
            onClick={() => onToggle(path)}
            className="mt-0.5 shrink-0 w-4 h-4 flex items-center justify-center text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 transition-transform"
          >
            <svg
              className={cn(
                'w-3 h-3 transition-transform',
                node.expanded ? 'rotate-90' : '',
              )}
              viewBox="0 0 16 16"
              fill="currentColor"
            >
              <path d="M6 4l4 4-4 4" />
            </svg>
          </button>
        ) : (
          <span className="w-4 shrink-0" />
        )}

        {/* 勾选框 */}
        <label className="flex items-center gap-1.5 cursor-pointer min-w-0 flex-1">
          <Checkbox
            checked={node.checked}
            indeterminate={partialChecked}
            onCheckedChange={(checked) => onCheck(path, checked)}
            className="shrink-0 w-3.5 h-3.5"
          />

          {/* 字段名 */}
          <span className="text-xs font-medium text-foreground truncate">
            {node.name}
            {node.isRequired && (
              <span className="text-red-500 dark:text-red-400 ml-0.5">*</span>
            )}
          </span>

          {/* 类型标签 */}
          <span className="shrink-0 text-[10px] font-mono px-1.5 py-0.5 rounded bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 leading-none">
            {node.typeName}
          </span>

          {/* 描述切换 */}
          {node.description && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setShowDesc(!showDesc);
              }}
              className="shrink-0 ml-auto opacity-0 group-hover:opacity-100 transition-opacity text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
              title={node.description}
            >
              <InfoIcon />
            </button>
          )}

          {/* 参数标记 */}
          {node.args.length > 0 && (
            <span className="shrink-0 text-[9px] text-orange-500 dark:text-orange-400 font-mono">
              ({node.args.filter((a) => a.isRequired).length} req)
            </span>
          )}
        </label>
      </div>

      {/* 内联描述 */}
      {showDesc && node.description && (
        <div
          className="ml-9 mr-2 mb-1 px-2 py-1 rounded bg-indigo-50/80 dark:bg-indigo-950/40 text-[11px] text-muted-foreground leading-relaxed"
          style={{ paddingLeft: `${8 + depth * 16 + 24}px` }}
        >
          {node.description}
          {node.args.length > 0 && (
            <div className="mt-1 pt-1 border-t border-indigo-100/50 dark:border-indigo-800/30 space-y-0.5">
              {node.args.map((arg) => (
                <div
                  key={arg.name}
                  className="flex flex-wrap gap-x-1.5 text-[10px]"
                >
                  <span className="font-mono text-indigo-600 dark:text-indigo-400">
                    {arg.name}
                  </span>
                  <span className="font-mono text-gray-400 dark:text-gray-500">
                    {arg.typeName}
                  </span>
                  {arg.isRequired && (
                    <span className="text-red-500 dark:text-red-400">
                      {getLL().panels.graphql_field.required()}
                    </span>
                  )}
                  {arg.defaultValue !== null && (
                    <span className="text-gray-400 dark:text-gray-500">
                      ={arg.defaultValue}
                    </span>
                  )}
                  {arg.description && (
                    <span className="text-gray-400 dark:text-gray-500">
                      — {arg.description}
                    </span>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* 子字段 */}
      {hasChildren && node.expanded && (
        <div>
          {node.children.map((child, idx) => (
            <FieldTreeNode
              key={`${child.name}-${idx}`}
              node={child}
              depth={depth + 1}
              path={[...path, idx]}
              onCheck={onCheck}
              onToggle={onToggle}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 内联 SVG 图标（避免引入 lucide-react 依赖）
// ============================================================================

function SendIcon() {
  return (
    <svg
      className="h-3 w-3"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="22" y1="2" x2="11" y2="13" />
      <polygon points="22 2 15 22 11 13 2 9 22 2" />
    </svg>
  );
}

function InfoIcon() {
  return (
    <svg className="w-3.5 h-3.5" viewBox="0 0 16 16" fill="currentColor">
      <path d="M8 1.5a6.5 6.5 0 100 13 6.5 6.5 0 000-13zM7.25 6h1.5v4.5h-1.5V6zm0-2.5h1.5V5h-1.5V3.5z" />
    </svg>
  );
}