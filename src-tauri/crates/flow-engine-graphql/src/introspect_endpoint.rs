//! GraphQL 端点内省
//!
//! 向 GraphQL 端点发送完整的 `__schema` 内省查询，返回 Schema 数据，
//! 并可选生成基本的 query/mutation 查询模板。供 Tauri IPC 命令调用。

use std::collections::{HashMap, HashSet};

use flow_engine_core::traits::{GraphqlIntrospectOutput, GraphqlQueryTemplate};

/// GraphQL Schema 内省
///
/// 设计时调用：向 GraphQL 端点发送内省查询，返回完整的 Schema 信息。
/// 不自动跟随重定向，手动处理 301→POST（保持 POST 方法，最多 5 次）。
pub async fn introspect_endpoint(
    endpoint: &str,
    headers: Option<&HashMap<String, String>>,
) -> GraphqlIntrospectOutput {
    log::info!("[GraphQL] Introspect request: endpoint={}", endpoint);

    // URL 格式验证
    if endpoint.is_empty() {
        return GraphqlIntrospectOutput {
            success: false,
            schema: None,
            query_templates: None,
            error: Some("Endpoint URL must not be empty".to_string()),
        };
    }
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        return GraphqlIntrospectOutput {
            success: false,
            schema: None,
            query_templates: None,
            error: Some("Endpoint URL must start with http:// or https://".to_string()),
        };
    }

    // GraphQL 内省查询
    let introspection_query = serde_json::json!({
        "query": r#"query IntrospectionQuery {
          __schema {
            queryType { name }
            mutationType { name }
            subscriptionType { name }
            types {
              kind name description
              fields(includeDeprecated: true) {
                name description
                type { ...TypeRef }
                args {
                  name description
                  type { ...TypeRef }
                  defaultValue
                }
              }
              inputFields {
                name description
                type { ...TypeRef }
                defaultValue
              }
              enumValues(includeDeprecated: true) { name description }
            }
          }
        }
        fragment TypeRef on __Type {
          kind name
          ofType { kind name ofType { kind name ofType { kind name ofType { kind name } } } }
        }"#
    });

    // 构建 HTTP 客户端（不自动跟随重定向，手动处理 301→POST）
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e));

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return GraphqlIntrospectOutput {
                success: false,
                schema: None,
                query_templates: None,
                error: Some(e),
            }
        }
    };

    // 手动跟随重定向（保持 POST 方法）
    let max_redirects = 5u32;
    let mut current_url = endpoint.to_string();
    let mut result = None;

    for _ in 0..=max_redirects {
        let mut req = client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&introspection_query);

        if let Some(headers) = headers {
            for (key, value) in headers {
                req = req.header(key, value);
            }
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_redirection() {
                    // 跟随重定向
                    if let Some(location) =
                        resp.headers().get("location").and_then(|v| v.to_str().ok())
                    {
                        let next_url = if location.starts_with("http://")
                            || location.starts_with("https://")
                        {
                            location.to_string()
                        } else if location.starts_with('/') {
                            // 以 / 开头的相对路径 → 相对 origin 解析
                            let origin = current_url
                                .split('/')
                                .take(3) // ["https:", "", "host"]
                                .collect::<Vec<_>>()
                                .join("/");
                            format!("{}{}", origin, location)
                        } else {
                            // 相对当前路径
                            let base = current_url.trim_end_matches('/');
                            format!("{}/{}", base, location.trim_start_matches('/'))
                        };
                        current_url = next_url;
                        continue;
                    }
                    result = Some(GraphqlIntrospectOutput {
                        success: false,
                        schema: None,
                        query_templates: None,
                        error: Some(format!("Redirect without Location header: {}", status)),
                    });
                    break;
                }

                // 非重定向 → 解析响应
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        if status.is_success() {
                            if let Some(errors) = body.get("errors") {
                                result = Some(GraphqlIntrospectOutput {
                                    success: false,
                                    schema: None,
                                    query_templates: None,
                                    error: Some(format!("GraphQL error: {}", errors)),
                                });
                            } else {
                                let schema = body.get("data").cloned().unwrap_or(body);
                                let query_templates = generate_graphql_templates(&schema);
                                result = Some(GraphqlIntrospectOutput {
                                    success: true,
                                    schema: Some(schema),
                                    query_templates,
                                    error: None,
                                });
                            }
                        } else {
                            result = Some(GraphqlIntrospectOutput {
                                success: false,
                                schema: None,
                                query_templates: None,
                                error: Some(format!("HTTP error: {}", status)),
                            });
                        }
                    }
                    Err(e) => {
                        result = Some(GraphqlIntrospectOutput {
                            success: false,
                            schema: None,
                            query_templates: None,
                            error: Some(format!("Failed to parse response: {}", e)),
                        });
                    }
                }
                break;
            }
            Err(e) => {
                result = Some(GraphqlIntrospectOutput {
                    success: false,
                    schema: None,
                    query_templates: None,
                    error: Some(format!("Request failed: {}", e)),
                });
                break;
            }
        }
    }

    result.unwrap_or_else(|| GraphqlIntrospectOutput {
        success: false,
        schema: None,
        query_templates: None,
        error: Some("Too many redirects".to_string()),
    })
}

