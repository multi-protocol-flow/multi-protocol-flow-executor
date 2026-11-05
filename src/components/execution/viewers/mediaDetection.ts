/**
 * mediaDetection - 从节点输出中检测可预览的图片/视频
 *
 * 三种来源：
 * 1. 后端媒体分流（`output_data.media`，HTTP `image/*` / `video/*` 响应）
 * 2. JSON 中的媒体 URL（`result_url`、`video_url`、`url` 等白名单键）
 * 3. JSON 中的 base64 载荷（`inlineData`、`b64_json`、`base64` 等，如 Gemini / OpenAI 图片接口）
 *
 * 全部为纯函数：无副作用、无 React 依赖，便于单独测试。
 */

/** 媒体大类 */
export type MediaKind = 'image' | 'video';

/** 一个可预览的媒体候选 */
export interface MediaCandidate {
  /** React list key：`${sourceType}:${index}` 或 `omitted:${index}` */
  id: string;
  /** 媒体大类 */
  kind: MediaKind;
  /** MIME，未知时为 '' */
  mime: string;
  /** 内容来源；超限占位时缺省 */
  source?: { type: 'data_url' | 'url'; value: string };
  /** 字节数（后端提供时才有） */
  bytes?: number;
  /** 超过内联上限、内容未保留 */
  omitted?: true;
}

/** 扫描字符串上限（8 MiB） */
const MAX_SCAN_STRING = 8 * 1024 * 1024;
/** base64 载荷最小长度：低于此值几乎不可能是真实媒体，避免误判短文本 */
const MIN_BASE64_LENGTH = 32;
/** 递归深度上限 */
const MAX_DEPTH = 8;
/** 候选总数上限 */
const MAX_CANDIDATES = 8;

/** 视为媒体 URL 的键名（小写） */
const URL_KEYS = ['url', 'result_url', 'video_url', 'image_url', 'output_url'];
/** 视为 base64 载荷的键名（小写） */
const INLINE_KEYS = ['inlinedata', 'inline_data', 'b64_json', 'base64', 'image'];

/** 已知 MIME → 扩展名 */
const MIME_EXTENSIONS: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/jpg': 'jpg',
  'image/gif': 'gif',
  'image/webp': 'webp',
  'image/bmp': 'bmp',
  'image/svg+xml': 'svg',
  'image/avif': 'avif',
  'video/mp4': 'mp4',
  'video/webm': 'webm',
  'video/quicktime': 'mov',
};

/** URL 路径中可识别的媒体后缀 */
const KNOWN_EXTENSIONS = [
  'png',
  'jpg',
  'jpeg',
  'gif',
  'webp',
  'bmp',
  'svg',
  'avif',
  'mp4',
  'webm',
  'mov',
];

/** 判断是否为普通对象（非数组、非 null） */
function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * 大小写不敏感地读取响应头第一个字符串值
 *
 * @param headers - 响应头对象（值可能是 string 或 string[]）
 * @param name - 头名
 */
export function headerValue(
  headers: Record<string, unknown> | undefined,
  name: string
): string | undefined {
  if (!headers) return undefined;
  const target = name.toLowerCase();
  for (const key of Object.keys(headers)) {
    if (key.toLowerCase() !== target) continue;
    const value = headers[key];
    if (typeof value === 'string') return value;
    if (Array.isArray(value)) {
      const first = value.find((v) => typeof v === 'string');
      if (typeof first === 'string') return first;
    }
  }
  return undefined;
}

/**
 * 人类可读的字节数文案（与后端 `human_size` 文案一致）
 *
 * @param bytes - 字节数
 */
