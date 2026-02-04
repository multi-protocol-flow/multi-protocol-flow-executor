import React from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { Node } from '@xyflow/react';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';

/**
 * 表单验证 Schema
 */
const nodeConfigSchema = z.object({
  name: z
    .string()
    .min(1, '名称不能为空')
    .max(50, '名称不能超过 50 个字符'),
});

export type NodeConfigValues = z.infer<typeof nodeConfigSchema>;

export interface ConfigPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  node: Node | null;
  onSave: (values: NodeConfigValues) => void;
}

/**
 * 配置面板组件
 *
 * 功能：
 * - 使用 Sheet 作为侧边抽屉
 * - 使用 Form 包裹表单字段
 * - 实现名称编辑字段（Input，最多 50 字符）
 * - 实现表单验证（required, max length: 50）
 * - 实现保存和取消按钮
 */
export function ConfigPanel({
  open,
  onOpenChange,
  node,
  onSave,
}: ConfigPanelProps) {
  const form = useForm<NodeConfigValues>({
    resolver: zodResolver(nodeConfigSchema),
    defaultValues: {
      name: node?.data?.label || node?.data?.name || '',
    },
  });

  // 当节点变化时，更新表单默认值
  React.useEffect(() => {
    if (node) {
      form.reset({
        name: node.data?.label || node.data?.name || '',
      });
    }
  }, [node, form]);

  /**
   * 处理表单提交
   */
  const onSubmit = (values: NodeConfigValues) => {
    onSave(values);
    onOpenChange(false);
  };

  /**
   * 处理取消
   */
  const handleCancel = () => {
    form.reset();
    onOpenChange(false);
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full sm:max-w-md">
        <SheetHeader>
          <SheetTitle>节点配置</SheetTitle>
          <SheetDescription>
            编辑节点的名称和其他属性
          </SheetDescription>
        </SheetHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>节点名称</FormLabel>
                  <FormControl>
                    <Input
                      placeholder="请输入节点名称"
                      {...field}
                      maxLength={50}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <SheetFooter>
              <Button
                type="button"
                variant="outline"
                onClick={handleCancel}
              >
                取消
              </Button>
              <Button type="submit">保存</Button>
            </SheetFooter>
          </form>
        </Form>
      </SheetContent>
    </Sheet>
  );
}
