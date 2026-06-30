/**
 * EmailViewer - IMAP/SMTP 邮件节点执行详情查看器
 * 结构化展示 IMAP 各类节点的执行结果:
 * Connect: 连接状态、服务器信息、能力列表
 * Search: 搜索条件、匹配 UID 列表
 * Fetch: 邮件列表（UID/主题/发件人/日期/标志）+ 邮件详情
 * List: 文件夹列表
 * 其他: 通用操作结果
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Search,
  Download,
  List,
  Mail,
  Inbox,
  Trash2,
  Copy,
  ArrowRightLeft,
  Flag,
  Eye,
  Unplug,
  FolderPlus,
  FolderMinus,
  Info,
  Paperclip,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import type {
  ImapConnectResult,
  ImapSearchResult,
  ImapFetchResult,
  ImapListResult,
  ImapMessage,
  ImapFolder,
} from '@/types/imap';
import { Badge } from '@/components/ui/badge';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { cn } from '@/lib/utils';
import { VariableChangesCard } from './VariableChangesCard';
import { getLL } from '@/i18n/i18n-lazy';

interface EmailViewerProps {
  report: NodeExecutionReport;
}

// ============================================================================
// 公共组件
// ============================================================================

/** 折叠面板 */
interface CollapsibleSectionProps {
  title: string;
  icon: React.ReactNode;
  defaultOpen?: boolean;
  extra?: React.ReactNode;
  children: React.ReactNode;
}

