/**
 * 节点配置面板主组件
 * 根据节点类型显示不同的配置表单
 */
import { useState } from 'react';
import { useFlowStore, useDebugStore } from '@/stores';
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
  const { getSelectedNode, updateNode } = useFlowStore();
  const modifyNodeConfig = useDebugStore((state) => state.modifyNodeConfig);
  
  // 优先使用外部传入的节点，否则回退到 store 中的选中节点
  const selectedNode = externalNode ?? getSelectedNode();

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

  /**
   * 处理配置保存
   * 动态从 store 获取当前选中的节点，避免闭包捕获过时的 selectedNode
   */
  const handleSave = (data: NodeData) => {
    const node = externalNode ?? useFlowStore.getState().getSelectedNode();
    if (node) {
      updateNode(node.uuid, { data });
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
            {selectedNode.type === 'entry' && isEntryConfig(selectedNode.data) && (
              <EntryConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* HTTP 节点：URL/方法/Headers/Body 配置 */}
            {selectedNode.type === 'http' && isHttpConfig(selectedNode.data) && (
              <HttpConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* Conditional 节点：条件配置 */}
            {selectedNode.type === 'conditional' && isConditionalConfig(selectedNode.data) && (
              <ConditionalConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* WS Connect 节点：WebSocket 连接配置 */}
            {selectedNode.type === 'ws_connect' && isWsConnectConfig(selectedNode.data) && (
              <WsConnectConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* WS Send+Collect 节点：WebSocket 发送采集配置 */}
            {selectedNode.type === 'ws_send_collect' && isWsSendCollectConfig(selectedNode.data) && (
              <WsSendCollectConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* WS Close 节点：WebSocket 关闭配置 */}
            {selectedNode.type === 'ws_close' && isWsCloseConfig(selectedNode.data) && (
              <WsCloseConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* Assertion 节点：断言配置 */}
            {selectedNode.type === 'assertion' && isAssertionConfig(selectedNode.data) && (
              <AssertionConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Connect 节点：数据库连接配置 */}
            {selectedNode.type === 'db_connect' && isDbConnectConfig(selectedNode.data) && (
              <DbConnectConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Query 节点：数据库查询配置 */}
            {selectedNode.type === 'db_query' && isDbQueryConfig(selectedNode.data) && (
              <DbQueryConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Begin 节点：事务开始配置 */}
            {selectedNode.type === 'db_begin' && isDbBeginConfig(selectedNode.data) && (
              <DbBeginConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Commit 节点：事务提交配置 */}
            {selectedNode.type === 'db_commit' && isDbCommitConfig(selectedNode.data) && (
              <DbCommitConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Rollback 节点：事务回滚配置 */}
            {selectedNode.type === 'db_rollback' && isDbRollbackConfig(selectedNode.data) && (
              <DbRollbackConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* DB Close 节点：数据库关闭配置 */}
            {selectedNode.type === 'db_close' && isDbCloseConfig(selectedNode.data) && (
              <DbCloseConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}

            {/* Script 节点：脚本配置 */}
            {selectedNode.type === 'script' && isScriptConfig(selectedNode.data) && (
              <ScriptConfig
                data={selectedNode.data}
                onSave={(data) => handleSave(data as NodeData)}
              />
            )}
          </div>
        </ScrollArea>
      </CardContent>

      {/* 调试模式下编辑配置弹窗 */}
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
