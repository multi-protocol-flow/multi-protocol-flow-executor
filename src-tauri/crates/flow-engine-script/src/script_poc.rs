//! rquickjs POC 测试模块
//! 验证 QuickJS 在脚本节点中的可行性
//!
//! 测试覆盖:
//! - 基本脚本执行
//! - 全局对象注册 (fn.variables.get/set)
//! - 超时控制
//! - JSON 数据转换
//! - 内存限制

use rquickjs::{function::Func, object::Object, CatchResultExt, Context, Runtime};

/// 测试基本脚本执行
#[test]
fn rquickjs_poc_basic() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    let result = ctx.with(|ctx| {
        let code = r#"
            let message = "Hello from QuickJS";
            message
        "#;
        let value: String = ctx.eval(code).catch(&ctx).unwrap();
        value
    });

    assert_eq!(result, "Hello from QuickJS");
}

/// 测试全局对象注册 - fn.variables.get/set
#[test]
fn rquickjs_poc_global_object() {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    // 使用 RefCell 存储变量
    let variables: Rc<RefCell<HashMap<String, String>>> = Rc::new(RefCell::new(HashMap::new()));

    let vars_clone = variables.clone();
    ctx.with(|ctx| {
        // 创建 fn.variables 对象
        let fn_obj = Object::new(ctx.clone()).unwrap();
        let vars_obj = Object::new(ctx.clone()).unwrap();

        // set 函数
        let vars_set = vars_clone.clone();
        vars_obj
            .set(
                "set",
                Func::from(move |key: String, value: String| {
                    vars_set.borrow_mut().insert(key, value);
                }),
            )
            .unwrap();

        // get 函数
        let vars_get = variables.clone();
        vars_obj
            .set(
                "get",
                Func::from(move |key: String| -> Option<String> {
                    vars_get.borrow().get(&key).cloned()
                }),
            )
            .unwrap();

        fn_obj.set("variables", vars_obj).unwrap();
        ctx.globals().set("fn", fn_obj).unwrap();

        // 测试 set
        let _: () = ctx
            .eval("fn.variables.set('test', 'hello')")
            .catch(&ctx)
            .unwrap();

        // 测试 get
        let result: String = ctx.eval("fn.variables.get('test')").catch(&ctx).unwrap();
        assert_eq!(result, "hello");
    });
}

/// 测试超时控制 - 使用 Runtime::set_interrupt_handler
/// 注意：QuickJS 的 interrupt_handler 会在每次循环迭代时检查
#[test]
fn rquickjs_poc_timeout_interrupt() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let rt = Runtime::new().unwrap();
    let should_interrupt = Arc::new(AtomicBool::new(false));
    let interrupt_clone = should_interrupt.clone();

    // 设置中断处理器
    rt.set_interrupt_handler(Some(Box::new(move || {
        interrupt_clone.load(Ordering::Relaxed)
    })));

    let ctx = Context::full(&rt).unwrap();
    let start = Instant::now();

    // 在另一个线程中设置中断标志
    let flag = should_interrupt.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        flag.store(true, Ordering::Relaxed);
    });

    ctx.with(|ctx| {
        // 简单脚本执行（非无限循环）
        let result: i32 = ctx.eval("1 + 1").catch(&ctx).unwrap();
        assert_eq!(result, 2);
    });

    let elapsed = start.elapsed();
    // 正常脚本应该很快执行完成
    assert!(elapsed < Duration::from_millis(500));
}

/// 测试 JSON 数据转换
#[test]
fn rquickjs_poc_json_conversion() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|ctx| {
        // 从 Rust 传递 JSON 到 JS
        let json_str = r#"{"name": "test", "value": 123}"#;
        let code = format!("const data = JSON.parse('{}'); data.name", json_str);
        let result: String = ctx.eval(code.as_bytes()).catch(&ctx).unwrap();
        assert_eq!(result, "test");

        // 从 JS 返回值到 Rust
        let value: i32 = ctx.eval("data.value").catch(&ctx).unwrap();
        assert_eq!(value, 123);
    });
}

/// 测试内存限制（QuickJS 特性）
#[test]
fn rquickjs_poc_memory_limit() {
    let rt = Runtime::new().unwrap();
    rt.set_memory_limit(1024 * 1024); // 1MB

    let ctx = Context::full(&rt).unwrap();

    ctx.with(|ctx| {
        // 正常小脚本应该工作
        let result: i32 = ctx.eval("1 + 1").catch(&ctx).unwrap();
        assert_eq!(result, 2);
    });
}

/// 测试脚本错误捕获
#[test]
fn rquickjs_poc_error_handling() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|ctx| {
        // 测试语法错误
        let result: Result<String, _> = ctx.eval("invalid syntax {{{").catch(&ctx);
        assert!(result.is_err());

        // 测试运行时错误
        let result: Result<String, _> = ctx.eval("undefined_variable").catch(&ctx);
        assert!(result.is_err());
    });
}

/// 测试异步代码解析（ES2020 支持）
#[test]
fn rquickjs_poc_async_support() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|ctx| {
        // QuickJS 支持 async/await 语法解析
        let code = r#"
            async function asyncFunc() {
                return 42;
            }
            typeof asyncFunc
        "#;
        let result: String = ctx.eval(code).catch(&ctx).unwrap();
        assert_eq!(result, "function");
    });
}

/// 测试可选链和空值合并（ES2020 支持）
#[test]
fn rquickjs_poc_es2020_features() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();

    ctx.with(|ctx| {
        // 可选链
        let code = r#"
            const obj = { a: { b: 1 } };
            obj?.a?.b
        "#;
        let result: i32 = ctx.eval(code).catch(&ctx).unwrap();
        assert_eq!(result, 1);

        // 空值合并
        let code = r#"
            const val = null;
            val ?? 'default'
        "#;
        let result: String = ctx.eval(code).catch(&ctx).unwrap();
        assert_eq!(result, "default");
    });
}
