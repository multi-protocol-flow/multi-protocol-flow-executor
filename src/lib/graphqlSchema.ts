/**
 * GraphQL Schema 解析工具
 *
 * 将内省 Schema JSON 转为前端可用的字段树结构，支持多选勾选和查询生成。
 */

// ============================================================================
// 类型定义
// ============================================================================

export interface FieldNode {
  /** 字段名 */
  name: string;
  /** 字段描述 */
  description: string | null;
  /** 类型显示名（如 "String", "[Film!]!", "Int"） */
  typeName: string;
  /** 类型种类（SCALAR / OBJECT / ENUM / INTERFACE / UNION） */
  typeKind: string;
  /** 是否必填（顶层 NON_NULL 包装） */
  isRequired: boolean;
  /** 是否为叶子节点（SCALAR / ENUM → 无子字段） */
  isLeaf: boolean;
  /** 字段参数 */
  args: FieldArg[];
  /** 子字段（OBJECT / INTERFACE 类型） */
  children: FieldNode[];
  /** 用户勾选状态 */
  checked: boolean;
  /** 树节点展开状态 */
  expanded: boolean;
  /** 父 GraphQL 类型名 */
  parentTypeName: string;
}

export interface FieldArg {
  /** 参数名 */
  name: string;
  /** 参数描述 */
  description: string | null;
  /** 参数类型显示名 */
  typeName: string;
  /** 是否必填 */
  isRequired: boolean;
  /** 默认值 */
  defaultValue: string | null;
}

// ============================================================================
// Schema 解析
// ============================================================================

/** 从 Schema JSON 构建类型索引 Map */
export function buildTypesMap(schema: unknown): Map<string, any> {
  const map = new Map<string, any>();
  const schemaObj = (schema as any)?.['__schema'];
  if (!schemaObj?.types) return map;
  for (const type of schemaObj.types) {
    if (type?.name && !type.name.startsWith('__')) {
      map.set(type.name, type);
    }
  }
  return map;
}

/** 获取 Query / Mutation / Subscription 的根类型名 */
export function getRootTypeName(schema: unknown, operation: 'query' | 'mutation' | 'subscription'): string | null {
  const schemaObj = (schema as any)?.['__schema'];
  if (!schemaObj) return null;
  const key = operation === 'query' ? 'queryType' : operation === 'mutation' ? 'mutationType' : 'subscriptionType';
  return schemaObj[key]?.name ?? null;
}

/** 提取基础类型名（剥掉 NON_NULL / LIST 包装） */
export function extractTypeName(typeRef: any): string | null {
  if (!typeRef) return null;
  let current = typeRef;
  for (let i = 0; i < 10; i++) {
    const kind = current?.kind;
    if (kind === 'NON_NULL' || kind === 'LIST') {
      current = current?.ofType;
      if (!current) return null;
    } else {
      return current?.name ?? null;
    }
  }
  return null;
}

/** 格式化类型显示名（如 "String", "[Film!]!", "Int"） */
export function formatTypeName(typeRef: any): string {
  if (!typeRef) return 'Unknown';
  
  const kind = typeRef.kind;
  const name = typeRef.name;
  const ofType = typeRef.ofType;
  
  switch (kind) {
    case 'NON_NULL':
      return `${formatTypeName(ofType)}!`;
    case 'LIST':
      return `[${formatTypeName(ofType)}]`;
    default:
      return name ?? 'Unknown';
  }
}

/** 解析类型引用：返回类型名、种类、是否必填、是否叶子 */
export function resolveTypeDisplay(typeRef: any): {
  name: string;
  kind: string;
  isRequired: boolean;
  isLeaf: boolean;
} {
  if (!typeRef) return { name: 'Unknown', kind: 'UNKNOWN', isRequired: false, isLeaf: true };
  
  let isRequired = false;
  let current = typeRef;
  
  // 剥掉顶层 NON_NULL
  if (current.kind === 'NON_NULL') {
    isRequired = true;
    current = current.ofType;
  }
  
  // 确定实际类型
  const kind = current?.kind ?? 'UNKNOWN';
  const name = current?.name ?? 'Unknown';
  
  // SCALAR 和 ENUM 是叶子节点
  const isLeaf = kind === 'SCALAR' || kind === 'ENUM';
  
  return { name, kind, isRequired, isLeaf };
}

// ============================================================================
// 字段树构建
// ============================================================================

const GRAPHQL_BUILTIN_TYPES = new Set([
  'String', 'Int', 'Float', 'Boolean', 'ID',
]);

/** 判断是否为 GraphQL 内置标量类型 */
function isBuiltinType(name: string): boolean {
  return GRAPHQL_BUILTIN_TYPES.has(name);
}

