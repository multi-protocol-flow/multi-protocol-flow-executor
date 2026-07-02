/**
 * 变量面板组件
 * 显示所有变量，按来源节点分组
 */
import { useState, useMemo, useCallback } from 'react';
import { Search, ChevronDown, ChevronRight, Variable as VariableIcon } from 'lucide-react';
import { Input } from '@/components/ui/input';
import type { Flow, Node } from '@/types';
import type { Variable } from '@/types/variable';
import { isVariableExtractorConfig } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';

interface VariablePanelProps {
  /** 当前流程 */
  flow: Flow;
  /** 所有节点 */
  nodes: Node[];
  /** 是否显示标题栏 */
  showHeader?: boolean;
}

export interface VariableGroup {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  variables: Variable[];
}

/**
 * 从流程中提取所有变量并按节点分组
 */
export function extractVariableGroups(flow: Flow): VariableGroup[] {
  const ll = getLL().panels.variablePanel;
  const groups: VariableGroup[] = [];
  const nodeMap = new Map<string, Node>();

  // 建立节点映射
  flow.nodes.forEach(node => {
    nodeMap.set(node.uuid, node);
  });

  // 遍历所有节点提取变量
  flow.nodes.forEach(node => {
    const variables: Variable[] = [];

    // 变量提取节点：提取 output_mappings
    if (isVariableExtractorConfig(node.data) && node.data.output_mappings) {
      node.data.output_mappings.forEach(om => {
        variables.push({
          name: om.target,
          value: `{{${om.source}}}`,
          sourceNodeUuid: node.uuid,
          description: om.description,
        });
      });
    }

    // 只有有变量的节点才加入分组
    if (variables.length > 0) {
      groups.push({
        nodeUuid: node.uuid,
        nodeName: node.name,
        nodeType: node.type,
        variables,
      });
    }
  });

    // 流程级别的 initial_variables
    if (flow.initial_variables && Object.keys(flow.initial_variables).length > 0) {
      const flowVars: Variable[] = Object.entries(flow.initial_variables).map(([key, value]) => ({
        name: key,
        value,
        sourceNodeUuid: '__flow__',
        description: ll.flowInitialVar(),
      }));
      groups.push({
        nodeUuid: '__flow__',
        nodeName: ll.flowVariables(),
        nodeType: 'flow',
        variables: flowVars,
      });
    }

// 激活环境的变量 — 仅在明确选择环境时才显示，避免与流程变量重复
    if (flow.active_environment && flow.environments) {
      const env = flow.environments.find((e) => e.name === flow.active_environment);
      if (env && Object.keys(env.variables).length > 0) {
        const envVars: Variable[] = Object.entries(env.variables).map(([key, value]) => ({
          name: key,
          value,
          sourceNodeUuid: '__environment__',
          description: ll.environmentVariables({ name: env.name }),
        }));
        groups.push({
          nodeUuid: '__environment__',
          nodeName: ll.environmentVariables({ name: env.name }),
          nodeType: 'environment',
          variables: envVars,
        });
      }
    }

    return groups;
}



/**
 * 格式化变量值用于显示
 */
function formatValue(value: unknown): string {
  if (value === undefined) return 'undefined';
  if (value === null) return 'null';
  if (typeof value === 'string') {
    // 长字符串截断
    if (value.length > 50) {
      return `${value.slice(0, 50)}...`;
    }
    return value;
  }
  if (typeof value === 'object') {
    try {
      const str = JSON.stringify(value);
      if (str.length > 50) {
        return `${str.slice(0, 50)}...`;
      }
      return str;
    } catch {
      return '[Object]';
    }
  }
  return String(value);
}

/**
 * 获取变量值的类型标签
 */
function getValueType(value: unknown): string {
  if (value === null) return 'null';
  if (value === undefined) return 'undefined';
  if (Array.isArray(value)) return 'array';
  return typeof value;
}

