//! 断言预设模板
//!
//! 提供常用的断言预设配置，用于快速设置常见场景的断言。

use flow_engine_core::node_config::{Assertion, AssertionOperator};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 预设信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetInfo {
    /// 预设名称 (`snake_case`)
    pub name: String,
    /// 预设描述
    pub description: String,
}

/// 断言预设管理器
pub struct AssertionPreset;

impl AssertionPreset {
    /// 获取预设的断言列表
    ///
    /// # 参数
    /// - `name`: 预设名称 (`snake_case`)
    ///
    /// # 返回
    /// - `Some(Vec<Assertion>)`: 预设存在，返回断言列表
    /// - `None`: 预设不存在
    pub fn get(name: &str) -> Option<Vec<Assertion>> {
        match name {
            "http_success" => Some(Self::http_success()),
            "http_client_error" => Some(Self::http_client_error()),
            "http_server_error" => Some(Self::http_server_error()),
            "json_response" => Some(Self::json_response()),
            "response_fast" => Some(Self::response_fast()),
            _ => None,
        }
    }

    /// 列出所有可用的预设
    pub fn list() -> Vec<PresetInfo> {
        vec![
            PresetInfo {
                name: "http_success".to_string(),
                description: "HTTP success response (2xx)".to_string(),
            },
            PresetInfo {
                name: "http_client_error".to_string(),
                description: "HTTP client error (4xx)".to_string(),
            },
            PresetInfo {
                name: "http_server_error".to_string(),
                description: "HTTP server error (5xx)".to_string(),
            },
            PresetInfo {
                name: "json_response".to_string(),
                description: "JSON response (Content-Type contains application/json)".to_string(),
            },
            PresetInfo {
                name: "response_fast".to_string(),
                description: "Fast response (response time < 500ms)".to_string(),
            },
        ]
    }

    /// `http_success` 预设: 状态码在 [200, 299] 范围内
    fn http_success() -> Vec<Assertion> {
        vec![Assertion {
            id: "preset_http_success_status".to_string(),
            description: Some("HTTP status code should be 2xx (200-299)".to_string()),
            target: "$.status".to_string(),
            operator: AssertionOperator::Between,
            expected: json!([200, 299]),
            enabled: true,
        }]
    }

    /// `http_client_error` 预设: 状态码在 [400, 499] 范围内
    fn http_client_error() -> Vec<Assertion> {
        vec![Assertion {
            id: "preset_http_client_error_status".to_string(),
            description: Some("HTTP status code should be 4xx (400-499)".to_string()),
            target: "$.status".to_string(),
            operator: AssertionOperator::Between,
            expected: json!([400, 499]),
            enabled: true,
        }]
    }

    /// `http_server_error` 预设: 状态码在 [500, 599] 范围内
    fn http_server_error() -> Vec<Assertion> {
        vec![Assertion {
            id: "preset_http_server_error_status".to_string(),
            description: Some("HTTP status code should be 5xx (500-599)".to_string()),
            target: "$.status".to_string(),
            operator: AssertionOperator::Between,
            expected: json!([500, 599]),
            enabled: true,
        }]
    }

    /// `json_response` 预设: Content-Type 包含 application/json
    fn json_response() -> Vec<Assertion> {
        vec![Assertion {
            id: "preset_json_response_content_type".to_string(),
            description: Some("Content-Type should contain application/json".to_string()),
            target: "$.headers[\"Content-Type\"]".to_string(),
            operator: AssertionOperator::Contains,
            expected: json!("application/json"),
            enabled: true,
        }]
    }

    /// `response_fast` 预设: 响应时间 < 500ms
    fn response_fast() -> Vec<Assertion> {
        vec![Assertion {
            id: "preset_response_fast_time".to_string(),
            description: Some("Response time should be less than 500ms".to_string()),
            target: "$.response_time".to_string(),
            operator: AssertionOperator::Lt,
            expected: json!(500),
            enabled: true,
        }]
    }
}

/// 根据名称获取预设的断言列表
///
/// 便捷函数，等同于 `AssertionPreset::get(name)`
pub fn get_preset(name: &str) -> Option<Vec<Assertion>> {
    AssertionPreset::get(name)
}

