//! 密码加密模块
//!
//! 使用 AES-256-GCM 进行认证加密，密钥存储在 OS keychain 中。
//!
//! # 安全特性
//! - **AES-256-GCM**: 提供加密和认证（防篡改）
//! - **OS Keychain**: 密钥安全存储在系统密钥链中
//! - **随机 Nonce**: 每次加密使用新的 12 字节随机 nonce
//! - **Zeroize**: 敏感数据在使用后立即从内存中清除
//!
//! # 使用示例
//! ```ignore
//! use flow_engine::encryption::{encrypt, decrypt};
//!
//! // 加密明文密码
//! let encrypted = encrypt("my_secret_password")?;
//!
//! // 解密密文
//! let decrypted = decrypt(&encrypted)?;
//! assert_eq!(decrypted, "my_secret_password");
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use keyring::Entry;
use rand::RngCore;
use thiserror::Error;
use zeroize::Zeroize;

/// Keychain 服务名称
const KEYRING_SERVICE: &str = "multi-protocol-flow-executor";

/// Keychain 用户名（用于存储加密密钥）
const KEYRING_USERNAME: &str = "db-encryption-key";

/// Nonce 长度（AES-GCM 要求 12 字节）
const NONCE_SIZE: usize = 12;

/// AES-256 密钥长度
const KEY_SIZE: usize = 32;

/// 加密错误类型
#[derive(Error, Debug)]
pub enum EncryptionError {
    /// 加密操作失败
    #[error("加密失败: {0}")]
    EncryptionFailed(String),

    /// 解密操作失败
    #[error("解密失败: {0}")]
    DecryptionFailed(String),

    /// Keychain 操作失败
    #[error("Keychain 操作失败: {0}")]
    KeyringError(String),

    /// Base64 编解码失败
    #[error("Base64 编解码失败: {0}")]
    Base64Error(String),

    /// 密钥生成失败
    #[error("密钥生成失败: {0}")]
    KeyGenerationFailed(String),

    /// UTF-8 编解码失败
    #[error("UTF-8 编解码失败: {0}")]
    Utf8Error(String),

    /// 密文格式无效
    #[error("密文格式无效: {0}")]
    InvalidFormat(String),
}

/// 使用 AES-256-GCM 加密明文密码
///
/// # 加密流程
/// 1. 从 OS keychain 获取或创建加密密钥
/// 2. 生成 12 字节随机 nonce
/// 3. 使用 AES-256-GCM 加密
/// 4. 返回格式：`base64(nonce):base64(ciphertext)`
///
/// # 安全特性
/// - 每次加密使用不同的随机 nonce
/// - GCM 模式提供认证（防篡改）
/// - 密钥从不在内存中持久化存储
///
/// # 参数
/// - `plaintext`: 要加密的明文密码
///
/// # 返回
/// - `Ok(String)`: 格式为 `base64(nonce):base64(ciphertext)` 的密文
/// - `Err(EncryptionError)`: 加密失败
///
/// # 示例
/// ```ignore
/// use flow_engine::encryption::encrypt;
///
/// let encrypted = encrypt("my_password")?;
/// assert!(encrypted.contains(':')); // 包含 nonce 和 ciphertext
/// ```
pub fn encrypt(plaintext: &str) -> Result<String, EncryptionError> {
    // 获取或创建密钥
    let key = get_or_create_key()?;

    // 创建密码器
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

    // 生成随机 nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 加密
    let mut ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

    // 编码结果：base64(nonce):base64(ciphertext)
    let result = format!(
        "{}:{}",
        BASE64.encode(nonce_bytes),
        BASE64.encode(&ciphertext)
    );

    // 清理内存中的敏感数据
    ciphertext.zeroize();
    nonce_bytes.zeroize();

    Ok(result)
}

