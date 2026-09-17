# MPE 数据源插件方案（定稿 v1.0）

> 状态：定稿 v1.0（2026-08-20）。基于与宿主架构的对抗式讨论，决策编号 D1-D8。
> 实施以本文档为准。标注 [待验证] 的条目需在首个数据源实现时用真实数据确认。

---

## 1. 背景与目标

### 1.1 现状痛点

用户已在外部 API 管理平台（Eolink / Postman / Apifox / YApi）维护了大量 API 定义（URL、参数、请求头、请求体示例、响应），到 MPE 中搭建流程时需要**从零重建**这些数据：手填 url、method、headers、body。数据越多，重复成本越高。

### 1.2 目标

- 数据源插件化：插件从外部平台拉取 API/其它协议数据，用户在 MPE 中浏览并**拖拽进画布**，生成配置齐全的节点
- 新增数据源 = 新增插件，宿主零改动（与协议插件同一生态理念）
- 首期数据源：Eolink（https://www.eolink.com）

### 1.3 非目标（明确不做）

- **不管理数据**：不做存储、不做双向同步、不写回平台。插件只读
- 不建立"宿主本地 API 库"（多 flow 引用的共享条目库），拖入即快照
- 数据源插件不参与流程执行（无 execute 路径）

---

## 2. 核心设计决策

| # | 决策 | 理由 |
|---|---|---|
| **D1** | 数据源是独立 kind 的插件，**不复用协议插件 execute 模型**，但复用 sidecar 基础设施 | 数据源是"编辑时"能力（浏览+拖拽），协议插件是"执行时"能力（flow 运行中 execute）。硬塞 NodePlugin trait 会污染两边契约。sidecar 的 discovery/manifest/quarantine/生命周期/JSON-RPC 传输/市场分发/前端 inline 面板全部复用 |
| **D2** | **转换归插件，宿主只收 `NodeTemplate`**。平台原生格式是输入，NodeTemplate 是唯一输出契约 | MPE 节点模型（NodeType + NodeData）本身就是多协议 DSL，不需要第三个"中间格式"。插件做"平台格式 → 节点模板"转换，宿主保持零转换器、彻底冻结。加新平台只写插件 |
| **D3** | 一平台一插件（推荐）；一个插件可声明多个 `datasources[]`（同家族强耦合场景） | 进程粒度 = 版本粒度 = 隔离粒度。平台 A 改版不连坐平台 B；一个数据源崩溃不 quarantine 其它数据源。Postman 云 + 本地集合这类同认证体系家族允许单插件多数据源 |
| **D4** | 数据源 SDK 并入现有 `mpe-plugin-sdk`（同仓库同 crate），不新起项目 | SDK 已有 `protocol/`（wire 类型，宿主 `default-features=false` 消费）与 `runtime/`（tokio，插件用）两个槽位，数据源契约正好落位。传输层（framing 2MiB 上限/背压/reader 容错/describe 超时 kill）是最贵最安全敏感的部分，新起 crate = 复制 = 漂移 |
| **D5** | 浏览 UI：**插件归一化字段，宿主渲染通用树**。P0 插件不写 UI | 列表行粒度不适合 inline HTML（性能/内存）；宿主统一视觉，协议徽标复用节点图标/颜色映射；归一化是"数据整形"，比写 UI 简单一个量级。P2 开放 inline 面板逃生舱 |
| **D6** | 数据源连接配置：**describe 声明 `config_schema`，宿主渲染通用表单，持久化到宿主 store**（`store/datasources.bin`） | 与现有节点 `config_schema` 动态表单同一模式；凭据属于用户环境而非插件包；连接配置不进 flow 文件 |
| **D7** | 拖入画布 = **快照**。生成自包含普通节点，之后与数据源无关联；节点 data 记录 `source_ref`（P2 用于可选的"重新同步"） | flow 文件独立、可分享、可离线执行。这是"减少重建"的核心价值 |
| **D8** | 支持平台自身多协议数据（HTTP/WS/SSE/...），不做 OpenAPI 单格式限制 | 本工具定位多协议；OpenAPI 只覆盖 HTTP，做唯一中间格式会丢平台特有协议数据 |

---

## 3. 架构总览

