# src/lib — Utility & Registry

**生成时间:** 2026-06-12

## OVERVIEW

Utility functions and the triple registry system that bridges backend node types with frontend rendering. The three registries work together: `NodeRegistry` fetches metadata from the backend via IPC, `configPanelRegistry` lazily loads config panel components, and `viewerRegistry` maps execution results to viewer components. Supporting utilities handle flow diff/restore/search, cURL import/export, grpcurl generation, proto field parsing, and GraphQL schema introspection.

## STRUCTURE

| File | Purpose |
|------|---------|
| `NodeRegistry.ts` | Backend-driven metadata registry (IPC `get_registered_node_types`), cached and grouped by category |
| `configPanelRegistry.ts` | Lazy-loaded config panel loader, maps node type to dynamic `import()` |
| `viewerRegistry.tsx` | Execution result viewer components, maps node type to viewer |
| `nodeMeta.ts` | Icon map + color map (Lucide icons) + `getNodeDisplayName()` integration with registry |
| `nodeDefaults.ts` | Context-aware default value inference for new nodes |
| `nodeHelpRegistry.ts` | Bilingual (zh-CN/en-US) help content for node configuration |
| `flowDiff.ts` | Deep field-level diff between two flows (nodes + connections) |
| `flowSearch.ts` | Full-text search across node names, configs, and variable references |
| `restoreFlow.ts` | Flow snapshot restore with dirty-canvas protection and conflict handling |
| `utils.ts` | `cn()` — Tailwind class merge utility (`clsx` + `twMerge`) |
| `curlParser.ts` | cURL string to `HttpConfigFormData` parser |
| `curlExporter.ts` | HTTP config to cURL CLI command generator |
| `grpcurlGenerator.ts` | gRPC call config to `grpcurl` CLI command generator |
| `protoFieldParser.ts` | Lightweight `.proto` file parser (proto3 field declarations, enums, messages) |
| `graphqlSchema.ts` | GraphQL introspection JSON to field tree (query builder support) |
| `hotkeyLabels.ts` | Platform-aware shortcut labels (Cmd on macOS, Ctrl on Windows) |

## WHERE TO LOOK

| Task | File |
|------|------|
| Add a new node type — register it | `NodeRegistry.ts` + `configPanelRegistry.ts` + `viewerRegistry.tsx` |
| Map node type to icon/color | `nodeMeta.ts` |
| Set reasonable defaults for a node | `nodeDefaults.ts` |
| Write help text for a node config | `nodeHelpRegistry.ts` |
| Compare two flows for changes | `flowDiff.ts` |
| Search across the current flow | `flowSearch.ts` |
| Restore a flow from snapshot | `restoreFlow.ts` |
| Import/export cURL commands | `curlParser.ts`, `curlExporter.ts` |
| Generate grpcurl command | `grpcurlGenerator.ts` |
| Parse .proto files for form UI | `protoFieldParser.ts` |
| Build GraphQL query from schema | `graphqlSchema.ts` |
| Format platform-aware shortcuts | `hotkeyLabels.ts` |

## CONVENTIONS

- **Triple registry pattern**: Every new node type must register in all 3 files — `NodeRegistry.ts` (metadata), `configPanelRegistry.ts` (config panel), `viewerRegistry.tsx` (result viewer).
- **Backend-first naming**: Display names in `nodeMeta.ts` prioritize names from `NodeRegistry` (backend) over hardcoded values. Use `getNodeDisplayName()` instead of direct string lookups.
- **Lazy loading**: Config panels use `() => import()` for code splitting — only loaded when user first edits that node type.
- **Bilingual help**: `nodeHelpRegistry.ts` entries provide both zh-CN and en-US content.

## ANTI-PATTERNS

- **Skipping one of the three registries** breaks the rendering pipeline. All three must be updated for a new node type.
- **Hardcoding display names** in `nodeMeta.ts` instead of using `getNodeDisplayName()` — backend may update names dynamically.
- **Direct panel imports** instead of lazy loading — defeats code splitting and increases initial bundle size.