/// 列出所有可用的预设
///
/// 便捷函数，等同于 `AssertionPreset::list()`
pub fn list_presets() -> Vec<PresetInfo> {
    AssertionPreset::list()
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // get_preset 测试
    // ========================================================================

    #[test]
    fn test_get_preset_http_success() {
        let assertions = get_preset("http_success");
        assert!(assertions.is_some());
        let assertions = assertions.unwrap();
        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].operator, AssertionOperator::Between);
        assert_eq!(assertions[0].expected, json!([200, 299]));
        assert!(assertions[0].enabled);
    }

    #[test]
    fn test_get_preset_http_client_error() {
        let assertions = get_preset("http_client_error");
        assert!(assertions.is_some());
        let assertions = assertions.unwrap();
        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].operator, AssertionOperator::Between);
        assert_eq!(assertions[0].expected, json!([400, 499]));
        assert!(assertions[0].enabled);
    }

    #[test]
    fn test_get_preset_http_server_error() {
        let assertions = get_preset("http_server_error");
        assert!(assertions.is_some());
        let assertions = assertions.unwrap();
        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].operator, AssertionOperator::Between);
        assert_eq!(assertions[0].expected, json!([500, 599]));
        assert!(assertions[0].enabled);
    }

    #[test]
    fn test_get_preset_json_response() {
        let assertions = get_preset("json_response");
        assert!(assertions.is_some());
        let assertions = assertions.unwrap();
        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].operator, AssertionOperator::Contains);
        assert_eq!(assertions[0].expected, json!("application/json"));
        assert_eq!(assertions[0].target, "$.headers[\"Content-Type\"]");
        assert!(assertions[0].enabled);
    }

    #[test]
    fn test_get_preset_response_fast() {
        let assertions = get_preset("response_fast");
        assert!(assertions.is_some());
        let assertions = assertions.unwrap();
        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].operator, AssertionOperator::Lt);
        assert_eq!(assertions[0].expected, json!(500));
        assert_eq!(assertions[0].target, "$.response_time");
        assert!(assertions[0].enabled);
    }

    #[test]
    fn test_get_preset_invalid_name() {
        let assertions = get_preset("nonexistent_preset");
        assert!(assertions.is_none());
    }

    // ========================================================================
    // list_presets 测试
    // ========================================================================

    #[test]
    fn test_list_presets_count() {
        let presets = list_presets();
        assert_eq!(presets.len(), 5);
    }

    #[test]
    fn test_list_presets_names() {
        let presets = list_presets();
        let names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"http_success"));
        assert!(names.contains(&"http_client_error"));
        assert!(names.contains(&"http_server_error"));
        assert!(names.contains(&"json_response"));
        assert!(names.contains(&"response_fast"));
    }

    #[test]
    fn test_list_presets_descriptions() {
        let presets = list_presets();
        for preset in &presets {
            assert!(!preset.description.is_empty());
        }
    }

    // ========================================================================
    // AssertionPreset::get 测试 (struct 方法)
    // ========================================================================

    #[test]
    fn test_assertion_preset_get_http_success() {
        let assertions = AssertionPreset::get("http_success");
        assert!(assertions.is_some());
        let assertions = assertions.unwrap();
        assert_eq!(assertions[0].id, "preset_http_success_status");
        assert_eq!(assertions[0].target, "$.status");
    }

    #[test]
    fn test_assertion_preset_get_invalid() {
        let assertions = AssertionPreset::get("invalid_preset");
        assert!(assertions.is_none());
    }

    // ========================================================================
    // AssertionPreset::list 测试 (struct 方法)
    // ========================================================================

    #[test]
    fn test_assertion_preset_list() {
        let presets = AssertionPreset::list();
        assert_eq!(presets.len(), 5);

        // 验证每个预设都有名称和描述
        for preset in &presets {
            assert!(!preset.name.is_empty());
            assert!(!preset.description.is_empty());
        }
    }

    // ========================================================================
    // 预设与求值器集成测试
    // ========================================================================

    #[test]
    fn test_http_success_preset_with_evaluator() {
        use crate::assertion::evaluator::AssertionEvaluator;
        use flow_engine_core::node_config::AssertionConfig;

        let assertions = get_preset("http_success").unwrap();
        let config = AssertionConfig {
            preset: Some("http_success".to_string()),
            mode: Default::default(),
            on_failure: Default::default(),
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };

        // 测试 200 通过
        let input = json!({"status": 200});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(output.passed);

        // 测试 201 通过
        let input = json!({"status": 201});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(output.passed);

        // 测试 299 通过
        let input = json!({"status": 299});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(output.passed);

        // 测试 300 Failed
        let input = json!({"status": 300});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(!output.passed);

        // 测试 404 Failed
        let input = json!({"status": 404});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(!output.passed);
    }

    #[test]
    fn test_json_response_preset_with_evaluator() {
        use crate::assertion::evaluator::AssertionEvaluator;
        use flow_engine_core::node_config::AssertionConfig;

        let assertions = get_preset("json_response").unwrap();
        let config = AssertionConfig {
            preset: Some("json_response".to_string()),
            mode: Default::default(),
            on_failure: Default::default(),
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };

        // 测试 application/json 通过
        let input = json!({"headers": {"Content-Type": "application/json"}});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(output.passed);

        // 测试 application/json; charset=utf-8 通过
        let input = json!({"headers": {"Content-Type": "application/json; charset=utf-8"}});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(output.passed);

        // 测试 text/html Failed
        let input = json!({"headers": {"Content-Type": "text/html"}});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(!output.passed);
    }

    #[test]
    fn test_response_fast_preset_with_evaluator() {
        use crate::assertion::evaluator::AssertionEvaluator;
        use flow_engine_core::node_config::AssertionConfig;

        let assertions = get_preset("response_fast").unwrap();
        let config = AssertionConfig {
            preset: Some("response_fast".to_string()),
            mode: Default::default(),
            on_failure: Default::default(),
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };

        // 测试 100ms 通过
        let input = json!({"response_time": 100});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(output.passed);

        // 测试 499ms 通过
        let input = json!({"response_time": 499});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(output.passed);

        // 测试 500ms Failed (边界值，< 500)
        let input = json!({"response_time": 500});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(!output.passed);

        // 测试 1000ms Failed
        let input = json!({"response_time": 1000});
        let output = AssertionEvaluator::evaluate_all(&config, &input);
        assert!(!output.passed);
    }
}