```
┌──────────────────────────── 宿主 (mpe) ────────────────────────────┐
│                                                                     │
│  ┌──────────────────────┐        ┌──────────────────────────────┐  │
│  │ 左侧栏 DataSourcePanel │ 拖拽   │ FlowCanvas.onDrop            │  │
│  │  (通用树渲染, 协议徽标)  │───────▶│  分支: template → 创建节点    │  │
│  └──────────┬───────────┘        └──────────────┬───────────────┘  │
│             │ browse/get_item/                   │ NodeTemplate[]   │
│             │ get_templates/validate_config      │                  │
│             ▼                                   ▼                  │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │ DataSourceManager（宿主侧）                                  │    │
│  │  ├─ 数据源插件注册表（describe 结果）                         │    │
│  │  ├─ 连接配置注入（env，仿 providers）                        │    │
│  │  └─ RPC 转发 + 错误归一化                                    │    │
│  └───────────────────────────┬────────────────────────────────┘    │
│                              │ JSON-RPC (LF 分隔)                   │
└──────────────────────────────┼────────────────────────────────────┘
                               ▼
        ┌──────────────────────────────────────┐
        │  数据源插件 sidecar（任意语言）         │
        │  eolink-plugin/                       │
        │  ├─ plugin.json (kind: "datasource")  │
        │  └─ plugin.py                         │
        │     ① 认证 ② 拉取 ③ 转换 → NodeTemplate │
        └──────────────────────────────────────┘
```

---

## 4. 插件契约（wire）

### 4.1 plugin.json

```json
{
  "name": "eolink",
  "version": "1.0.0",
  "kind": "datasource",
  "entry": { "command": "python", "args": ["plugin.py"] },
  "min_host_version": "0.1.0"
}
```

- `kind` 缺省为 `"protocol"`（现有插件零影响）；`"datasource"` 声明数据源插件
- 数据源插件不需要 `capabilities.streaming`（无执行流）；面板交互期间按需 spawn，空闲按默认策略回收

### 4.2 describe（数据源 kind 返回）

```json
{
  "plugin": { "name": "eolink", "kind": "datasource" },
  "datasources": [{
    "id": "eolink",
    "display_name": "Eolink",
    "auth": { "type": "api_key" },
    "config_schema": {
      "type": "object",
      "properties": {
        "api_key":  { "type": "string", "format": "password" },
        "space_id": { "type": "string" }
      },
      "required": ["api_key"]
    },
    "resources": [
      { "id": "project", "label": "项目", "children": "api" },
      { "id": "api",     "label": "接口", "leaf": true }
    ]
  }]
}
```

- `config_schema`：连接配置表单 schema（宿主通用表单渲染，同节点 config_schema 模式）。`auth.type` 取值 `api_key` | `oauth2` | `none`
- `resources`：可浏览的资源层级，宿主按此渲染树。`leaf: true` 的节点可拖拽

### 4.3 RPC 方法

| 方法 | 方向 | 参数 | 返回 | 说明 |
|---|---|---|---|---|
| `describe` | 宿主→插件 | 同协议插件 | 见 4.2 | 握手（复用现有机制） |
| `browse` | 宿主→插件 | `resource_id`, `parent_id?`, `page?`, `size?` | `DataItem[]` | 列表/分页 |
| `get_item` | 宿主→插件 | `resource_id`, `item_id` | `DataItemDetail` | 详情（预览面板） |
| `get_templates` | 宿主→插件 | `item_ids[]` | `NodeTemplate[]` | **转换：平台条目 → 可落画布节点**。支持批量 |
| `validate_config` | 宿主→插件 | 连接配置 | `{ valid, message? }` | 保存连接时即时验证 |

不需要：execute / execute_stream / cancel / flowEnded / provider（数据源无执行语义）。未知方法按现有约定返回 JSON-RPC `-32601`。

### 4.4 DataItem（浏览条目，协议无关）

SDK 定义协议枚举，对应宿主节点家族：

```rust
pub enum DataItemProtocol { Http, WebSocket, Sse, Tcp, Graphql, Mqtt, Kafka, Other }
```

插件把平台自身协议标记映射到该枚举；认不出的 → `Other` + 透传显示名。宿主因此不需要知道任何平台的协议词汇表。

```json
{
  "id": "api-123",
  "name": "获取用户列表",
  "protocol": "http",
  "method": "GET",
  "path": "/users",
  "summary": "分页获取用户",
  "extra": [{ "label": "标签", "value": "v1" }],
  "raw": { }
}
```

- `extra`：平台特有字段，宿主以通用 key-value 渲染
- `raw`：平台原始数据透传，`get_templates` 阶段使用

