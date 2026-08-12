# 服务层知识库

**生成时间:** 2026-06-12

## 概览

前端与后端之间的通信与数据持久化层。`tauri/` 封装 Tauri IPC 调用，`storage/` 提供本地文件读写抽象，`dataset.ts` 是数据驱动执行的独立 IPC 封装。

## 结构

```
services/
├── dataset.ts                        # 数据集执行 IPC（独立文件，因事件监听模式与 tauri/ 不同）
├── tauri/
│   ├── index.ts                      # 桥接层: isTauriEnv() 动态分发到真实 IPC 或 Mock
│   ├── flow.ts                       # 流程 CRUD + 执行 + Channel 订阅 + 验证
│   ├── debug.ts                      # 调试会话: 启动/命令/结束/状态
│   ├── events.ts                     # Channel 辅助函数（订阅日志）
│   ├── mock.ts                       # 全协议 Mock 实现（用于开发/测试）
│   ├── grpc.ts / mcp.ts / graphql.ts # 协议辅助命令（服务发现、内省、骨架生成）
│   ├── sse.ts    # 协议连接测试命令
│   ├── stressTest.ts                 # 压力测试命令
│   ├── tcp.ts                        # TCP/UDP 录制命令
│   ├── settings.ts / settings.mock.ts# 设置持久化（tauri-plugin-store + 目录对话框）
│   └── updater.ts / updater.mock.ts  # 应用更新检查与安装
├── storage/
│   ├── index.ts                      # re-export
│   ├── flowStorage.ts / .mock.ts     # 流程 .mpf 文件 CRUD + 文件夹管理
│   ├── reportStorage.ts / .mock.ts   # 执行报告文件 CRUD
│   ├── stressTestStorage.ts          # 压测报告文件 CRUD
│   └── migration.ts                  # localStorage → .mpf 文件一次性迁移
```

## 找什么

| 目标 | 文件 |
|------|------|
| 流程 CRUD / 执行 / Channel 订阅 | `tauri/flow.ts` |
| 调试会话控制 | `tauri/debug.ts` |
| 设置读写 + 目录对话框 | `tauri/settings.ts` |
| 应用更新 | `tauri/updater.ts` |
| gRPC 服务发现 / 骨架生成 / Proto 操作 | `tauri/grpc.ts` |
| MCP 工具发现 | `tauri/mcp.ts` |
| GraphQL Schema 内省 | `tauri/graphql.ts` |
| SSE 连接测试 | `tauri/sse.ts` |
| 压测控制 | `tauri/stressTest.ts` |
| TCP/UDP 录制读写 | `tauri/tcp.ts` |
| 流程 .mpf 文件读写 + 文件夹管理 | `storage/flowStorage.ts` |
| 执行报告 / 压测报告读写 | `storage/reportStorage.ts` / `stressTestStorage.ts` |
| localStorage 迁移到文件存储 | `storage/migration.ts` |
| 数据集执行 IPC | `dataset.ts` |
| Mock 实现（全部协议） | `tauri/mock.ts` |
| 环境检测 + 桥接入口 | `tauri/index.ts` |

## 约定

- **环境检测**: `isTauriEnv()` 检查 `__TAURI__ in window`。每个导出函数在 `tauri/index.ts` 中先判断环境再动态 `import()` 真实或 Mock 实现。
- **IPC 调用模式**: `invoke<T>('command_name', { camelCaseArgs })`。统一通过 `formatError()` 包装为 `IpcError`（带 command 名和 cause）。
- **Mock 策略**: 每个协议或存储模块有对应的 `*.mock.ts`。Mock 实现模拟网络延迟 + 返回预设数据，不支持文件系统/网络等真实环境操作。
- **实时数据流**: 执行日志/调试事件用 `Channel<T>` via `new Channel<T>()` + `channel.onmessage = cb`，通过 invoke 参数传递。数据集执行用全局 Tauri 事件 `listen('dataset-event', ...)`（因 Channel 会在命令结束后被清理）。
- **存储路径**: 通过 `invoke('get_app_data_dir')` 获取基础目录，可被用户设置覆盖。

## 备注

- 没有专门的 DB 连接测试 IPC 服务；没有 TCP/UDP 协议执行 IPC 服务（`tcp.ts` 仅处理录制）。
- `dataset.ts` 不在 `tauri/` 子目录内，因为它的执行事件走全局事件系统而非 Channel，且需要长时间后台运行。
