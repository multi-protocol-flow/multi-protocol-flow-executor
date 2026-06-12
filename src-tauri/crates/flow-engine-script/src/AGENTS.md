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
| `fn.variables` | `get(key)`, `set(key, value)`, `has(key)`, `keys()`, `all()` |
| `fn.response` | `status`, `body`, `headers`, `rawBody`, `size`, `time`, `rawOutput` |
| `fn.request` | `url`, `method`, `headers`, `body` |
| `fn.console` | `log()`, `info()`, `warn()`, `error()`, `debug()`, `table()` |
| `fn.flow` | `skip()`, `abort(msg?)`, `stop(msg?)` |
| `fn.info` | `nodeId`, `nodeName`, `flowId`, `flowName` |
| `fn.util` | `base64.encode()`, `base64.decode()`, `hex.encode()`, `hex.decode()`, `json.parse()`, `json.stringify()`, `uuid.v4()`, `timestamp()`, `sleep(ms)`, `hash.md5()`, `hash.sha1()`, `hash.sha256()` |
| `fn.expect` | `expect(actual)` — chain: `.toBe(expected)`, `.toEqual(expected)`, `.toContain(sub)`, `.toBeTruthy()`, `.toBeFalsy()`, `.toBeGreaterThan(n)`, `.toBeLessThan(n)`, `.toMatch(regex)`, `.not.*` |

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
