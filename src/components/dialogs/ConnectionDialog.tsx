/**
 * ConnectionDialog - 数据库连接编辑对话框（受控组件）
 * 通过 props 控制打开/关闭状态，表单数据通过 onSave 回调返回
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
import type { DbDriver } from '@/types/database';
import type { TestDbConnectionResult } from '@/types/ipc';
import { cn } from '@/lib/utils';
import { Loader2, CheckCircle, XCircle, Eye, EyeOff } from 'lucide-react';

/**
 * 检查是否在 Tauri 环境
 */
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/** 连接表单数据类型（导出供调用方使用） */
export interface ConnectionFormData {
  name: string;
  driver: DbDriver;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  password?: string;
  password_from_env?: string;
  connection_string?: string;
  min_connections: number;
  max_connections: number;
  connect_timeout_ms: number;
  idle_timeout_ms: number;
}

/** 受控组件 Props */
export interface ConnectionDialogProps {
  /** 是否打开 */
  open: boolean;
  /** 打开状态变更回调 */
  onOpenChange: (open: boolean) => void;
  /** 保存回调，返回表单数据 */
  onSave: (data: ConnectionFormData) => void;
  /** 初始数据（编辑时传入） */
  initialData?: Partial<ConnectionFormData>;
}

/** 数据库驱动配置 */
const DB_DRIVERS: { value: DbDriver; label: string; defaultPort: number }[] = [
  { value: 'postgres', label: 'PostgreSQL', defaultPort: 5432 },
  { value: 'mysql', label: 'MySQL', defaultPort: 3306 },
  { value: 'sqlite', label: 'SQLite', defaultPort: 0 },
  { value: 'redis', label: 'Redis', defaultPort: 6379 },
];

/** 默认连接配置 */
function getDefaultConnection(): ConnectionFormData {
  return {
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
  };
}

/** 从 initialData 构建表单初始值 */
function buildInitialFormData(initialData?: Partial<ConnectionFormData>): ConnectionFormData {
  return {
    ...getDefaultConnection(),
    ...initialData,
  };
}

