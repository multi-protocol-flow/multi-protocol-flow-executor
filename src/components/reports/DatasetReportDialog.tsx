/**
 * 数据集报告 Dialog
 *
 * 以弹窗形式展示数据驱动执行报告详情，替代全页面导航。
 * 支持最大化/还原，支持查看失败行详情。
 */
import { useState } from 'react';
import { getLL } from '@/i18n/i18n-lazy';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Maximize2, Minimize2 } from 'lucide-react';
import { useDatasetExecutionStore } from '@/stores/datasetExecution';
import { DatasetReportDetail } from './DatasetReportDetail';
import { DatasetRowDetail } from './DatasetRowDetail';

export function DatasetReportDialog() {
  const ll = getLL();
  const isOpen = useDatasetExecutionStore((s) => s.isReportDialogOpen);
  const selectedReportId = useDatasetExecutionStore((s) => s.selectedReportId);
  const closeDialog = useDatasetExecutionStore((s) => s.closeReportDialog);
  const [isMaximized, setIsMaximized] = useState(false);
  const [viewingRowIndex, setViewingRowIndex] = useState<number | null>(null);

  const handleClose = () => {
    setIsMaximized(false);
    setViewingRowIndex(null);
    closeDialog();
  };

  if (!isOpen || !selectedReportId) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent
        hideCloseButton
        className={`${isMaximized ? 'w-screen h-screen max-w-none' : 'max-w-5xl w-[90vw]'} max-h-[90vh] flex flex-col p-0 gap-0 ${isMaximized ? 'rounded-none' : ''}`}
      >
        <DialogTitle className="sr-only">
          {viewingRowIndex !== null
            ? ll.execution.datasetReport.row_detail_title({ n: viewingRowIndex + 1 })
            : ll.execution.datasetReport.title()}
        </DialogTitle>
        {/* 头部：标题 + 最大化/还原/关闭按钮 */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold">
              {viewingRowIndex !== null
                ? ll.execution.datasetReport.row_detail_title({ n: viewingRowIndex + 1 })
                : ll.execution.datasetReport.title()}
            </span>
          </div>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => setIsMaximized(!isMaximized)}
              title={isMaximized ? ll.execution.datasetReport.restore() : ll.execution.datasetReport.maximize()}
            >
              {isMaximized ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
            </Button>
          </div>
        </div>

        {/* 内容区：报告详情或行详情 */}
        <div className="flex-1 overflow-hidden">
          {viewingRowIndex !== null ? (
            <DatasetRowDetail
              reportId={selectedReportId}
              rowIndex={viewingRowIndex}
              onBack={() => setViewingRowIndex(null)}
            />
          ) : (
            <DatasetReportDetail
              reportId={selectedReportId}
              onViewRowDetail={(_reportId, rowIndex) => setViewingRowIndex(rowIndex)}
            />
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
