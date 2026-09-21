# 插件前端主题契约（Plugin Frontend Theme Contract）

> 状态：已定稿，随宿主 + `plugins/{agent,grpc,llm}` 一并落地。
> 关联：`docs/plugin-architecture.md` §9（前端 Iframe 桥接）。

---

## 1. 问题

自带独立前端页面的插件（`agent` / `grpc` / `llm`）通过沙箱 iframe 加载单文件 HTML。
沙箱为**不透明源**（`sandbox="allow-scripts"`，无 `allow-same-origin`），因此：

- 宿主无法向 iframe 注入 CSS；
- 宿主 `<html>` 上的 `.dark` class 不会跨文档继承；
- 唯一通道是 postMessage。

而改造前：

1. `init` 载荷只携带 `locale`，**没有 theme** —— 宿主主题对插件不可见；
2. 三个插件的 `styles.css` 一律只用 `@media (prefers-color-scheme: dark)`，
   即跟随**操作系统**；
3. 宿主从未调用 Tauri 的 `window.setTheme()`，且 `tauri.conf.json` 未配置 theme，
   所以 webview 的 `prefers-color-scheme` 就是系统设置；
4. 插件样式表**一个都没声明 `color-scheme`**。

后果：只要用户在设置里选了与系统不同的主题，面板配色与宿主不一致。更糟的是，
宿主在 `<html>` 上设置了 `color-scheme`，该属性可继承到 `<iframe>` 元素并影响被嵌入
文档的初始 color-scheme —— 于是 iframe 内**同时存在两个互不相干的主题来源**：
原生控件由宿主 color-scheme 决定，插件自己的 CSS 变量由系统媒体查询决定。两者打架时，
原生控件（`<select>` 弹出列表 / `<option>`、checkbox、range、滚动条）背景由 UA 按
浅色绘制（白），文字色却继承深色板的近白色 `--fg` —— 即**白底白字**。

## 2. 目标与非目标

**目标**：插件面板/查看器的明暗模式**严格跟随宿主**，且切换主题时实时生效。

**非目标**：

- **不同步设计令牌**。三个插件各有品牌色（agent 紫 / grpc 紫罗兰 / llm 翠绿），
  这是有意为之。宿主只下发**模式**，不下发颜色变量。
- **不在插件里实现 `system` 解析**。`system → light|dark` 的解析是宿主独有的职责，
  避免同一套逻辑在宿主与各插件里重复实现后漂移。

## 3. 契约

### 3.1 `init` 载荷新增 `theme`

```ts
export interface PluginIframeInitPayload {
  config: unknown;
  variables: Record<string, unknown>;
  host_api: PluginIframeHostApi;
  nodeType: string;
  locale: string;
  /**
   * 宿主**已解析**的主题（light/dark）。
   * 可选：旧宿主不携带；插件必须回退到 `prefers-color-scheme`。
   */
  theme?: 'light' | 'dark';
  nodes?: PluginIframeNodeSnapshot[];
}
```

**语义约束**：只传 resolved 值，**不传 `system`**。宿主是 `system` 解析的唯一事实来源。

### 3.2 新增宿主 → iframe 消息 `themeChanged`

```ts
export interface PluginIframeThemeChangedMessage {
  type: 'themeChanged';
  payload: { theme: 'light' | 'dark' };
}
```

宿主在 `resolvedTheme` 变化时推送给已就绪的 iframe，使已打开的面板无需重载即可换肤。

> 不复用 `configUpdated`：语义不同，且会覆盖用户正在编辑的表单配置。

### 3.3 消息总览（改造后）

**宿主 → iframe**

| type | 场景 |
|---|---|
| `init` | 初始 config + 变量快照 + host_api + nodeType + locale + **theme** + nodes |
| `configUpdated` | 宿主侧配置变更推送 |
| `actionResult` | 响应 `requestAction` |
| `themeChanged` | **新增**：宿主明暗模式变化 |

