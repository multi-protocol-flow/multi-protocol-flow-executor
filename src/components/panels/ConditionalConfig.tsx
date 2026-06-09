/**
 * Conditional 节点配置组件
 * 支持简单条件和高级表达式两种模式
 */
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useCallback, useEffect, useMemo, useRef } from "react";
import {
  conditionalConfigSchema,
  type ConditionalConfigFormData,
  OPERATOR_OPTIONS,
} from "./schemas";
import { Label } from "@/components/ui/label";
import { VariableInput } from "@/components/ui/VariableInput";
import { VariableValidationWarning } from "@/components/ui/VariableValidationWarning";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { SearchableNodeSelect } from "@/components/ui/SearchableNodeSelect";
import { Textarea } from "@/components/ui/textarea";
import { Code2, Split } from "lucide-react";
import type { ConditionalConfig as ConditionalConfigType, ConditionOperator } from "@/types";
import { useFlowVariables } from "@/hooks";
import { useFlowStore } from "@/stores";
import { getLL } from "@/i18n/i18n-lazy";

/** 不应出现在数据来源下拉中的节点类型（无输出数据或无意义） */
const EXCLUDED_SOURCE_TYPES: Set<string> = new Set([
  "entry",
  "conditional",
  "variable_extractor",
]);

interface ConditionalConfigProps {
  /** 当前节点数据 */
  data: ConditionalConfigType;
  /** 保存回调 */
  onSave: (data: ConditionalConfigType) => void;
}

/**
 * 将 ConditionalConfigType 转换为表单数据
 */
function toFormData(data: ConditionalConfigType): ConditionalConfigFormData {
  let expectedValue = "";
  if (data.expected_value !== undefined && data.expected_value !== null) {
    try {
      expectedValue = typeof data.expected_value === "string"
        ? data.expected_value
        : JSON.stringify(data.expected_value);
    } catch {
      expectedValue = String(data.expected_value);
    }
  }

  return {
    // 优先使用 condition_type 字段，不再根据 advanced_expression 推断
    // 这样可以支持两种模式的数据同时存在
    condition_type: data.condition_type || "simple",
    field_path: data.field_path || "",
    operator: (data.operator as ConditionalConfigFormData["operator"]) || "equal",
    expected_value: expectedValue || undefined,
    advanced_expression: data.advanced_expression || "",
    expected_value_type: data.expected_value_type,
    source_node_id: data.source_node_id || undefined,
  };
}

/**
 * 将表单数据转换为 ConditionalConfigType
 * 注意：保留两种模式的数据，切换模式时不会丢失已输入的内容
 */
function fromFormData(formData: ConditionalConfigFormData): ConditionalConfigType {
  let expectedValue: unknown = undefined;
  if (formData.expected_value?.trim()) {
    try {
      expectedValue = JSON.parse(formData.expected_value);
    } catch {
      // 如果不是有效的 JSON，保留为字符串
      expectedValue = formData.expected_value;
    }
  }

  return {
    type: "conditional",
    condition_type: formData.condition_type,
    // 保留简单条件数据，即使当前是高级模式
    field_path: formData.field_path || undefined,
    operator: (formData.operator as ConditionOperator) || undefined,
    expected_value: expectedValue,
    // 保留高级表达式数据，即使当前是简单模式
    advanced_expression: formData.advanced_expression || undefined,
    expected_value_type: formData.expected_value_type || undefined,
    source_node_id: formData.source_node_id || undefined,
  };
}

