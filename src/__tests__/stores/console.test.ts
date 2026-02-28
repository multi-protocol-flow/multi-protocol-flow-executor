/**
 * Console Store 测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useConsoleStore } from '@/stores/console';
import type { LogEvent, LogLevel } from '@/types/events';

describe('ConsoleStore', () => {
  // 创建测试日志的辅助函数
  const createTestLog = (
    overrides: Partial<LogEvent> = {}
  ): LogEvent => ({
    level: 'INFO',
    message: '测试消息',
    timestamp: Date.now(),
    flowUuid: 'test-flow-1',
    ...overrides,
  });

  beforeEach(() => {
    // 重置 store 到初始状态
    useConsoleStore.setState({
      logs: [],
      filter: {
        level: 'ALL',
        nodeUuid: 'ALL',
        searchKeyword: '',
      },
    });
  });

  describe('初始状态', () => {
    it('logs 为空数组', () => {
      expect(useConsoleStore.getState().logs).toEqual([]);
    });

    it('filter 为默认值', () => {
      const { filter } = useConsoleStore.getState();
      expect(filter.level).toBe('ALL');
      expect(filter.nodeUuid).toBe('ALL');
      expect(filter.searchKeyword).toBe('');
    });
  });

  describe('addLog', () => {
    it('添加日志', () => {
      const log = createTestLog();
      useConsoleStore.getState().addLog(log);

      const { logs } = useConsoleStore.getState();
      expect(logs).toHaveLength(1);
      expect(logs[0].level).toBe('INFO');
      expect(logs[0].message).toBe('测试消息');
      expect(logs[0].flowUuid).toBe('test-flow-1');
    });

    it('自动生成 id', () => {
      const log = createTestLog();
      useConsoleStore.getState().addLog(log);

      const { logs } = useConsoleStore.getState();
      expect(logs[0].id).toBeDefined();
      expect(typeof logs[0].id).toBe('string');
      expect(logs[0].id).toMatch(/^log-/);
    });

    it('保留原有日志', () => {
      const log1 = createTestLog({ message: '第一条日志' });
      const log2 = createTestLog({ message: '第二条日志' });

      useConsoleStore.getState().addLog(log1);
      useConsoleStore.getState().addLog(log2);

      const { logs } = useConsoleStore.getState();
      expect(logs).toHaveLength(2);
      expect(logs[0].message).toBe('第一条日志');
      expect(logs[1].message).toBe('第二条日志');
    });

    it('添加包含节点信息的日志', () => {
      const log = createTestLog({
        nodeUuid: 'node-1',
        nodeName: 'HTTP请求',
      });
      useConsoleStore.getState().addLog(log);

      const { logs } = useConsoleStore.getState();
      expect(logs[0].nodeUuid).toBe('node-1');
      expect(logs[0].nodeName).toBe('HTTP请求');
    });

    it('添加不同级别的日志', () => {
      const levels: LogLevel[] = ['INFO', 'DEBUG', 'WARN', 'ERROR'];
      
      levels.forEach((level) => {
        useConsoleStore.getState().addLog(createTestLog({ level }));
      });

      const { logs } = useConsoleStore.getState();
      expect(logs).toHaveLength(4);
      expect(logs.map((l) => l.level)).toEqual(levels);
    });
  });

  describe('clearLogs', () => {
    it('清空所有日志', () => {
      useConsoleStore.getState().addLog(createTestLog());
      useConsoleStore.getState().addLog(createTestLog());
      expect(useConsoleStore.getState().logs).toHaveLength(2);

      useConsoleStore.getState().clearLogs();

      expect(useConsoleStore.getState().logs).toHaveLength(0);
    });

    it('清空后可以继续添加日志', () => {
      useConsoleStore.getState().addLog(createTestLog());
      useConsoleStore.getState().clearLogs();
      useConsoleStore.getState().addLog(createTestLog({ message: '新日志' }));

      const { logs } = useConsoleStore.getState();
      expect(logs).toHaveLength(1);
      expect(logs[0].message).toBe('新日志');
    });
  });

  describe('setFilter', () => {
    it('设置级别过滤', () => {
      useConsoleStore.getState().setFilter({ level: 'ERROR' });
      expect(useConsoleStore.getState().filter.level).toBe('ERROR');
    });

    it('设置节点过滤', () => {
      useConsoleStore.getState().setFilter({ nodeUuid: 'node-1' });
      expect(useConsoleStore.getState().filter.nodeUuid).toBe('node-1');
    });

    it('设置搜索关键词', () => {
      useConsoleStore.getState().setFilter({ searchKeyword: '错误' });
      expect(useConsoleStore.getState().filter.searchKeyword).toBe('错误');
    });

    it('部分更新保留其他过滤条件', () => {
      useConsoleStore.getState().setFilter({ level: 'ERROR' });
      useConsoleStore.getState().setFilter({ searchKeyword: '错误' });

      const { filter } = useConsoleStore.getState();
      expect(filter.level).toBe('ERROR');
      expect(filter.searchKeyword).toBe('错误');
      expect(filter.nodeUuid).toBe('ALL');
    });
  });

  describe('setSearchKeyword', () => {
    it('设置搜索关键词', () => {
      useConsoleStore.getState().setSearchKeyword('测试关键词');
      expect(useConsoleStore.getState().filter.searchKeyword).toBe('测试关键词');
    });

    it('可以清空搜索关键词', () => {
      useConsoleStore.getState().setSearchKeyword('关键词');
      useConsoleStore.getState().setSearchKeyword('');
      expect(useConsoleStore.getState().filter.searchKeyword).toBe('');
    });
  });

  describe('setSelectedNode', () => {
    it('设置选中节点', () => {
      useConsoleStore.getState().setSelectedNode('node-1');
      expect(useConsoleStore.getState().filter.nodeUuid).toBe('node-1');
    });

    it('可以设置为 ALL', () => {
      useConsoleStore.getState().setSelectedNode('node-1');
      useConsoleStore.getState().setSelectedNode('ALL');
      expect(useConsoleStore.getState().filter.nodeUuid).toBe('ALL');
    });
  });

  describe('getFilteredLogs', () => {
    beforeEach(() => {
      // 添加多条测试日志
      useConsoleStore.getState().addLog(
        createTestLog({ level: 'INFO', message: '信息日志', nodeUuid: 'node-1', nodeName: '节点1' })
      );
      useConsoleStore.getState().addLog(
        createTestLog({ level: 'ERROR', message: '错误日志', nodeUuid: 'node-2', nodeName: '节点2' })
      );
      useConsoleStore.getState().addLog(
        createTestLog({ level: 'WARN', message: '警告消息', nodeUuid: 'node-1', nodeName: '节点1' })
      );
      useConsoleStore.getState().addLog(
        createTestLog({ level: 'DEBUG', message: '调试信息', nodeUuid: 'node-3', nodeName: '节点3' })
      );
    });

    it('无过滤时返回所有日志', () => {
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(4);
    });

    it('按级别过滤 - ERROR', () => {
      useConsoleStore.getState().setFilter({ level: 'ERROR' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(1);
      expect(filtered[0].level).toBe('ERROR');
    });

    it('按级别过滤 - INFO', () => {
      useConsoleStore.getState().setFilter({ level: 'INFO' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(1);
      expect(filtered[0].level).toBe('INFO');
    });

    it('按级别过滤 - ALL 返回所有', () => {
      useConsoleStore.getState().setFilter({ level: 'ALL' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(4);
    });

    it('按节点过滤', () => {
      useConsoleStore.getState().setFilter({ nodeUuid: 'node-1' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(2);
      filtered.forEach((log) => {
        expect(log.nodeUuid).toBe('node-1');
      });
    });

    it('按关键词搜索（大小写不敏感）', () => {
      useConsoleStore.getState().setFilter({ searchKeyword: '错误' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(1);
      expect(filtered[0].message).toBe('错误日志');
    });

    it('按关键词搜索（部分匹配）', () => {
      useConsoleStore.getState().setFilter({ searchKeyword: '信息' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(2); // '信息日志' 和 '调试信息'
    });

    it('按关键词搜索（大小写不敏感）', () => {
      useConsoleStore.getState().setFilter({ searchKeyword: 'INFO' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      // 无匹配，因为搜索的是 message 内容
      expect(filtered).toHaveLength(0);
    });

    it('组合过滤 - 级别 + 节点', () => {
      useConsoleStore.getState().setFilter({ level: 'WARN', nodeUuid: 'node-1' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(1);
      expect(filtered[0].level).toBe('WARN');
      expect(filtered[0].nodeUuid).toBe('node-1');
    });

    it('组合过滤 - 级别 + 关键词', () => {
      useConsoleStore.getState().setFilter({ level: 'DEBUG', searchKeyword: '信息' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(1);
      expect(filtered[0].level).toBe('DEBUG');
    });

    it('组合过滤 - 节点 + 关键词', () => {
      useConsoleStore.getState().setFilter({ nodeUuid: 'node-2', searchKeyword: '错误' });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(1);
      expect(filtered[0].nodeUuid).toBe('node-2');
    });

    it('组合过滤 - 全部条件', () => {
      useConsoleStore.getState().setFilter({
        level: 'ERROR',
        nodeUuid: 'node-2',
        searchKeyword: '错误',
      });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(1);
    });

    it('组合过滤 - 无匹配结果', () => {
      useConsoleStore.getState().setFilter({
        level: 'ERROR',
        nodeUuid: 'node-1',
      });
      const filtered = useConsoleStore.getState().getFilteredLogs();
      expect(filtered).toHaveLength(0);
    });
  });

  describe('getUniqueNodes', () => {
    it('空日志时返回空数组', () => {
      const nodes = useConsoleStore.getState().getUniqueNodes();
      expect(nodes).toEqual([]);
    });

    it('返回唯一节点列表', () => {
      useConsoleStore.getState().addLog(
        createTestLog({ nodeUuid: 'node-1', nodeName: '节点1' })
      );
      useConsoleStore.getState().addLog(
        createTestLog({ nodeUuid: 'node-2', nodeName: '节点2' })
      );
      useConsoleStore.getState().addLog(
        createTestLog({ nodeUuid: 'node-1', nodeName: '节点1' }) // 重复节点
      );

      const nodes = useConsoleStore.getState().getUniqueNodes();
      expect(nodes).toHaveLength(2);
      
      const uuids = nodes.map((n) => n.uuid).sort();
      expect(uuids).toEqual(['node-1', 'node-2']);
    });

    it('忽略没有节点信息的日志', () => {
      useConsoleStore.getState().addLog(
        createTestLog({ nodeUuid: 'node-1', nodeName: '节点1' })
      );
      useConsoleStore.getState().addLog(
        createTestLog({}) // 无节点信息
      );

      const nodes = useConsoleStore.getState().getUniqueNodes();
      expect(nodes).toHaveLength(1);
      expect(nodes[0].uuid).toBe('node-1');
    });

    it('返回正确的节点名称', () => {
      useConsoleStore.getState().addLog(
        createTestLog({ nodeUuid: 'node-1', nodeName: 'HTTP请求' })
      );

      const nodes = useConsoleStore.getState().getUniqueNodes();
      expect(nodes[0].name).toBe('HTTP请求');
    });
  });

  describe('Selectors', () => {
    it('selectLogs 返回 logs', () => {
      useConsoleStore.getState().addLog(createTestLog());
      const state = useConsoleStore.getState();
      expect(state.logs).toHaveLength(1);
    });

    it('selectFilter 返回 filter', () => {
      useConsoleStore.getState().setFilter({ level: 'ERROR' });
      const state = useConsoleStore.getState();
      expect(state.filter.level).toBe('ERROR');
    });

    it('selectSearchKeyword 返回 searchKeyword', () => {
      useConsoleStore.getState().setSearchKeyword('测试');
      const state = useConsoleStore.getState();
      expect(state.filter.searchKeyword).toBe('测试');
    });

    it('selectSelectedNode 返回 nodeUuid', () => {
      useConsoleStore.getState().setSelectedNode('node-1');
      const state = useConsoleStore.getState();
      expect(state.filter.nodeUuid).toBe('node-1');
    });
  });
});
