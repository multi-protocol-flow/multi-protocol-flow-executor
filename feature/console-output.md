# 控制台输出功能文档

**版本**: 1.0  
**创建日期**: 2026-03-01

---

## 功能概述

控制台输出功能提供流程执行过程中的实时日志显示：

- **实时日志流**: 通过 Tauri Channel API 实现前后端实时通信
- **多级别日志**: INFO / DEBUG / WARN / ERROR
- **节点关联**: 日志可关联到具体执行节点
- **过滤功能**: 支持按级别、节点、关键词过滤

---

## 日志级别

| 级别 | 用途 | 前端颜色 |
|------|------|----------|
| INFO | 一般信息 | 蓝色 |
| DEBUG | 调试信息 | 灰色 |
| WARN | 警告信息 | 黄色 |
| ERROR | 错误信息 | 红色 |

---

## 数据结构

### LogEvent

| 字段 | 类型 | 说明 |
|------|------|------|
| level | LogLevel | 日志级别 |
| message | string | 日志消息内容 |
| timestamp | number | 时间戳（毫秒） |
| nodeUuid | string? | 关联节点 UUID |
| nodeName | string? | 关联节点名称 |
| flowUuid | string | 关联流程 UUID |

---

## 相关文件

| 文件 | 功能 |
|------|------|
| `src/types/events.ts` | LogEvent 类型定义 |
| `src/stores/console.ts` | 控制台状态管理 |
| `src/components/execution/ConsolePanel.tsx` | 日志面板组件 |
| `src-tauri/src/log_event.rs` | LogEvent 结构定义 |
| `src-tauri/src/commands.rs` | 日志发送逻辑 |

---

**文档结束**