function CollapsibleSection({ title, icon, defaultOpen = false, extra, children }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <div className="flex items-center w-full bg-muted/30 hover:bg-muted/50 transition-colors">
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex-1 flex items-center gap-2 p-3 text-left"
          aria-expanded={isOpen}
        >
          <div className="text-muted-foreground">
            {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </div>
          <div className="text-muted-foreground">{icon}</div>
          <span className="font-medium text-foreground text-sm">{title}</span>
        </button>
        {extra && <div className="pr-3">{extra}</div>}
      </div>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/** 错误信息 */
function ErrorBlock({ error }: { error?: string }) {
  if (!error) return null;
  return (
    <div className="bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
      <p className="text-sm text-red-700 dark:text-red-300 break-all">{error}</p>
    </div>
  );
}

/** 信息行 */
function InfoRow({ label, value, mono }: { label: string; value?: string | number | null; mono?: boolean }) {
  if (value === undefined || value === null || value === '') return null;
  return (
    <div className="flex items-start gap-2 text-sm">
      <span className="text-muted-foreground shrink-0 min-w-16">{label}</span>
      {mono ? (
        <code className="text-xs bg-muted px-2 py-0.5 rounded font-mono break-all">{String(value)}</code>
      ) : (
        <span className="text-foreground break-all">{String(value)}</span>
      )}
    </div>
  );
}

// ============================================================================
// 邮件卡片
// ============================================================================

function ImapMessageCard({ message, index, onSelect, isSelected }: {
  message: ImapMessage;
  index: number;
  onSelect: () => void;
  isSelected: boolean;
}) {
  return (
    <div
      className={cn(
        'border rounded-lg p-2.5 cursor-pointer transition-colors',
        isSelected ? 'border-blue-400 bg-blue-50/50 dark:bg-blue-950/20' : 'border-border hover:bg-muted/30'
      )}
      onClick={onSelect}
    >
      <div className="flex items-center gap-2 mb-1">
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          #{index + 1}
        </Badge>
        <span className="text-xs text-muted-foreground font-mono">UID {message.uid}</span>
        {message.flags && message.flags.length > 0 && (
          <div className="flex gap-1">
            {message.flags.map((flag, i) => (
              <span key={i} className="text-[10px] text-blue-600 dark:text-blue-400 font-mono">
                {flag}
              </span>
            ))}
          </div>
        )}
        {message.date && (
          <span className="text-[10px] text-muted-foreground ml-auto">{message.date}</span>
        )}
      </div>
      <div className="text-sm font-medium text-foreground truncate">
        {message.subject || '(No Subject)'}
      </div>
      <div className="text-xs text-muted-foreground truncate">
        {message.from || '(Unknown Sender)'}
        {message.attachments && message.attachments.length > 0 && (
          <span className="ml-2 inline-flex items-center gap-0.5">
            <Paperclip className="w-3 h-3" />
            {message.attachments.length}
          </span>
        )}
      </div>
    </div>
  );
}

/** 邮件详情视图 */
function ImapMessageDetail({ message }: { message: ImapMessage }) {
  const ll = getLL();
  const [showHtml, setShowHtml] = useState(false);

  return (
    <div className="space-y-3">
      {/* 邮件头 */}
      <div className="space-y-1.5">
        <InfoRow label={ll.viewer.email.label_subject()} value={message.subject} />
        <InfoRow label={ll.viewer.email.label_from()} value={message.from} mono />
        {message.to && message.to.length > 0 && (
          <InfoRow label={ll.viewer.email.label_to()} value={message.to.join(', ')} mono />
        )}
        <InfoRow label="Date" value={message.date} />
        <InfoRow label="UID" value={message.uid} mono />
        {message.flags && message.flags.length > 0 && (
          <div className="flex items-start gap-2 text-sm">
            <span className="text-muted-foreground shrink-0 min-w-16">Flags</span>
            <div className="flex flex-wrap gap-1">
              {message.flags.map((flag, i) => (
                <Badge key={i} variant="outline" className="text-[10px] px-1 py-0 font-mono">
                  {flag}
                </Badge>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* 正文 */}
      {message.body_text && (
        <div className="space-y-1">
          <p className="text-xs font-medium text-foreground">{ll.viewer.email.label_plain_text()}</p>
          <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto bg-muted rounded p-2">
            {message.body_text}
          </pre>
        </div>
      )}

      {/* HTML 正文 */}
      {message.body_html && (
        <div className="space-y-1">
          <div className="flex items-center justify-between">
            <p className="text-xs font-medium text-foreground">{ll.viewer.email.label_html_content()}</p>
            <button
              type="button"
              onClick={() => setShowHtml(!showHtml)}
              className="text-[10px] text-blue-600 hover:underline"
            >
              {showHtml ? ll.viewer.email.label_hide_render() : ll.viewer.email.label_render_html()}
            </button>
          </div>
          {showHtml ? (
            <iframe
              srcDoc={message.body_html}
              className="w-full border rounded bg-white"
              style={{ height: '300px' }}
              sandbox="allow-same-origin"
              title={ll.viewer.email.label_html_content()}
            />
          ) : (
            <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-40 overflow-auto bg-muted rounded p-2">
              {message.body_html.length > 2000
                ? message.body_html.slice(0, 2000) + '...'
                : message.body_html}
            </pre>
          )}
        </div>
      )}

      {/* 附件列表 */}
      {message.attachments && message.attachments.length > 0 && (
        <div className="space-y-1">
          <p className="text-xs font-medium text-foreground flex items-center gap-1">
            <Paperclip className="w-3 h-3" />
            Attachments ({message.attachments.length})
          </p>
          <div className="space-y-1">
            {message.attachments.map((att, i) => (
              <div key={i} className="flex items-center gap-2 text-xs bg-muted rounded px-2 py-1">
                <Paperclip className="w-3 h-3 text-muted-foreground shrink-0" />
                <span className="text-foreground truncate">{att.filename || '(unnamed)'}</span>
                <span className="text-muted-foreground font-mono shrink-0">{att.content_type}</span>
                {att.size > 0 && (
                  <span className="text-muted-foreground font-mono shrink-0 ml-auto">
                    {att.size > 1024 ? `${(att.size / 1024).toFixed(1)}KB` : `${att.size}B`}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// 文件夹列表
// ============================================================================

function ImapFolderList({ folders }: { folders: ImapFolder[] }) {
  return (
    <div className="space-y-1">
      {folders.map((folder, i) => (
        <div key={i} className="flex items-center gap-2 text-sm bg-muted rounded px-2 py-1.5">
          <Inbox className="w-3.5 h-3.5 text-blue-500 shrink-0" />
          <code className="text-xs font-mono text-foreground break-all">{folder.name}</code>
          {folder.delimiter && (
            <span className="text-[10px] text-muted-foreground">Delimiter: {folder.delimiter}</span>
          )}
          {folder.attributes.length > 0 && (
            <div className="flex gap-1 ml-auto">
              {folder.attributes.map((attr, j) => (
                <Badge key={j} variant="outline" className="text-[10px] px-1 py-0 font-mono">
                  {attr}
                </Badge>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// 节点类型图标映射
// ============================================================================

function getNodeIcon(nodeType: string) {
  const iconMap: Record<string, React.ReactNode> = {
    imap_connect: <Cable className="w-4 h-4" />,
    imap_select: <Inbox className="w-4 h-4" />,
    imap_search: <Search className="w-4 h-4" />,
    imap_fetch: <Download className="w-4 h-4" />,
    imap_store_flags: <Flag className="w-4 h-4" />,
    imap_copy: <Copy className="w-4 h-4" />,
    imap_move: <ArrowRightLeft className="w-4 h-4" />,
    imap_expunge: <Trash2 className="w-4 h-4" />,
    imap_list: <List className="w-4 h-4" />,
    imap_folder_create: <FolderPlus className="w-4 h-4" />,
    imap_folder_delete: <FolderMinus className="w-4 h-4" />,
    imap_idle: <Eye className="w-4 h-4" />,
    imap_close: <Unplug className="w-4 h-4" />,
  };
  return iconMap[nodeType] || <Mail className="w-4 h-4" />;
}

function getNodeTitle(nodeType: string): string {
  const titleMap: Record<string, string> = {
    imap_connect: 'IMAP Connect',
    imap_select: 'IMAP Select Mailbox',
    imap_search: 'IMAP Search',
    imap_fetch: 'IMAP Fetch Messages',
    imap_store_flags: 'IMAP Modify Flags',
    imap_copy: 'IMAP Copy Messages',
    imap_move: 'IMAP Move Messages',
    imap_expunge: 'IMAP Expunge',
    imap_list: 'IMAP List Mailboxes',
    imap_folder_create: 'IMAP Create Folder',
    imap_folder_delete: 'IMAP Delete Folder',
    imap_idle: 'IMAP IDLE',
    imap_close: 'IMAP Close Connection',
  };
  return titleMap[nodeType] || 'IMAP Operation';
}

// ============================================================================
// 主组件
// ============================================================================

export const EmailViewer = memo(function EmailViewer({ report }: EmailViewerProps) {
  const ll = getLL();
  const nodeType = report.node_type;
  const output = report.output_data as Record<string, unknown> | undefined;
  const isSuccess = (output?.success as boolean) ?? report.status === 'success';
  const error = output?.error as string | undefined;

  // 通用状态面板
  const renderStatus = (successText: string, failText: string) => (
    <CollapsibleSection
      title={ll.viewer.common.section_execution_status()}
      icon={getNodeIcon(nodeType)}
      defaultOpen={true}
    >
      <div className="space-y-2">
        <div className="flex items-center gap-3">
          <StatusBadge isSuccess={isSuccess} successText={successText} failText={failText} />
          <span className="text-sm text-muted-foreground">
            {isSuccess ? getNodeTitle(nodeType) + ' Succeeded' : (error || 'Execution Failed')}
          </span>
        </div>
        <ErrorBlock error={!isSuccess ? error : undefined} />
      </div>
    </CollapsibleSection>
  );

  // 按节点类型渲染特定内容
  switch (nodeType) {
    case 'imap_connect': {
      const result = output as ImapConnectResult | undefined;
      return (
        <div className="space-y-3">
          {renderStatus('Connected', 'Connection Failed')}
          {isSuccess && result && (
            <>
              {result.server_info && (
                <CollapsibleSection title={ll.viewer.email.section_server_info()} icon={<Info className="w-4 h-4" />} defaultOpen={true}>
                  <InfoRow label="Server" value={result.server_info} />
                </CollapsibleSection>
              )}
              {result.capabilities && result.capabilities.length > 0 && (
                <CollapsibleSection title={ll.viewer.email.section_capabilities()} icon={<Cable className="w-4 h-4" />} defaultOpen={false}
                  extra={<Badge variant="outline" className="font-mono text-xs">{result.capabilities.length} 项</Badge>}
                >
                  <div className="flex flex-wrap gap-1">
                    {result.capabilities.map((cap, i) => (
                      <Badge key={i} variant="outline" className="text-[10px] px-1 py-0 font-mono">{cap}</Badge>
                    ))}
                  </div>
                </CollapsibleSection>
              )}
            </>
          )}
          <VariableChangesCard changes={report.variable_changes} />
        </div>
      );
    }

    case 'imap_search': {
      const result = output as ImapSearchResult | undefined;
      const uids = result?.uids ?? [];
      return (
        <div className="space-y-3">
          {renderStatus('Search Complete', 'Search Failed')}
          {isSuccess && result && (
            <CollapsibleSection title={ll.viewer.email.section_search()} icon={<Search className="w-4 h-4" />} defaultOpen={true}
              extra={<Badge variant="outline" className="font-mono text-xs">{result.count ?? uids.length} matches</Badge>}
            >
              {uids.length > 0 ? (
                <div className="flex flex-wrap gap-1">
                  {uids.map((uid, i) => (
                    <Badge key={i} variant="outline" className="text-[10px] px-1.5 py-0.5 font-mono">
                      UID {uid}
                    </Badge>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground italic">No matching emails found</p>
              )}
            </CollapsibleSection>
          )}
          <VariableChangesCard changes={report.variable_changes} />
        </div>
      );
    }

    case 'imap_fetch': {
      const result = output as ImapFetchResult | undefined;
      const messages = result?.messages ?? [];
      const [selectedIdx, setSelectedIdx] = useState<number>(0);

      return (
        <div className="space-y-3">
          {renderStatus('Fetch Complete', 'Fetch Failed')}
          {isSuccess && messages.length > 0 && (
            <>
              {/* 邮件列表 */}
              <CollapsibleSection title={ll.viewer.email.section_message_list()} icon={<Mail className="w-4 h-4" />} defaultOpen={true}
                extra={<Badge variant="outline" className="font-mono text-xs">{result?.count ?? messages.length}</Badge>}
              >
                <div className="space-y-1.5 max-h-60 overflow-auto">
                  {messages.map((msg, idx) => (
                    <ImapMessageCard
                      key={msg.uid}
                      message={msg}
                      index={idx}
                      isSelected={selectedIdx === idx}
                      onSelect={() => setSelectedIdx(idx)}
                    />
                  ))}
                </div>
              </CollapsibleSection>
              {/* 邮件详情 */}
              {messages[selectedIdx] && (
                <CollapsibleSection title={ll.viewer.email.section_message_detail()} icon={<Info className="w-4 h-4" />} defaultOpen={true}>
                  <ImapMessageDetail message={messages[selectedIdx]} />
                </CollapsibleSection>
              )}
            </>
          )}
          <VariableChangesCard changes={report.variable_changes} />
        </div>
      );
    }

    case 'imap_list': {
      const result = output as ImapListResult | undefined;
      const folders = result?.folders ?? [];
      return (
        <div className="space-y-3">
          {renderStatus('List Complete', 'List Failed')}
          {isSuccess && folders.length > 0 && (
            <CollapsibleSection title={ll.viewer.email.section_mailbox_folders()} icon={<Inbox className="w-4 h-4" />} defaultOpen={true}
              extra={<Badge variant="outline" className="font-mono text-xs">{folders.length}</Badge>}
            >
              <ImapFolderList folders={folders} />
            </CollapsibleSection>
          )}
          <VariableChangesCard changes={report.variable_changes} />
        </div>
      );
    }

    default: {
      // 通用操作结果（select, store_flags, copy, move, expunge, folder_create, folder_delete, idle, close）
      return (
        <div className="space-y-3">
          {renderStatus('Success', 'Failed')}
          <VariableChangesCard changes={report.variable_changes} />
        </div>
      );
    }
  }
});
