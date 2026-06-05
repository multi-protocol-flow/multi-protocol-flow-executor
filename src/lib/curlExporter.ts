/**
 * cURL 命令生成器
 * 根据 HTTP 请求配置生成等效的 curl CLI 命令
 */

/**
 * cURL 生成器输入参数
 */
export interface CurlExportOptions {
  /** 请求 URL */
  url: string;
  /** HTTP 方法（GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS） */
  method: string;
  /** 请求头列表 */
  headers?: { key: string; value: string }[];
  /** Body 类型 */
  bodyType?: 'none' | 'body' | 'form-data' | 'x-www-form-urlencoded' | 'binary';
  /** Body 原始内容（body 和 binary 类型使用） */
  bodyContent?: string;
  /** Form 字段列表（form-data 和 x-www-form-urlencoded 使用） */
  formData?: { key: string; value: string; type: 'text' | 'file' }[];
  /** Content-Type 映射 key（json/text/html/xml/javascript） */
  contentType?: string;
}

/** Content-Type 映射表 */
const CONTENT_TYPE_MAP: Record<string, string> = {
  json: 'application/json',
  text: 'text/plain',
  html: 'text/html',
  xml: 'application/xml',
  javascript: 'application/javascript',
};

/**
 * Shell 引号包裹
 * 使用单引号包裹，内部单引号转义为 '\''
 *
 * @param str - 需要包裹的字符串
 * @returns 包裹后的字符串
 */
export function shellQuote(str: string): string {
  // 如果字符串不含特殊字符，可以直接使用
  if (/^[a-zA-Z0-9_./:@\-]+$/.test(str)) {
    return str;
  }
  // 使用单引号包裹，转义内部单引号
  return `'${str.replace(/'/g, "'\\''")}'`;
}

/**
 * 将 HTTP 配置转换为 curl 命令字符串
 *
 * 规则：
 * - GET 请求省略 -X GET（curl 默认）
 * - 其他方法使用 -X
 * - 每个 Header 添加 -H
 * - Body 内容使用 -d（自动添加 Content-Type header）
 * - Form-data 字段使用 -F（file 类型添加 @ 前缀）
 * - x-www-form-urlencoded 使用 --data-urlencode
 * - Binary 文件使用 --data-binary @path
 * - URL 始终为最后一个位置参数
 * - 多行输出使用 ' \\\n  ' 连接
 *
 * @param options - curl 生成器输入参数
 * @returns 格式化的 curl 命令字符串
 */
export function generateCurlCommand(options: CurlExportOptions): string {
  const parts: string[] = [];
  const method = options.method.toUpperCase();

  // 第一行：curl + 方法（非 GET 时添加 -X）
  if (method !== 'GET') {
    parts.push(`curl -X ${method}`);
  } else {
    parts.push('curl');
  }

  // Headers
  if (options.headers && options.headers.length > 0) {
    for (const header of options.headers) {
      if (header.key.trim()) {
        parts.push(`-H ${shellQuote(`${header.key}: ${header.value}`)}`);
      }
    }
  }

  // Content-Type header（仅 body 类型）
  if (options.bodyType === 'body' && options.bodyContent && options.contentType) {
    const mimeType = CONTENT_TYPE_MAP[options.contentType] || options.contentType;
    parts.push(`-H ${shellQuote(`Content-Type: ${mimeType}`)}`);
  }

  // Body 内容
  if (options.bodyType === 'body' && options.bodyContent && options.bodyContent.trim()) {
    parts.push(`-d ${shellQuote(options.bodyContent)}`);
  }

  // Form-data 字段
  if (options.bodyType === 'form-data' && options.formData) {
    for (const field of options.formData) {
      if (field.key.trim()) {
        if (field.type === 'file') {
          parts.push(`-F ${shellQuote(`${field.key}=@${field.value}`)}`);
        } else {
          parts.push(`-F ${shellQuote(`${field.key}=${field.value}`)}`);
        }
      }
    }
  }

  // x-www-form-urlencoded 字段
  if (options.bodyType === 'x-www-form-urlencoded' && options.formData) {
    for (const field of options.formData) {
      if (field.key.trim()) {
        parts.push(`--data-urlencode ${shellQuote(`${field.key}=${field.value}`)}`);
      }
    }
  }

  // Binary 文件
  if (options.bodyType === 'binary' && options.bodyContent && options.bodyContent.trim()) {
    parts.push(`--data-binary ${shellQuote(`@${options.bodyContent}`)}`);
  }

  // URL 始终为最后一个位置参数
  parts.push(shellQuote(options.url));

  return parts.join(' \\\n  ');
}
