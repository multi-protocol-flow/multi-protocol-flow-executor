# 媒体预览与保存 — 设计与实现规范

**状态**：已实现（2026-11-04） | **影响面**：`flow-engine-http` / `flow-engine-core`（统计） / `flow-engine-executor`（导出） / 前端 viewer / Tauri CSP

> 落地时与本规范的三处偏差已在文中就地标注：占位文案改英文、V3 载荷长度阈值下调、IPC 直接传目标路径。

**相关**：循环与延时节点见 [`docs/flow-loop.md`](./flow-loop.md)、[`docs/flow-delay.md`](./flow-delay.md)

---

## 1. 背景与目标

### 1.1 背景

流程执行经常产生图片/视频：AI 生图（Gemini 3 Pro Image 返回 base64）、AI 生视频（Wan 3.0 返回媒体 URL）、直接下载媒体文件。当前报告只能看到 JSON 文本与 `"Invalid UTF-8: ..."` 乱码。

### 1.2 目标

| # | 目标 |
| :-- | :--- |
| 1 | 报告里直接看到响应中的图片/视频 |
| 2 | 用户手动保存媒体到本地（**不自动下载**） |
| 3 | 历史报告重新打开仍可查看（内容本就在报告里的场景） |
| 4 | 对现有 20 个 viewer、报告格式、`http:request` 配置**零影响** |

### 1.3 非目标

| 非目标 | 原因 |
| :--- | :--- |
| artifacts 概念 / artifacts 目录 | 需求只是"看响应内容"，不需要资产管理 |
| `asset:` 协议 + 运行时 scope | 本阶段不做本地文件重新打开 |
| 自动下载媒体 | 用户明确要求手动保存 |
| `http:request` 新增"保存响应"配置 | 保存是报告查看期动作，不属执行期 |
| 大二进制（>4MB）缓存 | 见 §10 已知缺口 |
| PDF / Office / 音频渲染 | 本阶段只做 image / video |

---

## 2. 现状约束（已验证）

| 事实 | 证据 | 影响 |
| :--- | :--- | :--- |
| 响应体被 `String::from_utf8` 转换 | `crates/flow-engine-http/src/http_executor.rs:923` | 二进制媒体当前已损坏，必须走后端分流 |
| `output_data` 含 `headers` | `http_executor.rs:975` | 前端可据 `content-type` 判定 |
| 压测不保留节点明细 | `stress_test/executor.rs:363/602/624`（`node_reports: Vec::new()`） | 无 N 轮累积风险 |
| 数据集不保留节点明细 | `dataset_executor/events.rs:799` | 同上 |
| `is_detail_stripped` 从未置 `true` | 全仓 grep 均为 `None` | 实时推送携带全量 `output_data`，见 §7.3 |
| 统计按 JSON 体积累加 | `flow-engine-core/src/statistics.rs:100` | 需排除 `media` |
| 保存对话框范式已存在 | `components/execution/ExecutionReport.tsx:186-201` | 直接复用 |

---

## 3. 展示设计

### 3.1 三种来源与检测分工

| # | 来源 | 检测层 | 判断依据 | 渲染 |
| :-- | :--- | :--- | :--- | :--- |
| **V1** | 响应体是媒体二进制 | **后端** | 响应头 `Content-Type` ∈ `image/*` \| `video/*` | `media.data_url` → `<img>` / `<video controls>` |
| **V2** | JSON 里带媒体 URL | **前端** | 字段名白名单 + 值为 `http(s)` | 直连 `<img>` / `<video>` |
| **V3** | JSON 里带 base64 | **前端** | 字段名白名单 + 解码后 magic 命中 | `data:` URL |

**为什么这样分工**：V1 的原始字节只有后端持有；V2/V3 的内容本来就在 `body` 里，前端检测**不产生任何重复存储**——若后端把 V3 也解析成 `data_url`，报告体积会翻倍（body 一份 + media 一份）。

### 3.2 V1：后端分流

