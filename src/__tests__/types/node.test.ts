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
  isWsSendWaitConfig,
  isWsCloseConfig,
  isAssertionConfig,
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
    case 'ws_send_wait':
      return {
        type: 'ws_send_wait',
        connection_id: 'conn-1',
        wait_for_response: true,
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
      'ws_send_wait',
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
      'ws_send_wait',
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
      'ws_send_wait',
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
      'ws_send_wait',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isWsConnectConfig(data)).toBe(false);
    });
  });
});

describe('isWsSendWaitConfig', () => {
  it('应该对 ws_send_wait 类型返回 true', () => {
    const data = createNodeData('ws_send_wait');
    expect(isWsSendWaitConfig(data)).toBe(true);
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
      expect(isWsSendWaitConfig(data)).toBe(false);
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
      'ws_send_wait',
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
      'ws_send_wait',
      'ws_close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isAssertionConfig(data)).toBe(false);
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

  it('isWsSendWaitConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('ws_send_wait');
    if (isWsSendWaitConfig(data)) {
      // TypeScript 应该识别 data 为 WsSendWaitConfig
      expect(data.type).toBe('ws_send_wait');
      expect(data.connection_id).toBe('conn-1');
      expect(data.wait_for_response).toBe(true);
      expect(data.message).toBe('test message');
    }
  });
});
