/**
 * ScriptHelpDialog - 脚本节点 API 参考弹窗
 * 展示 QuickJS 沙箱环境中所有可用的 fn.* API 详细说明
 */
import { useState, type ReactNode } from 'react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  BookOpen,
  ExternalLink,
  ChevronRight,
  Variable,
  Globe,
  Send,
  Terminal,
  GitBranch,
  Info,
  Wrench,
  CheckCheck,
} from 'lucide-react';
import { cn } from '@/lib/utils';

export interface ScriptHelpDialogProps {
  /** 是否显示弹窗 */
  open: boolean;
  /** 弹窗开关回调 */
  onOpenChange: (open: boolean) => void;
}

/** 表格行数据 */
interface TableRow {
  method: string;
  description: string;
}

/** 可折叠 Section 数据 */
interface SectionData {
  id: string;
  title: string;
  icon: ReactNode;
  defaultOpen?: boolean;
  content: ReactNode;
}

/**
 * 可折叠帮助 Section 组件
 */
function HelpSection({
  title,
  icon,
  defaultOpen = true,
  children,
}: {
  title: string;
  icon: ReactNode;
  defaultOpen?: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className={cn(
          'w-full flex items-center gap-2 px-4 py-2.5 text-left',
          'bg-muted/30 hover:bg-muted/50 transition-colors',
          'focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1'
        )}
      >
        <ChevronRight
          className={cn(
            'h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform duration-200',
            open && 'rotate-90'
          )}
        />
        <span className="text-muted-foreground shrink-0">{icon}</span>
        <span className="text-sm font-medium text-foreground">{title}</span>
      </button>
      {open && (
        <div className="px-4 py-3 space-y-3">
          {children}
        </div>
      )}
    </div>
  );
}

/**
 * API 方法表格
 */
