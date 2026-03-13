// 压力测试配置验证

use super::types::StressTestConfig;

impl StressTestConfig {
    /// 验证压测配置
    ///
    /// # 验证规则
    /// - `concurrent_users >= 1`（无上限）
    /// - `ramp_up_seconds <= duration_seconds`
    /// - `duration_seconds >= 0`
    ///
    /// # 返回
    /// - `Ok(())` - 验证通过
    /// - `Err(String)` - 验证失败，返回中文错误信息
    pub fn validate(&self) -> Result<(), String> {
        // 验证并发用户数 >= 1
        if self.concurrent_users < 1 {
            return Err(format!(
                "并发用户数必须大于等于 1，当前值: {}",
                self.concurrent_users
            ));
        }

        // 验证持续时间 >= 0
        // 注: u32 类型天然 >= 0，但保留验证以支持未来可能的 i32 类型

        // 验证预热时长不超过总时长
        if self.ramp_up_seconds > self.duration_seconds {
            return Err(format!(
                "预热时长({}秒)不能超过总时长({}秒)",
                self.ramp_up_seconds, self.duration_seconds
            ));
        }

        Ok(())
    }

    /// 创建默认配置
    pub fn new(flow_uuid: impl Into<String>) -> Self {
        Self {
            concurrent_users: 10,
            ramp_up_seconds: 10,
            duration_seconds: 60,
            flow_uuid: flow_uuid.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 10,
            duration_seconds: 60,
            flow_uuid: "test-flow".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_default_config() {
        let config = StressTestConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_concurrent_users() {
        let config = StressTestConfig {
            concurrent_users: 0,
            ramp_up_seconds: 10,
            duration_seconds: 60,
            flow_uuid: "test-flow".to_string(),
        };
        let result = config.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("并发用户数必须大于等于 1"));
        assert!(error.contains("当前值: 0"));
    }

    #[test]
    fn test_validate_ramp_up_exceeds_duration() {
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 120,
            duration_seconds: 60,
            flow_uuid: "test-flow".to_string(),
        };
        let result = config.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("预热时长(120秒)不能超过总时长(60秒)"));
    }

    #[test]
    fn test_validate_ramp_up_equals_duration() {
        // 边界情况：预热时长等于总时长，应该通过
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 60,
            duration_seconds: 60,
            flow_uuid: "test-flow".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_ramp_up() {
        // 预热时长为 0，应该通过
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 0,
            duration_seconds: 60,
            flow_uuid: "test-flow".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_duration() {
        // 持续时间为 0，预热时长也必须为 0
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 0,
            duration_seconds: 0,
            flow_uuid: "test-flow".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_duration_with_ramp_up() {
        // 持续时间为 0，但预热时长 > 0，应该失败
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 10,
            duration_seconds: 0,
            flow_uuid: "test-flow".to_string(),
        };
        let result = config.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("预热时长(10秒)不能超过总时长(0秒)"));
    }

    #[test]
    fn test_validate_high_concurrent_users() {
        // 无上限，高并发数应该通过
        let config = StressTestConfig {
            concurrent_users: 10000,
            ramp_up_seconds: 60,
            duration_seconds: 300,
            flow_uuid: "test-flow".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_concurrent_users_one() {
        // 边界情况：最小并发数 1
        let config = StressTestConfig {
            concurrent_users: 1,
            ramp_up_seconds: 0,
            duration_seconds: 60,
            flow_uuid: "test-flow".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_new_config() {
        let config = StressTestConfig::new("flow-123");
        assert_eq!(config.concurrent_users, 10);
        assert_eq!(config.ramp_up_seconds, 10);
        assert_eq!(config.duration_seconds, 60);
        assert_eq!(config.flow_uuid, "flow-123");
    }
}
