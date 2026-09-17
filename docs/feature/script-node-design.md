# 脚本节点设计方案

**版本**: v1.0  
**创建日期**: 2026-03-12  
**状态**: 设计中

---

## 一、概述

脚本节点允许用户在流程执行过程中运行 JavaScript 代码，用于：
- 数据提取与转换
- 变量操作
- 断言验证
- 流程控制

---

## 二、技术选型

### 2.1 JS 运行时选择

| 运行时 | 选择 | 理由 |
|--------|------|------|
| **rquickjs** | ✅ 采用 | 轻量、快速冷启动、无外部依赖、ES2020 支持 |
| V8 (rusty_v8) | ❌ 不采用 | 内存占用大、冷启动慢、部署复杂 |
| deno_core | ❌ 不采用 | API 变动频繁、较重 |
| boa | ❌ 不采用 | 功能不完整 |

### 2.2 为什么选择 QuickJS？

1. **冷启动快**: ~2-5ms vs V8 的 ~50-200ms
2. **内存占用小**: ~500KB vs V8 的 ~10-30MB
3. **部署简单**: 无平台特定二进制依赖
4. **ES2020 支持**: async/await、Promise、可选链等
5. **安全沙箱**: 天然隔离，无文件系统/网络访问

### 2.3 性能对比

| 场景 | QuickJS | V8 |
|------|---------|-----|
| 冷启动 | ~2-5ms | ~50-200ms |
| 简单脚本 (变量操作) | ~0.1ms | ~0.05ms |
| 复杂计算 (JIT 优势) | 慢 5-10x | 快 |
| 内存占用 | ~500KB | ~10-30MB |

**结论**: 对于短生命周期、简单逻辑的脚本节点，QuickJS 综合性能更优。

---

## 三、`fn.*` API 设计

对标 Postman 的 `pm.*` API，使用 `fn.*` 命名空间。

### 3.1 完整 API 定义

