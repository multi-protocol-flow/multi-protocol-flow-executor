use serde::{Deserialize, Serialize};
use std::time::Instant;

/// HTTP 请求各阶段计时数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpTiming {
    /// DNS 解析耗时（毫秒）
    pub dns_ms: Option<u64>,
    /// TCP 连接耗时（毫秒）
    pub connect_ms: Option<u64>,
    /// TLS 握手耗时（毫秒），HTTP 时为 None
    pub tls_ms: Option<u64>,
    /// 首字节时间（TTFB，毫秒）
    pub ttfb_ms: Option<u64>,
    /// 响应体下载耗时（毫秒）
    pub download_ms: Option<u64>,
    /// 网络总耗时（毫秒）= DNS + TCP + TLS + TTFB + Download + 请求发送
    /// 注意：这个值略大于各分段之和，因为还包括请求发送等未单独计时的阶段
    pub total_ms: Option<u64>,
}

/// 计时器上下文，用于记录各阶段时间点
pub struct TimingContext {
    start: Instant,
    dns_resolved: Option<Instant>,
    tcp_connected: Option<Instant>,
    tls_handshaked: Option<Instant>,
    first_byte: Option<Instant>,
    download_complete: Option<Instant>,
}

impl Default for TimingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TimingContext {
    /// 创建新的计时上下文
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            dns_resolved: None,
            tcp_connected: None,
            tls_handshaked: None,
            first_byte: None,
            download_complete: None,
        }
    }

    /// 标记 DNS 解析完成
    pub fn mark_dns_resolved(&mut self) {
        self.dns_resolved = Some(Instant::now());
    }

    /// 标记 TCP 连接完成
    pub fn mark_tcp_connected(&mut self) {
        self.tcp_connected = Some(Instant::now());
    }

    /// 标记 TLS 握手完成
    pub fn mark_tls_handshaked(&mut self) {
        self.tls_handshaked = Some(Instant::now());
    }

    /// 标记首字节到达
    pub fn mark_first_byte(&mut self) {
        if self.first_byte.is_none() {
            self.first_byte = Some(Instant::now());
        }
    }

    /// 检查是否已标记首字节
    pub fn has_first_byte(&self) -> bool {
        self.first_byte.is_some()
    }

    /// 标记下载完成
    pub fn mark_download_complete(&mut self) {
        self.download_complete = Some(Instant::now());
    }

    /// 计算各阶段计时数据
    pub fn calculate(&self) -> HttpTiming {
        HttpTiming {
            // DNS: 从开始到解析完成
            dns_ms: self
                .dns_resolved
                .map(|t| t.duration_since(self.start).as_millis() as u64),
            // TCP: 从 DNS 完成到 TCP 连接
            connect_ms: self.tcp_connected.and_then(|tcp| {
                self.dns_resolved
                    .map(|dns| tcp.duration_since(dns).as_millis() as u64)
            }),
            // TLS: 从 TCP 连接到 TLS 握手
            tls_ms: self.tls_handshaked.and_then(|tls| {
                self.tcp_connected
                    .map(|tcp| tls.duration_since(tcp).as_millis() as u64)
            }),
            // TTFB: 从 TLS/TCP/DNS 完成到首字节
            ttfb_ms: self.first_byte.map(|fb| {
                let baseline = self
                    .tls_handshaked
                    .or(self.tcp_connected)
                    .or(self.dns_resolved)
                    .unwrap_or(self.start);
                fb.duration_since(baseline).as_millis() as u64
            }),
            // Download: 从首字节到下载完成
            download_ms: self.download_complete.and_then(|end| {
                self.first_byte
                    .map(|fb| end.duration_since(fb).as_millis() as u64)
            }),
            // Total: 从开始到下载完成的总时间
            total_ms: self
                .download_complete
                .map(|end| end.duration_since(self.start).as_millis() as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timing_context_basic() {
        let mut timing = TimingContext::new();

        // 模拟 DNS 解析
        thread::sleep(Duration::from_millis(10));
        timing.mark_dns_resolved();

        // 模拟 TCP 连接
        thread::sleep(Duration::from_millis(15));
        timing.mark_tcp_connected();

        // 模拟 TLS 握手
        thread::sleep(Duration::from_millis(20));
        timing.mark_tls_handshaked();

        // 模拟首字节到达
        thread::sleep(Duration::from_millis(30));
        timing.mark_first_byte();

        // 模拟下载完成
        thread::sleep(Duration::from_millis(25));
        timing.mark_download_complete();

        let result = timing.calculate();

        // 验证所有计时字段都有值
        assert!(result.dns_ms.is_some());
        assert!(result.connect_ms.is_some());
        assert!(result.tls_ms.is_some());
        assert!(result.ttfb_ms.is_some());
        assert!(result.download_ms.is_some());

        // 验证计时值在合理范围内（允许 ±10ms 误差）
        let dns = result.dns_ms.unwrap_or_else(|| panic!("expected Some"));
        assert!((5..=20).contains(&dns), "DNS 应在 10ms 左右，实际: {}", dns);

        let connect = result.connect_ms.unwrap_or_else(|| panic!("expected Some"));
        assert!(
            (10..=25).contains(&connect),
            "Connect 应在 15ms 左右，实际: {}",
            connect
        );

        let tls = result.tls_ms.unwrap_or_else(|| panic!("expected Some"));
        assert!(
            (15..=30).contains(&tls),
            "TLS 应在 20ms 左右，实际: {}",
            tls
        );

        let ttfb = result.ttfb_ms.unwrap_or_else(|| panic!("expected Some"));
        assert!(
            (25..=40).contains(&ttfb),
            "TTFB 应在 30ms 左右，实际: {}",
            ttfb
        );

        let download = result
            .download_ms
            .unwrap_or_else(|| panic!("expected Some"));
        assert!(
            (20..=35).contains(&download),
            "Download 应在 25ms 左右，实际: {}",
            download
        );
    }

    #[test]
    fn test_timing_context_http_no_tls() {
        let mut timing = TimingContext::new();

        // HTTP 请求（无 TLS）
        timing.mark_dns_resolved();
        timing.mark_tcp_connected();
        timing.mark_first_byte();
        timing.mark_download_complete();

        let result = timing.calculate();

        // TLS 应该为 None
        assert!(result.tls_ms.is_none());
        // 其他字段应该有值
        assert!(result.dns_ms.is_some());
        assert!(result.connect_ms.is_some());
        assert!(result.ttfb_ms.is_some());
        assert!(result.download_ms.is_some());
    }

    #[test]
    fn test_timing_context_partial_failure() {
        let mut timing = TimingContext::new();

        // DNS Success
        timing.mark_dns_resolved();

        // TCP Failed（不标记）

        let result = timing.calculate();

        // DNS 应该有值
        assert!(result.dns_ms.is_some());
        // TCP 及后续阶段应该为 None
        assert!(result.connect_ms.is_none());
        assert!(result.tls_ms.is_none());
        assert!(result.ttfb_ms.is_none());
        assert!(result.download_ms.is_none());
    }

    #[test]
    fn test_http_timing_serialization() {
        let timing = HttpTiming {
            dns_ms: Some(15),
            connect_ms: Some(32),
            tls_ms: Some(28),
            ttfb_ms: Some(120),
            download_ms: Some(45),
            total_ms: Some(240),
        };

        let json = serde_json::to_string(&timing).unwrap_or_else(|e| panic!("序列化Failed: {e}"));
        assert!(json.contains("\"dns_ms\":15"));
        assert!(json.contains("\"connect_ms\":32"));
        assert!(json.contains("\"tls_ms\":28"));
        assert!(json.contains("\"ttfb_ms\":120"));
        assert!(json.contains("\"download_ms\":45"));
        assert!(json.contains("\"total_ms\":240"));

        let deserialized: HttpTiming =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("反序列化Failed: {e}"));
        assert_eq!(timing.dns_ms, deserialized.dns_ms);
        assert_eq!(timing.connect_ms, deserialized.connect_ms);
        assert_eq!(timing.tls_ms, deserialized.tls_ms);
        assert_eq!(timing.ttfb_ms, deserialized.ttfb_ms);
        assert_eq!(timing.download_ms, deserialized.download_ms);
        assert_eq!(timing.total_ms, deserialized.total_ms);
    }
}