### 4.5 NodeTemplate（唯一输出契约）

```json
{
  "type_id": "http:request",
  "name": "GET /users",
  "data": {
    "method": "GET",
    "url": "https://api.example.com/users",
    "headers": { "Authorization": "Bearer {{token}}" },
    "body_type": "none"
  },
  "source_ref": {
    "datasource": "eolink",
    "item_id": "123",
    "updated_at": "2026-08-20T10:00:00Z"
  }
}
```

- `type_id`：内置或插件节点类型 id（如 `mongo:find`），落画布时按注册表校验
- `data`：节点配置，**直接**作为画布节点 data（覆盖默认值）
- 单个平台条目可产出**多个**模板（如 Eolink WS API → `ws:connect` + `ws_send` 两个节点）
- `source_ref`：P0 只存不消费；P2 支持"重新同步"（拉最新覆盖节点数据）
- 大小上限：单模板 ≤ 1 MiB（对齐现有 report_data 上限）

### 4.6 SDK 落位（D4）

```
mpe-plugin-sdk/src/
├── protocol/
│   ├── mod.rs
│   ├── ...（现有协议插件 wire 类型）
│   └── datasource.rs      # 新增：DataItem/DataItemProtocol/NodeTemplate/
│                          #       browse/get_item/get_templates 请求响应类型（纯 serde）
└── runtime/
    ├── ...（现有 Plugin trait / 事件循环 / 分发）
    └── datasource.rs      # 新增：DatasourcePlugin trait（browse/get_item/
                           #       get_templates/validate_config + 默认空实现）
```

- 宿主经现有 `default-features = false` 消费 `protocol::datasource`，零新依赖、无 tokio
- 协议插件作者不受影响（runtime 仅多一个 trait；如后续需要可加 feature 门控）
- 版本：随 SDK 同仓库同 tag 发布（现 v0.2.2 机制）

---

## 5. 前端设计

### 5.1 展示位置：左侧栏新增面板

左侧栏（VSCode 风格面板容器）新增"数据源"面板（可折叠），与 FlowListPanel 同区：

- 顶部为数据源列表：每项显示名称 + 连接状态（未连接/已连接/连接失效）
- 展开后按 `resources` 层级渲染树：项目 → 接口
- 条目行：协议徽标（复用 `nodeIcons` 映射，颜色与画布节点一致）+ method 徽章 + 名称 + path

选左侧栏而非 NodePalette：数据源是树形浏览 + 长列表 + 持续可见；NodePalette 是画布内搜索弹出面板（400×420），语义是"选节点类型"，两者不冲突。

### 5.2 新增模块

| 模块 | 内容 |
|---|---|
| `components/sidebar/DataSourcePanel.tsx` | 面板渲染 |
| `stores/datasource.ts` | Zustand store：连接配置列表、浏览状态、列表缓存（实现 `reset()`） |
| `services/tauri/datasource.ts` | IPC 封装 |

后端：`flow-engine-plugin` 内新增 datasource manager（数据源插件注册表 + RPC 转发 + 连接配置注入），或独立 `flow-engine-datasource` crate（实现时定，倾向独立 crate 保持插件宿主 crate 聚焦）。

### 5.3 拖拽扩展

现有 `FlowCanvas.onDrop`（`application/reactflow/nodeType`）扩展：

- 数据源条目拖拽额外携带 `application/reactflow/datasource`：`{ item_ids: string[], datasource_id: string }`
- `onDrop` 分支：先 `get_templates(item_ids)` → 得到 `NodeTemplate[]` → 逐个以模板创建节点（template.data 覆盖 `createDefaultNodeData` 结果），落到鼠标位置或网格排布
- `type_id` 不在注册表 → 拒绝 + 告警（插件版本过期等场景）

### 5.4 缓存与刷新

- 浏览列表：面板内缓存 + 手动刷新按钮；每次展开拉最新（平台数据实时性优先）
- 转换：拖拽时实时 `get_templates`（单条目 ~1s 可接受），不做预取
- 错误：平台不可达/超时/认证失效 → toast + 行内状态；认证失效提示"重新配置连接"

---

## 6. 配置与存储（D6）

三层配置各归其位：

