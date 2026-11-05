/**
 * MediaPreview - 报告内的媒体预览网格
 *
 * 纯展示组件：不提供 section 外壳（由父 viewer 的 CollapsibleSection 提供），
 * 只负责渲染候选、加载失败降级、保存与复制链接。
 */

import { useState } from 'react';
import { Copy, Download } from 'lucide-react';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { buildDefaultFilename, formatBytes, type MediaCandidate } from './mediaDetection';
import { saveMediaCandidate } from './mediaSave';

/** 默认展示的候选数量，其余折叠 */
const INITIAL_VISIBLE = 4;
/** 后端内联上限，仅用于超限文案 */
const PREVIEW_LIMIT = '4 MB';

/** 操作按钮样式（与 CopyButton `size="xs"` 对齐） */
const ACTION_BUTTON_CLASS =
  'p-1 text-xs gap-1 h-6 flex items-center rounded hover:bg-muted disabled:opacity-50 disabled:cursor-not-allowed';

/** 媒体预览网格 */
export interface MediaPreviewProps {
  /** 待展示的候选 */
  candidates: MediaCandidate[];
  /** 节点名，用于默认文件名 */
  nodeName: string;
  /** 响应头，用于推断文件名与扩展名 */
  headers?: Record<string, unknown>;
}

/** 单个媒体卡片 */
function MediaCard({
  candidate,
  nodeName,
  headers,
}: {
  candidate: MediaCandidate;
  nodeName: string;
  headers?: Record<string, unknown>;
}) {
  const ll = getLL();
  const [failed, setFailed] = useState(false);
  const value = candidate.source?.value ?? '';
  const isVideo = candidate.kind === 'video';
  // data URL 内联载荷可达数 MB，复制到剪贴板既慢又无实际用途，仅 URL 来源提供复制
  const canCopy = candidate.source?.type === 'url';

  const handleSave = () => {
    void saveMediaCandidate(candidate, buildDefaultFilename(candidate, headers, nodeName));
  };

  const handleCopy = () => {
    if (!canCopy || !value) return;
    navigator.clipboard
      .writeText(value)
      .then(() => toast.success(ll.execution.report.copied_to_clipboard()))
      .catch(() => toast.error(ll.execution.report.copy_failed()));
  };

  if (candidate.omitted) {
    return (
      <div className="border border-border rounded-lg overflow-hidden bg-muted/30 p-3">
        <p className="text-xs text-muted-foreground">
          {ll.execution.viewers.media_omitted({
            size: formatBytes(candidate.bytes ?? 0),
            limit: PREVIEW_LIMIT,
          })}
        </p>
      </div>
    );
  }

  return (
    <div className="border border-border rounded-lg overflow-hidden bg-muted/30">
      <div className="flex items-center justify-center min-h-[80px]">
        {failed || !candidate.source ? (
          <div className="p-3 space-y-1">
            <a
              href={value}
              target="_blank"
              rel="noreferrer"
              className="break-all text-xs text-accent underline"
            >
              {value}
            </a>
            <p className="text-xs text-muted-foreground">
              {ll.execution.viewers.media_expired()}
            </p>
          </div>
        ) : isVideo ? (
          <video
            src={value}
            controls
            preload="metadata"
            className="max-h-72 w-full bg-black"
            onError={() => setFailed(true)}
          />
        ) : (
          <img
            src={value}
            alt={candidate.mime || candidate.kind}
            className="max-h-72 w-auto object-contain bg-black/5"
            onError={() => setFailed(true)}
          />
        )}
      </div>
      <div className="flex items-center gap-1 px-2 py-1 border-t border-border">
        <button
          type="button"
          onClick={handleSave}
          disabled={failed || !candidate.source}
          className={ACTION_BUTTON_CLASS}
          title={ll.execution.viewers.media_save()}
          aria-label={ll.execution.viewers.media_save()}
        >
          <Download className="w-3.5 h-3.5" />
          <span>{ll.execution.viewers.media_save()}</span>
        </button>
        {canCopy && (
          <button
            type="button"
            onClick={handleCopy}
            disabled={failed}
            className={ACTION_BUTTON_CLASS}
            title={ll.execution.viewers.media_copy_link()}
            aria-label={ll.execution.viewers.media_copy_link()}
          >
            <Copy className="w-3.5 h-3.5" />
            <span>{ll.execution.viewers.media_copy_link()}</span>
          </button>
        )}
      </div>
    </div>
  );
}

/** 媒体预览网格 */
export function MediaPreview({ candidates, nodeName, headers }: MediaPreviewProps) {
  const ll = getLL();
  const [expanded, setExpanded] = useState(false);
  const visible = expanded ? candidates : candidates.slice(0, INITIAL_VISIBLE);

  return (
    <div className="space-y-2">
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {visible.map((candidate) => (
          <MediaCard
            key={candidate.id}
            candidate={candidate}
            nodeName={nodeName}
            headers={headers}
          />
        ))}
      </div>
      {candidates.length > INITIAL_VISIBLE && (
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="text-xs text-accent hover:underline"
        >
          {expanded
            ? ll.execution.viewers.media_show_less()
            : ll.execution.viewers.media_show_all({ count: candidates.length })}
        </button>
      )}
    </div>
  );
}