```rust
// http_executor.rs —— 替换现有 `String::from_utf8` 那一段

const MEDIA_INLINE_MAX_BYTES: usize = 4 * 1024 * 1024;

let mime = headers_map
    .get("content-type")
    .map(|v| v.split(';').next().unwrap_or("").trim().to_ascii_lowercase())
    .unwrap_or_default();

let is_media = mime.starts_with("image/") || mime.starts_with("video/");
```

| 条件 | `output_data.media` | `output_data.body` |
| :--- | :--- | :--- |
| 非媒体 | 不出现该字段 | 与现状完全一致（文本 / JSON 解析） |
| 媒体，`len ≤ 4MB` | `{kind, mime, bytes, data_url}` | `"[image/png 662150 bytes]"` |
| 媒体，`len > 4MB` | `{kind, mime, bytes, omitted: true}` | `"[video/mp4 35.2 MB — exceeds 4 MB preview limit]"` |

实现落在 `crates/flow-engine-http/src/response_media.rs`（纯函数 `detect_media` + `human_size`），
`http_executor.rs` 只负责调用与拼装 `output_data.media`。**占位文案固定英文**：该字符串直接进入
`output_data.body`（响应体 / 原始 JSON 里可见），不走 i18n，中英界面混排更差；媒体区块自身的文案仍走 i18n。

契约：

```jsonc
"media": {
  "kind": "image" | "video",
  "mime": "image/png",
  "bytes": 662150,
  "data_url": "data:image/png;base64,…",   // omitted=true 时不存在
  "omitted": false                          // 可选，true 表示超限未保留
}
```

**非媒体响应 `output_data` 必须字节级不变**（不插入 `media: null`），避免影响既有报告格式与导出。

### 3.3 V2/V3：前端检测

```ts
// 白名单（字段名，大小写不敏感）
const URL_KEYS   = ['url', 'result_url', 'video_url', 'image_url', 'output_url'];
const BASE64_KEYS = ['inlinedata', 'inline_data', 'b64_json', 'base64', 'image'];
```

检测规则：

1. **V2**：递归遍历 `body`（深度上限 8、跳过 `media` 自身），命中 `URL_KEYS` 且值为 `http(s)://` → 候选。字段名含 `video` → `<video>`，含 `image` → `<img>`；两者都不含时看 URL 后缀（`mp4`/`webm`/`mov`/`m4v`/`ogv` → `<video>`，其余 `<img>`）。
2. **V2 降级**：`<img>`/`<video>` 的 `onError` → 渲染为链接卡片（"无法加载，可能已过期" + 完整链接 + 复制）。
3. **V3**：命中 `BASE64_KEYS` 且字符串长度 ≥ 32 字符、base64 解码成功、解码后 magic 命中 `image`/`video` → 生成 `data:` URL；magic 未命中时回退到同层的 `mimeType`/`mime_type` 声明，仍不命中则丢弃。
   **长度阈值说明**：规范初稿要求 > 4KB，但真实迷你媒体会被误杀（1×1 PNG 的 base64 仅 124 字符），
   而 magic 嗅探本身已是足够强的判别器，故阈值下调为 32 字符，仅用于挡掉明显非载荷的短串。
4. 不做 HEAD 探测，不扫描任意字符串（避免把日志路径误判为媒体）。

`inlineData` 的典型结构（Gemini）：

```jsonc
{ "candidates": [{ "content": { "parts": [
  { "inlineData": { "mimeType": "image/png", "data": "iVBORw0KGgo…" } }
]}}]}
```

### 3.4 展示位置

```
HttpNodeViewer / GenericViewer
├─ 请求信息
├─ 响应状态
├─ 重定向历史
├─ 响应头
├─ 响应 Cookie
├─ 媒体            ← 新增；defaultOpen；无媒体时返回 null
│    ├─ 卡片 1     [保存] [复制链接]
│    ├─ 卡片 2
│    └─ 超过 4 个 → 折叠为“展开全部 (N)”
├─ 响应体           ← extra 增加「保存原始响应」
└─ 耗时分解
```

- 组件：`src/components/execution/viewers/MediaPreview.tsx`（受控、无副作用，除保存动作）。
- 渲染按 `kind` 分流（而非按 `source.type`）：`image` → `<img>`，`video` → `<video controls>`；
  这样 V3 嗅探出的 `video/webm` data URL 也能播放，而不是被塞进 `<img>` 触发降级。