```typescript
interface FlowScriptApi {
  // ========== 变量系统 ==========
  fn.variables: {
    /** 获取变量值 */
    get(key: string): any;
    /** 设置单个变量 */
    set(key: string, value: any): void;
    /** 批量设置变量（性能优化） */
    setAll(obj: Record<string, any>): void;
    /** 检查变量是否存在 */
    has(key: string): boolean;
    /** 删除变量 */
    unset(key: string): void;
    /** 清空所有变量 */
    clear(): void;
    /** 获取所有变量 */
    all(): Record<string, any>;
  };

  // ========== 响应数据（只读）==========
  fn.response: {
    /** HTTP 状态码 */
    code: number;
    /** 状态文本 */
    status: string;
    /** 响应头 */
    headers: Record<string, string>;
    /** 响应体（自动 JSON 解析） */
    body: any;
    /** 获取原始文本 */
    text(): string;
    /** 强制 JSON 解析 */
    json(): any;
    /** 响应大小（字节） */
    size: number;
    /** 响应时间（毫秒） */
    time: number;
  };

  // ========== 请求信息（当前节点配置）==========
  fn.request: {
    /** 请求 URL */
    url: string;
    /** 请求方法 */
    method: string;
    /** 请求头 */
    headers: Record<string, string>;
    /** 请求体 */
    body: any;
  };

  // ========== 断言系统 ==========
  fn.test: {
    /** 定义测试用例 */
    define(name: string, fn: () => boolean | void): void;
    /** 断言结果 */
    results: Array<{
      name: string;
      passed: boolean;
      error?: string;
    }>;
    /** 通过数量 */
    passed: number;
    /** 失败数量 */
    failed: number;
  };
  
  /** 链式断言（类似 Chai） */
  fn.expect<T>(value: T): AssertionChain<T>;

  // ========== 流程控制 ==========
  fn.flow: {
    /** 跳过后续节点 */
    skip(): void;
    /** 终止流程并报错 */
    abort(message?: string): void;
    /** 正常终止流程 */
    stop(): void;
  };

  // ========== 日志系统 ==========
  fn.console: {
    log(...args: any[]): void;
    info(...args: any[]): void;
    warn(...args: any[]): void;
    error(...args: any[]): void;
    debug(...args: any[]): void;
    table(data: any): void;
  };

  // ========== 工具函数 ==========
  fn.util: {
    // ----- 编码 -----
    base64Encode(str: string): string;
    base64Decode(str: string): string;
    urlEncode(str: string): string;
    urlDecode(str: string): string;
    htmlEscape(str: string): string;
    htmlUnescape(str: string): string;

    // ----- 加密 -----
    md5(str: string): string;
    sha256(str: string): string;
    sha512(str: string): string;
    hmacSha256(key: string, data: string): string;
    hmacSha512(key: string, data: string): string;

    // ----- JWT -----
    jwtDecode(token: string): { header: any; payload: any; signature: string };
    jwtSign(payload: any, secret: string, options?: JwtOptions): string;
    jwtVerify(token: string, secret: string): boolean;

    // ----- 时间 -----
    timestamp(): number;
    now(): number;
    formatTime(ts: number, format: string): string;
    parseTime(str: string): number;

    // ----- UUID -----
    uuid(): string;
    uuidv4(): string;

    // ----- JSON -----
    jsonParse(str: string): any;
    jsonStringify(obj: any, indent?: number): string;
    jsonPath(obj: any, path: string): any[];

    // ----- 字符串 -----
    randomString(length: number, charset?: string): string;
    repeat(str: string, n: number): string;
    trim(str: string): string;
    trimStart(str: string): string;
    trimEnd(str: string): string;
    replace(str: string, pattern: string, replacement: string): string;
    replaceAll(str: string, pattern: string, replacement: string): string;
    split(str: string, sep: string): string[];
    join(arr: string[], sep: string): string;
    toLowerCase(str: string): string;
    toUpperCase(str: string): string;
    substring(str: string, start: number, end?: number): string;
    charAt(str: string, index: number): string;
    indexOf(str: string, search: string): number;
    includes(str: string, search: string): boolean;
    startsWith(str: string, search: string): boolean;
    endsWith(str: string, search: string): boolean;

    // ----- 数学 -----
    random(): number;
    randomInt(min: number, max: number): number;
    floor(n: number): number;
    ceil(n: number): number;
    round(n: number): number;
    abs(n: number): number;
    pow(base: number, exp: number): number;
    sqrt(n: number): number;
    min(...nums: number[]): number;
    max(...nums: number[]): number;

    // ----- 数组 -----
    arrayContains(arr: any[], value: any): boolean;
    arrayFlatten(arr: any[]): any[];
    arrayUnique(arr: any[]): any[];
    arrayFirst(arr: any[]): any;
    arrayLast(arr: any[]): any;
    arrayTake(arr: any[], n: number): any[];

    // ----- 类型检查 -----
    isArray(v: any): boolean;
    isObject(v: any): boolean;
    isString(v: any): boolean;
    isNumber(v: any): boolean;
    isBoolean(v: any): boolean;
    isNull(v: any): boolean;
    isUndefined(v: any): boolean;
    isFunction(v: any): boolean;
    isEmpty(v: any): boolean;
    typeOf(v: any): string;

    // ----- 对象操作 -----
    keys(obj: any): string[];
    values(obj: any): any[];
    entries(obj: any): Array<[string, any]>;
    hasKey(obj: any, key: string): boolean;
    pick(obj: any, keys: string[]): any;
    omit(obj: any, keys: string[]): any;
    merge(target: any, source: any): any;
    clone(obj: any): any;
  };

  // ========== 环境信息 ==========
  fn.info: {
    /** 流程名称 */
    flowName: string;
    /** 流程 ID */
    flowId: string;
    /** 当前节点 ID */
    nodeId: string;
    /** 当前节点名称 */
    nodeName: string;
  };
}

// ========== JWT 选项 ==========
interface JwtOptions {
  /** 过期时间（秒或时间字符串如 '1h', '2d'） */
  expiresIn?: number | string;
  /** 签发者 */
  issuer?: string;
  /** 接收者 */
  audience?: string;
  /** 签发时间 */
  issuedAt?: number;
}

// ========== 断言链式 API ==========
interface AssertionChain<T> {
  // 链式修饰符
  to: AssertionChain<T>;
  be: AssertionChain<T>;
  have: AssertionChain<T>;
  not: AssertionChain<T>;

  // 相等性
  equal(expected: T): void;
  eql(expected: T): void;           // 深度相等
  notEqual(expected: T): void;

  // 存在性
  exist(): void;
  beDefined(): void;
  beUndefined(): void;
  beNull(): void;
  notBeNull(): void;

  // 类型
  beTrue(): void;
  beFalse(): void;
  beTruthy(): void;
  beFalsy(): void;
  beArray(): void;
  beObject(): void;
  beString(): void;
  beNumber(): void;
  beBoolean(): void;
  beFunction(): void;

  // 包含
  contain(expected: any): void;
  containKey(key: string): void;
  containKeys(keys: string[]): void;

  // 长度
  haveLength(n: number): void;
  haveLengthAbove(n: number): void;
  haveLengthBelow(n: number): void;
  haveLengthWithin(min: number, max: number): void;

  // 字符串
  match(regex: string | RegExp): void;
  startWith(prefix: string): void;
  endWith(suffix: string): void;

  // 数值
  above(n: number): void;
  below(n: number): void;
  atLeast(n: number): void;
  atMost(n: number): void;
  within(min: number, max: number): void;
  closeTo(expected: number, delta: number): void;

  // 数组
  haveMembers(members: any[]): void;
  beEmpty(): void;

  // 函数
  throw(): void;
  throwError(message: string): void;
}
```