/** 递归构建字段树 */
export function buildFieldTree(
  schema: unknown,
  parentTypeName: string,
  typesMap: Map<string, any>,
  depth: number = 0,
  visited: Set<string> = new Set(),
  maxDepth: number = 3,
): FieldNode[] {
  if (depth > maxDepth) return [];
  if (visited.has(parentTypeName)) return [];
  
  const typeDef = typesMap.get(parentTypeName);
  if (!typeDef) return [];
  
  // INTERFACE 和 UNION 类型没有 fields
  if (typeDef.kind !== 'OBJECT' && typeDef.kind !== 'INTERFACE') return [];
  
  const rawFields = typeDef.fields;
  if (!rawFields || !Array.isArray(rawFields)) return [];
  
  visited.add(parentTypeName);
  const nodes: FieldNode[] = [];
  
  for (const field of rawFields) {
    const name = field.name;
    if (name.startsWith('__')) continue;
    
    const display = resolveTypeDisplay(field.type);
    const isLeaf = display.isLeaf || depth >= maxDepth;
    
    let children: FieldNode[] = [];
    if (!isLeaf) {
      const baseTypeName = extractTypeName(field.type);
      if (baseTypeName && !isBuiltinType(baseTypeName) && typesMap.has(baseTypeName)) {
        children = buildFieldTree(
          schema,
          baseTypeName,
          typesMap,
          depth + 1,
          new Set(visited),
          maxDepth,
        );
      }
    }
    
    // 构建参数
    const args: FieldArg[] = [];
    if (field.args && Array.isArray(field.args)) {
      for (const arg of field.args) {
        const argDisplay = resolveTypeDisplay(arg.type);
        args.push({
          name: arg.name,
          description: arg.description ?? null,
          typeName: formatTypeName(arg.type),
          isRequired: argDisplay.isRequired,
          defaultValue: arg.defaultValue ?? null,
        });
      }
    }
    
    nodes.push({
      name,
      description: field.description ?? null,
      typeName: formatTypeName(field.type),
      typeKind: display.kind,
      isRequired: display.isRequired,
      isLeaf,
      args,
      children,
      checked: false,
      expanded: depth < 1, // 第一层展开，更深层折叠
      parentTypeName,
    });
  }
  
  visited.delete(parentTypeName);
  return nodes;
}

// ============================================================================
// 查询生成
// ============================================================================

function pascalCase(s: string): string {
  if (!s) return '';
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/**
 * 从选中的字段节点生成 GraphQL 查询字符串
 * @param selectedNodes 被选中的字段节点（从不同根字段收集）
 * @param operation 操作类型
 * @param rootFieldName 根字段名（用于生成操作名）
 */
export function generateQueryFromSelection(
  selectedNodes: FieldNode[],
  operation: 'query' | 'mutation' | 'subscription',
  rootFieldName: string,
): string {
  if (selectedNodes.length === 0) return '';
  
  const opName = pascalCase(rootFieldName);
  const lines: string[] = [];
  
  // 生成每个选中字段的选择
  const selections = selectedNodes
    .map(n => renderFieldSelection(n, 0))
    .filter(Boolean)
    .join('\n');
  
  if (!selections) return '';
  
  lines.push(`${operation} ${opName} {`);
  lines.push(indent(selections, 2));
  lines.push('}');
  
  return lines.join('\n');
}

/** 递归渲染单个字段的选择（支持父节点未勾选但子节点已勾选的场景） */
function renderFieldSelection(node: FieldNode, depth: number): string {
  // 先递归收集已勾选的子字段，以此判断本节点是否需要渲染
  const childSelections = node.children
    .map(c => renderFieldSelection(c, depth + 1))
    .filter(Boolean);
  
  // 本节点未勾选且无任何子字段被勾选 → 跳过
  if (!node.checked && childSelections.length === 0) return '';
  
  const indent = '  '.repeat(depth);
  const fieldLine = `${indent}${node.name}`;
  
  if (childSelections.length > 0) {
    const children = childSelections.join('\n');
    return `${fieldLine} {\n${children}\n${indent}}`;
  }
  
  return fieldLine;
}

/** 缩进文本 */
function indent(text: string, spaces: number): string {
  const prefix = ' '.repeat(spaces);
  return text.split('\n').map(line => `${prefix}${line}`).join('\n');
}

/**
 * 获取所有根字段的完整 FieldNode 树（用于展示操作类型选择）
 */
export function buildOperationFieldTrees(
  schema: unknown,
  operation: 'query' | 'mutation' | 'subscription',
): { rootTypeName: string | null; rootFields: FieldNode[] } {
  const typesMap = buildTypesMap(schema);
  const rootTypeName = getRootTypeName(schema, operation);
  
  if (!rootTypeName) {
    return { rootTypeName: null, rootFields: [] };
  }
  
  // 每个根字段作为一个独立的树
  const rootTypeDef = typesMap.get(rootTypeName);
  if (!rootTypeDef?.fields) {
    return { rootTypeName, rootFields: [] };
  }
  
  const visited = new Set<string>();
  const rootFields: FieldNode[] = [];
  
  for (const field of rootTypeDef.fields) {
    const name = field.name;
    if (name.startsWith('__')) continue;
    
    const display = resolveTypeDisplay(field.type);
    const baseTypeName = extractTypeName(field.type);
    
    let children: FieldNode[] = [];
    if (baseTypeName && !isBuiltinType(baseTypeName) && display.kind !== 'SCALAR' && display.kind !== 'ENUM') {
      visited.clear();
      children = buildFieldTree(schema, baseTypeName, typesMap, 1, visited, 3);
    }
    
    const args: FieldArg[] = [];
    if (field.args && Array.isArray(field.args)) {
      for (const arg of field.args) {
        const argDisplay = resolveTypeDisplay(arg.type);
        args.push({
          name: arg.name,
          description: arg.description ?? null,
          typeName: formatTypeName(arg.type),
          isRequired: argDisplay.isRequired,
          defaultValue: arg.defaultValue ?? null,
        });
      }
    }
    
    rootFields.push({
      name,
      description: field.description ?? null,
      typeName: formatTypeName(field.type),
      typeKind: display.kind,
      isRequired: false,
      isLeaf: false,
      args,
      children,
      checked: false,
      expanded: true,
      parentTypeName: rootTypeName,
    });
  }
  
  return { rootTypeName, rootFields };
}
