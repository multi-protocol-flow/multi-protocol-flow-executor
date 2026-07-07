/**
 * ConnectionDialog - 数据库连接编辑对话框（受控组件）
 * 通过 props 控制打开/关闭状态，表单数据通过 onSave 回调返回
 */
import { useState, useEffect, useCallback, useMemo } from 'react';
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
import { Loader2, CheckCircle, XCircle } from 'lucide-react';
import { VariableInput } from '@/components/ui/VariableInput';
import { useFlowVariables } from '@/hooks';
import { getLL } from '@/i18n/i18n-lazy';
import { isTauriEnv } from '@/lib/platform';

/**
 * 检查是否在 Tauri 环境
 */

/**
 * 解析字符串中的 {{variable}} 引用，替换为变量实际值
 * 变量未找到时保留原始 {{name}} 不变
 */
function resolveVariables(
  value: string | undefined | null,
  varMap: Map<string, unknown>,
): string | undefined | null {
  if (!value) return value;
  return value.replace(
    /\{\{([^}]+)\}\}/g,
    (match: string, varPath: string) => {
      const trimmed = varPath.trim();
      if (varMap.has(trimmed)) {
        const resolved = varMap.get(trimmed);
        if (resolved !== null && resolved !== undefined) {
          return String(resolved);
        }
      }
      // 保留原始模板，让后端报错时能看到
      return match;
    },
  );
}