function ApiTable({ rows }: { rows: TableRow[] }) {
  return (
    <table className="w-full text-xs">
      <thead>
        <tr className="bg-muted/50">
          <th className="px-3 py-2 text-left font-medium w-[45%]">方法 / 属性</th>
          <th className="px-3 py-2 text-left font-medium">说明</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.method}>
            <td className="px-3 py-2 border-t border-border">
              <code className="text-[11px] font-mono bg-muted/60 px-1.5 py-0.5 rounded">{row.method}</code>
            </td>
            <td className="px-3 py-2 border-t border-border text-muted-foreground">{row.description}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/**
 * 代码示例块
 */
function CodeBlock({ code }: { code: string }) {
  return (
    <div className="relative">
      <span className="absolute top-2 right-2 text-[10px] text-muted-foreground/60 font-mono">javascript</span>
      <pre className="bg-muted/50 rounded-md p-3 text-xs font-mono overflow-x-auto">
        <code>{code}</code>
      </pre>
    </div>
  );
}

/**
 * 工具函数子类别
 */
function UtilSubCategory({ title, items }: { title: string; items: string[] }) {
  return (
    <div>
      <p className="text-xs font-medium text-foreground mb-1.5">{title}</p>
      <div className="flex flex-wrap gap-1.5">
        {items.map((item) => (
          <code key={item} className="text-[11px] font-mono bg-muted/60 px-1.5 py-0.5 rounded">
            {item}
          </code>
        ))}
      </div>
    </div>
  );
}

export function ScriptHelpDialog({ open, onOpenChange }: ScriptHelpDialogProps) {
  const sections: SectionData[] = [
    // 概述
    {
      id: 'overview',
      title: '概述',
      icon: <BookOpen className="h-4 w-4" />,
      content: (
        <div className="text-xs text-muted-foreground space-y-2">
          <p>脚本节点基于 QuickJS 运行时，提供安全沙箱环境。</p>
          <ul className="list-disc list-inside space-y-1 ml-1">
            <li>支持 ES2020 语法（async/await、Promise、可选链等）</li>
            <li>冷启动 ~2-5ms，内存占用 ~500KB</li>
            <li>禁止文件系统、网络请求、模块加载、外部进程访问</li>
          </ul>
        </div>
      ),
    },
    // fn.variables
    {
      id: 'variables',
      title: 'fn.variables - 变量操作',
      icon: <Variable className="h-4 w-4" />,
      content: (
        <>
          <ApiTable rows={[
            { method: 'get(key)', description: '获取变量值' },
            { method: 'set(key, value)', description: '设置单个变量' },
            { method: 'setAll(obj)', description: '批量设置变量' },
            { method: 'has(key)', description: '检查变量是否存在' },
            { method: 'unset(key)', description: '删除变量' },
            { method: 'clear()', description: '清空所有变量' },
            { method: 'all()', description: '获取所有变量（JSON 字符串）' },
          ]} />
          <CodeBlock code={`fn.variables.set('token', 'abc123');
fn.variables.setAll({ token: 'abc123', userId: 'user-001' });
const token = fn.variables.get('token');
if (fn.variables.has('token')) { fn.console.log('exists'); }
const allVars = JSON.parse(fn.variables.all());`} />
        </>
      ),
    },
    // fn.response
    {
      id: 'response',
      title: 'fn.response - 节点输出数据（只读）',
      icon: <Globe className="h-4 w-4" />,
      content: (
        <>
          <p className="text-xs text-muted-foreground mb-2">
            兼容所有协议节点（HTTP/数据库/WebSocket/gRPC/MQTT 等）。
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono ml-1">fn.response.raw</code> 返回完整原始输出。
          </p>
          <ApiTable rows={[
            { method: 'code', description: 'HTTP 状态码（非 HTTP 节点为 0）' },
            { method: 'status', description: '状态文本（非 HTTP 节点为空）' },
            { method: 'headers', description: '响应头对象（非 HTTP 节点为空）' },
            { method: 'body', description: '响应体（JSON 字符串，HTTP 节点专用）' },
            { method: 'raw', description: '完整原始输出（JSON 字符串），含 status/headers/body 等全部字段' },
            { method: 'size', description: '响应大小（字节）' },
            { method: 'time', description: '响应时间（毫秒）' },
            { method: 'text()', description: '获取原始文本' },
            { method: 'json()', description: '获取 JSON 对象（JSON 字符串）' },
          ]} />
          <CodeBlock code={`// 任意节点类型 — 获取完整输出
const data = JSON.parse(fn.response.raw);
fn.variables.set('count', data.rows.length.toString());
fn.variables.set('firstName', data.rows[0].name);

// HTTP 节点（向下兼容）
const code = fn.response.code;
const body = JSON.parse(fn.response.body);
fn.variables.set('userId', body.user.id);`} />
        </>
      ),
    },
    // fn.request
    {
      id: 'request',
      title: 'fn.request - 请求数据（只读）',
      icon: <Send className="h-4 w-4" />,
      content: (
        <ApiTable rows={[
          { method: 'url', description: '请求 URL' },
          { method: 'method', description: '请求方法' },
          { method: 'headers', description: '请求头对象' },
          { method: 'body', description: '请求体（JSON 字符串）' },
        ]} />
      ),
    },
    // fn.console
    {
      id: 'console',
      title: 'fn.console - 日志输出',
      icon: <Terminal className="h-4 w-4" />,
      content: (
        <ApiTable rows={[
          { method: 'log(message)', description: '普通日志' },
          { method: 'info(message)', description: '信息日志' },
          { method: 'warn(message)', description: '警告日志' },
          { method: 'error(message)', description: '错误日志' },
          { method: 'debug(message)', description: '调试日志' },
          { method: 'table(data)', description: '表格数据' },
        ]} />
      ),
    },
    // fn.flow
    {
      id: 'flow',
      title: 'fn.flow - 流程控制',
      icon: <GitBranch className="h-4 w-4" />,
      content: (
        <>
          <ApiTable rows={[
            { method: 'skip()', description: '跳过后续节点，流程继续' },
            { method: 'abort(message)', description: '终止流程并报错' },
            { method: 'stop()', description: '正常终止流程' },
          ]} />
          <CodeBlock code={`if (fn.response.code !== 200) {
  fn.flow.abort('请求失败: ' + fn.response.code);
}
if (JSON.parse(fn.response.body).done) { fn.flow.stop(); }
if (JSON.parse(fn.response.body).skip) { fn.flow.skip(); }`} />
        </>
      ),
    },
    // fn.info
    {
      id: 'info',
      title: 'fn.info - 流程信息（只读）',
      icon: <Info className="h-4 w-4" />,
      content: (
        <ApiTable rows={[
          { method: 'flowName', description: '流程名称' },
          { method: 'flowId', description: '流程 ID' },
          { method: 'nodeId', description: '当前节点 ID' },
          { method: 'nodeName', description: '当前节点名称' },
        ]} />
      ),
    },
    // fn.util
    {
      id: 'util',
      title: 'fn.util - 工具函数',
      icon: <Wrench className="h-4 w-4" />,
      content: (
        <div className="space-y-3">
          <UtilSubCategory title="编码函数" items={[
            'base64Encode(str)', 'base64Decode(str)', 'urlEncode(str)',
            'urlDecode(str)', 'htmlEscape(str)', 'htmlUnescape(str)',
          ]} />
          <UtilSubCategory title="加密函数" items={[
            'md5(str)', 'sha256(str)', 'sha512(str)',
            'hmacSha256(key, data)', 'hmacSha512(key, data)',
          ]} />
          <UtilSubCategory title="时间函数" items={[
            'timestamp()', 'now()', 'formatTime(format)', 'parseTime(dateStr, format)',
          ]} />
          <UtilSubCategory title="UUID 函数" items={['uuid()', 'uuidv4()']} />
          <UtilSubCategory title="JSON 函数" items={[
            'jsonParse(str)', 'jsonStringify(obj, indent?)', 'jsonPath(objJson, path)',
          ]} />
          <UtilSubCategory title="类型检查" items={[
            'isArray(v)', 'isObject(v)', 'isString(v)', 'isNumber(v)',
            'isBoolean(v)', 'isNull(v)', 'isUndefined(v)', 'isFunction(v)',
            'isEmpty(v)', 'typeOf(v)',
          ]} />
          <UtilSubCategory title="对象操作" items={[
            'keys(obj)', 'values(obj)', 'entries(obj)', 'hasKey(obj, key)',
            'pick(obj, keys)', 'omit(obj, keys)', 'merge(target, source)', 'clone(obj)',
          ]} />
        </div>
      ),
    },
    // fn.expect
    {
      id: 'expect',
      title: 'fn.expect() - 链式断言',
      icon: <CheckCheck className="h-4 w-4" />,
      content: (
        <>
          <div className="text-xs text-muted-foreground mb-2">
            修饰符：<code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.to</code>{' '}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.be</code>{' '}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.have</code>{' '}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.not</code>{' '}
            （链式调用 / 反转）
          </div>
          <ApiTable rows={[
            { method: 'equal(expected)', description: '严格相等' },
            { method: 'eql(expected)', description: '深度相等' },
            { method: 'notEqual(expected)', description: '不相等' },
            { method: 'exist()', description: '存在（非 null/undefined）' },
            { method: 'beDefined()', description: '已定义' },
            { method: 'beNull()', description: '为 null' },
            { method: 'notBeNull()', description: '不为 null' },
            { method: 'beTrue()', description: '为 true' },
            { method: 'beFalse()', description: '为 false' },
            { method: 'beTruthy()', description: '为真值' },
            { method: 'beFalsy()', description: '为假值' },
            { method: 'contain(expected)', description: '包含（数组/字符串）' },
            { method: 'containKey(key)', description: '包含键' },
            { method: 'haveLength(n)', description: '长度为 n' },
            { method: 'match(regex)', description: '正则匹配' },
            { method: 'startWith(prefix)', description: '以指定字符串开头' },
            { method: 'endWith(suffix)', description: '以指定字符串结尾' },
            { method: 'above(n)', description: '大于 n' },
            { method: 'below(n)', description: '小于 n' },
          ]} />
          <CodeBlock code={`fn.expect(fn.response.code).to.equal(200);
fn.expect(fn.response.body).to.containKey('token');
fn.expect(fn.response.code).not.to.equal(500);
fn.expect(result).to.eql({ status: 'ok' });`} />
        </>
      ),
    },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-[680px] h-[85vh] flex flex-col overflow-hidden p-0 gap-0"
        hideCloseButton
      >
        {/* 固定头部 */}
        <div className="shrink-0 px-6 pt-6 pb-3">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-violet-100 flex items-center justify-center">
              <BookOpen className="w-5 h-5 text-violet-600" />
            </div>
            <div>
              <DialogTitle className="text-lg">脚本节点 API 参考</DialogTitle>
              <div className="flex items-center gap-2 mt-0.5">
                <p className="text-sm text-muted-foreground">
                  QuickJS 沙箱环境 · ES2020 语法 · 安全隔离执行
                </p>
                <button
                  type="button"
                  onClick={() => openUrl('https://bellard.org/quickjs/')}
                  className="inline-flex items-center gap-1 text-xs text-violet-600 hover:text-violet-700 hover:underline transition-colors"
                >
                  <ExternalLink className="h-3 w-3" />
                  QuickJS 文档
                </button>
                <span className="text-muted-foreground/40">|</span>
                <button
                  type="button"
                  onClick={() => openUrl('https://es6.ruanyifeng.com/')}
                  className="inline-flex items-center gap-1 text-xs text-violet-600 hover:text-violet-700 hover:underline transition-colors"
                >
                  <ExternalLink className="h-3 w-3" />
                  ES6 入门教程
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* 可滚动内容区 */}
        <div className="flex-1 min-h-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="px-6 pb-4 space-y-3">
              {sections.map((section) => (
                <HelpSection
                  key={section.id}
                  title={section.title}
                  icon={section.icon}
                >
                  {section.content}
                </HelpSection>
              ))}
            </div>
          </ScrollArea>
        </div>

        {/* 固定底部 */}
        <div className="shrink-0 px-6 py-4 border-t border-border flex justify-end">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
