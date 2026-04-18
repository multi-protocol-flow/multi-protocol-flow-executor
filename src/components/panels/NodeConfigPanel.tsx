/**
 * 节点配置面板主组件
 * 根据节点类型显示不同的配置表单
 */
import { useState, useRef, useMemo, useEffect } from 'react';

/** 深度比较两个值是否相等（属性顺序无关，undefined 值视为缺失） */
function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a == null || b == null) return a === b;
  if (typeof a !== typeof b) return false;
  if (Array.isArray(a)) {
    if (!Array.isArray(b)) return false;
    if (a.length !== b.length) return false;
    return a.every((item, i) => deepEqual(item, b[i]));
  }
  if (typeof a === 'object') {
    const aObj = a as Record<string, unknown>;
    const bObj = b as Record<string, unknown>;
    const aKeys = Object.keys(aObj).filter(k => aObj[k] !== undefined);
    const bKeys = Object.keys(bObj).filter(k => bObj[k] !== undefined);
    if (aKeys.length !== bKeys.length) return false;
    const bKeySet = new Set(bKeys);
    return aKeys.every(k => bKeySet.has(k) && deepEqual(aObj[k], bObj[k]));
  }
  return false;
}
import { useFlowStore, useDebugStore, selectHistoryVersion } from '@/stores';
import { DebugNodeConfigDialog } from '@/components/dialogs/DebugNodeConfigDialog';
import { EntryConfig } from './EntryConfig';
import { HttpConfig } from './HttpConfig';
import { ConditionalConfig } from './ConditionalConfig';
import { WsConnectConfig } from './WsConnectConfig';
import { WsSendCollectConfig } from './WsSendCollectConfig';
import { WsCloseConfig } from './WsCloseConfig';
import { AssertionConfig } from './AssertionConfig';
import { DbConnectConfig } from './DbConnectConfig';
import { DbQueryConfig } from './DbQueryConfig';
import { DbBeginConfig } from './DbBeginConfig';
import { DbCommitConfig } from './DbCommitConfig';
import { DbRollbackConfig } from './DbRollbackConfig';
import { DbCloseConfig } from './DbCloseConfig';
import { ScriptConfig } from './ScriptConfig';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { NodeData, Node } from '@/types';
import { isEntryConfig, isHttpConfig, isConditionalConfig, isWsConnectConfig, isWsSendCollectConfig, isWsCloseConfig, isAssertionConfig, isDbConnectConfig, isDbQueryConfig, isDbBeginConfig, isDbCommitConfig, isDbRollbackConfig, isDbCloseConfig, isScriptConfig } from '@/types';
import { Settings2, MousePointerClick, Lock, PencilLine } from 'lucide-react';
import { cn } from '@/lib/utils';

interface NodeConfigPanelProps {
  /** 只读模式（调试时查看节点配置） */
  readOnly?: boolean;
  /** 指定要显示的节点（用于只读模式，不使用 store 中的选中节点） */
  node?: Node | null;
}