### 3.2 使用示例

```javascript
// ===== 示例 1: 提取 token 并设置变量 =====
const data = fn.response.body;
fn.variables.set('token', data.access_token);
fn.variables.set('userId', data.user.id);

// 批量设置（性能更好）
fn.variables.setAll({
  token: data.access_token,
  userId: data.user.id,
  userName: data.user.name
});

// ===== 示例 2: 断言响应 =====
fn.test.define('状态码应为 200', () => fn.response.code === 200);
fn.test.define('响应包含 token', () => fn.response.body.token !== undefined);

// 链式断言
fn.expect(fn.response.code).to.equal(200);
fn.expect(fn.response.body).to.containKey('token');
fn.expect(fn.response.body.items).to.haveLength(10);

// ===== 示例 3: 数据处理 =====
const items = fn.response.body.items;
const ids = items.map(item => item.id);
fn.variables.set('itemIds', ids);

// JSONPath 查询
const names = fn.util.jsonPath(fn.response.body, '$.users[*].name');
fn.variables.set('userNames', names);

// ===== 示例 4: 加密与编码 =====
const signature = fn.util.hmacSha256('secret_key', data);
fn.variables.set('signature', signature);

const encoded = fn.util.base64Encode(JSON.stringify(data));
fn.variables.set('payload', encoded);

// ===== 示例 5: JWT 处理 =====
const decoded = fn.util.jwtDecode(fn.response.body.token);
fn.variables.set('userId', decoded.payload.sub);

const newToken = fn.util.jwtSign(
  { userId: 123, role: 'admin' },
  'my-secret',
  { expiresIn: '1h' }
);
fn.variables.set('newToken', newToken);

// ===== 示例 6: 流程控制 =====
if (fn.response.code !== 200) {
  fn.flow.abort('请求失败，状态码: ' + fn.response.code);
}

// 跳过后续节点
if (fn.response.body.skip === true) {
  fn.flow.skip();
}

// ===== 示例 7: 日志输出 =====
fn.console.log('当前变量:', fn.variables.all());
fn.console.table(fn.response.body.items);
fn.console.warn('这是一个警告');
```

---

## 四、节点配置设计

### 4.1 Rust 类型定义

