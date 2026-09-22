import type { BaseTranslation } from '../i18n-types';

const zh_CN_stages: BaseTranslation = {
  tab_setup: '准备阶段',
  tab_main: '核心流程',
  tab_teardown: '收尾阶段',
  short_setup: '准备',
  short_main: '核心',
  short_teardown: '收尾',
  hint_setup: '测试前执行 1 次：用于登录预热、变量初始化与资源准备',
  hint_main: '核心业务流程：单次运行、压测并发循环或数据集逐行消费',
  hint_teardown: '测试后执行 1 次：可通过 $report 变量获取本次执行统计',
  node_count: '{count} 个节点',
  status_done: '已完成',
  status_paused: '暂停在断点',
  status_waiting: '等待中',
  status_running: '执行中',
  back_to_paused: '暂停在{stage}：{node}',
  back_to_paused_action: '点击定位',
  report_setup: '准备阶段 (Setup)',
  report_main: '核心流程 (Main)',
  report_teardown: '收尾阶段 (Teardown)',
  report_lifecycle: '生命周期执行记录 (Setup & Teardown)',
  not_configured: '未配置',
  no_nodes: '无节点执行记录',
  skipped: '已跳过',
  view_in_canvas: '在画布中查看',
  duration: '耗时',
  unlinked_warning: '建议连线指明顺序',
  unlinked_warning_tooltip: '检测到多个未连线的独立节点。建议通过连线明确先后顺序；未连线节点将按画布位置自上而下顺序执行',
};

export default zh_CN_stages;
