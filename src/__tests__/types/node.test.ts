/**
 * 类型守卫函数测试
 * 验证 NodeData 判别联合类型的运行时类型检查
 */

import { describe, it, expect } from 'vitest';
import {
  isEntryConfig,
  isHttpConfig,
  isConditionalConfig,
  isWsConnectConfig,
  isWsSendCollectConfig,
  isWsCloseConfig,
  isAssertionConfig,
  isDbConnectConfig,
  isDbQueryConfig,
  isDbBeginConfig,
  isDbCommitConfig,
  isDbRollbackConfig,
  isDbCloseConfig,
  isScriptConfig,
} from '../../types/node';
import type { NodeData } from '../../types/node';

/**
 * 测试数据工厂：创建各类型节点的配置数据
 */
function createNodeData(type: NodeData['type']): NodeData {
  switch (type) {
    case 'entry':
      return { type: 'entry' };
    case 'http':
      return { type: 'http', url: 'https://example.com' };
    case 'conditional':
      return { type: 'conditional' };
    case 'ws_connect':
      return { type: 'ws_connect', url: 'wss://example.com' };
    case 'ws_send_collect':
      return {
        type: 'ws_send_collect',
        connection_id: 'conn-1',
        collect_response: true,
        message: 'test message',
      };
    case 'ws_close':
      return { type: 'ws_close', connection_id: 'conn-1' };
    case 'assertion':
      return {
        type: 'assertion',
        assertions: [],
        mode: 'all',
        on_failure: 'route_false',
        continue_on_failure: false,
      };
    case 'db_connect':
      return { type: 'db_connect', driver: 'postgres', host: 'localhost', port: 5432, database: 'testdb' };
    case 'db_query':
      return { type: 'db_query', connection_uuid: 'db-conn-1' };
    case 'db_begin':
      return { type: 'db_begin', connection_uuid: 'db-conn-1' };
    case 'db_commit':
      return { type: 'db_commit', connection_uuid: 'db-conn-1' };
    case 'db_rollback':
      return { type: 'db_rollback', connection_uuid: 'db-conn-1' };
    case 'db_close':
      return { type: 'db_close', connection_uuid: 'db-conn-1' };
    case 'script':
      return { type: 'script', script: '// test script' };
    case 'grpc_connect':
      return { type: 'grpc_connect', url: 'http://localhost:50051' };
    case 'grpc_call':
      return { type: 'grpc_call', connection_id: 'grpc-conn-1', service_name: 'TestService', method_name: 'TestMethod', request_json: '{}' };
    case 'grpc_close':
      return { type: 'grpc_close', connection_id: 'grpc-conn-1' };
  }
}

describe('isEntryConfig', () => {
  it('应该对 entry 类型返回 true', () => {
    const data = createNodeData('entry');
    expect(isEntryConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'http',
      'conditional',
      'ws_connect',
      'ws_send_collect',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isEntryConfig(data)).toBe(false);
    });
  });
});

describe('isHttpConfig', () => {
  it('应该对 http 类型返回 true', () => {
    const data = createNodeData('http');
    expect(isHttpConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'conditional',
      'ws_connect',
      'ws_send_collect',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isHttpConfig(data)).toBe(false);
    });
  });
});

describe('isConditionalConfig', () => {
  it('应该对 conditional 类型返回 true', () => {
    const data = createNodeData('conditional');
    expect(isConditionalConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http',
      'ws_connect',
      'ws_send_collect',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isConditionalConfig(data)).toBe(false);
    });
  });
});

describe('isWsConnectConfig', () => {
  it('应该对 ws_connect 类型返回 true', () => {
    const data = createNodeData('ws_connect');
    expect(isWsConnectConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http',
      'conditional',
      'ws_send_collect',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isWsConnectConfig(data)).toBe(false);
    });
  });
});

