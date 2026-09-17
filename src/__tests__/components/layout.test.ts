import { describe, expect, it } from 'vitest';
import { getLayoutedPositions, getAdaptiveLayoutedPositions } from '@/components/canvas/layout';
import { createDefaultNodeData } from '@/components/canvas/utils';
import type { Node, Connection, AnyNodeType } from '@/types';

function makeNode(uuid: string, type: string = 'http:request'): Node {
  return {
    uuid,
    type: type as Node['type'],
    name: uuid,
    position: { x: 0, y: 0 },
    data: createDefaultNodeData(type as AnyNodeType),
  };
}

function makeConn(
  source: string,
  target: string,
  source_port_id: string = 'true',
  target_port_id: string = 'in'
): Connection {
  return {
    id: `${source}-${target}`,
    source_node_uuid: source,
    target_node_uuid: target,
    source_port_id,
    target_port_id,
  };
}

describe('layout.ts - getLayoutedPositions (原有水平布局)', () => {
  it('空节点返回空 Map', () => {
    const result = getLayoutedPositions([], []);
    expect(result.size).toBe(0);
  });

  it('线性节点按水平 LR 单向单行排列', () => {
    const nodes = [makeNode('n1', 'entry'), makeNode('n2'), makeNode('n3')];
    const conns = [makeConn('n1', 'n2', 'out'), makeConn('n2', 'n3')];

    const positions = getLayoutedPositions(nodes, conns, 'LR');
    expect(positions.size).toBe(3);

    const p1 = positions.get('n1')!;
    const p2 = positions.get('n2')!;
    const p3 = positions.get('n3')!;

    expect(p1.x).toBeLessThan(p2.x);
    expect(p2.x).toBeLessThan(p3.x);
    // 所有节点在同一水平线上
    expect(p1.y).toBe(p2.y);
    expect(p2.y).toBe(p3.y);
  });
});

