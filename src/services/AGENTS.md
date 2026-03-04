# 服务层 (Services)

## 概述

前端与后端通信的抽象层，包含 Tauri IPC 调用和本地存储两类服务。

## 结构

```
services/
├── tauri/                    # Tauri IPC 服务
│   ├── index.ts              # 主入口，自动检测环境切换实现
│   ├── flow.ts               # 流程创建/执行（真实 IPC）
│   ├── mock.ts               # Mock 实现（非 Tauri 环境）
│   ├── events.ts             # 事件订阅
│   ├── settings.ts           # 全局设置（Tauri）
│   └── settings.mock.ts      # 设置 Mock
└── storage/                  # 本地存储服务
    ├── index.ts              # 统一导出
    ├── flowStorage.ts        # 流程元数据存储
    ├── historyStorage.ts     # 执行历史存储
    └── reportStorage.ts      # 报告存储
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 添加新 IPC 命令 | `tauri/index.ts` + `tauri/flow.ts` |
| 修改 Mock 行为 | `tauri/mock.ts` |
| 调试 IPC 调用 | `tauri/flow.ts` 的 `formatError()` |
| 存储流程数据 | `storage/flowStorage.ts` |
| 执行历史管理 | `storage/historyStorage.ts` |

## 约定

### 环境检测
```typescript
// tauri/index.ts 自动检测
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}
```

### IPC 调用模式
- 真实环境: `invoke('command_name', { args })`
- Mock 环境: 模拟延迟 + 返回测试数据

### 错误处理
- 统一使用 `IpcError` 类封装错误
- 包含 `command` 字段标识失败命令

### Mock 用途
- Vitest 单元测试
- Vite 开发服务器（无 Tauri 后端）
- 组件独立开发调试