- 接入：`HttpNodeViewer.tsx` 与 `GenericViewer.tsx` 各插入一次；其余 viewer 不动。
- 无媒体 → `return null`，现有 UI 零视觉变化。

---

## 4. 保存设计

### 4.1 两个动作

| 动作 | 位置 | 保存内容 |
| :--- | :--- | :--- |
| **保存** | 媒体卡片右上 | 该媒体（V2 走宿主下载；V1/V3 解码后写盘） |
| **保存原始响应** | 响应体 section 的 `extra`（与现有 `CopyButton` 并排） | `output_data.body` 原样 |

### 4.2 文件名与扩展名

| 场景 | 顺序 |
| :--- | :--- |
| 扩展名 | ① `candidate.mime`（V1 来自响应 `Content-Type`，V3 来自 magic bytes）② `Content-Disposition` 的文件名后缀 ③ URL pathname 的已知媒体后缀 ④ `.bin` |
| 文件名 | ① `Content-Disposition`（`filename*=UTF-8''…` 优先，回退 `filename=`）② V2 的 URL basename ③ `<节点名>-<时间戳>` |

**消毒**：只取 basename，移除 `..`、`/`、`\`、控制字符与 Windows 非法字符，长度截断 120；同名存在时追加 `-1`。

### 4.3 通道

| 环境 | 实现 |
| :--- | :--- |
| Tauri | `save({ defaultPath, filters })` → IPC `save_response_media` → 宿主写盘 → toast + `revealItemInDir` |
| 浏览器（playground） | Blob + `<a download>`（沿用 `ExecutionReport.tsx:202-213`） |

IPC（实现与初稿的差异：路径由前端对话框先取得并直接传入，用户取消在前端 `return`，因此返回值为写盘后的最终路径而非 `Option`）：

```rust
#[tauri::command]
async fn save_response_media(
    source: MediaSaveSource,   // { Url(url) } | { DataUrl(data_url) } | { Body(body) }
    file_path: String,         // 用户已选定的目标路径
) -> Result<String, String>;   // 返回最终路径（同名自动去重为 stem-1.ext）
```

前端封装：`src/services/tauri/media.ts` 的 `saveResponseMedia(source, filePath)`；
保存编排集中在 `src/components/execution/viewers/mediaSave.ts`（含浏览器 Blob 兜底）。
`Url` 分支额外有 512 MB 硬上限，防止误点超大直链把内存撑爆。

- `Url` 分支由**宿主侧 reqwest 下载**（webview `fetch` 会被 CORS 阻塞，Wan 的 CDN 通常不返回 CORS 头）。
- 写盘使用 `std::fs::write`，路径来自用户对话框，无需额外 scope。

---

## 5. 失败态

| 情况 | 表现 |
| :--- | :--- |
| V2 链接失效（Wan `result_url` 约 24h 过期） | 链接卡片："无法加载，可能已过期" + 复制链接；保存按钮禁用 |
| V1 超过 4MB（`omitted: true`） | "响应 35.2 MB，超过预览上限，未保留内容" |
| 无媒体 | 不渲染 |
| 保存失败（网络 / 磁盘） | toast 报错，含错误信息 |

---

## 6. 连带影响

### 6.1 统计口径

`statistics.rs:100` 的 `estimate_json_size(&output_data)` 需跳过 `media` 键——它是预览载荷，不是业务数据，否则输出体积统计会跳一个数量级。

### 6.2 导出 HTML 报告

内联媒体（`data_url`）在导出时替换为占位 `[media omitted]`；V2 的 URL 原样保留（在线可看）。避免导出成几十 MB 的单文件。

### 6.3 实时执行推送

`is_detail_stripped` 从未被置 `true`，`NodeCompleted`（`executor.rs:1049/1199/1293`）会携带完整 `output_data`。V1 的 4MB 媒体 = 5.3MB base64 一次跨 IPC。

**结论**：本阶段接受（媒体节点通常一次执行推送一次）。若实测卡顿，再实现按需拉取。

### 6.4 压测 / 数据集

两者均不保留节点明细，媒体不会累积。无需额外处理。

### 6.5 CSP

```jsonc
"img-src":   "'self' data: http: https:",
"media-src": "'self' data: http: https:"   // 新增该键，此前 video 一律不放行
```

`script-src` 维持 `'self'` 不变。`http:` 一并放行：`connect-src` 本就允许 `http:`，而流程里常见的
本地/内网媒体服务（`http://127.0.0.1:…`、局域网 CDN）若被拦下，会以「无法加载」误导用户。

