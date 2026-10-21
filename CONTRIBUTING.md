# Contributing to MPE Flow

Thanks for taking the time to contribute. This document covers the mechanics;
the architecture and coding conventions live in [`AGENTS.md`](./AGENTS.md) and
the per-directory `AGENTS.md` files.

## License of contributions

MPE Flow is released under the [MIT License](./LICENSE).

By submitting a pull request you agree to the **Developer Certificate of
Origin (DCO) 1.1** — you certify that you wrote the contribution, or that you
have the right to submit it under the project's license. This is a lighter
alternative to a CLA: it grants the project the same rights you already have
over your own work, and it keeps the project able to relicense or reorganise
crates later without chasing every past contributor.

Every commit must be signed off:

```bash
git commit -s -m "fix(http): correct timeout propagation"
```

which appends:

```
Signed-off-by: Your Name <you@example.com>
```

Pull requests with unsigned commits will fail the DCO check. Amend with
`git rebase --signoff HEAD~N` if you forgot.

## Before you start

- **Bug reports and feature requests**: open an issue using one of the
  templates in [`.github/ISSUE_TEMPLATE/`](./.github/ISSUE_TEMPLATE).
- **Large changes**: open an issue first. Anything touching the flow data
  model, the plugin wire protocol, or the report schema needs agreement before
  code — those are versioned contracts consumed by `.mpf` files, plugins, and
  CI pipelines.
- **Security issues**: do **not** open a public issue. See
  [`SECURITY.md`](./SECURITY.md).

## Repository layout

This repository contains the **host** only: the execution engine, the Tauri
desktop application, the CLI, and the plugin host.

The protocol plugins (`redis`, `db`, `mongo`, `grpc`, `kafka`, `smtp`, `mcp`,
`llm`, `agent`, `openapi`, `postman`, `eolink`) live in **separate
repositories** under the [`multi-protocol-flow`](https://github.com/multi-protocol-flow)
organisation and are **not** part of this working tree — `plugins/` and `echo/`
are git-ignored. To run the full test matrix you need to clone the plugins you
care about and build them individually:

```bash
git clone https://github.com/multi-protocol-flow/mpe-plugin-redis.git
cd mpe-plugin-redis && cargo nextest run
```

The plugin SDK is [`mpe-plugin-sdk`](https://github.com/multi-protocol-flow/mpe-plugin-sdk),
consumed here as a tagged git dependency.

## Development setup

Requirements: **Rust >= 1.85.0**, **Node.js >= 18**, and a package manager
(`npm` is what CI uses).

```bash
npm install
npm run tauri dev     # desktop app with hot reload
npm run dev:web       # browser-only frontend against mock IPC
```

Backend-only work does not need the frontend toolchain:

```bash
cd src-tauri
cargo build -p mpe-cli
```

## Checks you must pass

CI runs these; run them locally first.

| Area | Command |
|---|---|
| Rust tests | `cd src-tauri && cargo nextest run` |
| Rust doctests | `cd src-tauri && cargo test --doc` |
| Rust lints | `cd src-tauri && cargo clippy` |
| Formatting | `cd src-tauri && cargo fmt` |
| Workspace hack freshness | `cd src-tauri && cargo hakari generate --diff` |
| TypeScript | `npm run typecheck` |
| Frontend tests | `npm run test` |
| i18n keys | `npm run typesafe-i18n` (commit the regenerated `i18n-types.ts`) |

Hard rules enforced by the compiler and CI:

- `unsafe_code` is forbidden workspace-wide (the `app` crate allows it only for
  the Windows `FreeConsole` call).
- No `unwrap()` in production paths — propagate with `?`. `expect_used` is a
  warning, `unwrap_used` is a deny.
- No `as any` / `@ts-ignore` / `@ts-expect-error` in TypeScript.
- New i18n keys must be added to **both** `zh-CN` and `en-US` and to
  `i18n-types.ts`, otherwise the build fails.
- Only ES2020 runtime APIs are allowed in the frontend (see `AGENTS.md` for why).

## Commit messages

Conventional Commits, scoped by area:

```
feat(canvas): support inline node rename
fix(executor): propagate cancellation into teardown
docs(plugin): correct the manifest schema
```

Keep unrelated changes in separate commits. Do not mix formatting-only churn
with behaviour changes.

## Pull requests

- Target `master`. Long-lived work happens on `dev`.
- Describe **what changed and why**, not just what files were touched.
- Add a `CHANGELOG.md` entry under `[Unreleased]` for any user-visible change.
  This file is the single source of truth for release notes and feeds both the
  in-app updater and the website.
- Include a test that fails before and passes after for bug fixes. If a test is
  impractical, say so and describe how you verified the change manually.
- Screenshots or a short recording for UI changes.

## Adding a protocol plugin

Plugin authors do **not** need to modify this repository. Start with
[`docs/plugin-sdk.md`](./docs/plugin-sdk.md), the wire protocol in
[`docs/plugin-protocol.md`](./docs/plugin-protocol.md), and the runnable
templates in [`sdk-templates/`](./sdk-templates) (Python and Node.js).
`plugin.json` plus an entry script is enough to register new `<vendor>:<node>`
types at runtime.

Changes to the plugin **wire protocol** itself are contract changes: open an
issue against `mpe-plugin-sdk` first.