export function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} bytes`;
}

/**
 * 清理文件名：只保留 basename，去掉路径穿越与非法字符
 *
 * @param name - 原始文件名
 * @param fallback - 清理后为空时的兜底名
 */
export function sanitizeFilename(name: string, fallback: string): string {
  const base = name.split(/[\\/]/).pop() ?? '';
  const cleaned = base
    // eslint-disable-next-line no-control-regex
    .replace(/[\u0000-\u001f]/g, '')
    .replace(/[<>:"|?*]/g, '')
    .replace(/\.\./g, '')
    .replace(/[. ]+$/, '')
    .trim()
    .slice(0, 120);
  return cleaned.length > 0 ? cleaned : fallback;
}

/** 从 Content-Disposition 中解析文件名 */
function filenameFromDisposition(disposition: string): string | undefined {
  const starMatch = /filename\*=(?:UTF-8'')?([^;]+)/i.exec(disposition);
  if (starMatch) {
    const raw = starMatch[1].trim().replace(/^"|"$/g, '');
    try {
      return decodeURIComponent(raw);
    } catch {
      // 编码损坏时退回普通 filename
    }
  }
  const plainMatch = /filename="?([^";]+)"?/i.exec(disposition);
  if (plainMatch) return plainMatch[1].trim();
  return undefined;
}

/** 从 URL 中解析 pathname 的 basename */
function basenameFromUrl(value: string): string | undefined {
  try {
    const pathname = new URL(value).pathname;
    const base = pathname.split('/').filter(Boolean).pop();
    return base ? decodeURIComponent(base) : undefined;
  } catch {
    return undefined;
  }
}

/** 视为视频的 URL 后缀 */
const VIDEO_EXTENSIONS = ['mp4', 'webm', 'mov', 'm4v', 'ogv'];

/** 从键名与 URL 后缀推断媒体大类 */
function kindForUrl(key: string, value: string): MediaKind {
  if (key.indexOf('video') >= 0) return 'video';
  if (key.indexOf('image') >= 0) return 'image';
  const base = basenameFromUrl(value);
  const match = base ? /\.([A-Za-z0-9]{1,5})$/.exec(base) : null;
  if (match && VIDEO_EXTENSIONS.indexOf(match[1].toLowerCase()) >= 0) return 'video';
  return 'image';
}

/**
 * 推断候选的扩展名（含点）
 *
 * @param candidate - 媒体候选
 * @param headers - 响应头（用于 Content-Disposition）
 */
export function extensionFor(
  candidate: MediaCandidate,
  headers: Record<string, unknown> | undefined
): string {
  const mime = candidate.mime.toLowerCase();
  const mapped = MIME_EXTENSIONS[mime];
  if (mapped) return `.${mapped}`;
  if (mime.startsWith('image/') || mime.startsWith('video/')) {
    const subtype = mime.slice(mime.indexOf('/') + 1).replace(/\+xml$/, '');
    if (/^[a-z0-9]+$/.test(subtype)) return `.${subtype}`;
  }

  const disposition = headerValue(headers, 'content-disposition');
  if (disposition) {
    const filename = filenameFromDisposition(disposition);
    const match = filename ? /\.([A-Za-z0-9]{1,5})$/.exec(filename) : null;
    if (match) return `.${match[1].toLowerCase()}`;
  }

  if (candidate.source?.type === 'url') {
    const base = basenameFromUrl(candidate.source.value);
    const match = base ? /\.([A-Za-z0-9]{1,5})$/.exec(base) : null;
    if (match && KNOWN_EXTENSIONS.indexOf(match[1].toLowerCase()) >= 0) {
      return `.${match[1].toLowerCase()}`;
    }
  }

  return '.bin';
}

/**
 * 生成默认保存文件名（含扩展名）
 *
 * @param candidate - 媒体候选
 * @param headers - 响应头（用于 Content-Disposition）
 * @param nodeName - 节点名，作为最后兜底
 */
export function buildDefaultFilename(
  candidate: MediaCandidate,
  headers: Record<string, unknown> | undefined,
  nodeName: string
): string {
  const disposition = headerValue(headers, 'content-disposition');
  let raw: string | undefined;
  if (disposition) raw = filenameFromDisposition(disposition);
  if (!raw && candidate.source?.type === 'url') raw = basenameFromUrl(candidate.source.value);
  if (!raw) raw = `${nodeName}-${Date.now()}`;

  const safe = sanitizeFilename(raw, `${nodeName}-${Date.now()}`);
  return /\.[A-Za-z0-9]{1,5}$/.test(safe) ? safe : `${safe}${extensionFor(candidate, headers)}`;
}

/** 嗅探媒体魔数，返回 MIME；无法识别时返回 null */
function sniffMime(bytes: Uint8Array): string | null {
  const startsWith = (prefix: number[], offset = 0): boolean =>
    prefix.every((b, i) => bytes[offset + i] === b);

  if (startsWith([0x89, 0x50, 0x4e, 0x47])) return 'image/png';
  if (startsWith([0xff, 0xd8, 0xff])) return 'image/jpeg';
  if (startsWith([0x47, 0x49, 0x46, 0x38])) return 'image/gif';
  if (startsWith([0x52, 0x49, 0x46, 0x46]) && startsWith([0x57, 0x45, 0x42, 0x50], 8)) {
    return 'image/webp';
  }
  if (startsWith([0x42, 0x4d])) return 'image/bmp';
  if (startsWith([0x1a, 0x45, 0xdf, 0xa3])) return 'video/webm';
  if (startsWith([0x66, 0x74, 0x79, 0x70], 4)) {
    const brand = String.fromCharCode(
      bytes[8] ?? 0,
      bytes[9] ?? 0,
      bytes[10] ?? 0,
      bytes[11] ?? 0
    );
    if (brand === 'avif' || brand === 'avis') return 'image/avif';
    return 'video/mp4';
  }
  return null;
}

/** 把 base64 载荷转换为候选；非法或非媒体时返回 null */
function candidateFromBase64(
  payload: unknown,
  declaredMime: unknown
): { mime: string; dataUrl: string } | null {
  if (typeof payload !== 'string') return null;
  // URL-safe base64（`-`/`_`）同样常见，先归一化再校验
  const cleaned = payload.replace(/\s+/g, '').replace(/-/g, '+').replace(/_/g, '/');
  if (cleaned.length < MIN_BASE64_LENGTH || cleaned.length > MAX_SCAN_STRING) return null;
  if (cleaned.length % 4 !== 0) return null;
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(cleaned)) return null;

  let head: Uint8Array;
  try {
    const binary = atob(cleaned.slice(0, 64));
    head = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      head[i] = binary.charCodeAt(i);
    }
  } catch {
    return null;
  }

  let mime = sniffMime(head);
  if (!mime) {
    const declared = typeof declaredMime === 'string' ? declaredMime.trim().toLowerCase() : '';
    if (declared.startsWith('image/') || declared.startsWith('video/')) {
      mime = declared;
    }
  }
  if (!mime) return null;

  return { mime, dataUrl: `data:${mime};base64,${cleaned}` };
}

/** 从后端媒体分流结果（V1）构造候选 */
function candidateFromBackendMedia(
  media: Record<string, unknown>,
  index: number
): MediaCandidate | null {
  const kind = media.kind;
  const mime = media.mime;
  if (kind !== 'image' && kind !== 'video') return null;
  if (typeof mime !== 'string' || mime.length === 0) return null;

  const bytes = typeof media.bytes === 'number' ? media.bytes : undefined;

  if (media.omitted === true) {
    return { id: `omitted:${index}`, kind, mime, bytes, omitted: true };
  }

  const dataUrl = media.data_url;
  if (typeof dataUrl === 'string' && dataUrl.startsWith('data:')) {
    return {
      id: `data_url:${index}`,
      kind,
      mime,
      bytes,
      source: { type: 'data_url', value: dataUrl },
    };
  }
  return null;
}

/**
 * 从节点输出中检测媒体候选
 *
 * @param data - 节点 `output_data`
 * @returns 按发现顺序排列的候选列表，最多 8 个
 */
export function detectMedia(data: unknown): MediaCandidate[] {
  const candidates: MediaCandidate[] = [];
  const seen = new Set<string>();

  const push = (candidate: MediaCandidate | null): void => {
    if (!candidate) return;
    if (candidates.length >= MAX_CANDIDATES) return;
    const key = candidate.source ? candidate.source.value.slice(0, 64) : candidate.id;
    if (seen.has(key)) return;
    seen.add(key);
    candidates.push(candidate);
  };

  // V1：后端媒体分流
  if (isPlainObject(data) && isPlainObject(data.media)) {
    push(candidateFromBackendMedia(data.media, candidates.length));
  }

  // V2/V3：扫描响应体
  const root = isPlainObject(data) && 'body' in data ? data.body : data;

  const walk = (node: unknown, depth: number): void => {
    if (depth > MAX_DEPTH || candidates.length >= MAX_CANDIDATES) return;
    if (Array.isArray(node)) {
      for (const item of node) walk(item, depth + 1);
      return;
    }
    if (!isPlainObject(node)) return;

    for (const [key, value] of Object.entries(node)) {
      const lower = key.toLowerCase();
      if (lower === 'media') continue;

      if (typeof value === 'string') {
        if (value.length > MAX_SCAN_STRING) continue;
        if (URL_KEYS.indexOf(lower) >= 0 && /^https?:\/\//.test(value)) {
          push({
            id: `url:${candidates.length}`,
            kind: kindForUrl(lower, value),
            mime: '',
            source: { type: 'url', value },
          });
          continue;
        }
        if (INLINE_KEYS.indexOf(lower) >= 0) {
          const found = candidateFromBase64(value, undefined);
          if (found) {
            push({
              id: `data_url:${candidates.length}`,
              kind: found.mime.startsWith('video/') ? 'video' : 'image',
              mime: found.mime,
              source: { type: 'data_url', value: found.dataUrl },
            });
          }
        }
        continue;
      }

      if (isPlainObject(value)) {
        if (INLINE_KEYS.indexOf(lower) >= 0) {
          const payload = value.data ?? value.base64 ?? value.b64_json;
          const declared = value.mimeType ?? value.mime_type;
          const found = candidateFromBase64(payload, declared);
          if (found) {
            push({
              id: `data_url:${candidates.length}`,
              kind: found.mime.startsWith('video/') ? 'video' : 'image',
              mime: found.mime,
              source: { type: 'data_url', value: found.dataUrl },
            });
          }
        }
        walk(value, depth + 1);
        continue;
      }

      if (Array.isArray(value)) {
        walk(value, depth + 1);
      }
    }
  };

  walk(root, 0);

  return candidates;
}