export function ConditionalConfig({ data, onSave }: ConditionalConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    getValues,
    reset,
    formState: { isDirty },
  } = useForm<ConditionalConfigFormData>({
    resolver: zodResolver(conditionalConfigSchema),
    defaultValues: toFormData(data),
    mode: "onBlur",
  });

  const conditionType = watch("condition_type");
  const watchedOperator = watch("operator");

  // 获取流程变量
  const { variables, availableVariableNames, nodeLabels } = useFlowVariables();

  // 获取当前流程节点列表和当前选中节点（用于数据来源选择）
  const flow = useFlowStore((state) => state.flow);
  const selectedNodeId = useFlowStore((state) => state.selectedNodeId);

  // 可选的数据来源节点（排除自身和无输出节点类型）
  const sourceNodes = flow?.nodes.filter(
    (node) => node.uuid !== selectedNodeId && !EXCLUDED_SOURCE_TYPES.has(node.type)
  ) ?? [];

  // 数据来源下拉选项：自动 + 所有上游节点
  const sourceSelectItems = useMemo(() => {
    const _ll = getLL().panels.conditional;
    return [
      { uuid: "", name: _ll.sourceAuto(), type: "" },
      ...sourceNodes.map((node) => ({
        uuid: node.uuid,
        name: node.name,
        type: node.type,
      })),
    ];
  }, [sourceNodes]);

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
        console.error("保存条件配置失败:", error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
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
   * 切换条件类型
   */
  const handleConditionTypeChange = useCallback((value: string) => {
    setValue("condition_type", value as "simple" | "advanced", { shouldDirty: true });
    saveData();
  }, [setValue, saveData]);

  /**
   * 处理数据来源选择变更
   */
  const handleSourceNodeChange = useCallback((value: string) => {
    setValue("source_node_id", value || undefined, { shouldDirty: true });
    setTimeout(saveData, 100);
  }, [setValue, saveData]);

  const ll = getLL().panels.conditional;

  return (
    <form onBlur={handleBlur} className="w-full space-y-5">
      {/* 数据来源选择 */}
      <div className="space-y-2">
        <Label htmlFor="source_node_id" className="text-sm font-medium text-foreground">
          {ll.sourceNode()}
        </Label>
        <SearchableNodeSelect
          value={watch("source_node_id") || ""}
          onValueChange={handleSourceNodeChange}
          items={sourceSelectItems}
          placeholder={ll.sourceAuto()}
          noResultsText={ll.sourceHelp()}
          searchPlaceholder={getLL().panels.nodeSearchPlaceholder()}
        />
        <p className="text-xs text-muted-foreground">
          {ll.sourceHelp()}
        </p>
      </div>

      {/* 条件类型选择 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{ll.conditionType()}</Label>
        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            onClick={() => handleConditionTypeChange("simple")}
            className={`flex items-center justify-center gap-2 rounded-lg border-2 p-3 transition-all ${
              conditionType === "simple"
                ? "border-sky-500 bg-sky-50 text-sky-700"
                : "border-border hover:bg-accent"
            }`}
>
            <Split className="h-4 w-4" />
            <span className="text-sm font-medium">{ll.simple()}</span>
          </button>
          <button
            type="button"
            onClick={() => handleConditionTypeChange("advanced")}
            className={`flex items-center justify-center gap-2 rounded-lg border-2 p-3 transition-all ${
              conditionType === "advanced"
                ? "border-violet-500 bg-violet-50 text-violet-700"
                : "border-border hover:bg-accent"
            }`}
>
            <Code2 className="h-4 w-4" />
            <span className="text-sm font-medium">{ll.advanced()}</span>
          </button>
        </div>
      </div>

      {/* 简单条件配置 */}
      {conditionType === "simple" && (
        <>
          {/* 字段路径 */}
          <div className="space-y-2">
            <Label htmlFor="field_path" className="text-sm font-medium text-foreground">
              {ll.fieldPath()}
            </Label>
            <VariableInput
              id="field_path"
              variables={variables}
              nodeLabels={nodeLabels}
              value={watch("field_path") || ""}
              onValueChange={(value) => setValue("field_path", value, { shouldDirty: true })}
              placeholder={ll.fieldPathPlaceholder("")}
              className="font-mono text-sm"
            />
            <div className="text-xs text-muted-foreground space-y-1">
              <p>{ll.fieldPathHelpTitle()}</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li><code className="rounded bg-muted px-1">{"{{变量名}}"}</code>{" "}{ll.fieldPathFormatVariable()}</li>
                <li><code className="rounded bg-muted px-1">$.json.path</code>{" "}{ll.fieldPathFormatJsonPath()}</li>
              </ul>
            </div>
            <VariableValidationWarning
              value={watch("field_path") || ""}
              availableVariables={availableVariableNames}
              compact
            />
          </div>

          {/* 操作符 */}
          <div className="space-y-2">
            <Label htmlFor="operator" className="text-sm font-medium text-foreground">
              {ll.operator()}
            </Label>
            <Select
              value={watchedOperator}
              onValueChange={(value) => {
                setValue("operator", value as ConditionalConfigFormData["operator"], { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger id="operator">
                <SelectValue placeholder={ll.operatorPlaceholder()} />
              </SelectTrigger>
              <SelectContent>
                {OPERATOR_OPTIONS.map((op) => (
                  <SelectItem key={op.value} value={op.value}>
                    {op.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* 值类型选择器 - 仅当操作符为 equal 或 not_equal 时显示 */}
          {(watchedOperator === "equal" || watchedOperator === "not_equal") && (
            <div className="space-y-2">
              <Label htmlFor="expected_value_type" className="text-sm font-medium text-foreground">
                {ll.valueType()}
              </Label>
              <Select
                value={watch("expected_value_type") || "string"}
                onValueChange={(value) => {
                  setValue("expected_value_type", value as "number" | "string" | "boolean" | "null", { shouldDirty: true });
                  saveData();
                }}
              >
                <SelectTrigger id="expected_value_type">
                  <SelectValue placeholder={ll.valueTypePlaceholder()} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="string">{ll.valueTypeString()}</SelectItem>
                  <SelectItem value="number">{ll.valueTypeNumber()}</SelectItem>
                  <SelectItem value="boolean">{ll.valueTypeBoolean()}</SelectItem>
                  <SelectItem value="null">{ll.valueTypeNull()}</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {ll.valueTypeHelp()}
              </p>
            </div>
          )}

          {/* 期望值 */}
          <div className="space-y-2">
            <Label htmlFor="expected_value" className="text-sm font-medium text-foreground">
              {ll.expectedValue()}
            </Label>
            <VariableInput
              id="expected_value"
              variables={variables}
              nodeLabels={nodeLabels}
              value={watch("expected_value") || ""}
              onValueChange={(value) => setValue("expected_value", value, { shouldDirty: true })}
              placeholder={ll.expectedValuePlaceholder()}
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground break-all">
              {ll.expectedValueHelp()}
            </p>
            <VariableValidationWarning
              value={watch("expected_value") || ""}
              availableVariables={availableVariableNames}
              compact
            />
          </div>
        </>
      )}

      {/* 高级表达式配置 */}
      {conditionType === "advanced" && (
        <div className="space-y-2">
          <Label htmlFor="advanced_expression" className="text-sm font-medium text-foreground">
            {ll.advancedExpression()}
          </Label>
          <Textarea
            id="advanced_expression"
            {...register("advanced_expression")}
            placeholder={ll.advancedPlaceholder()}
            className="min-h-[120px] resize-y font-mono text-sm"
            rows={4}
          />
          <div className="rounded-md bg-muted/50 p-3 text-xs text-muted-foreground space-y-2">
            <div>
              <p className="mb-1 font-medium text-foreground">{ll.operatorsTitle()}</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li>{ll.comparisonOps()}{" "}<code className="rounded bg-muted px-1">{">"} {">="} {"<"} {"<="} {"=="} {"!="}</code></li>
                <li>{ll.logicalOps()}{" "}<code className="rounded bg-muted px-1">{"&&"} {"||"} {"!"}</code></li>
              </ul>
            </div>
            <div>
              <p className="mb-1 font-medium text-foreground">{ll.varRefTitle()}</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li>{ll.varRefDirect("")}</li>
                <li>{ll.varRefNested()}{" "}<code className="rounded bg-muted px-1">body.data.token == "abc"</code></li>
                <li>{ll.varRefComposite()}{" "}<code className="rounded bg-muted px-1">status == 200 && code == 0</code></li>
              </ul>
            </div>
            <div>
              <p className="mb-1 font-medium text-sky-600 dark:text-sky-400">{ll.dataSourceTitle()}</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li>{ll.dataSourceUpstream()}</li>
                <li>{ll.dataSourceVariable()}</li>
              </ul>
            </div>
            <div className="border-t border-border pt-2 mt-2">
              <p className="mb-1 font-medium text-amber-600">{ll.notesTitle()}</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li>{ll.noteStringQuote()}</li>
                <li>{ll.noteNestedField()}（自动展开，等同于 <code className="rounded bg-muted px-1">$.body.data.token</code>）</li>
              </ul>
            </div>
          </div>
          <VariableValidationWarning
            value={watch("advanced_expression") || ""}
            availableVariables={availableVariableNames}
          />
        </div>
      )}
    </form>
  );
}
