/**
 * 共享输出映射编辑器组件
 * 从 HttpConfig.tsx 提取的通用 output_mapping 编辑 UI
 */
import { useFieldArray, type Control, type FieldErrors, type UseFormWatch, type UseFormRegister } from 'react-hook-form';
import { Plus, Trash2, ArrowRightFromLine, Info } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { getLL } from '@/i18n/i18n-lazy';

export interface OutputMappingsEditorProps {
  /** react-hook-form control */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  control: Control<any>;
  /** react-hook-form errors */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  errors: FieldErrors<any>;
  /** react-hook-form watch */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  watch: UseFormWatch<any>;
  /** react-hook-form register */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  register: UseFormRegister<any>;
  /** 保存数据回调（字段变更时触发） */
  saveData: () => void;
  /** JSONPath 预设列表 */
  jsonPathPresets: Array<{ value: string; label: string }>;
  /** 添加新映射时的默认 source 值 */
  defaultSource?: string;
}

/**
 * 输出映射编辑器
 *
 * 允许用户添加、删除、编辑 output_mapping 条目。
 * 每个映射包含 source (JSONPath)、target (变量名)、description (说明)。
 */
export function OutputMappingsEditor({
  control,
  errors,
  watch,
  register,
  saveData,
  jsonPathPresets,
  defaultSource = '$.status',
}: OutputMappingsEditorProps) {
  const { fields, append, remove } = useFieldArray({
    control,
    name: 'output_mappings' as never,
  });

  const handleAdd = () => {
    append({ source: defaultSource, target: '', description: '' });
  };

  const handleRemove = (index: number) => {
    remove(index);
    // 删除后延迟保存，确保表单状态已更新
    setTimeout(saveData, 100);
  };

  // 监听当前映射值
  const outputMappingsList = watch('output_mappings') || [];
  const panelsLL = getLL().panels;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ArrowRightFromLine className="h-4 w-4 text-muted-foreground" />
          <Label className="text-sm font-medium text-foreground">{panelsLL.outputMappings()}</Label>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleAdd}
          className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          <Plus className="h-3 w-3" />
          {panelsLL.addMapping()}
        </Button>
      </div>

      {/* 映射列表 */}
      <div className="space-y-2">
        {fields.length === 0 ? (
          <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
            <p className="text-xs text-muted-foreground">{panelsLL.noOutputMappings()}</p>
            <p className="mt-1 text-xs text-muted-foreground/60">
              {panelsLL.addMappingHint()}
            </p>
          </div>
        ) : (
          fields.map((field, index) => {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const mappingErrors = (errors.output_mappings as any)?.[index];
            const targetError = mappingErrors?.target;
            const sourceError = mappingErrors?.source;

            // 检查变量名是否重复
            const currentTarget = outputMappingsList[index]?.target || '';
            const duplicateTargets = outputMappingsList
              .map((m: { target?: string }, i: number) => (m.target && i !== index && m.target === currentTarget ? i : -1))
              .filter((i: number) => i !== -1);
            const hasDuplicate = duplicateTargets.length > 0 && currentTarget.trim() !== '';

            return (
              <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                {/* 第一行：变量名 + 删除按钮 */}
                <div className="flex items-center gap-2">
                  <Input
                    {...register(`output_mappings.${index}.target`)}
                    placeholder={panelsLL.mappingTargetPlaceholder()}
                    className={`h-8 flex-1 font-mono text-xs ${targetError || hasDuplicate ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => handleRemove(index)}
                    className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>

                {/* 第二行：源字段 */}
                <div className="relative">
                  <Input
                    {...register(`output_mappings.${index}.source`)}
                    placeholder={panelsLL.mappingSourcePlaceholder()}
                    className={`h-8 w-full font-mono text-xs ${sourceError ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                    list={`jsonpath-presets-${index}`}
                  />
                  <datalist id={`jsonpath-presets-${index}`}>
                    {jsonPathPresets.map((preset) => (
                      <option key={preset.value} value={preset.value} />
                    ))}
                  </datalist>
                </div>

                {/* 错误提示 */}
                {sourceError && (
                  <p className="text-xs text-rose-500">{sourceError.message}</p>
                )}
                {targetError && (
                  <p className="text-xs text-rose-500">{targetError.message}</p>
                )}
                {hasDuplicate && !targetError && (
                  <p className="text-xs text-rose-500">{panelsLL.variableNameExists()}</p>
                )}

                {/* 第三行：说明 */}
                <Input
                  {...register(`output_mappings.${index}.description`)}
                  placeholder={panelsLL.descriptionOptional()}
                  className="h-8 w-full text-xs"
                />
              </div>
            );
          })
        )}
      </div>

      {/* 提示信息 - JSONPath 语法说明 */}
      {fields.length > 0 && (
        <div className="flex items-start gap-2 rounded-md bg-muted/30 p-2">
          <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
          <div className="text-xs text-muted-foreground space-y-1">
            <p className="font-medium text-foreground">{panelsLL.jsonpathSyntax()}</p>
            <p><code className="rounded bg-muted px-1">$.status</code> {panelsLL.jsonpathExampleStatusCode()}</p>
            <p><code className="rounded bg-muted px-1">$.body.data.token</code> {panelsLL.jsonpathExampleNestedField()}</p>
            <p><code className="rounded bg-muted px-1">$.body.items[0]</code> {panelsLL.jsonpathExampleArrayIndex()}</p>
            <p><code className="rounded bg-muted px-1">$.headers.content-type</code> {panelsLL.jsonpathExampleResponseHeader()}</p>
            <p className="text-muted-foreground/70 mt-1">{panelsLL.variableNameRules()}</p>
          </div>
        </div>
      )}
    </div>
  );
}
