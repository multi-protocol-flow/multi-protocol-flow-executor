/**
 * VariableDependencyGraph - 变量依赖关系图
 * 显示"哪些节点使用了某变量"和"某节点使用了哪些变量"
 */
import { useMemo, useState } from 'react';
import { Search, Variable, ArrowRight, ArrowLeft } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import type { Flow } from '@/types/flow';
import type { Node } from '@/types';

/** 变量定义 */
interface VarDefinition {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  variablePath: string;
}

/** 变量使用 */
interface VarUsage {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  variablePath: string;
}

/** 依赖图数据 */
interface DependencyGraph {
  definitions: VarDefinition[];
  usages: VarUsage[];
}

/** 提取节点配置中的 {{}} 变量引用 */
function extractVariableRefs(data: Record<string, unknown>): string[] {
  const refs: string[] = [];

  function walk(value: unknown): void {
    if (typeof value === 'string') {
      const matches = value.match(/\{\{([^}]+)\}\}/g);
      if (matches) {
        for (const match of matches) {
          const name = match.slice(2, -2).trim();
          if (!refs.includes(name)) {
            refs.push(name);
          }
        }
      }
    } else if (Array.isArray(value)) {
      value.forEach(walk);
    } else if (typeof value === 'object' && value !== null) {
      Object.values(value as Record<string, unknown>).forEach(walk);
    }
  }

  walk(data);
  return refs;
}

/** 获取节点定义的输出变量 */
function getNodeOutputVariables(node: Node): string[] {
  const vars: string[] = [];
  const data = node.data as unknown as Record<string, unknown>;

  // Entry 节点的 initial_variables
  if (node.type === 'entry') {
    const initialVars = data.initial_variables as Array<{ name: string }> | undefined;
    if (initialVars) {
      for (const iv of initialVars) {
        vars.push(`initial_variables.${iv.name}`);
      }
    }
  }

  // VariableExtractor 节点的 output_mappings
  if (node.type === 'variable_extractor' || node.type === 'script') {
    const outputMappings = data.output_mappings as Array<{ target: string }> | undefined;
    if (outputMappings) {
      for (const om of outputMappings) {
        vars.push(om.target);
      }
    }
  }

  // 通用输出变量约定：body.*, status, headers.*
  if (['http'].includes(node.type)) {
    vars.push('status');
    vars.push('body');
    vars.push('headers');
  }

  return vars;
}

/** 构建变量依赖图 */
function buildVariableDependencyGraph(flow: Flow): DependencyGraph {
  const graph: DependencyGraph = { definitions: [], usages: [] };

  for (const node of flow.nodes) {
    // 1. 找出该节点定义的变量
    const outputs = getNodeOutputVariables(node);
    for (const path of outputs) {
      graph.definitions.push({
        nodeId: node.uuid,
        nodeName: node.name,
        nodeType: node.type,
        variablePath: path,
      });
    }

    // 2. 找出该节点使用的变量（配置中的 {{}} 引用）
    const refs = extractVariableRefs(node.data as unknown as Record<string, unknown>);
    for (const ref of refs) {
      graph.usages.push({
        nodeId: node.uuid,
        nodeName: node.name,
        nodeType: node.type,
        variablePath: ref,
      });
    }
  }

  return graph;
}

interface VariableDependencyGraphProps {
  flow: Flow;
}