```rust
/// 脚本节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptNodeConfig {
    /// 脚本内容（JavaScript 代码）
    pub script: String,
    
    /// 超时时间（毫秒），默认 5000
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    
    /// 脚本执行时机
    #[serde(default)]
    pub execution_phase: ScriptExecutionPhase,
    
    /// 错误处理策略
    #[serde(default)]
    pub on_error: ScriptErrorStrategy,
    
    /// 断言失败时的行为
    #[serde(default)]
    pub on_assertion_fail: AssertionFailStrategy,
}

fn default_timeout() -> u64 {
    5000
}

/// 脚本执行时机
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ScriptExecutionPhase {
    /// 后置脚本（请求后执行）- 默认
    #[default]
    PostRequest,
    /// 前置脚本（请求前执行）
    PreRequest,
    /// 独立脚本（作为单独节点）
    Standalone,
}

/// 脚本错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ScriptErrorStrategy {
    /// 可恢复错误 → 路由到 false 端口（默认）
    #[default]
    Recoverable,
    /// 致命错误 → 终止整个流程
    Fatal,
}

/// 断言失败处理策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AssertionFailStrategy {
    /// 路由到 false 端口（默认）
    #[default]
    RouteFalse,
    /// 忽略断言，继续 true 端口
    Ignore,
    /// 终止整个流程
    Abort,
}
```

### 4.2 NodeData 枚举扩展

```rust
// node_config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeData {
    Entry(EntryNodeConfig),
    Http(HttpNodeConfig),
    Conditional(ConditionalNodeConfig),
    WsConnect(WsConnectConfig),
    WsSendWait(WsSendWaitConfig),
    WsClose(WsCloseConfig),
    Assertion(AssertionNodeConfig),
    
    // 新增：脚本节点
    Script(ScriptNodeConfig),
}
```

### 4.3 NodeType 枚举扩展

```rust
// node.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Entry,
    Http,
    Conditional,
    WsConnect,
    WsSendWait,
    WsClose,
    Assertion,
    
    // 新增：脚本节点
    Script,
}
```

---

## 五、执行器设计

### 5.1 执行器结构

```rust
/// 脚本节点执行器
pub struct ScriptNodeExecutor {
    /// Runtime 池（复用以提升性能）
    runtime_pool: Arc<ScriptRuntimePool>,
}

/// JS Runtime 池
pub struct ScriptRuntimePool {
    runtimes: Mutex<Vec<QuickJsRuntime>>,
    max_size: usize,
}

/// 脚本执行上下文（Rust ↔ JS 桥接）
pub struct ScriptExecutionContext {
    /// 输入：从 ExecutionContext 读取
    pub variables: HashMap<String, Value>,
    pub response: Option<ResponseData>,
    pub request: Option<RequestData>,
    pub info: FlowInfo,
    
    /// 输出：写回 ExecutionContext
    pub output_variables: HashMap<String, Value>,
    pub assertion_results: Vec<AssertionResult>,
    pub logs: Vec<LogEntry>,
    
    /// 控制标志
    pub should_skip: bool,
    pub should_abort: bool,
    pub abort_message: Option<String>,
    pub should_stop: bool,
}

/// 响应数据（传递给 fn.response）
#[derive(Debug, Clone, Serialize)]
pub struct ResponseData {
    pub code: u16,
    pub status: String,
    pub headers: HashMap<String, String>,
    pub body: Value,
    pub raw_body: String,
    pub size: usize,
    pub time: u64,
}

/// 请求数据（传递给 fn.request）
#[derive(Debug, Clone, Serialize)]
pub struct RequestData {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

/// 流程信息（传递给 fn.info）
#[derive(Debug, Clone, Serialize)]
pub struct FlowInfo {
    pub flow_name: String,
    pub flow_id: String,
    pub node_id: String,
    pub node_name: String,
}

/// 断言结果
#[derive(Debug, Clone, Serialize)]
pub struct AssertionResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// 日志条目
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: u64,
}
```

### 5.2 NodeExecutor 实现

