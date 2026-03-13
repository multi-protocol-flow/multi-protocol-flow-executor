/**
 * StressTestConfigDialog - 压力测试配置对话框
 * 用于配置和启动压力测试
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
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
import { useFlowStore, useStressTestStore } from '@/stores';
import { AlertTriangle } from 'lucide-react';

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

  // 关闭对话框时重置表单
  const handleClose = () => {
    reset();
    onOpenChange(false);
  };

  // 提交表单
  const onSubmit = async (data: ConfigFormData) => {
    if (!flow) return;

    try {
      // 1. 启动压力测试
      await startStressTest(flow, {
        ...data,
        flowUuid: flow.uuid,
      });

      // 2. 锁定流程
      lockFlow('压测进行中');

      // 3. 关闭对话框
      handleClose();
    } catch {
      // 错误已在 store 中处理
    }
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && handleClose()}>
      <DialogContent className="w-full max-w-[425px]">
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
