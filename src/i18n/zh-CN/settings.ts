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
  update: {
    title: '更新',
    check_updates: '检查更新',
    current_version: '当前版本',
    checking: '正在检查更新...',
    update_available: '发现新版本 {version}',
    up_to_date: '已是最新版本',
    download_and_install: '下载并安装',
    later: '稍后提醒',
    skip_version: '跳过此版本',
    downloading: '正在下载... ({progress}%)',
    update_error: '更新失败: {error}',
    release_notes: '更新说明',
    released_at: '发布于 {date}',
  },
};

export default zh_CN_settings;
