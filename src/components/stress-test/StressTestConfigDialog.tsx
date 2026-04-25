/**
 * StressTestConfigDialog - 压力测试配置对话框
 * 用于配置和启动压力测试，支持可选的 CSV 数据源
 */
import { useState, useCallback } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useFlowStore, useStressTestStore } from '@/stores';
import { datasetService } from '@/services/dataset';
import { AlertTriangle, ChevronDown, ChevronRight, FileText, Loader2 } from 'lucide-react';
import type { CsvInfo } from '@/types/dataset';
import type { DataPickStrategy } from '@/types/stressTest';

// ============================================================================
// 类型定义
// ============================================================================

interface StressTestConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// ============================================================================
// Zod Schema
// ============================================================================

const configSchema = z.object({
  concurrentUsers: z
    .number({ message: '请输入有效的数字' })
    .min(1, '并发用户数至少为 1'),
  rampUpSeconds: z
    .number({ message: '请输入有效的数字' })
    .min(0, '预热时长不能为负数'),
  durationSeconds: z
    .number({ message: '请输入有效的数字' })
    .min(1, '总运行时长至少为 1 秒'),
});

type ConfigFormData = z.infer<typeof configSchema>;

// ============================================================================
// 默认值
// ============================================================================

const DEFAULT_VALUES: ConfigFormData = {
  concurrentUsers: 50,
  rampUpSeconds: 10,
  durationSeconds: 60,
};

// ============================================================================
// 编码选项
// ============================================================================

const ENCODING_OPTIONS = [
  { value: 'auto', label: '自动检测' },
  { value: 'utf-8', label: 'UTF-8' },
  { value: 'gbk', label: 'GBK' },
  { value: 'gb2312', label: 'GB2312' },
  { value: 'big5', label: 'Big5' },
] as const;

// ============================================================================
// 策略选项
// ============================================================================

const STRATEGY_OPTIONS: { value: DataPickStrategy; label: string; description: string }[] = [
  { value: 'round_robin', label: 'RoundRobin（轮询）', description: '均匀循环使用每一行' },
  { value: 'random', label: 'Random（随机）', description: '每次迭代随机取一行' },
  { value: 'unique', label: 'Unique（独占）', description: '每个虚拟用户绑定固定行' },
];

// ============================================================================
// 组件
// ============================================================================

