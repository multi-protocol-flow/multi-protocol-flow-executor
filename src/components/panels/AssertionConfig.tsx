/**
 * Assertion 节点配置组件
 * 支持断言列表管理、预设选择、全局配置
 */
import { useForm, useFieldArray } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useCallback, useEffect, useRef } from "react";
import {
  assertionConfigSchema,
  type AssertionConfigFormData,
  type AssertionFormData,
} from "./schemas";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Plus, Trash2, CheckCircle, XCircle, Zap } from "lucide-react";
import type { AssertionNodeConfig, Assertion, AssertionOperator } from "@/types";
import { useFlowStore } from "@/stores";
import { getLL } from "@/i18n/i18n-lazy";

/**
 * 断言操作符选项 (15种) — 已国际化
 */
const ASSERTION_OPERATOR_OPTIONS: { value: AssertionFormData["operator"]; label: string }[] = [
  { value: "equal", label: getLL().panels.assertion.operators.equal() },
  { value: "not_equal", label: getLL().panels.assertion.operators.not_equal() },
  { value: "greater", label: getLL().panels.assertion.operators.greater() },
  { value: "greater_or_equal", label: getLL().panels.assertion.operators.greater_or_equal() },
  { value: "less", label: getLL().panels.assertion.operators.less() },
  { value: "less_or_equal", label: getLL().panels.assertion.operators.less_or_equal() },
  { value: "contains", label: getLL().panels.assertion.operators.contains() },
  { value: "not_contains", label: getLL().panels.assertion.operators.not_contains() },
  { value: "starts_with", label: getLL().panels.assertion.operators.starts_with() },
  { value: "ends_with", label: getLL().panels.assertion.operators.ends_with() },
  { value: "exists", label: getLL().panels.assertion.operators.exists() },
  { value: "not_exists", label: getLL().panels.assertion.operators.not_exists() },
  { value: "is_empty", label: getLL().panels.assertion.operators.is_empty() },
  { value: "is_not_empty", label: getLL().panels.assertion.operators.is_not_empty() },
  { value: "matches_regex", label: getLL().panels.assertion.operators.matches_regex() },
];

/**
 * 预设配置 — 已国际化
 */
const ASSERTION_PRESETS = [
  { value: "none", label: getLL().panels.assertion.presets.none() },
  { value: "http_success", label: getLL().panels.assertion.presets.http_success() },
  { value: "http_client_error", label: getLL().panels.assertion.presets.http_client_error() },
  { value: "http_server_error", label: getLL().panels.assertion.presets.http_server_error() },
  { value: "json_response", label: getLL().panels.assertion.presets.json_response() },
  { value: "response_fast", label: getLL().panels.assertion.presets.response_fast() },
];

/**
 * 预设断言数据
 */
const PRESET_ASSERTIONS: Record<string, AssertionFormData[]> = {
  http_success: [
    { target: "$.status", operator: "greater_or_equal", expected: "200", enabled: true },
    { target: "$.status", operator: "less", expected: "300", enabled: true },
  ],
  http_client_error: [
    { target: "$.status", operator: "greater_or_equal", expected: "400", enabled: true },
    { target: "$.status", operator: "less", expected: "500", enabled: true },
  ],
  http_server_error: [
    { target: "$.status", operator: "greater_or_equal", expected: "500", enabled: true },
    { target: "$.status", operator: "less", expected: "600", enabled: true },
  ],
  json_response: [
    { target: "$.headers.content-type", operator: "contains", expected: "application/json", enabled: true },
    { target: "$.body", operator: "is_not_empty", expected: "", enabled: true },
  ],
  response_fast: [
    { target: "$.duration_ms", operator: "less", expected: "1000", enabled: true },
  ],
};

/**
 * 不应出现在数据来源下拉中的节点类型（无输出数据或无意义）
 */
const EXCLUDED_SOURCE_TYPES: Set<string> = new Set([
  "entry",
  "assertion",
]);

/**
 * 生成唯一 ID
 */