---

## 7. 改动清单

| # | 改动 | 文件 |
| :-- | :--- | :--- |
| 1 | 媒体类型分流 + `output_data.media` | `crates/flow-engine-http/src/http_executor.rs` |
| 2 | 统计跳过 `media` | `crates/flow-engine-core/src/statistics.rs` |
| 3 | 导出 HTML 替换内联媒体 | `crates/flow-engine-executor/src/report_export/` |
| 4 | CSP 放宽 | `src-tauri/tauri.conf.json` |
| 5 | `MediaPreview` 组件 | `src/components/execution/viewers/MediaPreview.tsx` |
| 6 | 接入两个 viewer | `HttpNodeViewer.tsx`、`GenericViewer.tsx` |
| 7 | IPC `save_response_media` | `src-tauri/src/tauri_commands.rs` + `src/services/tauri/` |
| 8 | i18n 三处同步 | `zh-CN` / `en-US` / `i18n-types.ts` |

i18n 实际新增的键（`execution.viewers.*`）：`media`、`media_save`、`media_copy_link`、`media_expired`、
`media_omitted`、`media_show_all`、`media_show_less`、`save_raw_body`；保存成功/失败复用既有的
`execution.toast.save_success` / `save_failed`，不新增同义键。`i18n-types.ts` 由 `npm run i18n:generate` 生成，不手改。

---

## 8. 验收

| # | 场景 | 预期 |
| :-- | :--- | :--- |
| 1 | Wan 提交响应（JSON 含 `result_url`） | 出播放器；保存得到 `.mp4` |
| 2 | Gemini 响应（`inlineData` base64） | 出图；**重启后重开报告仍是图** |
| 3 | `GET` 一个 `image/png` | 出图；保存得到 `.png` |
| 4 | 无媒体响应 | viewer 与改动前视觉一致（逐节点对照） |
| 5 | `Content-Disposition: attachment; filename="../../x.png"` | 文件名为 `x.png`，不越界 |
| 6 | `result_url` 指向已失效链接 | 显示链接卡片与"可能已过期" |
| 7 | 20MB 的 mp4 响应 | 显示超限占位，UI 不卡死 |
| 8 | 压测 / 数据集执行 | 无体积异常、无累积 |
| 9 | 导出 HTML | 无内联 base64，文件大小正常 |

---

## 9. 实施顺序

1. 后端 V1 分流 + 统计排除（跑通 `cargo nextest -p flow-engine-http`、`-p flow-engine-core`）
2. CSP + `MediaPreview`（先只做 V1，用一个本地图片响应端到端验证）
3. 加 V2/V3 检测与降级
4. 保存通道（媒体 + 原始响应）与浏览器兜底
5. i18n 与两个 viewer 接入
6. 验收 §8 全表

---

## 10. 已知缺口

| 缺口 | 说明 |
| :--- | :--- |
| >4MB 二进制媒体 | 只显示大小，无法预览与保存。后续方案：写系统临时目录 + 白名单读取，待定 |
| 音频 / PDF | 未支持 |
| 实时推送体积 | `is_detail_stripped` 未实现，大媒体随实时事件推送；实测卡顿再优化 |
| V2 无扩展名 URL | 仅凭字段名与 URL 后缀推断类型，可能把视频当图片渲染，由 `onError` 降级兜底 |
| V2 的远端直链 | 浏览器直接向源站拉取媒体，会向该主机暴露查看者 IP；`http:` 已在 CSP 中放行，失败统一降级为链接卡片 |
| 导出里的 `$report` 副本 | 执行器把整份报告镜像进 `final_variables.$report`，导出时与 `node_reports` 一并剥离内联媒体 |
