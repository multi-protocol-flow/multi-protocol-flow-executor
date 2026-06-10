/**
 * 节点帮助抽屉组件
 * 从右侧滑入，显示节点类型的帮助内容（概述、参数说明、使用场景、常见问题）
 */
import { useEffect, useState } from 'react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { X, BookOpen, ListChecks, Lightbulb, HelpCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { getNodeHelp, type NodeHelpContent } from '@/lib/nodeHelpRegistry';
import { getLL } from '@/i18n/i18n-lazy';

interface HelpDrawerProps {
  /** Node type ID to show help for */
  nodeTypeId: string;
  /** Whether the drawer is open */
  open: boolean;
  /** Close handler */
  onClose: () => void;
  /** 当前节点面板的边界（用于定位，保持与面板同宽高） */
  panelBounds?: { top: number; left: number; width: number; height: number } | null;
}

export function HelpDrawer({ nodeTypeId, open, onClose, panelBounds }: HelpDrawerProps) {
  const [helpContent, setHelpContent] = useState<NodeHelpContent | null>(null);
  const ll = getLL();

  useEffect(() => {
    if (open && nodeTypeId) {
      setHelpContent(getNodeHelp(nodeTypeId) ?? null);
    }
  }, [open, nodeTypeId]);

  if (!open) return null;

  // 用 panelBounds 定位，保持与节点面板同宽高
  const drawerStyle: React.CSSProperties | undefined = panelBounds
    ? {
        position: 'fixed',
        top: panelBounds.top,
        left: panelBounds.left,
        width: panelBounds.width,
        height: panelBounds.height,
        zIndex: 50,
      }
    : undefined;

  // 当有 panelBounds 时，使用固定定位严格与节点面板重叠；无则回退全屏遮罩
  const hasBounds = !!panelBounds;

  return (
    <>
      {/* Backdrop - 只在有 panelBounds 时用面板尺寸的遮罩，否则全屏 */}
      {hasBounds ? (
        /* eslint-disable-next-line jsx-a11y/click-events-have-key-events, jsx-a11y/no-static-element-interactions */
        <div
          style={{
            position: 'fixed',
            top: panelBounds!.top,
            left: panelBounds!.left,
            width: panelBounds!.width,
            height: panelBounds!.height,
            zIndex: 49,
          }}
          className="bg-black/20"
          onClick={onClose}
        />
      ) : (
        /* eslint-disable-next-line jsx-a11y/click-events-have-key-events, jsx-a11y/no-static-element-interactions */
        <div className="fixed inset-0 z-40 bg-black/20" onClick={onClose} />
      )}

      {/* Drawer panel */}
      <div
        style={drawerStyle}
        className={cn(
          'flex flex-col bg-background shadow-xl overflow-hidden',
          hasBounds ? 'border border-border rounded-lg' : 'fixed inset-0 z-50',
          'animate-in slide-in-from-right duration-300',
        )}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
          <div className="flex items-center gap-2">
            <BookOpen className="w-4 h-4 text-muted-foreground" />
            <h3 className="text-sm font-semibold">
              {ll.welcome.help_title?.()}
            </h3>
            <Badge variant="outline" className="text-xs font-mono">
              {nodeTypeId}
            </Badge>
          </div>
          <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onClose}>
            <X className="w-4 h-4" />
          </Button>
        </div>

        <ScrollArea className="flex-1">
          {helpContent ? (
            <div className="p-4 space-y-6">
              {/* Summary */}
              <section>
                <div className="flex items-center gap-2 mb-2 text-sm font-medium">
                  <BookOpen className="w-4 h-4 text-primary" />
                  {ll.welcome.help_summary?.()}
                </div>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {helpContent.summary}
                </p>
              </section>

              {/* Parameters */}
              {helpContent.params.length > 0 && (
                <section>
                  <div className="flex items-center gap-2 mb-2 text-sm font-medium">
                    <ListChecks className="w-4 h-4 text-primary" />
                    {ll.welcome.help_params?.()}
                  </div>
                  <div className="space-y-2">
                    {helpContent.params.map((param, i) => (
                      <div key={i} className="text-sm border border-border rounded-lg p-3">
                        <div className="flex items-center gap-2 mb-1">
                          <code className="text-xs font-semibold bg-muted px-1.5 py-0.5 rounded">
                            {param.name}
                          </code>
                          <span className="text-xs text-muted-foreground">{param.type}</span>
                          {param.required && (
                            <span className="text-xs text-destructive">{getLL().panels.helpDrawer.required()}</span>
                          )}
                        </div>
                        <p className="text-xs text-muted-foreground mb-1">
                          {param.description}
                        </p>
                        <div className="text-xs">
                          <span className="text-muted-foreground">{getLL().panels.helpDrawer.example()}: </span>
                          <code className="bg-muted/50 px-1 rounded text-foreground/80">
                            {param.example}
                          </code>
                        </div>
                      </div>
                    ))}
                  </div>
                </section>
              )}

              {/* Use Cases */}
              {helpContent.useCases.length > 0 && (
                <section>
                  <div className="flex items-center gap-2 mb-2 text-sm font-medium">
                    <Lightbulb className="w-4 h-4 text-amber-500" />
                    {ll.welcome.help_use_cases?.()}
                  </div>
                  <div className="space-y-2">
                    {helpContent.useCases.map((uc, i) => (
                      <div key={i} className="text-sm border border-border rounded-lg p-3">
                        <div className="font-medium text-xs mb-0.5">{uc.title}</div>
                        <p className="text-xs text-muted-foreground">{uc.description}</p>
                      </div>
                    ))}
                  </div>
                </section>
              )}

              {/* FAQ */}
              {helpContent.faq.length > 0 && (
                <section>
                  <div className="flex items-center gap-2 mb-2 text-sm font-medium">
                    <HelpCircle className="w-4 h-4 text-sky-500" />
                    {ll.welcome.help_faq?.()}
                  </div>
                  <div className="space-y-2">
                    {helpContent.faq.map((faq, i) => (
                      <div key={i} className="text-sm border border-border rounded-lg p-3">
                        <div className="font-medium text-xs mb-0.5">Q: {faq.question}</div>
                        <p className="text-xs text-muted-foreground">A: {faq.answer}</p>
                      </div>
                    ))}
                  </div>
                </section>
              )}
            </div>
          ) : (
            <div className="flex items-center justify-center h-full">
              <p className="text-sm text-muted-foreground">
                {ll.welcome.help_no_content?.()}
              </p>
            </div>
          )}
        </ScrollArea>
      </div>
    </>
  );
}
