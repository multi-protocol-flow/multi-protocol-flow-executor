# flow-engine (门面 crate)

**Layer 3 (门面)** | 2 文件 | 205 行 | 依赖全部 8 个 crate

## 概述

纯 re-export 门面 crate，无自身业务逻辑。`app` (src-tauri/) 仅依赖此 crate。

## 结构

```
flow-engine/src/
├── lib.rs         # re-export 全部子 crate (203 行)
└── context.rs     # re-export ExecutionContext from core
```

## 约定

- 此 crate 不应有自身业务逻辑
- 仅做 re-export，保持 API 表面稳定
- 新增子 crate 时需在此处添加 re-export

## 编译影响

修改此 crate → 仅 `app` 重编译 (~1 crate)

```bash
cd src-tauri
cargo build -p flow-engine             # 编译门面 (触发全部依赖)
```