| 配置 | 存放 | 谁写 | 进 flow 文件？ |
|---|---|---|---|
| 插件自身（plugin.json/entry/env） | 插件目录（`~/.mpe/plugins/<p>/`） | 插件作者，随插件分发 | 否 |
| **数据源连接**（API key/base url） | 宿主 store `store/datasources.bin` | 用户在 UI 填，宿主持久化 | **否**（凭据不随流程分发，多 flow 共享） |
| 节点数据（拖入快照） | flow 文件节点 data | 拖拽时宿主写入 | 是 |

```json
{
  "connections": [{
    "id": "uuid",
    "datasource_id": "eolink",
    "display_name": "我的 Eolink",
    "config": { "api_key": "xxxx", "space_id": "yyy" },
    "created_at": "2026-08-20T10:00:00Z"
  }]
}
```

- 用户**不手写配置文件**：与现有 `settings.bin`（pluginDir/locale/logDir）同一模式，UI 改 + store 存
- 插件进程运行时不读 store：宿主把连接配置注入插件进程 env（仿 providers 传递），插件无状态
- 凭据安全：P0 store 明文（对齐现有 settings 模式）；P1 系统钥匙串加密

---

## 7. 首个数据源：Eolink

### 7.1 端点（社区 MCP 实现 scarqin/mcp-apikit 已验证）

| 用途 | 端点 |
|---|---|
| 项目列表 | `GET https://api.eolink.com/projects` |
| 项目详情 | `GET /projects/{pid}` |
| API 列表 | `GET /v3/api-management/apis?project_id=X&page=1&size=999[&space_id=Y]` |
| API 详情 | `GET /projects/{pid}/apis/{aid}` |
| 搜索 | `GET /search/apis?q=` |

认证：header 携带 API key（`EOLINK_API_KEY` 模式）。响应统一 `data` 包装（列表为 `data.items`）。

### 7.2 映射

- HTTP 条目 → `http:request` 节点：method/path/headers/params/body example → 节点 data
- WS 条目（[待验证] Eolink OpenAPI 返回是否含 WebSocket 定义及字段结构）→ `ws:connect` + `ws_send` 模板
- base url 来源 [待验证]：优先 Eolink 项目配置字段；缺失则拖入时用户填一次（存入模板 data，不落连接配置）

### 7.3 参考实现

Eolink 插件作为数据源插件的官方参考实现（dogfood 契约）。Python 模板实现（requests + JSON 整形，~100-200 行）。验证清单：

- [ ] 抓真实项目返回，确认 `protocol` 字段与 WS 定义结构
- [ ] 认证失效（key 过期）的错误表现与宿主错误归一化
- [ ] 分页（size=999 上限，大项目游标行为）
- [ ] base url 实际来源

---

## 8. 生命周期与安全

- **进程生命周期**：复用现有 on-demand spawn + 空闲回收（数据源插件不声明 streaming）；面板交互期间进程保持活跃
- **隔离**：describe 失败 3 次 → quarantine（现有机制，数据源插件同规则）
- **凭据注入**：仅面板/浏览会话注入，不落日志
- **只读边界**：契约无任何写回方法；插件不应实现平台写入（D2/D3 边界）
- **帧上限**：复用 2 MiB 帧上限；超大数据集由插件分页控制

---

## 9. 里程碑

### P0（首个可用）

- SDK：`protocol::datasource` wire 类型 + `DatasourcePlugin` trait
- 宿主：datasource manager（注册/转发/连接注入）+ 连接配置 store + `store/datasources.bin`
- 前端：DataSourcePanel + datasource store + IPC + onDrop 拖拽扩展
- Eolink 插件：HTTP 条目 → http:request 节点（含 [待验证] 项）

### P1

- OpenAPI 适配器（SDK 共享转换库，HTTP 类数据源大量复用）
- 通用导入数据源（OpenAPI URL / 文件拖入，宿主内置，不占平台插件名额）
- 批量拖拽（项目/文件夹级多选 → 多节点排布）
- `source_ref` 消费：节点"重新同步"按钮
- 系统钥匙串凭据加密

### P2

- 数据源 inline 面板逃生舱（插件自绘 UI，复用协议插件 inline 机制）
- AsyncAPI 适配器（WS/MQTT 等异步协议标准格式）
- Postman Collection 适配器

---

## 10. 决策记录汇总

D1 独立 kind 不复用 execute；D2 转换归插件、NodeTemplate 唯一契约；D3 一平台一插件；D4 SDK 同仓库；D5 宿主渲染通用树；D6 配置宿主 store + describe schema；D7 拖入即快照 + source_ref；D8 支持平台多协议数据。
