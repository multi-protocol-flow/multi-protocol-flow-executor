/**
 * 节点帮助内容注册表
 * 支持双语（zh-CN / en-US），根据当前 locale 返回对应内容
 */
export interface NodeHelpParam {
  name: string;
  type: string;
  required: boolean;
  description: string;
  example: string;
}

export interface NodeHelpUseCase {
  title: string;
  description: string;
}

export interface NodeHelpFAQ {
  question: string;
  answer: string;
}

export interface NodeHelpContent {
  typeId: string;
  summary: string;
  params: NodeHelpParam[];
  useCases: NodeHelpUseCase[];
  faq: NodeHelpFAQ[];
}

// ============================================================================
// 中文帮助内容
// ============================================================================

const zhHelp: Record<string, NodeHelpContent> = {
  http: {
    typeId: 'http',
    summary: '发送 HTTP 请求，支持 GET/POST/PUT/DELETE/PATCH 等方法。支持 JSON、表单、文件等多种请求体格式。',
    params: [
      { name: 'url', type: 'string', required: true, description: '请求目标 URL', example: 'https://api.example.com/users' },
      { name: 'method', type: 'enum', required: true, description: 'HTTP 请求方法', example: 'GET' },
      { name: 'headers', type: 'object', required: false, description: '自定义请求头', example: '{ "Authorization": "Bearer token" }' },
      { name: 'timeout_ms', type: 'number', required: false, description: '请求超时时间（毫秒）', example: '30000' },
    ],
    useCases: [
      { title: 'REST API 调用', description: '调用 RESTful API 获取或提交数据，支持各种认证方式。' },
      { title: 'Webhook 触发', description: '作为流程中的 Webhook 调用节点，触发外部系统操作。' },
    ],
    faq: [
      { question: '支持 HTTPS 吗？', answer: '支持，TLS 配置中可选择跳过证书验证用于开发环境。' },
      { question: '如何引用上一个节点的响应？', answer: '使用 {{body.field}} 引用响应体，{{status}} 引用状态码。' },
    ],
  },
  entry: {
    typeId: 'entry',
    summary: '流程的起点节点。每个流程必须有且仅有一个入口节点。',
    params: [
      { name: 'initial_variables', type: 'object', required: false, description: '流程初始变量', example: '{ "api_base": "https://api.example.com" }' },
    ],
    useCases: [
      { title: '定义初始变量', description: '在入口节点设置流程的初始变量，供后续节点引用。' },
    ],
    faq: [
      { question: '一个流程能有多个入口吗？', answer: '不可以，每个流程必须有且仅有一个入口节点。' },
    ],
  },
  conditional: {
    typeId: 'conditional',
    summary: '根据条件选择执行路径，支持简单比较和高级表达式。',
    params: [
      { name: 'condition_type', type: 'enum', required: true, description: '条件类型：simple 或 advanced', example: 'simple' },
      { name: 'field_path', type: 'string', required: true, description: '要检查的字段路径', example: 'status' },
      { name: 'operator', type: 'enum', required: true, description: '比较操作符', example: 'equal' },
      { name: 'expected_value', type: 'any', required: true, description: '期望值', example: '200' },
    ],
    useCases: [
      { title: 'HTTP 状态码判断', description: '根据 HTTP 响应的状态码选择不同执行路径。' },
      { title: '数据字段校验', description: '根据上游节点输出中的字段值进行分支选择。' },
    ],
    faq: [
      { question: 'true 和 false 端口有什么区别？', answer: '条件成立时走 true 端口，不成立时走 false 端口。' },
    ],
  },
  script: {
    typeId: 'script',
    summary: '执行 QuickJS JavaScript 代码，用于数据转换、变量操作和复杂逻辑。',
    params: [
      { name: 'code', type: 'string', required: true, description: 'JavaScript 脚本代码', example: 'fn.variables.set("result", body.data)' },
      { name: 'timeout_ms', type: 'number', required: false, description: '脚本执行超时', example: '5000' },
    ],
    useCases: [
      { title: '数据转换', description: '将上游节点的输出数据转换为下游需要的格式。' },
      { title: '变量操作', description: '读取、设置或删除流程变量，实现复杂状态管理。' },
    ],
    faq: [
      { question: '支持 async/await 吗？', answer: '支持 ES2020 语法，包括 async/await、Promise 等。' },
      { question: '脚本中能访问网络吗？', answer: '不能，脚本在沙箱环境中运行，禁止网络请求和文件系统访问。' },
    ],
  },
};

