//! 通用工具函数

use serde_json_path::JsonPath;

/// 获取当前 Unix 时间戳（毫秒）
///
/// 使用 `unwrap_or_default()` 而非 `unwrap()`/`expect()`，
/// 避免系统时间异常（如 NTP 回跳）时导致 panic。
/// 系统时间异常时返回 `0`。
pub fn unix_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 按 `JSONPath` 从 JSON 中提取值
///
/// 支持的路径格式：
/// - `$.status` - 根字段
/// - `$.body.data.token` - 嵌套对象
/// - `$.body.items[0]` - 数组索引
/// - `$.body.items[-1]` - 负索引（从末尾）
pub fn extract_value_by_path(value: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    if path.is_empty() {
        return None;
    }

    // 解析 JSONPath
    let json_path = match JsonPath::parse(path) {
        Ok(p) => p,
        Err(_) => {
            log::warn!("无效的 JSONPath 语法: {}", path);
            return None;
        }
    };

    // 执行查询
    let result = json_path.query(value);

    // 获取单个结果
    match result.exactly_one() {
        Ok(v) => Some(v.clone()),
        Err(_) => None,
    }
}

/// 应用输出映射，从响应数据中提取变量写入执行上下文
///
/// 遍历 mappings，用 `JSONPath` 从 response 中提取值，
/// Success时通过 `context.variables.set_variable` 写入并记录 info 日志，
/// 未找到值时记录 warn 日志。
pub fn apply_output_mappings(
    response: &serde_json::Value,
    mappings: &[crate::variable::OutputMapping],
    context: &mut crate::context::ExecutionContext,
    node_uuid: &str,
) {
    for mapping in mappings {
        if let Some(value) = extract_value_by_path(response, &mapping.source) {
            log::info!(
                "[流程 {}] [输出映射] 节点: {}, 路径: {} -> 变量: {}",
                context.flow_uuid,
                node_uuid,
                mapping.source,
                mapping.target
            );
            context
                .variables
                .set_variable(mapping.target.clone(), value, node_uuid.to_string());
        } else {
            log::warn!(
                "[流程 {}] [输出映射] 节点: {}, 路径 '{}' 未找到值",
                context.flow_uuid,
                node_uuid,
                mapping.source
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_value_by_jsonpath_basic() {
        let data = json!({
            "status": 200,
            "body": {
                "data": {
                    "token": "abc123",
                    "user": {
                        "name": "test"
                    }
                }
            }
        });

        // 基础字段
        assert_eq!(extract_value_by_path(&data, "$.status"), Some(json!(200)));
        assert_eq!(
            extract_value_by_path(&data, "$.body.data.token"),
            Some(json!("abc123"))
        );
        assert_eq!(
            extract_value_by_path(&data, "$.body.data.user.name"),
            Some(json!("test"))
        );
        assert_eq!(extract_value_by_path(&data, "$.not.exist"), None);
    }

    #[test]
    fn test_extract_value_by_jsonpath_array_index() {
        let data = json!({
            "body": {
                "items": [
                    {"id": 1, "name": "first"},
                    {"id": 2, "name": "second"}
                ],
                "users": [1, 2, 3]
            }
        });

        assert_eq!(
            extract_value_by_path(&data, "$.body.items[0]"),
            Some(json!({"id": 1, "name": "first"}))
        );
        assert_eq!(
            extract_value_by_path(&data, "$.body.items[1].name"),
            Some(json!("second"))
        );
        assert_eq!(
            extract_value_by_path(&data, "$.body.users[2]"),
            Some(json!(3))
        );
        // 负索引
        assert_eq!(
            extract_value_by_path(&data, "$.body.items[-1].name"),
            Some(json!("second"))
        );
        assert_eq!(
            extract_value_by_path(&data, "$.body.users[-2]"),
            Some(json!(2))
        );
    }

    #[test]
    fn test_extract_value_by_jsonpath_edge_cases() {
        let data = json!({
            "status": 200,
            "body": {"key": "value"},
            "items": [1, 2, 3]
        });

        // 空路径
        assert_eq!(extract_value_by_path(&data, ""), None);

        // 根路径
        let root = extract_value_by_path(&data, "$");
        assert!(root.is_some());

        // 嵌套对象
        assert_eq!(
            extract_value_by_path(&data, "$.body"),
            Some(json!({"key": "value"}))
        );

        // 数组索引
        assert_eq!(extract_value_by_path(&data, "$.items[0]"), Some(json!(1)));

        // 越界索引
        let data_with_items = json!({"items": [1, 2, 3]});
        assert_eq!(extract_value_by_path(&data_with_items, "$.items[10]"), None);
    }

    #[test]
    fn test_extract_value_by_jsonpath_invalid_syntax() {
        let data = json!({"key": "value"});

        // 无效语法返回 None（不 panic）
        assert_eq!(extract_value_by_path(&data, "invalid["), None);
        assert_eq!(extract_value_by_path(&data, "$..[["), None);
    }

    #[test]
    fn test_apply_output_mappings_basic() {
        use crate::context::ExecutionContext;
        use crate::variable::OutputMapping;

        let response = serde_json::json!({
            "status": 200,
            "body": {"token": "abc"}
        });

        let mappings = vec![
            OutputMapping {
                source: "$.status".to_string(),
                target: "status_code".to_string(),
                description: None,
            },
            OutputMapping {
                source: "$.body.token".to_string(),
                target: "auth_token".to_string(),
                description: None,
            },
        ];

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        apply_output_mappings(&response, &mappings, &mut context, "node-001");

        let status_var = context
            .variables
            .variables
            .iter()
            .find(|v| v.name == "status_code");
        assert!(status_var.is_some());
        assert_eq!(status_var.unwrap().value, serde_json::json!(200));

        let token_var = context
            .variables
            .variables
            .iter()
            .find(|v| v.name == "auth_token");
        assert!(token_var.is_some());
        assert_eq!(token_var.unwrap().value, serde_json::json!("abc"));
    }

    #[test]
    fn test_unix_timestamp_ms_returns_recent_value() {
        let ts = super::unix_timestamp_ms();
        // 2023-01-01 in milliseconds: 1672531200000
        // Any value less than this means the clock is not set to a recent time
        assert!(
            ts > 1_672_531_200_000,
            "unix_timestamp_ms() returned {} which is before 2023-01-01",
            ts
        );
    }

    #[test]
    fn test_unix_timestamp_ms_increases_over_time() {
        let t1 = super::unix_timestamp_ms();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let t2 = super::unix_timestamp_ms();
        assert!(
            t2 >= t1,
            "timestamps should not go backwards: t1={}, t2={}",
            t1,
            t2
        );
    }
}
