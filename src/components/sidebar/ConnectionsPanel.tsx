/**
 * ConnectionsPanel - 数据库连接列表面板
 * 显示已配置的数据库连接，支持新增、编辑、删除、测试连接
 */
import { useEffect, useCallback } from 'react';
import { Plus, Database, Pencil, Trash2, Loader2, CheckCircle, XCircle, Circle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useConnectionsStore, selectDbConnections } from '@/stores/connections';
import { useUIStore } from '@/stores';
import { cn } from '@/lib/utils';
import type { DbConnection } from '@/types/database';

/** 数据库驱动显示名称 */
const DRIVER_LABELS: Record<string, string> = {
  postgres: 'PostgreSQL',
  mysql: 'MySQL',
  sqlite: 'SQLite',
  redis: 'Redis',
};

/** 数据库驱动颜色 */
const DRIVER_COLORS: Record<string, string> = {
  postgres: 'text-blue-600',
  mysql: 'text-orange-600',
  sqlite: 'text-green-600',
  redis: 'text-red-600',
};

/** 获取连接状态图标 */
function getStatusIcon(status: DbConnection['status']) {
  switch (status) {
    case 'connected':
      return <CheckCircle className="w-3.5 h-3.5 text-green-500" />;
    case 'error':
      return <XCircle className="w-3.5 h-3.5 text-red-500" />;
    case 'testing':
      return <Loader2 className="w-3.5 h-3.5 text-amber-500 animate-spin" />;
    default:
      return <Circle className="w-3.5 h-3.5 text-muted-foreground" />;
  }
}

interface ConnectionsPanelProps {
  showHeader?: boolean;
}

export function ConnectionsPanel({ showHeader = true }: ConnectionsPanelProps) {
  const connections = useConnectionsStore(selectDbConnections);
  const isLoading = useConnectionsStore((state) => state.isLoading);
  const loadConnections = useConnectionsStore((state) => state.loadConnections);
  const deleteConnection = useConnectionsStore((state) => state.deleteConnection);
  const testConnection = useConnectionsStore((state) => state.testConnection);

  const openConnectionDialog = useUIStore((state) => state.openConnectionDialog);
  const openConnectionDialogForEdit = useUIStore((state) => state.openConnectionDialogForEdit);
  const showConfirm = useUIStore((state) => state.showConfirm);

  // 组件挂载时加载连接列表
  useEffect(() => {
    loadConnections();
  }, [loadConnections]);

  // 新建连接
  const handleAdd = useCallback(() => {
    openConnectionDialog();
  }, [openConnectionDialog]);

  // 编辑连接
  const handleEdit = useCallback((e: React.MouseEvent, connectionId: string) => {
    e.stopPropagation();
    openConnectionDialogForEdit(connectionId);
  }, [openConnectionDialogForEdit]);

  // 删除连接
  const handleDelete = useCallback((e: React.MouseEvent, connection: DbConnection) => {
    e.stopPropagation();
    showConfirm({
      title: '删除连接',
      description: `确定要删除连接「${connection.name}」吗？删除后无法恢复。`,
      confirmText: '删除',
      onConfirm: () => {
        deleteConnection(connection.id);
      },
    });
  }, [deleteConnection, showConfirm]);

  // 测试连接
  const handleTest = useCallback((e: React.MouseEvent, connectionId: string) => {
    e.stopPropagation();
    testConnection(connectionId);
  }, [testConnection]);

  return (
    <div className="flex flex-col h-full">
      {/* 标题栏 */}
      {showHeader && (
        <div className="flex items-center justify-between px-3 py-2 border-b border-border">
          <span className="text-sm font-medium text-foreground">数据库连接</span>
          <Button
            variant="ghost"
            size="icon"
            onClick={handleAdd}
            className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent"
            title="新建连接"
          >
            <Plus className="w-4 h-4 text-foreground" />
          </Button>
        </div>
      )}

      {/* 连接列表 */}
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden scrollbar-thin">
        <div className="p-2 space-y-1">
          {isLoading ? (
            <div className="flex items-center justify-center py-6 text-muted-foreground">
              <Loader2 className="w-5 h-5 animate-spin mr-2" />
              <span className="text-sm">加载中...</span>
            </div>
          ) : connections.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-6 text-muted-foreground">
              <Database className="w-8 h-8 mb-2 opacity-40" />
              <p className="text-sm">暂无数据库连接</p>
              <p className="text-xs mt-1 opacity-70">点击 + 创建新连接</p>
            </div>
          ) : (
            connections.map((connection) => (
              <div
                key={connection.id}
                className={cn(
                  'group w-full flex flex-col px-3 py-2.5 rounded-lg',
                  'bg-background border border-border',
                  'hover:bg-accent transition-all duration-150'
                )}
              >
                {/* 第一行：状态 + 名称 + 驱动 */}
                <div className="flex items-center gap-2">
                  {getStatusIcon(connection.status)}
                  <span className="text-sm font-medium text-foreground truncate flex-1">
                    {connection.name}
                  </span>
                  <span className={cn('text-xs font-medium', DRIVER_COLORS[connection.driver])}>
                    {DRIVER_LABELS[connection.driver] || connection.driver}
                  </span>
                </div>

                {/* 第二行：连接地址 */}
                <div className="flex items-center gap-2 mt-1 pl-5">
                  <span className="text-xs text-muted-foreground truncate flex-1 font-mono">
                    {connection.driver === 'sqlite'
                      ? connection.connection_string || '(未设置路径)'
                      : connection.host
                        ? `${connection.host}${connection.port ? `:${connection.port}` : ''}${connection.database ? ` / ${connection.database}` : ''}`
                        : '(未配置)'}
                  </span>
                </div>

                {/* 操作按钮（悬停显示） */}
                <div className="flex items-center gap-1 mt-2 pl-5 opacity-0 group-hover:opacity-100 transition-opacity">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(e) => handleTest(e, connection.id)}
                    className="h-6 px-2 text-xs text-muted-foreground hover:text-foreground hover:bg-accent"
                    title="测试连接"
                    disabled={connection.status === 'testing'}
                  >
                    {connection.status === 'testing' ? (
                      <Loader2 className="w-3 h-3 mr-1 animate-spin" />
                    ) : null}
                    测试
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(e) => handleEdit(e, connection.id)}
                    className="h-6 px-2 text-xs text-muted-foreground hover:text-foreground hover:bg-accent"
                    title="编辑连接"
                  >
                    <Pencil className="w-3 h-3 mr-1" />
                    编辑
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(e) => handleDelete(e, connection)}
                    className="h-6 px-2 text-xs text-muted-foreground hover:text-red-600 hover:bg-red-50"
                    title="删除连接"
                  >
                    <Trash2 className="w-3 h-3 mr-1" />
                    删除
                  </Button>
                </div>

                {/* 错误信息 */}
                {connection.last_error && (
                  <div className="mt-2 pl-5">
                    <p className="text-xs text-red-500 truncate" title={connection.last_error}>
                      错误: {connection.last_error}
                    </p>
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      </div>

      {/* 底部提示 */}
      {connections.length > 0 && (
        <div className="px-3 py-2 border-t border-border bg-muted">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Database className="w-3.5 h-3.5 text-violet-500" />
            <span>共 {connections.length} 个连接配置</span>
          </div>
        </div>
      )}
    </div>
  );
}
