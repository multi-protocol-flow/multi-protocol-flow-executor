/**
 * ConfirmDialog - 通用确认对话框组件
 * 用于删除、清空等危险操作的二次确认
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
import { AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

export interface ConfirmDialogProps {
  /** 是否显示对话框 */
  open: boolean;
  /** 对话框标题 */
  title: string;
  /** 对话框描述（可选） */
  description?: string;
  /** 确认按钮文本 */
  confirmText?: string;
  /** 取消按钮文本 */
  cancelText?: string;
  /** 确认按钮变体样式 */
  confirmVariant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  /** 确认回调 */
  onConfirm: () => void;
  /** 取消/关闭回调 */
  onCancel: () => void;
  /** 是否加载中 */
  loading?: boolean;
}

export function ConfirmDialog({
  open,
  title,
  description,
  confirmText = getLL().common.CONFIRM(),
  cancelText = getLL().common.CANCEL(),
  confirmVariant = 'destructive',
  onConfirm,
  onCancel,
  loading = false,
}: ConfirmDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onCancel()}>
      <DialogContent className="w-full max-w-[400px]">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className={cn(
              'w-10 h-10 rounded-full flex items-center justify-center',
              confirmVariant === 'destructive' ? 'bg-red-100' : 'bg-muted'
            )}>
              <AlertTriangle className={cn(
                'w-5 h-5',
                confirmVariant === 'destructive' ? 'text-red-500' : 'text-muted-foreground'
              )} />
            </div>
            <DialogTitle className="text-lg">{title}</DialogTitle>
          </div>
        </DialogHeader>
        
        {description && (
          <DialogDescription className="text-sm text-muted-foreground pt-2">
            {description}
          </DialogDescription>
        )}
        
        <DialogFooter className="gap-2 sm:gap-0">
          <Button 
            variant="outline" 
            onClick={onCancel} 
            disabled={loading}
          >
            {cancelText}
          </Button>
          <Button 
            variant={confirmVariant} 
            onClick={onConfirm} 
            disabled={loading}
          >
            {loading ? getLL().flow.processing() : confirmText}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
