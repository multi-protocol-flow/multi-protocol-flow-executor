/**
 * McpViewer - MCP 节点执行详情查看器
 * 结构化展示 MCP Connect/Call Tool/Read Resource/Get Prompt/Close 五种节点的执行结果
 *
 * Connect: 连接配置、服务信息、发现列表（工具/资源/提示词）、耗时
 * Call Tool: 工具配置、内容块列表（text/image/resource）、耗时
 * Read Resource: 资源 URI、MIME 类型、内容、耗时
 * Get Prompt: 提示词名称、消息列表（role + content）、耗时
 * Close: 关闭配置、关闭状态、耗时
 */
import { useState, memo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cable,
  Wrench,
  FileText,
  MessageSquare,
  Unplug,
  Clock,
  Globe,
  Server,
  Image,
  Box,
} from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { RawJsonViewer } from './RawJsonViewer';
import { getLL } from '@/i18n/i18n-lazy';
import { VariableChangesCard } from './VariableChangesCard';

interface McpViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

/**
 * 折叠面板组件
 */
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
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full flex items-center gap-2 p-3 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
        aria-expanded={isOpen}
      >
        <div className="text-muted-foreground">
          {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        </div>
        <div className="text-muted-foreground">{icon}</div>
        <span className="font-medium text-foreground text-sm">{title}</span>
        {extra && <div className="ml-auto">{extra}</div>}
      </button>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

// ============================================================================
// MCP 输出数据类型
// ============================================================================

/**
 * MCP Connect 输出数据
 */
interface McpConnectOutput {
  success?: boolean;
  connection_id?: string | null;
  url?: string;
  server_info?: {
    name?: string;
    version?: string;
  };
  tools?: Array<{
    name: string;
    description?: string;
    input_schema?: Record<string, unknown>;
  }>;
  resources?: Array<{
    uri: string;
    name: string;
    description?: string;
    mime_type?: string;
  }>;
  prompts?: Array<{
    name: string;
    description?: string;
    arguments?: Array<{
      name: string;
      description?: string;
      required?: boolean;
    }>;
  }>;
  request?: {
    url?: string;
    protocol_version?: string;
    transport?: string;
  };
  timing?: {
    connect_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/**
 * MCP Call Tool 输出数据
 */
interface McpCallToolOutput {
  success?: boolean;
  tool_name?: string;
  result?: {
    content?: Array<Record<string, unknown>>;
    is_error?: boolean;
  };
  is_error?: boolean;
  request?: {
    connection_id?: string;
    tool_name?: string;
    arguments?: Record<string, unknown>;
  };
  timing?: {
    call_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/**
 * MCP Read Resource 输出数据
 */
interface McpReadResourceOutput {
  success?: boolean;
  resource_uri?: string;
  contents?: Array<Record<string, unknown>>;
  request?: {
    connection_id?: string;
    resource_uri?: string;
  };
  timing?: {
    read_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/**
 * MCP Get Prompt 输出数据
 */
interface McpGetPromptOutput {
  success?: boolean;
  prompt_name?: string;
  result?: {
    description?: string;
    messages?: Array<{
      role?: string;
      content?: unknown;
    }>;
  };
  request?: {
    connection_id?: string;
    prompt_name?: string;
    arguments?: Record<string, string>;
  };
  timing?: {
    call_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

/**
 * MCP Close 输出数据
 */
interface McpCloseOutput {
  success?: boolean;
  connection_id?: string;
  request?: {
    connection_id?: string;
  };
  timing?: {
    close_ms?: number;
    total_ms?: number;
  };
  error?: string;
}

// ============================================================================
// 内容块渲染组件
// ============================================================================

/** 单个内容块卡片 */
function ContentBlockCard({ block }: { block: Record<string, unknown> }) {
  const blockType = getContentBlockType(block);

  return (
    <div className="border border-border rounded-lg p-2.5">
      {/* 内容块标题 */}
      <div className="flex items-center gap-2 mb-2">
        <ContentBlockIcon type={blockType} />
        <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
          <ContentBlockLabel type={blockType} />
        </Badge>
      </div>

      {blockType === 'text' && <TextBlockContent text={typeof block.text === 'string' ? block.text : undefined} />}
      {blockType === 'image' && <ImageBlockContent block={block} />}
      {blockType === 'resource' && typeof block.resource === 'object' && block.resource !== null && (
        <ResourceBlockContent resource={block.resource as Record<string, unknown>} />
      )}
      {blockType === 'unknown' && (
        <div className="bg-muted rounded p-2">
          <RawJsonViewer data={block} />
        </div>
      )}
    </div>
  );
}

/** text 类型内容 */
function TextBlockContent({ text }: { text: string | undefined }) {
  if (!text) return null;
  return (
    <pre className="text-sm font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto bg-muted rounded p-2">
      {text}
    </pre>
  );
}

/** image 类型内容 */
function ImageBlockContent({ block }: { block: Record<string, unknown> }) {
  const mimeType = typeof block.mimeType === 'string' ? block.mimeType : undefined;
  const data = typeof block.data === 'string' ? block.data : undefined;
  const url = typeof block.url === 'string' ? block.url : undefined;

  return (
    <div className="space-y-1.5">
      {mimeType && (
        <Badge variant="outline" className="font-mono text-xs">
          {mimeType}
        </Badge>
      )}
      {data && (
        <p className="text-xs text-muted-foreground italic">
          Base64 数据 ({data.length} 字符)
        </p>
      )}
      {url && (
        <code className="text-xs font-mono text-foreground break-all bg-muted px-2 py-1 rounded block">
          {url}
        </code>
      )}
    </div>
  );
}

/** resource 类型内容 */
function ResourceBlockContent({ resource }: { resource: Record<string, unknown> }) {
  const uri = typeof resource.uri === 'string' ? resource.uri : undefined;
  const mimeType = typeof resource.mimeType === 'string' ? resource.mimeType : undefined;
  const text = typeof resource.text === 'string' ? resource.text : undefined;

  return (
    <div className="space-y-1.5">
      {uri && (
        <code className="text-xs font-mono text-foreground break-all bg-muted px-2 py-1 rounded block">
          {uri}
        </code>
      )}
      {mimeType && (
        <Badge variant="outline" className="font-mono text-xs">
          {mimeType}
        </Badge>
      )}
      {text && (
        <pre className="text-xs font-mono text-foreground whitespace-pre-wrap break-all max-h-40 overflow-auto bg-muted rounded p-2">
          {text}
        </pre>
      )}
    </div>
  );
}

// ============================================================================
// 公共组件
// ============================================================================

/** MCP 协议标签 */
function McpBadge() {
  return (
    <Badge
      variant="outline"
      className="font-mono text-xs text-cyan-600 dark:text-cyan-400 bg-cyan-50 dark:bg-cyan-950/50 border-0 shrink-0"
    >
      MCP
    </Badge>
  );
}

/** 状态 Badge */
function StatusBadge({ isSuccess, successText, failText }: { isSuccess: boolean; successText: string; failText: string }) {
  return (
    <Badge
      className={cn(
        'font-mono text-xs px-2 py-0.5',
        isSuccess
          ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50'
          : 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/50'
      )}
    >
      {isSuccess ? successText : failText}
    </Badge>
  );
}

/** 错误信息展示 */
function ErrorBlock({ error }: { error?: string }) {
  if (!error) return null;
  return (
    <div className="bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded p-2">
      <p className="text-sm text-red-700 dark:text-red-300 break-all">{error}</p>
    </div>
  );
}

/** 耗时指标卡片 */
function TimingMetric({ label, value }: { label: string; value: number | undefined }) {
  if (value === undefined) return null;
  return (
    <div className="flex justify-between items-center p-2 bg-muted rounded">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono text-foreground">{value}ms</span>
    </div>
  );
}

// ============================================================================
// 内容块类型判断
// ============================================================================

/** MCP 工具调用的 content block 类型 */
type ContentBlockType = 'text' | 'image' | 'resource' | 'unknown';

function getContentBlockType(block: Record<string, unknown>): ContentBlockType {
  if (block.type === 'text') return 'text';
  if (block.type === 'image') return 'image';
  if (block.type === 'resource') return 'resource';
  return 'unknown';
}

function ContentBlockIcon({ type }: { type: ContentBlockType }) {
  switch (type) {
    case 'text': return <FileText className="w-3.5 h-3.5" />;
    case 'image': return <Image className="w-3.5 h-3.5" />;
    case 'resource': return <Box className="w-3.5 h-3.5" />;
    default: return <FileText className="w-3.5 h-3.5" />;
  }
}

function ContentBlockLabel({ type }: { type: ContentBlockType }) {
  switch (type) {
    case 'text': return '文本';
    case 'image': return '图片';
    case 'resource': return '资源';
    default: return '内容';
  }
}

// ============================================================================
// 主组件
// ============================================================================

export const McpViewer = memo(function McpViewer({ report }: McpViewerProps) {
  const ll = getLL();
  const nodeType = report.node_type;

  switch (nodeType) {
    case 'mcp_connect':
      return <McpConnectSection report={report} />;
    case 'mcp_call_tool':
      return <McpCallToolSection report={report} />;
    case 'mcp_read_resource':
      return <McpReadResourceSection report={report} />;
    case 'mcp_get_prompt':
      return <McpGetPromptSection report={report} />;
    case 'mcp_close':
      return <McpCloseSection report={report} />;
    default:
      return (
        <div className="p-4 text-sm text-muted-foreground italic">
          {ll.viewer.mcp.unknown_node_type({ type: nodeType })}
    </div>
  );
  }
});

// ============================================================================
// MCP Connect 查看器
// ============================================================================

function McpConnectSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as McpConnectOutput | undefined;
  const resolvedConfig = report.resolved_config as McpConnectOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  const requestConfig = output?.request || resolvedConfig;
  const configUrl = requestConfig?.url || output?.url || '';
  const connectDuration = output?.timing?.connect_ms;
  const totalDuration = output?.timing?.total_ms;
  const serverInfo = output?.server_info;
  const tools = output?.tools ?? [];
  const resources = output?.resources ?? [];
  const prompts = output?.prompts ?? [];

  return (
    <div className="space-y-3">
      {/* 服务器信息 */}
      <CollapsibleSection
        title={ll.viewer.mcp.section_server_info()}
        icon={<Cable className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* URL */}
          <div className="flex items-center gap-2">
            <McpBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {configUrl || <span className="text-muted-foreground italic">{getLL().execution.viewers.config_url()}</span>}
            </code>
          </div>

          {/* 传输模式 / 协议版本 */}
          {requestConfig && (
            <div className="flex flex-wrap gap-2">
              {requestConfig.transport && (
                <Badge variant="outline" className="font-mono text-xs">
                  传输: {requestConfig.transport === 'stdio' ? 'Stdio' : 'Streamable HTTP'}
                </Badge>
              )}
              {requestConfig.protocol_version && (
                <Badge variant="outline" className="font-mono text-xs">
                  版本: {requestConfig.protocol_version}
                </Badge>
              )}
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 连接状态 */}
      <CollapsibleSection
        title={ll.viewer.mcp.section_connection()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mcp.status_connected()} failText={ll.viewer.mcp.status_connection_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.mcp.connect_success()
                : output?.error ?? ll.viewer.mcp.connect_error()}
            </span>
          </div>

          {/* 连接 ID */}
          {output?.connection_id && (
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{getLL().execution.viewers.connection_id_label()}:</span>
              <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                {output.connection_id}
              </code>
            </div>
          )}

          {/* 服务器信息 */}
          {isSuccess && serverInfo && (
            <div className="flex flex-wrap gap-2">
              {serverInfo.name && (
<Badge variant="outline" className="font-mono text-xs">
                   {getLL().execution.viewers.mcp_server()}: {serverInfo.name}
                 </Badge>
              )}
              {serverInfo.version && (
<Badge variant="outline" className="font-mono text-xs">
                   Version: {serverInfo.version}
                 </Badge>
              )}
            </div>
          )}

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 发现列表 - 工具 */}
      {isSuccess && tools.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.mcp.section_available_tools()}
          icon={<Wrench className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {tools.length} 个工具
            </Badge>
          }
        >
          <div className="space-y-2">
            {tools.map((tool) => (
              <div key={tool.name} className="border border-border rounded-lg p-2.5">
                <div className="font-mono text-sm font-medium text-foreground mb-1">
                  {tool.name}
                </div>
                {tool.description && (
                  <p className="text-xs text-muted-foreground mb-1.5">{tool.description}</p>
                )}
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 发现列表 - 资源 */}
      {isSuccess && resources.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.mcp.section_available_resources()}
          icon={<Box className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {resources.length} 个资源
            </Badge>
          }
        >
          <div className="space-y-2">
            {resources.map((resource) => (
              <div key={resource.uri} className="border border-border rounded-lg p-2.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-mono text-sm font-medium text-foreground">
                    {resource.name}
                  </span>
                  {resource.mime_type && (
                    <Badge variant="outline" className="text-[10px] px-1 py-0 font-mono">
                      {resource.mime_type}
                    </Badge>
                  )}
                </div>
                <code className="text-xs font-mono text-muted-foreground break-all">
                  {resource.uri}
                </code>
                {resource.description && (
                  <p className="text-xs text-muted-foreground mt-1">{resource.description}</p>
                )}
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 发现列表 - 提示词 */}
      {isSuccess && prompts.length > 0 && (
        <CollapsibleSection
          title={ll.viewer.mcp.section_available_prompts()}
          icon={<MessageSquare className="w-4 h-4" />}
          defaultOpen={false}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {prompts.length} 个提示词
            </Badge>
          }
        >
          <div className="space-y-2">
            {prompts.map((prompt) => (
              <div key={prompt.name} className="border border-border rounded-lg p-2.5">
                <div className="font-mono text-sm font-medium text-foreground mb-1">
                  {prompt.name}
                </div>
                {prompt.description && (
                  <p className="text-xs text-muted-foreground mb-1">{prompt.description}</p>
                )}
                {prompt.arguments && prompt.arguments.length > 0 && (
                  <div className="flex flex-wrap gap-1 mt-1">
                    {prompt.arguments.map((arg) => (
                      <Badge
                        key={arg.name}
                        variant="outline"
                        className={cn(
                          'text-[10px] px-1 py-0 font-mono',
                          arg.required
                            ? 'text-amber-600 dark:text-amber-400 border-amber-300 dark:border-amber-700'
                            : ''
                        )}
                      >
                        {arg.name}{arg.required ? '*' : ''}
                      </Badge>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(connectDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-cyan-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="连接耗时" value={connectDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// MCP Call Tool 查看器
// ============================================================================

function McpCallToolSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as McpCallToolOutput | undefined;
  const resolvedConfig = report.resolved_config as McpCallToolOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';
  const isError = output?.is_error ?? false;

  const requestConfig = output?.request || resolvedConfig;
  const toolName = output?.tool_name || requestConfig?.tool_name || '';
  const callDuration = output?.timing?.call_ms;
  const totalDuration = output?.timing?.total_ms;
  const contentBlocks = output?.result?.content ?? [];

  return (
    <div className="space-y-3">
      {/* 调用配置 */}
      <CollapsibleSection
        title="调用配置"
        icon={<Wrench className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 工具名称 */}
          <div className="flex items-center gap-2">
            <McpBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {toolName || <span className="text-muted-foreground italic">未配置工具名称</span>}
            </code>
          </div>

          {/* 连接 ID / 参数 */}
          {requestConfig && (
            <div className="flex flex-wrap gap-2">
              {requestConfig.connection_id && (
                <Badge variant="outline" className="font-mono text-xs">
                  连接: {requestConfig.connection_id}
                </Badge>
              )}
            </div>
          )}

          {/* 参数 JSON */}
          {requestConfig?.arguments && Object.keys(requestConfig.arguments).length > 0 && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">调用参数</div>
              <div className="bg-muted rounded p-2">
                <RawJsonViewer data={requestConfig.arguments} />
              </div>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 调用状态 */}
      <CollapsibleSection
        title={ll.viewer.mcp.section_tool_result()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge
              isSuccess={isSuccess && !isError}
              successText={ll.viewer.mcp.status_call_success()}
              failText={ll.viewer.mcp.status_call_failed()}
            />
            <span className="text-sm text-muted-foreground">
              {isSuccess && !isError
                ? `${toolName} 调用成功`
                : output?.error ?? '工具调用失败'}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 内容块 */}
      {contentBlocks.length > 0 && (
        <CollapsibleSection
          title="返回内容"
          icon={<FileText className="w-4 h-4" />}
          defaultOpen={true}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {contentBlocks.length} 个内容块
            </Badge>
          }
        >
          <div className="space-y-2">
            {contentBlocks.map((block, idx) => (
              <ContentBlockCard key={idx} block={block} />
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(callDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-cyan-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="调用耗时" value={callDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// MCP Read Resource 查看器
// ============================================================================

function McpReadResourceSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as McpReadResourceOutput | undefined;
  const resolvedConfig = report.resolved_config as McpReadResourceOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  const requestConfig = output?.request || resolvedConfig;
  const resourceUri = output?.resource_uri || requestConfig?.resource_uri || '';
  const contents = output?.contents ?? [];
  const readDuration = output?.timing?.read_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 资源配置 */}
      <CollapsibleSection
        title="资源配置"
        icon={<FileText className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* URI */}
          <div className="flex items-center gap-2">
            <McpBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {resourceUri || <span className="text-muted-foreground italic">未配置资源 URI</span>}
            </code>
          </div>

          {requestConfig?.connection_id && (
            <div className="flex flex-wrap gap-2">
              <Badge variant="outline" className="font-mono text-xs">
                连接: {requestConfig.connection_id}
              </Badge>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 读取状态 */}
      <CollapsibleSection
        title={ll.viewer.mcp.section_resource_content()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="读取成功" failText="读取失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `资源读取成功，${contents.length} 个内容块`
                : output?.error ?? '资源读取失败'}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 资源内容 */}
      {contents.length > 0 && (
        <CollapsibleSection
          title="资源内容"
          icon={<Server className="w-4 h-4" />}
          defaultOpen={true}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {contents.length} 个内容块
            </Badge>
          }
        >
          <div className="space-y-2">
            {contents.map((content, idx) => (
              <div key={idx} className="border border-border rounded-lg p-2.5">
                {/* MIME 类型标签 */}
                {typeof content.mimeType === 'string' && (
                  <div className="mb-2">
                    <Badge variant="outline" className="font-mono text-xs">
                      {content.mimeType}
                    </Badge>
                  </div>
                )}

                {/* URI 标签 */}
                {typeof content.uri === 'string' && (
                  <div className="mb-2">
                    <code className="text-xs font-mono text-muted-foreground break-all">
                      {content.uri}
                    </code>
                  </div>
                )}

                {/* 文本内容 */}
                {typeof content.text === 'string' && (
                  <pre className="text-sm font-mono text-foreground whitespace-pre-wrap break-all max-h-60 overflow-auto bg-muted rounded p-2">
                    {content.text}
                  </pre>
                )}

                {/* Base64/Blob 数据 */}
                {typeof content.data === 'string' && content.text === undefined && (
                  <p className="text-xs text-muted-foreground italic">
                    二进制数据 ({content.data.length} 字符)
                  </p>
                )}

                {/* 其他内容：JSON 展示 */}
                {content.text === undefined && content.data === undefined && (
                  <div className="bg-muted rounded p-2">
                    <RawJsonViewer data={content} />
                  </div>
                )}
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(readDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-cyan-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="读取耗时" value={readDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// MCP Get Prompt 查看器
// ============================================================================

function McpGetPromptSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as McpGetPromptOutput | undefined;
  const resolvedConfig = report.resolved_config as McpGetPromptOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  const requestConfig = output?.request || resolvedConfig;
  const promptName = output?.prompt_name || requestConfig?.prompt_name || '';
  const callDuration = output?.timing?.call_ms;
  const totalDuration = output?.timing?.total_ms;
  const messages = output?.result?.messages ?? [];
  const description = output?.result?.description;

  return (
    <div className="space-y-3">
      {/* 提示词配置 */}
      <CollapsibleSection
        title="提示词配置"
        icon={<MessageSquare className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 提示词名称 */}
          <div className="flex items-center gap-2">
            <McpBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {promptName || <span className="text-muted-foreground italic">未配置提示词名称</span>}
            </code>
          </div>

          {requestConfig?.connection_id && (
            <div className="flex flex-wrap gap-2">
              <Badge variant="outline" className="font-mono text-xs">
                连接: {requestConfig.connection_id}
              </Badge>
            </div>
          )}

          {/* 调用参数 */}
          {requestConfig?.arguments && Object.keys(requestConfig.arguments).length > 0 && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1.5">调用参数</div>
              <div className="bg-muted rounded p-2">
                <RawJsonViewer data={requestConfig.arguments} />
              </div>
            </div>
          )}
        </div>
      </CollapsibleSection>

      {/* 获取状态 */}
      <CollapsibleSection
        title={ll.viewer.mcp.section_prompt_content()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText="获取成功" failText="获取失败" />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? `提示词获取成功，${messages.length} 条消息`
                : output?.error ?? '提示词获取失败'}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 提示词描述 */}
      {description && (
        <div className="px-3 py-2 bg-muted/30 rounded-lg">
          <div className="text-xs text-muted-foreground mb-1">描述</div>
          <p className="text-sm text-foreground">{description}</p>
        </div>
      )}

      {/* 消息列表 */}
      {messages.length > 0 && (
        <CollapsibleSection
          title="消息列表"
          icon={<Server className="w-4 h-4" />}
          defaultOpen={true}
          extra={
            <Badge variant="outline" className="font-mono text-xs">
              {messages.length} 条消息
            </Badge>
          }
        >
          <div className="space-y-2">
            {messages.map((msg, idx) => (
              <div key={idx} className="border border-border rounded-lg p-2.5">
                {/* Role */}
                {typeof msg.role === 'string' && (
                  <div className="mb-2">
                    <Badge
                      variant="outline"
                      className={cn(
                        'font-mono text-xs',
                        msg.role === 'user'
                          ? 'text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-700'
                          : msg.role === 'assistant'
                            ? 'text-emerald-600 dark:text-emerald-400 border-emerald-300 dark:border-emerald-700'
                            : ''
                      )}
                    >
                      {msg.role}
                    </Badge>
                  </div>
                )}

                {/* 内容 */}
                {msg.content !== undefined && (
                  typeof msg.content === 'string' ? (
                    <pre className="text-sm font-mono text-foreground whitespace-pre-wrap break-all max-h-40 overflow-auto bg-muted rounded p-2">
                      {msg.content}
                    </pre>
                  ) : (
                    <div className="bg-muted rounded p-2">
                      <RawJsonViewer data={msg.content} />
                    </div>
                  )
                )}
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* 耗时分解 */}
      {(callDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-cyan-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="调用耗时" value={callDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}

// ============================================================================
// MCP Close 查看器
// ============================================================================

function McpCloseSection({ report }: { report: NodeExecutionReport }) {
  const ll = getLL();
  const output = report.output_data as McpCloseOutput | undefined;
  const resolvedConfig = report.resolved_config as McpCloseOutput['request'] | undefined;
  const isSuccess = output?.success ?? report.status === 'success';

  const requestConfig = output?.request || resolvedConfig;
  const connectionId = output?.connection_id || requestConfig?.connection_id || '';
  const closeDuration = output?.timing?.close_ms;
  const totalDuration = output?.timing?.total_ms;

  return (
    <div className="space-y-3">
      {/* 关闭配置 */}
      <CollapsibleSection
        title="关闭配置"
        icon={<Unplug className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-3">
          {/* 连接 ID */}
          <div className="flex items-center gap-2">
            <McpBadge />
            <code className="flex-1 text-sm font-mono text-foreground break-all bg-muted px-2 py-1 rounded">
              {connectionId || <span className="text-muted-foreground italic">未指定连接 ID</span>}
            </code>
          </div>
        </div>
      </CollapsibleSection>

      {/* 关闭状态 */}
      <CollapsibleSection
        title={ll.viewer.mcp.section_close()}
        icon={<Globe className="w-4 h-4" />}
        defaultOpen={true}
      >
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <StatusBadge isSuccess={isSuccess} successText={ll.viewer.mcp.status_closed()} failText={ll.viewer.mcp.status_close_failed()} />
            <span className="text-sm text-muted-foreground">
              {isSuccess
                ? ll.viewer.mcp.close_success()
                : output?.error ?? ll.viewer.mcp.close_error()}
            </span>
          </div>

          <ErrorBlock error={!isSuccess ? output?.error : undefined} />
        </div>
      </CollapsibleSection>

      {/* 耗时分解 */}
      {(closeDuration !== undefined || totalDuration !== undefined) && (
        <CollapsibleSection
          title="耗时分解"
          icon={<Clock className="w-4 h-4" />}
          defaultOpen={false}
          extra={totalDuration !== undefined && (
            <span className="text-xs font-mono text-cyan-600 font-medium">
              总计 {totalDuration}ms
            </span>
          )}
        >
          <div className="grid grid-cols-2 gap-2 text-sm">
            <TimingMetric label="关闭耗时" value={closeDuration} />
            <TimingMetric label="总耗时" value={totalDuration} />
          </div>
        </CollapsibleSection>
      )}

      {/* 变量变更 */}
      <VariableChangesCard changes={report.variable_changes} />
    </div>
  );
}
