//! 脚本节点 API 模块
//!
//! 提供 Rust ↔ `QuickJS` 桥接层，实现 `fn.variables.*`、`fn.response.*`、`fn.request.*` API

use rquickjs::{function::Func, Ctx, Object, Value};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// 日志级别（log、info、warn、error、debug、table）
    pub level: String,
    /// 日志消息内容
    pub message: String,
}

/// 响应数据（传递给 fn.response）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    /// HTTP 状态码（非 HTTP 节点为 0）
    pub code: u16,
    /// 状态文本（非 HTTP 节点为空字符串）
    pub status: String,
    /// 响应头（非 HTTP 节点为空）
    pub headers: HashMap<String, String>,
    /// 响应体（自动 JSON 解析后的值，非 HTTP 节点为完整 output）
    pub body: serde_json::Value,
    /// 原始响应体文本
    pub raw_body: String,
    /// 响应大小（字节）
    pub size: usize,
    /// 响应时间（毫秒）
    pub time: u64,
    /// 完整原始输出数据（不限协议类型，即 `last_node_output` 的完整内容）
    pub raw_output: serde_json::Value,
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

/// 测试断言结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// 断言是否通过
    pub passed: bool,
    /// 断言描述信息
    pub message: String,
    /// 期望值
    pub expected: Option<serde_json::Value>,
    /// 实际值
    pub actual: Option<serde_json::Value>,
}

/// `JS` 值与 JSON 互转的深度上限：超过后按 null 截断。
///
/// 脚本可构造循环引用对象（`const a = {}; a.self = a;`），递归转换必须在有限深度内停止，
/// 否则会栈溢出直接打死宿主进程。
const MAX_JS_JSON_DEPTH: usize = 32;

/// 单个数组允许转换的元素数上限：超过则整体落为 `null`。
///
/// 沙箱内存限制只约束真实数据，`arr.length = 1e9` 这类稀疏数组不占 JS 堆，必须在转换侧设闸。
const MAX_JS_JSON_ARRAY_ELEMENTS: usize = 1_000_000;

/// 数组转换时的预分配上限：避免稀疏数组的 `length` 直接变成多 GB 的宿主分配
const ARRAY_PREALLOC_ELEMENTS: usize = 4096;

/// `JS` 值 → `serde_json::Value`
///
/// `fn.util.*` 的对象入参必须经过这里。直接 `obj.get::<_, String>(key)` 读取属性只能拿到
/// 字符串——数字、布尔、嵌套对象、数组一律转换失败并被丢弃，这正是
/// `pick` / `omit` / `merge` / `clone` / `values` / `entries` / `jsonStringify`
/// 对非字符串对象静默返回 `{}` / `[]` 的根因。
fn js_value_to_json(value: &Value) -> serde_json::Value {
    js_value_to_json_at(value, 0).unwrap_or(serde_json::Value::Null)
}

/// 递归转换；返回 `None` 表示该属性应被丢弃（函数属性，与 `JSON.stringify` 一致）；
/// 对象属性值为 `undefined` 同样会被丢弃，数组元素则为 `null`。
fn js_value_to_json_at(value: &Value, depth: usize) -> Option<serde_json::Value> {
    use serde_json::Value as Json;

    if value.is_undefined() || value.is_null() {
        return Some(Json::Null);
    }
    if value.is_function() {
        return None;
    }
    if let Some(b) = value.as_bool() {
        return Some(Json::Bool(b));
    }
    if let Some(n) = value.as_number() {
        return Some(number_to_json(n));
    }
    if let Some(s) = value.as_string() {
        // 原样保留：不再把字符串二次解析成 JSON（那会把 "123" 变成数字、把 "null" 变成 null）
        return s.to_string().ok().map(Json::String);
    }
    if depth >= MAX_JS_JSON_DEPTH {
        return Some(Json::Null);
    }

    let obj = value.clone().into_object()?;

    if value.is_array() {
        let len = obj.get::<_, i32>("length").unwrap_or(0).max(0) as usize;

        // 稀疏数组的 `length` 只是一个数字：`const a = []; a.length = 1e9;` 在 JS 侧几乎不占内存，
        // 但按 length 预分配 / 逐位转换会让宿主直接 OOM 或长时间卡死（脚本超时不会中断 Rust 侧循环）。
        // 因此：预分配按小上限封顶，并按元素数硬限制转换规模，超限数组整体落为 null（与深度上限同一策略）。
        if len > MAX_JS_JSON_ARRAY_ELEMENTS {
            return Some(Json::Null);
        }
        let mut items = Vec::with_capacity(len.min(ARRAY_PREALLOC_ELEMENTS));
        for index in 0..len {
            items.push(
                obj.get::<_, Value>(index as u32)
                    .ok()
                    .and_then(|item| js_value_to_json_at(&item, depth + 1))
                    .unwrap_or(Json::Null),
            );
        }
        return Some(Json::Array(items));
    }

    let mut map = serde_json::Map::new();
    for key in Object::keys::<String>(&obj).filter_map(|k| k.ok()) {
        let Ok(prop) = obj.get::<_, Value>(key.as_str()) else {
            continue;
        };
        // `JSON.stringify` 丢弃 undefined 属性（数组元素则落成 null，见上面的分支）
        if prop.is_undefined() {
            continue;
        }
        if let Some(converted) = js_value_to_json_at(&prop, depth + 1) {
            map.insert(key, converted);
        }
    }
    Some(Json::Object(map))
}

