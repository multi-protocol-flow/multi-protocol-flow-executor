/**
 * DynamicVariablesHelpDialog - 动态变量参考手册弹窗
 * 展示系统中支持的所有内置动态变量（如 {{$guid}}, {{$timestamp}} 等），
 * 并提供即时示例刷新、搜索过滤与一键复制功能。
 */
import { useState, useEffect, useMemo, type ReactNode } from 'react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Sparkles,
  ChevronRight,
  Copy,
  Check,
  RefreshCw,
  Search,
  Hash,
  Calendar,
  Binary,
  Globe,
  User,
  Mail,
  FileText,
  MapPin,
  X,
} from 'lucide-react';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import {
  DYNAMIC_VARIABLES,
  type DynamicVariableCategory,
  type DynamicVariableItem,
} from '@/types/dynamicVariable';

export interface DynamicVariablesHelpDialogProps {
  /** 是否显示弹窗 */
  open: boolean;
  /** 弹窗开关回调 */
  onOpenChange: (open: boolean) => void;
}

interface CategoryConfig {
  key: DynamicVariableCategory;
  getTitle: () => string;
  icon: ReactNode;
}

const CATEGORY_CONFIGS: CategoryConfig[] = [
  {
    key: 'common',
    getTitle: () => getLL().flow.dynamic_cat_common(),
    icon: <Hash className="w-4 h-4" />,
  },
  {
    key: 'datetime',
    getTitle: () => getLL().flow.dynamic_cat_datetime(),
    icon: <Calendar className="w-4 h-4" />,
  },
  {
    key: 'numeric',
    getTitle: () => getLL().flow.dynamic_cat_numeric(),
    icon: <Binary className="w-4 h-4" />,
  },
  {
    key: 'network',
    getTitle: () => getLL().flow.dynamic_cat_network(),
    icon: <Globe className="w-4 h-4" />,
  },
  {
    key: 'identity',
    getTitle: () => getLL().flow.dynamic_cat_identity(),
    icon: <User className="w-4 h-4" />,
  },
  {
    key: 'contact',
    getTitle: () => getLL().flow.dynamic_cat_contact(),
    icon: <Mail className="w-4 h-4" />,
  },
  {
    key: 'text',
    getTitle: () => getLL().flow.dynamic_cat_text(),
    icon: <FileText className="w-4 h-4" />,
  },
  {
    key: 'location',
    getTitle: () => getLL().flow.dynamic_cat_location(),
    icon: <MapPin className="w-4 h-4" />,
  },
];

function getVariableDescription(name: string): string {
  const ll = getLL().flow;
  switch (name) {
    case '$guid':
      return ll.dynamic_var_guid();
    case '$uuid':
      return ll.dynamic_var_uuid();
    case '$randomUUID':
      return ll.dynamic_var_randomUUID();
    case '$timestamp':
      return ll.dynamic_var_timestamp();
    case '$timestampMs':
      return ll.dynamic_var_timestampMs();
    case '$isoTimestamp':
      return ll.dynamic_var_isoTimestamp();
    case '$randomDatePast':
      return ll.dynamic_var_randomDatePast();
    case '$randomDateFuture':
      return ll.dynamic_var_randomDateFuture();
    case '$randomDateRecent':
      return ll.dynamic_var_randomDateRecent();
    case '$randomInt':
      return ll.dynamic_var_randomInt();
    case '$randomPrice':
      return ll.dynamic_var_randomPrice();
    case '$randomBoolean':
      return ll.dynamic_var_randomBoolean();
    case '$randomAlphaNumeric':
      return ll.dynamic_var_randomAlphaNumeric();
    case '$randomString':
      return ll.dynamic_var_randomString();
    case '$randomHexColor':
      return ll.dynamic_var_randomHexColor();
    case '$randomColor':
      return ll.dynamic_var_randomColor();
    case '$randomSemver':
      return ll.dynamic_var_randomSemver();
    case '$randomIP':
      return ll.dynamic_var_randomIP();
    case '$randomIPV6':
      return ll.dynamic_var_randomIPV6();
    case '$randomMACAddress':
      return ll.dynamic_var_randomMACAddress();
    case '$randomPort':
      return ll.dynamic_var_randomPort();
    case '$randomUrl':
      return ll.dynamic_var_randomUrl();
    case '$randomDomainName':
      return ll.dynamic_var_randomDomainName();
    case '$randomFirstName':
      return ll.dynamic_var_randomFirstName();
    case '$randomLastName':
      return ll.dynamic_var_randomLastName();
    case '$randomFullName':
      return ll.dynamic_var_randomFullName();
    case '$randomUserName':
      return ll.dynamic_var_randomUserName();
    case '$randomJobTitle':
      return ll.dynamic_var_randomJobTitle();
    case '$randomEmail':
      return ll.dynamic_var_randomEmail();
    case '$randomExampleEmail':
      return ll.dynamic_var_randomExampleEmail();
    case '$randomPhoneNumber':
      return ll.dynamic_var_randomPhoneNumber();
    case '$randomPhone':
      return ll.dynamic_var_randomPhone();
    case '$randomWord':
      return ll.dynamic_var_randomWord();
    case '$randomWords':
      return ll.dynamic_var_randomWords();
    case '$randomCity':
      return ll.dynamic_var_randomCity();
    case '$randomCountry':
      return ll.dynamic_var_randomCountry();
    case '$randomCountryCode':
      return ll.dynamic_var_randomCountryCode();
    case '$randomLatitude':
      return ll.dynamic_var_randomLatitude();
    case '$randomLongitude':
      return ll.dynamic_var_randomLongitude();
    case '$randomCurrencyCode':
      return ll.dynamic_var_randomCurrencyCode();
    case '$randomCompanyName':
      return ll.dynamic_var_randomCompanyName();
    case '$randomStreetAddress':
      return ll.dynamic_var_randomStreetAddress();
    case '$randomPostalCode':
      return ll.dynamic_var_randomPostalCode();
    default:
      return '';
  }
}

