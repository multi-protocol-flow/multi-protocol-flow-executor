/**
 * MessageSchemaView — Recursive read-only protobuf schema tree viewer
 *
 * Displays field definitions, nested structures, oneof groups, enums,
 * WKTs, circular references, and reserved fields from gRPC message
 * definitions returned by the backend.
 */
import type { GrpcMessageInfo, GrpcFieldInfo, GrpcOneofInfo } from '@/types';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import { useState, useCallback, useMemo } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';

// ============================================================================
// Types
// ============================================================================

interface MessageSchemaViewProps {
  /** The message definition to render */
  message: GrpcMessageInfo;
  /** Flat map of all known message definitions keyed by full_name (for fallback lookup) */
  messageDefinitions: Record<string, GrpcMessageInfo>;
  /** Current nesting depth (starts at 0) */
  depth?: number;
  /** Optional label like "Request" or "Response" shown next to the type name */
  label?: string;
}

// ============================================================================
// Helpers
// ============================================================================

/** Format [[1,3],[10,10]] → "1-3, 10" */
function formatReservedRanges(ranges: [number, number][]): string {
  return ranges
    .map(([start, end]) => (start === end ? String(start) : `${start}-${end}`))
    .join(', ');
}

/** Build a Set of all field numbers that belong to any oneof group */
function buildOneofFieldNumbers(oneofs: GrpcOneofInfo[]): Set<number> {
  const set = new Set<number>();
  for (const g of oneofs) {
    for (const n of g.field_numbers) set.add(n);
  }
  return set;
}

// ============================================================================
// Sub-components
// ============================================================================

/** Single field row rendered inside a message */
interface FieldRowProps {
  field: GrpcFieldInfo;
  messageDefinitions: Record<string, GrpcMessageInfo>;
  depth: number;
}

function FieldRow({ field, messageDefinitions, depth }: FieldRowProps) {
  const ll = getLL();
  const [expanded, setExpanded] = useState(false);

  const handleToggle = useCallback(() => setExpanded((p) => !p), []);

  // Resolve the nested message to display when this field is expanded:
  //  1. Inline nested_message from the backend
  //  2. Fallback: look up by type_full_name in the flat definitions map
  const resolvedNested = useMemo<GrpcMessageInfo | null>(() => {
    if (field.nested_message) return field.nested_message;
    if (
      field.type_kind === 'message' &&
      field.type_full_name &&
      messageDefinitions[field.type_full_name]
    ) {
      return messageDefinitions[field.type_full_name];
    }
    return null;
  }, [field, messageDefinitions]);

  const canExpand =
    field.type_kind === 'message' && resolvedNested !== null && !resolvedNested.is_wkt;

  return (
    <div>
      <div className="flex items-center gap-1.5 py-0.5">
        {/* Expand chevron for nested message fields */}
        {canExpand ? (
          <button
            type="button"
            onClick={handleToggle}
            className="p-0 hover:bg-muted/30 rounded-sm shrink-0"
            title={expanded ? ll.panels.grpcConnect.collapseType() : ll.panels.grpcConnect.expandType()}
          >
            {expanded ? (
              <ChevronDown className="h-3 w-3 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-3 w-3 text-muted-foreground" />
            )}
          </button>
        ) : (
          <span className="inline-block w-3 shrink-0" />
        )}

        {/* Type display — clickable for expandable message types */}
        {canExpand ? (
          <button
            type="button"
            onClick={handleToggle}
            className="font-mono text-xs text-violet-600 hover:text-violet-800 hover:underline shrink-0 cursor-pointer"
          >
            {field.type_display}
          </button>
        ) : (
          <span className="font-mono text-xs text-violet-600 shrink-0">
            {field.type_display}
          </span>
        )}

        {/* Field name */}
        <span className="font-mono text-xs text-foreground shrink-0">{field.name}</span>

        {/* Field number */}
        <span className="text-xs text-muted-foreground font-mono shrink-0">
          = {field.number}
        </span>

        {/* Enum values inline (hidden on narrow screens) */}
        {field.type_kind === 'enum' &&
          field.enum_values &&
          field.enum_values.length > 0 && (
            <span className="text-[10px] text-muted-foreground truncate hidden xl:inline">
              // (
              {field.enum_values.map((v) => `${v.name}=${v.number}`).join(', ')}
              )
            </span>
          )}

        {/* Circular reference indicator */}
        {field.type_kind === 'circular_ref' && (
          <span className="text-[10px] text-muted-foreground italic shrink-0">
            ({ll.panels.grpcConnect.circularReference()})
          </span>
        )}

        {/* Cardinality label badge */}
        {(field.label === 'repeated' || field.label === 'optional') && !field.is_map && (
          <span className="inline-flex items-center rounded bg-muted px-1 py-0.5 text-[10px] font-mono text-muted-foreground shrink-0">
            {field.label}
          </span>
        )}

        {/* WKT badge for well-known types inlined as fields */}
        {field.type_kind === 'message' &&
          !canExpand &&
          field.type_full_name?.startsWith('google.protobuf.') && (
            <span className="inline-flex items-center rounded bg-violet-100 px-1.5 py-0.5 text-[10px] font-medium text-violet-700 shrink-0">
              WKT
            </span>
          )}
      </div>

      {/* Expanded nested message */}
      {canExpand && expanded && resolvedNested && (
        <MessageSchemaView
          message={resolvedNested}
          messageDefinitions={messageDefinitions}
          depth={depth + 1}
        />
      )}
    </div>
  );
}

