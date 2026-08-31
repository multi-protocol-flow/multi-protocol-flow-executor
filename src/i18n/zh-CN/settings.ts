import type { BaseTranslation } from '../i18n-types';

const zh_CN_settings: BaseTranslation = {
  language: '语言',
  theme: '主题',
  reports_dir: '报告目录',
  zh_CN: '简体中文',
  en_US: 'English',
  panel_title: '全局设置',
  config_file: '配置文件',
  loading: '加载中...',
  unknown: '未知',
  select_theme: '选择主题',
  theme_light: '浅色',
  theme_dark: '暗黑',
  theme_system: '跟随系统',
  flows_dir: '流程保存目录',
  logs_dir: '日志目录',
  not_set: '未设置',
  select_dir: '选择目录',
  open_dir: '打开目录',
  default_timeout: '节点默认执行超时 (ms)',
  default_timeout_tip: '当节点、插件或数据源自身未显式指定超时时间时，统一采用此全局时长作为超时兜底（默认 30000ms）。',
  plugin_max_concurrency: '插件最大并发数',
  plugin_max_concurrency_tip: '单个插件子进程同时处理的最大请求数（信号量配额，默认 256，高并发压测可调大至 1024+）。',
  plugin_queue_capacity: '插件通信队列容量',
  plugin_queue_capacity_tip: '宿主与插件标准输入输出通信帧队列深度（默认 2048）。',
};

export default zh_CN_settings;
