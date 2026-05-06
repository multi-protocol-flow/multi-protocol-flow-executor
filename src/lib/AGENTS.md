# src/lib/

工具函数 + 三重注册表系统：节点元数据、配置面板、结果查看器。

## 文件结构

| 文件 | 职责 |
|------|------|
| `NodeRegistry.ts` | 后端节点元数据获取/缓存、分类排序、颜色映射 |
| `configPanelRegistry.ts` | 节点类型 → 配置面板组件映射 |
| `viewerRegistry.tsx` | 节点类型 → 结果查看器组件映射 |
| `nodeMeta.ts` | 节点图标、显示名称、前端静态元数据 |
| `flowDiff.ts` | 流程对比工具（节点/连接/字段级变化） |
| `restoreFlow.ts` | 流程快照恢复（脏画布保护、UUID冲突处理） |
| `utils.ts` | 通用工具（cn 函数合并 Tailwind class） |
| `curlParser.ts` | cURL 命令解析 |
| `grpcurlGenerator.ts` | grpcurl 命令生成 |
| `protoFieldParser.ts` | Protobuf 字段解析 |

## 快速定位

| 任务 | 文件 |
|------|------|
| 注册新节点元数据 | `NodeRegistry.ts` |
| 注册配置面板 | `configPanelRegistry.ts` |
| 注册结果查看器 | `viewerRegistry.tsx` |
| 流程对比 | `flowDiff.ts` |
| 流程恢复 | `restoreFlow.ts` |

## 约定

**三重注册表模式**：新增节点类型必须同时在以下 3 个文件中注册：
1. `NodeRegistry.ts` - 元数据/分类/排序
2. `configPanelRegistry.ts` - 配置面板组件
3. `viewerRegistry.tsx` - 结果查看器组件

## 反模式

- 跳过任一注册表导致节点无法完整渲染
- 在 `nodeMeta.ts` 硬编码名称而不优先使用后端动态名称
- `restoreFlow.ts` 中不注入依赖而直接调用外部服务
