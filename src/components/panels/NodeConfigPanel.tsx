/**
 * 节点配置面板主组件
 * 根据节点类型显示不同的配置表单
 */
import { useState, useRef, useEffect } from 'react';
import { useFlowStore, useUIStore } from '@/stores';
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
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { NodeData, Node } from '@/types';
import { isEntryConfig, isHttpConfig, isConditionalConfig, isWsConnectConfig, isWsSendCollectConfig, isWsCloseConfig, isAssertionConfig, isDbConnectConfig, isDbQueryConfig, isDbBeginConfig, isDbCommitConfig, isDbRollbackConfig, isDbCloseConfig, isScriptConfig } from '@/types';
import { Settings2, MousePointerClick, Trash2, Pencil, Lock } from 'lucide-react';
import { cn } from '@/lib/utils';

/**
 * 节点类型配置映射
 */
const NODE_TYPE_CONFIG = {
  entry: {
    label: '入口',
    badgeVariant: 'default' as const,
    badgeClass: 'bg-emerald-50 text-emerald-700',
  },
  http: {
    label: 'HTTP',
    badgeVariant: 'secondary' as const,
    badgeClass: 'bg-sky-50 text-sky-700',
  },
  conditional: {
    label: '条件',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-violet-200 text-violet-700',
  },
  ws_connect: {
    label: 'WS 连接',
    badgeVariant: 'secondary' as const,
    badgeClass: 'bg-amber-50 text-amber-700',
  },
  ws_send_collect: {
    label: 'WS 发送采集',
    badgeVariant: 'secondary' as const,
    badgeClass: 'bg-teal-50 text-teal-700',
  },
  ws_close: {
    label: 'WS 关闭',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-rose-200 text-rose-700',
  },
  assertion: {
    label: '断言',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-indigo-200 text-indigo-700',
  },
  db_connect: {
    label: 'DB 连接',
    badgeVariant: 'secondary' as const,
    badgeClass: 'bg-purple-50 text-purple-700',
  },
  db_query: {
    label: 'DB 查询',
    badgeVariant: 'secondary' as const,
    badgeClass: 'bg-cyan-50 text-cyan-700',
  },
  db_begin: {
    label: 'DB 开始事务',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-lime-200 text-lime-700',
  },
  db_commit: {
    label: 'DB 提交',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-green-200 text-green-700',
  },
  db_rollback: {
    label: 'DB 回滚',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-red-200 text-red-700',
  },
  db_close: {
    label: 'DB 关闭',
    badgeVariant: 'outline' as const,
    badgeClass: 'border-border text-muted-foreground',
  },
  script: {
    label: '脚本',
    badgeVariant: 'secondary' as const,
    badgeClass: 'bg-fuchsia-50 text-fuchsia-700',
  },
};

interface NodeConfigPanelProps {
  /** 只读模式（调试时查看节点配置） */
  readOnly?: boolean;
  /** 指定要显示的节点（用于只读模式，不使用 store 中的选中节点） */
  node?: Node | null;
}

export function NodeConfigPanel({ readOnly = false, node: externalNode }: NodeConfigPanelProps) {
  const { getSelectedNode, updateNode, removeNode, selectNode } = useFlowStore();
  const { showConfirm } = useUIStore();
  
  // 只读模式使用外部传入的节点，否则使用 store 中的选中节点
  const selectedNode = readOnly ? externalNode : getSelectedNode();
  
  // 节点名称编辑状态
  const [isEditingName, setIsEditingName] = useState(false);
  const [editingName, setEditingName] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  // 当选中节点变化时，重置编辑状态
  useEffect(() => {
    setIsEditingName(false);
  }, [selectedNode?.uuid]);

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

  const typeConfig = NODE_TYPE_CONFIG[selectedNode.type];

  /**
   * 处理配置保存
   */
  const handleSave = (data: NodeData) => {
    updateNode(selectedNode.uuid, { data });
  };

  /**
   * 处理删除节点
   */
  const handleDelete = () => {
    showConfirm({
      title: '删除节点',
      description: `确定要删除节点「${selectedNode.name}」吗？`,
      confirmText: '删除',
      onConfirm: () => {
        removeNode(selectedNode.uuid);
        selectNode(null);
      },
    });
  };

  /**
   * 开始编辑节点名称
   */
  const handleStartEdit = () => {
    setEditingName(selectedNode.name);
    setIsEditingName(true);
  };

  /**
   * 保存节点名称
   */
  const handleSaveName = () => {
    const trimmedName = editingName.trim();
    if (trimmedName && trimmedName !== selectedNode.name) {
      updateNode(selectedNode.uuid, { name: trimmedName });
    }
    setIsEditingName(false);
  };

  /**
   * 取消编辑节点名称
   */
  const handleCancelEdit = () => {
    setIsEditingName(false);
    setEditingName(selectedNode.name);
  };

  /**
   * 处理键盘事件
   */
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSaveName();
    } else if (e.key === 'Escape') {
      handleCancelEdit();
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
            {isEditingName && !readOnly ? (
              <input
                ref={inputRef}
                type="text"
                value={editingName}
                onChange={(e) => setEditingName(e.target.value)}
                onBlur={handleSaveName}
                onKeyDown={handleKeyDown}
                className="text-base font-semibold text-foreground bg-background border-b-2 border-border outline-none flex-1 min-w-0 px-0.5"
                autoFocus
              />
            ) : (
              <>
                <CardTitle className="text-base font-semibold text-foreground truncate cursor-default">
                  {selectedNode.name}
                </CardTitle>
                {!readOnly && (
                  <button
                    onClick={handleStartEdit}
                    className="shrink-0 p-1 rounded hover:bg-muted text-muted-foreground hover:text-accent-foreground transition-colors"
                    title="编辑节点名称"
                  >
                    <Pencil className="h-3.5 w-3.5" />
                  </button>
                )}
              </>
            )}
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {readOnly && (
              <Badge variant="outline" className="bg-amber-50 text-amber-700 border-amber-200">
                只读
              </Badge>
            )}
            <Badge 
              variant={typeConfig.badgeVariant}
              className={typeConfig.badgeClass}
            >
              {typeConfig.label}
            </Badge>
            {!readOnly && selectedNode.type !== 'entry' && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-muted-foreground hover:text-red-500 hover:bg-red-50"
                onClick={handleDelete}
                title="删除节点"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
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
    </Card>
  );
}