**iframe → 宿主**

| type | 场景 |
|---|---|
| `ready` | 初始化完成 |
| `configChanged` | 表单变更 → 宿主保存 |
| `requestAction` | 请求宿主能力 |
| `resize` | 自适应高度 |
| `error` | 面板内部错误 |

### 3.4 兼容规则

| 场景 | 行为 |
|---|---|
| 新宿主 + 新插件 | `init.theme` 或 `themeChanged` 生效，属性驱动 |
| 新宿主 + 旧插件（已发布 dist） | 旧插件忽略未知字段/消息；宿主额外通过 `window.setTheme()` 让 webview 的 `prefers-color-scheme` 等于宿主主题 → 旧插件的媒体查询自动跟随 |
| 旧宿主 + 新插件 | `init.theme` 缺失 → 插件回退 `prefers-color-scheme`（与改造前行为一致） |
| 浏览器独立调试插件页 | 无宿主 → 回退 `prefers-color-scheme` |

**因此新增字段必须是可选的，新增消息必须是可忽略的** —— 不引入握手/能力协商，
不引入协议版本号。插件对未知 `type` 静默忽略即可。

## 4. 插件实现规范

### 4.1 JS：把 theme 落到 `documentElement`

```ts
type Theme = 'light' | 'dark';

let hostTheme: Theme | null = null; // 宿主下发值；null = 未下发

function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  root.dataset.theme = theme;      // CSS 权威来源
  root.style.colorScheme = theme;  // 原生控件（滚动条/下拉/checkbox）
}

// init 到达时
if (initPayload.theme === 'light' || initPayload.theme === 'dark') {
  hostTheme = initPayload.theme;
  applyTheme(hostTheme);
}

// themeChanged 到达时
case 'themeChanged':
  if (payload.theme === 'light' || payload.theme === 'dark') {
    hostTheme = payload.theme;
    applyTheme(hostTheme);
  }
  break;
```

**不要**在插件里读 `ThemeMode` 的 `system`，也不要自己写 `system` 分支 —— 宿主已解析。

### 4.2 CSS：三块结构

```css
/* 1) 基础：浅色（也是 data-theme="light" 的落点） */
:root {
  color-scheme: light;
  --bg: #ffffff;
  --bg-input: #ffffff;
  --fg: #0f172a;
  /* ... 其余浅色变量 ... */
}

/* 2) 兜底：宿主未下发 theme 时跟随系统（旧宿主 / 浏览器独立调试）
      —— 保证 init 到达前不闪白（面板在暗色系统下首帧即暗色） */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    color-scheme: dark;
    /* ... 深色变量（与第 3 块保持一致） ... */
  }
}

/* 3) 权威：宿主下发的 resolved theme 覆盖一切 */
:root[data-theme="dark"] {
  color-scheme: dark;
  /* ... 深色变量（与第 2 块保持一致） ... */
}
```

**要点**

- 第 2、3 块的深色变量**必须逐条一致**。CSS 无法在两条规则间共享变量块，
  这是本契约接受的重复；两块必须相邻放置，改一处必须同步另一处。
- 第 2 块用 `:not([data-theme])` 限定，保证第 3 块生效时不被媒体查询干扰。
  两块选择器互斥，无需依赖优先级/顺序。
- `color-scheme` 是**原生控件换肤的正解**：声明后 UA 会自动把 `<select>` 弹出列表、
  `<option>`、checkbox/radio、range、滚动条按对应模式绘制。这是修「白底白字」的关键，
  而不是逐个手写控件样式。
- 仍需显式声明的部分（各引擎对原生下拉的实现不一致，只靠 `color-scheme` 不够）：

  ```css
  option {
    background: var(--bg-input); /* grpc 用 var(--bg)，跟随该插件表单底色 */
    color: var(--fg);
  }
  ```

  滚动条与 `<input type="range">` 本次交由 `color-scheme` 覆盖，未额外声明
  （llm 的 range 本就有 `accent-color`）。