describe('layout.ts - getAdaptiveLayoutedPositions (全新自适应折行布局)', () => {
  it('空节点返回空 Map', () => {
    const result = getAdaptiveLayoutedPositions([], []);
    expect(result.size).toBe(0);
  });

  it('单个节点分配正常坐标', () => {
    const nodes = [makeNode('single', 'entry')];
    const positions = getAdaptiveLayoutedPositions(nodes, []);

    expect(positions.size).toBe(1);
    const pos = positions.get('single')!;
    expect(pos.x).toBeGreaterThanOrEqual(0);
    expect(pos.y).toBeGreaterThanOrEqual(0);
  });

  it('线性流程超出列容量时自动折行到下一行', () => {
    // 创建 10 个线性节点
    const nodes = Array.from({ length: 10 }, (_, i) =>
      makeNode(`n${i + 1}`, i === 0 ? 'entry' : 'http:request')
    );
    const conns = Array.from({ length: 9 }, (_, i) =>
      makeConn(`n${i + 1}`, `n${i + 2}`, i === 0 ? 'out' : 'true')
    );

    // 显式限制 maxColumns 为 4
    const positions = getAdaptiveLayoutedPositions(nodes, conns, {
      maxColumns: 4,
      colGap: 80,
      rowGap: 120,
      leftMargin: 80,
      topMargin: 80,
    });

    expect(positions.size).toBe(10);

    const p1 = positions.get('n1')!;
    const p2 = positions.get('n2')!;
    const p3 = positions.get('n3')!;
    const p4 = positions.get('n4')!;
    const p5 = positions.get('n5')!;
    const p8 = positions.get('n8')!;
    const p9 = positions.get('n9')!;
    const p10 = positions.get('n10')!;

    // 第一行：n1 ~ n4 在同一 Y 轴
    expect(p1.y).toBe(p2.y);
    expect(p2.y).toBe(p3.y);
    expect(p3.y).toBe(p4.y);
    expect(p1.x).toBe(80);
    expect(p2.x).toBe(80 + (180 + 80) * 1);
    expect(p3.x).toBe(80 + (180 + 80) * 2);
    expect(p4.x).toBe(80 + (180 + 80) * 3);

    // 第二行：n5 折行到第 0 列
    expect(p5.x).toBe(p1.x); // x 坐标对齐
    expect(p5.y).toBe(p1.y + 80 + 120); // y 坐标下移 (NODE_HEIGHT + rowGap)

    // 第三行：n9 再次折行到第 0 列
    expect(p9.x).toBe(p1.x);
    expect(p9.y).toBe(p5.y + 80 + 120);

    // 第四个在第二行末尾，第九、十个在第三行
    expect(p8.x).toBe(p4.x);
    expect(p10.x).toBe(p2.x);
    expect(p10.y).toBe(p9.y);
  });

  it('根据 viewportWidth 动态计算每行最大列数', () => {
    const nodes = Array.from({ length: 8 }, (_, i) => makeNode(`n${i + 1}`));
    const conns = Array.from({ length: 7 }, (_, i) => makeConn(`n${i + 1}`, `n${i + 2}`));

    // 较窄视口 (800px): 可用宽度 800 - 160 = 640px, (640+80)/260 = 2 列
    const narrowPos = getAdaptiveLayoutedPositions(nodes, conns, { viewportWidth: 800 });
    const narrowN1 = narrowPos.get('n1')!;
    const narrowN2 = narrowPos.get('n2')!;
    const narrowN3 = narrowPos.get('n3')!;
    // n3 应该换行到第二行（第 0 列），所以 n3.x === n1.x
    expect(narrowN3.x).toBe(narrowN1.x);
    expect(narrowN3.y).toBeGreaterThan(narrowN1.y);
    expect(narrowN2.y).toBe(narrowN1.y);

    // 较宽视口 (1600px): 可用宽度 1440px, (1440+80)/260 = 5 列
    const widePos = getAdaptiveLayoutedPositions(nodes, conns, { viewportWidth: 1600 });
    const wideN1 = widePos.get('n1')!;
    const wideN3 = widePos.get('n3')!;
    const wideN5 = widePos.get('n5')!;
    const wideN6 = widePos.get('n6')!;
    // 宽屏下 n3 和 n5 仍在第一行，n6 才换行
    expect(wideN3.y).toBe(wideN1.y);
    expect(wideN5.y).toBe(wideN1.y);
    expect(wideN6.x).toBe(wideN1.x);
    expect(wideN6.y).toBeGreaterThan(wideN1.y);
  });

  it('正确处理条件分支 (True/False) 的垂直高度展开与居中对齐', () => {
    const nodes = [
      makeNode('entry', 'entry'),
      makeNode('cond', 'conditional'),
      makeNode('true_branch', 'http:request'),
      makeNode('false_branch', 'http:request'),
      makeNode('merge', 'script'),
    ];

    const conns = [
      makeConn('entry', 'cond', 'out'),
      makeConn('cond', 'true_branch', 'true'),
      makeConn('cond', 'false_branch', 'false'),
      makeConn('true_branch', 'merge', 'true'),
      makeConn('false_branch', 'merge', 'true'),
    ];

    const positions = getAdaptiveLayoutedPositions(nodes, conns, {
      maxColumns: 4,
    });

    const pCond = positions.get('cond')!;
    const pTrue = positions.get('true_branch')!;
    const pFalse = positions.get('false_branch')!;
    const pMerge = positions.get('merge')!;

    // true_branch 和 false_branch 处于同一 Rank，同属一个列
    expect(pTrue.x).toBe(pFalse.x);
    expect(pTrue.x).toBeGreaterThan(pCond.x);
    // 两分支垂直间距不重叠
    expect(Math.abs(pTrue.y - pFalse.y)).toBeGreaterThanOrEqual(80);
    // merge 节点位于其下游列
    expect(pMerge.x).toBeGreaterThan(pTrue.x);
  });

  it('支持带有环路 (Cycle) 的流程拓扑，不崩溃不卡死', () => {
    const nodes = [makeNode('n1'), makeNode('n2'), makeNode('n3'), makeNode('n4')];
    const conns = [
      makeConn('n1', 'n2'),
      makeConn('n2', 'n3'),
      makeConn('n3', 'n4'),
      makeConn('n4', 'n2'), // 环路
    ];

    expect(() => {
      const positions = getAdaptiveLayoutedPositions(nodes, conns);
      expect(positions.size).toBe(4);
    }).not.toThrow();
  });

  it('优先排列含 entry 节点的子图，独立子图垂直依次排列', () => {
    // Flow 1: 辅助无 entry 子图 (sub1 -> sub2)
    // Flow 2: 主业务含 entry 子图 (main1 [entry] -> main2)
    const nodes = [
      makeNode('sub1', 'http:request'),
      makeNode('sub2', 'http:request'),
      makeNode('main1', 'entry'),
      makeNode('main2', 'http:request'),
    ];

    const conns = [
      makeConn('sub1', 'sub2'),
      makeConn('main1', 'main2', 'out'),
    ];

    const positions = getAdaptiveLayoutedPositions(nodes, conns);

    const pMain1 = positions.get('main1')!;
    const pSub1 = positions.get('sub1')!;

    // 含 entry 的主子图排在最上方
    expect(pMain1.y).toBeLessThan(pSub1.y);
  });

  it('孤立未连线节点在底部整齐网格排列', () => {
    const nodes = [
      makeNode('flow1', 'entry'),
      makeNode('flow2', 'http:request'),
      makeNode('iso1', 'script'),
      makeNode('iso2', 'script'),
    ];

    const conns = [makeConn('flow1', 'flow2', 'out')];

    const positions = getAdaptiveLayoutedPositions(nodes, conns, { maxColumns: 4 });

    const pFlow1 = positions.get('flow1')!;
    const pIso1 = positions.get('iso1')!;
    const pIso2 = positions.get('iso2')!;

    // 孤立节点位于连通图下方
    expect(pIso1.y).toBeGreaterThan(pFlow1.y);
    expect(pIso2.y).toBe(pIso1.y); // 在同一行
    expect(pIso2.x).toBeGreaterThan(pIso1.x); // 并排排列
  });
});
