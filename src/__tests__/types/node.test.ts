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
    case 'script':
      return { type: 'script', script: '// test script' };
    case 'grpc_connect':
      return { type: 'grpc_connect', url: 'http://localhost:50051' };
    case 'grpc_call':
      return { type: 'grpc_call', connection_id: 'grpc-conn-1', service_name: 'TestService', method_name: 'TestMethod', request_json: '{}' };
    case 'grpc_close':
      return { type: 'grpc_close', connection_id: 'grpc-conn-1' };
    case 'mcp_connect':
      return { type: 'mcp_connect', url: 'http://localhost:3000/mcp' };
    case 'mcp_call_tool':
      return { type: 'mcp_call_tool', connection_id: 'mcp-conn-1', tool_name: 'test_tool' };
    case 'mcp_read_resource':
      return { type: 'mcp_read_resource', connection_id: 'mcp-conn-1', resource_uri: 'test://resource' };
    case 'mcp_get_prompt':
      return { type: 'mcp_get_prompt', connection_id: 'mcp-conn-1', prompt_name: 'test_prompt' };
    case 'mcp_close':
      return { type: 'mcp_close', connection_id: 'mcp-conn-1' };
    case 'tcp_connect':
      return { type: 'tcp_connect', host: '127.0.0.1', port: 8080 };
    case 'tcp_send':
      return { type: 'tcp_send', connection_id: 'tcp-conn-1', data: 'hello' };
    case 'tcp_close':
      return { type: 'tcp_close', connection_id: 'tcp-conn-1' };
    case 'udp_bind':
      return { type: 'udp_bind', local_port: 9090 };
    case 'udp_send_to':
      return { type: 'udp_send_to', connection_id: 'udp-conn-1', data: 'hello', target_addr: '127.0.0.1', target_port: 9090 };
    case 'udp_recv_from':
      return { type: 'udp_recv_from', connection_id: 'udp-conn-1' };
    case 'udp_close':
      return { type: 'udp_close', connection_id: 'udp-conn-1' };
    case 'sse_connect':
      return { type: 'sse_connect', url: 'https://example.com/sse' };
    case 'sse_listen':
      return { type: 'sse_listen', connection_id: 'sse-conn-1' };
    case 'sse_disconnect':
      return { type: 'sse_disconnect', connection_id: 'sse-conn-1' };
    case 'graphql_connect':
      return { type: 'graphql_connect', endpoint: 'http://localhost:4000/graphql' };
    case 'graphql_query':
      return { type: 'graphql_query', connection_id: 'gql-conn-1', query: '{ __typename }' };
    case 'graphql_subscribe':
      return { type: 'graphql_subscribe', connection_id: 'gql-conn-1', subscription: 'subscription { test }' };
    case 'graphql_introspect':
      return { type: 'graphql_introspect', connection_id: 'gql-conn-1' };
    case 'graphql_disconnect':
      return { type: 'graphql_disconnect', connection_id: 'gql-conn-1' };

    case 'variable_extractor':
      return { type: 'variable_extractor' };
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
      expect(data.dataset_path).toBeUndefined();
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

  it('isVariableExtractorConfig 应该正确缩窄类型', () => {
    const data: NodeData = createNodeData('variable_extractor');
    expect(isVariableExtractorConfig(data)).toBe(true);
    if (isVariableExtractorConfig(data)) {
      expect(data.type).toBe('variable_extractor');
      expect(data.output_mappings).toBeUndefined();
      expect(data.source_node_id).toBeUndefined();
    }
    // 非 variable_extractor 类型应返回 false
    const httpData: NodeData = createNodeData('http');
    expect(isVariableExtractorConfig(httpData)).toBe(false);
  });
});