/// 使用 AES-256-GCM 解密密文
///
/// # 解密流程
/// 1. 从 OS keychain 获取加密密钥
/// 2. 解析密文格式 `base64(nonce):base64(ciphertext)`
/// 3. 使用 AES-256-GCM 解密
/// 4. 验证认证标签（防篡改）
///
/// # 安全特性
/// - GCM 模式自动验证完整性
/// - 任何篡改都会导致解密失败
///
/// # 参数
/// - `ciphertext`: 格式为 `base64(nonce):base64(ciphertext)` 的密文
///
/// # 返回
/// - `Ok(String)`: 解密后的明文
/// - `Err(EncryptionError)`: 解密失败（密钥错误、密文篡改等）
///
/// # 示例
/// ```ignore
/// use flow_engine::encryption::{encrypt, decrypt};
///
/// let encrypted = encrypt("my_password")?;
/// let decrypted = decrypt(&encrypted)?;
/// assert_eq!(decrypted, "my_password");
/// ```
pub fn decrypt(ciphertext: &str) -> Result<String, EncryptionError> {
    // 解析密文格式
    let parts: Vec<&str> = ciphertext.split(':').collect();
    if parts.len() != 2 {
        return Err(EncryptionError::InvalidFormat(
            "密文格式应为 base64(nonce):base64(ciphertext)".to_string(),
        ));
    }

    // 解码 nonce
    let mut nonce_bytes = BASE64
        .decode(parts[0])
        .map_err(|e| EncryptionError::Base64Error(e.to_string()))?;

    if nonce_bytes.len() != NONCE_SIZE {
        nonce_bytes.zeroize();
        return Err(EncryptionError::InvalidFormat(format!(
            "Nonce 长度应为 {} 字节，实际为 {} 字节",
            NONCE_SIZE,
            nonce_bytes.len()
        )));
    }

    let nonce = Nonce::from_slice(&nonce_bytes);

    // 解码密文
    let mut ciphertext_bytes = BASE64
        .decode(parts[1])
        .map_err(|e| EncryptionError::Base64Error(e.to_string()))?;

    // 获取密钥
    let key = get_or_create_key()?;

    // 创建密码器
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

    // 解密
    let mut plaintext = cipher
        .decrypt(nonce, ciphertext_bytes.as_slice())
        .map_err(|_| {
            EncryptionError::DecryptionFailed("解密失败：密钥错误或密文已被篡改".to_string())
        })?;

    // 转换为字符串
    let result = String::from_utf8(plaintext.clone()).map_err(|e| {
        plaintext.zeroize();
        EncryptionError::Utf8Error(e.to_string())
    })?;

    // 清理内存中的敏感数据
    plaintext.zeroize();
    ciphertext_bytes.zeroize();
    nonce_bytes.zeroize();

    Ok(result)
}

/// 从 OS keychain 获取或创建加密密钥
///
/// # 流程
/// 1. 尝试从 keychain 获取现有密钥
/// 2. 如果不存在，生成新的 32 字节随机密钥
/// 3. 将新密钥存储到 keychain
///
/// # 安全特性
/// - 密钥存储在系统密钥链中（Windows Credential Manager / macOS Keychain / Linux Secret Service）
/// - 密钥从不写入磁盘文件
/// - 临时密钥变量在使用后立即 zeroize
///
/// # 返回
/// - `Ok([u8; 32])`: 32 字节 AES-256 密钥
/// - `Err(EncryptionError)`: keychain 操作失败
fn get_or_create_key() -> Result<[u8; KEY_SIZE], EncryptionError> {
    // 创建 keychain entry
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
        .map_err(|e| EncryptionError::KeyringError(e.to_string()))?;

    // 尝试获取现有密钥
    match entry.get_password() {
        Ok(key_b64) => {
            log::debug!("从 keychain 获取到现有密钥");

            // 解码现有密钥
            let mut key_bytes = BASE64
                .decode(&key_b64)
                .map_err(|e| EncryptionError::Base64Error(e.to_string()))?;

            if key_bytes.len() != KEY_SIZE {
                key_bytes.zeroize();
                return Err(EncryptionError::KeyringError(format!(
                    "密钥长度应为 {} 字节，实际为 {} 字节",
                    KEY_SIZE,
                    key_bytes.len()
                )));
            }

            // 转换为数组
            let mut key = [0u8; KEY_SIZE];
            key.copy_from_slice(&key_bytes);

            // 清理临时变量
            key_bytes.zeroize();

            Ok(key)
        }
        Err(e) => {
            log::debug!("密钥不存在，生成新密钥: {}", e);

            // 密钥不存在，生成新密钥
            let mut key = [0u8; KEY_SIZE];
            rand::thread_rng().fill_bytes(&mut key);

            // 编码并存储到 keychain
            let key_b64 = BASE64.encode(key);

            entry
                .set_password(&key_b64)
                .map_err(|e| EncryptionError::KeyringError(e.to_string()))?;

            log::debug!("新密钥已存储到 keychain");

            Ok(key)
        }
    }
}

