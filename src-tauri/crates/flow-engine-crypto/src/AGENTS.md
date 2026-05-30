# flow-engine-crypto

**Layer 0 (叶子)** | 2 文件 | ~461 行 | 无 workspace 依赖

## 概述

AES-256-GCM 加密 + OS Keychain 集成。为数据库密码提供加密存储，使用 Windows Credential Manager / macOS Keychain / Linux Secret Service。

## 结构

```
flow-engine-crypto/src/
├── lib.rs              # 模块导出 (pub mod encryption)
└── encryption.rs       # 加密/解密 + Keychain 集成
```

## 快速定位

| 任务 | 文件/函数 |
|------|----------|
| 加密密码 | `encryption.rs` → `encrypt_data()` |
| 解密密码 | `encryption.rs` → `decrypt_data()` |
| Keychain 存储 | `encryption.rs` → Keychain 相关实现 |
| 增删改密码 | 通过 executor 层的 `storage.rs` 间接调用 |

## 约定

- 此 crate 不得依赖任何其他 workspace crate（保持叶子状态）
- 修改此 crate 会触发 `executor → flow-engine → app` 约 3 个 crate 重编译
- Keychain 操作可能因平台不同而行为差异（CI 环境无桌面 Keychain）
