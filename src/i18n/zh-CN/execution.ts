import type { BaseTranslation } from '../i18n-types';

const zh_CN_execution: BaseTranslation = {
  node_types: {
    entry: '入口',
    http: 'HTTP',
    conditional: '条件',
    script: '脚本',
    assertion: '断言',
    end: '终止',
    ws_connect: 'WS-连接',
    ws_send_collect: 'WS-发送采集',
    ws_close: 'WS-关闭',
    db_connect: 'DB-连接',
    db_query: 'DB-查询',
    db_begin: 'DB-开始事务',
    db_commit: 'DB-提交',
    db_rollback: 'DB-回滚',
    db_close: 'DB-关闭',
  },
  assertion: {
    operators: {
      equal: '等于',
      not_equal: '不等于',
      greater: '大于',
      greater_or_equal: '大于等于',
      less: '小于',
      less_or_equal: '小于等于',
      contains: '包含',
      not_contains: '不包含',
      starts_with: '以...开始',
      ends_with: '以...结束',
      exists: '存在',
      not_exists: '不存在',
      is_empty: '为空',
      is_not_empty: '不为空',
      matches_regex: '匹配正则',
    },
    modes: {
      all: '全部通过 (AND)',
      any: '任一通过 (OR)',
    },
    failure_modes: {
      route_false: '路由到 false 分支',
      throw: '抛出异常终止流程',
    },
    presets: {
      http_status: 'HTTP 状态码检查',
      http_success: 'HTTP 成功响应',
      response_body_exists: '响应体存在',
      json_field_exists: 'JSON 字段存在',
      empty_check: '非空检查',
    },
    descriptions: {
      status_200: '状态码为 200',
      status_ge_200: '状态码 >= 200',
      status_lt_300: '状态码 < 300',
      body_exists: '响应体存在',
      data_field_exists: 'data 字段存在',
      body_not_empty: '响应体不为空',
    },
  },
  status: {
    success: '成功',
    failed: '失败'
  }
};

export default zh_CN_execution;