# mpe-cli/src

**Generated:** 2026-08-05 | **纯 CLI 二进制 crate**

## OVERVIEW

独立纯 Rust CLI 二进制，面向 Linux 服务器、CI/CD、Docker。14 个源文件，~2,500 行。同时为二合一 `mpe` 二进制提供 `cli` 模块（GUI 无参启动，有参走 CLI）。依赖 `flow-engine` 且 `default-features = false`（按需转发 feature）。

## STRUCTURE

```
src/
├── main.rs        # 二进制入口: 调 run_cli() 后 std::process::exit
├── lib.rs         # pub mod cli; pub use cli::run_cli; (供 app crate 复用)
└── cli.rs         # clap derive: Cli + Commands + async_main() 分发 + run_cli() 同步包装
    ├── config.rs      # 全局配置
    ├── debug_cmd.rs   # debug 模式 NDJSON 协议
    ├── flow.rs        # flow list/show/create/copy/delete/import
    ├── flow_edit.rs   # node/connection/variable 编辑
    ├── output.rs      # 输出格式化 (json/table)
    ├── report.rs      # report list/show/delete
    ├── run.rs         # 执行流程
    ├── run_node.rs    # 单节点连通性检查
    ├── stress.rs      # stress run/report
    └── validate.rs    # 流程验证
```

## WHERE TO LOOK

| Task | File |
|------|------|
| 新增子命令 | `cli.rs` Commands 枚举 + 新建 `cli/xxx.rs` + `cli.rs` 加 `pub mod` |
| 执行流程 | `cli/run.rs` |
| 流程验证 | `cli/validate.rs` |
| Debug 断点/NDJSON 流 | `cli/debug_cmd.rs` |
| 压测 | `cli/stress.rs` |
| 流程 CRUD | `cli/flow.rs` |
| 流程编辑 (node/connection/variable) | `cli/flow_edit.rs` |
| 报告查询 | `cli/report.rs` |
| 输出格式 (json/table) | `cli/output.rs` |
| 全局配置 | `cli/config.rs` |
| 单节点连通性预检 | `cli/run_node.rs` |
| CLI 集成测试 | `tests/cli_tests.rs` (assert_cmd + predicates + tempfile) |
| 测试夹具 | 仓库根 `tests/cli/` (JSON flow + FlowFile 包装) + `tests/fixtures/flows/` |

## CONVENTIONS

- 输出统一走 `cli/output.rs`（json/table 二选一，默认 json），不直接 println 原始结构
- `run_cli()` 是同步入口，`main.rs` 和 app crate 的 `main.rs` 都经它进入；内部 `async_main()` 做实际分发
- 退出码由 `run_cli()` 映射 `CliError`：成功 0，`InvalidArgs`/`Io` 1，`Config`/`Execution` 2
- `CliError` 含 `Config`/`Execution`/`InvalidArgs`/`Io` 变体，有 `From<std::io::Error>` 实现
- 全局 flag `--format json|table`；`--var KEY=VALUE` 注入初始变量
- 插件 e2e 测试通过 env `MPE_PLUGIN_DIR` 指向 `tests/plugin-fixtures/pyecho/`（plugin.json + plugin.py）
- 协议 feature 按需转发：`--no-default-features -F protocol-http -F protocol-script` 控制体积

## ANTI-PATTERNS

- 误改 `src-tauri/src/cli.rs` + `src-tauri/src/cli/`（过期副本，不参与编译；真实 CLI 在此 crate，app 通过 `pub use mpe_cli::cli;` 引入）
- 绕过 `output.rs` 直接拼 JSON 输出 → 破坏 --format 一致性
- 生产代码 `unwrap()` → 应传播为 `CliError` 并映射退出码
- 新子命令漏注册 `pub mod` / 漏加入 Commands 枚举 → 编译不通过或不可达
