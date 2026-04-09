/**
 * 变量面板组件
 * 显示所有变量，按来源节点分组
 */
import { useState, useMemo, useCallback } from 'react';
import { Search, ChevronDown, ChevronRight, Variable as VariableIcon } from 'lucide-react';
import { Input } from '@/components/ui/input';
import type { Flow, Node } from '@/types';
import type { Variable } from '@/types/variable';
import { isEntryConfig, isHttpConfig, isWsConnectConfig, isWsSendCollectConfig } from '@/types/node';

interface VariablePanelProps {
  /** 当前流程 */
  flow: Flow;
  /** 所有节点 */
  nodes: Node[];
  /** 是否显示标题栏 */
  showHeader?: boolean;
}

interface VariableGroup {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  variables: Variable[];
}

/**
 * 从流程中提取所有变量并按节点分组
 */
function extractVariableGroups(flow: Flow): VariableGroup[] {
  const groups: VariableGroup[] = [];
  const nodeMap = new Map<string, Node>();
  
  // 建立节点映射
  flow.nodes.forEach(node => {
    nodeMap.set(node.uuid, node);
  });

  // 遍历所有节点提取变量
  flow.nodes.forEach(node => {
    const variables: Variable[] = [];

    // Entry 节点：提取 initial_variables
    if (isEntryConfig(node.data) && node.data.initial_variables) {
      node.data.initial_variables.forEach(iv => {
        variables.push({
          name: iv.name,
          value: iv.value,
          sourceNodeUuid: node.uuid,
          description: iv.description,
        });
      });
    }

    // HTTP 节点：提取 output_mappings（模拟已执行的变量）
    if (isHttpConfig(node.data) && node.data.output_mappings) {
      node.data.output_mappings.forEach(om => {
        variables.push({
          name: om.target,
          value: `{{${om.source}}}`, // 显示引用路径
          sourceNodeUuid: node.uuid,
          description: om.description,
        });
      });
    }

    // WebSocket Connect 节点：提取 output_mappings
    if (isWsConnectConfig(node.data) && node.data.output_mappings) {
      node.data.output_mappings.forEach(om => {
        variables.push({
          name: om.target,
          value: `{{${om.source}}}`, // 显示引用路径
          sourceNodeUuid: node.uuid,
          description: om.description,
        });
      });
    }

    // WebSocket Send+Collect 节点：提取 output_mappings
    if (isWsSendCollectConfig(node.data) && node.data.output_mappings) {
      node.data.output_mappings.forEach(om => {
        variables.push({
          name: om.target,
          value: `{{${om.source}}}`, // 显示引用路径
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
    <div className="flex h-full flex-col bg-gradient-to-br from-slate-50 to-slate-100/50">
      {/* 标题栏 */}
      {showHeader && (
        <div className="shrink-0 border-b border-slate-200/80 bg-white/80 backdrop-blur-sm px-4 py-3">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <VariableIcon className="h-4 w-4 text-violet-600" />
              <h3 className="text-sm font-semibold text-slate-800">变量面板</h3>
            </div>
            <span className="text-xs text-slate-500 bg-slate-100 px-2 py-0.5 rounded-full">
              {totalVariables} 个变量
            </span>
          </div>

          {/* 搜索框 */}
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400" />
            <Input
              type="text"
              placeholder="搜索变量..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 pl-8 text-xs bg-white border-slate-200 focus:border-violet-400 focus:ring-violet-400/20"
            />
          </div>

          {/* 展开/折叠按钮 */}
          <div className="flex items-center gap-2 mt-2">
            <button
              type="button"
              onClick={expandAll}
              className="text-xs text-slate-500 hover:text-violet-600 transition-colors"
          >
            全部展开
          </button>
          <span className="text-slate-300">|</span>
          <button
            type="button"
            onClick={collapseAll}
            className="text-xs text-slate-500 hover:text-violet-600 transition-colors"
          >
            全部折叠
          </button>
        </div>
      </div>
      )}

      {/* 变量列表 */}
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden scrollbar-thin p-2">
        {filteredGroups.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center py-12">
            <VariableIcon className="h-10 w-10 text-slate-300 mb-3" />
            <p className="text-sm text-slate-500">
              {searchQuery ? '未找到匹配的变量' : '暂无变量'}
            </p>
            <p className="text-xs text-slate-400 mt-1">
              {searchQuery ? '尝试其他搜索词' : '在 Entry 节点添加初始变量'}
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {filteredGroups.map((group) => {
              const isExpanded = expandedGroups.has(group.nodeUuid);
              
              return (
                <div
                  key={group.nodeUuid}
                  className="rounded-lg border border-slate-200/80 bg-white shadow-sm overflow-hidden"
                >
                  {/* 分组标题 */}
                  <button
                    type="button"
                    onClick={() => toggleGroup(group.nodeUuid)}
                    className="w-full flex items-center gap-2 px-3 py-2.5 text-left hover:bg-slate-50/50 transition-colors"
                  >
                    {isExpanded ? (
                      <ChevronDown className="h-4 w-4 text-slate-400 shrink-0" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-slate-400 shrink-0" />
                    )}
                    <span className="text-sm font-medium text-slate-700 flex-1 truncate">
                      {group.nodeName}
                    </span>
                    <span className="text-xs text-slate-400 bg-slate-100 px-1.5 py-0.5 rounded">
                      {group.variables.length}
                    </span>
                  </button>

                  {/* 变量列表 */}
                  {isExpanded && (
                    <div className="border-t border-slate-100 bg-slate-50/50">
                      {group.variables.map((variable, index) => (
                        <div
                          key={`${variable.name}-${index}`}
                          className="px-3 py-2 border-b border-slate-100/80 last:border-b-0 hover:bg-violet-50/50 transition-colors"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="min-w-0 flex-1">
                              {/* 变量名 */}
                              <div className="flex items-center gap-2">
                                <code className="text-xs font-mono text-violet-700 bg-violet-100/60 px-1.5 py-0.5 rounded">
                                  {variable.name}
                                </code>
                                <span className="text-[10px] text-slate-400 bg-slate-200/60 px-1 py-0.5 rounded uppercase">
                                  {getValueType(variable.value)}
                                </span>
                              </div>
                              
                              {/* 变量值 */}
                              <div className="mt-1.5">
                                <code className="text-xs font-mono text-slate-600 break-all line-clamp-2">
                                  {formatValue(variable.value)}
                                </code>
                              </div>

                              {/* 变量说明 */}
                              {variable.description && (
                                <p className="mt-1 text-xs text-slate-500 line-clamp-1">
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
