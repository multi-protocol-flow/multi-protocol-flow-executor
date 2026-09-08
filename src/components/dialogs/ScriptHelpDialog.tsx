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
  Sparkles,
} from 'lucide-react';
import { useUIStore } from '@/stores/ui';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

/** 打开外部链接（优先通过 Tauri opener 唤起系统浏览器，兜底通过 window.open） */
async function openUrl(url: string) {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } catch {
    window.open(url, '_blank', 'noopener,noreferrer');
  }
}

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
          <th className="px-3 py-2 text-left font-medium w-[45%]">{getLL().flow.api_table_method()}</th>
          <th className="px-3 py-2 text-left font-medium">{getLL().flow.api_table_desc()}</th>
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
    {
      id: 'overview',
      title: getLL().flow.script_section_overview(),
      icon: <BookOpen className="h-4 w-4" />,
      content: (
        <div className="text-xs text-muted-foreground space-y-2">
          <p>{getLL().flow.script_section_overview_desc1()}</p>
          <ul className="list-disc list-inside space-y-1 ml-1">
            <li>{getLL().flow.script_section_overview_list1()}</li>
            <li>{getLL().flow.script_section_overview_list2()}</li>
            <li>{getLL().flow.script_section_overview_list3()}</li>
          </ul>
        </div>
      ),
    },
    {
      id: 'variables',
      title: getLL().flow.script_section_variables(),
      icon: <Variable className="h-4 w-4" />,
      content: (
        <>
          <ApiTable rows={[
            { method: 'get(key)', description: getLL().flow.fn_get_value() },
            { method: 'set(key, value)', description: getLL().flow.fn_set_value() },
            { method: 'setAll(obj)', description: getLL().flow.fn_set_all() },
            { method: 'replaceIn(template)', description: getLL().flow.fn_replace_in() },
            { method: 'has(key)', description: getLL().flow.fn_has() },
            { method: 'unset(key)', description: getLL().flow.fn_unset() },
            { method: 'clear()', description: getLL().flow.fn_clear() },
            { method: 'all()', description: getLL().flow.fn_get_all() },
          ]} />
          <div className="flex items-center justify-between pt-1">
            <span className="text-xs text-muted-foreground">
              {getLL().flow.script_help_dynamic_vars_support()}{' '}
              <code className="text-[11px] font-mono text-emerald-600 dark:text-emerald-400">fn.variables.get('$guid')</code>
            </span>
            <button
              type="button"
              onClick={() => {
                onOpenChange(false);
                useUIStore.getState().openDynamicVariablesHelpDialog();
              }}
              className="inline-flex items-center gap-1 text-xs text-emerald-600 hover:text-emerald-700 hover:underline transition-colors"
            >
              <Sparkles className="h-3 w-3" />
              {getLL().flow.script_help_dynamic_vars_link()}
            </button>
          </div>
          <CodeBlock code={`fn.variables.set('token', 'abc123');
const token = fn.variables.get('token');
${getLL().flow.script_comment_dynamic_vars()}
const guid = fn.variables.get('$guid');
const authHeader = fn.variables.replaceIn('Bearer {{$guid}}');
const allVars = JSON.parse(fn.variables.all());`} />
        </>
      ),
    },
    {
      id: 'response',
      title: getLL().flow.script_section_response(),
      icon: <Globe className="h-4 w-4" />,
      content: (
        <>
          <p className="text-xs text-muted-foreground mb-2">
            {getLL().flow.script_section_response_desc()}{' '}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">fn.response.raw</code>{' '}
            {getLL().flow.script_section_response_raw()}
          </p>
          <ApiTable rows={[
            { method: 'code', description: getLL().flow.fn_response_code() },
            { method: 'status', description: getLL().flow.fn_response_status() },
            { method: 'headers', description: getLL().flow.fn_response_headers() },
            { method: 'body', description: getLL().flow.fn_response_body() },
            { method: 'raw', description: getLL().flow.fn_response_raw() },
            { method: 'size', description: getLL().flow.fn_response_size() },
            { method: 'time', description: getLL().flow.fn_response_time() },
            { method: 'text()', description: getLL().flow.fn_response_text() },
            { method: 'json()', description: getLL().flow.fn_response_json() },
          ]} />
          <CodeBlock code={`${getLL().flow.script_comment_all_nodes()}
const data = JSON.parse(fn.response.raw);
fn.variables.set('count', data.rows.length.toString());
fn.variables.set('firstName', data.rows[0].name);

${getLL().flow.script_comment_http_node()}
const code = fn.response.code;
const body = JSON.parse(fn.response.body);
fn.variables.set('userId', body.user.id);`} />
        </>
      ),
    },
    {
      id: 'request',
      title: getLL().flow.script_section_request(),
      icon: <Send className="h-4 w-4" />,
      content: (
        <ApiTable rows={[
          { method: 'url', description: getLL().flow.fn_request_url() },
          { method: 'method', description: getLL().flow.fn_request_method() },
          { method: 'headers', description: getLL().flow.fn_request_headers() },
          { method: 'body', description: getLL().flow.fn_request_body() },
        ]} />
      ),
    },
    {
      id: 'console',
      title: getLL().flow.script_section_console(),
      icon: <Terminal className="h-4 w-4" />,
      content: (
        <ApiTable rows={[
          { method: 'log(message)', description: getLL().flow.fn_console_log() },
          { method: 'info(message)', description: getLL().flow.fn_console_info() },
          { method: 'warn(message)', description: getLL().flow.fn_console_warn() },
          { method: 'error(message)', description: getLL().flow.fn_console_error() },
          { method: 'debug(message)', description: getLL().flow.fn_console_debug() },
          { method: 'table(data)', description: getLL().flow.fn_console_table() },
        ]} />
      ),
    },
    {
      id: 'flow',
      title: getLL().flow.script_section_flow(),
      icon: <GitBranch className="h-4 w-4" />,
      content: (
        <>
          <ApiTable rows={[
            { method: 'skip()', description: getLL().flow.fn_flow_skip() },
            { method: 'abort(message)', description: getLL().flow.fn_flow_abort() },
            { method: 'stop()', description: getLL().flow.fn_flow_stop() },
          ]} />
          <CodeBlock code={`if (fn.response.code !== 200) {
  fn.flow.abort('${getLL().flow.script_error_request_failed()}' + fn.response.code);
}
if (JSON.parse(fn.response.body).done) { fn.flow.stop(); }
if (JSON.parse(fn.response.body).skip) { fn.flow.skip(); }`} />
        </>
      ),
    },
    {
      id: 'info',
      title: getLL().flow.script_section_info(),
      icon: <Info className="h-4 w-4" />,
      content: (
        <ApiTable rows={[
          { method: 'flowName', description: getLL().flow.fn_info_flow_name() },
          { method: 'flowId', description: getLL().flow.fn_info_flow_id() },
          { method: 'nodeId', description: getLL().flow.fn_info_node_id() },
          { method: 'nodeName', description: getLL().flow.fn_info_node_name() },
        ]} />
      ),
    },
    {
      id: 'util',
      title: getLL().flow.script_section_util(),
      icon: <Wrench className="h-4 w-4" />,
      content: (
        <div className="space-y-3">
          <UtilSubCategory title={getLL().flow.script_sub_encoding()} items={[
            'base64Encode(str)', 'base64Decode(str)', 'urlEncode(str)',
            'urlDecode(str)', 'htmlEscape(str)', 'htmlUnescape(str)',
          ]} />
          <UtilSubCategory title={getLL().flow.script_sub_crypto()} items={[
            'md5(str)', 'sha256(str)', 'sha512(str)',
            'hmacSha256(key, data)', 'hmacSha512(key, data)',
          ]} />
          <UtilSubCategory title={getLL().flow.script_sub_time()} items={[
            'timestamp()', 'now()', 'formatTime(format)', 'parseTime(dateStr, format)', 'sleep(ms)',
          ]} />
          <UtilSubCategory title={getLL().flow.script_sub_uuid()} items={['uuid()', 'uuidv4()']} />
          <UtilSubCategory title={getLL().flow.script_sub_json()} items={[
            'jsonParse(str)', 'jsonStringify(obj, indent?)', 'jsonPath(objJson, path)',
          ]} />
          <UtilSubCategory title={getLL().flow.script_sub_type_check()} items={[
            'isArray(v)', 'isObject(v)', 'isString(v)', 'isNumber(v)',
            'isBoolean(v)', 'isNull(v)', 'isUndefined(v)', 'isFunction(v)',
            'isEmpty(v)', 'typeOf(v)',
          ]} />
          <UtilSubCategory title={getLL().flow.script_sub_object_ops()} items={[
            'keys(obj)', 'values(obj)', 'entries(obj)', 'hasKey(obj, key)',
            'pick(obj, keys)', 'omit(obj, keys)', 'merge(target, source)', 'clone(obj)',
          ]} />
        </div>
      ),
    },
    {
      id: 'expect',
      title: getLL().flow.script_section_expect(),
      icon: <CheckCheck className="h-4 w-4" />,
      content: (
        <>
          <div className="text-xs text-muted-foreground mb-2">
            {getLL().flow.script_modifier_desc()}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.to</code>{' '}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.be</code>{' '}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.have</code>{' '}
            <code className="bg-muted/60 px-1.5 py-0.5 rounded text-[11px] font-mono">.not</code>{' '}
            {getLL().flow.script_modifier_chain()}
          </div>
          <ApiTable rows={[
            { method: 'equal(expected)', description: getLL().flow.fn_expect_equal() },
            { method: 'eql(expected)', description: getLL().flow.fn_expect_eql() },
            { method: 'notEqual(expected)', description: getLL().flow.fn_expect_not_equal() },
            { method: 'exist()', description: getLL().flow.fn_expect_exist() },
            { method: 'beDefined()', description: getLL().flow.fn_expect_be_defined() },
            { method: 'beNull()', description: getLL().flow.fn_expect_be_null() },
            { method: 'notBeNull()', description: getLL().flow.fn_expect_not_be_null() },
            { method: 'beTrue()', description: getLL().flow.fn_expect_be_true() },
            { method: 'beFalse()', description: getLL().flow.fn_expect_be_false() },
            { method: 'beTruthy()', description: getLL().flow.fn_expect_be_truthy() },
            { method: 'beFalsy()', description: getLL().flow.fn_expect_be_falsy() },
            { method: 'contain(expected)', description: getLL().flow.fn_expect_contain() },
            { method: 'containKey(key)', description: getLL().flow.fn_expect_contain_key() },
            { method: 'haveLength(n)', description: getLL().flow.fn_expect_have_length() },
            { method: 'match(regex)', description: getLL().flow.fn_expect_match() },
            { method: 'startWith(prefix)', description: getLL().flow.fn_expect_start_with() },
            { method: 'endWith(suffix)', description: getLL().flow.fn_expect_end_with() },
            { method: 'above(n)', description: getLL().flow.fn_expect_above() },
            { method: 'below(n)', description: getLL().flow.fn_expect_below() },
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
        <div className="shrink-0 px-6 pt-6 pb-3">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-violet-100 flex items-center justify-center">
              <BookOpen className="w-5 h-5 text-violet-600" />
            </div>
            <div>
              <DialogTitle className="text-lg">{getLL().flow.script_help_title()}</DialogTitle>
              <div className="flex items-center gap-2 mt-0.5">
                <p className="text-sm text-muted-foreground">
                  {getLL().flow.script_help_subtitle()}
                </p>
                <button
                  type="button"
                  onClick={() => openUrl('https://bellard.org/quickjs/')}
                  className="inline-flex items-center gap-1 text-xs text-violet-600 hover:text-violet-700 hover:underline transition-colors"
                >
                  <ExternalLink className="h-3 w-3" />
                  {getLL().flow.script_help_quickjs_doc()}
                </button>
                <span className="text-muted-foreground/40">|</span>
                <button
                  type="button"
                  onClick={() => openUrl('https://262.ecma-international.org/6.0/')}
                  className="inline-flex items-center gap-1 text-xs text-violet-600 hover:text-violet-700 hover:underline transition-colors"
                >
                  <ExternalLink className="h-3 w-3" />
                  {getLL().flow.script_help_es6_tutorial()}
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

        <div className="shrink-0 px-6 py-4 border-t border-border flex justify-end">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {getLL().flow.script_help_close()}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
