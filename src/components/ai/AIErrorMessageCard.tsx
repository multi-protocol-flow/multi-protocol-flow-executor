import React, { useState, useMemo, memo } from 'react';
import {
  AlertCircle,
  RotateCcw,
  Copy,
  Check,
  ChevronDown,
  ChevronRight,
  WifiOff,
  Clock,
  KeyRound,
  ShieldAlert,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { getLL } from '@/i18n/i18n-lazy';
import { diagnoseAIError, type AIErrorKind } from '@/services/ai/aiErrorDiagnose';
import { useAIStore } from '@/stores/ai';
import type { AIMessage } from '@/types/ai';

interface AIErrorMessageCardProps {
  message: AIMessage;
  compact?: boolean;
  onRetry?: () => void;
}

function getErrorIcon(kind: AIErrorKind) {
  switch (kind) {
    case 'stream_interrupted':
    case 'connect_failed':
      return <WifiOff className="h-3.5 w-3.5 text-destructive shrink-0" />;
    case 'timeout':
      return <Clock className="h-3.5 w-3.5 text-destructive shrink-0" />;
    case 'unauthorized':
      return <KeyRound className="h-3.5 w-3.5 text-destructive shrink-0" />;
    case 'rate_limit':
      return <ShieldAlert className="h-3.5 w-3.5 text-destructive shrink-0" />;
    default:
      return <AlertCircle className="h-3.5 w-3.5 text-destructive shrink-0" />;
  }
}

export const AIErrorMessageCard: React.FC<AIErrorMessageCardProps> = memo(function AIErrorMessageCard({
  message,
  compact = false,
  onRetry,
}) {
  const ll = getLL();
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const [isRetrying, setIsRetrying] = useState(false);

  const diagnosis = useMemo(
    () => diagnoseAIError(message.error || message.content),
    [message.error, message.content],
  );

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(diagnosis.rawError);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // 剪贴板不可用时静默失败
    }
  };

  const handleRetry = async () => {
    if (isRetrying) return;
    setIsRetrying(true);
    try {
      if (onRetry) {
        onRetry();
      } else {
        await useAIStore.getState().retryLastMessage();
      }
    } finally {
      setIsRetrying(false);
    }
  };

  return (
    <div
      className={`rounded-2xl rounded-tl-sm border border-destructive/30 bg-destructive/5 dark:bg-destructive/10 text-foreground text-xs leading-relaxed space-y-2 ${
        compact ? 'p-2.5' : 'p-3'
      }`}
    >
      {/* 头部标题与重试动作 */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          {getErrorIcon(diagnosis.kind)}
          <span className="font-semibold text-destructive truncate">
            {ll.ai.copilot.error_title()}
          </span>
        </div>

        {diagnosis.retryable && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleRetry}
            disabled={isRetrying}
            className="h-6 px-2 text-[11px] gap-1 border-destructive/30 text-destructive hover:bg-destructive/15 hover:text-destructive shrink-0"
          >
            <RotateCcw className={`h-3 w-3 ${isRetrying ? 'animate-spin' : ''}`} />
            <span>{isRetrying ? ll.ai.copilot.error_retrying() : ll.ai.copilot.error_retry()}</span>
          </Button>
        )}
      </div>

      {/* 智能诊断解释 */}
      <p className="text-muted-foreground leading-normal break-words">
        {diagnosis.diagnosticText}
      </p>

      {/* 可折叠底层技术报错 */}
      <div className="pt-0.5 border-t border-destructive/15">
        <div className="flex items-center justify-between pt-1">
          <button
            type="button"
            onClick={() => setDetailsOpen(!detailsOpen)}
            className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
          >
            {detailsOpen ? (
              <ChevronDown className="h-3 w-3 shrink-0" />
            ) : (
              <ChevronRight className="h-3 w-3 shrink-0" />
            )}
            <span>{ll.ai.copilot.error_details()}</span>
          </button>

          {detailsOpen && (
            <button
              type="button"
              onClick={handleCopy}
              className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors px-1 py-0.5 rounded hover:bg-muted/40"
              title={ll.ai.copilot.error_copy()}
            >
              {copied ? (
                <>
                  <Check className="h-2.5 w-2.5 text-emerald-500" />
                  <span className="text-emerald-500">{ll.ai.copilot.error_copied()}</span>
                </>
              ) : (
                <>
                  <Copy className="h-2.5 w-2.5" />
                  <span>{ll.ai.copilot.error_copy()}</span>
                </>
              )}
            </button>
          )}
        </div>

        {detailsOpen && (
          <pre className="mt-1.5 p-2 rounded bg-background/80 border border-destructive/20 font-mono text-[10.5px] text-destructive/90 whitespace-pre-wrap break-all max-h-36 overflow-y-auto select-text">
            {diagnosis.rawError}
          </pre>
        )}
      </div>
    </div>
  );
});
