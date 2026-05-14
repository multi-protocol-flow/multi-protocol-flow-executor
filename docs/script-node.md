# 脚本节点 (Script Node)

脚本节点允许在流程执行过程中运行 JavaScript 代码，用于数据提取、变量操作、断言验证和流程控制。

## 概述

脚本节点基于 QuickJS 运行时，提供安全沙箱环境。支持 ES2020 语法（async/await、Promise、可选链等），冷启动快（~2-5ms），内存占用小（~500KB）。

**主要用途:**
- 提取 HTTP 响应数据并设置变量
- 数据转换与处理
- 断言验证响应结果
- 流程控制（跳过、终止）

## fn.* API 参考

### fn.variables - 变量操作

| 方法 | 说明 |
|------|------|
| `fn.variables.get(key)` | 获取变量值 |
| `fn.variables.set(key, value)` | 设置单个变量 |
| `fn.variables.setAll(obj)` | 批量设置变量 |
| `fn.variables.has(key)` | 检查变量是否存在 |
| `fn.variables.unset(key)` | 删除变量 |
| `fn.variables.clear()` | 清空所有变量 |
| `fn.variables.all()` | 获取所有变量（JSON 字符串） |

```javascript
// 设置变量
fn.variables.set('token', 'abc123');

// 批量设置（性能更好）
fn.variables.setAll({
  token: 'abc123',
  userId: 'user-001'
});

// 获取变量
const token = fn.variables.get('token');

// 检查存在
if (fn.variables.has('token')) {
  fn.console.log('token exists');
}

// 获取所有变量
const allVars = JSON.parse(fn.variables.all());
```

### fn.response - 节点输出数据（只读）

`fn.response` 兼容**所有协议节点**的输出，不再局限于 HTTP 响应。

| 属性/方法 | 说明 |
|-----------|------|
| `fn.response.code` | HTTP 状态码（非 HTTP 节点为 0） |
| `fn.response.status` | 状态文本（非 HTTP 节点为空字符串） |
| `fn.response.headers` | 响应头对象（非 HTTP 节点为空） |
| `fn.response.body` | 响应体（JSON 字符串，非 HTTP 节点为完整输出） |
| `fn.response.size` | 响应大小（字节） |
| `fn.response.time` | 响应时间（毫秒） |
| `fn.response.raw` | **完整原始输出数据**（JSON 字符串，不限协议类型） |
| `fn.response.text()` | 获取原始文本 |
| `fn.response.json()` | 获取 JSON 对象（JSON 字符串） |

**`fn.response.raw`** 是最通用的接口，返回前一个节点的完整 `output` 数据，适用于所有协议：

```javascript
// 获取完整输出（任意节点类型）
const raw = JSON.parse(fn.response.raw);

// HTTP 节点示例
const code = fn.response.code;
const body = JSON.parse(fn.response.body);

// 数据库查询节点示例
const data = JSON.parse(fn.response.raw);
fn.variables.set('count', data.rows.length.toString());
fn.variables.set('first_name', data.rows[0].name);

// WebSocket/gRPC 等其他节点
const result = JSON.parse(fn.response.raw);
fn.variables.set('message', result.data);

// 原始 API 仍可用（HTTP 节点兼容）
const rawText = fn.response.text();
const contentType = fn.response.headers['content-type'];
```

### fn.request - 请求数据（只读）

| 属性 | 说明 |
|------|------|
| `fn.request.url` | 请求 URL |
| `fn.request.method` | 请求方法 |
| `fn.request.headers` | 请求头对象 |
| `fn.request.body` | 请求体（JSON 字符串） |

```javascript
fn.console.log('URL:', fn.request.url);
fn.console.log('Method:', fn.request.method);
```

### fn.console - 日志输出

| 方法 | 说明 |
|------|------|
| `fn.console.log(message)` | 普通日志 |
| `fn.console.info(message)` | 信息日志 |
| `fn.console.warn(message)` | 警告日志 |
| `fn.console.error(message)` | 错误日志 |
| `fn.console.debug(message)` | 调试日志 |
| `fn.console.table(data)` | 表格数据 |

```javascript
fn.console.log('当前变量:', fn.variables.all());
fn.console.table(JSON.parse(fn.response.body).items);
fn.console.warn('这是一个警告');
```

### fn.flow - 流程控制

| 方法 | 说明 |
|------|------|
| `fn.flow.skip()` | 跳过后续节点（不再继续执行） |
| `fn.flow.abort(message)` | 终止流程并报错 |
| `fn.flow.stop(message?)` | 正常终止流程（可选终止消息） |