export function StressTestConfigDialog({
  open,
  onOpenChange,
}: StressTestConfigDialogProps) {
  const flow = useFlowStore((state) => state.flow);
  const lockFlow = useFlowStore((state) => state.lockFlow);
  const startStressTest = useStressTestStore((state) => state.startStressTest);
  const isLoading = useStressTestStore((state) => state.isLoading);

  const {
    register,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<ConfigFormData>({
    resolver: zodResolver(configSchema),
    defaultValues: DEFAULT_VALUES,
  });

  // 数据源相关状态
  const [isDataSourceOpen, setIsDataSourceOpen] = useState(false);
  const [datasetPath, setDatasetPath] = useState<string | null>(null);
  const [datasetName, setDatasetName] = useState<string>('');
  const [encoding, setEncoding] = useState<string>('auto');
  const [strategy, setStrategy] = useState<DataPickStrategy>('round_robin');
  const [csvInfo, setCsvInfo] = useState<CsvInfo | null>(null);
  const [isValidating, setIsValidating] = useState(false);
  const [datasetError, setDatasetError] = useState<string | null>(null);

  // 选择 CSV 文件
  const handleSelectCsv = useCallback(async () => {
    try {
      const selected = await openFileDialog({
        multiple: false,
        filters: [{ name: 'CSV 文件', extensions: ['csv', 'txt'] }],
      });

      if (!selected) return;

      const filePath = selected as string;
      const fileName = filePath.split(/[/\\]/).pop() || filePath;

      setIsValidating(true);
      setDatasetError(null);

      const encodingParam = encoding === 'auto' ? undefined : encoding;
      const info = await datasetService.validateCsv(filePath, encodingParam);

      setDatasetPath(filePath);
      setDatasetName(fileName);
      setCsvInfo(info);
    } catch (err) {
      setDatasetError((err as Error).message || 'CSV 文件验证失败');
      setDatasetPath(null);
      setDatasetName('');
      setCsvInfo(null);
    } finally {
      setIsValidating(false);
    }
  }, [encoding]);

  // 清除数据源
  const handleClearDataset = useCallback(() => {
    setDatasetPath(null);
    setDatasetName('');
    setCsvInfo(null);
    setEncoding('auto');
    setStrategy('round_robin');
    setDatasetError(null);
  }, []);

  // 编码变更时重新验证
  const handleEncodingChange = useCallback(async (newEncoding: string) => {
    setEncoding(newEncoding);
    if (datasetPath) {
      try {
        setIsValidating(true);
        setDatasetError(null);
        const encodingParam = newEncoding === 'auto' ? undefined : newEncoding;
        const info = await datasetService.validateCsv(datasetPath, encodingParam);
        setCsvInfo(info);
      } catch (err) {
        setDatasetError((err as Error).message || 'CSV 验证失败');
        setCsvInfo(null);
      } finally {
        setIsValidating(false);
      }
    }
  }, [datasetPath]);

  // 重置全部状态
  const resetAll = useCallback(() => {
    reset();
    setIsDataSourceOpen(false);
    setDatasetPath(null);
    setDatasetName('');
    setEncoding('auto');
    setStrategy('round_robin');
    setCsvInfo(null);
    setIsValidating(false);
    setDatasetError(null);
  }, [reset]);

  // 关闭对话框时重置表单
  const handleClose = () => {
    resetAll();
    onOpenChange(false);
  };

  // 提交表单
  const onSubmit = async (data: ConfigFormData) => {
    if (!flow) return;

    try {
      // 构建完整配置，包含可选的数据源字段
      const config = {
        ...data,
        flowUuid: flow.uuid,
        ...(datasetPath ? {
          datasetPath,
          datasetEncoding: encoding === 'auto' ? undefined : encoding,
          datasetStrategy: strategy,
        } : {}),
      };

      // 启动压力测试
      await startStressTest(flow, config);

      // 锁定流程
      lockFlow('压测进行中');

      // 关闭对话框
      handleClose();
    } catch {
      // 错误已在 store 中处理
    }
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && handleClose()}>
      <DialogContent className="w-full max-w-[425px] max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>压力测试配置</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          {/* 警告提示 */}
          <div className="flex items-start gap-2 p-3 bg-amber-50 border border-amber-200 rounded-md text-amber-700 text-sm">
            <AlertTriangle className="w-4 h-4 mt-0.5 flex-shrink-0" />
            <span>压测期间流程将被锁定，无法编辑</span>
          </div>

          {/* 并发用户数 */}
          <div className="space-y-2">
            <Label htmlFor="concurrentUsers">并发用户数</Label>
            <Input
              id="concurrentUsers"
              type="number"
              min={1}
              {...register('concurrentUsers', { valueAsNumber: true })}
              disabled={isLoading}
            />
            {errors.concurrentUsers && (
              <p className="text-sm text-red-500">
                {errors.concurrentUsers.message}
              </p>
            )}
          </div>

          {/* 预热时长 */}
          <div className="space-y-2">
            <Label htmlFor="rampUpSeconds">预热时长（秒）</Label>
            <Input
              id="rampUpSeconds"
              type="number"
              min={0}
              {...register('rampUpSeconds', { valueAsNumber: true })}
              disabled={isLoading}
            />
            {errors.rampUpSeconds && (
              <p className="text-sm text-red-500">
                {errors.rampUpSeconds.message}
              </p>
            )}
          </div>

          {/* 总运行时长 */}
          <div className="space-y-2">
            <Label htmlFor="durationSeconds">总运行时长（秒）</Label>
            <Input
              id="durationSeconds"
              type="number"
              min={1}
              {...register('durationSeconds', { valueAsNumber: true })}
              disabled={isLoading}
            />
            {errors.durationSeconds && (
              <p className="text-sm text-red-500">
                {errors.durationSeconds.message}
              </p>
            )}
          </div>

          {/* ========== 数据源（可选）折叠区 ========== */}
          <div className="border border-border rounded-md">
            {/* 折叠标题 */}
            <button
              type="button"
              className="flex items-center gap-2 w-full px-3 py-2 text-sm font-medium text-foreground hover:bg-accent transition-colors rounded-md"
              onClick={() => setIsDataSourceOpen((prev) => !prev)}
            >
              {isDataSourceOpen ? (
                <ChevronDown className="h-4 w-4 text-muted-foreground" />
              ) : (
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              )}
              数据源（可选）
            </button>

            {/* 折叠内容 */}
            {isDataSourceOpen && (
              <div className="px-3 pb-3 space-y-3 border-t border-border pt-3">
                {!datasetPath ? (
                  <div>
                    <p className="text-sm text-muted-foreground">
                      不选择数据源时，所有迭代使用相同参数
                    </p>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={handleSelectCsv}
                      className="mt-2"
                      disabled={isValidating}
                    >
                      {isValidating ? (
                        <>
                          <Loader2 className="h-4 w-4 animate-spin" />
                          验证中...
                        </>
                      ) : (
                        '选择 CSV 文件'
                      )}
                    </Button>
                  </div>
                ) : (
                  <div className="space-y-3">
                    {/* 已选文件信息 */}
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2 min-w-0">
                        <FileText className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
                        <span className="text-sm truncate">{datasetName}</span>
                      </div>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={handleClearDataset}
                        disabled={isValidating}
                      >
                        清除
                      </Button>
                    </div>

                    {/* CSV 信息 */}
                    {csvInfo && (
                      <div className="text-xs text-muted-foreground space-y-0.5">
                        <p>{csvInfo.row_count} 行数据 · 检测编码: {csvInfo.detected_encoding}</p>
                        <p className="truncate">列: {csvInfo.column_names.join(', ')}</p>
                      </div>
                    )}

                    {/* 验证错误 */}
                    {datasetError && (
                      <p className="text-sm text-red-500">{datasetError}</p>
                    )}

                    {/* 编码选择器 */}
                    <div className="space-y-1.5">
                      <Label className="text-xs">编码</Label>
                      <Select value={encoding} onValueChange={handleEncodingChange}>
                        <SelectTrigger>
                          <SelectValue placeholder="自动检测" />
                        </SelectTrigger>
                        <SelectContent>
                          {ENCODING_OPTIONS.map((opt) => (
                            <SelectItem key={opt.value} value={opt.value}>
                              {opt.label}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>

                    {/* 策略选择器 */}
                    <div className="space-y-1.5">
                      <Label className="text-xs">取行策略</Label>
                      <Select value={strategy} onValueChange={(v) => setStrategy(v as DataPickStrategy)}>
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {STRATEGY_OPTIONS.map((opt) => (
                            <SelectItem key={opt.value} value={opt.value}>
                              <div className="flex flex-col">
                                <span>{opt.label}</span>
                                <span className="text-xs text-muted-foreground">{opt.description}</span>
                              </div>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>

                    {/* 数据预览（列名 + 行数摘要） */}
                    {csvInfo && csvInfo.column_names.length > 0 && (
                      <div className="overflow-x-auto border border-border rounded-md">
                        <table className="w-full text-xs">
                          <thead>
                            <tr className="bg-muted/50">
                              <th className="px-2 py-1 text-left font-medium text-muted-foreground">#</th>
                              {csvInfo.column_names.map((col) => (
                                <th
                                  key={col}
                                  className="px-2 py-1 text-left font-medium text-muted-foreground whitespace-nowrap"
                                >
                                  {col}
                                </th>
                              ))}
                            </tr>
                          </thead>
                          <tbody>
                            <tr className="border-t border-border">
                              <td
                                colSpan={csvInfo.column_names.length + 1}
                                className="px-2 py-2 text-center text-muted-foreground"
                              >
                                共 {csvInfo.row_count} 行数据
                              </td>
                            </tr>
                          </tbody>
                        </table>
                      </div>
                    )}

                    {/* 重新验证按钮 */}
                    {isValidating && (
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        正在验证文件...
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={handleClose}
              disabled={isLoading}
            >
              取消
            </Button>
            <Button type="submit" disabled={isLoading || !flow}>
              {isLoading ? '启动中...' : '开始压测'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
