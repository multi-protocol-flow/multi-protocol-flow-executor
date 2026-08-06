/**
 * plugin-iframe postMessage 协议类型守卫测试
 * 覆盖 isPluginIframeMessage 信封 + 各类型 payload 形状校验 + 消息构造器
 */
import { describe, it, expect } from 'vitest';
import {
  isPluginIframeMessage,
  createInitMessage,
  createConfigUpdatedMessage,
  createActionResultMessage,
} from '@/types/plugin-iframe';

const CONFIG = { url: 'https://example.com', method: 'get' };
const VARIABLES = { api_base: 'https://api.example.com' };
const HOST_API = { testConnection: true, openFileDialog: true };

describe('isPluginIframeMessage', () => {
  describe('合法消息通过', () => {
    it('init 完整 payload', () => {
      const msg = { type: 'init', payload: { config: CONFIG, variables: VARIABLES, host_api: HOST_API } };
      expect(isPluginIframeMessage(msg)).toBe(true);
    });

    it('configUpdated', () => {
      expect(isPluginIframeMessage({ type: 'configUpdated', payload: { config: CONFIG } })).toBe(true);
    });

    it('actionResult 无 error', () => {
      expect(isPluginIframeMessage({ type: 'actionResult', payload: { requestId: 'r1', result: { ok: true } } })).toBe(true);
    });

    it('actionResult 带 error', () => {
      expect(isPluginIframeMessage({ type: 'actionResult', payload: { requestId: 'r1', result: null, error: 'boom' } })).toBe(true);
    });

    it('ready（空 payload）', () => {
      expect(isPluginIframeMessage({ type: 'ready', payload: {} })).toBe(true);
    });

    it('configChanged', () => {
      expect(isPluginIframeMessage({ type: 'configChanged', payload: { config: { url: 'x' } } })).toBe(true);
    });

    it('requestAction 无 params', () => {
      expect(isPluginIframeMessage({ type: 'requestAction', payload: { requestId: 'r2', action: 'testConnection' } })).toBe(true);
    });

    it('requestAction 带 params', () => {
      expect(isPluginIframeMessage({ type: 'requestAction', payload: { requestId: 'r2', action: 'custom', params: { a: 1 } } })).toBe(true);
    });

    it('resize 正整数', () => {
      expect(isPluginIframeMessage({ type: 'resize', payload: { height: 420 } })).toBe(true);
    });

    it('resize 零', () => {
      expect(isPluginIframeMessage({ type: 'resize', payload: { height: 0 } })).toBe(true);
    });

    it('error', () => {
      expect(isPluginIframeMessage({ type: 'error', payload: { message: 'panel crashed' } })).toBe(true);
    });
  });

  describe('畸形消息被拒绝', () => {
    it('非对象 data（null/字符串/数字/数组）', () => {
      expect(isPluginIframeMessage(null)).toBe(false);
      expect(isPluginIframeMessage(undefined)).toBe(false);
      expect(isPluginIframeMessage('hello')).toBe(false);
      expect(isPluginIframeMessage(42)).toBe(false);
      expect(isPluginIframeMessage(['init'])).toBe(false);
    });

    it('缺少 type 或 type 非字符串', () => {
      expect(isPluginIframeMessage({ payload: {} })).toBe(false);
      expect(isPluginIframeMessage({ type: 123, payload: {} })).toBe(false);
      expect(isPluginIframeMessage({ type: null, payload: {} })).toBe(false);
    });

    it('未知消息类型被拒绝', () => {
      expect(isPluginIframeMessage({ type: 'hack', payload: {} })).toBe(false);
      expect(isPluginIframeMessage({ type: 'eval', payload: { code: 'x' } })).toBe(false);
    });

    it('payload 缺失或非对象', () => {
      expect(isPluginIframeMessage({ type: 'ready' })).toBe(false);
      expect(isPluginIframeMessage({ type: 'resize', payload: '420' })).toBe(false);
      expect(isPluginIframeMessage({ type: 'error', payload: null })).toBe(false);
      expect(isPluginIframeMessage({ type: 'ready', payload: [1, 2] })).toBe(false);
    });

    it('resize payload 形状错误', () => {
      expect(isPluginIframeMessage({ type: 'resize', payload: {} })).toBe(false);
      expect(isPluginIframeMessage({ type: 'resize', payload: { height: '420' } })).toBe(false);
      expect(isPluginIframeMessage({ type: 'resize', payload: { height: -10 } })).toBe(false);
      expect(isPluginIframeMessage({ type: 'resize', payload: { height: Number.NaN } })).toBe(false);
      expect(isPluginIframeMessage({ type: 'resize', payload: { height: Number.POSITIVE_INFINITY } })).toBe(false);
    });

    it('requestAction payload 形状错误', () => {
      expect(isPluginIframeMessage({ type: 'requestAction', payload: {} })).toBe(false);
      expect(isPluginIframeMessage({ type: 'requestAction', payload: { requestId: 'r', action: 42 } })).toBe(false);
      expect(isPluginIframeMessage({ type: 'requestAction', payload: { requestId: 1, action: 'x' } })).toBe(false);
    });

    it('actionResult payload 形状错误', () => {
      expect(isPluginIframeMessage({ type: 'actionResult', payload: {} })).toBe(false);
      expect(isPluginIframeMessage({ type: 'actionResult', payload: { result: 1 } })).toBe(false);
      expect(isPluginIframeMessage({ type: 'actionResult', payload: { requestId: 'r', result: 1, error: 42 } })).toBe(false);
    });

    it('error payload 形状错误', () => {
      expect(isPluginIframeMessage({ type: 'error', payload: {} })).toBe(false);
      expect(isPluginIframeMessage({ type: 'error', payload: { message: 42 } })).toBe(false);
    });

    it('init payload 缺字段', () => {
      expect(isPluginIframeMessage({ type: 'init', payload: { config: CONFIG } })).toBe(false);
      expect(isPluginIframeMessage({ type: 'init', payload: { variables: VARIABLES, host_api: HOST_API } })).toBe(false);
      expect(isPluginIframeMessage({ type: 'init', payload: { config: CONFIG, variables: 'x', host_api: HOST_API } })).toBe(false);
    });

    it('configChanged payload 缺 config', () => {
      expect(isPluginIframeMessage({ type: 'configChanged', payload: {} })).toBe(false);
    });
  });
});

