/**
 * ValidationErrorDialog - 流程验证错误对话框
 * 用于展示流程验证失败或文件损坏时的错误信息
 * 支持 i18n 错误码映射和详细错误消息展示
 */
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertTriangle, FileX2, AlertCircle } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { getLL } from '@/i18n/i18n-lazy';
import type { ValidationEntry } from '@/types/ipc';
import { resolveErrorCode } from '@/services/tauri/flow';

export interface ValidationErrorDialogProps {
  /** 是否显示对话框 */
  open: boolean;
  /** 关闭回调 */
  onClose: () => void;
  /** 错误类型 */
  type?: 'validation' | 'corrupted';
  /** 错误标题（可选，默认使用 i18n） */
  title?: string;
  /** 错误描述（可选，默认使用 i18n） */
  description?: string;
  /** 结构化错误列表（含 code + message） */
  errors: ValidationEntry[];
  /** 警告信息列表（可选） */
  warnings?: string[];
}

export function ValidationErrorDialog({
  open,
  onClose,
  type = 'validation',
  title,
  description,
  errors,
  warnings,
}: ValidationErrorDialogProps) {
  const ll = getLL();

  const dialogTitle = title || (
    type === 'corrupted'
      ? ll.flow.file_corrupted_title()
      : ll.flow.validation_error_title()
  );

  const dialogDesc = description || (
    type === 'corrupted'
      ? ll.flow.file_corrupted_desc()
      : ll.flow.validation_error_desc()
  );

  const hasContent = errors.length > 0 || (warnings && warnings.length > 0);

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="w-full max-w-[540px]">
        <DialogHeader>
          <div className="flex items-start gap-3">
            <div className={`w-10 h-10 rounded-full flex items-center justify-center shrink-0 ${
              type === 'corrupted' ? 'bg-red-100' : 'bg-amber-100'
            }`}>
              {type === 'corrupted' ? (
                <FileX2 className="w-5 h-5 text-red-500" />
              ) : (
                <AlertTriangle className="w-5 h-5 text-amber-500" />
              )}
            </div>
            <div className="min-w-0">
              <DialogTitle className="text-lg">{dialogTitle}</DialogTitle>
              {dialogDesc && (
                <DialogDescription className="text-sm text-muted-foreground pt-1">
                  {dialogDesc}
                </DialogDescription>
              )}
            </div>
          </div>
        </DialogHeader>

        {hasContent && (
          <ScrollArea className="max-h-[360px] rounded-md border bg-muted/30">
            <div className="p-3 space-y-3">
              {/* 错误列表 */}
              {errors.map((error, index) => {
                const localizedSummary = resolveErrorCode(error.code);
                return (
                  <div key={index} className="flex items-start gap-2 text-sm">
                    <AlertCircle className="w-4 h-4 text-red-500 mt-0.5 shrink-0" />
                    <div className="min-w-0 space-y-1">
                      {/* 第一行：i18n 摘要（当前语言）+ 原始详情 */}
                      <p className="text-foreground font-medium leading-snug">
                        {localizedSummary || error.message}
                      </p>
                      {localizedSummary && (
                        <p className="text-muted-foreground text-xs leading-relaxed break-words">
                          {error.message}
                        </p>
                      )}
                    </div>
                  </div>
                );
              })}

              {/* 警告列表 */}
              {warnings && warnings.length > 0 && (
                <div className="pt-1 space-y-2">
                  {warnings.map((warning, index) => (
                    <div key={`w-${index}`} className="flex items-start gap-2 text-sm text-amber-600">
                      <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
                      <span>{warning}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </ScrollArea>
        )}

        <DialogFooter>
          <Button variant="default" onClick={onClose}>
            {ll.common.CONFIRM()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
