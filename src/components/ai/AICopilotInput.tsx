/**
 * AI Copilot 输入框与快捷指令、多模态附件组件
 * 状态就近隔离：输入打字时不触发父面板及历史消息树 Re-render
 */

import { useState, useRef, useEffect, memo } from 'react';
import {
  Send,
  Square,
  Folder,
  X,
  Image as ImageIcon,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { findMatchingSlashCommands } from '@/services/ai/slashCommands';
import {
  compressAndConvertToBase64,
  extractImagesFromClipboard,
  extractImagesFromDrop,
} from '@/lib/imageUtils';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIWorkspaceContext } from '@/types/ai';
import type { AIProviderConfig } from '@/types/ai';

export interface AICopilotInputProps {
  textareaRef?: React.RefObject<HTMLTextAreaElement | null>;
  activeProvider?: AIProviderConfig | null;
  activeWorkspace?: AIWorkspaceContext | null;
  isGenerating?: boolean;
  onSendMessage: (prompt: string, images?: string[]) => void;
  onStopGeneration: () => void;
  onSelectWorkspaceFolder: () => void;
  onUnlinkWorkspace: () => void;
  onSetWorkspaceFolder?: (folder: AIWorkspaceContext) => void;
}

export const AICopilotInput = memo(function AICopilotInput({
  textareaRef: externalTextareaRef,
  activeProvider,
  activeWorkspace,
  isGenerating = false,
  onSendMessage,
  onStopGeneration,
  onSelectWorkspaceFolder,
  onUnlinkWorkspace,
  onSetWorkspaceFolder,
}: AICopilotInputProps) {
  const ll = getLL();

  const [inputPrompt, setInputPrompt] = useState('');
  const [attachedImages, setAttachedImages] = useState<string[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const [selectedCommandIndex, setSelectedCommandIndex] = useState(0);

  const internalTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const textareaRef = externalTextareaRef || internalTextareaRef;
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const matchingCommands =
    inputPrompt.startsWith('/') && !inputPrompt.includes(' ')
      ? findMatchingSlashCommands(inputPrompt)
      : [];

  useEffect(() => {
    setSelectedCommandIndex(0);
  }, [inputPrompt]);

  const handleSend = () => {
    const text = inputPrompt.trim();
    if (!text && attachedImages.length === 0) return;
    const imgs = attachedImages.length > 0 ? [...attachedImages] : undefined;
    setInputPrompt('');
    setAttachedImages([]);
    onSendMessage(text, imgs);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (matchingCommands.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedCommandIndex((prev) => (prev + 1) % matchingCommands.length);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedCommandIndex(
          (prev) => (prev - 1 + matchingCommands.length) % matchingCommands.length,
        );
        return;
      }
      if ((e.key === 'Enter' || e.key === 'Tab') && !e.shiftKey) {
        e.preventDefault();
        const selectedCmd = matchingCommands[selectedCommandIndex];
        if (selectedCmd) {
          setInputPrompt('');
          selectedCmd.run('');
          return;
        }
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        setInputPrompt('');
        return;
      }
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handlePaste = async (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const images = await extractImagesFromClipboard(e.clipboardData);
    if (images.length > 0) {
      e.preventDefault();
      setAttachedImages((prev) => [...prev, ...images]);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  };

  const handleDragLeave = () => {
    setIsDragOver(false);
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);

    // 优先检查是否拖入本地工程文件夹或项目目录
    const droppedFile = e.dataTransfer.files?.[0] as (File & { path?: string }) | undefined;
    if (droppedFile?.path && !droppedFile.type.startsWith('image/')) {
      const filePath = droppedFile.path;
      const cleanPath = filePath.replace(/[/\\]+$/, '');
      const segments = cleanPath.split(/[/\\]/);
      const name = segments[segments.length - 1] || 'workspace';
      onSetWorkspaceFolder?.({
        path: filePath,
        name,
      });
      return;
    }

    const images = await extractImagesFromDrop(e.dataTransfer);
    if (images.length > 0) {
      setAttachedImages((prev) => [...prev, ...images]);
    }
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    const newImages: string[] = [];
    for (const file of files) {
      if (file.type.startsWith('image/')) {
        try {
          const base64 = await compressAndConvertToBase64(file);
          newImages.push(base64);
        } catch {
          // 忽略
        }
      }
    }
    if (newImages.length > 0) {
      setAttachedImages((prev) => [...prev, ...newImages]);
    }
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleRemoveImage = (index: number) => {
    setAttachedImages((prev) => prev.filter((_, i) => i !== index));
  };

  return (
    <footer
      className="p-3 border-t border-border bg-background shrink-0 space-y-2"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* 工作区胶囊组件 (Workspace Context Pill) */}
      <div className="flex items-center justify-between">
        {!activeWorkspace ? (
          <button
            type="button"
            onClick={onSelectWorkspaceFolder}
            className="inline-flex items-center gap-1.5 px-2 py-1 text-[11px] font-medium text-muted-foreground hover:text-foreground bg-muted/40 hover:bg-muted/70 rounded-md border border-border/60 transition-colors"
            title={ll.ai.copilot.link_workspace_tip()}
          >
            <Folder className="h-3.5 w-3.5 text-primary/70" />
            <span>{ll.ai.copilot.link_workspace()}</span>
          </button>
        ) : (
          <div className="w-full flex items-center justify-between text-[11px] bg-primary/5 border border-primary/20 rounded-md px-2 py-1 text-foreground">
            <div
              className="flex items-center gap-1.5 min-w-0 max-w-[240px] truncate cursor-help"
              title={`${activeWorkspace.path}${activeWorkspace.techStack ? ` (${activeWorkspace.techStack})` : ''}`}
            >
              <Folder className="h-3.5 w-3.5 text-primary shrink-0" />
              <span className="font-medium text-foreground truncate">{activeWorkspace.name}</span>
              <span className="text-[10px] text-muted-foreground truncate max-w-[120px]">{activeWorkspace.path}</span>
              {activeWorkspace.techStack && (
                <span className="shrink-0 px-1 py-0.2 text-[9px] bg-primary/10 text-primary rounded font-mono">
                  {activeWorkspace.techStack}
                </span>
              )}
            </div>
            <div className="flex items-center gap-1 shrink-0 ml-2">
              <button
                type="button"
                onClick={onSelectWorkspaceFolder}
                className="text-[10px] text-muted-foreground hover:text-foreground px-1 py-0.5 rounded hover:bg-muted transition-colors"
                title={ll.ai.copilot.switch_workspace_tip()}
              >
                {ll.ai.copilot.switch_workspace()}
              </button>
              <button
                type="button"
                onClick={onUnlinkWorkspace}
                className="text-muted-foreground hover:text-destructive p-0.5 rounded hover:bg-destructive/10 transition-colors"
                title={ll.ai.copilot.unlink_workspace_tip()}
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          </div>
        )}
      </div>

      {/* 图片附件待发预览条 */}
      {attachedImages.length > 0 && (
        <div className="flex flex-wrap gap-2 p-1.5 bg-muted/30 rounded-lg border border-border/50">
          {attachedImages.map((imgSrc, idx) => (
            <div key={idx} className="relative group">
              <img
                src={imgSrc}
                alt={`Preview ${idx + 1}`}
                className="h-12 w-12 object-cover rounded-md border border-border"
              />
              <button
                type="button"
                className="absolute -top-1 -right-1 h-4 w-4 rounded-full bg-destructive text-destructive-foreground flex items-center justify-center text-[10px] shadow-sm hover:scale-110 transition-transform"
                onClick={() => handleRemoveImage(idx)}
                title={ll.ai.copilot.remove_image()}
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}

      <div
        className={`relative rounded-lg border bg-background transition-all ${
          isDragOver
            ? 'border-primary ring-2 ring-primary/20 bg-primary/5'
            : 'border-border focus-within:border-primary focus-within:ring-1 focus-within:ring-primary/30'
        }`}
      >
        {/* 斜杠快捷指令自动补全浮层 */}
        {matchingCommands.length > 0 && (
          <div className="absolute bottom-full left-0 right-0 mb-1.5 rounded-lg border border-border bg-popover text-popover-foreground shadow-xl p-1 z-30 space-y-0.5 animate-in fade-in slide-in-from-bottom-1 duration-100">
            <div className="px-2 py-1 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center justify-between border-b border-border/40 select-none">
              <span>{ll.ai.copilot.slash_commands_title()}</span>
              <span className="text-[9px] font-normal font-mono">{ll.ai.copilot.slash_commands_hint()}</span>
            </div>
            <div className="max-h-48 overflow-y-auto space-y-0.5">
              {matchingCommands.map((cmd, idx) => {
                const isSelected = idx === selectedCommandIndex;
                return (
                  <button
                    key={cmd.id}
                    type="button"
                    className={`w-full flex items-center justify-between px-2.5 py-1.5 rounded-md text-left text-xs transition-colors cursor-pointer ${
                      isSelected
                        ? 'bg-primary text-primary-foreground font-medium'
                        : 'hover:bg-muted text-foreground'
                    }`}
                    onMouseEnter={() => setSelectedCommandIndex(idx)}
                    onClick={() => {
                      setInputPrompt('');
                      cmd.run('');
                    }}
                  >
                    <div className="flex items-center gap-2 min-w-0 flex-1 mr-2">
                      <span className="font-mono font-semibold">{cmd.name}</span>
                      <span
                        className={`truncate text-xs ${
                          isSelected
                            ? 'text-primary-foreground/90'
                            : 'text-muted-foreground'
                        }`}
                      >
                        {cmd.title()}
                      </span>
                    </div>
                    <span
                      className={`text-[10px] truncate max-w-[150px] shrink-0 ${
                        isSelected
                          ? 'text-primary-foreground/75'
                          : 'text-muted-foreground/60'
                      }`}
                    >
                      {cmd.description()}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        )}

        <textarea
          ref={textareaRef}
          value={inputPrompt}
          onChange={(e) => setInputPrompt(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder={
            activeProvider
              ? isDragOver
                ? ll.ai.copilot.drop_image_here()
                : isGenerating
                  ? ll.ai.copilot.input_midway_placeholder()
                  : ll.ai.copilot.input_placeholder()
              : ll.ai.copilot.input_disabled_placeholder()
          }
          rows={2}
          className="w-full resize-none border-0 bg-transparent p-2.5 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0 shadow-none min-h-[50px] max-h-32 disabled:cursor-not-allowed disabled:opacity-50"
        />

        {/* 隐藏的图片文件选择器 */}
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          multiple
          className="hidden"
          onChange={handleFileSelect}
        />

        <div className="flex items-center justify-between px-2.5 pb-2">
          <div className="flex items-center gap-1">
            {/* 附加图片按钮 */}
            <Button
              size="icon"
              variant="ghost"
              className="h-6 w-6 text-muted-foreground hover:text-foreground"
              onClick={() => fileInputRef.current?.click()}
              disabled={!activeProvider}
              title={ll.ai.copilot.attach_image()}
            >
              <ImageIcon className="h-3.5 w-3.5" />
            </Button>
            <span className="text-[10px] text-muted-foreground">
              {attachedImages.length > 0
                ? ll.ai.copilot.image_attached({ count: attachedImages.length })
                : ll.ai.copilot.image_support()}
            </span>
          </div>

          <div className="flex items-center gap-1.5">
            {isGenerating ? (
              <>
                <Button
                  size="sm"
                  variant="destructive"
                  className="h-6 px-2 text-[11px] rounded-md gap-1"
                  onClick={onStopGeneration}
                  title={ll.ai.copilot.stop()}
                >
                  <Square className="h-3 w-3 fill-current" />
                  {ll.ai.copilot.stop()}
                </Button>
                {Boolean(inputPrompt.trim() || attachedImages.length > 0) && (
                  <Button
                    size="sm"
                    className="h-6 px-2 text-[11px] rounded-md bg-amber-600 hover:bg-amber-700 text-white gap-1 animate-in fade-in duration-100"
                    onClick={handleSend}
                    title={ll.ai.copilot.send_correction_tip()}
                  >
                    <Send className="h-3 w-3" />
                    <span>{ll.ai.copilot.send_correction()}</span>
                  </Button>
                )}
              </>
            ) : (
              <Button
                size="sm"
                className="h-6 w-6 p-0 rounded-md bg-primary hover:bg-primary/90 text-primary-foreground"
                onClick={handleSend}
                disabled={(!inputPrompt.trim() && attachedImages.length === 0) || !activeProvider}
                title={ll.ai.copilot.send()}
              >
                <Send className="h-3 w-3" />
              </Button>
            )}
          </div>
        </div>
      </div>
    </footer>
  );
});