describe('消息构造器', () => {
  it('createInitMessage 组装 init 信封', () => {
    const msg = createInitMessage(CONFIG, VARIABLES, HOST_API);
    expect(msg.type).toBe('init');
    expect(msg.payload).toEqual({ config: CONFIG, variables: VARIABLES, host_api: HOST_API });
    // 构造器产物必须通过守卫（反向一致性）
    expect(isPluginIframeMessage(msg)).toBe(true);
  });

  it('createConfigUpdatedMessage 组装 configUpdated 信封', () => {
    const msg = createConfigUpdatedMessage(CONFIG);
    expect(msg.type).toBe('configUpdated');
    expect(msg.payload).toEqual({ config: CONFIG });
    expect(isPluginIframeMessage(msg)).toBe(true);
  });

  it('createActionResultMessage 成功结果', () => {
    const msg = createActionResultMessage('req-1', { ok: true });
    expect(msg.type).toBe('actionResult');
    expect(msg.payload).toEqual({ requestId: 'req-1', result: { ok: true } });
    expect(msg.payload.error).toBeUndefined();
    expect(isPluginIframeMessage(msg)).toBe(true);
  });

  it('createActionResultMessage 失败结果带 error', () => {
    const msg = createActionResultMessage('req-2', null, 'connection refused');
    expect(msg.type).toBe('actionResult');
    expect(msg.payload).toEqual({ requestId: 'req-2', result: null, error: 'connection refused' });
    expect(isPluginIframeMessage(msg)).toBe(true);
  });
});