```rust
#[async_trait::async_trait]
impl NodeExecutor for ScriptNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<Value>), String> {
        let config = match &node.data {
            NodeData::Script(config) => config,
            _ => return Err("节点配置类型不匹配".to_string()),
        };

        // 1. 准备脚本上下文
        let script_context = self.prepare_context(node, context);

        // 2. 从池中获取 runtime
        let runtime = self.runtime_pool.acquire().await;

        // 3. 执行脚本（带超时）
        let result = tokio::time::timeout(
            Duration::from_millis(config.timeout_ms),
            self.execute_script(&runtime, &config.script, script_context),
        )
        .await
        .map_err(|_| format!("脚本执行超时 ({}ms)", config.timeout_ms))?;

        // 4. 处理执行结果
        self.handle_result(result, config, context)
    }
}
```

### 5.3 执行流程

```
┌─────────────────────────────────────────────────────────────┐
│                    脚本节点执行流程                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. 准备阶段                                                 │
│     ├── 从 ExecutionContext 读取变量                         │
│     ├── 从 last_node_output 读取响应数据                      │
│     ├── 构建 fn.info 信息                                    │
│     └── 从 Runtime 池获取可用实例                             │
│                                                             │
│  2. 注册 API                                                 │
│     ├── fn.variables.* → 变量操作                            │
│     ├── fn.response.* → 响应数据（只读）                      │
│     ├── fn.request.* → 请求信息（只读）                       │
│     ├── fn.test.* / fn.expect → 断言                         │
│     ├── fn.flow.* → 流程控制                                 │
│     ├── fn.console.* → 日志输出                              │
│     └── fn.util.* → 工具函数                                 │
│                                                             │
│  3. 执行脚本（带超时）                                        │
│     ├── 运行 JavaScript 代码                                 │
│     ├── 捕获 fn.console.* 输出                               │
│     ├── 记录断言结果                                         │
│     └── 检测控制指令（skip/abort/stop）                       │
│                                                             │
│  4. 提取结果                                                 │
│     ├── 同步 fn.variables 修改到 ExecutionContext            │
│     ├── 收集断言结果                                         │
│     ├── 收集日志                                             │
│     └── 归还 Runtime 到池                                    │
│                                                             │
│  5. 决定路由                                                 │
│     ├── fn.flow.abort() → Err（终止流程）                    │
│     ├── fn.flow.skip() → Ok([], output)（跳过后续）          │
│     ├── fn.flow.stop() → Ok([], output)（正常终止）          │
│     ├── 断言失败 → 根据 on_assertion_fail 决定               │
│     ├── 执行错误 → 根据 on_error 决定                        │
│     └── 正常完成 → Ok([true/false 端口], output)             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 六、端口与路由

### 6.1 端口定义

```rust
// 脚本节点有两个输出端口
impl ScriptNodeExecutor {
    fn get_ports() -> Vec<Port> {
        vec![
            Port {
                id: "true".to_string(),
                name: "成功".to_string(),
                port_type: PortType::Output,
            },
            Port {
                id: "false".to_string(),
                name: "失败".to_string(),
                port_type: PortType::Output,
            },
        ]
    }
}
```

### 6.2 路由规则

| 场景 | 路由 |
|------|------|
| 脚本正常执行，所有断言通过 | `true` 端口 |
| 脚本正常执行，有断言失败 | 根据 `on_assertion_fail` 配置 |
| 脚本执行错误 | 根据 `on_error` 配置 |
| 调用 `fn.flow.skip()` | 无后续节点，流程继续 |
| 调用 `fn.flow.stop()` | 无后续节点，流程正常结束 |
| 调用 `fn.flow.abort()` | 返回错误，流程终止 |
| 超时 | 根据 `on_error` 配置 |

---

## 七、性能优化

### 7.1 Runtime 池化

```rust
pub struct ScriptRuntimePool {
    runtimes: Mutex<Vec<QuickJsRuntime>>,
    max_size: usize,
    available: Semaphore,
}

