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
  isScriptConfig,
  isVariableExtractorConfig,
} from '../../types/node';
import type { NodeData } from '../../types/node';

/**
 * 测试数据工厂：创建各类型节点的配置数据
 */
function createNodeData(type: NodeData['type']): NodeData {
  switch (type) {
    case 'entry':
      return { type: 'entry' };
    case 'http:request':
      return { type: 'http:request', url: 'https://example.com' };
    case 'conditional':
      return { type: 'conditional' };
    case 'ws:connect':
      return { type: 'ws:connect', url: 'wss://example.com' };
    case 'ws:send_collect':
      return {
        type: 'ws:send_collect',
        connection_id: 'conn-1',
        collect_response: true,
        message: 'test message',
      };
    case 'ws:close':
      return { type: 'ws:close', connection_id: 'conn-1' };
    case 'assertion':
      return {
        type: 'assertion',
        assertions: [],
        mode: 'all',
        on_failure: 'route_false',
        continue_on_failure: false,
      };
    case 'script':
      return { type: 'script', script: '// test script' };
    case 'tcp:connect':
      return { type: 'tcp:connect', host: '127.0.0.1', port: 8080 };
    case 'tcp:send':
      return { type: 'tcp:send', connection_id: 'tcp-conn-1', data: 'hello' };
    case 'tcp:close':
      return { type: 'tcp:close', connection_id: 'tcp-conn-1' };
    case 'udp:bind':
      return { type: 'udp:bind', local_port: 9090 };
    case 'udp:send_to':
      return { type: 'udp:send_to', connection_id: 'udp-conn-1', data: 'hello', target_addr: '127.0.0.1', target_port: 9090 };
    case 'udp:recv_from':
      return { type: 'udp:recv_from', connection_id: 'udp-conn-1' };
    case 'udp:close':
      return { type: 'udp:close', connection_id: 'udp-conn-1' };
    case 'sse:connect':
      return { type: 'sse:connect', url: 'https://example.com/sse' };
    case 'sse:listen':
      return { type: 'sse:listen', connection_id: 'sse-conn-1' };
    case 'sse:disconnect':
      return { type: 'sse:disconnect', connection_id: 'sse-conn-1' };
    case 'graphql:connect':
      return { type: 'graphql:connect', endpoint: 'http://localhost:4000/graphql' };
    case 'graphql:query':
      return { type: 'graphql:query', connection_id: 'gql-conn-1', query: '{ __typename }' };
    case 'graphql:subscribe':
      return { type: 'graphql:subscribe', connection_id: 'gql-conn-1', subscription: 'subscription { test }' };
    case 'graphql:introspect':
      return { type: 'graphql:introspect', connection_id: 'gql-conn-1' };
    case 'graphql:disconnect':
      return { type: 'graphql:disconnect', connection_id: 'gql-conn-1' };

    case 'variable:extractor':
      return { type: 'variable:extractor' };
    default:
      return { type: 'entry' } as NodeData;
  }
}

describe('isEntryConfig', () => {
  it('应该对 entry 类型返回 true', () => {
    const data = createNodeData('entry');
    expect(isEntryConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'http:request',
      'conditional',
      'ws:connect',
      'ws:send_collect',
      'ws:close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isEntryConfig(data)).toBe(false);
    });
  });
});

describe('isHttpConfig', () => {
  it('应该对 http 类型返回 true', () => {
    const data = createNodeData('http:request');
    expect(isHttpConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'conditional',
      'ws:connect',
      'ws:send_collect',
      'ws:close',
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
      'http:request',
      'ws:connect',
      'ws:send_collect',
      'ws:close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isConditionalConfig(data)).toBe(false);
    });
  });
});

describe('isWsConnectConfig', () => {
  it('应该对 ws:connect 类型返回 true', () => {
    const data = createNodeData('ws:connect');
    expect(isWsConnectConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http:request',
      'conditional',
      'ws:send_collect',
      'ws:close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isWsConnectConfig(data)).toBe(false);
    });
  });
});

describe('isWsSendCollectConfig', () => {
  it('应该对 ws_send_wait 类型返回 true', () => {
    const data = createNodeData('ws:send_collect');
    expect(isWsSendCollectConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http:request',
      'conditional',
      'ws:connect',
      'ws:close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isWsSendCollectConfig(data)).toBe(false);
    });
  });
});

describe('isWsCloseConfig', () => {
  it('应该对 ws:close 类型返回 true', () => {
    const data = createNodeData('ws:close');
    expect(isWsCloseConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = [
      'entry',
      'http:request',
      'conditional',
      'ws:connect',
      'ws:send_collect',
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
      'http:request',
      'conditional',
      'ws:connect',
      'ws:send_collect',
      'ws:close',
    ];
    otherTypes.forEach((type) => {
      const data = createNodeData(type);
      expect(isAssertionConfig(data)).toBe(false);
    });
  });
});

describe('isScriptConfig', () => {
  it('应该对 script 类型返回 true', () => {
    const data = createNodeData('script');
    expect(isScriptConfig(data)).toBe(true);
  });

  it('应该对其他类型返回 false', () => {
    const otherTypes: NodeData['type'][] = ['entry', 'http:request', 'conditional', 'assertion'];
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
      expect(data.dataset_path).toBeUndefined();
    }
  });

  it('isHttpConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('http:request');
    if (isHttpConfig(data)) {
      // TypeScript 应该识别 data 为 HttpConfig
      expect(data.type).toBe('http:request');
      expect(data.url).toBe('https://example.com');
      // 可选字段应该可以访问
      expect(data.method).toBeUndefined();
    }
  });

  it('isWsSendCollectConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('ws:send_collect');
    if (isWsSendCollectConfig(data)) {
      // TypeScript 应该识别 data 为 WsSendCollectConfig
      expect(data.type).toBe('ws:send_collect');
      expect(data.connection_id).toBe('conn-1');
      expect(data.collect_response).toBe(true);
      expect(data.message).toBe('test message');
    }
  });

  it('isVariableExtractorConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('variable:extractor');
    expect(isVariableExtractorConfig(data)).toBe(true);
    if (isVariableExtractorConfig(data)) {
      expect(data.type).toBe('variable:extractor');
      expect(data.output_mappings).toBeUndefined();
      expect(data.source_node_id).toBeUndefined();
    }
    // 非 variable:extractor 类型应返回 false
    const httpData: NodeData = createNodeData('http:request');
    expect(isVariableExtractorConfig(httpData)).toBe(false);
  });
});
