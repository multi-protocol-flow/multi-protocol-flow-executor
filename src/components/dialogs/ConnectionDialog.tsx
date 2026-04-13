/**
 * ConnectionDialog - 数据库连接编辑对话框
 * 支持新增、编辑、测试数据库连接
 */
import { useState, useEffect, useCallback } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useUIStore, useConnectionsStore } from '@/stores';
import type { DbConnection, DbDriver } from '@/types/database';
import { Loader2, CheckCircle, XCircle, Eye, EyeOff } from 'lucide-react';
import { cn } from '@/lib/utils';

/** 生成 UUID */
function generateUuid(): string {
  return 'conn-' + crypto.randomUUID().slice(0, 8);
}

/** 数据库驱动配置 */
const DB_DRIVERS: { value: DbDriver; label: string; defaultPort: number }[] = [
  { value: 'postgres', label: 'PostgreSQL', defaultPort: 5432 },
  { value: 'mysql', label: 'MySQL', defaultPort: 3306 },
  { value: 'sqlite', label: 'SQLite', defaultPort: 0 },
  { value: 'redis', label: 'Redis', defaultPort: 6379 },
];

/** 默认连接配置 */
function getDefaultConnection(): DbConnection {
  return {
    id: generateUuid(),
    name: '',
    driver: 'postgres',
    host: 'localhost',
    port: 5432,
    database: '',
    username: '',
    password: '',
    min_connections: 1,
    max_connections: 10,
    connect_timeout_ms: 10000,
    idle_timeout_ms: 300000,
    status: 'disconnected',
  };
}

