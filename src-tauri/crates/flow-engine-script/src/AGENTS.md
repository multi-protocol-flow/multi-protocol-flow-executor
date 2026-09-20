# flow-engine-script

**Generated:** 2026-06-12 | **Branch:** dev | **Layer:** 1 (Protocol) | **Depends on:** flow-engine-core

## Overview

Script protocol crate. Embeds a QuickJS (rquickjs) runtime to execute JavaScript in flow nodes. 5 source files, ~4,200 lines.

## Files

| File | Lines | Role |
|------|-------|------|
| `lib.rs` | 10 | Crate root: exposes `plugin`, `script_api`, `script_executor`. `script_poc` behind `#[cfg(test)]`. |
| `script_api.rs` | ~2,932 | Rust-JS bridge. `ScriptExecutionContext` holds variables, logs, response, request, flow control flags. Registers all `fn.*` functions into the QuickJS global scope. |
| `script_executor.rs` | ~973 | `ScriptNodeExecutor` implementing `NodeExecutor`. Creates isolated QuickJS runtime per execution, injects context, enforces timeout/memory limits, parses script output. |
| `plugin.rs` | ~100 | `ScriptPlugin` implementing `NodePlugin`. Two output ports: `"true"` (success) / `"false"` (failure). Resolves port from `$.success` boolean in output. Default config includes timeout, memory limit, error strategy. |
| `script_poc.rs` | ~218 | `#[cfg(test)]` POC tests verifying QuickJS execution, global object registration, timeouts, JSON conversion, memory limits. |

## Key APIs (fn.*)

Registered on the `fn` global object in QuickJS:

| Namespace | Functions |
|-----------|-----------|
| `fn.variables` | `get(key)` (→ string \| null), `replaceIn(template)`, `set(key, value)` (任意 JS 值按 JSON 类型保存), `has(key)`, `unset(key)`, `clear()`, `setAll(obj)`, `all()` (→ JSON 文本) |
| `fn.response` | `code`, `status`, `headers`, `body` (JSON 文本), `raw` (完整 last-node output，JSON 文本), `size`, `time`, `text()`, `json()` — 仅当一个上游节点产出过响应时存在 |
| `fn.request` | `url`, `method`, `headers`, `body` |
| `fn.console` | `log()`, `info()`, `warn()`, `error()`, `debug()` |
| `fn.flow` | `skip()`, `abort(msg?)`, `stop(msg?)` |
| `fn.info` | `nodeId`, `nodeName`, `flowId`, `flowName` |
| `fn.util`（标量） | `base64Encode/Decode`, `urlEncode/Decode`, `htmlEscape/Unescape`, `md5`, `sha256`, `sha512`, `hmacSha256(key, message)`, `hmacSha512`, `uuid()`, `uuidv4()`, `timestamp()` (ms), `now()`, `formatTime(fmt)`, `parseTime(str, fmt)`, `sleep(ms)`, `typeOf/isArray/isObject/isString/isNumber/isBoolean/isNull/isUndefined/isEmpty` |
| `fn.util`（对象/JSON） | `keys(v)`, `values(v)`, `entries(v)`, `hasKey(v, k)`, `pick(v, keys)`, `omit(v, keys)`, `merge(target, source)`, `clone(v)`, `jsonStringify(v, indent)`, `jsonPath(v, path)`, `jsonParse(str)` — 除 `hasKey` 外均返回 JSON 文本；键列表可用原生数组或 `'["a"]'` 字符串；入参经 `js_value_to_json` 递归转换（含深度上限，循环引用按 null 截断） |
| `fn.expect` | `expect(actual)` — 修饰符 `.to` / `.be` / `.have` / `.not`；断言方法 `.equal(v)`（严格）、`.eql(v)`（深比较）、`.notEqual`、`.exist`、`.beDefined`、`.beNull`、`.notBeNull`、`.beTrue`、`.beFalse`、`.beTruthy`、`.beFalsy`、`.contain`、`.containKey`、`.haveLength`、`.match`、`.startWith`、`.endWith`、`.above`、`.below`。**没有 `toBe`/`toEqual`**；断言结果只记录（节点输出 `test_results` / `all_assertions_passed`），失败不改变节点状态 |

## Execution Flow

1. `plugin.rs::execute()` creates `ScriptNodeExecutor` and delegates.
2. `script_executor` creates isolated QuickJS `Runtime` + `Context`.
3. `script_api::register_all()` binds all `fn.*` functions onto the global scope.
4. User script runs, reads/writes variables via `fn.variables.*`, logs via `fn.console.*`, controls flow via `fn.flow.*`.
5. Executor collects modified variables, logs, test results, and flow signals.
6. Output JSON includes `$.success` (bool) for port routing.

## Error Strategies

- `fatal` — script failure propagates as node error.
- `recoverable` — catches exceptions, sets `$.success = false`, continues to "false" port.