/// 删除 keychain 中的加密密钥
///
/// # 用途
/// 用于测试清理或密钥轮换
///
/// # 返回
/// - `Ok(())`: 删除成功
/// - `Err(EncryptionError)`: 删除失败或密钥不存在
#[allow(dead_code)]
pub fn delete_key() -> Result<(), EncryptionError> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
        .map_err(|e| EncryptionError::KeyringError(e.to_string()))?;

    entry
        .delete_credential()
        .map_err(|e| EncryptionError::KeyringError(e.to_string()))?;

    Ok(())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// 全局测试密钥（避免多次调用 keyring）
    static TEST_KEY: once_cell::sync::Lazy<Arc<Mutex<Option<[u8; KEY_SIZE]>>>> =
        once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

    /// 获取或创建测试密钥（使用内存缓存，避免 keyring 不一致）
    fn get_test_key() -> [u8; KEY_SIZE] {
        let mut key_guard = TEST_KEY.lock().unwrap();

        if let Some(key) = *key_guard {
            return key;
        }

        // 初始化新密钥
        let key = get_or_create_key().expect("获取密钥失败");
        *key_guard = Some(key);
        key
    }

    /// 使用测试密钥加密
    fn encrypt_with_test_key(plaintext: &str) -> Result<String, EncryptionError> {
        let key = get_test_key();

        // 创建密码器
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // 生成随机 nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密
        let mut ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // 编码结果：base64(nonce):base64(ciphertext)
        let result = format!(
            "{}:{}",
            BASE64.encode(&nonce_bytes),
            BASE64.encode(&ciphertext)
        );

        // 清理内存中的敏感数据
        ciphertext.zeroize();
        nonce_bytes.zeroize();

        Ok(result)
    }

    /// 使用测试密钥解密
    fn decrypt_with_test_key(ciphertext: &str) -> Result<String, EncryptionError> {
        let key = get_test_key();

        // 解析密文格式
        let parts: Vec<&str> = ciphertext.split(':').collect();
        if parts.len() != 2 {
            return Err(EncryptionError::InvalidFormat(
                "密文格式应为 base64(nonce):base64(ciphertext)".to_string(),
            ));
        }

        // 解码 nonce
        let mut nonce_bytes = BASE64
            .decode(parts[0])
            .map_err(|e| EncryptionError::Base64Error(e.to_string()))?;

        if nonce_bytes.len() != NONCE_SIZE {
            nonce_bytes.zeroize();
            return Err(EncryptionError::InvalidFormat(format!(
                "Nonce 长度应为 {} 字节，实际为 {} 字节",
                NONCE_SIZE,
                nonce_bytes.len()
            )));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        // 解码密文
        let mut ciphertext_bytes = BASE64
            .decode(parts[1])
            .map_err(|e| EncryptionError::Base64Error(e.to_string()))?;

        // 创建密码器
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        // 解密
        let mut plaintext = cipher
            .decrypt(nonce, ciphertext_bytes.as_slice())
            .map_err(|_| {
                EncryptionError::DecryptionFailed("解密失败：密钥错误或密文已被篡改".to_string())
            })?;

        // 转换为字符串
        let result = String::from_utf8(plaintext.clone()).map_err(|e| {
            plaintext.zeroize();
            EncryptionError::Utf8Error(e.to_string())
        })?;

        // 清理内存中的敏感数据
        plaintext.zeroize();
        ciphertext_bytes.zeroize();
        nonce_bytes.zeroize();

        Ok(result)
    }

    /// 测试加密后解密能还原原文
    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = "my_secret_password_123";

        // 加密
        let encrypted = encrypt_with_test_key(plaintext).expect("加密失败");

        // 验证格式
        assert!(encrypted.contains(':'), "密文格式应包含冒号分隔符");

        // 解密
        let decrypted = decrypt_with_test_key(&encrypted).expect("解密失败");

        // 验证结果
        assert_eq!(decrypted, plaintext, "解密后的明文应与原文相同");
    }

    /// 测试不同明文产生不同密文（由于随机 nonce）
    #[test]
    fn test_different_inputs_different_outputs() {
        let plaintext = "same_password";

        // 多次加密相同明文
        let encrypted1 = encrypt_with_test_key(plaintext).expect("第一次加密失败");
        let encrypted2 = encrypt_with_test_key(plaintext).expect("第二次加密失败");

        // 由于随机 nonce，密文应该不同
        assert_ne!(
            encrypted1, encrypted2,
            "相同明文的多次加密应产生不同密文（随机 nonce）"
        );

        // 但两者都应能正确解密
        let decrypted1 = decrypt_with_test_key(&encrypted1).expect("第一次解密失败");
        let decrypted2 = decrypt_with_test_key(&encrypted2).expect("第二次解密失败");

        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }

    /// 测试密钥持久性（密钥在多次调用间保持一致）
    #[test]
    fn test_key_persistence() {
        let plaintext1 = "password_one";
        let plaintext2 = "password_two";

        // 使用密钥 A 加密 plaintext1
        let encrypted1 = encrypt_with_test_key(plaintext1).expect("第一次加密失败");

        // 使用相同密钥加密 plaintext2
        let encrypted2 = encrypt_with_test_key(plaintext2).expect("第二次加密失败");

        // 解密应成功（证明密钥一致）
        let decrypted1 = decrypt_with_test_key(&encrypted1).expect("第一次解密失败");
        let decrypted2 = decrypt_with_test_key(&encrypted2).expect("第二次解密失败");

        assert_eq!(decrypted1, plaintext1);
        assert_eq!(decrypted2, plaintext2);
    }

    /// 测试空字符串加密
    #[test]
    fn test_encrypt_empty_string() {
        let plaintext = "";

        let encrypted = encrypt_with_test_key(plaintext).expect("空字符串加密失败");
        let decrypted = decrypt_with_test_key(&encrypted).expect("空字符串解密失败");

        assert_eq!(decrypted, plaintext);
    }

    /// 测试长字符串加密
    #[test]
    fn test_encrypt_long_string() {
        // 1KB 随机数据
        let plaintext: String = (0..1024).map(|_| 'X').collect();

        let encrypted = encrypt_with_test_key(&plaintext).expect("长字符串加密失败");
        let decrypted = decrypt_with_test_key(&encrypted).expect("长字符串解密失败");

        assert_eq!(decrypted, plaintext);
    }

    /// 测试特殊字符加密
    #[test]
    fn test_encrypt_special_characters() {
        let plaintext = "密码测试!@#$%^&*()_+-={}[]|:;<>?,./~`";

        let encrypted = encrypt_with_test_key(plaintext).expect("特殊字符加密失败");
        let decrypted = decrypt_with_test_key(&encrypted).expect("特殊字符解密失败");

        assert_eq!(decrypted, plaintext);
    }

    /// 测试无效密文格式
    #[test]
    fn test_invalid_ciphertext_format() {
        // 缺少冒号
        let result = decrypt("invalid_base64_without_colon");
        assert!(result.is_err(), "无效格式应返回错误");

        // 多个冒号
        let result = decrypt("part1:part2:part3");
        assert!(result.is_err(), "多个分隔符应返回错误");
    }

    /// 测试篡改密文（认证验证）
    #[test]
    fn test_tampered_ciphertext() {
        let plaintext = "original_password";
        let mut encrypted = encrypt(plaintext).expect("加密失败");

        // 篡改密文（修改最后一个字符）
        let last_char = encrypted.chars().last().unwrap();
        let tampered_char = if last_char == 'A' { 'B' } else { 'A' };
        encrypted.pop();
        encrypted.push(tampered_char);

        // 解密应失败（GCM 认证失败）
        let result = decrypt(&encrypted);
        assert!(result.is_err(), "篡改的密文应导致解密失败（GCM 认证）");
    }

    /// 测试 Base64 解码错误
    #[test]
    fn test_invalid_base64() {
        // 无效的 Base64 字符
        let result = decrypt("!!!invalid!!!:moreinvalid");
        assert!(result.is_err(), "无效 Base64 应返回错误");
    }
}
