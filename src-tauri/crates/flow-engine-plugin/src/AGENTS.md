# flow-engine-plugin/src

**Generated:** 2026-08-05 | **Branch:** feature/plugins

## OVERVIEW

宿主侧 sidecar 插件管理器。实现 MPE 插件协议的宿主半边：插件目录扫描、子进程生命周期、stdio 上的 JSON-RPC 传输、`SidecarPlugin` NodePlugin 包装。32 个 .rs 文件，约 4200 行，workspace 最大 crate。依赖 `flow-engine-core`（宿主类型）+ `mpe-plugin-sdk`（共享 wire 契约，git 依赖 + tag 固定，`default-features = false`）+ `flow-engine-executor`。

## STRUCTURE

```
src/
├── lib.rs                  # 模块声明骨架
├── manager.rs              # PluginManager: scan + describe + quarantine + lifecycle + registration
├── discovery.rs            # 插件目录扫描
├── describe.rs             # Describe RPC 握手 + P0 前端归一化
├── manifest.rs             # plugin.json 解析与校验
├── registration.rs         # describe 结果注册进宿主 registry（冲突策略, M2c）
├── reserved.rs             # 保留的内置节点 type id（冲突策略用）
├── lifecycle.rs            # 子进程 start / recycle / shutdown
├── process.rs              # 子进程 spawn 卫生: stderr drain, Job Object, CLOEXEC
├── transport/              # pipelined JSON-RPC 传输
│   ├── mod.rs, reader.rs, writer.rs, connection.rs, backpressure.rs
├── framing.rs              # LF 行分隔 + 2 MiB 上限
├── protocol.rs             # JSON-RPC wire 类型（经 mpe-plugin-sdk::protocol）
├── semver.rs               # 最小化 semver 解析 + min_host_version 门槛
├── sidecar.rs              # SidecarPlugin: NodePlugin 包装
├── wrapper.rs              # execute RPC 映射 helper（pub(crate)，250 LOC 上限外置）
├── error.rs                # 插件错误类型
├── bin/echo_mock.rs        # echo mock 插件 demo 二进制
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| PluginManager 入口 / 总流程 | `manager.rs` |
| 新增或冲突的节点类型注册 | `registration.rs` + `reserved.rs` |
| 插件进程生命周期（复用、回收） | `lifecycle.rs` |
| 子进程 spawn 卫生、Job Object | `process.rs` |
| JSON-RPC 传输 / 反压 | `transport/` + `framing.rs` |
| plugin.json 字段与校验规则 | `manifest.rs` |
| 版本门槛（min_host_version） | `semver.rs` |
| 插件节点执行包装 | `sidecar.rs` + `wrapper.rs` |
| wire 协议细节 | `docs/plugin-protocol.md` / `plugin-perf.md` / `plugin-sdk.md` |

## CONVENTIONS

- **内置节点 id 永不被插件覆盖**；未知/重复 id 只记录日志，绝不致命（`reserved.rs` + `registration.rs`）
- **reader 遇单条畸形行不退出**：跳过并继续读后续帧（`framing.rs` / `transport/reader.rs`）
- **describe 握手有界**：带超时 + 超时即 kill 子进程（近期修复，勿回退）
- **unsafe 唯一例外**：`process.rs` 中 Windows Job Object FFI（`CreateJobObjectW`、`AssignProcessToJobObject`、JobObject 的 unsafe Send/Sync），其余代码维持 workspace `forbid unsafe_code`
- 业务代码禁 `unwrap()`；execute RPC 映射保持独立于 `sidecar.rs`（250 纯 LOC 上限）
- 传输层变更需同步跑 `benches/pipelining.rs` 基准，防流水线吞吐回退

## ANTI-PATTERNS

- 让插件节点覆盖内置节点 id（违反保留策略）
- reader 对单条坏帧整体崩溃，或让帧大小突破 2 MiB 上限
- describe 握手无超时等待（子进程可能永久挂起）
- 在 `process.rs` 之外新增 unsafe 代码（需 workspace 检查）
- 新模块逻辑塞进 `sidecar.rs` 导致超过 250 行上限，应放入 `wrapper.rs`
- 单测只覆盖 happy path，不覆盖冲突注册 / 畸形帧 / 超时 kill