/// JS 数字 → JSON 数字：整数值保留整数形态，`NaN` / `Infinity` 落到 `null`
/// （`JSON.stringify` 对二者的处理也是 `null`）
fn number_to_json(n: f64) -> serde_json::Value {
    const MAX_EXACT_INT_F64: f64 = 9_007_199_254_740_992.0;
    if !n.is_finite() {
        return serde_json::Value::Null;
    }
    if n.fract() == 0.0 && n.abs() < MAX_EXACT_INT_F64 {
        return serde_json::Value::Number((n as i64).into());
    }
    serde_json::Number::from_f64(n).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

/// 对象/数组归一化为 `{key: value}` 形式；其他类型返回空对象
fn to_object_map(value: &Value) -> serde_json::Map<String, serde_json::Value> {
    match js_value_to_json(value) {
        serde_json::Value::Object(map) => map,
        serde_json::Value::Array(items) => items
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect(),
        _ => serde_json::Map::new(),
    }
}

/// `pick` / `omit` 的键列表参数：接受原生数组 `['a','b']`，也兼容旧的 JSON 字符串 `'["a","b"]'`
fn coerce_key_list(value: &Value) -> Vec<String> {
    if let Some(s) = value.as_string().and_then(|s| s.to_string().ok()) {
        if let Ok(list) = serde_json::from_str::<Vec<String>>(&s) {
            return list;
        }
        // 单个键名也接受，避免调用方必须懂 JSON 字符串约定
        return vec![s];
    }
    match js_value_to_json(value) {
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|item| match item {
                serde_json::Value::String(s) => Some(s),
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        serde_json::Value::Object(map) => map.into_iter().map(|(k, _)| k).collect(),
        _ => Vec::new(),
    }
}

/// 序列化为紧凑 JSON 文本（`fn.util.*` 的对象类 helper 统一返回 JSON 字符串）
fn to_json_text(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

/// 序列化为 JSON 文本，`indent > 0` 时按指定空格数美化
fn to_json_text_with_indent(value: &serde_json::Value, indent: i32) -> String {
    use serde::Serialize;

    if indent <= 0 {
        return to_json_text(value);
    }
    let pad = vec![b' '; indent.min(16) as usize];
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(&pad);
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    match value.serialize(&mut serializer) {
        Ok(()) => String::from_utf8(buf).unwrap_or_else(|_| to_json_text(value)),
        Err(_) => to_json_text(value),
    }
}

/// 脚本执行上下文（Rust ↔ JS 桥接）
///
/// 提供变量存储和日志输出功能，支持在 `QuickJS` 中访问和修改。
pub struct ScriptExecutionContext {
    /// 变量存储
    pub variables: Rc<RefCell<HashMap<String, serde_json::Value>>>,
    /// 日志输出
    pub logs: Rc<RefCell<Vec<LogEntry>>>,
    /// 响应数据（只读，来自前置节点）
    pub response: Option<ResponseData>,
    /// 请求数据（只读，当前节点配置）
    pub request: Option<RequestData>,
    /// 流程控制：跳过后续节点
    pub should_skip: Rc<RefCell<bool>>,
    /// 流程控制：终止流程并报错
    pub should_abort: Rc<RefCell<bool>>,
    /// 流程控制：终止消息
    pub abort_message: Rc<RefCell<Option<String>>>,
    /// 流程控制：正常终止流程
    pub should_stop: Rc<RefCell<bool>>,
    /// 流程控制：终止消息
    pub stop_message: Rc<RefCell<Option<String>>>,
    /// 流程信息：流程名称
    pub flow_name: Option<String>,
    /// 流程信息：流程 ID
    pub flow_id: Option<String>,
    /// 流程信息：节点 ID
    pub node_id: Option<String>,
    /// 流程信息：节点名称
    pub node_name: Option<String>,
    /// 测试断言结果
    pub test_results: Rc<RefCell<Vec<TestResult>>>,
}

impl ScriptExecutionContext {
    /// 创建新的脚本执行上下文
    pub fn new() -> Self {
        Self {
            variables: Rc::new(RefCell::new(HashMap::new())),
            logs: Rc::new(RefCell::new(Vec::new())),
            response: None,
            request: None,
            should_skip: Rc::new(RefCell::new(false)),
            should_abort: Rc::new(RefCell::new(false)),
            abort_message: Rc::new(RefCell::new(None)),
            should_stop: Rc::new(RefCell::new(false)),
            stop_message: Rc::new(RefCell::new(None)),
            flow_name: None,
            flow_id: None,
            node_id: None,
            node_name: None,
            test_results: Rc::new(RefCell::new(Vec::new())),
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

    /// 设置流程信息（builder pattern）
    pub fn with_flow_info(mut self, name: String, id: String) -> Self {
        self.flow_name = Some(name);
        self.flow_id = Some(id);
        self
    }

    /// 设置节点信息（builder pattern）
    pub fn with_node_info(mut self, id: String, name: String) -> Self {
        self.node_id = Some(id);
        self.node_name = Some(name);
        self
    }

    /// 注册 `fn.variables.*` API 到 `QuickJS` 上下文
    ///
    /// # 注册的方法
    /// - `fn.variables.get(key)` - 获取变量值（支持普通变量与动态变量如 $guid，返回字符串或 null）
    /// - `fn.variables.replaceIn(template)` - 替换模板字符串中的变量与动态变量
    /// - `fn.variables.set(key, value)` - 设置变量（字符串值）
    /// - `fn.variables.setAll(obj)` - 批量设置变量
    /// - `fn.variables.has(key)` - 检查变量是否存在
    /// - `fn.variables.unset(key)` - 删除变量
    /// - `fn.variables.clear()` - 清空所有变量
    /// - `fn.variables.all()` - 获取所有变量（返回对象）
    pub fn register_variables_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = Object::new(ctx.clone())?;
        let vars_obj = Object::new(ctx.clone())?;

        // get(key) - 获取变量（支持普通变量与动态变量）
        let vars_get = self.variables.clone();
        vars_obj.set(
            "get",
            Func::from(move |key: String| -> Option<String> {
                let user_val = vars_get.borrow().get(&key).and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => v.as_str().map(|s| s.to_string()),
                });
                if user_val.is_some() {
                    return user_val;
                }
                if key.starts_with('$') {
                    return flow_engine_core::dynamic_variable::resolve_dynamic_variable(&key);
                }
                None
            }),
        )?;

        // replaceIn(template) - 替换模板字符串中的普通变量与动态变量
        let vars_replace = self.variables.clone();
        vars_obj.set(
            "replaceIn",
            Func::from(move |template: String| -> String {
                if !template.contains("{{") {
                    return template;
                }
                let mut exec_ctx =
                    flow_engine_core::context::ExecutionContext::new("script-eval".to_string(), 0);
                for (k, v) in vars_replace.borrow().iter() {
                    exec_ctx
                        .variables
                        .set_variable(k.clone(), v.clone(), "script".to_string());
                }
                flow_engine_core::executor::replace_variables(&template, &exec_ctx)
                    .unwrap_or(template)
            }),
        )?;

        // set(key, value) - 设置变量（任意 JS 值按 JSON 类型保存）
        let vars_set = self.variables.clone();
        vars_obj.set(
            "set",
            Func::from(move |key: String, value: Value| {
                vars_set.borrow_mut().insert(key, js_value_to_json(&value));
            }),
        )?;

        // has(key) - 检查变量是否存在（支持普通变量与动态变量）
        let vars_has = self.variables.clone();
        vars_obj.set(
            "has",
            Func::from(move |key: String| -> bool {
                vars_has.borrow().contains_key(&key)
                    || (key.starts_with('$')
                        && flow_engine_core::dynamic_variable::is_dynamic_variable(&key))
            }),
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

        // setAll(obj) - 批量设置变量（任意 JS 值按 JSON 类型保存）
        let vars_set_all = self.variables.clone();
        vars_obj.set(
            "setAll",
            Func::from(move |obj: Value| {
                let entries = match js_value_to_json(&obj) {
                    serde_json::Value::Object(map) => map,
                    _ => return,
                };
                let mut vars = vars_set_all.borrow_mut();
                for (key, value) in entries {
                    vars.insert(key, value);
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
                let snapshot: serde_json::Map<String, serde_json::Value> = vars_all
                    .borrow()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                to_json_text(&serde_json::Value::Object(snapshot))
            }),
        )?;

        fn_obj.set("variables", vars_obj)?;
        ctx.globals().set("fn", fn_obj)?;
        Ok(())
    }

    /// 注册 `fn.response.*` API 到 `QuickJS` 上下文
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

            // raw - 完整原始输出数据（JSON 字符串，不限协议类型）
            // 对于非 HTTP 节点（数据库/WebSocket/gRPC 等），此为完整的 last_node_output
            // JS 端可用 JSON.parse(fn.response.raw) 获取完整对象
            response_obj.set("raw", resp.raw_output.to_string())?;
        }

        fn_obj.set("response", response_obj)?;
        Ok(())
    }

    /// 注册 `fn.request.*` API 到 `QuickJS` 上下文
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

    /// 注册 `fn.console.*` API 到 `QuickJS` 上下文
    ///
    /// # 注册的方法
    /// - `fn.console.log(message)` - 普通日志
    /// - `fn.console.info(message)` - 信息日志
    /// - `fn.console.warn(message)` - 警告日志
    /// - `fn.console.error(message)` - Error日志
    /// - `fn.console.debug(message)` - 调试日志
    /// - `fn.console.table(data)` - 表格数据
    pub fn register_console_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = ctx.globals().get::<_, Object>("fn")?;
        let console_obj = Object::new(ctx.clone())?;

        // log(message)
        let logs_log = self.logs.clone();
        console_obj.set(
            "log",
            Func::from(move |message: String| {
                logs_log.borrow_mut().push(LogEntry {
                    level: "log".to_string(),
                    message,
                });
            }),
        )?;

        // info(message)
        let logs_info = self.logs.clone();
        console_obj.set(
            "info",
            Func::from(move |message: String| {
                logs_info.borrow_mut().push(LogEntry {
                    level: "info".to_string(),
                    message,
                });
            }),
        )?;

        // warn(message)
        let logs_warn = self.logs.clone();
        console_obj.set(
            "warn",
            Func::from(move |message: String| {
                logs_warn.borrow_mut().push(LogEntry {
                    level: "warn".to_string(),
                    message,
                });
            }),
        )?;

        // error(message)
        let logs_error = self.logs.clone();
        console_obj.set(
            "error",
            Func::from(move |message: String| {
                logs_error.borrow_mut().push(LogEntry {
                    level: "error".to_string(),
                    message,
                });
            }),
        )?;

        // debug(message)
        let logs_debug = self.logs.clone();
        console_obj.set(
            "debug",
            Func::from(move |message: String| {
                logs_debug.borrow_mut().push(LogEntry {
                    level: "debug".to_string(),
                    message,
                });
            }),
        )?;

        // table(data)
        let logs_table = self.logs.clone();
        console_obj.set(
            "table",
            Func::from(move |data: String| {
                logs_table.borrow_mut().push(LogEntry {
                    level: "table".to_string(),
                    message: data,
                });
            }),
        )?;

        fn_obj.set("console", console_obj)?;
        Ok(())
    }

    /// 注册 `fn.flow.*` API 到 `QuickJS` 上下文
    ///
    /// # 注册的方法
    /// - `fn.flow.skip()` - 跳过后续节点
    /// - `fn.flow.abort(message?)` - 终止流程并报错
    /// - `fn.flow.stop(message?)` - 正常终止流程
    pub fn register_flow_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = ctx.globals().get::<_, Object>("fn")?;
        let flow_obj = Object::new(ctx.clone())?;

        // skip()
        let should_skip = self.should_skip.clone();
        flow_obj.set(
            "skip",
            Func::from(move || {
                *should_skip.borrow_mut() = true;
            }),
        )?;

        // abort(message) - 终止流程
        let should_abort = self.should_abort.clone();
        let abort_message = self.abort_message.clone();
        flow_obj.set(
            "abort",
            Func::from(move |message: String| {
                *should_abort.borrow_mut() = true;
                *abort_message.borrow_mut() = if message.is_empty() {
                    None
                } else {
                    Some(message)
                };
            }),
        )?;

        // _stop(message) - 底层实现（始终接收字符串参数）
        let should_stop = self.should_stop.clone();
        let stop_message = self.stop_message.clone();
        flow_obj.set(
            "_stop",
            Func::from(move |message: String| {
                *should_stop.borrow_mut() = true;
                *stop_message.borrow_mut() = if message.is_empty() {
                    None
                } else {
                    Some(message)
                };
            }),
        )?;

        fn_obj.set("flow", flow_obj)?;

        // JS 包装：`fn.flow.stop()` / `fn.flow.abort()` 都支持有参和无参两种调用方式。
        // 底层 Rust 函数参数是 String，直接无参调用会抛 "expected string, got undefined"。
        ctx.eval::<(), _>(
            r#"
            (function() {
                var origStop = fn.flow._stop;
                fn.flow.stop = function(message) {
                    if (arguments.length === 0 || message === undefined) {
                        message = '';
                    }
                    return origStop(message);
                };
                var origAbort = fn.flow.abort;
                fn.flow.abort = function(message) {
                    if (arguments.length === 0 || message === undefined) {
                        message = '';
                    }
                    return origAbort(message);
                };
            })();
        "#,
        )?;

        Ok(())
    }

    /// 注册 `fn.info.*` API 到 `QuickJS` 上下文
    ///
    /// # 注册的属性（只读）
    /// - `fn.info.flowName` - 流程名称
    /// - `fn.info.flowId` - 流程 ID
    /// - `fn.info.nodeId` - 当前节点 ID
    /// - `fn.info.nodeName` - 当前节点名称
    pub fn register_info_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = ctx.globals().get::<_, Object>("fn")?;
        let info_obj = Object::new(ctx.clone())?;

        if let Some(ref name) = self.flow_name {
            info_obj.set("flowName", name.clone())?;
        }
        if let Some(ref id) = self.flow_id {
            info_obj.set("flowId", id.clone())?;
        }
        if let Some(ref id) = self.node_id {
            info_obj.set("nodeId", id.clone())?;
        }
        if let Some(ref name) = self.node_name {
            info_obj.set("nodeName", name.clone())?;
        }

        fn_obj.set("info", info_obj)?;
        Ok(())
    }

    /// 注册 `fn.util.*` API 到 `QuickJS` 上下文
    ///
    /// # 注册的方法
    /// - `fn.util.base64Encode(str)` - Base64 编码
    /// - `fn.util.base64Decode(str)` - Base64 解码
    /// - `fn.util.urlEncode(str)` - URL 编码
    /// - `fn.util.urlDecode(str)` - URL 解码
    /// - `fn.util.htmlEscape(str)` - HTML 转义
    /// - `fn.util.htmlUnescape(str)` - HTML 反转义
    pub fn register_util_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        let fn_obj = ctx.globals().get::<_, Object>("fn")?;
        let util_obj = Object::new(ctx.clone())?;

        // base64Encode(str) - Base64 编码
        util_obj.set(
            "base64Encode",
            Func::from(move |s: String| -> String {
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                STANDARD.encode(s.as_bytes())
            }),
        )?;

        // base64Decode(str) - Base64 解码
        util_obj.set(
            "base64Decode",
            Func::from(move |s: String| -> String {
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                STANDARD
                    .decode(&s)
                    .ok()
                    .and_then(|v| String::from_utf8(v).ok())
                    .unwrap_or_default()
            }),
        )?;

        // urlEncode(str) - URL 编码
        util_obj.set(
            "urlEncode",
            Func::from(move |s: String| -> String { urlencoding::encode(&s).to_string() }),
        )?;

        // urlDecode(str) - URL 解码
        util_obj.set(
            "urlDecode",
            Func::from(move |s: String| -> String {
                urlencoding::decode(&s)
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            }),
        )?;

        // htmlEscape(str) - HTML 转义
        util_obj.set(
            "htmlEscape",
            Func::from(move |s: String| -> String {
                s.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\'', "&#39;")
            }),
        )?;

        // htmlUnescape(str) - HTML 反转义
        util_obj.set(
            "htmlUnescape",
            Func::from(move |s: String| -> String {
                s.replace("&amp;", "&")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&#39;", "'")
            }),
        )?;

        // md5(str) - MD5 哈希
        util_obj.set(
            "md5",
            Func::from(move |s: String| -> String {
                use md5::{Digest, Md5};
                let mut hasher = Md5::new();
                hasher.update(s.as_bytes());
                let result = hasher.finalize();
                hex::encode(result)
            }),
        )?;

        // sha256(str) - SHA-256 哈希
        util_obj.set(
            "sha256",
            Func::from(move |s: String| -> String {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(s.as_bytes());
                let result = hasher.finalize();
                hex::encode(result)
            }),
        )?;

        // sha512(str) - SHA-512 哈希
        util_obj.set(
            "sha512",
            Func::from(move |s: String| -> String {
                use sha2::{Digest, Sha512};
                let mut hasher = Sha512::new();
                hasher.update(s.as_bytes());
                let result = hasher.finalize();
                hex::encode(result)
            }),
        )?;

        // hmacSha256(key, data) - HMAC-SHA256
        util_obj.set(
            "hmacSha256",
            Func::from(move |key: String, data: String| -> String {
                use hmac::{Hmac, Mac};
                use sha2::Sha256;
                type HmacSha256 = Hmac<Sha256>;
                let mut mac = match HmacSha256::new_from_slice(key.as_bytes()) {
                    Ok(m) => m,
                    Err(_) => return String::new(),
                };
                mac.update(data.as_bytes());
                hex::encode(mac.finalize().into_bytes())
            }),
        )?;

        // hmacSha512(key, data) - HMAC-SHA512
        util_obj.set(
            "hmacSha512",
            Func::from(move |key: String, data: String| -> String {
                use hmac::{Hmac, Mac};
                use sha2::Sha512;
                type HmacSha512 = Hmac<Sha512>;
                let mut mac = match HmacSha512::new_from_slice(key.as_bytes()) {
                    Ok(m) => m,
                    Err(_) => return String::new(),
                };
                mac.update(data.as_bytes());
                hex::encode(mac.finalize().into_bytes())
            }),
        )?;

        // === 类型检查函数 ===

        // isArray(v) - 检查是否为数组
        util_obj.set(
            "isArray",
            Func::from(move |v: Value| -> bool { v.is_array() }),
        )?;

        // isObject(v) - 检查是否为对象（非数组、非 null）
        util_obj.set(
            "isObject",
            Func::from(move |v: Value| -> bool { v.is_object() && !v.is_array() && !v.is_null() }),
        )?;

        // isString(v) - 检查是否为字符串
        util_obj.set(
            "isString",
            Func::from(move |v: Value| -> bool { v.is_string() }),
        )?;

        // isNumber(v) - 检查是否为数字
        util_obj.set(
            "isNumber",
            Func::from(move |v: Value| -> bool { v.is_number() }),
        )?;

        // isBoolean(v) - 检查是否为布尔值
        util_obj.set(
            "isBoolean",
            Func::from(move |v: Value| -> bool { v.is_bool() }),
        )?;

        // isNull(v) - 检查是否为 null
        util_obj.set(
            "isNull",
            Func::from(move |v: Value| -> bool { v.is_null() }),
        )?;

        // isUndefined(v) - 检查是否为 undefined
        util_obj.set(
            "isUndefined",
            Func::from(move |v: Value| -> bool { v.is_undefined() }),
        )?;

        // isEmpty(v) - 检查是否为空值
        util_obj.set(
            "isEmpty",
            Func::from(move |v: Value| -> bool {
                if v.is_null() || v.is_undefined() {
                    return true;
                }
                if let Some(s) = v.as_string() {
                    if let Ok(s) = s.to_string() {
                        return s.is_empty();
                    }
                }
                if v.is_array() {
                    // 对于数组，需要转换为对象来检查长度
                    if let Some(obj) = v.clone().into_object() {
                        if let Ok(len) = obj.get::<_, i32>("length") {
                            return len == 0;
                        }
                    }
                }
                if v.is_object() && !v.is_array() {
                    // 对于对象，检查是否有键
                    if let Some(obj) = v.into_object() {
                        return Object::keys::<String>(&obj).next().is_none();
                    }
                }
                false
            }),
        )?;

        // typeOf(v) - 返回类型字符串
        util_obj.set(
            "typeOf",
            Func::from(move |v: Value| -> String {
                if v.is_undefined() {
                    "undefined".to_string()
                } else if v.is_null() {
                    "null".to_string()
                } else if v.is_bool() {
                    "boolean".to_string()
                } else if v.is_number() {
                    "number".to_string()
                } else if v.is_string() {
                    "string".to_string()
                } else if v.is_array() {
                    "array".to_string()
                } else if v.is_object() {
                    "object".to_string()
                } else if v.is_function() {
                    "function".to_string()
                } else {
                    "unknown".to_string()
                }
            }),
        )?;

        // === 对象函数 ===

        // keys(obj) - 返回对象键数组（JSON 字符串）
        util_obj.set(
            "keys",
            Func::from(move |value: Value| -> String {
                let keys: Vec<serde_json::Value> = to_object_map(&value)
                    .into_iter()
                    .map(|(k, _)| serde_json::Value::String(k))
                    .collect();
                to_json_text(&serde_json::Value::Array(keys))
            }),
        )?;

        // values(obj) - 返回对象值数组（JSON 字符串）
        util_obj.set(
            "values",
            Func::from(move |value: Value| -> String {
                let values: Vec<serde_json::Value> = to_object_map(&value).into_values().collect();
                to_json_text(&serde_json::Value::Array(values))
            }),
        )?;

        // entries(obj) - 返回对象键值对数组（JSON 字符串）
        util_obj.set(
            "entries",
            Func::from(move |value: Value| -> String {
                let entries: Vec<serde_json::Value> = to_object_map(&value)
                    .into_iter()
                    .map(|(k, v)| serde_json::Value::Array(vec![serde_json::Value::String(k), v]))
                    .collect();
                to_json_text(&serde_json::Value::Array(entries))
            }),
        )?;

        // hasKey(obj, key) - 检查对象/数组是否包含该键
        // 直接在 JS 对象上查自有键：走 JSON 转换会把 undefined / 函数属性丢掉，
        // 导致 `hasKey({a: undefined}, 'a')` 误报 false，且大对象白跑一次深拷贝。
        util_obj.set(
            "hasKey",
            Func::from(move |value: Value, key: String| -> bool {
                value
                    .into_object()
                    .and_then(|obj| obj.contains_key(key.as_str()).ok())
                    .unwrap_or(false)
            }),
        )?;

        // pick(obj, keys) - 从对象中选择指定键（JSON 字符串）
        util_obj.set(
            "pick",
            Func::from(move |value: Value, keys: Value| -> String {
                let wanted = coerce_key_list(&keys);
                let source = to_object_map(&value);
                let mut result = serde_json::Map::new();
                for key in wanted {
                    if let Some(item) = source.get(&key) {
                        result.insert(key, item.clone());
                    }
                }
                to_json_text(&serde_json::Value::Object(result))
            }),
        )?;

        // omit(obj, keys) - 从对象中排除指定键（JSON 字符串）
        util_obj.set(
            "omit",
            Func::from(move |value: Value, keys: Value| -> String {
                let unwanted = coerce_key_list(&keys);
                let result: serde_json::Map<String, serde_json::Value> = to_object_map(&value)
                    .into_iter()
                    .filter(|(k, _)| !unwanted.contains(k))
                    .collect();
                to_json_text(&serde_json::Value::Object(result))
            }),
        )?;

        // merge(target, source) - 浅合并两个对象，source 覆盖同名键（JSON 字符串）
        util_obj.set(
            "merge",
            Func::from(move |target: Value, source: Value| -> String {
                let mut result = to_object_map(&target);
                for (key, value) in to_object_map(&source) {
                    result.insert(key, value);
                }
                to_json_text(&serde_json::Value::Object(result))
            }),
        )?;

        // clone(obj) - 深拷贝任意值（JSON 字符串；数组/标量同样可用）
        util_obj.set(
            "clone",
            Func::from(move |value: Value| -> String { to_json_text(&js_value_to_json(&value)) }),
        )?;

        // === 时间函数 ===

        // timestamp() - 返回当前 Unix 时间戳（毫秒）
        util_obj.set(
            "timestamp",
            Func::from(move || -> i64 {
                use chrono::Utc;
                Utc::now().timestamp_millis()
            }),
        )?;

        // now() - 返回 ISO 8601 格式的日期字符串
        util_obj.set(
            "now",
            Func::from(move || -> String {
                use chrono::Utc;
                Utc::now().to_rfc3339()
            }),
        )?;

        // formatTime(format) - 使用指定格式格式化当前时间
        // 支持的格式占位符:
        // %Y - 年, %m - 月, %d - 日, %H - 时, %M - 分, %S - 秒
        util_obj.set(
            "formatTime",
            Func::from(move |format: String| -> String {
                use chrono::Utc;
                let now = Utc::now();
                now.format(&format).to_string()
            }),
        )?;

        // parseTime(dateStr, format) - 解析日期字符串为时间戳（毫秒）
        util_obj.set(
            "parseTime",
            Func::from(move |date_str: String, format: String| -> String {
                use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
                // 尝试解析为 RFC3339 格式
                if let Ok(dt) = DateTime::parse_from_rfc3339(&date_str) {
                    return dt.timestamp_millis().to_string();
                }
                // 尝试使用自定义格式解析
                if let Ok(ndt) = NaiveDateTime::parse_from_str(&date_str, &format) {
                    if let Some(dt) = Utc.from_local_datetime(&ndt).single() {
                        return dt.timestamp_millis().to_string();
                    }
                }
                // 解析Failed返回空字符串
                "".to_string()
            }),
        )?;

        // sleep(ms) - 阻塞延迟指定毫秒数
        util_obj.set(
            "sleep",
            Func::from(move |ms: u64| {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }),
        )?;

        // === UUID 函数 ===

        // uuid() - 生成 UUID v4 字符串
        util_obj.set(
            "uuid",
            Func::from(move || -> String {
                use uuid::Uuid;
                Uuid::new_v4().to_string()
            }),
        )?;

        // uuidv4() - uuid() 的别名
        util_obj.set(
            "uuidv4",
            Func::from(move || -> String {
                use uuid::Uuid;
                Uuid::new_v4().to_string()
            }),
        )?;

        // === JSON 函数 ===

        // jsonParse(str) - 解析 JSON 字符串为对象（返回 JSON 字符串供 JS 端解析）
        util_obj.set(
            "jsonParse",
            Func::from(move |s: String| -> String {
                // 验证是否为有效 JSON，如果有效直接返回
                if serde_json::from_str::<serde_json::Value>(&s).is_ok() {
                    s
                } else {
                    "null".to_string()
                }
            }),
        )?;

        // jsonStringify(value, indent) - 将任意 JS 值转换为 JSON 字符串（等价于 JSON.stringify）
        util_obj.set(
            "jsonStringify",
            Func::from(move |value: Value, indent: Option<i32>| -> String {
                to_json_text_with_indent(&js_value_to_json(&value), indent.unwrap_or(0))
            }),
        )?;

        // jsonPath(value, path) - 使用 JSONPath 查询对象/数组
        // 接受 JS 对象（推荐）或 JSON 字符串；返回匹配结果的 JSON 数组字符串
        util_obj.set(
            "jsonPath",
            Func::from(move |value: Value, path: String| -> String {
                use jsonpath_rust::JsonPath;
                // 字符串入参按 JSON 文本解析（保留旧调用方式），其他类型直接转换
                let subject = match value.as_string().and_then(|s| s.to_string().ok()) {
                    Some(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                        Ok(parsed) => parsed,
                        Err(_) => return "[]".to_string(),
                    },
                    None => js_value_to_json(&value),
                };
                match subject.query(&path) {
                    Ok(result) => {
                        // 将结果转换为 JSON 数组
                        let arr: Vec<&serde_json::Value> = result.into_iter().collect();
                        to_json_text(&serde_json::Value::Array(
                            arr.into_iter().cloned().collect(),
                        ))
                    }
                    Err(_) => "[]".to_string(),
                }
            }),
        )?;

        fn_obj.set("util", util_obj)?;
        Ok(())
    }

    /// 注册 `fn.expect(value)` API 到 `QuickJS` 上下文
    ///
    /// # 注册的方法
    /// - `fn.expect(value)` - 返回断言链对象
    ///
    /// # 链式修饰符
    /// - `.to`, `.be`, `.have` - 返回 self 用于链式调用
    /// - `.not` - 反转下一个断言结果
    ///
    /// # 断言方法
    /// - `.equal(expected)` / `.eql(expected)` - 严格相等
    /// - `.notEqual(expected)` - 不相等
    /// - `.exist()` / `.beDefined()` - 存在性检查
    /// - `.beNull()` / `.notBeNull()` - null 检查
    /// - `.beTrue()` / `.beFalse()` - 布尔值检查
    /// - `.beTruthy()` / `.beFalsy()` - 真值检查
    /// - `.contain(expected)` - 数组/字符串包含
    /// - `.containKey(key)` - 对象包含键
    /// - `.haveLength(n)` - 长度检查
    /// - `.match(regex)` - 正则匹配
    /// - `.startWith(prefix)` / `.endWith(suffix)` - 字符串首尾检查
    /// - `.above(n)` / `.below(n)` - 数值比较
    pub fn register_expect_api(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        // 使用 JavaScript 代码创建 expect 函数，避免 Rust 闭包生命周期问题
        let expect_code = r#"
            (function() {
                const results = [];
                
                function expect(value) {
                    const state = { 
                        _value: value, 
                        _negated: false,
                        _assertionName: ''
                    };
                    
                    // 记录断言结果的辅助函数
                    function recordAssertion(passed, message, expected, actual) {
                        results.push({
                            passed: passed,
                            message: message,
                            expected: expected,
                            actual: actual
                        });
                        return chain;
                    }
                    
                    // 深度比较函数
                    function deepEqual(a, b) {
                        if (a === b) return true;
                        if (a == null || b == null) return false;
                        if (typeof a !== typeof b) return false;
                        if (typeof a !== 'object') return a === b;
                        if (Array.isArray(a) !== Array.isArray(b)) return false;
                        const keysA = Object.keys(a);
                        const keysB = Object.keys(b);
                        if (keysA.length !== keysB.length) return false;
                        for (const key of keysA) {
                            if (!keysB.includes(key)) return false;
                            if (!deepEqual(a[key], b[key])) return false;
                        }
                        return true;
                    }
                    
                    const chain = {
                        // 链式修饰符
                        get to() { return chain; },
                        get be() { return chain; },
                        get have() { return chain; },
                        get not() { 
                            state._negated = !state._negated; 
                            return chain; 
                        },
                        
                        // 相等性断言
                        equal: function(expected) {
                            const passed = state._negated ? state._value !== expected : state._value === expected;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to equal ' + JSON.stringify(expected)
                                : 'expect ' + JSON.stringify(state._value) + ' to equal ' + JSON.stringify(expected);
                            state._negated = false;
                            return recordAssertion(passed, message, expected, state._value);
                        },
                        
                        eql: function(expected) {
                            const passed = state._negated 
                                ? !deepEqual(state._value, expected) 
                                : deepEqual(state._value, expected);
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to deeply equal ' + JSON.stringify(expected)
                                : 'expect ' + JSON.stringify(state._value) + ' to deeply equal ' + JSON.stringify(expected);
                            state._negated = false;
                            return recordAssertion(passed, message, expected, state._value);
                        },
                        
                        notEqual: function(expected) {
                            const passed = state._value !== expected;
                            const message = 'expect ' + JSON.stringify(state._value) + ' not to equal ' + JSON.stringify(expected);
                            return recordAssertion(passed, message, expected, state._value);
                        },
                        
                        // 存在性断言
                        exist: function() {
                            const passed = state._negated ? (state._value === null || state._value === undefined) : (state._value !== null && state._value !== undefined);
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to exist'
                                : 'expect ' + JSON.stringify(state._value) + ' to exist';
                            state._negated = false;
                            return recordAssertion(passed, message, null, state._value);
                        },
                        
                        beDefined: function() {
                            const passed = state._negated ? (state._value === undefined) : (state._value !== undefined);
                            const message = state._negated 
                                ? 'expect value not to be defined'
                                : 'expect value to be defined';
                            state._negated = false;
                            return recordAssertion(passed, message, null, state._value);
                        },
                        
                        beNull: function() {
                            const passed = state._negated ? (state._value !== null) : (state._value === null);
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to be null'
                                : 'expect ' + JSON.stringify(state._value) + ' to be null';
                            state._negated = false;
                            return recordAssertion(passed, message, null, state._value);
                        },
                        
                        notBeNull: function() {
                            const passed = state._value !== null;
                            const message = 'expect ' + JSON.stringify(state._value) + ' not to be null';
                            return recordAssertion(passed, message, null, state._value);
                        },
                        
                        // 布尔断言
                        beTrue: function() {
                            const passed = state._negated ? (state._value !== true) : (state._value === true);
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to be true'
                                : 'expect ' + JSON.stringify(state._value) + ' to be true';
                            state._negated = false;
                            return recordAssertion(passed, message, true, state._value);
                        },
                        
                        beFalse: function() {
                            const passed = state._negated ? (state._value !== false) : (state._value === false);
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to be false'
                                : 'expect ' + JSON.stringify(state._value) + ' to be false';
                            state._negated = false;
                            return recordAssertion(passed, message, false, state._value);
                        },
                        
                        beTruthy: function() {
                            const passed = state._negated ? !state._value : !!state._value;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to be truthy'
                                : 'expect ' + JSON.stringify(state._value) + ' to be truthy';
                            state._negated = false;
                            return recordAssertion(passed, message, null, state._value);
                        },
                        
                        beFalsy: function() {
                            const passed = state._negated ? !!state._value : !state._value;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to be falsy'
                                : 'expect ' + JSON.stringify(state._value) + ' to be falsy';
                            state._negated = false;
                            return recordAssertion(passed, message, null, state._value);
                        },
                        
                        // 包含断言
                        contain: function(expected) {
                            let passed = false;
                            if (typeof state._value === 'string') {
                                passed = state._value.includes(expected);
                            } else if (Array.isArray(state._value)) {
                                passed = state._value.some(item => deepEqual(item, expected));
                            }
                            if (state._negated) passed = !passed;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to contain ' + JSON.stringify(expected)
                                : 'expect ' + JSON.stringify(state._value) + ' to contain ' + JSON.stringify(expected);
                            state._negated = false;
                            return recordAssertion(passed, message, expected, state._value);
                        },
                        
                        containKey: function(key) {
                            let passed = false;
                            if (state._value !== null && typeof state._value === 'object') {
                                passed = Object.prototype.hasOwnProperty.call(state._value, key);
                            }
                            if (state._negated) passed = !passed;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to contain key "' + key + '"'
                                : 'expect ' + JSON.stringify(state._value) + ' to contain key "' + key + '"';
                            state._negated = false;
                            return recordAssertion(passed, message, key, state._value);
                        },
                        
                        // 长度断言
                        haveLength: function(n) {
                            let len = 0;
                            if (state._value !== null && state._value !== undefined) {
                                if (typeof state._value === 'string' || Array.isArray(state._value)) {
                                    len = state._value.length;
                                } else if (typeof state._value === 'object') {
                                    len = Object.keys(state._value).length;
                                }
                            }
                            const passed = state._negated ? (len !== n) : (len === n);
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to have length ' + n
                                : 'expect ' + JSON.stringify(state._value) + ' to have length ' + n;
                            state._negated = false;
                            return recordAssertion(passed, message, n, len);
                        },
                        
                        // 字符串断言
                        match: function(pattern) {
                            let passed = false;
                            if (typeof state._value === 'string') {
                                try {
                                    const regex = new RegExp(pattern);
                                    passed = regex.test(state._value);
                                } catch (e) {
                                    passed = false;
                                }
                            }
                            if (state._negated) passed = !passed;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to match ' + JSON.stringify(pattern)
                                : 'expect ' + JSON.stringify(state._value) + ' to match ' + JSON.stringify(pattern);
                            state._negated = false;
                            return recordAssertion(passed, message, pattern, state._value);
                        },
                        
                        startWith: function(prefix) {
                            let passed = false;
                            if (typeof state._value === 'string' && typeof prefix === 'string') {
                                passed = state._value.startsWith(prefix);
                            }
                            if (state._negated) passed = !passed;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to start with ' + JSON.stringify(prefix)
                                : 'expect ' + JSON.stringify(state._value) + ' to start with ' + JSON.stringify(prefix);
                            state._negated = false;
                            return recordAssertion(passed, message, prefix, state._value);
                        },
                        
                        endWith: function(suffix) {
                            let passed = false;
                            if (typeof state._value === 'string' && typeof suffix === 'string') {
                                passed = state._value.endsWith(suffix);
                            }
                            if (state._negated) passed = !passed;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to end with ' + JSON.stringify(suffix)
                                : 'expect ' + JSON.stringify(state._value) + ' to end with ' + JSON.stringify(suffix);
                            state._negated = false;
                            return recordAssertion(passed, message, suffix, state._value);
                        },
                        
                        // 数值断言
                        above: function(n) {
                            let passed = false;
                            if (typeof state._value === 'number') {
                                passed = state._value > n;
                            }
                            if (state._negated) passed = !passed;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to be above ' + n
                                : 'expect ' + JSON.stringify(state._value) + ' to be above ' + n;
                            state._negated = false;
                            return recordAssertion(passed, message, n, state._value);
                        },
                        
                        below: function(n) {
                            let passed = false;
                            if (typeof state._value === 'number') {
                                passed = state._value < n;
                            }
                            if (state._negated) passed = !passed;
                            const message = state._negated 
                                ? 'expect ' + JSON.stringify(state._value) + ' not to be below ' + n
                                : 'expect ' + JSON.stringify(state._value) + ' to be below ' + n;
                            state._negated = false;
                            return recordAssertion(passed, message, n, state._value);
                        }
                    };
                    
                    return chain;
                }
                
                // 获取测试结果的方法
                expect._getTestResults = function() {
                    return JSON.stringify(results);
                };
                
                // 清空测试结果的方法
                expect._clearTestResults = function() {
                    results.length = 0;
                };
                
                return expect;
            })()
        "#;

        // 执行 JavaScript 代码创建 expect 函数
        let expect_fn: Value = ctx.eval(expect_code)?;

        // 获取 expect 函数对象
        let expect_obj = expect_fn
            .clone()
            .into_object()
            .ok_or(rquickjs::Error::Unknown)?;

        // 获取或创建 fn 对象
        let fn_obj: Object = match ctx.globals().get("fn") {
            Ok(obj) => obj,
            Err(_) => {
                // fn 不存在，创建新的 fn 对象
                let new_fn_obj = Object::new(ctx.clone())?;
                ctx.globals().set("fn", new_fn_obj.clone())?;
                new_fn_obj
            }
        };
        fn_obj.set("expect", expect_obj)?;

        Ok(())
    }

    /// 获取测试断言结果
    ///
    /// 从 `QuickJS` 上下文中获取所有测试结果
    pub fn get_test_results(&self, ctx: &Ctx) -> Result<Vec<TestResult>, rquickjs::Error> {
        // 调用 JS 端的 _getTestResults 方法获取结果
        let results_json: String = ctx.eval("fn.expect._getTestResults()")?;

        // 解析 JSON 结果
        match serde_json::from_str::<Vec<TestResult>>(&results_json) {
            Ok(results) => Ok(results),
            Err(_) => Ok(Vec::new()),
        }
    }

    /// 注册所有 API 到 `QuickJS` 上下文
    ///
    /// 包括 fn.variables.*、fn.response.*、fn.request.*、fn.console.*、fn.flow.*、fn.info.*、fn.util.* 和 fn.expect
    pub fn register_all_apis(&self, ctx: &Ctx) -> Result<(), rquickjs::Error> {
        self.register_variables_api(ctx)?;
        self.register_response_api(ctx)?;
        self.register_request_api(ctx)?;
        self.register_console_api(ctx)?;
        self.register_flow_api(ctx)?;
        self.register_info_api(ctx)?;
        self.register_util_api(ctx)?;
        self.register_expect_api(ctx)?;
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
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('test', 'hello')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let result: String = ctx
                .eval("fn.variables.get('test')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(result, "hello");
        });
    }

    #[test]
    fn test_fn_variables_has() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('exists', 'value')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let has_key: bool = ctx
                .eval("fn.variables.has('exists')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(has_key);
            let not_has: bool = ctx
                .eval("fn.variables.has('nonexistent')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_has);
        });
    }

    #[test]
    fn test_fn_variables_unset() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('remove_me', 'value')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.unset('remove_me')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let result: Option<String> = ctx
                .eval("fn.variables.get('remove_me')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(result.is_none());
        });
    }

    #[test]
    fn test_fn_variables_clear() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('a', '1')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('b', '2')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.clear()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let has_a: bool = ctx
                .eval("fn.variables.has('a')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!has_a);
        });
    }

    #[test]
    fn test_fn_variables_set_all() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.setAll({ x: '10', y: '20' })")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let x: String = ctx
                .eval("fn.variables.get('x')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let y: String = ctx
                .eval("fn.variables.get('y')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(x, "10");
            assert_eq!(y, "20");
        });
    }

    #[test]
    fn test_fn_variables_all() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('a', '1')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('b', '2')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 获取所有变量并检查（返回 JSON 字符串，需要解析）
            let all_json: String = ctx
                .eval("fn.variables.all()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let all_obj: Object = ctx
                .eval(format!("JSON.parse('{}')", all_json).as_bytes())
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let a_val: String = all_obj.get("a").unwrap_or_else(|e| panic!("{e}"));
            let b_val: String = all_obj.get("b").unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(a_val, "1");
            assert_eq!(b_val, "2");
        });
    }

    #[test]
    fn test_fn_variables_overwrite() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('key', 'first')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('key', 'second')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let result: String = ctx
                .eval("fn.variables.get('key')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
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
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

        let raw_output = serde_json::json!({
            "status": 200,
            "body": {"token": "abc123"},
            "headers": {"Content-Type": "application/json"}
        });

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
            raw_output: raw_output.clone(),
        };

        let script_ctx = ScriptExecutionContext::new().with_response(response);

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let code: u16 = ctx
                .eval("fn.response.code")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(code, 200);

            let status: String = ctx
                .eval("fn.response.status")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(status, "OK");

            let content_type: String = ctx
                .eval("fn.response.headers['Content-Type']")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(content_type, "application/json");

            let size: usize = ctx
                .eval("fn.response.size")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(size, 19);

            let time: u64 = ctx
                .eval("fn.response.time")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(time, 150);

            // 测试 text() 函数
            let text: String = ctx
                .eval("fn.response.text()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(text, r#"{"token":"abc123"}"#);

            // 测试 json() 函数
            let json: String = ctx
                .eval("fn.response.json()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(json.contains("token"));

            // 测试 raw 属性 - 完整原始输出
            let raw_str: String = ctx
                .eval("fn.response.raw")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let raw_parsed: serde_json::Value =
                serde_json::from_str(&raw_str).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(raw_parsed["status"], 200);
            assert_eq!(raw_parsed["body"]["token"], "abc123");
        });
    }

    #[test]
    fn test_fn_response_null_when_no_response() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

        let script_ctx = ScriptExecutionContext::new(); // response = None

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 访问不存在的 response 应该返回 undefined
            let code: Option<u16> = ctx
                .eval("fn.response?.code")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(code.is_none());
        });
    }

    #[test]
    fn test_fn_response_body_parsing() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

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
            raw_output: serde_json::json!({"user":{"id":1,"name":"test"},"items":[1,2,3]}),
        };

        let script_ctx = ScriptExecutionContext::new().with_response(response);

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // body 返回 JSON 字符串，可以用 JSON.parse 解析
            let body_str: String = ctx
                .eval("fn.response.body")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let parsed: Object = ctx
                .eval(format!("JSON.parse('{}')", body_str).as_bytes())
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let user: Object = parsed.get("user").unwrap_or_else(|e| panic!("{e}"));
            let id: i32 = user.get("id").unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(id, 1);
        });
    }

    #[test]
    fn test_fn_response_raw_with_non_http_output() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

        // 模拟数据库查询输出（非 HTTP 格式）
        let raw_output = serde_json::json!({
            "rows": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ],
            "affected": 2,
            "duration_ms": 15
        });

        let response = ResponseData {
            code: 0,
            status: String::new(),
            headers: HashMap::new(),
            body: raw_output.clone(),
            raw_body: raw_output.to_string(),
            size: raw_output.to_string().len(),
            time: 15,
            raw_output: raw_output.clone(),
        };

        let script_ctx = ScriptExecutionContext::new().with_response(response);

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // HTTP 专属字段应该为空或 0
            let code: u16 = ctx
                .eval("fn.response.code")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(code, 0);
            let status: String = ctx
                .eval("fn.response.status")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(status, "");

            // 通过 raw 访问完整输出
            let raw_str: String = ctx
                .eval("fn.response.raw")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let raw_parsed: serde_json::Value =
                serde_json::from_str(&raw_str).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(raw_parsed["rows"][0]["name"], "Alice");
            assert_eq!(raw_parsed["affected"], 2);

            // 验证可以 JSON.parse + 遍历
            let names: String = ctx
                .eval(
                    r#"
                    var data = JSON.parse(fn.response.raw);
                    JSON.stringify(data.rows.map(function(r) { return r.name; }))
                "#,
                )
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(names, r#"["Alice","Bob"]"#);
        });
    }

    #[test]
    fn test_fn_request_with_data() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

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
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let url: String = ctx
                .eval("fn.request.url")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(url, "https://api.example.com/users");

            let method: String = ctx
                .eval("fn.request.method")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(method, "POST");

            let content_type: String = ctx
                .eval("fn.request.headers['Content-Type']")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(content_type, "application/json");
        });
    }

    #[test]
    fn test_fn_request_null_when_no_request() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

        let script_ctx = ScriptExecutionContext::new(); // request = None

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 当没有请求数据时，fn.request 对象存在但属性为 undefined
            // 使用可选链访问会返回 undefined (在 rquickjs 中转换为 null)
            let result: Option<String> = ctx
                .eval("fn.request?.url")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(result.is_none());
        });
    }

    #[test]
    fn test_fn_console_log() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.console.log('hello world')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let logs = script_ctx.logs.borrow();
            assert_eq!(logs.len(), 1);
            assert_eq!(logs[0].level, "log");
            assert_eq!(logs[0].message, "hello world");
        });
    }

    #[test]
    fn test_fn_console_levels() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.console.log('log msg')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.console.info('info msg')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.console.warn('warn msg')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.console.error('error msg')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.console.debug('debug msg')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.console.table('table data')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let logs = script_ctx.logs.borrow();
            assert_eq!(logs.len(), 6);
            assert_eq!(logs[0].level, "log");
            assert_eq!(logs[1].level, "info");
            assert_eq!(logs[2].level, "warn");
            assert_eq!(logs[3].level, "error");
            assert_eq!(logs[4].level, "debug");
            assert_eq!(logs[5].level, "table");
        });
    }

    #[test]
    fn test_fn_flow_skip() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!*script_ctx.should_skip.borrow()); // 初始值应为 false

            let _: () = ctx
                .eval("fn.flow.skip()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(*script_ctx.should_skip.borrow()); // 调用后应为 true
        });
    }

    #[test]
    fn test_fn_flow_abort() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!*script_ctx.should_abort.borrow());

            let _: () = ctx
                .eval("fn.flow.abort('Something went wrong')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            assert!(*script_ctx.should_abort.borrow());
            assert_eq!(
                *script_ctx.abort_message.borrow(),
                Some("Something went wrong".to_string())
            );
        });
    }

    #[test]
    fn test_fn_flow_abort_without_message() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 真正的无参调用（文档写的是 abort(msg?)）：底层 Rust 参数是 String，
            // 没有 JS 包装时会抛 "expected string, got undefined"
            script_ctx.should_abort.replace(false);
            script_ctx.abort_message.replace(None);
            let _: () = ctx
                .eval("fn.flow.abort()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(*script_ctx.should_abort.borrow());
            assert_eq!(*script_ctx.abort_message.borrow(), None);

            // 传入 undefined 同样按无消息处理
            script_ctx.should_abort.replace(false);
            let _: () = ctx
                .eval("fn.flow.abort(undefined)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(*script_ctx.should_abort.borrow());
            assert_eq!(*script_ctx.abort_message.borrow(), None);

            // 传入空字符串表示无消息
            script_ctx.should_abort.replace(false);
            let _: () = ctx
                .eval("fn.flow.abort('')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            assert!(*script_ctx.should_abort.borrow());
            assert_eq!(*script_ctx.abort_message.borrow(), None);
        });
    }

    #[test]
    fn test_fn_flow_stop() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!*script_ctx.should_stop.borrow());

            // 测试无参调用
            script_ctx.should_stop.replace(false);
            script_ctx.stop_message.replace(None);
            let _: () = ctx
                .eval("fn.flow.stop()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(*script_ctx.should_stop.borrow());
            assert_eq!(*script_ctx.stop_message.borrow(), None);

            // 测试传空字符串
            script_ctx.should_stop.replace(false);
            script_ctx.stop_message.replace(None);
            let _: () = ctx
                .eval("fn.flow.stop('')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(*script_ctx.should_stop.borrow());
            assert_eq!(*script_ctx.stop_message.borrow(), None);

            // 测试带消息的 stop
            script_ctx.should_stop.replace(false);
            script_ctx.stop_message.replace(None);
            let _: () = ctx
                .eval("fn.flow.stop('自定义终止消息')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(*script_ctx.should_stop.borrow());
            assert_eq!(
                *script_ctx.stop_message.borrow(),
                Some("自定义终止消息".to_string())
            );
        });
    }

    #[test]
    fn test_fn_info_basic() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

        let script_ctx = ScriptExecutionContext::new()
            .with_flow_info("Test Flow".to_string(), "flow-123".to_string())
            .with_node_info("node-456".to_string(), "Script Node".to_string());

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let flow_name: String = ctx
                .eval("fn.info.flowName")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(flow_name, "Test Flow");

            let flow_id: String = ctx
                .eval("fn.info.flowId")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(flow_id, "flow-123");

            let node_id: String = ctx
                .eval("fn.info.nodeId")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(node_id, "node-456");

            let node_name: String = ctx
                .eval("fn.info.nodeName")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(node_name, "Script Node");
        });
    }

    #[test]
    fn test_fn_info_none() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));

        let script_ctx = ScriptExecutionContext::new(); // No flow/node info

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 当没有设置信息时，属性应该为 undefined
            let flow_name: Option<String> = ctx
                .eval("fn.info?.flowName")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(flow_name.is_none());

            let flow_id: Option<String> = ctx
                .eval("fn.info?.flowId")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(flow_id.is_none());
        });
    }

    #[test]
    fn test_with_flow_info_builder() {
        let ctx = ScriptExecutionContext::new()
            .with_flow_info("My Flow".to_string(), "id-001".to_string());

        assert_eq!(ctx.flow_name, Some("My Flow".to_string()));
        assert_eq!(ctx.flow_id, Some("id-001".to_string()));
    }

    #[test]
    fn test_with_node_info_builder() {
        let ctx = ScriptExecutionContext::new()
            .with_node_info("node-123".to_string(), "Test Node".to_string());

        assert_eq!(ctx.node_id, Some("node-123".to_string()));
        assert_eq!(ctx.node_name, Some("Test Node".to_string()));
    }

    #[test]
    fn test_fn_util_base64_encode_decode() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试编码
            let encoded: String = ctx
                .eval("fn.util.base64Encode('Hello, World!')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");

            // 测试解码
            let decoded: String = ctx
                .eval("fn.util.base64Decode('SGVsbG8sIFdvcmxkIQ==')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(decoded, "Hello, World!");

            // 测试往返
            let roundtrip: String = ctx
                .eval("fn.util.base64Decode(fn.util.base64Encode('测试中文'))")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(roundtrip, "测试中文");
        });
    }

    #[test]
    fn test_fn_util_url_encode_decode() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 URL 编码
            let encoded: String = ctx
                .eval("fn.util.urlEncode('hello world&key=value')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(encoded, "hello%20world%26key%3Dvalue");

            // 测试 URL 解码
            let decoded: String = ctx
                .eval("fn.util.urlDecode('hello%20world%26key%3Dvalue')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(decoded, "hello world&key=value");

            // 测试往返
            let roundtrip: String = ctx
                .eval("fn.util.urlDecode(fn.util.urlEncode('测试/路径?查询=值'))")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(roundtrip, "测试/路径?查询=值");
        });
    }

    #[test]
    fn test_fn_util_html_escape_unescape() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 HTML 转义
            let escaped: String = ctx
                .eval("fn.util.htmlEscape('<script>alert(\"XSS\")</script>')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                escaped,
                "&lt;script&gt;alert(&quot;XSS&quot;)&lt;/script&gt;"
            );

            // 测试 HTML 反转义
            let unescaped: String = ctx
                .eval("fn.util.htmlUnescape('&lt;div&gt;Hello&amp;World&lt;/div&gt;')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(unescaped, "<div>Hello&World</div>");

            // 测试往返
            let roundtrip: String = ctx
                .eval("fn.util.htmlUnescape(fn.util.htmlEscape('<a href=\"test\">link</a>'))")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(roundtrip, "<a href=\"test\">link</a>");
        });
    }

    #[test]
    fn test_fn_util_md5() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let hash: String = ctx
                .eval("fn.util.md5('test')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(hash, "098f6bcd4621d373cade4e832627b4f6");
        });
    }

    #[test]
    fn test_fn_util_sha256() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let hash: String = ctx
                .eval("fn.util.sha256('test')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        });
    }

    #[test]
    fn test_fn_util_sha512() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let hash: String = ctx
                .eval("fn.util.sha512('test')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(hash.len(), 128); // SHA-512 = 64 bytes = 128 hex chars
        });
    }

    #[test]
    fn test_fn_util_hmac_sha256() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let hash: String = ctx
                .eval("fn.util.hmacSha256('key', 'data')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!hash.is_empty());
            assert_eq!(hash.len(), 64); // SHA256 = 32 bytes = 64 hex chars
        });
    }

    #[test]
    fn test_fn_util_hmac_sha512() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let hash: String = ctx
                .eval("fn.util.hmacSha512('key', 'data')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!hash.is_empty());
            assert_eq!(hash.len(), 128); // SHA512 = 64 bytes = 128 hex chars
        });
    }

    #[test]
    fn test_fn_util_type_checks() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 isString
            let is_str: bool = ctx
                .eval("fn.util.isString('hello')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_str);
            let not_str: bool = ctx
                .eval("fn.util.isString(123)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_str);

            // 测试 isNumber
            let is_num: bool = ctx
                .eval("fn.util.isNumber(123)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_num);
            let not_num: bool = ctx
                .eval("fn.util.isNumber('123')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_num);

            // 测试 isBoolean
            let is_bool: bool = ctx
                .eval("fn.util.isBoolean(true)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_bool);
            let not_bool: bool = ctx
                .eval("fn.util.isBoolean(1)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_bool);

            // 测试 isNull
            let is_null: bool = ctx
                .eval("fn.util.isNull(null)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_null);
            let not_null: bool = ctx
                .eval("fn.util.isNull(undefined)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_null);

            // 测试 isUndefined
            let is_undef: bool = ctx
                .eval("fn.util.isUndefined(undefined)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_undef);
            let not_undef: bool = ctx
                .eval("fn.util.isUndefined(null)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_undef);

            // 测试 isArray
            let is_arr: bool = ctx
                .eval("fn.util.isArray([1,2,3])")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_arr);
            let not_arr: bool = ctx
                .eval("fn.util.isArray({})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_arr);

            // 测试 isObject
            let is_obj: bool = ctx
                .eval("fn.util.isObject({a:1})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_obj);
            let not_obj: bool = ctx
                .eval("fn.util.isObject([1,2])")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_obj);

            // 测试 typeOf
            let type_str: String = ctx
                .eval("fn.util.typeOf('test')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(type_str, "string");
            let type_num: String = ctx
                .eval("fn.util.typeOf(42)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(type_num, "number");
            let type_arr: String = ctx
                .eval("fn.util.typeOf([])")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(type_arr, "array");
            let type_obj: String = ctx
                .eval("fn.util.typeOf({})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(type_obj, "object");
        });
    }

    #[test]
    fn test_fn_util_is_empty() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 null 和 undefined
            let is_empty_null: bool = ctx
                .eval("fn.util.isEmpty(null)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_empty_null);
            let is_empty_undef: bool = ctx
                .eval("fn.util.isEmpty(undefined)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_empty_undef);

            // 测试空字符串
            let is_empty_str: bool = ctx
                .eval("fn.util.isEmpty('')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_empty_str);
            let not_empty_str: bool = ctx
                .eval("fn.util.isEmpty('test')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_empty_str);

            // 测试空数组
            let is_empty_arr: bool = ctx
                .eval("fn.util.isEmpty([])")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_empty_arr);

            // 测试空对象
            let is_empty_obj: bool = ctx
                .eval("fn.util.isEmpty({})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(is_empty_obj);
        });
    }

    #[test]
    fn test_fn_util_object_keys() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let keys_json: String = ctx
                .eval("fn.util.keys({b: '2', a: '1'})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(keys_json, r#"["b","a"]"#); // 保持 JS 的插入顺序

            // 数组按索引取键
            let array_keys: String = ctx
                .eval("fn.util.keys([7, 8])")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(array_keys, r#"["0","1"]"#);

            let scalar_keys: String = ctx
                .eval("fn.util.keys('nope')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(scalar_keys, "[]");
        });
    }

    #[test]
    fn test_fn_util_has_key() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let has_a: bool = ctx
                .eval("fn.util.hasKey({a: 1, b: '2'}, 'a')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(has_a);
            let has_c: bool = ctx
                .eval("fn.util.hasKey({a: 1, b: '2'}, 'c')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!has_c);
        });
    }

    #[test]
    fn test_fn_util_object_values() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let values_json: String = ctx
                .eval("fn.util.values({b: 'y', a: 'x'})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(values_json, r#"["y","x"]"#);
        });
    }

    #[test]
    fn test_fn_util_object_entries() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let entries_json: String = ctx
                .eval("fn.util.entries({a: '1'})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(entries_json, r#"[["a","1"]]"#);
        });
    }

    #[test]
    fn test_fn_util_pick_omit() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 pick（原生数组入参 + 非字符串值）
            let picked: String = ctx
                .eval("fn.util.pick({b: '2', a: 1, c: true}, ['a', 'b'])")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(picked, r#"{"a":1,"b":"2"}"#);

            // 兼容旧的 JSON 字符串写法
            let picked_legacy: String = ctx
                .eval(r#"fn.util.pick({a: '1', b: '2', c: '3'}, '["a", "b"]')"#)
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(picked_legacy, r#"{"a":"1","b":"2"}"#);

            // 测试 omit（保留非字符串值）
            let omitted: String = ctx
                .eval("fn.util.omit({a: 1, b: '2', c: true}, ['c'])")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(omitted, r#"{"a":1,"b":"2"}"#);
        });
    }

    #[test]
    fn test_fn_util_merge_clone() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 merge（浅合并 + 嵌套值原样保留）
            let merged: String = ctx
                .eval("fn.util.merge({a: 1, n: {x: 1}}, {b: true, n: {y: 2}})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(merged, r#"{"a":1,"n":{"y":2},"b":true}"#);

            // 测试 clone（深拷贝任意值）
            let cloned: String = ctx
                .eval("fn.util.clone({a: [1, '2'], x: 'test'})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(cloned, r#"{"a":[1,"2"],"x":"test"}"#);
        });
    }

    /// 稀疏数组的 length 不能变成宿主的多 GB 预分配（`arr.length = 1e9` 是 DoS 入口）
    #[test]
    fn test_fn_util_sparse_array_is_bounded() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 稀疏数组（只有 length，无元素）在旧实现下会触发 Vec::with_capacity(1e9)
            let json: String = ctx
                .eval("const a = []; a.length = 1000000000; fn.util.jsonStringify(a, 0)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(json, "null");

            // 正常数组不受影响，空洞按 null 补齐（与 JSON.stringify 一致）
            let holes: String = ctx
                .eval("const b = ['x']; b.length = 3; fn.util.jsonStringify(b, 0)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(holes, r#"["x",null,null]"#);
        });
    }

    /// hasKey 必须看 JS 自有键，而不是 JSON 转换结果（后者会丢弃 undefined / 函数属性）
    #[test]
    fn test_fn_util_has_key_keeps_undefined_and_function_keys() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let has_undefined: bool = ctx
                .eval("fn.util.hasKey({a: undefined}, 'a')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(has_undefined);

            let has_function: bool = ctx
                .eval("fn.util.hasKey({fn: () => 1}, 'fn')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(has_function);

            let missing: bool = ctx
                .eval("fn.util.hasKey({a: 1}, 'b')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!missing);

            let not_object: bool = ctx
                .eval("fn.util.hasKey('nope', 'length')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!not_object);
        });
    }

    // ========== 时间函数测试 ==========

    #[test]
    fn test_fn_util_timestamp() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let ts: i64 = ctx
                .eval("fn.util.timestamp()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 时间戳应该是当前时间的毫秒数（约 1.7 万亿）
            assert!(ts > 1700000000000);
            // 应该是 13 位数字
            assert!(ts > 0);
        });
    }

    #[test]
    fn test_fn_util_now() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let now_str: String = ctx
                .eval("fn.util.now()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 应该返回 ISO 8601 格式，例如 "2026-03-12T10:30:00+00:00"
            assert!(now_str.contains('T'));
            assert!(now_str.contains('-'));
            assert!(now_str.contains(':'));
        });
    }

    #[test]
    fn test_fn_util_format_time() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试格式化
            let formatted: String = ctx
                .eval("fn.util.formatTime('%Y-%m-%d %H:%M:%S')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 应该包含日期时间格式的字符串
            assert!(formatted.contains('-'));
            assert!(formatted.contains(':'));
            // 检查长度（格式：YYYY-MM-DD HH:MM:SS = 19 字符）
            assert_eq!(formatted.len(), 19);
        });
    }

    #[test]
    fn test_fn_util_parse_time() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试解析 ISO 8601 格式
            let ts: String = ctx
                .eval("fn.util.parseTime('2026-03-12T10:30:00Z', '')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 应该返回时间戳
            assert!(!ts.is_empty());
            let ts_num: i64 = ts.parse().unwrap_or_else(|e| panic!("{e}"));
            assert!(ts_num > 0);

            // 测试解析自定义格式
            let ts2: String = ctx
                .eval("fn.util.parseTime('2026-03-12 10:30:00', '%Y-%m-%d %H:%M:%S')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(!ts2.is_empty());
        });
    }

    // ========== UUID 函数测试 ==========

    #[test]
    fn test_fn_util_uuid() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let uuid: String = ctx
                .eval("fn.util.uuid()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // UUID v4 格式: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
            assert_eq!(uuid.len(), 36);
            assert!(uuid.contains('-'));
            // 第 14 个字符应该是 '4' (UUID v4)
            let chars: Vec<char> = uuid.chars().collect();
            assert_eq!(chars[14], '4');
        });
    }

    #[test]
    fn test_fn_util_uuidv4() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let uuid1: String = ctx
                .eval("fn.util.uuidv4()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let uuid2: String = ctx
                .eval("fn.util.uuidv4()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 两个 UUID 应该不同
            assert_ne!(uuid1, uuid2);
            // 都应该是有效的 UUID 格式
            assert_eq!(uuid1.len(), 36);
            assert_eq!(uuid2.len(), 36);
        });
    }

    // ========== JSON 函数测试 ==========

    #[test]
    fn test_fn_util_json_parse() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试解析有效 JSON
            let parsed: String = ctx
                .eval("fn.util.jsonParse('{\"name\":\"test\",\"value\":123}')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(parsed.contains("name"));
            assert!(parsed.contains("test"));

            // 测试解析无效 JSON
            let invalid: String = ctx
                .eval("fn.util.jsonParse('not valid json')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(invalid, "null");
        });
    }

    #[test]
    fn test_fn_util_json_stringify() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 混合类型：数字/布尔/null/数组/嵌套对象原样保留，字符串不被二次解析成 JSON
            let json: String = ctx
                .eval(
                    "fn.util.jsonStringify({n: 1, f: 1.5, s: '123', b: true, nil: null, arr: [1, '2'], obj: {k: 'v'}, fn: () => 1, und: undefined}, 0)",
                )
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                json,
                r#"{"n":1,"f":1.5,"s":"123","b":true,"nil":null,"arr":[1,"2"],"obj":{"k":"v"}}"#
            );

            // 带缩进
            let pretty: String = ctx
                .eval("fn.util.jsonStringify({x: 'test'}, 2)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(pretty, "{\n  \"x\": \"test\"\n}");
        });
    }

    /// 循环引用必须被深度上限截断，而不是递归爆栈打死宿主进程
    #[test]
    fn test_fn_util_json_stringify_cycle_is_bounded() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let json: String = ctx
                .eval("const a = {x: 1}; a.self = a; fn.util.jsonStringify(a, 0)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 仍是合法 JSON（末尾被 null 截断），且确实递归了若干层
            let parsed: serde_json::Value =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("{json} -> {e}"));
            assert_eq!(parsed["x"], serde_json::json!(1));
            assert!(json.contains("\"self\""));
            assert!(json.len() > 50);
        });
    }

    /// 变量池必须保留 JSON 类型：原先 set/setAll 只接受字符串，数字/布尔/对象被丢弃
    #[test]
    fn test_fn_variables_keep_json_types() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_all_apis(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let all: String = ctx
                .eval(
                    "fn.variables.set('n', 5);\nfn.variables.set('o', {a: [1, 2]});\nfn.variables.setAll({b: true, c: 'x'});\nfn.variables.all()",
                )
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // `fn.variables.all()` 返回 JSON 文本（底层变量池是 HashMap，键序不稳定）
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&all)
                    .unwrap_or_else(|e| panic!("{all} -> {e}")),
                serde_json::json!({"n": 5, "o": {"a": [1, 2]}, "b": true, "c": "x"})
            );

            let variables = script_ctx.variables.borrow().clone();
            assert_eq!(variables.get("n"), Some(&serde_json::json!(5)));
            assert_eq!(
                variables.get("o"),
                Some(&serde_json::json!({"a": [1, 2]}))
            );
        });
    }

    #[test]
    fn test_fn_util_json_path() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx.register_all_apis(&ctx).unwrap_or_else(|e| panic!("{e}"));

            // 测试简单路径查询
            let result: String = ctx
                .eval("fn.util.jsonPath('{\"store\":{\"book\":[{\"title\":\"Title1\"},{\"title\":\"Title2\"}]}}', '$.store.book[*].title')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 应该返回标题数组
            assert!(result.contains("Title1") || result.contains("Title2") || result.contains("title"));

            // 测试根路径
            let root: String = ctx
                .eval("fn.util.jsonPath('{\"a\":1}', '$')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(root, r#"[{"a":1}]"#);

            // 直接传 JS 对象（不再需要手工拼 JSON 字符串）
            let from_object: String = ctx
                .eval("fn.util.jsonPath({items: [{id: 7}, {id: 8}]}, '$.items[*].id')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(from_object, "[7,8]");

            // 非法 JSON 文本返回空数组而不是抛错
            let invalid: String = ctx
                .eval("fn.util.jsonPath('not json', '$')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(invalid, "[]");
        });
    }

    // ==================== fn.expect 测试 ====================

    #[test]
    fn test_fn_expect_equal() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试相等情况
            let _: () = ctx
                .eval("fn.expect(5).to.equal(5)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 获取测试结果
            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 1);
            assert!(results[0].passed);
        });
    }

    #[test]
    fn test_fn_expect_equal_with_not() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 not.equal
            let _: () = ctx
                .eval("fn.expect(5).to.not.equal(3)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 1);
            assert!(results[0].passed);
        });
    }

    #[test]
    fn test_fn_expect_eql_deep_equal() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试深度相等
            let _: () = ctx
                .eval("fn.expect({a: 1, b: 2}).to.eql({a: 1, b: 2})")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 1);
            assert!(results[0].passed);
        });
    }

    #[test]
    fn test_fn_expect_chain_modifiers() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试链式调用 to.be.have
            let _: () = ctx
                .eval("fn.expect('hello').to.be.have.equal('hello')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 1);
            assert!(results[0].passed);
        });
    }

    #[test]
    fn test_fn_expect_boolean_assertions() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 beTrue
            let _: () = ctx
                .eval("fn.expect(true).to.beTrue()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 测试 beFalse
            let _: () = ctx
                .eval("fn.expect(false).to.beFalse()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 测试 beTruthy
            let _: () = ctx
                .eval("fn.expect(1).to.beTruthy()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 测试 beFalsy
            let _: () = ctx
                .eval("fn.expect(0).to.beFalsy()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 4);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_expect_null_assertions() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 beNull
            let _: () = ctx
                .eval("fn.expect(null).to.beNull()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 测试 notBeNull
            let _: () = ctx
                .eval("fn.expect('value').to.notBeNull()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 2);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_expect_contain() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试字符串包含
            let _: () = ctx
                .eval("fn.expect('hello world').to.contain('world')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试数组包含
            let _: () = ctx
                .eval("fn.expect([1, 2, 3]).to.contain(2)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 2);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_expect_contain_key() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试对象包含键
            let _: () = ctx
                .eval("fn.expect({name: 'test', age: 25}).to.containKey('name')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 1);
            assert!(results[0].passed);
        });
    }

    #[test]
    fn test_fn_expect_have_length() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试字符串长度
            let _: () = ctx
                .eval("fn.expect('hello').to.haveLength(5)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试数组长度
            let _: () = ctx
                .eval("fn.expect([1, 2, 3]).to.haveLength(3)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 2);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_expect_string_assertions() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试正则匹配
            let _: () = ctx
                .eval("fn.expect('hello123').to.match('[0-9]+')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 startWith
            let _: () = ctx
                .eval("fn.expect('hello world').to.startWith('hello')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 endWith
            let _: () = ctx
                .eval("fn.expect('hello world').to.endWith('world')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 3);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_expect_numeric_assertions() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 above
            let _: () = ctx
                .eval("fn.expect(10).to.be.above(5)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 below
            let _: () = ctx
                .eval("fn.expect(3).to.be.below(10)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 2);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_expect_failed_assertion() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试Failed的断言
            let _: () = ctx
                .eval("fn.expect(5).to.equal(10)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 1);
            assert!(!results[0].passed);
            assert!(results[0].message.contains("5"));
            assert!(results[0].message.contains("10"));
        });
    }

    #[test]
    fn test_fn_expect_exist_defined() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 测试 exist
            let _: () = ctx
                .eval("fn.expect('value').to.exist()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 测试 beDefined
            let _: () = ctx
                .eval("fn.expect('value').to.beDefined()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            // 测试 not exist (null)
            let _: () = ctx
                .eval("fn.expect(null).to.not.exist()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 3);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_expect_multiple_assertions() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_expect_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            // 多个断言
            let _: () = ctx
                .eval("fn.expect(200).to.equal(200)")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.expect('success').to.contain('succ')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.expect(true).to.beTrue()")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));

            let results = script_ctx
                .get_test_results(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(results.len(), 3);
            assert!(results.iter().all(|r| r.passed));
        });
    }

    #[test]
    fn test_fn_variables_get_dynamic_guid() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let guid: String = ctx
                .eval("fn.variables.get('$guid')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(guid.len(), 36);
            assert!(uuid::Uuid::parse_str(&guid).is_ok());

            let has_guid: bool = ctx
                .eval("fn.variables.has('$guid')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(has_guid);
        });
    }

    #[test]
    fn test_fn_variables_replace_in() {
        let rt = Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        let ctx = Context::full(&rt).unwrap_or_else(|e| panic!("{e}"));
        let script_ctx = ScriptExecutionContext::new();

        ctx.with(|ctx| {
            script_ctx
                .register_variables_api(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let _: () = ctx
                .eval("fn.variables.set('token', 'secret123')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            let res: String = ctx
                .eval("fn.variables.replaceIn('Bearer {{token}}')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(res, "Bearer secret123");

            let dynamic_res: String = ctx
                .eval("fn.variables.replaceIn('ID: {{$guid}}')")
                .catch(&ctx)
                .unwrap_or_else(|e| panic!("{e}"));
            assert!(dynamic_res.starts_with("ID: "));
            assert_eq!(dynamic_res.len(), 4 + 36);
        });
    }
}