export function NodeConfigPanel({ readOnly = false, node: externalNode }: NodeConfigPanelProps) {
  const { flow, getSelectedNode, updateNode } = useFlowStore();
  const modifyNodeConfig = useDebugStore((state) => state.modifyNodeConfig);
  const historyVersion = useFlowStore(selectHistoryVersion);
  
  // 优先使用外部传入的节点，否则回退到 store 中的选中节点
  const selectedNode = externalNode ?? getSelectedNode();

  // 冻结节点 UUID（ref 持久化），确保组件卸载时（如点击画布取消选中）仍能正确保存
  // 使用 useEffect 更新而非渲染期间更新，确保子组件的 cleanup effect 执行时
  // ref 仍是旧节点的 UUID，避免卸载保存时写入错误节点
  const frozenNodeUuidRef = useRef<string | null>(null);
  useEffect(() => {
    frozenNodeUuidRef.current = selectedNode?.uuid ?? null;
  }, [selectedNode?.uuid]);

  // 稳定数据引用：仅在节点 UUID 变化或 undo/redo 时更新
  // 避免自动保存后 store 更新 → data prop 变化 → 子组件 reset() → 输入框失焦
  // historyVersion 作为额外依赖，确保 undo/redo 能正确同步表单状态
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const stableData = useMemo(() => selectedNode?.data ?? null, [selectedNode?.uuid, historyVersion]);

  // 调试模式下编辑配置弹窗状态
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);

  // 未选中节点时的空状态
  if (!selectedNode) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-6 text-center">
        <div className="rounded-full bg-muted p-4 mb-4">
          <MousePointerClick className="h-8 w-8 text-muted-foreground" />
        </div>
        <h3 className="text-lg font-medium text-foreground">
          {readOnly ? '调试模式' : '未选中节点'}
        </h3>
        <p className="mt-2 text-sm text-muted-foreground">
          {readOnly 
            ? '点击画布上的节点查看其配置信息' 
            : '请在画布上点击选择一个节点进行配置'}
        </p>
      </div>
    );
  }

  // 冻结节点 UUID，防止快速切换节点时闭包捕获到过时的节点
  // 仅用于调试模式下 DebugNodeConfigDialog 的回调
  const frozenNodeUuid = selectedNode.uuid;

  // stableData 在节点 UUID 变化后才更新，此處 selectedNode 必定非空
  const nodeData = stableData as NodeData;

  /**
   * 处理配置保存
   * 使用 ref 持久化的 UUID，确保取消选中节点后仍能正确保存
   * 去重：使用深度比较（属性顺序无关），避免表单序列化差异触发无意义保存
   */
  const handleSave = (data: NodeData) => {
    if (frozenNodeUuidRef.current) {
      // 从 flow 中按 UUID 查找节点（不用 getSelectedNode，因为卸载时选中状态可能已变）
      const node = flow?.nodes.find((n) => n.uuid === frozenNodeUuidRef.current);
      if (node && deepEqual(node.data, data)) {
        return;
      }
      updateNode(frozenNodeUuidRef.current, { data });
    }
  };

  return (
    <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none w-full min-w-0">
      <CardHeader className="shrink-0 border-b border-muted bg-background pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 flex-1 min-w-0">
            {readOnly ? (
              <Lock className="h-4 w-4 text-amber-500 shrink-0" />
            ) : (
              <Settings2 className="h-4 w-4 text-muted-foreground shrink-0" />
            )}
            <CardTitle className="text-base font-semibold text-foreground truncate">
              {selectedNode.name}
            </CardTitle>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {readOnly && (
              <>
                <span className="inline-flex items-center rounded-md border border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-950/50 px-2 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-300">
                  只读
                </span>
                {selectedNode && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-6 gap-1.5 text-xs"
                    onClick={() => setIsEditDialogOpen(true)}
                  >
                    <PencilLine className="h-3 w-3" />
                    编辑配置
                  </Button>
                )}
              </>
            )}
          </div>
        </div>
      </CardHeader>
      
      <CardContent className="flex-1 overflow-hidden p-0 relative">
        <ScrollArea className="h-full">
          <div className={cn(
            "w-full min-w-0 max-w-full overflow-hidden px-6 py-4",
            readOnly && "pointer-events-none select-none opacity-80"
          )}>
            {/* Entry 节点：初始变量配置 */}
            {selectedNode.type === 'entry' && isEntryConfig(nodeData) && (
              <EntryConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* HTTP 节点：URL/方法/Headers/Body 配置 */}
            {selectedNode.type === 'http' && isHttpConfig(nodeData) && (
              <HttpConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* Conditional 节点：条件配置 */}
            {selectedNode.type === 'conditional' && isConditionalConfig(nodeData) && (
              <ConditionalConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* WS Connect 节点：WebSocket 连接配置 */}
            {selectedNode.type === 'ws_connect' && isWsConnectConfig(nodeData) && (
              <WsConnectConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* WS Send+Collect 节点：WebSocket 发送采集配置 */}
            {selectedNode.type === 'ws_send_collect' && isWsSendCollectConfig(nodeData) && (
              <WsSendCollectConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* WS Close 节点：WebSocket 关闭配置 */}
            {selectedNode.type === 'ws_close' && isWsCloseConfig(nodeData) && (
              <WsCloseConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* Assertion 节点：断言配置 */}
            {selectedNode.type === 'assertion' && isAssertionConfig(nodeData) && (
              <AssertionConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Connect 节点：数据库连接配置 */}
            {selectedNode.type === 'db_connect' && isDbConnectConfig(nodeData) && (
              <DbConnectConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Query 节点：数据库查询配置 */}
            {selectedNode.type === 'db_query' && isDbQueryConfig(nodeData) && (
              <DbQueryConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Begin 节点：事务开始配置 */}
            {selectedNode.type === 'db_begin' && isDbBeginConfig(nodeData) && (
              <DbBeginConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Commit 节点：事务提交配置 */}
            {selectedNode.type === 'db_commit' && isDbCommitConfig(nodeData) && (
              <DbCommitConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Rollback 节点：事务回滚配置 */}
            {selectedNode.type === 'db_rollback' && isDbRollbackConfig(nodeData) && (
              <DbRollbackConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Close 节点：数据库关闭配置 */}
            {selectedNode.type === 'db_close' && isDbCloseConfig(nodeData) && (
              <DbCloseConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* Script 节点：脚本配置 */}
            {selectedNode.type === 'script' && isScriptConfig(nodeData) && (
              <ScriptConfig
                data={nodeData}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}
          </div>
        </ScrollArea>
      </CardContent>

      {/* 调试模式下编辑配置弹窗（使用 selectedNode.data 以获取最新数据） */}
      {readOnly && selectedNode && (
        <DebugNodeConfigDialog
          open={isEditDialogOpen}
          nodeData={selectedNode.data}
          nodeName={selectedNode.name}
          onSave={async (data) => {
            await modifyNodeConfig(frozenNodeUuid, data);
          }}
          onCancel={() => setIsEditDialogOpen(false)}
        />
      )}
    </Card>
  );
}