impl ScriptRuntimePool {
    pub fn new(max_size: usize) -> Self {
        let mut runtimes = Vec::with_capacity(max_size);
        for _ in 0..max_size {
            runtimes.push(Self::create_runtime());
        }
        
        Self {
            runtimes: Mutex::new(runtimes),
            max_size,
            available: Semaphore::new(max_size),
        }
    }

    pub async fn acquire(&self) -> RuntimeGuard<'_> {
        self.available.acquire().await.unwrap();
        let runtime = self.runtimes.lock().unwrap().pop().unwrap();
        RuntimeGuard { pool: self, runtime: Some(runtime) }
    }
}
```

### 7.2 字节码缓存（可选增强）

```rust
/// 编译后的脚本缓存
pub struct CompiledScript {
    /// QuickJS 字节码
    bytecode: Vec<u8>,
    /// 脚本哈希（用于验证缓存有效性）
    hash: u64,
}

impl ScriptNodeExecutor {
    fn compile_script(&self, script: &str) -> Result<CompiledScript, String> {
        let hash = Self::hash_script(script);
        
        // 检查缓存
        if let Some(cached) = self.cache.get(&hash) {
            return Ok(cached.clone());
        }
        
        // 编译并缓存
        let bytecode = self.compile_to_bytecode(script)?;
        let compiled = CompiledScript { bytecode, hash };
        self.cache.insert(hash, compiled.clone());
        Ok(compiled)
    }
}
```

### 7.3 性能预估

| 优化级别 | 首次执行 | 后续执行 | 内存 |
|----------|----------|----------|------|
| 无优化 | ~50ms | ~50ms | 高 |
| Runtime 池化 | ~50ms | ~1ms | 中 |
| Runtime 池 + 字节码缓存 | ~5ms | ~0.5ms | 低 |

---

## 八、错误处理

### 8.1 错误类型扩展

```rust
// error.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorKind {
    // 现有错误类型...
    Configuration,
    ConditionSyntax,
    Execution,
    // ...
    
    // 新增：脚本相关错误
    ScriptSyntax,       // 脚本语法错误
    ScriptRuntime,      // 脚本运行时错误
    ScriptTimeout,      // 脚本执行超时
    ScriptAssertion,    // 断言失败
    ScriptAbort,        // 用户调用 fn.flow.abort()
}
```

### 8.2 错误分类

| 错误类型 | 是否致命 | 说明 |
|----------|----------|------|
| `ScriptSyntax` | 取决于 `on_error` 配置 | 语法错误 |
| `ScriptRuntime` | 取决于 `on_error` 配置 | 运行时错误（如访问未定义变量） |
| `ScriptTimeout` | 取决于 `on_error` 配置 | 执行超时 |
| `ScriptAssertion` | 取决于 `on_assertion_fail` 配置 | 断言失败 |
| `ScriptAbort` | 始终致命 | 用户主动终止 |

---

## 九、依赖清单

```toml
# Cargo.toml 新增依赖

[dependencies]
# JS 运行时
rquickjs = { version = "0.6", features = ["bindgen", "chrono", "array-buffer"] }

# 加密
sha2 = "0.10"
md-5 = "0.10"
hmac = "0.10"
base64 = "0.22"

# JWT
jsonwebtoken = "9"

# 其他（部分已存在）
# jsonpath-rust = "1.0"  # 已有
# uuid = "1"             # 已有
# url = "2.5"            # 已有
```

---

## 十、前端适配

### 10.1 节点配置面板

```typescript
// ScriptNodeConfigPanel.tsx
interface ScriptNodeConfigPanelProps {
  node: Node;
  onUpdate: (config: Partial<ScriptNodeConfig>) => void;
}