function generateId(): string {
  return `assertion_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}

/**
 * 将 AssertionNodeConfig 转换为表单数据
 */
function toFormData(data: AssertionNodeConfig): AssertionConfigFormData {
  const assertions: AssertionFormData[] = data.assertions.map((a) => ({
    target: a.target,
    operator: mapOperatorToForm(a.operator),
    expected: typeof a.expected === "string" ? a.expected : JSON.stringify(a.expected),
    enabled: a.enabled ?? true,
    description: a.description,
  }));

  return {
    type: "assertion",
    preset: data.preset || "none",
    mode: data.mode,
    on_failure: data.on_failure,
    continue_on_failure: data.continue_on_failure,
    assertions,
    source_node_id: data.source_node_id || undefined,
  };
}

/**
 * 将表单数据转换为 AssertionNodeConfig
 */
function fromFormData(formData: AssertionConfigFormData): AssertionNodeConfig {
  const assertions: Assertion[] = formData.assertions.map((a) => {
    let expected: unknown = a.expected;
    // 尝试解析为 JSON，失败则保留字符串
    if (a.expected.trim()) {
      try {
        expected = JSON.parse(a.expected);
      } catch {
        expected = a.expected;
      }
    }

    return {
      id: generateId(),
      target: a.target,
      operator: mapOperatorFromForm(a.operator),
      expected,
      enabled: a.enabled,
      description: a.description,
    };
  });

  return {
    type: "assertion",
    // 预设仅用于前端 UI 填充断言模板，不传给后端。
    // 后端 merge_preset_and_custom 会根据 preset 字段再追加预设断言，
    // 导致与已展平到 assertions 中的预设断言重复计算。
    preset: undefined,
    mode: formData.mode,
    on_failure: formData.on_failure,
    continue_on_failure: formData.continue_on_failure,
    assertions,
    source_node_id: formData.source_node_id || undefined,
  };
}

/**
 * 映射操作符：表单 -> 类型定义
 * schemas.ts 使用的操作符名称与 node.ts 不同，需要映射
 */
function mapOperatorFromForm(op: AssertionFormData["operator"]): AssertionOperator {
  const mapping: Record<AssertionFormData["operator"], AssertionOperator> = {
    equal: "eq",
    not_equal: "ne",
    greater: "gt",
    greater_or_equal: "gte",
    less: "lt",
    less_or_equal: "lte",
    contains: "contains",
    not_contains: "not_contains",
    starts_with: "contains", // 映射到最接近的
    ends_with: "contains", // 映射到最接近的
    exists: "exists",
    not_exists: "not_exists",
    is_empty: "eq", // 需要配合期望值使用
    is_not_empty: "ne", // 需要配合期望值使用
    matches_regex: "matches",
  };
  return mapping[op] || "eq";
}

/**
 * 映射操作符：类型定义 -> 表单
 */
function mapOperatorToForm(op: AssertionOperator): AssertionFormData["operator"] {
  const mapping: Record<AssertionOperator, AssertionFormData["operator"]> = {
    eq: "equal",
    ne: "not_equal",
    gt: "greater",
    gte: "greater_or_equal",
    lt: "less",
    lte: "less_or_equal",
    contains: "contains",
    not_contains: "not_contains",
    matches: "matches_regex",
    exists: "exists",
    not_exists: "not_exists",
    in: "contains", // 映射到最接近的
    not_in: "not_contains", // 映射到最接近的
    type_is: "equal", // 需要配合期望值使用
    between: "greater_or_equal", // 需要配合期望值使用
  };
  return mapping[op] || "equal";
}

interface AssertionConfigProps {
  data: AssertionNodeConfig;
  onSave: (data: AssertionNodeConfig) => void;
}

export function AssertionConfig({ data, onSave }: AssertionConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    getValues,
    reset,
    formState: { errors, isDirty },
  } = useForm<AssertionConfigFormData>({
    resolver: zodResolver(assertionConfigSchema) as never,
    defaultValues: toFormData(data),
    mode: "onBlur",
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: "assertions" as never,
  });

  const watchedPreset = watch("preset");
  const watchedMode = watch("mode");
  const watchedOnFailure = watch("on_failure");
  const watchedContinueOnFailure = watch("continue_on_failure");
  const watchedSourceNodeId = watch("source_node_id");

  // 获取当前流程节点列表和当前选中节点（用于数据来源选择）
  const flow = useFlowStore((state) => state.flow);
  const selectedNodeId = useFlowStore((state) => state.selectedNodeId);

  // 可选的数据来源节点（排除自身和无输出节点类型）
  const sourceNodes = flow?.nodes.filter(
    (node) => node.uuid !== selectedNodeId && !EXCLUDED_SOURCE_TYPES.has(node.type)
  ) ?? [];

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 追踪表单实际修改状态（useRef 用于卸载时读取）
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  /**
   * 安全的延迟执行包装器，注册定时器以便卸载时清理
   */
  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  // 组件卸载时同步保存（仅当用户实际修改过表单时）
  useEffect(() => {
    return () => {
      if (!isActuallyDirtyRef.current) return;
      try {
        const formData = getValues();
        const config = fromFormData(formData);
        onSave(config);
      } catch {
        // 静默处理卸载时的保存错误
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error("保存断言配置失败:", error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理数据来源选择变更
   */
  const handleSourceNodeChange = useCallback((value: string) => {
    setValue("source_node_id", value || undefined, { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理失焦事件 (100ms debounce)
   */
  const handleBlur = useCallback(() => {
        safeSetTimeout(() => {
          if (!isActuallyDirtyRef.current) return;
          saveData();
        }, 100);
      }, [safeSetTimeout, saveData]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener("flush-config-panel", handleFlush);
    return () => { window.removeEventListener("flush-config-panel", handleFlush); };
  }, [saveData]);

  /**
   * 添加断言
   */
  const handleAddAssertion = useCallback(() => {
    append({
      target: "",
      operator: "equal",
      expected: "",
      enabled: true,
    } as AssertionFormData);
  }, [append]);

  /**
   * 删除断言
   */
  const handleRemoveAssertion = useCallback((index: number) => {
    remove(index);
    setTimeout(saveData, 100);
  }, [remove, saveData]);

  /**
   * 处理预设选择
   */
  const handlePresetChange = useCallback((value: string) => {
    setValue("preset", value, { shouldDirty: true });
    if (value !== "none" && PRESET_ASSERTIONS[value]) {
      // 使用预设断言替换当前断言
      setValue("assertions", [...PRESET_ASSERTIONS[value]], { shouldDirty: true });
    }
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理组合模式变更
   */
  const handleModeChange = useCallback((value: string) => {
    setValue("mode", value as "all" | "any", { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理失败行为变更
   */
  const handleOnFailureChange = useCallback((value: string) => {
    setValue("on_failure", value as "route_false" | "throw", { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理失败后继续执行变更
   */
  const handleContinueOnFailureChange = useCallback((checked: boolean) => {
    setValue("continue_on_failure", checked, { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理断言启用/禁用切换
   */
  const handleEnabledChange = useCallback((index: number, checked: boolean) => {
    setValue(`assertions.${index}.enabled`, checked, { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理断言操作符变更
   */
  const handleOperatorChange = useCallback((index: number, value: string) => {
    setValue(`assertions.${index}.operator`, value as AssertionFormData["operator"], { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 检查操作符是否需要期望值
   */
  const operatorNeedsExpected = useCallback((op: AssertionFormData["operator"]) => {
    return !["exists", "not_exists", "is_empty", "is_not_empty"].includes(op);
  }, []);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 数据来源选择 */}
      <div className="space-y-2">
        <Label htmlFor="source_node_id" className="text-sm font-medium text-foreground">
          {getLL().panels.assertion.sourceNode()}
        </Label>
        <Select
          value={watchedSourceNodeId || ""}
          onValueChange={handleSourceNodeChange}
        >
          <SelectTrigger id="source_node_id" className="font-mono">
            <SelectValue placeholder={getLL().panels.assertion.sourcePlaceholder()} />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">
              {getLL().panels.assertion.sourceAuto()}
            </SelectItem>
            {sourceNodes.map((node) => (
              <SelectItem key={node.uuid} value={node.uuid}>
                {node.name} ({node.type})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {getLL().panels.assertion.sourceHelp()}
        </p>
      </div>

      {/* 预设选择 */}
      <div className="space-y-2">
        <Label htmlFor="preset" className="text-sm font-medium text-foreground flex items-center gap-2">
          <Zap className="h-4 w-4 text-amber-500" />
          {getLL().panels.assertion.preset()}
        </Label>
        <Select
          value={watchedPreset || "none"}
          onValueChange={handlePresetChange}
        >
          <SelectTrigger id="preset" className="font-mono">
            <SelectValue placeholder={getLL().panels.assertion.presetPlaceholder()} />
          </SelectTrigger>
          <SelectContent>
            {ASSERTION_PRESETS.map((preset) => (
              <SelectItem key={preset.value} value={preset.value}>
                {preset.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {getLL().panels.assertion.presetHelp()}
        </p>
      </div>

      {/* 全局配置区域 */}
      <div className="space-y-4 rounded-lg border bg-muted/20 p-4">
        <h3 className="text-sm font-medium text-foreground">{getLL().panels.assertion.globalConfig()}</h3>
        
        {/* 组合模式 */}
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">{getLL().panels.assertion.modeLabel()}</Label>
          <div className="grid grid-cols-2 gap-2">
            <button
              type="button"
              onClick={() => handleModeChange("all")}
              className={`flex items-center justify-center gap-2 rounded-lg border-2 p-2.5 transition-all text-sm ${
                watchedMode === "all"
                  ? "border-emerald-500 bg-emerald-50 dark:bg-emerald-950/50 text-emerald-700 dark:text-emerald-300"
                  : "border-border hover:bg-muted/50"
              }`}
            >
              <CheckCircle className="h-4 w-4" />
              {getLL().panels.assertion.modeAll()}
            </button>
            <button
              type="button"
              onClick={() => handleModeChange("any")}
              className={`flex items-center justify-center gap-2 rounded-lg border-2 p-2.5 transition-all text-sm ${
                watchedMode === "any"
                  ? "border-sky-500 bg-sky-50 dark:bg-sky-950/50 text-sky-700 dark:text-sky-300"
                  : "border-border hover:bg-muted/50"
              }`}
            >
              {getLL().panels.assertion.modeAny()}
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            {watchedMode === "all" 
              ? getLL().panels.assertion.modeAllDesc()
              : getLL().panels.assertion.modeAnyDesc()}
          </p>
        </div>

        {/* 失败行为 */}
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">{getLL().panels.assertion.failureActionLabel()}</Label>
          <div className="grid grid-cols-2 gap-2">
            <button
              type="button"
              onClick={() => handleOnFailureChange("route_false")}
              className={`flex items-center justify-center gap-2 rounded-lg border-2 p-2.5 transition-all text-sm ${
                watchedOnFailure === "route_false"
                  ? "border-amber-500 bg-amber-50 dark:bg-amber-950/50 text-amber-700 dark:text-amber-300"
                  : "border-border hover:bg-muted/50"
              }`}
            >
              <CheckCircle className="h-4 w-4" />
              {getLL().panels.assertion.routeFalse()}
            </button>
            <button
              type="button"
              onClick={() => handleOnFailureChange("throw")}
              className={`flex items-center justify-center gap-2 rounded-lg border-2 p-2.5 transition-all text-sm ${
                watchedOnFailure === "throw"
                  ? "border-rose-500 bg-rose-50 dark:bg-rose-950/50 text-rose-700 dark:text-rose-300"
                  : "border-border hover:bg-muted/50"
              }`}
            >
              <XCircle className="h-4 w-4" />
              {getLL().panels.assertion.throwError()}
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            {watchedOnFailure === "route_false"
              ? getLL().panels.assertion.routeFalseDesc()
              : getLL().panels.assertion.throwErrorDesc()}
          </p>
        </div>

        {/* 失败后继续执行 */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label className="text-sm text-muted-foreground">{getLL().panels.assertion.continueOnFailure()}</Label>
            <p className="text-xs text-muted-foreground">
              {getLL().panels.assertion.continueOnFailureDesc()}
            </p>
          </div>
          <Switch
            checked={watchedContinueOnFailure}
            onCheckedChange={handleContinueOnFailureChange}
          />
        </div>
      </div>

      {/* 断言列表区域 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.assertion.listLabel()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddAssertion}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            {getLL().panels.assertion.addAssertion()}
          </Button>
        </div>

        <div className="space-y-2">
          {fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.assertion.empty()}</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                {getLL().panels.assertion.emptyHint()}
              </p>
            </div>
          ) : (
            fields.map((field, index) => {
              const operator = watch(`assertions.${index}.operator`) as AssertionFormData["operator"];
              const targetError = errors.assertions?.[index]?.target;
              const expectedError = errors.assertions?.[index]?.expected;
              const isEnabled = watch(`assertions.${index}.enabled`) ?? true;

              return (
                <div
                  key={field.id}
                  className={`space-y-2 p-3 rounded-md border transition-all ${
                    isEnabled ? "bg-muted/30" : "bg-muted/10 opacity-60"
                  }`}
                >
                  {/* 第一行：启用开关 + 目标字段 + 删除按钮 */}
                  <div className="flex items-center gap-2">
                    <Switch
                      checked={isEnabled}
                      onCheckedChange={(checked) => handleEnabledChange(index, checked)}
                      className="shrink-0"
                    />
                    <Input
                      {...register(`assertions.${index}.target`)}
                      placeholder={getLL().panels.assertion.targetPlaceholder()}
                      className={`h-8 flex-1 font-mono text-xs ${targetError ? "border-rose-500" : ""} ${!isEnabled ? "bg-muted/50" : ""}`}
                      disabled={!isEnabled}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveAssertion(index)}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>

                  {/* 第二行：操作符 + 期望值 */}
                  <div className="flex items-center gap-2">
                    <Select
                      value={operator}
                      onValueChange={(value) => handleOperatorChange(index, value)}
                      disabled={!isEnabled}
                    >
                      <SelectTrigger className={`h-8 w-40 font-mono text-xs ${!isEnabled ? "bg-muted/50" : ""}`}>
                        <SelectValue placeholder={getLL().panels.assertion.operatorPlaceholder()} />
                      </SelectTrigger>
                      <SelectContent>
                        {ASSERTION_OPERATOR_OPTIONS.map((op) => (
                          <SelectItem key={op.value} value={op.value}>
                            {op.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>

                    {operatorNeedsExpected(operator) && (
                      <Input
                        {...register(`assertions.${index}.expected`)}
                        placeholder={getLL().panels.assertion.expectedPlaceholder()}
                        className={`h-8 flex-1 font-mono text-xs ${expectedError ? "border-rose-500" : ""} ${!isEnabled ? "bg-muted/50" : ""}`}
                        disabled={!isEnabled}
                      />
                    )}
                  </div>

                  {/* 错误提示 */}
                  {targetError && (
                    <p className="text-xs text-rose-500">{targetError.message}</p>
                  )}
                  {expectedError && operatorNeedsExpected(operator) && (
                    <p className="text-xs text-rose-500">{expectedError.message}</p>
                  )}

                  {/* 第三行：说明（可选） */}
                  <Input
                    {...register(`assertions.${index}.description`)}
                    placeholder={getLL().panels.entryNode.variableDescPlaceholder()}
                    className={`h-8 w-full text-xs ${!isEnabled ? "bg-muted/50" : ""}`}
                    disabled={!isEnabled}
                  />
                </div>
              );
            })
          )}
        </div>

        {/* 提示信息 */}
        {fields.length > 0 && (
          <div className="rounded-md bg-muted/30 p-3 text-xs text-muted-foreground space-y-2">
              <p className="font-medium text-foreground">{getLL().panels.assertion.jsonpathTitle()}</p>
            <ul className="ml-3 list-disc space-y-0.5">
              <li><code className="rounded bg-muted px-1">$.status</code> {getLL().panels.assertion.jsonpathStatusCode()}</li>
              <li><code className="rounded bg-muted px-1">$.body.data.token</code> {getLL().panels.assertion.jsonpathNestedField()}</li>
              <li><code className="rounded bg-muted px-1">$.body.items[0]</code> {getLL().panels.assertion.jsonpathArrayIndex()}</li>
              <li><code className="rounded bg-muted px-1">$.headers.content-type</code> {getLL().panels.assertion.jsonpathResponseHeader()}</li>
              <li><code className="rounded bg-muted px-1">$.duration_ms</code> {getLL().panels.assertion.jsonpathResponseTime()}</li>
            </ul>
          </div>
        )}
      </div>
    </form>
  );
}