/** 连接表单数据类型（导出供调用方使用） */
export interface ConnectionFormData {
  name: string;
  driver: DbDriver;
  host?: string;
  /** 端口，支持 {{variable}} 引用，使用 string 类型 */
  port?: string;
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
const DB_DRIVERS: { value: DbDriver; label: string; defaultPort: string }[] = [
  { value: 'postgres', label: 'PostgreSQL', defaultPort: '5432' },
  { value: 'mysql', label: 'MySQL', defaultPort: '3306' },
  { value: 'sqlite', label: 'SQLite', defaultPort: '' },

];

/** 默认连接配置 */
function getDefaultConnection(): ConnectionFormData {
  return {
    name: '',
    driver: 'postgres',
    host: 'localhost',
    port: '5432',
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
  const LL = getLL();
  const [formData, setFormData] = useState<ConnectionFormData>(buildInitialFormData(initialData));
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);
  const [testError, setTestError] = useState<string | null>(null);
  const { variables, nodeLabels } = useFlowVariables();
  // 构建变量名→值的映射，用于测试连接时预替换 {{variable}}
  const varMap = useMemo(
    () => new Map(variables.map((v) => [v.name, v.value])),
    [variables]
  );

  // 当对话框打开时，初始化表单数据
  useEffect(() => {
    if (open) {
      setFormData(buildInitialFormData(initialData));
      setTestResult(null);
      setTestError(null);
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

    try {
      if (!isTauriEnv()) {
        console.warn('[数据库连接测试] 非 Tauri 环境，跳过真实连接测试');
        await new Promise((resolve) => setTimeout(resolve, 500));
        setTestResult('success');
        setIsTesting(false);
        return;
      }

      // 预替换连接字段中的 {{variable}} 为实际值
      // 注意：变量是拍平存储的，使用 {{name}} 而非 {{initial_variables.name}}
      const resolvedHost = resolveVariables(formData.host, varMap);
      const resolvedPort = resolveVariables(formData.port, varMap);
      const resolvedDatabase = resolveVariables(formData.database, varMap);
      const resolvedUsername = resolveVariables(formData.username, varMap);
      const resolvedPassword = resolveVariables(formData.password, varMap);
      const resolvedConnectionString = resolveVariables(
        formData.connection_string,
        varMap,
      );

      // 解析端口为数值（后端 DbConnection 仍使用 Option<u16>）
      const parsedPort = resolvedPort ? parseInt(resolvedPort, 10) : null;

      // 动态导入 invoke，避免非 Tauri 环境加载报错
      const { invoke } = await import('@tauri-apps/api/core');

      const result = await invoke<TestDbConnectionResult>('test_db_connection', {
        connection: {
          id: `test-${Date.now()}`,
          name: formData.name || LL.common.label_test_connection(),
          driver: formData.driver,
          host: resolvedHost || null,
          port: parsedPort,
          database: resolvedDatabase || null,
          username: resolvedUsername || null,
          password: resolvedPassword || null,
          passwordFromEnv: formData.password_from_env || null,
          connectionString: resolvedConnectionString || null,
          minConnections: formData.min_connections,
          maxConnections: formData.max_connections,
          connectTimeoutMs: formData.connect_timeout_ms,
          idleTimeoutMs: formData.idle_timeout_ms,
          status: 'disconnected',
          lastError: null,
          lastTestedAt: null,
        },
      });

      if (result.success) {
        setTestResult('success');
      } else {
        setTestResult('error');
        setTestError(result.error || LL.flow.db_conn_test_fail());
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
            {initialData ? LL.flow.db_conn_edit_title() : LL.flow.db_conn_new_title()}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-5 py-4">
          {/* 基本信息区 */}
          <div className="space-y-4">
            <h4 className="text-sm font-medium text-foreground border-b border-border pb-1">
              {LL.flow.db_conn_basic()}
            </h4>

            {/* 连接名称 */}
            <div className="space-y-2">
              <Label htmlFor="conn-name">{LL.flow.db_conn_name()} <span className="text-rose-500">*</span></Label>
              <Input
                id="conn-name"
                value={formData.name}
                onChange={(e) => updateField('name', e.target.value)}
                placeholder={LL.flow.db_conn_name_placeholder_name()}
              />
            </div>

            {/* 数据库类型 */}
            <div className="space-y-2">
              <Label htmlFor="conn-driver">{LL.flow.db_conn_driver()}</Label>
              <Select
                value={formData.driver}
                onValueChange={(v) => handleDriverChange(v as DbDriver)}
              >
                <SelectTrigger id="conn-driver">
                  <SelectValue placeholder={LL.flow.db_conn_driver_placeholder()} />
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
                {LL.flow.db_conn_params()}
              </h4>

              {/* 连接字符串（对所有数据库类型可见，允许用户重置此字段） */}
              <div className="space-y-2">
                <Label htmlFor="conn-connection-string-non-sqlite">
                  {LL.flow.db_conn_connection_string()}
                </Label>
                <VariableInput
                  id="conn-connection-string-non-sqlite"
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={formData.connection_string || ''}
                  onValueChange={(value) => updateField('connection_string', value)}
                  placeholder={isSqlite ? '/path/to/database.db' : 'mysql://user:pass@host:3306/db'}
                />
                <p className="text-xs text-muted-foreground">
                  {LL.flow.db_conn_connection_string_hint()}
                </p>
              </div>

              {/* 主机和端口（SQLite 不使用） */}
              <div className="grid grid-cols-3 gap-3">
                <div className="col-span-2 space-y-2">
                  <Label htmlFor="conn-host">{LL.flow.db_conn_host()}</Label>
                  <VariableInput
                    id="conn-host"
                    variables={variables}
                    nodeLabels={nodeLabels}
                    value={formData.host || ''}
                    onValueChange={(value) => updateField('host', value)}
                    placeholder={LL.flow.db_conn_host_placeholder()}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="conn-port">{LL.flow.db_conn_port()}</Label>
                  <Input
                    id="conn-port"
                    type="text"
                    value={formData.port || ''}
                    onChange={(e) => updateField('port', e.target.value)}
                    placeholder={LL.flow.db_conn_port_placeholder()}
                  />
                </div>
              </div>

              {/* 数据库名 */}
              <div className="space-y-2">
                <Label htmlFor="conn-database">{LL.flow.db_conn_database()}</Label>
                <VariableInput
                  id="conn-database"
                  variables={variables}
                  nodeLabels={nodeLabels}
                  value={formData.database || ''}
                  onValueChange={(value) => updateField('database', value)}
                    placeholder={LL.flow.db_conn_database_placeholder()}
                />
              </div>

              {/* 用户名和密码 */}
              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-2">
                  <Label htmlFor="conn-username">{LL.flow.db_conn_username()}</Label>
                  <VariableInput
                    id="conn-username"
                    variables={variables}
                    nodeLabels={nodeLabels}
                    value={formData.username || ''}
                    onValueChange={(value) => updateField('username', value)}
                    placeholder={LL.flow.db_conn_username_placeholder()}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="conn-password">{LL.flow.db_conn_password()}</Label>
                  <Input
                    id="conn-password"
                    type="text"
                    value={formData.password || ''}
                    onChange={(e) => updateField('password', e.target.value)}
                    placeholder={LL.flow.db_conn_password_placeholder()}
                  />
                </div>
              </div>

              {/* 环境变量引用 */}
              <div className="space-y-2">
                <Label htmlFor="conn-password-env">{LL.flow.db_conn_password_env()}</Label>
                <Input
                  id="conn-password-env"
                  value={formData.password_from_env || ''}
                  onChange={(e) => updateField('password_from_env', e.target.value)}
                  placeholder={LL.flow.db_conn_password_env_placeholder()}
                />
                <p className="text-xs text-muted-foreground">
                  {LL.flow.db_conn_password_env_hint()}
                </p>
              </div>
            </div>
          )}

              {/* SQLite 文件路径 */}
              {isSqlite && (
                <div className="space-y-4">
                  <h4 className="text-sm font-medium text-foreground border-b border-border pb-1">
                    {LL.flow.db_conn_params()}
                  </h4>
                  <div className="space-y-2">
                    <Label htmlFor="conn-connection-string">{LL.flow.db_conn_sqlite_path()}</Label>
                    <VariableInput
                      id="conn-connection-string"
                      variables={variables}
                      nodeLabels={nodeLabels}
                      value={formData.connection_string || ''}
                      onValueChange={(value) => updateField('connection_string', value)}
                      placeholder={LL.flow.db_conn_sqlite_path_placeholder()}
                    />
                    <p className="text-xs text-muted-foreground">
                      {LL.flow.db_conn_sqlite_hint()}
                    </p>
                  </div>
                </div>
              )}

          {/* 连接池配置 */}
          <div className="space-y-4">
            <h4 className="text-sm font-medium text-foreground border-b border-border pb-1">
              {LL.flow.db_conn_pool()}
            </h4>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-2">
                <Label htmlFor="conn-min-conn">{LL.flow.db_conn_min_conn()}</Label>
                <Input
                  id="conn-min-conn"
                  type="number"
                  min={0}
                  value={formData.min_connections}
                  onChange={(e) => updateField('min_connections', parseInt(e.target.value) || 1)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="conn-max-conn">{LL.flow.db_conn_max_conn()}</Label>
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
                  <span>{LL.flow.db_conn_test_success()}</span>
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <XCircle className="w-5 h-5" />
                    <span>{LL.flow.db_conn_test_fail()}</span>
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
            {LL.flow.db_conn_test_btn()}
          </Button>
          <Button variant="outline" onClick={handleClose}>
            {LL.common.CANCEL()}
          </Button>
          <Button onClick={handleSave} disabled={!formData.name.trim()}>
            {LL.flow.db_conn_save()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