```javascript
// 条件跳过
if (fn.response.code !== 200) {
  fn.flow.abort('请求失败: ' + fn.response.code);
}

// 正常终止（无消息）
if (JSON.parse(fn.response.body).done) {
  fn.flow.stop();
}

// 正常终止（带消息）
if (JSON.parse(fn.response.body).error) {
  fn.flow.stop('API验证失败：数据校验未通过');
}

// 跳过后续节点
if (JSON.parse(fn.response.body).skip) {
  fn.flow.skip();
}
```

### fn.info - 流程信息（只读）

| 属性 | 说明 |
|------|------|
| `fn.info.flowName` | 流程名称 |
| `fn.info.flowId` | 流程 ID |
| `fn.info.nodeId` | 当前节点 ID |
| `fn.info.nodeName` | 当前节点名称 |

```javascript
fn.console.log('流程:', fn.info.flowName);
fn.console.log('节点:', fn.info.nodeName);
```

### fn.util - 工具函数

#### 编码函数

| 方法 | 说明 |
|------|------|
| `fn.util.base64Encode(str)` | Base64 编码 |
| `fn.util.base64Decode(str)` | Base64 解码 |
| `fn.util.urlEncode(str)` | URL 编码 |
| `fn.util.urlDecode(str)` | URL 解码 |
| `fn.util.htmlEscape(str)` | HTML 转义 |
| `fn.util.htmlUnescape(str)` | HTML 反转义 |

```javascript
const encoded = fn.util.base64Encode('hello');  // "aGVsbG8="
const decoded = fn.util.base64Decode('aGVsbG8=');  // "hello"
const urlSafe = fn.util.urlEncode('a b c');  // "a%20b%20c"
```

#### 加密函数

| 方法 | 说明 |
|------|------|
| `fn.util.md5(str)` | MD5 哈希 |
| `fn.util.sha256(str)` | SHA-256 哈希 |
| `fn.util.sha512(str)` | SHA-512 哈希 |
| `fn.util.hmacSha256(key, data)` | HMAC-SHA256 |
| `fn.util.hmacSha512(key, data)` | HMAC-SHA512 |

```javascript
const hash = fn.util.md5('test');  // "098f6bcd4621d373cade4e832627b4f6"
const sig = fn.util.hmacSha256('secret', 'data');
```

#### 时间函数

| 方法 | 说明 |
|------|------|
| `fn.util.timestamp()` | 当前时间戳（毫秒） |
| `fn.util.now()` | 当前 ISO 8601 时间字符串 |
| `fn.util.formatTime(format)` | 格式化当前时间 |
| `fn.util.parseTime(dateStr, format)` | 解析时间为时间戳 |

```javascript
const ts = fn.util.timestamp();  // 1704067200000
const iso = fn.util.now();  // "2024-01-01T00:00:00+00:00"
const formatted = fn.util.formatTime('%Y-%m-%d %H:%M:%S');  // "2024-01-01 12:00:00"
```

#### UUID 函数

| 方法 | 说明 |
|------|------|
| `fn.util.uuid()` | 生成 UUID v4 |
| `fn.util.uuidv4()` | 生成 UUID v4（别名） |

```javascript
const id = fn.util.uuid();  // "550e8400-e29b-41d4-a716-446655440000"
```

#### JSON 函数

| 方法 | 说明 |
|------|------|
| `fn.util.jsonParse(str)` | 解析 JSON 字符串 |
| `fn.util.jsonStringify(obj, indent?)` | 对象转 JSON 字符串 |
| `fn.util.jsonPath(objJson, path)` | JSONPath 查询 |

```javascript
// JSONPath 查询
const body = fn.response.body;
const names = fn.util.jsonPath(body, '$.users[*].name');
fn.variables.set('names', names);
```

#### 类型检查函数

| 方法 | 说明 |
|------|------|
| `fn.util.isArray(v)` | 是否为数组 |
| `fn.util.isObject(v)` | 是否为对象 |
| `fn.util.isString(v)` | 是否为字符串 |
| `fn.util.isNumber(v)` | 是否为数字 |
| `fn.util.isBoolean(v)` | 是否为布尔值 |
| `fn.util.isNull(v)` | 是否为 null |
| `fn.util.isUndefined(v)` | 是否为 undefined |
| `fn.util.isFunction(v)` | 是否为函数 |
| `fn.util.isEmpty(v)` | 是否为空值 |
| `fn.util.typeOf(v)` | 返回类型字符串 |

```javascript
fn.util.isArray([1, 2, 3]);  // true
fn.util.isEmpty('');  // true
fn.util.typeOf(123);  // "number"
```

#### 对象操作函数

