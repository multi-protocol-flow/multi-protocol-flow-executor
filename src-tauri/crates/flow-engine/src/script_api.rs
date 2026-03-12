//! 脚本节点 API 模块
//!
//! 提供 Rust ↔ QuickJS 桥接层，实现 `fn.variables.*`、`fn.response.*`、`fn.request.*` API

use rquickjs::{function::Func, Ctx, Object};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
}

/// 响应数据（传递给 fn.response）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    /// HTTP 状态码
    pub code: u16,
    /// 状态文本
    pub status: String,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体（自动 JSON 解析后的值）
    pub body: serde_json::Value,
    /// 原始响应体文本
    pub raw_body: String,
    /// 响应大小（字节）
    pub size: usize,
    /// 响应时间（毫秒）
    pub time: u64,
}

/// 请求数据（传递给 fn.request）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestData {
    /// 请求 URL
    pub url: String,
    /// 请求方法
    pub method: String,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体
    pub body: serde_json::Value,
}

/// 脚本执行上下文（Rust ↔ JS 桥接）
///
/// 提供变量存储和日志输出功能，支持在 QuickJS 中访问和修改。
pub struct ScriptExecutionContext {
    /// 变量存储
    pub variables: Rc<RefCell<HashMap<String, serde_json::Value>>>,
    /// 日志输出
    pub logs: Rc<RefCell<Vec<LogEntry>>>,
    /// 响应数据（只读，来自前置节点）
    pub response: Option<ResponseData>,
    /// 请求数据（只读，当前节点配置）
    pub request: Option<RequestData>,
}

impl ScriptExecutionContext {
    /// 创建新的脚本执行上下文
    pub fn new() -> Self {
        Self {
            variables: Rc::new(RefCell::new(HashMap::new())),
            logs: Rc::new(RefCell::new(Vec::new())),
            response: None,
            request: None,
        }
    }

    /// 设置响应数据（builder pattern）
    pub fn with_response(mut self, response: ResponseData) -> Self {
        self.response = Some(response);
        self
    }

    /// 设置请求数据（builder pattern）
    pub fn with_request(mut self, request: RequestData) -> Self {
        self.request = Some(request);
        self
    }

    /// 注册 `fn.variables.*` API 到 QuickJS 上下文
    ///
    /// # 注册的方法
    /// - `fn.variables.get(key)` - 获取变量值（返回字符串或 null）
    /// - `fn.variables.set(key, value)` - 设置变量（字符串值）
    /// - `fn.variables.setAll(obj)` - 批量设置变量
    /// - `fn.variables.has(key)` - 检查变量是否存在
    /// - `fn.variables.unset(key)` - 删除变量
    /// - `fn.variables.clear()` - 清空所有变量
    /// - `fn.variables.all()` - 获取所有变量（返回对象）
    pub fn register_variables_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = Object::new(ctx.clone())?;
        let vars_obj = Object::new(ctx.clone())?;

        // get(key) - 获取变量
        let vars_get = self.variables.clone();
        vars_obj.set(
            "get",
            Func::from(move |key: String| -> Option<String> {
                vars_get
                    .borrow()
                    .get(&key)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            }),
        )?;

        // set(key, value) - 设置变量
        let vars_set = self.variables.clone();
        vars_obj.set(
            "set",
            Func::from(move |key: String, value: String| {
                vars_set
                    .borrow_mut()
                    .insert(key, serde_json::Value::String(value));
            }),
        )?;

        // has(key) - 检查变量是否存在
        let vars_has = self.variables.clone();
        vars_obj.set(
            "has",
            Func::from(move |key: String| -> bool { vars_has.borrow().contains_key(&key) }),
        )?;

        // unset(key) - 删除变量
        let vars_unset = self.variables.clone();
        vars_obj.set(
            "unset",
            Func::from(move |key: String| {
                vars_unset.borrow_mut().remove(&key);
            }),
        )?;

        // clear() - 清空所有变量
        let vars_clear = self.variables.clone();
        vars_obj.set(
            "clear",
            Func::from(move || {
                vars_clear.borrow_mut().clear();
            }),
        )?;

