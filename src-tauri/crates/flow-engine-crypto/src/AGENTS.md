# flow-engine-crypto 知识库

## OVERVIEW

Layer 0 叶子 crate。AES-256-GCM 认证加密 + OS Keychain 集成（Windows Credential Manager / macOS Keychain / Linux Secret Service）。加密格式为 `base64(nonce):base64(ciphertext)`，每次加密使用 12 字节随机 nonce。密钥通过 `keyring` crate 存储在系统密钥链，首次调用自动生成 32 字节密钥。使用 `zeroize` 清除内存中的敏感数据。

## STRUCTURE

```
src/
├── lib.rs           # pub mod encryption
└── encryption.rs    # encrypt() + decrypt() + get_or_create_key() + delete_key()
```

`lib.rs` 只导出 `encryption` 模块。`encryption.rs` 包含所有加密逻辑、`EncryptionError` 错误类型、单元测试。

## WHERE TO LOOK

| 关注点 | 位置 |
|--------|------|
| 加密入口 | `encrypt()` / `decrypt()` in encryption.rs |
| Keychain 交互 | `get_or_create_key()` in encryption.rs |
| 密钥删除 | `delete_key()` in encryption.rs |
| 错误类型 | `EncryptionError` enum in encryption.rs |
| 测试 | `#[cfg(test)] mod tests` in encryption.rs |

## CONVENTIONS

- **零 workspace 依赖**: 不依赖同 workspace 内的任何其他 crate，是最底层的叶子节点。
- **Keychain 平台差异**: Windows 用 Credential Manager，macOS 用 Keychain，Linux 用 Secret Service (libsecret)。CI 环境（无桌面 Keychain）下测试使用内存缓存密钥，不依赖 OS keychain。
- **敏感数据清除**: 所有临时密钥、nonce、明文、密文在使用后必须调用 `.zeroize()`。
- **单元测试不依赖实际 Keychain**: 通过 `once_cell::Lazy<Arc<Mutex>>` 缓存密钥，避免 CI 环境 keyring 不一致问题。

## COMPILE IMPACT

修改此 crate 触发 ~3 个 crates 重建：`flow-engine-crypto` -> `flow-engine` -> `app`。因无下层依赖，编译速度快。