describe('isWsSendCollectConfig', () => {
  it('应该对 ws_send_wait 类型返回 true', () => {
    const data = createNodeData('ws_send_collect');
    expect(isWsSendCollectConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http',
      'conditional',
      'ws_connect',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isWsSendCollectConfig(data)).toBe(false);
    });
  });
});

describe('isWsCloseConfig', () => {
  it('应该对 ws_close 类型返回 true', () => {
    const data = createNodeData('ws_close');
    expect(isWsCloseConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http',
      'conditional',
      'ws_connect',
      'ws_send_collect',
      'assertion',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isWsCloseConfig(data)).toBe(false);
    });
  });
});

describe('isAssertionConfig', () => {
  it('应该对 assertion 类型返回 true', () => {
    const data = createNodeData('assertion');
    expect(isAssertionConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http',
      'conditional',
      'ws_connect',
      'ws_send_collect',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isAssertionConfig(data)).toBe(false);
    });
  });
});

describe('isDbConnectConfig', () => {
  it('应该对 db_connect 类型返回 true', () => {
    const data = createNodeData('db_connect');
    expect(isDbConnectConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http', 'db_query', 'db_close'];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isDbConnectConfig(data)).toBe(false);
    });
  });
});

describe('isDbQueryConfig', () => {
  it('应该对 db_query 类型返回 true', () => {
    const data = createNodeData('db_query');
    expect(isDbQueryConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http', 'db_connect', 'db_close'];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isDbQueryConfig(data)).toBe(false);
    });
  });
});

describe('isDbBeginConfig', () => {
  it('应该对 db_begin 类型返回 true', () => {
    const data = createNodeData('db_begin');
    expect(isDbBeginConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http', 'db_connect', 'db_commit'];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isDbBeginConfig(data)).toBe(false);
    });
  });
});

describe('isDbCommitConfig', () => {
  it('应该对 db_commit 类型返回 true', () => {
    const data = createNodeData('db_commit');
    expect(isDbCommitConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http', 'db_begin', 'db_rollback'];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isDbCommitConfig(data)).toBe(false);
    });
  });
});

describe('isDbRollbackConfig', () => {
  it('应该对 db_rollback 类型返回 true', () => {
    const data = createNodeData('db_rollback');
    expect(isDbRollbackConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http', 'db_begin', 'db_commit'];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isDbRollbackConfig(data)).toBe(false);
    });
  });
});

describe('isDbCloseConfig', () => {
  it('应该对 db_close 类型返回 true', () => {
    const data = createNodeData('db_close');
    expect(isDbCloseConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http', 'db_connect', 'db_query'];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isDbCloseConfig(data)).toBe(false);
    });
  });
});

describe('isScriptConfig', () => {
  it('应该对 script 类型返回 true', () => {
    const data = createNodeData('script');
    expect(isScriptConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http', 'conditional', 'assertion'];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isScriptConfig(data)).toBe(false);
    });
  });
});

describe('类型守卫类型缩窄', () => {
  it('isEntryConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('entry');
    if (isEntryConfig(data)) {
      // TypeScript 应该识别 data 为 EntryConfig
      expect(data.type).toBe('entry');
      // 可选字段应该可以访问
      const vars = data.initial_variables;
      expect(vars).toBeUndefined();
    }
  });

  it('isHttpConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('http');
    if (isHttpConfig(data)) {
      // TypeScript 应该识别 data 为 HttpConfig
      expect(data.type).toBe('http');
      expect(data.url).toBe('https://example.com');
      // 可选字段应该可以访问
      expect(data.method).toBeUndefined();
    }
  });

  it('isWsSendCollectConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('ws_send_collect');
    if (isWsSendCollectConfig(data)) {
      // TypeScript 应该识别 data 为 WsSendCollectConfig
      expect(data.type).toBe('ws_send_collect');
      expect(data.connection_id).toBe('conn-1');
      expect(data.collect_response).toBe(true);
      expect(data.message).toBe('test message');
    }
  });
});