function ScriptNodeConfigPanel({ node, onUpdate }: ScriptNodeConfigPanelProps) {
  const config = node.data as ScriptNodeConfig;

  return (
    <div className="space-y-4">
      {/* 脚本编辑器 */}
      <ScriptEditor
        value={config.script}
        onChange={(script) => onUpdate({ script })}
        language="javascript"
        height={300}
      />

      {/* 超时配置 */}
      <TimeoutInput
        value={config.timeout_ms}
        onChange={(timeout_ms) => onUpdate({ timeout_ms })}
        min={100}
        max={60000}
        default={5000}
      />

      {/* 错误处理策略 */}
      <Select
        label="错误处理"
        value={config.on_error}
        options={[
          { value: 'recoverable', label: '可恢复（路由到失败端口）' },
          { value: 'fatal', label: '致命（终止流程）' },
        ]}
        onChange={(on_error) => onUpdate({ on_error })}
      />

      {/* 断言失败策略 */}
      <Select
        label="断言失败时"
        value={config.on_assertion_fail}
        options={[
          { value: 'route_false', label: '路由到失败端口' },
          { value: 'ignore', label: '忽略，继续执行' },
          { value: 'abort', label: '终止流程' },
        ]}
        onChange={(on_assertion_fail) => onUpdate({ on_assertion_fail })}
      />
    </div>
  );
}
```

### 10.2 TypeScript 类型定义

```typescript
// types/node.ts
export interface ScriptNodeConfig {
  type: 'script';
  script: string;
  timeout_ms: number;
  execution_phase: 'pre_request' | 'post_request' | 'standalone';
  on_error: 'recoverable' | 'fatal';
  on_assertion_fail: 'route_false' | 'ignore' | 'abort';
}

export type NodeData = 
  | EntryNodeConfig
  | HttpNodeConfig
  | ConditionalNodeConfig
  | WsConnectConfig
  | WsSendWaitConfig
  | WsCloseConfig
  | AssertionNodeConfig
  | ScriptNodeConfig;  // 新增
```

---

## 十一、测试计划

### 11.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_script_set_variable() {
        let executor = ScriptNodeExecutor::new();
        let node = create_script_node(r#"
            fn.variables.set('test', 'hello');
        "#);
        let mut context = create_test_context();
        
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        
        assert!(result.is_ok());
        assert_eq!(context.variables.get("test"), Some(&json!("hello")));
    }

    #[tokio::test]
    async fn test_script_assertion_pass() {
        let executor = ScriptNodeExecutor::new();
        let node = create_script_node(r#"
            fn.test.define('should pass', () => true);
        "#);
        let mut context = create_test_context();
        
        let (next_nodes, _) = executor.execute(&node, &mut context, &[], &HashMap::new())
            .await.unwrap();
        
        assert_eq!(next_nodes, vec!["true"]);
    }

    #[tokio::test]
    async fn test_script_assertion_fail() {
        let executor = ScriptNodeExecutor::new();
        let node = create_script_node(r#"
            fn.test.define('should fail', () => false);
        "#);
        let mut context = create_test_context();
        
        let (next_nodes, _) = executor.execute(&node, &mut context, &[], &HashMap::new())
            .await.unwrap();
        
        assert_eq!(next_nodes, vec!["false"]);
    }

    #[tokio::test]
    async fn test_script_timeout() {
        let executor = ScriptNodeExecutor::new();
        let node = create_script_node_with_timeout(r#"
            while(true) {}
        "#, 100);
        let mut context = create_test_context();
        
        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("超时"));
    }

    #[tokio::test]
    async fn test_script_util_functions() {
        let executor = ScriptNodeExecutor::new();
        let node = create_script_node(r#"
            const hash = fn.util.md5('test');
            const encoded = fn.util.base64Encode('hello');
            fn.variables.set('hash', hash);
            fn.variables.set('encoded', encoded);
        "#);
        let mut context = create_test_context();
        
        executor.execute(&node, &mut context, &[], &HashMap::new()).await.unwrap();
        
        assert_eq!(context.variables.get("hash"), Some(&json!("098f6bcd4621d373cade4e832627b4f6")));
        assert_eq!(context.variables.get("encoded"), Some(&json!("aGVsbG8=")));
    }
}
```

### 11.2 集成测试