export function ConnectionDialog({ open, onOpenChange, onSave, initialData }: ConnectionDialogProps) {
  const [formData, setFormData] = useState<ConnectionFormData>(buildInitialFormData(initialData));
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);
  const [testError, setTestError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);

  // 当对话框打开时，初始化表单数据
  useEffect(() => {
    if (open) {
      setFormData(buildInitialFormData(initialData));
      setTestResult(null);
      setTestError(null);
      setShowPassword(false);
    }
  }, [open, initialData]);

  // 更新表单字段
  const updateField = useCallback(<K extends keyof ConnectionFormData>(field: K, value: ConnectionFormData[K]) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    setTestResult(null);
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

    console.log('[数据库连接测试] 开始测试...', {
      driver: formData.driver,
      host: formData.host,
      port: formData.port,
      database: formData.database,
      username: formData.username,
    });

    try {
      if (!isTauriEnv()) {
        console.warn('[数据库连接测试] 非 Tauri 环境，跳过真实连接测试');
        await new Promise((resolve) => setTimeout(resolve, 500));
        setTestResult('success');
        setIsTesting(false);
        return;
      }

      // 动态导入 invoke，避免非 Tauri 环境加载报错
      const { invoke } = await import('@tauri-apps/api/core');

      const result = await invoke<TestDbConnectionResult>('test_db_connection', {
        connection: {
          id: `test-${Date.now()}`,
          name: formData.name || '测试连接',
          driver: formData.driver,
          host: formData.host || null,
          port: formData.port || null,
          database: formData.database || null,
          username: formData.username || null,
          password: formData.password || null,
          passwordFromEnv: formData.password_from_env || null,
          connectionString: formData.connection_string || null,
          minConnections: formData.min_connections,
          maxConnections: formData.max_connections,
          connectTimeoutMs: formData.connect_timeout_ms,
          idleTimeoutMs: formData.idle_timeout_ms,
          status: 'disconnected',
          lastError: null,
          lastTestedAt: null,
        },
      });

      console.log('[数据库连接测试] 结果:', result);

      if (result.success) {
        setTestResult('success');
      } else {
        setTestResult('error');
        setTestError(result.error || '连接测试失败');
      }
    } catch (err) {
      console.error('[数据库连接测试] 异常:', err);
      setTestResult('error');
      setTestError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsTesting(false);
    }
  }, [formData]);

  // 保存连接
  const handleSave = useCallback(() => {
    if (!formData.name.trim()) return;
    onSave(formData);
    onOpenChange(false);
  }, [formData, onSave, onOpenChange]);

  // 关闭对话框
  const handleClose = useCallback(() => {
    onOpenChange(false);
  }, [onOpenChange]);

  const isSqlite = formData.driver === 'sqlite';

  return (
    <Dialog open={open} onOpenChange={(isOpen) => { if (!isOpen) handleClose(); }}>
      <DialogContent className="w-full max-w-[550px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {initialData ? '编辑数据库连接' : '配置数据库连接'}
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
              <Label htmlFor="conn-name">连接名称 <span className="text-rose-500">*</span></Label>
              <Input
                id="conn-name"
                value={formData.name}
                onChange={(e) => updateField('name', e.target.value)}
                placeholder="例如：生产环境 MySQL"
              />
            </div>

            {/* 数据库类型 */}
            <div className="space-y-2">
              <Label htmlFor="conn-driver">数据库类型</Label>
              <Select
                value={formData.driver}
                onValueChange={(v) => handleDriverChange(v as DbDriver)}
              >
                <SelectTrigger id="conn-driver">
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
                  <Label htmlFor="conn-host">主机地址</Label>
                  <Input
                    id="conn-host"
                    value={formData.host || ''}
                    onChange={(e) => updateField('host', e.target.value)}
                    placeholder="localhost"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="conn-port">端口</Label>
                  <Input
                    id="conn-port"
                    type="number"
                    value={formData.port || ''}
                    onChange={(e) => updateField('port', parseInt(e.target.value) || 0)}
                    placeholder="5432"
                  />
                </div>
              </div>

              {/* 数据库名 */}
              <div className="space-y-2">
                <Label htmlFor="conn-database">数据库名</Label>
                <Input
                  id="conn-database"
                  value={formData.database || ''}
                  onChange={(e) => updateField('database', e.target.value)}
                  placeholder="mydb"
                />
              </div>

              {/* 用户名和密码 */}
              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-2">
                  <Label htmlFor="conn-username">用户名</Label>
                  <Input
                    id="conn-username"
                    value={formData.username || ''}
                    onChange={(e) => updateField('username', e.target.value)}
                    placeholder="root"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="conn-password">密码</Label>
                  <div className="relative">
                    <Input
                      id="conn-password"
                      type={showPassword ? 'text' : 'password'}
                      value={formData.password || ''}
                      onChange={(e) => updateField('password', e.target.value)}
                      placeholder="••••••••"
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
                <Label htmlFor="conn-password-env">密码环境变量名（可选）</Label>
                <Input
                  id="conn-password-env"
                  value={formData.password_from_env || ''}
                  onChange={(e) => updateField('password_from_env', e.target.value)}
                  placeholder="DB_PASSWORD（优先于密码字段）"
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
                <Label htmlFor="conn-connection-string">数据库文件路径</Label>
                <Input
                  id="conn-connection-string"
                  value={formData.connection_string || ''}
                  onChange={(e) => updateField('connection_string', e.target.value)}
                  placeholder="/path/to/database.db"
                />
                <p className="text-xs text-muted-foreground">
                  SQLite 数据库文件的绝对路径或相对路径
                </p>
              </div>
            </div>
          )}

          {/* 连接池配置 */}
          <div className="space-y-4">
            <h4 className="text-sm font-medium text-foreground border-b border-border pb-1">
              连接池配置
            </h4>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-2">
                <Label htmlFor="conn-min-conn">最小连接数</Label>
                <Input
                  id="conn-min-conn"
                  type="number"
                  min={0}
                  value={formData.min_connections}
                  onChange={(e) => updateField('min_connections', parseInt(e.target.value) || 1)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="conn-max-conn">最大连接数</Label>
                <Input
                  id="conn-max-conn"
                  type="number"
                  min={1}
                  value={formData.max_connections}
                  onChange={(e) => updateField('max_connections', parseInt(e.target.value) || 10)}
                />
              </div>
            </div>
          </div>

          {/* 测试结果 */}
          {testResult && (
            <div
              className={cn(
                'p-3 rounded-lg',
                testResult === 'success' ? 'bg-green-50 dark:bg-green-950/50 text-green-700 dark:text-green-300' : 'bg-red-50 dark:bg-red-950/50 text-red-700 dark:text-red-300'
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
                    <div className="text-sm bg-red-100 dark:bg-red-900/50 rounded p-2 font-mono break-all">
                      {testError}
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={handleTest} disabled={isTesting}>
            {isTesting && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
            测试连接
          </Button>
          <Button variant="outline" onClick={handleClose}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={!formData.name.trim()}>
            保存
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