- 三个插件的表单样式约定不同，改造时按现状各自处理：
  - `agent`：**元素选择器**（`input[type=text|password|number], select, textarea`）
  - `llm` / `grpc`：**类名**（`.input` / `.select` / `.textarea`）

## 5. 宿主实现清单

| 文件 | 改动 |
|---|---|
| `src/types/plugin-iframe.ts` | `PluginIframeInitPayload` 加 `theme?`；新增 `themeChanged` 消息类型；`isPluginIframeMessage` 登记并校验；新增 `createThemeChangedMessage` |
| `src/hooks/usePluginIframe.ts` | options 加 `theme`；`init` 携带 theme；theme 变化且已 ready → 推 `themeChanged` |
| `src/components/plugin/PluginIframe.tsx` | 加 `theme` prop 并透传 |
| `src/components/panels/NodeConfigPanel.tsx` | 从 `useTheme()` 取 `resolvedTheme` 传入 |
| `src/components/execution/viewers/PluginIframeViewer.tsx` | **第二套手写桥**：`init` 携带 theme，主题变化推 `themeChanged` |
| `src/hooks/useTheme.ts` | resolvedTheme 变化时调 Tauri `getCurrentWindow().setTheme(resolved)`（`system` → `null`），作为存量插件的兼容杠杆 |

> 宿主存在**两套** iframe 桥：配置面板走 `usePluginIframe`，查看器走
> `PluginIframeViewer` 内联的 `PluginReportIframe`。两处都必须改。

## 6. 插件改造清单

对 `plugins/{agent,grpc,llm}` 各执行：

1. `frontend/src/types.ts` —— `PluginIframeInitPayload` 加 `theme?: 'light' | 'dark'`；
   `PluginStreamMessage` 同级新增 `themeChanged` 入站消息类型（如该文件已定义消息联合）。
2. `frontend/src/bridge.ts` —— `init` 读取 `theme` 并 `applyTheme`；
   `switch (data.type)` 增加 `case 'themeChanged'`。
3. `frontend/src/styles.css` —— 按 §4.2 改为三块结构；补 `option` / 滚动条 / range 样式。
4. `npm run build` 重新生成 `dist/panel.html` 与 `dist/viewer.html`（单文件内联）。
5. 提交并发布（各插件为独立仓库，各自 CI 出 Release 产物）。

## 7. 验证要点

- 宿主设置：**浅色 / 暗黑 / 跟随系统** 三态 × 系统明暗两种，共 6 种组合，
  逐一确认插件面板与查看器与宿主一致。
- 面板打开状态下切换宿主主题 → 面板应**即时**换肤（不重载、不丢表单状态）。
- 暗色下检查原生控件：`<select>` 展开的选项列表、checkbox、range、滚动条，
  确认无白底白字。
- 旧版插件（未改造的 dist）在新宿主下的表现：依赖 `window.setTheme()` 杠杆，
  需在 **Linux / macOS / Windows** 三平台实测 `setTheme` 是否影响
  `prefers-color-scheme`（Linux 上 GTK 主题与 webkit 设置是两回事，最可能失效）。
  若某平台失效，该平台存量插件维持改造前的表现，不阻塞本次改动。

## 8. 后续（不在本次范围）

iframe postMessage 契约目前是**宿主单方所有 + 每个插件手抄一份**：三个插件的
`bridge.ts` 各自手写、互不共享，`frontend/package.json` 无任何共享依赖，
契约文档只在宿主仓库。本次以「加一个可选字段 + 一个可忽略消息」的代价接受重复。

若后续契约继续演进，建议从 SDK 仓库发布 `@mpe/plugin-frontend`
（bridge + 主题处理 + 基础 CSS），构建期内联进单文件 HTML（不破坏自包含），
把 N 处手抄收敛为 1 处。
