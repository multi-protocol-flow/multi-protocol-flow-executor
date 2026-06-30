/**
 * cURL 命令解析工具
 * 将 cURL 命令字符串解析为 HttpConfigFormData 格式
 */
import parseCurl from '@bany/curl-to-json';
import type { HttpConfigFormData, HeaderEntry, HttpBodyType, ContentType } from '@/components/panels/schemas';

/** @bany/curl-to-json 的输出类型 */
interface CurlParseResult {
  url?: string;
  method?: string;
  header?: Record<string, string>;
  data?: Record<string, string> | string;
  form?: string[];
  cookie?: string;
  params?: Record<string, string | string[]>;
}

/** 判断字符串是否为有效的 JSON */
function isJsonString(str: string): boolean {
  try {
    const parsed = JSON.parse(str);
    return typeof parsed === 'object' && parsed !== null;
  } catch {
    return false;
  }
}

/** 根据 Content-Type 推断 ContentType 枚举值 */
function detectContentType(headers: Record<string, string> | undefined, body: string): ContentType {
  const ct = headers?.['Content-Type']?.toLowerCase() ?? '';
  if (ct.includes('text/html')) return 'html';
  if (ct.includes('text/xml') || ct.includes('application/xml')) return 'xml';
  if (ct.includes('text/plain')) return 'text';
  if (ct.includes('javascript')) return 'javascript';
  if (isJsonString(body)) return 'json';
  return 'json';
}

/** 根据 Content-Type 和 body 内容推断 HttpBodyType */
function detectBodyType(
  headers: Record<string, string> | undefined,
  data: Record<string, string> | string | undefined,
  form: string[] | undefined,
): HttpBodyType {
  if (form && form.length > 0) return 'form-data';

  if (data === undefined || data === null) return 'none';

  const ct = headers?.['Content-Type']?.toLowerCase() ?? '';
  if (ct.includes('application/x-www-form-urlencoded')) return 'x-www-form-urlencoded';

  return 'body';
}

/**
 * 将 cURL 命令解析为 HttpConfigFormData
 *
 * @param curlString - 用户粘贴的 cURL 命令
 * @returns 解析后的表单数据，或解析失败时返回 null
 */
export function parseCurlToFormData(curlString: string): HttpConfigFormData | null {
  if (!curlString.trim()) return null;

  try {
    const result = parseCurl(curlString) as CurlParseResult | null;
    if (!result || !result.url) return null;

    // 方法映射（大写转小写）
    const methodMap: Record<string, string> = {
      GET: 'get', POST: 'post', PUT: 'put', DELETE: 'delete', PATCH: 'patch',
      HEAD: 'head', OPTIONS: 'options',
    };
    const method = methodMap[result.method?.toUpperCase() ?? 'GET'] ?? 'get';

    // Headers：从对象转为 HeaderEntry 数组
    const headers: HeaderEntry[] = [];
    if (result.header && typeof result.header === 'object') {
      // 将 cookie 合并到 headers 中
      const allHeaders = { ...result.header };
      if (result.cookie) {
        allHeaders['Cookie'] = result.cookie;
      }
      for (const [key, value] of Object.entries(allHeaders)) {
        if (key && value !== undefined) {
          headers.push({ key, value: String(value) });
        }
      }
    } else if (result.cookie) {
      headers.push({ key: 'Cookie', value: result.cookie });
    }

    // URL：将 params 合并到 URL query string 中
    let url = result.url;
    if (result.params && Object.keys(result.params).length > 0) {
      const urlObj = new URL(url);
      for (const [key, val] of Object.entries(result.params)) {
        const values = Array.isArray(val) ? val : [val];
        values.forEach((v) => urlObj.searchParams.append(key, v));
      }
      url = urlObj.toString();
    }

    // Body 处理
    const bodyType = detectBodyType(result.header, result.data, result.form);
    let body: string | undefined;
    let formFields: { key: string; value: string; type?: 'text' | 'file'; file_path?: string }[] | undefined;
    let contentType: ContentType = 'json';

    if (bodyType === 'form-data' && result.form) {
      // -F 表单字段
      formFields = result.form.map((entry) => {
        // 格式: "key=value" 或 "key=@filepath"
        if (entry.startsWith('@') || entry.includes('=@')) {
          // 文件上传
          const eqIndex = entry.indexOf('=@');
          if (eqIndex >= 0) {
            const key = entry.substring(0, eqIndex);
            const filePath = entry.substring(eqIndex + 2);
            return { key, value: '', field_type: 'file' as const, file_path: filePath };
          }
          // 无 key 的文件: @filepath
          return { key: 'file', value: '', field_type: 'file' as const, file_path: entry.substring(1) };
        }
        const eqIndex = entry.indexOf('=');
        if (eqIndex >= 0) {
          return {
            key: entry.substring(0, eqIndex),
            value: entry.substring(eqIndex + 1),
            field_type: 'text' as const,
          };
        }
        return { key: entry, value: '', type: 'text' as const };
      });
    } else if (bodyType === 'x-www-form-urlencoded' && result.data && typeof result.data === 'object') {
      // URL-encoded 表单
      formFields = Object.entries(result.data).map(([key, value]) => ({
        key,
        value: String(value),
      }));
    } else if (bodyType === 'body') {
      // Raw body
      if (typeof result.data === 'string') {
        body = result.data;
      } else if (typeof result.data === 'object') {
        body = JSON.stringify(result.data, null, 2);
      }
      contentType = detectContentType(result.header, body ?? '');
    }

    return {
      url,
      method: method as HttpConfigFormData['method'],
      headers: headers.length > 0 ? headers : undefined,
      body_type: bodyType,
      content_type: contentType,
      body,
      form_fields: formFields,
      timeout_ms: 30000,
    };
  } catch {
    return null;
  }
}