export function ConnectionDialog() {
  const showConnectionDialog = useUIStore((state) => state.showConnectionDialog);
  const connectionDialogMode = useUIStore((state) => state.connectionDialogMode);
  const editingConnectionId = useUIStore((state) => state.editingConnectionId);
  const closeConnectionDialog = useUIStore((state) => state.closeConnectionDialog);

  const connections = useConnectionsStore((state) => state.connections);
  const saveConnection = useConnectionsStore((state) => state.saveConnection);
  const testConnection = useConnectionsStore((state) => state.testConnection);

  const [formData, setFormData] = useState<DbConnection>(getDefaultConnection());
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);
  const [testError, setTestError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);

  // 当对话框打开时，初始化表单数据
  useEffect(() => {
    if (showConnectionDialog) {
      if (connectionDialogMode === 'edit' && editingConnectionId) {
        const existing = connections.find((c) => c.id === editingConnectionId);
        if (existing) {
          setFormData({ ...existing });
        } else {
          setFormData(getDefaultConnection());
        }
      } else {
        setFormData(getDefaultConnection());
      }
      setTestResult(null);
      setTestError(null);
      setShowPassword(false);
    }
  }, [showConnectionDialog, connectionDialogMode, editingConnectionId, connections]);

  // 更新表单字段
  const updateField = useCallback(<K extends keyof DbConnection>(field: K, value: DbConnection[K]) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    setTestResult(null); // 任何修改都清除测试结果
  }, []);

  // 当驱动类型改变时，更新默认端口
  const handleDriverChange = useCallback((driver: DbDriver) => {
    const driverConfig = DB_DRIVERS.find((d) => d.value === driver);
    setFormData((prev) => ({
      ...prev,
      driver,
      port: driverConfig?.defaultPort ?? prev.port,
    }));
    setTestResult(null);
  }, []);

  // 测试连接
  const handleTest = useCallback(async () => {
    setIsTesting(true);
    setTestResult(null);
    setTestError(null);
    try {
      // 临时保存连接以便测试
      const tempConnection = { ...formData, status: 'testing' as const };
      await saveConnection(tempConnection);
      const result = await testConnection(tempConnection.id);
      setTestResult(result.success ? 'success' : 'error');
      if (!result.success && result.error) {
        setTestError(result.error);
      }
    } catch (e) {
      setTestResult('error');
      setTestError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsTesting(false);
    }
  }, [formData, saveConnection, testConnection]);

  // 保存连接
  const handleSave = useCallback(async () => {
    if (!formData.name.trim()) return;

    setIsSaving(true);
    try {
      await saveConnection({
        ...formData,
        status: 'disconnected', // 保存时重置状态
      });
      closeConnectionDialog();
    } finally {
      setIsSaving(false);
    }
  }, [formData, saveConnection, closeConnectionDialog]);

  // 关闭对话框
  const handleClose = useCallback(() => {
    closeConnectionDialog();
  }, [closeConnectionDialog]);

  if (!showConnectionDialog) return null;

  const isSqlite = formData.driver === 'sqlite';

  return (
    <Dialog open={showConnectionDialog} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent className="w-full max-w-[550px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {connectionDialogMode === 'edit' ? '编辑数据库连接' : '新建数据库连接'}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-5 py-4">
          {/* 基本信息区 */}
          <div className="space-y-4">
            <h4 className="text-sm font-medium text-foreground border-b border-border pb-1">
              基本信息
            </h4>

            {/* 连接名称 */}
            <div className="space-y-2">
              <Label htmlFor="name">连接名称 <span className="text-rose-500">*</span></Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => updateField('name', e.target.value)}
                placeholder="例如：生产环境 MySQL"
                disabled={isSaving}
              />
            </div>

            {/* 数据库类型 */}
            <div className="space-y-2">
              <Label htmlFor="driver">数据库类型</Label>
              <Select
                value={formData.driver}
                onValueChange={(v) => handleDriverChange(v as DbDriver)}
                disabled={isSaving}
              >
                <SelectTrigger id="driver">
                  <SelectValue placeholder="选择数据库类型" />
                </SelectTrigger>
                <SelectContent>
                  {DB_DRIVERS.map((d) => (
                    <SelectItem key={d.value} value={d.value}>
                      {d.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* 连接参数区（非 SQLite） */}
          {!isSqlite && (
            <div className="space-y-4">
              <h4 className="text-sm font-medium text-foreground border-b border-border pb-1">
                连接参数
              </h4>

              {/* 主机和端口 */}
              <div className="grid grid-cols-3 gap-3">
                <div className="col-span-2 space-y-2">
                  <Label htmlFor="host">主机地址</Label>
                  <Input
                    id="host"
                    value={formData.host || ''}
                    onChange={(e) => updateField('host', e.target.value)}
                    placeholder="localhost"
                    disabled={isSaving}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="port">端口</Label>
                  <Input
                    id="port"
                    type="number"
                    value={formData.port || ''}
                    onChange={(e) => updateField('port', parseInt(e.target.value) || 0)}
                    placeholder="5432"
                    disabled={isSaving}
                  />
                </div>
              </div>

              {/* 数据库名 */}
              <div className="space-y-2">
                <Label htmlFor="database">数据库名</Label>
                <Input
                  id="database"
                  value={formData.database || ''}
                  onChange={(e) => updateField('database', e.target.value)}
                  placeholder="mydb"
                  disabled={isSaving}
                />
              </div>

              {/* 用户名和密码 */}
              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-2">
                  <Label htmlFor="username">用户名</Label>
                  <Input
                    id="username"
                    value={formData.username || ''}
                    onChange={(e) => updateField('username', e.target.value)}
                    placeholder="root"
                    disabled={isSaving}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="password">密码</Label>
                  <div className="relative">
                    <Input
                      id="password"
                      type={showPassword ? 'text' : 'password'}
                      value={formData.password || ''}
                      onChange={(e) => updateField('password', e.target.value)}
                      placeholder="••••••••"
                      disabled={isSaving}
                      className="pr-10"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="absolute right-0 top-0 h-full w-10 hover:bg-transparent"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <EyeOff className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <Eye className="h-4 w-4 text-muted-foreground" />
                      )}
                    </Button>
                  </div>
                </div>
              </div>

              {/* 环境变量引用 */}
              <div className="space-y-2">
                <Label htmlFor="password_from_env">密码环境变量名（可选）</Label>
                <Input
                  id="password_from_env"
                  value={formData.password_from_env || ''}
                  onChange={(e) => updateField('password_from_env', e.target.value)}
                  placeholder="DB_PASSWORD（优先于密码字段）"
                  disabled={isSaving}
                />
                <p className="text-xs text-muted-foreground">
                  填写后将从环境变量读取密码，优先于上方密码字段
                </p>
              </div>
            </div>
          )}

          {/* SQLite 文件路径 */}
          {isSqlite && (
            <div className="space-y-4">
              <h4 className="text-sm font-medium text-foreground border-b border-border pb-1">
                连接参数
              </h4>
              <div className="space-y-2">
                <Label htmlFor="connection_string">数据库文件路径</Label>
                <Input
                  id="connection_string"
                  value={formData.connection_string || ''}
                  onChange={(e) => updateField('connection_string', e.target.value)}
                  placeholder="/path/to/database.db"
                  disabled={isSaving}
                />
                <p className="text-xs text-muted-foreground">
                  SQLite 数据库文件的绝对路径或相对路径
                </p>
              </div>
            </div>
          )}

          {/* 测试结果 */}
          {testResult && (
            <div
              className={cn(
                'p-3 rounded-lg',
                testResult === 'success' ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'
              )}
            >
              {testResult === 'success' ? (
                <div className="flex items-center gap-2">
                  <CheckCircle className="w-5 h-5" />
                  <span>连接测试成功</span>
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <XCircle className="w-5 h-5" />
                    <span>连接测试失败</span>
                  </div>
                  {testError && (
                    <div className="text-sm bg-red-100 rounded p-2 font-mono break-all">
                      {testError}
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={handleTest} disabled={isSaving || isTesting}>
            {isTesting && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
            测试连接
          </Button>
          <Button variant="outline" onClick={handleClose} disabled={isSaving}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={isSaving || !formData.name.trim()}>
            {isSaving ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                保存中...
              </>
            ) : (
              '保存'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
