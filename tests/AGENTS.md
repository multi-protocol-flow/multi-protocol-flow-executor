# 测试夹具与集成测试支撑

**生成时间:** 2026-08-05 | **Branch:** feature/plugins

## OVERVIEW

CLI 测试夹具 + 集成测试支撑数据。75 个文件、58 个 JSON 流程文件，覆盖 CLI e2e 场景、验证/执行 fixture、TLS 证书与本地测试服务。消费方：`src-tauri/tests/integration.rs`（通过 `fixtures_dir()` 加载）与 `mpe-cli`。

## STRUCTURE

```
tests/
├── cli/                        CLI e2e 场景流程（FlowFile 包装格式）
│   ├── 01_simple_entry_http.json → 09_full_workflow.json
│   │                           渐进式场景：HTTP 方法/头/体、条件操作符、
│   │                           变量引用、多 HTTP 链、带分支全流程
│   ├── http-upload.json, script_node_example.json
│   ├── tls_01_basic_ca_verify.json → tls_05_wrong_ca.json
│   ├── edge_cases/            12 个编号文件，刻意触发错误（空流程、无 entry、
│   │                          JSON 语法错误、孤立节点、缺 URL…）
│   ├── 10_http_body_types/    none/text/json/form 请求体类型
│   ├── websocket/
│   └── README.md              FlowFile schema、节点/连接结构、10 个条件操作符、
│                              变量引用语法、文件目录
├── fixtures/                   集成测试的验证/执行 fixture
│   ├── flows/                 01_valid_start_with_connection.json →
│   │                          10_complete_example.json + 协议流
│   │                          （assertion_flow.json、imap_fetch_unread.json、
│   │                          smtp_send_email.json、ws_basic_connect.json…）
│   └── test-upload.txt         上传 fixture
├── certs/                      TLS 证书 + generate-certs.ps1（再生成脚本）
├── https-test-server.mjs       编排器：拉起 TLS 服务，对编译产物
│   + run-tls-tests.mjs         src-tauri/target/debug/mpe.exe 运行 tests/cli/tls_*.json
└── graphql-subscription-server/  Node GraphQL subscription 测试服务
```

## WHERE TO LOOK

| 任务 | 位置 |
|------|------|
| CLI e2e 场景 | `cli/01-09` 编号流，`cli/README.md` 有文件目录 |
| 错误路径验证 | `cli/edge_cases/` |
| 集成测试 fixture | `fixtures/flows/`（`integration.rs` 经 `load_flow_fixture()` 解包 `flow` 键） |
| TLS 校验场景 | `cli/tls_*.json` + `certs/`，经 `run-tls-tests.mjs` 跑编译后的 debug 二进制 |
| 上传/脚本场景 | `cli/http-upload.json`、`cli/script_node_example.json` |
| 协议消费端参考 | `src-tauri/tests/integration.rs` 的 `fixtures_dir()`（CARGO_MANIFEST_DIR 上跳一级 → `tests/fixtures/flows/`） |

## CONVENTIONS

- **FlowFile 包装格式**: 顶层 `_version` / `flow`（`_meta` 可选，serde 默认补全；现有夹具均只带 `_version` + `flow`）；节点用 `entry` 类型，非旧版 `start`/`end`。
- **编号前缀**: 两位数前缀（`01-…`）决定执行/展示顺序；`edge_cases/` = 错误触发专用。
- **tls_* 配对**: `tls_*` 文件与 `certs/` 证书配对，通过 `tests/run-tls-tests.mjs` 对已编译的 debug 二进制运行。
- **网络依赖**: 依赖网络的流程用 httpbin.org 或 `echo/` Docker 服务。
- **schema 文档**: 文件格式、10 个条件操作符、变量引用语法（`{{xxx}}`、`{{http_response.data.xxx}}`、`{{http_response.status}}`）以 `cli/README.md` 为准。

## ANTI-PATTERNS

- 随意重命名 fixture 文件 — 编号前缀被 `run-tls-tests.mjs` 和 CLI 测试文档引用，改名即破坏用例。
- 在 fixture 里存真实凭据 — 流程文件是共享测试数据。
- 手改 `certs/` 证书 — 已检入仓库；需变更时用 `generate-certs.ps1` 再生成。
