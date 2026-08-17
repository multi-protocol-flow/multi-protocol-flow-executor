/**
 * 插件设置与插件市场相关类型
 *
 * 与 Rust 侧 serde 输出严格对齐（camelCase）：
 * - `PluginDirInfo` / `InstalledPlugin` ← `src-tauri/src/tauri_commands.rs`
 * - `MarketplacePlugin` ← `flow-engine-plugin-marketplace/src/model.rs`
 */

// ============================================================================
// 插件目录信息
// ============================================================================

/** 插件目录来源 */
export type PluginDirSource = 'env' | 'config' | 'default';

/** 当前生效的插件目录信息（`get_plugin_dir_info` 返回） */
export interface PluginDirInfo {
  /** 当前生效的插件目录（绝对路径字符串） */
  effective_dir: string;
  /** 目录来源 */
  source: PluginDirSource;
  /** 设置文件中的 `pluginDir` 值（未配置则为 null） */
  configured: string | null;
}

// ============================================================================
// 已安装插件
// ============================================================================

/** 已安装插件条目（`list_installed_plugins` 返回） */
export interface InstalledPlugin {
  /** 插件名 */
  name: string;
  /** 插件版本 */
  version: string;
  /** 描述 */
  description: string | null;
  /** 安装目录 */
  path: string;
  /** 状态：`loaded`（已登记进注册表）| `error` */
  status: 'loaded' | 'error';
  /** 错误/跳过原因 */
  error: string | null;
}

// ============================================================================
// 插件市场
// ============================================================================

/** 某平台的下载资产（zip 归档） */
export interface MarketplacePlatformAsset {
  /** 归档下载 URL */
  url: string;
  /** 归档 sha256 摘要（十六进制；可选） */
  sha256?: string | null;
  /** 归档大小（字节；可选） */
  size?: number | null;
}

/** 市场插件条目（`GET {base}/plugins` 响应，契约见 docs/plugin-marketplace.md） */
export interface MarketplacePlugin {
  /** 插件名 */
  name: string;
  /** 插件版本（semver） */
  version: string;
  /** 描述 */
  description?: string | null;
  /** 作者 */
  author?: string | null;
  /** 最低宿主版本（semver；缺省无门控） */
  min_host_version?: string | null;
  /** 图标标识或 URL */
  icon?: string | null;
  /** 分类标签 */
  categories?: string[];
  /** 最近更新时间（展示用） */
  updated_at?: string | null;
  /** 各平台下载资产（key 为 `windows-x64|windows-arm64|linux-x64|linux-arm64|macos-x64|macos-arm64`） */
  platforms: Record<string, MarketplacePlatformAsset>;
}