/** Oneof group with a bordered container */
interface OneofGroupViewProps {
  group: GrpcOneofInfo;
  allFields: GrpcFieldInfo[];
  messageDefinitions: Record<string, GrpcMessageInfo>;
  depth: number;
}

function OneofGroupView({
  group,
  allFields,
  messageDefinitions,
  depth,
}: OneofGroupViewProps) {
  const ll = getLL();
  const [expanded, setExpanded] = useState(true);

  const handleToggle = useCallback(() => setExpanded((p) => !p), []);

  const groupFields = useMemo(
    () => allFields.filter((f) => group.field_numbers.includes(f.number)),
    [allFields, group.field_numbers],
  );

  if (groupFields.length === 0) return null;

  return (
    <div className="ml-1 border-l-2 border-violet-200 pl-3 my-1">
      <button
        type="button"
        onClick={handleToggle}
        className="flex items-center gap-1.5 py-0.5 text-left hover:bg-muted/30 rounded-sm transition-colors w-full"
      >
        {expanded ? (
          <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
        )}
        <span className="font-mono text-[10px] font-semibold uppercase tracking-wider text-violet-500">
          {ll.panels.grpcConnect.oneofGroup({ group: group.name })}
        </span>
      </button>
      {expanded && (
        <div className="space-y-0.5 pt-0.5">
          {groupFields.map((field) => (
            <FieldRow
              key={field.number}
              field={field}
              messageDefinitions={messageDefinitions}
              depth={depth}
            />
          ))}
        </div>
      )}
    </div>
  );
}

/** Reserved ranges and names display */
interface ReservedFieldsViewProps {
  reservedRanges?: [number, number][];
  reservedNames?: string[];
  depth: number;
}

function ReservedFieldsView({ reservedRanges, reservedNames }: ReservedFieldsViewProps) {
  const ll = getLL();
  const parts: string[] = [];

  if (reservedRanges && reservedRanges.length > 0) {
    parts.push(ll.panels.grpcConnect.reserved({ ranges: formatReservedRanges(reservedRanges) }));
  }

  if (reservedNames && reservedNames.length > 0) {
    parts.push(ll.panels.grpcConnect.reservedNames({ names: reservedNames.join(', ') }));
  }

  if (parts.length === 0) return null;

  return (
    <div className="py-0.5">
      <span className="text-[10px] italic text-muted-foreground/60">{parts.join('; ')}</span>
    </div>
  );
}

