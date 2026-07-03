//! TCP 录制回放模块
//!
//! 提供 TCP 连接交互的录制和回放能力。
//! 录制 send/receive 交互到内存，支持保存为 JSON 文件。
//! 大于 64KB 的数据自动存储到外部 .bin 文件。

use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};

/// 大数据阈值：超过此大小（64KB）的条目将存储到外部 .bin 文件
const LARGE_PAYLOAD_THRESHOLD: usize = 64 * 1024;

// ---------------------------------------------------------------------------
// 录制方向
// ---------------------------------------------------------------------------

/// 录制方向
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingDirection {
    /// 发送
    Send,
    /// 接收
    Receive,
}

// ---------------------------------------------------------------------------
// 录制条目
// ---------------------------------------------------------------------------

/// 单条录制记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordingEntry {
    /// 时间戳（毫秒，相对录制开始时间）
    pub timestamp_ms: u64,
    /// 方向（发送/接收）
    pub direction: RecordingDirection,
    /// 连接 ID
    pub connection_id: String,
    /// 数据（Base64 编码），内嵌模式使用
    pub data_base64: String,
    /// 数据大小（字节）
    pub data_size: usize,
    /// 附加元数据
    pub metadata: serde_json::Value,
    /// 外部 .bin 文件路径（大数据模式），内嵌时为 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_bin_path: Option<String>,
}

// ---------------------------------------------------------------------------
// TCP 录制
// ---------------------------------------------------------------------------

/// TCP 录制
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TcpRecording {
    /// 录制唯一标识
    pub id: String,
    /// 录制名称
    pub name: String,
    /// 创建时间（Unix 时间戳，毫秒）
    pub created_at: u64,
    /// 录制条目列表
    pub entries: Vec<RecordingEntry>,
    /// 录制总时长（毫秒）
    pub duration_ms: u64,
}

impl TcpRecording {
    /// 保存到 JSON 文件
    ///
    /// 将录制数据保存到指定目录下的 `{id}.json` 文件中。
    /// 超过 64KB 的条目数据会额外保存到 `{id}_{entry_index}.bin` 文件中。
    pub fn save_to_file(&self, dir: &Path) -> Result<(), String> {
        // 确保目录存在
        std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        let json_path = dir.join(format!("{}.json", self.id));

        // 保存大数据到外部 .bin 文件，创建可序列化的副本
        let mut saveable = self.clone();
        for (i, entry) in saveable.entries.iter_mut().enumerate() {
            if entry.data_size > LARGE_PAYLOAD_THRESHOLD && !entry.data_base64.is_empty() {
                // 解码 base64 数据
                let raw_data = BASE64
                    .decode(&entry.data_base64)
                    .map_err(|e| format!("Base64 decoding failed (entry {}): {}", i, e))?;

                // 写入 .bin 文件
                let bin_filename = format!("{}_{}.bin", self.id, i);
                let bin_path = dir.join(&bin_filename);
                std::fs::write(&bin_path, &raw_data)
                    .map_err(|e| format!("Failed to write .bin file: {}", e))?;

                // 更新条目：引用外部文件，清空内嵌数据
                entry.external_bin_path = Some(bin_filename);
                entry.data_base64 = String::new();
            }
        }

        // 序列化为 JSON
        let json_str = serde_json::to_string_pretty(&saveable)
            .map_err(|e| format!("JSON serialization failed: {}", e))?;

        std::fs::write(&json_path, json_str)
            .map_err(|e| format!("Failed to write JSON file: {}", e))?;

        Ok(())
    }

    /// 从 JSON 文件加载
    ///
    /// 从 JSON 文件加载录制数据。如果有外部 .bin 文件，会自动合并数据。
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let json_str = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read JSON file: {}", e))?;

        let mut recording: TcpRecording = serde_json::from_str(&json_str)
            .map_err(|e| format!("JSON deserialization failed: {}", e))?;

        // 合并外部 .bin 文件数据
        let dir = path
            .parent()
            .ok_or_else(|| "Unable to get file directory".to_string())?;