| 方法 | 说明 |
|------|------|
| `fn.util.keys(obj)` | 获取对象键数组（JSON 字符串） |
| `fn.util.values(obj)` | 获取对象值数组（JSON 字符串） |
| `fn.util.entries(obj)` | 获取键值对数组（JSON 字符串） |
| `fn.util.hasKey(obj, key)` | 检查是否包含键 |
| `fn.util.pick(obj, keys)` | 选择指定键（JSON 字符串） |
| `fn.util.omit(obj, keys)` | 排除指定键（JSON 字符串） |
| `fn.util.merge(target, source)` | 合并对象（JSON 字符串） |
| `fn.util.clone(obj)` | 深拷贝对象（JSON 字符串） |

```javascript
const keys = JSON.parse(fn.util.keys(myObj));
const merged = JSON.parse(fn.util.merge(obj1, obj2));
const hasKey = fn.util.hasKey(myObj, 'name');
```

### fn.expect() - 链式断言

链式断言 API，类似 Chai 风格。

#### 链式修饰符

| 修饰符 | 说明 |
|--------|------|
| `.to` | 链式调用（无操作） |
| `.be` | 链式调用（无操作） |
| `.have` | 链式调用（无操作） |
| `.not` | 反转下一个断言结果 |

#### 断言方法

| 方法 | 说明 |
|------|------|
| `.equal(expected)` | 严格相等 |
| `.eql(expected)` | 深度相等 |
| `.notEqual(expected)` | 不相等 |
| `.exist()` | 存在（非 null/undefined） |
| `.beDefined()` | 已定义 |
| `.beNull()` | 为 null |
| `.notBeNull()` | 不为 null |
| `.beTrue()` | 为 true |
| `.beFalse()` | 为 false |
| `.beTruthy()` | 为真值 |
| `.beFalsy()` | 为假值 |
| `.contain(expected)` | 包含（数组/字符串） |
| `.containKey(key)` | 包含键 |
| `.haveLength(n)` | 长度为 n |
| `.match(regex)` | 正则匹配 |
| `.startWith(prefix)` | 以指定字符串开头 |
| `.endWith(suffix)` | 以指定字符串结尾 |
| `.above(n)` | 大于 n |
| `.below(n)` | 小于 n |

```javascript
// 基本断言
fn.expect(fn.response.code).to.equal(200);
fn.expect(fn.response.code).to.be.above(199);

// 链式断言
fn.expect(fn.response.body)
  .to.containKey('token');

fn.expect(JSON.parse(fn.response.body).items)
  .to.haveLength(10);

// 否定断言
fn.expect(fn.response.code).not.to.equal(500);

// 深度相等
fn.expect(result).to.eql({ status: 'ok' });
```

## 节点配置

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `script` | string | - | JavaScript 代码 |
| `timeout_ms` | number | 5000 | 超时时间（毫秒） |
| `on_error` | string | "recoverable" | 错误处理策略 |
| `on_assertion_fail` | string | "route_false" | 断言失败处理 |

### 错误处理策略

| 值 | 说明 |
|----|------|
| `recoverable` | 路由到 false 端口 |
| `fatal` | 终止整个流程 |

### 断言失败处理

| 值 | 说明 |
|----|------|
| `route_false` | 路由到 false 端口 |
| `ignore` | 忽略，继续 true 端口 |
| `abort` | 终止整个流程 |

## 使用示例

### 提取 Token

```javascript
const body = JSON.parse(fn.response.body);
fn.variables.set('token', body.access_token);
fn.variables.set('userId', body.user.id);
```

### 断言响应

```javascript
fn.expect(fn.response.code).to.equal(200);

const body = JSON.parse(fn.response.body);
fn.expect(body).to.containKey('data');
fn.expect(body.data).to.haveLength(5);
```

### 数据处理

```javascript
const body = JSON.parse(fn.response.body);
const ids = body.items.map(item => item.id).join(',');
fn.variables.set('itemIds', ids);
```

### 加密签名

```javascript
const data = JSON.stringify(JSON.parse(fn.response.body));
const signature = fn.util.hmacSha256('secret_key', data);
fn.variables.set('signature', signature);
```

### 条件流程控制

```javascript
if (fn.response.code !== 200) {
  fn.flow.abort('请求失败: ' + fn.response.code);
}

const body = JSON.parse(fn.response.body);
if (body.skip === true) {
  fn.flow.skip();
}
```

## 安全约束

脚本节点运行在沙箱环境中，**禁止**以下操作:

- 文件系统访问
- 网络请求
- 模块加载（require/import）
- 访问外部进程

这些限制由 QuickJS 运行时天然保证，确保脚本执行的安全性。
