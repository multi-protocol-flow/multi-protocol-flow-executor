# grpc 节点插件化实施技术文档

> **Status:** 已实施（2026-08-18）
> **范围:** 内置 gRPC 节点（`flow-engine-grpc`，9.3k 行，宿主唯一重量级独有依赖树 tonic/prost/protox/axum/tower 0.4）从宿主彻底剥离，以 sidecar 插件形式重新提供 3 个节点：`grpc:connect` / `grpc:call` / `grpc:close`
> **关联文档:** `docs/plugin-architecture.md`（方案）、`docs/plugin-protocol.md`（wire 规范）、`docs/plugin-sdk.md`（作者指南）
> **仓库位置:** `plugins/grpc/`（独立 workspace、独立 Cargo.lock；插件自带 React 前端 `plugins/grpc/frontend/`）
> **SDK:** `mpe-plugin-sdk` v0.2.1（新增 `ctx.emit` backpressure 通知 + `NodeDescription.viewer` 报告页）

---

## 0. 实施摘要

- **类型命名空间**：`grpc:connect` / `grpc:call` / `grpc:close`（同 `mcp:*` 先例）；宿主 `NodeType`/`NodeData` 的 grpc 变体、`GrpcProvider` trait、`protocol-grpc` feature（4 层）、9 个 IPC 命令、`GrpcCancelSessionManager` 全部删除，无别名/兼容路径。
- **保留全部体验**：3 个配置面板（连接/调用/关闭）、GrpcFormEditor 表单编辑、服务发现（反射 + proto 文件）、骨架生成、JSON 校验、channelz 内省、流式实时消息、报告 viewer、单节点测试、grpcurl 导出——全部由插件自带前端（iframe + postMessage 桥）承载。
- **连接池**：插件进程内按 `(execution_id, connection_id)` 键控；`flowEnded` 清空该 execution 全部连接；`grpc:close` 显式释放单条。
- **流式链路**：插件 `ctx.emit("grpc.stream", …)`（backpressure，自动带 correlation）→ 宿主 `flow-engine-plugin` 通知 handler → `DebugEvent::GrpcStreamMessage/Error` → 现有 debug store（grpcStreamMessages 按 call_id 累积）→ 报告 viewer iframe（postMessage 转发）。宿主 debug store 数据路径未改。
- **SDK 扩展（v0.2.1）**：`ctx.emit(method, params)`（响应级 backpressure，不丢帧）+ `NodeDescription.viewer`（第二前端页，报告 viewer）；`ExecuteContext::request_id`（SDK `cancel` 钩子与 in-flight 调用的关联）。

## 1. 架构

```
宿主（MPE）                                       插件进程（mpe-plugin-grpc，常驻）
┌─────────────────────────────┐                 ┌──────────────────────────────┐
│ flow-engine-executor        │                 │ lib.rs（describe/validate/   │
│  └ default_registry 无 grpc  │   JSON-RPC 2.0  │   execute/ui_call/cancel/    │
│ flow-engine-plugin          │◄────stdio──────►│   flow_ended 分派）           │
│  ├ sidecar.rs 执行包装        │                 │ pool.rs — (execution_id,     │
│  │  └ grpc.stream 通知 handler│                 │   connection_id) 键控池       │
│  │    → DebugEvent::Grpc*   │                 │ grpc_{connect,call,close}_    │
│  ├ describe.rs viewer URL   │                 │   executor.rs                 │
│  ├ reserved.rs 无 grpc_*    │                 │ tls/codec/helpers/            │
│  └ plugin_protocol.rs /viewer│                 │   error_detail/proto_parser/  │
│    路由                      │                 │   reflection（原样迁移）       │
│ stores/debug.ts（未改）      │                 │ ui.rs — 10 个设计时子方法      │
│ PluginIframeViewer（报告 iframe│                 │ frontend/dist/panel+viewer   │
│  + stream 转发）              │                 │   （React 单文件 HTML）       │
└─────────────────────────────┘                 └──────────────────────────────┘
```

## 2. 安装与使用

### 2.1 安装

插件以 marketplace 分发（同 redis/mcp/mongo）：

```bash
mpe plugin install grpc --registry https://multi-protocol-flow.github.io/mpe-plugin-registry
# 或本地构建
cargo build --release --manifest-path plugins/grpc/Cargo.toml --bin mpe_plugin_grpc
# 产物拷入 MPE_PLUGIN_DIR（含 plugin.json + mpe_plugin_grpc）
```

### 2.2 节点

