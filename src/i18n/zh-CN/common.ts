import type { BaseTranslation } from '../i18n-types';

const zh_CN_common: Partial<BaseTranslation> = {
  APP_NAME: '多协议流程执行器',
  CANCEL: '取消',
  CONFIRM: '确认',
  SAVE: '保存',
  DELETE: '删除',
  save_success: '保存成功',
  delete_success: '删除成功',
  zoom_in: '放大 (Ctrl+=)',
  zoom_out: '缩小 (Ctrl+-)',
  zoom_reset: '重置缩放 (Ctrl+0)',
  settings_label: '设置',
  toggle_sidebar_collapse: '收起侧边栏',

  toggle_sidebar_expand: '展开侧边栏',
  toggle_config_collapse: '收起配置面板',
  toggle_config_expand: '展开配置面板',
  drag_resize_width: '拖拽调整面板宽度',
  drag_resize_height: '拖拽调整面板高度',
  clear: '清空',
  /** 未配置 */
  not_configured: '未配置',
  or: '或',
  space_drag: 'Space + 拖拽',
  ctrl_scroll: 'Ctrl+滚轮',
  ctrl_zoom: 'Ctrl + - / =',
  collapse_panel_aria: '折叠{title}',
  expand_panel_aria: '展开{title}',
  flow_initial_variables: '流程初始变量',
  environment_variable: '环境变量 ({name})',
  label_flow_variables: '流程变量',
  label_environment_variables: '环境变量',
  node_copy_suffix: ' (副本)',
  label_browser_storage: '浏览器本地存储 (localStorage)',
  label_test_connection: '测试连接',
  csv_validation_failed: 'CSV 文件验证失败',
  csv_validation_error: 'CSV 验证失败',
  variable_name_example: '变量名',
};

export default zh_CN_common;
