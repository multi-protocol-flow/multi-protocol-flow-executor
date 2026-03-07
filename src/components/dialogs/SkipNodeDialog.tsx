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
  // 选中的输出端口
  const [selectedPort, setSelectedPort] = useState<string>('');
  // 模拟输出 JSON 字符串
  const [simulatedOutputText, setSimulatedOutputText] = useState<string>('');
  // JSON 解析错误
  const [jsonError, setJsonError] = useState<string | null>(null);

  // 是否需要模拟输出（仅 HTTP/WebSocket 节点）
  const needsSimulatedOutput = nodeType === 'http' || nodeType === 'ws_connect' || nodeType === 'ws_send_wait';

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
      const example = nodeType === 'http' 
        ? { status: 200, body: { message: '模拟响应' }, headers: {} }
        : { message: '模拟消息', timestamp: Date.now() };
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
      setJsonError('JSON 格式错误');
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
      toast.error('请选择输出分支');
      return;
    }

    // 如果需要模拟输出，验证 JSON
    if (needsSimulatedOutput && simulatedOutputText.trim()) {
      if (!validateJson(simulatedOutputText)) {
        toast.error('模拟输出 JSON 格式错误');
        return;
      }
      try {
        const simulatedOutput = JSON.parse(simulatedOutputText);
        onConfirm(selectedPort, simulatedOutput);
      } catch {
        toast.error('模拟输出 JSON 解析失败');
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
        return '条件分支';
      case 'http':
        return 'HTTP 请求';
      case 'ws_connect':
        return 'WebSocket 连接';
      case 'ws_send_wait':
        return 'WebSocket 发送等待';
      case 'ws_close':
        return 'WebSocket 关闭';
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
            <DialogTitle className="text-lg">跳过节点</DialogTitle>
          </div>
        </DialogHeader>

        <DialogDescription className="text-sm text-slate-500 pt-2">
          跳过节点「{nodeName}」({getNodeTypeName()})，不执行该节点
        </DialogDescription>

        <div className="space-y-4 py-4">
          {/* 分支选择 */}
          <div className="space-y-2">
            <Label className="text-sm font-medium">选择执行分支</Label>
            <Select value={selectedPort} onValueChange={setSelectedPort}>
              <SelectTrigger>
                <SelectValue placeholder="请选择分支" />
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
                模拟输出 <span className="text-slate-400 font-normal">(可选)</span>
              </Label>
              <p className="text-xs text-slate-500">
                提供模拟输出数据，将注入到变量池供后续节点使用
              </p>
              <Textarea
                value={simulatedOutputText}
                onChange={(e) => handleSimulatedOutputChange(e.target.value)}
                placeholder='{"status": 200, "body": {}}'
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
                <strong>注意：</strong>跳过条件节点不会产生任何输出变量，只会按照选择的分支继续执行
              </p>
            </div>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button variant="outline" onClick={onCancel}>
            取消
          </Button>
          <Button onClick={handleConfirm} disabled={!selectedPort}>
            确认跳过
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
