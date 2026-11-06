# flow-engine-exec — 本地命令 / 本机信息节点

**Crate:** `flow-engine-exec` | **Layer:** 2.5（被 `flow-engine-executor` 以 feature `local-exec` 依赖） | **Nodes:** `local:exec`、`local:sysinfo`

## WHY THIS CRATE EXISTS

两个节点都必须在**宿主进程内**运行（而非 sidecar 插件）：`local:exec` 的防注入语义依赖
"变量替换**之前**的模板原文"，而插件拿到的 config 是宿主已替换过的字符串。

## WHERE TO LOOK

| 任务 | 位置 |
|---|---|
| 执行策略（总开关 / 白名单 / shell 许可 / 审批模式） | `src/policy.rs` |
| 审批请求、信任记录、命令哈希 | `src/approval.rs` |
| 配置 → 可执行命令（变量替换、环境清洗、路径解析） | `src/spec.rs` |
| 子进程执行（超时 / 取消 / 输出上限 / 进程组终止） | `src/process.rs` |
| 产物捕获（复用媒体预览 `media` 契约） | `src/media.rs` |
| 本机信息探测（只读、硬编码探针） | `src/sysinfo.rs` |
| 两个 `NodePlugin` 实现 | `src/exec_plugin.rs`、`src/sysinfo_plugin.rs` |

## 关键不变量

- **argv 模式不拆词**：`args[]` 逐参数 `replace_variables` 后作为单个 `Command::arg` 传递，零注入面。
- **shell 模式禁模板**：`script` 含 `{{` 在 `validate` 与 `build_command` 双重拒绝；脚本只能读 `MPE_VAR_*`。
- **环境清洗**：`env_clear()` + 白名单继承（`PATH HOME TMPDIR TEMP TMP LANG LC_ALL SYSTEMROOT WINDIR COMSPEC PATHEXT`）+ `inherit_env`，所有 `MPE_*` 一律不继承。
- **缺省即拒绝**：provider 缺失 / `enabled == false` / 白名单拒绝 / 未批准 → `E_EXEC_DENIED`（走 `on_error`，非致命）。
- **非 0 退出走 `Ok` + `false` 端口**：保留 stdout/stderr，executor 对 `used_port_id == "false"` 已按失败上报并取 `output_data.error`。
- **超时/取消必须由阻塞线程观察**：`spawn_blocking` 的 `JoinHandle` 被 drop 时线程继续运行，executor 取消只 drop future。

## ANTI-PATTERNS

- 把 `script` 也做变量替换 → 变量值进 shell 字符串，注入面重现。
- 用 `with_provider` 之后又 `with_extra_providers`（后者是**整体替换**） → 丢掉已注入的 provider（Ingress 的响应通道曾因此丢失，见 `ingress/server.rs`）。
- 在生产代码 `unwrap()` / `expect()` 于用户输入路径（`unwrap_used = deny`）。
- 在此 crate 里复制 `supervisor/process.rs` 的单 pid 语义 → 子进程组无法整组终止。