        // setAll(obj) - 批量设置变量
        let vars_set_all = self.variables.clone();
        vars_obj.set(
            "setAll",
            Func::from(move |obj: Object| {
                let keys: Vec<String> = Object::keys(&obj).filter_map(|k| k.ok()).collect();
                for key in keys {
                    if let Ok(value) = obj.get::<String, String>(key.clone()) {
                        vars_set_all
                            .borrow_mut()
                            .insert(key, serde_json::Value::String(value));
                    }
                }
            }),
        )?;

        // all() - 获取所有变量
        // 注意：返回一个 JSON 字符串而不是 Object，以避免生命周期问题
        // 用户可以在 JS 中使用 JSON.parse(fn.variables.all()) 来获取对象
        let vars_all = self.variables.clone();
        vars_obj.set(
            "all",
            Func::from(move || -> String {
                let map: HashMap<String, String> = vars_all
                    .borrow()
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect();
                serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
            }),
        )?;

        fn_obj.set("variables", vars_obj)?;
        ctx.globals().set("fn", fn_obj)?;
        Ok(())
    }

    /// 注册 `fn.response.*` API 到 QuickJS 上下文
    ///
    /// # 注册的属性
    /// - `fn.response.code` - HTTP 状态码
    /// - `fn.response.status` - 状态文本
    /// - `fn.response.headers` - 响应头对象
    /// - `fn.response.body` - 响应体（自动 JSON 解析）
    /// - `fn.response.size` - 响应大小（字节）
    /// - `fn.response.time` - 响应时间（毫秒）
    /// - `fn.response.text()` - 获取原始文本
    /// - `fn.response.json()` - 强制 JSON 解析
    pub fn register_response_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = ctx.globals().get::<_, Object>("fn")?;
        let response_obj = Object::new(ctx.clone())?;

        if let Some(ref resp) = self.response {
            response_obj.set("code", resp.code)?;
            response_obj.set("status", resp.status.clone())?;

            // headers 对象
            let headers_obj = Object::new(ctx.clone())?;
            for (key, value) in &resp.headers {
                headers_obj.set(key, value.clone())?;
            }
            response_obj.set("headers", headers_obj)?;

            // body - 直接设置为 JSON 字符串，JS 端可用 JSON.parse
            response_obj.set("body", resp.body.to_string())?;

            response_obj.set("size", resp.size)?;
            response_obj.set("time", resp.time)?;

            // text() 函数 - 返回原始响应体
            let raw_body = resp.raw_body.clone();
            response_obj.set("text", Func::from(move || -> String { raw_body.clone() }))?;

            // json() 函数 - 返回 JSON 字符串（JS 端可用 JSON.parse）
            let body_clone = resp.body.clone();
            response_obj.set(
                "json",
                Func::from(move || -> String { body_clone.to_string() }),
            )?;
        }

        fn_obj.set("response", response_obj)?;
        Ok(())
    }

    /// 注册 `fn.request.*` API 到 QuickJS 上下文
    ///
    /// # 注册的属性
    /// - `fn.request.url` - 请求 URL
    /// - `fn.request.method` - 请求方法
    /// - `fn.request.headers` - 请求头对象
    /// - `fn.request.body` - 请求体
    pub fn register_request_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = ctx.globals().get::<_, Object>("fn")?;
        let request_obj = Object::new(ctx.clone())?;

        if let Some(ref req) = self.request {
            request_obj.set("url", req.url.clone())?;
            request_obj.set("method", req.method.clone())?;

            // headers 对象
            let headers_obj = Object::new(ctx.clone())?;
            for (key, value) in &req.headers {
                headers_obj.set(key, value.clone())?;
            }
            request_obj.set("headers", headers_obj)?;

            // body - 直接设置为 JSON 字符串，JS 端可用 JSON.parse
            request_obj.set("body", req.body.to_string())?;
        }

        fn_obj.set("request", request_obj)?;
        Ok(())
    }

    /// 注册所有 API 到 QuickJS 上下文
    ///
    /// 包括 fn.variables.*、fn.response.* 和 fn.request.*
    pub fn register_all_apis(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        self.register_variables_api(ctx)?;
        self.register_response_api(ctx)?;
        self.register_request_api(ctx)?;
        Ok(())
    }
}