export function VariablePanel({ flow, nodes: _nodes, showHeader = true }: VariablePanelProps) {
  const ll = getLL().panels.variablePanel;
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set());

  // 提取变量分组
  const variableGroups = useMemo(() => extractVariableGroups(flow), [flow]);

  // 过滤变量
  const filteredGroups = useMemo(() => {
    if (!searchQuery.trim()) {
      return variableGroups;
    }

    const query = searchQuery.toLowerCase();
    return variableGroups.map(group => {
      const filteredVariables = group.variables.filter(
        v =>
          v.name.toLowerCase().includes(query) ||
          (v.description && v.description.toLowerCase().includes(query)) ||
          String(v.value).toLowerCase().includes(query)
      );

      if (filteredVariables.length === 0) {
        return null;
      }

      return {
        ...group,
        variables: filteredVariables,
      };
    }).filter((g): g is VariableGroup => g !== null);
  }, [variableGroups, searchQuery]);

  // 切换分组展开状态
  const toggleGroup = useCallback((nodeUuid: string) => {
    setExpandedGroups(prev => {
      const next = new Set(prev);
      if (next.has(nodeUuid)) {
        next.delete(nodeUuid);
      } else {
        next.add(nodeUuid);
      }
      return next;
    });
  }, []);

  // 展开所有分组
  const expandAll = useCallback(() => {
    setExpandedGroups(new Set(variableGroups.map(g => g.nodeUuid)));
  }, [variableGroups]);

  // 折叠所有分组
  const collapseAll = useCallback(() => {
    setExpandedGroups(new Set());
  }, []);

  // 计算总变量数
  const totalVariables = useMemo(
    () => variableGroups.reduce((sum, g) => sum + g.variables.length, 0),
    [variableGroups]
  );

  return (
    <div className="flex h-full flex-col bg-muted">
      {/* 标题栏 */}
      {showHeader && (
        <div className="shrink-0 border-b border-border bg-background/80 backdrop-blur-sm px-4 py-3">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <VariableIcon className="h-4 w-4 text-violet-600" />
              <h3 className="text-sm font-semibold text-foreground">{ll.title()}</h3>
            </div>
            <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
              {ll.variableCount({ count: totalVariables })}
            </span>
          </div>

          {/* 搜索框 */}
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              type="text"
              placeholder={ll.searchPlaceholder()}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 pl-8 text-xs bg-background border-border focus:border-violet-400 focus:ring-violet-400/20"
            />
          </div>

          {/* 展开/折叠按钮 */}
          <div className="flex items-center gap-2 mt-2">
            <button
              type="button"
              onClick={expandAll}
              className="text-xs text-muted-foreground hover:text-violet-600 transition-colors"
          >
            {ll.expandAll()}
          </button>
          <span className="text-muted-foreground">|</span>
          <button
            type="button"
            onClick={collapseAll}
            className="text-xs text-muted-foreground hover:text-violet-600 transition-colors"
          >
            {ll.collapseAll()}
          </button>
        </div>
      </div>
      )}

      {/* 变量列表 */}
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden scrollbar-thin p-2">
        {filteredGroups.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center py-12">
            <VariableIcon className="h-10 w-10 text-muted-foreground mb-3" />
            <p className="text-sm text-muted-foreground">
              {searchQuery ? ll.noMatch() : ll.empty()}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              {searchQuery ? ll.tryOtherSearch() : ll.addEntryVarHint()}
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {filteredGroups.map((group) => {
              const isExpanded = expandedGroups.has(group.nodeUuid);

              return (
                <div
                  key={group.nodeUuid}
                  className="rounded-lg border border-border bg-background shadow-sm overflow-hidden"
                >
                  {/* 分组标题 */}
                  <button
                    type="button"
                    onClick={() => toggleGroup(group.nodeUuid)}
                    className="w-full flex items-center gap-2 px-3 py-2.5 text-left hover:bg-muted transition-colors"
                  >
                    {isExpanded ? (
                      <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
                    )}
                    <span className="text-sm font-medium text-foreground flex-1 truncate">
                      {group.nodeName}
                    </span>
                    <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
                      {group.variables.length}
                    </span>
                  </button>

                  {/* 变量列表 */}
                  {isExpanded && (
                    <div className="border-t border-border bg-muted">
                      {group.variables.map((variable, index) => (
                        <div
                          key={`${variable.name}-${index}`}
                          className="px-3 py-2 border-b border-border/80 last:border-b-0 hover:bg-violet-50/50 dark:hover:bg-violet-950/30 transition-colors"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="min-w-0 flex-1">
                              {/* 变量名 */}
                              <div className="flex items-center gap-2">
                                <code className="text-xs font-mono text-violet-700 dark:text-violet-300 bg-violet-100/60 dark:bg-violet-950/50 px-1.5 py-0.5 rounded">
                                  {variable.name}
                                </code>
                                <span className="text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded uppercase">
                                  {getValueType(variable.value)}
                                </span>
                              </div>

                              {/* 变量值 */}
                              <div className="mt-1.5">
                                <code className="text-xs font-mono text-muted-foreground break-all line-clamp-2">
                                  {formatValue(variable.value)}
                                </code>
                              </div>

                              {/* 变量说明 */}
                              {variable.description && (
                                <p className="mt-1 text-xs text-muted-foreground line-clamp-1">
                                  {variable.description}
                                </p>
                              )}
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
