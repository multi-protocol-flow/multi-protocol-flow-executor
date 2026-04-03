//! 通用工具函数

use serde_json_path::JsonPath;

/// 按 JSONPath 从 JSON 中提取值
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
}