impl Default for ScriptExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{CatchResultExt, Context, Object, Runtime};

    #[test]
    fn test_fn_variables_set_get() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_variables_api(&ctx).unwrap();
            let _: () = ctx
                .eval("fn.variables.set('test', 'hello')")
                .catch(&ctx)
                .unwrap();
            let result: String = ctx.eval("fn.variables.get('test')").catch(&ctx).unwrap();
            assert_eq!(result, "hello");
        });
    }

    #[test]
    fn test_fn_variables_has() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_variables_api(&ctx).unwrap();
            let _: () = ctx
                .eval("fn.variables.set('exists', 'value')")
                .catch(&ctx)
                .unwrap();
            let has_key: bool = ctx.eval("fn.variables.has('exists')").catch(&ctx).unwrap();
            assert!(has_key);
            let not_has: bool = ctx
                .eval("fn.variables.has('nonexistent')")
                .catch(&ctx)
                .unwrap();
            assert!(!not_has);
        });
    }

    #[test]
    fn test_fn_variables_unset() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_variables_api(&ctx).unwrap();
            let _: () = ctx
                .eval("fn.variables.set('remove_me', 'value')")
                .catch(&ctx)
                .unwrap();
            let _: () = ctx
                .eval("fn.variables.unset('remove_me')")
                .catch(&ctx)
                .unwrap();
            let result: Option<String> = ctx
                .eval("fn.variables.get('remove_me')")
                .catch(&ctx)
                .unwrap();
            assert!(result.is_none());
        });
    }

    #[test]
    fn test_fn_variables_clear() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_variables_api(&ctx).unwrap();
            let _: () = ctx.eval("fn.variables.set('a', '1')").catch(&ctx).unwrap();
            let _: () = ctx.eval("fn.variables.set('b', '2')").catch(&ctx).unwrap();
            let _: () = ctx.eval("fn.variables.clear()").catch(&ctx).unwrap();
            let has_a: bool = ctx.eval("fn.variables.has('a')").catch(&ctx).unwrap();
            assert!(!has_a);
        });
    }

    #[test]
    fn test_fn_variables_set_all() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_variables_api(&ctx).unwrap();
            let _: () = ctx
                .eval("fn.variables.setAll({ x: '10', y: '20' })")
                .catch(&ctx)
                .unwrap();
            let x: String = ctx.eval("fn.variables.get('x')").catch(&ctx).unwrap();
            let y: String = ctx.eval("fn.variables.get('y')").catch(&ctx).unwrap();
            assert_eq!(x, "10");
            assert_eq!(y, "20");
        });
    }

    #[test]
    fn test_fn_variables_all() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_variables_api(&ctx).unwrap();
            let _: () = ctx.eval("fn.variables.set('a', '1')").catch(&ctx).unwrap();
            let _: () = ctx.eval("fn.variables.set('b', '2')").catch(&ctx).unwrap();

            // 获取所有变量并检查（返回 JSON 字符串，需要解析）
            let all_json: String = ctx.eval("fn.variables.all()").catch(&ctx).unwrap();
            let all_obj: Object = ctx
                .eval(format!("JSON.parse('{}')", all_json).as_bytes())
                .catch(&ctx)
                .unwrap();
            let a_val: String = all_obj.get("a").unwrap();
            let b_val: String = all_obj.get("b").unwrap();
            assert_eq!(a_val, "1");
            assert_eq!(b_val, "2");
        });
    }

    #[test]
    fn test_fn_variables_overwrite() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_variables_api(&ctx).unwrap();
            let _: () = ctx
                .eval("fn.variables.set('key', 'first')")
                .catch(&ctx)
                .unwrap();
            let _: () = ctx
                .eval("fn.variables.set('key', 'second')")
                .catch(&ctx)
                .unwrap();
            let result: String = ctx.eval("fn.variables.get('key')").catch(&ctx).unwrap();
            assert_eq!(result, "second");
        });
    }

    #[test]
    fn test_script_execution_context_default() {
        let ctx = ScriptExecutionContext::default();
        assert!(ctx.variables.borrow().is_empty());
        assert!(ctx.logs.borrow().is_empty());
        assert!(ctx.response.is_none());
        assert!(ctx.request.is_none());
    }

    #[test]
    fn test_fn_response_with_data() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        let response = ResponseData {
            code: 200,
            status: "OK".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: serde_json::json!({"token": "abc123"}),
            raw_body: r#"{"token":"abc123"}"#.to_string(),
            size: 19,
            time: 150,
        };

        let script_ctx = ScriptExecutionContext::new().with_response(response);

        ctx.with(|ctx| {
            script_ctx.register_all_apis(&ctx).unwrap();

            let code: u16 = ctx.eval("fn.response.code").catch(&ctx).unwrap();
            assert_eq!(code, 200);

            let status: String = ctx.eval("fn.response.status").catch(&ctx).unwrap();
            assert_eq!(status, "OK");

            let content_type: String = ctx
                .eval("fn.response.headers['Content-Type']")
                .catch(&ctx)
                .unwrap();
            assert_eq!(content_type, "application/json");

            let size: usize = ctx.eval("fn.response.size").catch(&ctx).unwrap();
            assert_eq!(size, 19);

            let time: u64 = ctx.eval("fn.response.time").catch(&ctx).unwrap();
            assert_eq!(time, 150);

            // 测试 text() 函数
            let text: String = ctx.eval("fn.response.text()").catch(&ctx).unwrap();
            assert_eq!(text, r#"{"token":"abc123"}"#);

            // 测试 json() 函数
            let json: String = ctx.eval("fn.response.json()").catch(&ctx).unwrap();
            assert!(json.contains("token"));
        });
    }

    #[test]
    fn test_fn_response_null_when_no_response() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        let script_ctx = ScriptExecutionContext::new(); // response = None

        ctx.with(|ctx| {
            script_ctx.register_all_apis(&ctx).unwrap();

            // 访问不存在的 response 应该返回 undefined
            let code: Option<u16> = ctx.eval("fn.response?.code").catch(&ctx).unwrap();
            assert!(code.is_none());
        });
    }

    #[test]
    fn test_fn_response_body_parsing() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        let response = ResponseData {
            code: 200,
            status: "OK".to_string(),
            headers: HashMap::new(),
            body: serde_json::json!({
                "user": {
                    "id": 1,
                    "name": "test"
                },
                "items": [1, 2, 3]
            }),
            raw_body: r#"{"user":{"id":1,"name":"test"},"items":[1,2,3]}"#.to_string(),
            size: 50,
            time: 100,
        };

        let script_ctx = ScriptExecutionContext::new().with_response(response);

        ctx.with(|ctx| {
            script_ctx.register_all_apis(&ctx).unwrap();

            // body 返回 JSON 字符串，可以用 JSON.parse 解析
            let body_str: String = ctx.eval("fn.response.body").catch(&ctx).unwrap();
            let parsed: Object = ctx
                .eval(format!("JSON.parse('{}')", body_str).as_bytes())
                .catch(&ctx)
                .unwrap();
            let user: Object = parsed.get("user").unwrap();
            let id: i32 = user.get("id").unwrap();
            assert_eq!(id, 1);
        });
    }

    #[test]
    fn test_fn_request_with_data() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        let request = RequestData {
            url: "https://api.example.com/users".to_string(),
            method: "POST".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: serde_json::json!({"name": "test"}),
        };

        let script_ctx = ScriptExecutionContext::new().with_request(request);

        ctx.with(|ctx| {
            script_ctx.register_all_apis(&ctx).unwrap();

            let url: String = ctx.eval("fn.request.url").catch(&ctx).unwrap();
            assert_eq!(url, "https://api.example.com/users");

            let method: String = ctx.eval("fn.request.method").catch(&ctx).unwrap();
            assert_eq!(method, "POST");

            let content_type: String = ctx
                .eval("fn.request.headers['Content-Type']")
                .catch(&ctx)
                .unwrap();
            assert_eq!(content_type, "application/json");
        });
    }

    #[test]
    fn test_fn_request_null_when_no_request() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        let script_ctx = ScriptExecutionContext::new(); // request = None

        ctx.with(|ctx| {
            script_ctx.register_all_apis(&ctx).unwrap();

            // 当没有请求数据时，fn.request 对象存在但属性为 undefined
            // 使用可选链访问会返回 undefined (在 rquickjs 中转换为 null)
            let result: Option<String> = ctx.eval("fn.request?.url").catch(&ctx).unwrap();
            assert!(result.is_none());
        });
    }
}
