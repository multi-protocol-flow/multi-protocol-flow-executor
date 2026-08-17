# 插件市场契约（Plugin Marketplace Contract）

本文档定义宿主（GUI 插件设置面板「插件市场」Tab + CLI `mpe plugin install`）与插件市场服务端之间的 HTTP 接口契约。**当前尚无真实市场服务端**，此文档与宿主代码类型（`flow-engine-plugin-marketplace/src/model.rs` ↔ `src/types/plugins.ts`）即为未来市场接口的对齐基准；真实接口上线后若字段名有差异，仅需修改 `model.rs`。

## 1. 插件列表

```
GET {base}/plugins
```

### 响应

`200 OK`，`Content-Type: application/json`：

```json
{
  "plugins": [
    {
      "name": "mongo",
      "version": "1.0.0",
      "description": "MongoDB 连接与 CRUD 节点",
      "author": "multi-protocol-flow",
      "minHostVersion": "0.1.0",
      "icon": "database",
      "categories": ["database"],
      "updatedAt": "2026-08-01",
      "platforms": {
        "windows-x64": { "url": "https://market.example.com/assets/mongo-1.0.0-windows-x64.zip", "sha256": "abc123...", "size": 1048576 },
        "linux-x64":   { "url": "https://market.example.com/assets/mongo-1.0.0-linux-x64.zip",   "sha256": "def456...", "size": 1048576 },
        "macos-arm64": { "url": "https://market.example.com/assets/mongo-1.0.0-macos-arm64.zip", "sha256": "789abc...", "size": 1048576 }
      }
    }
  ]
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | ✅ | 插件名（也是安装目录名，须与 zip 内 `plugin.json` 的 `name` 一致） |
| `version` | string | ✅ | 插件版本（semver） |
| `description` | string | - | 一句话描述 |
| `author` | string | - | 作者 |
| `minHostVersion` | string | - | 最低宿主版本（semver；缺省无门控） |
| `icon` | string | - | 图标标识（kebab-case，宿主 `resolveBackendIcon` 映射）或 URL |
| `categories` | string[] | - | 分类标签 |
| `updatedAt` | string | - | 最近更新时间（展示用，格式自由） |
| `platforms` | object | ✅ | 平台 key → 资产；平台 key 枚举见下 |

### 平台 key 枚举

`<os>-<arch>`，共 6 个：

```
windows-x64 | windows-arm64 | linux-x64 | linux-arm64 | macos-x64 | macos-arm64
```

### 资产对象

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `url` | string | ✅ | zip 归档下载地址（绝对 URL，可与市场同源或 CDN） |
| `sha256` | string | - | 归档字节的 sha256 十六进制摘要；**提供时宿主强制校验**，不匹配拒绝安装 |
| `size` | number | - | 归档大小（字节，展示用） |

### 排序约定

**同名多版本：服务端按 `version` 降序返回**。客户端取第一个匹配项；`mpe plugin install --version <v>` 时精确匹配。

## 2. 下载物（zip 归档）

- 归档根目录含一个与插件同名的一级目录，其中直接包含 `plugin.json` 与插件文件：

```
mongo.zip
└── mongo/
    ├── plugin.json
    └── mongo          # entry.command 对应文件（unix 平台安装时置 0o755）
```

- 宿主安装目标目录为 `install_dir/<name>/`，解压时自动剥离归档中冗余的 `<name>/` 根组件（归档直接平铺 `plugin.json` 亦可）。
- 归档内的条目路径**不得**为绝对路径、含 `..` 遍历或 Windows 盘符前缀——宿主拒绝此类归档；symlink 条目被跳过并告警。
- `plugin.json` 遵循 `mpe-plugin-sdk` 的 `PluginManifest` schema（`name`/`version`/`entry.command` 必填）：

```json
{
  "name": "mongo",
  "version": "1.0.0",
  "entry": { "command": "./mongo", "args": [] }
}
```

## 3. 客户端行为

- `MarketplaceClient::list_plugins`：`GET {base}/plugins`；非 2xx → `Http { status }`；JSON 不可解析或缺 `plugins` 字段 → `BadResponse`。
- `install_plugin`：平台门控（当前平台无资产 → `PlatformUnsupported`）→ 下载 → sha256 校验（可选）→ `install_dir/<name>/plugin.json` 已存在且非 `--force` → `AlreadyInstalled` → 解压（遍历防护）→ `plugin.json` 校验（名称一致 + `entry.command` 非空）→ unix 平台对 entry 文件置 0o755。任一步失败即清理已创建目录，不留半安装。

## 4. 地址配置

| 入口 | 配置方式 |
|---|---|
| GUI | 设置文件 `pluginRegistryUrl`（插件设置对话框「市场」Tab 保存并加载） |
| CLI | `mpe plugin install --registry <url>` 或环境变量 `MPE_PLUGIN_REGISTRY` |