// ============================================================================
// 英文帮助内容
// ============================================================================

const enHelp: Record<string, NodeHelpContent> = {
  http: {
    typeId: 'http',
    summary: 'Send HTTP requests with GET/POST/PUT/DELETE/PATCH methods. Supports JSON, form-data, file uploads and more.',
    params: [
      { name: 'url', type: 'string', required: true, description: 'Request target URL', example: 'https://api.example.com/users' },
      { name: 'method', type: 'enum', required: true, description: 'HTTP request method', example: 'GET' },
      { name: 'headers', type: 'object', required: false, description: 'Custom request headers', example: '{ "Authorization": "Bearer token" }' },
      { name: 'timeout_ms', type: 'number', required: false, description: 'Request timeout in milliseconds', example: '30000' },
    ],
    useCases: [
      { title: 'REST API Call', description: 'Call RESTful APIs to fetch or submit data with various auth methods.' },
      { title: 'Webhook Trigger', description: 'Act as a webhook caller in flows to trigger external systems.' },
    ],
    faq: [
      { question: 'Does it support HTTPS?', answer: 'Yes, TLS configuration allows skipping certificate verification for development.' },
      { question: 'How to reference upstream node response?', answer: 'Use {{body.field}} for response body, {{status}} for status code.' },
    ],
  },
  entry: {
    typeId: 'entry',
    summary: 'The starting point of a flow. Every flow must have exactly one entry node.',
    params: [
      { name: 'initial_variables', type: 'object', required: false, description: 'Flow initial variables', example: '{ "api_base": "https://api.example.com" }' },
    ],
    useCases: [
      { title: 'Define Initial Variables', description: 'Set initial flow variables at the entry node for downstream nodes.' },
    ],
    faq: [
      { question: 'Can a flow have multiple entries?', answer: 'No, each flow must have exactly one entry node.' },
    ],
  },
  conditional: {
    typeId: 'conditional',
    summary: 'Route execution based on conditions. Supports simple comparisons and advanced expressions.',
    params: [
      { name: 'condition_type', type: 'enum', required: true, description: 'Condition type: simple or advanced', example: 'simple' },
      { name: 'field_path', type: 'string', required: true, description: 'Field path to evaluate', example: 'status' },
      { name: 'operator', type: 'enum', required: true, description: 'Comparison operator', example: 'equal' },
      { name: 'expected_value', type: 'any', required: true, description: 'Expected value', example: '200' },
    ],
    useCases: [
      { title: 'HTTP Status Check', description: 'Route based on HTTP response status code.' },
      { title: 'Data Field Validation', description: 'Branch based on field values in upstream node output.' },
    ],
    faq: [
      { question: 'What is the difference between true and false ports?', answer: 'True port executes when condition matches, false port when it does not.' },
    ],
  },
  script: {
    typeId: 'script',
    summary: 'Execute QuickJS JavaScript for data transformation, variable manipulation and complex logic.',
    params: [
      { name: 'code', type: 'string', required: true, description: 'JavaScript code', example: 'fn.variables.set("result", body.data)' },
      { name: 'timeout_ms', type: 'number', required: false, description: 'Script execution timeout', example: '5000' },
    ],
    useCases: [
      { title: 'Data Transformation', description: 'Transform upstream output to match downstream format.' },
      { title: 'Variable Manipulation', description: 'Read, set or delete flow variables for complex state management.' },
    ],
    faq: [
      { question: 'Does it support async/await?', answer: 'Yes, ES2020 features including async/await and Promise are supported.' },
      { question: 'Can scripts access the network?', answer: 'No, scripts run in a sandbox with no network or filesystem access.' },
    ],
  },
};

/** 获取当前 locale */
function getLocale(): 'zh-CN' | 'en-US' {
  try {
    const saved = localStorage.getItem('locale');
    if (saved === 'en-US') return 'en-US';
  } catch {}
  return 'zh-CN';
}

/**
 * 获取指定节点类型的帮助内容（根据当前 locale 返回对应语言）
 */
export function getNodeHelp(typeId: string): NodeHelpContent | undefined {
  const locale = getLocale();
  const helpMap = locale === 'en-US' ? enHelp : zhHelp;
  return helpMap[typeId];
}

/**
 * 获取所有有帮助内容的节点类型
 */
export function getNodeHelpTypes(): string[] {
  const locale = getLocale();
  const helpMap = locale === 'en-US' ? enHelp : zhHelp;
  return Object.keys(helpMap);
}