export function VariableDependencyGraph({ flow }: VariableDependencyGraphProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedVar, setSelectedVar] = useState<string | null>(null);

  // 构建依赖图
  const graph = useMemo(() => buildVariableDependencyGraph(flow), [flow]);

  // 所有唯一的变量路径
  const allVariablePaths = useMemo(() => {
    const paths = new Set<string>();
    for (const def of graph.definitions) paths.add(def.variablePath);
    for (const usage of graph.usages) paths.add(usage.variablePath);
    return Array.from(paths).sort();
  }, [graph]);

  // 过滤后的变量路径
  const filteredPaths = useMemo(() => {
    if (!searchQuery.trim()) return allVariablePaths;
    const q = searchQuery.toLowerCase();
    return allVariablePaths.filter((p) => p.toLowerCase().includes(q));
  }, [allVariablePaths, searchQuery]);

  // 选中变量的定义和使用
  const selectedDefs = useMemo(
    () => graph.definitions.filter((d) => d.variablePath === selectedVar),
    [graph.definitions, selectedVar]
  );
  const selectedUsages = useMemo(
    () => graph.usages.filter((u) => u.variablePath === selectedVar),
    [graph.usages, selectedVar]
  );

  // 计算每个变量的定义/使用数
  const varStats = useMemo(() => {
    const stats = new Map<string, { defs: number; usages: number }>();
    for (const path of allVariablePaths) {
      stats.set(path, {
        defs: graph.definitions.filter((d) => d.variablePath === path).length,
        usages: graph.usages.filter((u) => u.variablePath === path).length,
      });
    }
    return stats;
  }, [allVariablePaths, graph]);

  return (
    <div className="flex flex-col h-full">
      {/* 搜索框 */}
      <div className="p-2 border-b border-border">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            type="text"
            placeholder="搜索变量..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8 h-8 text-xs"
          />
        </div>
      </div>

      <div className="flex-1 flex min-h-0">
        {/* 左侧：变量列表 */}
        <div className="w-1/2 border-r border-border overflow-y-auto">
          <ScrollArea className="h-full">
            {filteredPaths.length > 0 ? (
              <div className="py-1">
                {filteredPaths.map((path) => {
                  const stats = varStats.get(path)!;
                  const isSelected = selectedVar === path;
                  return (
                    <button
                      key={path}
                      type="button"
                      onClick={() => setSelectedVar(isSelected ? null : path)}
                      className={cn(
                        'w-full flex items-center gap-2 px-3 py-2 text-left transition-colors cursor-pointer',
                        isSelected
                          ? 'bg-violet-100 dark:bg-violet-900/30 border-l-2 border-violet-500'
                          : 'hover:bg-accent/50 border-l-2 border-transparent'
                      )}
                    >
                      <Variable className="w-3.5 h-3.5 text-violet-500 shrink-0" />
                      <div className="flex-1 min-w-0">
                        <p className="text-xs font-mono text-foreground truncate">{path}</p>
                        <p className="text-[10px] text-muted-foreground">
                          定义: {stats.defs} · 使用: {stats.usages}
                        </p>
                      </div>
                    </button>
                  );
                })}
              </div>
            ) : (
              <div className="py-8 text-center text-xs text-muted-foreground">
                {searchQuery ? '未找到匹配变量' : '暂无变量'}
              </div>
            )}
          </ScrollArea>
        </div>

        {/* 右侧：选中变量的详情 */}
        <div className="w-1/2 overflow-y-auto">
          {selectedVar ? (
            <div className="p-3 space-y-4">
              {/* 变量名标题 */}
              <div>
                <code className="text-xs font-mono text-violet-600 dark:text-violet-400 bg-violet-100/60 dark:bg-violet-950/50 px-1.5 py-0.5 rounded">
                  {selectedVar}
                </code>
              </div>

              {/* 定义节点 */}
              {selectedDefs.length > 0 && (
                <div>
                  <h4 className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider mb-1.5 flex items-center gap-1">
                    <ArrowRight className="w-3 h-3 text-green-500" />
                    定义节点
                  </h4>
                  <div className="space-y-1">
                    {selectedDefs.map((def, i) => (
                      <div key={i} className="flex items-center gap-2 px-2 py-1 rounded bg-green-50 dark:bg-green-950/20">
                        <span className="text-xs text-foreground">{def.nodeName}</span>
                        <span className="text-[10px] text-muted-foreground">({def.nodeType})</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* 使用节点 */}
              {selectedUsages.length > 0 && (
                <div>
                  <h4 className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider mb-1.5 flex items-center gap-1">
                    <ArrowLeft className="w-3 h-3 text-amber-500" />
                    使用节点
                  </h4>
                  <div className="space-y-1">
                    {selectedUsages.map((usage, i) => (
                      <div key={i} className="flex items-center gap-2 px-2 py-1 rounded bg-amber-50 dark:bg-amber-950/20">
                        <span className="text-xs text-foreground">{usage.nodeName}</span>
                        <span className="text-[10px] text-muted-foreground">({usage.nodeType})</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {selectedDefs.length === 0 && selectedUsages.length === 0 && (
                <p className="text-xs text-muted-foreground py-4 text-center">
                  未找到该变量的定义或使用信息
                </p>
              )}
            </div>
          ) : (
            <div className="py-8 text-center text-xs text-muted-foreground">
              选择一个变量查看其依赖关系
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default VariableDependencyGraph;
