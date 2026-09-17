/**
 * SkipNodeDialog - 跳过节点对话框
 * 用于调试模式下跳过节点时选择分支和模拟输出
 */
import { useState, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { SkipForward, AlertCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import type { NodeType } from '@/types/node';

export interface SkipNodeDialogProps {
  /** 是否显示对话框 */
  open: boolean;
  /** 当前节点类型 */
  nodeType: NodeType;
  /** 当前节点名称 */
  nodeName: string;
  /** 可用的输出端口列表 */
  outputPorts: Array<{ id: string; label: string }>;
  /** 确认回调（selectedPort 和 simulatedOutput） */
  onConfirm: (selectedPort: string, simulatedOutput?: unknown) => void;
  /** 取消回调 */
  onCancel: () => void;
}

export function SkipNodeDialog({
  open,
  nodeType,
  nodeName,
  outputPorts,
  onConfirm,
  onCancel,
}: SkipNodeDialogProps) {
  const ll = getLL();
  // 选中的输出端口
  const [selectedPort, setSelectedPort] = useState<string>('');
  // 模拟输出 JSON 字符串
  const [simulatedOutputText, setSimulatedOutputText] = useState<string>('');
  // JSON 解析错误
  const [jsonError, setJsonError] = useState<string | null>(null);

  // 是否需要模拟输出（仅 HTTP/WebSocket 节点）
  const needsSimulatedOutput = nodeType === 'http:request' || nodeType === 'ws:connect' || nodeType === 'ws:send_collect';

  // 初始化选中的端口（默认选择第一个端口）
  useEffect(() => {
    if (outputPorts.length > 0 && !selectedPort) {
      setSelectedPort(outputPorts[0].id);
    }
  }, [outputPorts, selectedPort]);

  // 初始化模拟输出示例
  useEffect(() => {
    if (needsSimulatedOutput && !simulatedOutputText) {
      // 提供一个示例 JSON
      const example = nodeType === 'http:request' 
        ? { status: 200, body: { message: 'Simulated response' }, headers: {} }
        : { message: 'Simulated message', timestamp: Date.now() };
      setSimulatedOutputText(JSON.stringify(example, null, 2));
    }
  }, [needsSimulatedOutput, nodeType, simulatedOutputText]);

  // 验证 JSON 格式
  const validateJson = (text: string): boolean => {
    if (!text.trim()) {
      setJsonError(null);
      return true;
    }
    try {
      JSON.parse(text);
      setJsonError(null);
      return true;
    } catch {
      setJsonError(ll.flow.skip_json_error());
      return false;
    }
  };

  // 处理模拟输出文本变化
  const handleSimulatedOutputChange = (value: string) => {
    setSimulatedOutputText(value);
    validateJson(value);
  };

  // 确认跳过
  const handleConfirm = () => {
    if (!selectedPort) {
      toast.error(ll.flow.skip_select_branch_error());
      return;
    }

    // 如果需要模拟输出，验证 JSON
    if (needsSimulatedOutput && simulatedOutputText.trim()) {
      if (!validateJson(simulatedOutputText)) {
        toast.error(ll.flow.skip_json_format_error());
        return;
      }
      try {
        const simulatedOutput = JSON.parse(simulatedOutputText);
        onConfirm(selectedPort, simulatedOutput);
      } catch {
        toast.error(ll.flow.skip_json_parse_error());
        return;
      }
    } else {
      onConfirm(selectedPort);
    }
  };

  // 获取节点类型的中文名称
  const getNodeTypeName = () => {
    switch (nodeType) {
      case 'conditional':
        return ll.flow.skip_node_type_conditional();
      case 'http:request':
        return ll.flow.skip_node_type_http();
      case 'ws:connect':
        return ll.flow.skip_node_type_ws_connect();
      case 'ws:send_collect':
        return ll.flow.skip_node_type_ws_send_collect();
      case 'ws:close':
        return ll.flow.skip_node_type_ws_close();
      default:
        return nodeType;
    }
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onCancel()}>
      <DialogContent className="w-full max-w-[500px]">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-blue-100 flex items-center justify-center">
              <SkipForward className="w-5 h-5 text-blue-500" />
            </div>
            <DialogTitle className="text-lg">{ll.flow.skip_node_title()}</DialogTitle>
          </div>
        </DialogHeader>

        <DialogDescription className="text-sm text-muted-foreground pt-2">
          {ll.flow.skip_node_desc({ name: nodeName, type: getNodeTypeName() })}
        </DialogDescription>

        <div className="space-y-4 py-4">
          {/* 分支选择 */}
          <div className="space-y-2">
            <Label className="text-sm font-medium">{ll.flow.skip_select_branch()}</Label>
            <Select value={selectedPort} onValueChange={setSelectedPort}>
              <SelectTrigger>
                <SelectValue placeholder={ll.flow.skip_select_branch_placeholder()} />
              </SelectTrigger>
              <SelectContent>
                {outputPorts.map((port) => (
                  <SelectItem key={port.id} value={port.id}>
                    {port.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* 模拟输出（仅 HTTP/WebSocket 节点） */}
          {needsSimulatedOutput && (
            <div className="space-y-2">
              <Label className="text-sm font-medium">
                {ll.flow.skip_simulated_output()} <span className="text-muted-foreground font-normal">({ll.flow.skip_simulated_output_optional()})</span>
              </Label>
              <p className="text-xs text-muted-foreground">
                {ll.flow.skip_simulated_output_hint()}
              </p>
              <Textarea
                value={simulatedOutputText}
                onChange={(e) => handleSimulatedOutputChange(e.target.value)}
                placeholder={ll.flow.skip_http_example()}
                className={cn(
                  'font-mono text-xs min-h-[120px]',
                  jsonError && 'border-red-500 focus:ring-red-500'
                )}
              />
              {jsonError && (
                <div className="flex items-center gap-1 text-xs text-red-500">
                  <AlertCircle className="w-3 h-3" />
                  <span>{jsonError}</span>
                </div>
              )}
            </div>
          )}

          {/* 提示信息 */}
          {nodeType === 'conditional' && (
            <div className="p-3 bg-amber-50 border border-amber-200 rounded-lg">
              <p className="text-xs text-amber-800">
                <strong>{ll.flow.skip_conditional_notice_title()}</strong>{ll.flow.skip_conditional_notice()}
              </p>
            </div>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button variant="outline" onClick={onCancel}>
            {ll.flow.cancel()}
          </Button>
          <Button onClick={handleConfirm} disabled={!selectedPort}>
            {ll.flow.skip_confirm_label()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