| 节点 | 类型 id | 端口 | 说明 |
|---|---|---|---|
| gRPC Connect | `grpc:connect` | in/true/false | 建立 gRPC 连接（TLS/mTLS/SNI/keepalive/负载均衡），proto 文件或 Server Reflection 服务发现；`discovered_services` 设计时缓存写回 config（保存/加载天然保留） |
| gRPC Call | `grpc:call` | in/true/false | 调用方法（Unary/Server/Client/Bidi Streaming），骨架生成、JSON 校验、表单编辑、grpcurl 导出、调用级 metadata/超时/压缩、重试 |
| gRPC Close | `grpc:close` | in/true/false | 显式释放单条连接（`flowEnded` 是另一释放路径，双释放无害） |

面板功能（全部由插件 iframe 承载）：URL/TLS 高级配置、测试连接 & 发现服务、服务树（方法流式徽章 + MessageSchemaView）、手动 Proto 文件管理（添加文件/选择目录/导入 Descriptor Set）、默认 Metadata、连接超时、压缩、keepalive、channelz 内省、负载均衡端点、重试配置。

报告 viewer（报告页 iframe）：ConnectSection/CallSection/CloseSection + 实时流式消息逐条追加（`grpc.stream` → debug store → viewer iframe postMessage）。

### 2.3 设计时查询（uiCall 子方法，宿主纯转发）

`grpc.discover` / `grpc.scanProtoDirectory` / `grpc.readProtoFiles` / `grpc.parseDescriptorSet` / `grpc.skeleton` / `grpc.skeletonReflection` / `grpc.validate` / `grpc.channelz` / `grpc.health` / `grpc.cancelStream`——方法名与参数形状见 `plugins/grpc/src/ui.rs`。

## 3. 宿主改造清单

后端：
- `flow-engine-grpc` crate 删除（workspace member 移除）；`protocol-grpc` feature 从 executor/facade/app/mpe-cli 4 层摘除
- `flow-engine-core`：`NodeType`/`NodeData` grpc 变体、`Grpc*Config`/`GrpcMetadataEntry`/`StreamMessage`/`ProtoFile` 类型、`GrpcProvider` trait、`ProviderKey::Grpc`、`GRPC_*` 常量、`ErrorKind::Grpc*`/`E_GRPC_*` 删除；`DebugEvent::GrpcStreamMessage/Error` **保留**（debug store 消费）
- `flow-engine-executor`：`default_registry` grpc 注册、`SUPPORTED_SINGLE_NODE_TYPES` grpc_connect、`with_default_providers_and_grpc` 删除
- `flow-engine-plugin`：`sidecar.rs` 执行 grpc:* 节点时注册 `grpc.stream` 通知 handler（`wrapper.rs::register_grpc_stream_if_needed`，非 grpc 节点零开销；未匹配 execution 丢弃）；`describe.rs` `normalize_node` 生成 viewer URL；`NodePlugin::viewer()`/`NodeMetadata.viewer` 新增；`reserved.rs` 移出 grpc_*
- `app`：9 个 grpc IPC 命令、`GrpcCancelSessionManager`、grpc_state 接线、invoke_handler 条目删除；`plugin_protocol.rs` 增加 `/viewer` 路由（与 config-panel 同逻辑）
- `workspace-hack`：hakari 再生成（tonic/prost 等退出统一依赖）

前端：
- 内置 grpc 组件/服务/i18n 全部删除（面板、GrpcFormEditor、MessageSchemaView、GrpcViewer、画布节点、`services/tauri/grpc.ts`、注册表条目、i18n keys）
- `PluginIframeViewer` 升级：节点元数据 `viewer.url` 存在时渲染报告 iframe（init 携带 node_report），实时转发 debug store 的 grpc 流消息
- `DynamicPluginNode` 增加 `grpc:connect` 徽章（`🏛services ⇄methods ⤳streaming`，violet）
- `pluginRequestActions.ts` `openFileDialog` 增强（`multiple`/`filters`/`directory` 参数）

## 4. 验证

- 插件：`cargo test`（72 单元 + 6 roundtrip + 4 集成：全生命周期 connect→unary→streaming→cancel→channelz→health→close→flow_ended、反射发现、长流取消、骨架生成）
- 宿主依赖差集：`cargo tree -p flow-engine-executor --no-default-features --features protocol-http,protocol-ws,protocol-script,protocol-assertion,protocol-tcp,protocol-sse,protocol-graphql -e normal` 无 tonic/prost/protox/axum；`cargo hakari generate` 后 workspace-hack 0 命中
- `cargo nextest run`、`npm run typecheck`、`npm run test` 全量通过
- UI 手工：画布添加 `grpc:connect` → iframe 面板 → 测试连接 & 发现 → 服务树 → `grpc:call` 调用 → 报告 viewer 实时流 → cancelStream 停流 → 保存/重载后服务树与徽章保留