/// 从 Schema 类型定义中提取字段的嵌套子字段（递归，最大深度 2）
fn generate_nested_fields(
    type_name: &str,
    types_map: &HashMap<&str, &serde_json::Value>,
    depth: usize,
    visited: &mut HashSet<String>,
) -> String {
    if depth > 2 || visited.contains(type_name) {
        return String::new();
    }
    visited.insert(type_name.to_string());

    let type_def = match types_map.get(type_name) {
        Some(t) => t,
        None => return String::new(),
    };

    let fields = match type_def.get("fields").and_then(|f| f.as_array()) {
        Some(f) => f,
        None => return String::new(), // SCALAR/ENUM → 无子字段
    };

    let mut sub_fields = Vec::new();
    for field in fields {
        let name = match field.get("name").and_then(|n| n.as_str()) {
            Some(n) if !n.starts_with("__") => n,
            _ => continue,
        };

        // 提取字段的返回值类型名（剥掉 NON_NULL / LIST 包装）
        let return_type = resolve_type_name(field.get("type"));
        if let Some(rt) = return_type {
            if is_leaf_type(&rt) {
                sub_fields.push(name.to_string());
            } else {
                let nested = generate_nested_fields(&rt, types_map, depth + 1, visited);
                if nested.is_empty() {
                    sub_fields.push(name.to_string());
                } else {
                    sub_fields.push(format!("{} {{\n{}  }}", name, nested));
                }
            }
        } else {
            sub_fields.push(name.to_string());
        }
    }

    visited.remove(type_name);
    sub_fields.join("\n  ")
}

/// 判断类型是否为叶子类型（SCALAR / ENUM）
fn is_leaf_type(name: &str) -> bool {
    matches!(name, "String" | "Int" | "Float" | "Boolean" | "ID")
}

/// 从 type 引用结构中提取实际类型名（剥掉 `NON_NULL` / LIST 包装）
fn resolve_type_name(type_ref: Option<&serde_json::Value>) -> Option<String> {
    let mut current = type_ref?;
    loop {
        match current.get("kind").and_then(|k| k.as_str()) {
            Some("NON_NULL") | Some("LIST") => {
                current = current.get("ofType")?;
            }
            Some(_) => {
                return current
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
            }
            None => return None,
        }
    }
}

/// 从 Schema 数据生成查询模板（含展开的子字段）
fn generate_graphql_templates(schema: &serde_json::Value) -> Option<Vec<GraphqlQueryTemplate>> {
    let schema_obj = schema.get("__schema")?;
    let types = schema_obj.get("types").and_then(|t| t.as_array())?;

    // 构建类型索引: type_name → type_def
    let types_map: HashMap<&str, &serde_json::Value> = types
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|name| (name, t)))
        .collect();

    let mut templates = Vec::new();

    // 生成 Query 模板
    if let Some(qt_name) = schema_obj
        .get("queryType")
        .and_then(|qt| qt.get("name"))
        .and_then(|n| n.as_str())
    {
        if let Some(query_type_def) = types_map.get(qt_name) {
            if let Some(query_fields) = query_type_def.get("fields").and_then(|f| f.as_array()) {
                for field in query_fields {
                    let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name.starts_with("__") {
                        continue;
                    }

                    let field_return_type = resolve_type_name(field.get("type"));
                    let field_nested = field_return_type
                        .as_ref()
                        .map(|rt| {
                            let mut v = HashSet::new();
                            generate_nested_fields(rt, &types_map, 0, &mut v)
                        })
                        .unwrap_or_default();

                    let template = if field_nested.is_empty() {
                        format!("query {} {{\n  {}\n}}", pascal_case(name), name)
                    } else {
                        format!(
                            "query {} {{\n  {} {{\n{}  }}\n}}",
                            pascal_case(name),
                            name,
                            indent(&field_nested, 4)
                        )
                    };

                    templates.push(GraphqlQueryTemplate {
                        type_name: qt_name.to_string(),
                        operation: "query".to_string(),
                        template,
                    });
                }
            }
        }
    }

    // 生成 Mutation 模板
    if let Some(mutation_type) = schema_obj.get("mutationType").and_then(|mt| mt.get("name")) {
        let type_name = mutation_type.as_str().unwrap_or("Mutation");
        if let Some(mutation_type_def) = types_map.get(type_name) {
            if let Some(mutation_fields) =
                mutation_type_def.get("fields").and_then(|f| f.as_array())
            {
                for field in mutation_fields {
                    let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("");

                    let field_return_type = resolve_type_name(field.get("type"));
                    let field_nested = field_return_type
                        .as_ref()
                        .map(|rt| {
                            let mut v = HashSet::new();
                            generate_nested_fields(rt, &types_map, 0, &mut v)
                        })
                        .unwrap_or_default();

                    let template = if field_nested.is_empty() {
                        format!("mutation {} {{\n  {}\n}}", pascal_case(name), name)
                    } else {
                        format!(
                            "mutation {} {{\n  {} {{\n{}  }}\n}}",
                            pascal_case(name),
                            name,
                            indent(&field_nested, 4)
                        )
                    };

                    templates.push(GraphqlQueryTemplate {
                        type_name: type_name.to_string(),
                        operation: "mutation".to_string(),
                        template,
                    });
                }
            }
        }
    }

    if templates.is_empty() {
        None
    } else {
        Some(templates)
    }
}