// ============================================================================
// Main component
// ============================================================================

export function MessageSchemaView({
  message,
  messageDefinitions,
  depth = 0,
  label,
}: MessageSchemaViewProps) {
  const ll = getLL();
  const [expanded, setExpanded] = useState(depth < 2);

  const handleToggle = useCallback(() => setExpanded((p) => !p), []);

  // ── 1. WKT — collapsed single line ──────────────────────────────────
  if (message.is_wkt) {
    return (
      <div className={cn('flex items-center gap-2 py-0.5', depth > 0 && 'pl-4')}>
        <span className="font-mono text-xs text-foreground">{message.full_name}</span>
        <span className="inline-flex items-center rounded bg-violet-100 px-1.5 py-0.5 text-[10px] font-medium text-violet-700 shrink-0">
          WKT
        </span>
      </div>
    );
  }

  // ── 2. Empty message ────────────────────────────────────────────────
  if (
    message.fields.length === 0 &&
    message.oneof_groups.length === 0 &&
    (!message.reserved_ranges || message.reserved_ranges.length === 0) &&
    (!message.reserved_names || message.reserved_names.length === 0)
  ) {
    return (
      <div className={cn('flex items-center gap-2 py-0.5', depth > 0 && 'pl-4')}>
        {depth > 0 && (
          <span className="inline-block w-3 shrink-0" />
        )}
        <span className="font-mono text-xs text-foreground">{message.full_name}</span>
        <span className="text-xs italic text-muted-foreground">
          {ll.panels.grpcConnect.emptyMessage()}
        </span>
      </div>
    );
  }

  // ── 3. Max depth reached ────────────────────────────────────────────
  if (depth >= 5) {
    return (
      <div className={cn('flex items-center gap-2 py-0.5', depth > 0 && 'pl-4')}>
        {depth > 0 && (
          <span className="inline-block w-3 shrink-0" />
        )}
        <span className="font-mono text-xs text-foreground">{message.full_name}</span>
        <span className="text-xs text-muted-foreground">
          ... {ll.panels.grpcConnect.maxDepth()}
        </span>
      </div>
    );
  }

  // ── 4. Full rendering ───────────────────────────────────────────────
  const oneofFieldNumbers = useMemo(
    () => buildOneofFieldNumbers(message.oneof_groups),
    [message.oneof_groups],
  );

  const regularFields = useMemo(
    () => message.fields.filter((f) => !oneofFieldNumbers.has(f.number)),
    [message.fields, oneofFieldNumbers],
  );

  return (
    <div className={cn(depth > 0 && 'pl-4')}>
      {/* Collapsible header */}
      <button
        type="button"
        onClick={handleToggle}
        className={cn(
          'flex w-full items-center gap-1.5 py-1 text-left',
          'hover:bg-muted/30 rounded-sm transition-colors',
        )}
      >
        {expanded ? (
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        )}
        <span className="font-mono text-xs font-semibold text-violet-700">
          {message.full_name}
        </span>
        {label && (
          <span className="inline-flex items-center rounded bg-violet-100 px-1.5 py-0.5 text-[10px] font-medium text-violet-600 shrink-0">
            {label}
          </span>
        )}
      </button>

      {/* Expanded body */}
      {expanded && (
        <div className="space-y-0.5 pt-0.5">
          {/* Regular fields (not part of any oneof group) */}
          {regularFields.map((field) => (
            <FieldRow
              key={field.number}
              field={field}
              messageDefinitions={messageDefinitions}
              depth={depth}
            />
          ))}

          {/* Oneof groups */}
          {message.oneof_groups.map((group) => (
            <OneofGroupView
              key={group.name}
              group={group}
              allFields={message.fields}
              messageDefinitions={messageDefinitions}
              depth={depth}
            />
          ))}

          {/* Reserved fields */}
          <ReservedFieldsView
            reservedRanges={message.reserved_ranges}
            reservedNames={message.reserved_names}
            depth={depth}
          />
        </div>
      )}
    </div>
  );
}
