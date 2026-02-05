import React from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import type { Node } from '@xyflow/react';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from './ui/sheet';
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from './ui/form';
import { Input } from './ui/input';
import { Button } from './ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu';

/**
 * HTTP 请求头类型
 */
type HttpHeader = {
  name: string;
  value: string;
};

/**
 * HTTP 配置类型
 */
type HttpConfig = {
  url?: string;
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE';
  headers?: HttpHeader[];
};

/**
 * 表单验证 Schema
 */
const nodeConfigSchema = z.object({
  name: z
    .string()
    .min(1, '名称不能为空')
    .max(50, '名称不能超过 50 个字符'),
  // HTTP 配置字段（可选）
  url: z.string().url('Invalid URL format').optional(),
  method: z.enum(['GET', 'POST', 'PUT', 'DELETE']).optional(),
  headers: z.array(z.object({ name: z.string(), value: z.string() })).optional(),
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
      name: (node?.data?.label as string) || (node?.data?.name as string) || '',
      url: (node?.data?.url as string) || '',
      method: (node?.data?.method as 'GET' | 'POST' | 'PUT' | 'DELETE') || 'GET',
      headers: (node?.data?.headers as HttpHeader[]) || [],
    },
  });

  // 当节点变化时，更新表单默认值
  React.useEffect(() => {
    if (node) {
      form.reset({
        name: (node.data?.label as string) || (node.data?.name as string) || '',
        url: (node.data?.url as string) || '',
        method: (node.data?.method as 'GET' | 'POST' | 'PUT' | 'DELETE') || 'GET',
        headers: (node.data?.headers as HttpHeader[]) || [],
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

            {/* HTTP 节点特定配置 */}
            {node?.data?.taskType === 'http' && (
              <>
                <FormField
                  control={form.control}
                  name="url"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>请求 URL</FormLabel>
                      <FormControl>
                        <Input
                          type="url"
                          placeholder="https://api.example.com"
                          className="nodrag"
                          {...field}
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="method"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>HTTP 方法</FormLabel>
                      <FormControl>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="outline" className="w-full justify-start nodrag">
                              {field.value || 'GET'}
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent>
                            {['GET', 'POST', 'PUT', 'DELETE'].map((method) => (
                              <DropdownMenuItem
                                key={method}
                                onClick={() => field.onChange(method)}
                              >
                                {method}
                              </DropdownMenuItem>
                            ))}
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="headers"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>请求头</FormLabel>
                      <div className="space-y-2">
                        {field.value?.map((header, index) => (
                          <div key={`${header.name}-${header.value}-${index}`} className="flex gap-2 items-start">
                            <FormControl>
                              <Input
                                placeholder="Header name"
                                className="nodrag flex-1"
                                value={header.name}
                                onChange={(e) => {
                                  const newHeaders = [...(field.value || [])];
                                  newHeaders[index] = { ...newHeaders[index], name: e.target.value };
                                  field.onChange(newHeaders);
                                }}
                              />
                            </FormControl>
                            <FormControl>
                              <Input
                                placeholder="Header value"
                                className="nodrag flex-1"
                                value={header.value}
                                onChange={(e) => {
                                  const newHeaders = [...(field.value || [])];
                                  newHeaders[index] = { ...newHeaders[index], value: e.target.value };
                                  field.onChange(newHeaders);
                                }}
                              />
                            </FormControl>
                            <Button
                              type="button"
                              variant="destructive"
                              size="sm"
                              className="nodrag"
                              onClick={() => {
                                const newHeaders = [...(field.value || [])];
                                newHeaders.splice(index, 1);
                                field.onChange(newHeaders);
                              }}
                            >
                              删除
                            </Button>
                          </div>
                        ))}
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          className="nodrag"
                          onClick={() => {
                            const newHeaders = [...(field.value || [])];
                            newHeaders.push({ name: '', value: '' });
                            field.onChange(newHeaders);
                          }}
                        >
                          添加请求头
                        </Button>
                      </div>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </>
            )}

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