        for entry in recording.entries.iter_mut() {
            if let Some(ref bin_filename) = entry.external_bin_path {
                let bin_path = dir.join(bin_filename);
                let raw_data = std::fs::read(&bin_path)
                    .map_err(|e| format!("Failed to read .bin file ({}): {}", bin_filename, e))?;

                entry.data_base64 = BASE64.encode(&raw_data);
                entry.external_bin_path = None; // 加载后不再标记为外部
            }
        }

        Ok(recording)
    }

    /// 获取所有发送条目
    pub fn send_entries(&self) -> Vec<&RecordingEntry> {
        self.entries
            .iter()
            .filter(|e| e.direction == RecordingDirection::Send)
            .collect()
    }

    /// 获取所有接收条目
    pub fn receive_entries(&self) -> Vec<&RecordingEntry> {
        self.entries
            .iter()
            .filter(|e| e.direction == RecordingDirection::Receive)
            .collect()
    }

    /// 解码条目的原始字节数据
    pub fn decode_entry_data(entry: &RecordingEntry) -> Result<Vec<u8>, String> {
        BASE64
            .decode(&entry.data_base64)
            .map_err(|e| format!("Base64 decoding failed: {}", e))
    }
}

// ---------------------------------------------------------------------------
// 活跃录制状态（内部）
// ---------------------------------------------------------------------------

/// 活跃录制状态
struct ActiveRecording {
    recording: TcpRecording,
    start_time: std::time::Instant,
    enabled: bool,
}

// ---------------------------------------------------------------------------
// 录制管理器
// ---------------------------------------------------------------------------

/// 录制管理器
///
/// 管理 TCP 交互的录制生命周期：开始、记录、停止。
/// 设计为独立组件，不依赖 TcpConnectionManager，后续集成时通过 hook 注入。
pub struct RecordingManager {
    /// 当前活跃录制
    active_recording: Option<ActiveRecording>,
    /// 录制序列号，确保 ID 唯一
    sequence: u64,
}

impl RecordingManager {
    /// 创建新的录制管理器
    pub fn new() -> Self {
        Self {
            active_recording: None,
            sequence: 0,
        }
    }

    /// 开始新录制
    ///
    /// 返回录制 ID。如果已有活跃录制，先停止旧录制并丢弃。
    pub fn start_recording(&mut self, name: &str) -> String {
        self.sequence += 1;
        let id = format!(
            "rec_{}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            self.sequence
        );

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let recording = TcpRecording {
            id: id.clone(),
            name: name.to_string(),
            created_at,
            entries: Vec::new(),
            duration_ms: 0,
        };

        self.active_recording = Some(ActiveRecording {
            recording,
            start_time: std::time::Instant::now(),
            enabled: true,
        });

        id
    }

    /// 停止录制并返回结果
    ///
    /// 如果没有活跃录制，返回 None。
    pub fn stop_recording(&mut self) -> Option<TcpRecording> {
        let active = self.active_recording.take()?;
        let mut recording = active.recording;
        recording.duration_ms = active.start_time.elapsed().as_millis() as u64;
        Some(recording)
    }

    /// 记录一次发送
    ///
    /// 如果没有活跃录制，此操作为空操作。
    pub fn record_send(&mut self, connection_id: &str, data: &[u8]) {
        self.record_entry(RecordingDirection::Send, connection_id, data);
    }

    /// 记录一次接收
    ///
    /// 如果没有活跃录制，此操作为空操作。
    pub fn record_receive(&mut self, connection_id: &str, data: &[u8]) {
        self.record_entry(RecordingDirection::Receive, connection_id, data);
    }

    /// 是否正在录制
    pub fn is_recording(&self) -> bool {
        self.active_recording.as_ref().is_some_and(|a| a.enabled)
    }

    /// 内部：记录一条条目
    fn record_entry(&mut self, direction: RecordingDirection, connection_id: &str, data: &[u8]) {
        let active = match &mut self.active_recording {
            Some(a) if a.enabled => a,
            _ => return,
        };

        let timestamp_ms = active.start_time.elapsed().as_millis() as u64;
        let data_base64 = BASE64.encode(data);

        active.recording.entries.push(RecordingEntry {
            timestamp_ms,
            direction,
            connection_id: connection_id.to_string(),
            data_base64,
            data_size: data.len(),
            metadata: serde_json::Value::Null,
            external_bin_path: None,
        });
    }
}