/// 首字母大写
fn pascal_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

/// 缩进文本
fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造最小内省 fixture：Query 含叶子字段 + 嵌套对象字段，Mutation 含单字段
    fn sample_schema() -> serde_json::Value {
        serde_json::json!({
            "__schema": {
                "queryType": { "name": "Query" },
                "mutationType": { "name": "Mutation" },
                "types": [
                    {
                        "name": "Query",
                        "kind": "OBJECT",
                        "fields": [
                            {
                                "name": "user",
                                "type": { "kind": "OBJECT", "name": "User", "ofType": null }
                            },
                            {
                                "name": "version",
                                "type": { "kind": "SCALAR", "name": "String", "ofType": null }
                            }
                        ]
                    },
                    {
                        "name": "Mutation",
                        "kind": "OBJECT",
                        "fields": [
                            {
                                "name": "createUser",
                                "type": { "kind": "OBJECT", "name": "User", "ofType": null }
                            }
                        ]
                    },
                    {
                        "name": "User",
                        "kind": "OBJECT",
                        "fields": [
                            {
                                "name": "id",
                                "type": { "kind": "NON_NULL", "name": null, "ofType": { "kind": "SCALAR", "name": "ID", "ofType": null } }
                            },
                            {
                                "name": "name",
                                "type": { "kind": "SCALAR", "name": "String", "ofType": null }
                            }
                        ]
                    },
                    {
                        "name": "String",
                        "kind": "SCALAR",
                        "fields": null
                    },
                    {
                        "name": "ID",
                        "kind": "SCALAR",
                        "fields": null
                    }
                ]
            }
        })
    }

    #[test]
    fn test_generate_templates_query_and_mutation() {
        let schema = sample_schema();
        let templates =
            generate_graphql_templates(&schema).unwrap_or_else(|| panic!("templates expected"));

        // Query: version 是叶子字段 → 无嵌套
        let version = templates
            .iter()
            .find(|t| t.operation == "query" && t.template.contains("version"))
            .unwrap_or_else(|| panic!("version query template missing"));
        assert_eq!(version.type_name, "Query");
        assert_eq!(version.operation, "query");
        assert_eq!(version.template, "query Version {\n  version\n}");

        // Query: user 返回对象 → 展开 id/name 子字段
        let user = templates
            .iter()
            .find(|t| t.operation == "query" && t.template.contains("user"))
            .unwrap_or_else(|| panic!("user query template missing"));
        assert_eq!(user.type_name, "Query");
        assert_eq!(user.operation, "query");
        assert_eq!(
            user.template,
            "query User {\n  user {\n    id\n      name  }\n}"
        );

        // Mutation: createUser → mutation 模板
        let mutation = templates
            .iter()
            .find(|t| t.operation == "mutation")
            .unwrap_or_else(|| panic!("mutation template missing"));
        assert_eq!(mutation.type_name, "Mutation");
        assert_eq!(mutation.operation, "mutation");
        assert_eq!(
            mutation.template,
            "mutation CreateUser {\n  createUser {\n    id\n      name  }\n}"
        );
    }

    #[test]
    fn test_generate_templates_empty_schema_returns_none() {
        assert!(generate_graphql_templates(&serde_json::json!({})).is_none());
        // 有 queryType 但无 fields → 模板为空 → None
        let schema = serde_json::json!({
            "__schema": {
                "queryType": { "name": "Query" },
                "types": [
                    { "name": "Query", "kind": "OBJECT", "fields": [] }
                ]
            }
        });
        assert!(generate_graphql_templates(&schema).is_none());
    }

    #[test]
    fn test_resolve_type_name_unwraps_wrappers() {
        // LIST 套 NON_NULL 套 SCALAR → 最内层名字
        let type_ref = serde_json::json!({
            "kind": "LIST",
            "name": null,
            "ofType": {
                "kind": "NON_NULL",
                "name": null,
                "ofType": { "kind": "SCALAR", "name": "String", "ofType": null }
            }
        });
        assert_eq!(
            resolve_type_name(Some(&type_ref)),
            Some("String".to_string())
        );
        assert_eq!(resolve_type_name(None), None);
    }

    #[test]
    fn test_pascal_case_and_indent() {
        assert_eq!(pascal_case("hello"), "Hello");
        assert_eq!(pascal_case(""), "");
        assert_eq!(indent("a\nb", 4), "    a\n    b");
    }
}