```rust
#[tokio::test]
async fn test_script_node_in_flow() {
    // 创建包含脚本节点的完整流程
    let flow = Flow {
        uuid: "test-flow".to_string(),
        name: "测试脚本节点流程".to_string(),
        nodes: vec![
            create_entry_node(),
            create_http_node("https://api.example.com/data"),
            create_script_node(r#"
                const data = fn.response.body;
                fn.variables.set('userId', data.id);
                fn.test.define('status 200', () => fn.response.code === 200);
            "#),
        ],
        connections: vec![
            create_connection("entry", "out", "http:request", "input"),
            create_connection("http:request", "true", "script", "input"),
        ],
        initial_variables: None,
    };

    let executor = FlowExecutor::new();
    let result = executor.execute_flow(&flow, None, None).await;

    assert!(result.success);
    assert!(result.flow_report.is_some());
    
    let report = result.flow_report.unwrap();
    assert_eq!(report.executed_count, 3);
}
```

---

## 十二、实现路线图

### Phase 1: 基础实现（1 周）

- [ ] 添加依赖（rquickjs, 加密库）
- [ ] 实现 `ScriptNodeConfig` 类型
- [ ] 实现 `ScriptNodeExecutor` 基础框架
- [ ] 实现 `fn.variables.*` API
- [ ] 实现 `fn.response.*` API（只读）
- [ ] 基本单元测试

### Phase 2: 完整 API（1 周）

- [ ] 实现 `fn.test.*` / `fn.expect` 断言系统
- [ ] 实现 `fn.console.*` 日志系统
- [ ] 实现 `fn.flow.*` 流程控制
- [ ] 实现 `fn.util.*` 工具函数（编码、加密、时间等）
- [ ] 实现 `fn.info` 环境信息
- [ ] Runtime 池化

### Phase 3: 前端集成（1 周）

- [ ] 前端类型定义
- [ ] 脚本节点 UI 组件
- [ ] 脚本编辑器（代码高亮）
- [ ] 配置面板
- [ ] API 文档提示

### Phase 4: 优化与测试（1 周）

- [ ] 字节码缓存
- [ ] 性能测试
- [ ] 集成测试
- [ ] 文档完善
- [ ] 示例流程

---

## 十三、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| JS 运行时安全漏洞 | 高 | QuickJS 无网络/文件系统访问，天然沙箱 |
| 脚本死循环 | 中 | 强制超时机制 |
| 内存泄漏 | 中 | Runtime 池化 + 定期重置 |
| 跨平台兼容性 | 低 | rquickjs 无平台依赖 |

---

## 十四、后续增强（可选）

1. **脚本模板库**: 内置常用脚本片段
2. **脚本调试**: 支持断点、变量检查
3. **TypeScript 支持**: 编译时类型检查
4. **自定义函数注册**: 允许用户注册 Rust 函数到 JS
5. **脚本性能分析**: 记录脚本执行时间

---

## 附录 A: fn.util 函数实现对照表

| 函数 | Rust 实现 | 说明 |
|------|-----------|------|
| `base64Encode` | `base64::Engine::encode` | Base64 编码 |
| `base64Decode` | `base64::Engine::decode` | Base64 解码 |
| `urlEncode` | `urlencoding::encode` | URL 编码 |
| `urlDecode` | `urlencoding::decode` | URL 解码 |
| `md5` | `md5::compute` | MD5 哈希 |
| `sha256` | `sha2::Sha256::digest` | SHA-256 哈希 |
| `sha512` | `sha2::Sha512::digest` | SHA-512 哈希 |
| `hmacSha256` | `hmac::Hmac<Sha256>` | HMAC-SHA256 |
| `jwtDecode` | `jsonwebtoken::decode` | JWT 解码 |
| `jwtSign` | `jsonwebtoken::encode` | JWT 签名 |
| `jwtVerify` | `jsonwebtoken::decode` + 验证 | JWT 验证 |
| `uuid` | `uuid::Uuid::new_v4` | UUID 生成 |
| `jsonPath` | `jsonpath_rust::JsonPath::query` | JSONPath 查询 |