function HelpSection({
  title,
  icon,
  defaultOpen = true,
  count,
  children,
}: {
  title: string;
  icon: ReactNode;
  defaultOpen?: boolean;
  count?: number;
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
        <span className="text-sm font-medium text-foreground flex-1">{title}</span>
        {count !== undefined && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground">
            {count}
          </span>
        )}
      </button>
      {open && <div className="px-4 py-3">{children}</div>}
    </div>
  );
}

export function DynamicVariablesHelpDialog({
  open,
  onOpenChange,
}: DynamicVariablesHelpDialogProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [copiedVar, setCopiedVar] = useState<string | null>(null);
  const [sampleValues, setSampleValues] = useState<Record<string, string>>({});

  const refreshAllSamples = () => {
    const next: Record<string, string> = {};
    for (const item of DYNAMIC_VARIABLES) {
      next[item.name] = item.generateSample();
    }
    setSampleValues(next);
  };

  useEffect(() => {
    if (open) {
      refreshAllSamples();
    }
  }, [open]);

  const copyVariable = (name: string) => {
    const textToCopy = `{{${name}}}`;
    navigator.clipboard.writeText(textToCopy).then(() => {
      setCopiedVar(name);
      toast.success(`${getLL().flow.dynamic_variables_copied()}: ${textToCopy}`);
      setTimeout(() => setCopiedVar(null), 2000);
    }).catch(() => {
      // 剪贴板失败静默降级
    });
  };

  const filteredVariables = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) return DYNAMIC_VARIABLES;

    return DYNAMIC_VARIABLES.filter((v) => {
      const desc = getVariableDescription(v.name).toLowerCase();
      const rawName = v.name.toLowerCase();
      const wrappedName = `{{${rawName}}}`;
      return (
        rawName.includes(query) ||
        wrappedName.includes(query) ||
        desc.includes(query) ||
        v.category.toLowerCase().includes(query)
      );
    });
  }, [searchQuery]);

  const variablesByCategory = useMemo(() => {
    const map: Record<DynamicVariableCategory, DynamicVariableItem[]> = {
      common: [],
      datetime: [],
      numeric: [],
      network: [],
      identity: [],
      contact: [],
      text: [],
      location: [],
    };

    for (const item of filteredVariables) {
      map[item.category].push(item);
    }
    return map;
  }, [filteredVariables]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-[720px] h-[85vh] flex flex-col overflow-hidden p-0 gap-0"
        hideCloseButton
      >
        {/* 头部区 */}
        <div className="shrink-0 px-6 pt-6 pb-4 border-b border-border space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-full bg-emerald-100 dark:bg-emerald-950/50 flex items-center justify-center shrink-0">
                <Sparkles className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
              </div>
              <div>
                <DialogTitle className="text-lg font-semibold">
                  {getLL().flow.dynamic_variables_help_title()}
                </DialogTitle>
                <p className="text-xs text-muted-foreground mt-0.5">
                  {getLL().flow.dynamic_variables_help_subtitle()}
                </p>
              </div>
            </div>

            <Button
              variant="outline"
              size="sm"
              onClick={refreshAllSamples}
              className="gap-1.5 text-xs h-8 text-muted-foreground hover:text-foreground"
            >
              <RefreshCw className="w-3.5 h-3.5" />
              {getLL().flow.dynamic_variables_refresh_samples()}
            </Button>
          </div>

          {/* 搜索栏 */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <Input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={getLL().flow.dynamic_variables_search_placeholder()}
              className="pl-9 pr-8 h-9 text-sm"
            />
            {searchQuery && (
              <button
                type="button"
                onClick={() => setSearchQuery('')}
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground p-0.5 rounded-sm"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>

        {/* 可滚动内容区 */}
        <div className="flex-1 min-h-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="px-6 py-4 space-y-3">
              {CATEGORY_CONFIGS.map((cat) => {
                const items = variablesByCategory[cat.key];
                if (items.length === 0) return null;

                return (
                  <HelpSection
                    key={cat.key}
                    title={cat.getTitle()}
                    icon={cat.icon}
                    count={items.length}
                    defaultOpen={true}
                  >
                    <table className="w-full text-xs">
                      <thead>
                        <tr className="bg-muted/40 text-muted-foreground">
                          <th className="px-3 py-2 text-left font-medium w-[28%]">
                            {getLL().flow.dynamic_variables_col_name()}
                          </th>
                          <th className="px-3 py-2 text-left font-medium w-[38%]">
                            {getLL().flow.dynamic_variables_col_desc()}
                          </th>
                          <th className="px-3 py-2 text-left font-medium w-[26%]">
                            {getLL().flow.dynamic_variables_col_sample()}
                          </th>
                          <th className="px-2 py-2 text-right font-medium w-[8%]" />
                        </tr>
                      </thead>
                      <tbody>
                        {items.map((item) => {
                          const isCopied = copiedVar === item.name;
                          const currentSample = sampleValues[item.name] || item.example;

                          return (
                            <tr
                              key={item.name}
                              className="hover:bg-muted/30 transition-colors group"
                            >
                              <td className="px-3 py-2 border-t border-border align-middle">
                                <button
                                  type="button"
                                  onClick={() => copyVariable(item.name)}
                                  className="inline-flex items-center text-left"
                                  title={getLL().flow.dynamic_variables_copy_hint()}
                                >
                                  <code className="text-[11px] font-mono font-medium text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 px-1.5 py-0.5 rounded hover:bg-emerald-100 dark:hover:bg-emerald-900/60 transition-colors">
                                    {`{{${item.name}}}`}
                                  </code>
                                </button>
                              </td>
                              <td className="px-3 py-2 border-t border-border text-foreground/90 align-middle">
                                {getVariableDescription(item.name)}
                              </td>
                              <td className="px-3 py-2 border-t border-border font-mono text-[11px] text-muted-foreground truncate max-w-[170px] align-middle" title={currentSample}>
                                {currentSample}
                              </td>
                              <td className="px-2 py-2 border-t border-border text-right align-middle">
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  onClick={() => copyVariable(item.name)}
                                  className="h-7 w-7 text-muted-foreground hover:text-foreground opacity-60 group-hover:opacity-100 transition-opacity"
                                  title={getLL().flow.dynamic_variables_copy_hint()}
                                >
                                  {isCopied ? (
                                    <Check className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />
                                  ) : (
                                    <Copy className="h-3.5 w-3.5" />
                                  )}
                                </Button>
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </HelpSection>
                );
              })}

              {filteredVariables.length === 0 && (
                <div className="py-12 text-center text-sm text-muted-foreground space-y-1">
                  <p>{getLL().flow.no_matching_nodes()}</p>
                </div>
              )}
            </div>
          </ScrollArea>
        </div>

        {/* 底部区 */}
        <div className="shrink-0 px-6 py-3.5 border-t border-border flex items-center justify-between bg-muted/10">
          <p className="text-xs text-muted-foreground">
            {getLL().flow.dynamic_variables_footer_note()}
          </p>
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {getLL().flow.script_help_close()}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