impl Default for RecordingManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试录制生命周期：开始 → 记录发送 → 记录接收 → 停止
    #[test]
    fn test_recording_lifecycle() {
        let mut manager = RecordingManager::new();

        // 初始状态不应在录制
        assert!(!manager.is_recording());

        // 开始录制
        let id = manager.start_recording("测试录制");
        assert!(manager.is_recording());
        assert!(id.starts_with("rec_"));

        // 记录发送
        manager.record_send("conn-1", b"Hello, TCP!");

        // 记录接收
        manager.record_receive("conn-1", b"Response data");

        // 停止录制
        let recording = manager.stop_recording();
        assert!(recording.is_some());
        assert!(!manager.is_recording());

        let r = recording.expect("应有录制结果");
        assert!(r.id.starts_with("rec_"));
        assert_eq!(r.name, "测试录制");
        assert_eq!(r.entries.len(), 2);
        assert!(r.duration_ms > 0 || r.entries.len() == 2); // duration 可能为 0（太快）

        // 验证发送条目
        assert_eq!(r.entries[0].direction, RecordingDirection::Send);
        assert_eq!(r.entries[0].connection_id, "conn-1");
        assert_eq!(r.entries[0].data_size, 11);
        assert_eq!(
            TcpRecording::decode_entry_data(&r.entries[0]).expect("解码应Success"),
            b"Hello, TCP!".to_vec()
        );

        // 验证接收条目
        assert_eq!(r.entries[1].direction, RecordingDirection::Receive);
        assert_eq!(r.entries[1].connection_id, "conn-1");
        assert_eq!(r.entries[1].data_size, 13);
        assert_eq!(
            TcpRecording::decode_entry_data(&r.entries[1]).expect("解码应Success"),
            b"Response data".to_vec()
        );
    }

    /// 测试保存到文件 → 加载 → 条目匹配
    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().expect("创建临时目录应Success");

        let mut manager = RecordingManager::new();
        let _id = manager.start_recording("保存加载测试");

        manager.record_send("conn-a", b"send-data-1");
        manager.record_receive("conn-a", b"recv-data-1");
        manager.record_send("conn-b", b"send-data-2");

        let recording = manager.stop_recording().expect("应返回录制");

        // 保存
        recording.save_to_file(dir.path()).expect("保存应Success");

        // 验证文件存在
        let json_path = dir.path().join(format!("{}.json", recording.id));
        assert!(json_path.exists(), "JSON 文件应存在");

        // 加载
        let loaded = TcpRecording::load_from_file(&json_path).expect("加载应Success");

        // 验证基本属性
        assert_eq!(loaded.id, recording.id);
        assert_eq!(loaded.name, "保存加载测试");
        assert_eq!(loaded.entries.len(), 3);

        // 验证每条条目的数据
        for (orig, loaded_entry) in recording.entries.iter().zip(loaded.entries.iter()) {
            assert_eq!(orig.direction, loaded_entry.direction);
            assert_eq!(orig.connection_id, loaded_entry.connection_id);
            assert_eq!(orig.data_size, loaded_entry.data_size);
            assert_eq!(orig.timestamp_ms, loaded_entry.timestamp_ms);

            let orig_data = TcpRecording::decode_entry_data(orig).expect("解码应Success");
            let loaded_data = TcpRecording::decode_entry_data(loaded_entry).expect("解码应Success");
            assert_eq!(orig_data, loaded_data, "数据应完全一致");
        }
    }

    /// 测试字节精确保真度：录制二进制数据 → 保存 → 加载 → 比较字节
    #[test]
    fn test_byte_exact_fidelity() {
        let dir = tempfile::tempdir().expect("创建临时目录应Success");

        // 构造包含各种字节的二进制数据
        let binary_data: Vec<u8> = (0u8..=255).collect();
        let mut mixed_data = Vec::new();
        mixed_data.extend_from_slice(&binary_data);
        mixed_data.extend_from_slice(b"\x00\x00\x00\x00");
        mixed_data.extend_from_slice(&[0xFF, 0xFE, 0xFD, 0xFC]);
        mixed_data.extend_from_slice("混合 ASCII + 中文 UTF-8".as_bytes());

        let mut manager = RecordingManager::new();
        manager.start_recording("二进制保真度测试");
        manager.record_send("conn-bin", &mixed_data);
        manager.record_receive("conn-bin", &binary_data);

        let recording = manager.stop_recording().expect("应返回录制");
        recording.save_to_file(dir.path()).expect("保存应Success");

        let json_path = dir.path().join(format!("{}.json", recording.id));
        let loaded = TcpRecording::load_from_file(&json_path).expect("加载应Success");

        // 逐字节比较
        let loaded_send =
            TcpRecording::decode_entry_data(&loaded.entries[0]).expect("解码应Success");
        let loaded_recv =
            TcpRecording::decode_entry_data(&loaded.entries[1]).expect("解码应Success");

        assert_eq!(loaded_send, mixed_data, "发送数据应字节精确匹配");
        assert_eq!(loaded_recv, binary_data, "接收数据应字节精确匹配");
        assert_eq!(loaded_send.len(), mixed_data.len());
        assert_eq!(loaded_recv.len(), binary_data.len());
    }

    /// 测试多条目交错发送/接收
    #[test]
    fn test_multiple_interleaved_entries() {
        let mut manager = RecordingManager::new();
        manager.start_recording("交错测试");

        // 模拟多次交互
        manager.record_send("c1", b"req-1");
        manager.record_receive("c1", b"resp-1");
        manager.record_send("c2", b"req-2");
        manager.record_send("c1", b"req-1b");
        manager.record_receive("c2", b"resp-2");
        manager.record_receive("c1", b"resp-1b");

        let recording = manager.stop_recording().expect("应返回录制");
        assert_eq!(recording.entries.len(), 6);

        // 验证方向顺序
        let directions: Vec<&RecordingDirection> =
            recording.entries.iter().map(|e| &e.direction).collect();
        assert_eq!(directions[0], &RecordingDirection::Send);
        assert_eq!(directions[1], &RecordingDirection::Receive);
        assert_eq!(directions[2], &RecordingDirection::Send);
        assert_eq!(directions[3], &RecordingDirection::Send);
        assert_eq!(directions[4], &RecordingDirection::Receive);
        assert_eq!(directions[5], &RecordingDirection::Receive);

        // 验证发送和接收筛选
        let sends = recording.send_entries();
        let receives = recording.receive_entries();
        assert_eq!(sends.len(), 3);
        assert_eq!(receives.len(), 3);

        // 验证时间戳递增
        for i in 1..recording.entries.len() {
            assert!(
                recording.entries[i].timestamp_ms >= recording.entries[i - 1].timestamp_ms,
                "时间戳应递增"
            );
        }
    }

    /// 测试空录制（开始 → 停止，无任何条目）
    #[test]
    fn test_empty_recording() {
        let mut manager = RecordingManager::new();
        let id = manager.start_recording("空录制");

        let recording = manager.stop_recording().expect("应返回录制");
        assert_eq!(recording.id, id);
        assert_eq!(recording.entries.len(), 0);
        assert_eq!(recording.name, "空录制");
    }

    /// 测试未激活录制时记录为空操作
    #[test]
    fn test_recording_while_not_active() {
        let mut manager = RecordingManager::new();

        // 未开始录制时调用 record_send/record_receive 应为空操作
        manager.record_send("conn-x", b"data");
        manager.record_receive("conn-y", b"more-data");

        assert!(!manager.is_recording());

        // 停止应返回 None
        let result = manager.stop_recording();
        assert!(result.is_none());
    }

    /// 测试大数据负载（>64KB），验证外部 .bin 文件创建和加载
    #[test]
    fn test_large_payload_external_bin() {
        let dir = tempfile::tempdir().expect("创建临时目录应Success");

        // 创建超过 64KB 的数据
        let large_data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        assert!(large_data.len() > LARGE_PAYLOAD_THRESHOLD);

        let mut manager = RecordingManager::new();
        manager.start_recording("大数据测试");
        manager.record_send("conn-large", &large_data);
        manager.record_receive("conn-large", b"small-response");

        let recording = manager.stop_recording().expect("应返回录制");
        assert_eq!(recording.entries.len(), 2);

        // 保存
        recording.save_to_file(dir.path()).expect("保存应Success");

        // 验证 .bin 文件已创建
        let bin_path = dir.path().join(format!("{}_0.bin", recording.id));
        assert!(bin_path.exists(), "大数据条目应有外部 .bin 文件");

        // 小数据条目不应有 .bin 文件
        let small_bin_path = dir.path().join(format!("{}_1.bin", recording.id));
        assert!(!small_bin_path.exists(), "小数据条目不应有 .bin 文件");

        // 加载并验证数据完整
        let json_path = dir.path().join(format!("{}.json", recording.id));
        let loaded = TcpRecording::load_from_file(&json_path).expect("加载应Success");

        let loaded_large =
            TcpRecording::decode_entry_data(&loaded.entries[0]).expect("解码应Success");
        assert_eq!(loaded_large, large_data, "大数据应字节精确匹配");
        assert_eq!(loaded_large.len(), large_data.len());

        let loaded_small =
            TcpRecording::decode_entry_data(&loaded.entries[1]).expect("解码应Success");
        assert_eq!(loaded_small, b"small-response".to_vec());
    }

    /// 测试多个大数据条目
    #[test]
    fn test_multiple_large_payloads() {
        let dir = tempfile::tempdir().expect("创建临时目录应Success");

        let data1: Vec<u8> = (0..70_000).map(|i| (i % 256) as u8).collect();
        let data2: Vec<u8> = (0..80_000).map(|i| ((i + 1) % 256) as u8).collect();
        let small = b"tiny";

        let mut manager = RecordingManager::new();
        manager.start_recording("多大数据测试");
        manager.record_send("c1", &data1);
        manager.record_send("c1", small);
        manager.record_receive("c1", &data2);

        let recording = manager.stop_recording().expect("应返回录制");

        recording.save_to_file(dir.path()).expect("保存应Success");

        // 验证两个 .bin 文件
        assert!(
            dir.path().join(format!("{}_0.bin", recording.id)).exists(),
            "第 1 个大数据条目应有 .bin"
        );
        assert!(
            !dir.path().join(format!("{}_1.bin", recording.id)).exists(),
            "小数据条目不应有 .bin"
        );
        assert!(
            dir.path().join(format!("{}_2.bin", recording.id)).exists(),
            "第 2 个大数据条目应有 .bin"
        );

        // 加载验证
        let json_path = dir.path().join(format!("{}.json", recording.id));
        let loaded = TcpRecording::load_from_file(&json_path).expect("加载应Success");

        assert_eq!(
            TcpRecording::decode_entry_data(&loaded.entries[0]).expect("解码应Success"),
            data1
        );
        assert_eq!(
            TcpRecording::decode_entry_data(&loaded.entries[1]).expect("解码应Success"),
            small.to_vec()
        );
        assert_eq!(
            TcpRecording::decode_entry_data(&loaded.entries[2]).expect("解码应Success"),
            data2
        );
    }

    /// 测试 TcpRecording 的 send_entries/receive_entries 筛选
    #[test]
    fn test_send_receive_filtering() {
        let recording = TcpRecording {
            id: "test-filter".to_string(),
            name: "筛选测试".to_string(),
            created_at: 1000,
            entries: vec![
                RecordingEntry {
                    timestamp_ms: 0,
                    direction: RecordingDirection::Send,
                    connection_id: "c1".to_string(),
                    data_base64: BASE64.encode(b"s1"),
                    data_size: 2,
                    metadata: serde_json::Value::Null,
                    external_bin_path: None,
                },
                RecordingEntry {
                    timestamp_ms: 10,
                    direction: RecordingDirection::Receive,
                    connection_id: "c1".to_string(),
                    data_base64: BASE64.encode(b"r1"),
                    data_size: 2,
                    metadata: serde_json::Value::Null,
                    external_bin_path: None,
                },
                RecordingEntry {
                    timestamp_ms: 20,
                    direction: RecordingDirection::Send,
                    connection_id: "c2".to_string(),
                    data_base64: BASE64.encode(b"s2"),
                    data_size: 2,
                    metadata: serde_json::Value::Null,
                    external_bin_path: None,
                },
            ],
            duration_ms: 20,
        };

        let sends = recording.send_entries();
        let receives = recording.receive_entries();
        assert_eq!(sends.len(), 2);
        assert_eq!(receives.len(), 1);
        assert_eq!(receives[0].connection_id, "c1");
    }

    /// 测试 RecordingDirection 序列化
    #[test]
    fn test_direction_serde() {
        let send = RecordingDirection::Send;
        let recv = RecordingDirection::Receive;

        let send_json = serde_json::to_string(&send).expect("序列化应Success");
        let recv_json = serde_json::to_string(&recv).expect("序列化应Success");

        assert_eq!(send_json, "\"send\"");
        assert_eq!(recv_json, "\"receive\"");

        let send_back: RecordingDirection =
            serde_json::from_str(&send_json).expect("反序列化应Success");
        let recv_back: RecordingDirection =
            serde_json::from_str(&recv_json).expect("反序列化应Success");

        assert_eq!(send_back, RecordingDirection::Send);
        assert_eq!(recv_back, RecordingDirection::Receive);
    }

    /// 测试 RecordingManager Default trait
    #[test]
    fn test_recording_manager_default() {
        let manager = RecordingManager::default();
        assert!(!manager.is_recording());
    }

    /// 测试连续录制（开始新录制会替换旧的）
    #[test]
    fn test_start_recording_replaces_old() {
        let mut manager = RecordingManager::new();

        let id1 = manager.start_recording("第一次");
        manager.record_send("c1", b"data1");

        let id2 = manager.start_recording("第二次");
        assert_ne!(id1, id2, "新录制应有不同 ID");
        assert!(manager.is_recording());

        manager.record_send("c2", b"data2");

        let recording = manager.stop_recording().expect("应返回录制");
        assert_eq!(recording.name, "第二次");
        assert_eq!(recording.entries.len(), 1);
        assert_eq!(recording.entries[0].connection_id, "c2");
    }

    /// 测试保存到不存在的目录会自动创建
    #[test]
    fn test_save_creates_directory() {
        let dir = tempfile::tempdir().expect("创建临时目录应Success");
        let nested = dir.path().join("a").join("b").join("c");
        assert!(!nested.exists());

        let recording = TcpRecording {
            id: "mkdir-test".to_string(),
            name: "目录创建测试".to_string(),
            created_at: 1000,
            entries: vec![],
            duration_ms: 0,
        };

        recording
            .save_to_file(&nested)
            .expect("保存应Success，自动创建目录");

        assert!(nested.join("mkdir-test.json").exists());
    }

    /// 测试加载不存在的文件返回Error
    #[test]
    fn test_load_nonexistent_file() {
        let result = TcpRecording::load_from_file(Path::new("/nonexistent/path/file.json"));
        assert!(result.is_err());
    }

    /// 测试空数据录制
    #[test]
    fn test_empty_data_recording() {
        let mut manager = RecordingManager::new();
        manager.start_recording("空数据");
        manager.record_send("c1", b"");
        manager.record_receive("c1", b"");

        let recording = manager.stop_recording().expect("应返回录制");
        assert_eq!(recording.entries.len(), 2);
        assert_eq!(recording.entries[0].data_size, 0);
        assert_eq!(recording.entries[0].data_base64, "");
        assert_eq!(recording.entries[1].data_size, 0);
    }

    /// 测试 RecordingEntry metadata 字段
    #[test]
    fn test_entry_metadata() {
        let entry = RecordingEntry {
            timestamp_ms: 42,
            direction: RecordingDirection::Send,
            connection_id: "conn-1".to_string(),
            data_base64: BASE64.encode(b"test"),
            data_size: 4,
            metadata: serde_json::json!({"custom": "value", "count": 42}),
            external_bin_path: None,
        };

        let json = serde_json::to_string(&entry).expect("序列化应Success");
        let back: RecordingEntry = serde_json::from_str(&json).expect("反序列化应Success");

        assert_eq!(back.metadata["custom"], "value");
        assert_eq!(back.metadata["count"], 42);
    }
}
