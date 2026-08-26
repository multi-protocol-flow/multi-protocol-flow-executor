# MPE - MCP Server 可视化调试与评测 (Visual Debug & Eval) 宣传视频录制指引

> **目标**：以直击 AI 开发者痛点的视角，在 60~75 秒内展示：
> 1. 无需写复杂脚本，像使用 Postman 一样直观连接、发现、调试 MCP Server (Stdio / SSE)。
> 2. 支持多工具调用编排、数据提取与自动化断言评测（Evals）。
> 3. 支持无头 CLI 模式运行，直接嵌入 CI/CD 流水线。

---

## 推荐录屏与制作方案（根据你的偏好选择）

### 方案 A：零剪辑极速出片（强烈推荐 ⭐️⭐️⭐️⭐️⭐️）
- **工具**：**Screen Studio** (macOS) 或 **CleanShot X** / **CapCut (剪映)**
- **特点**：
  - Screen Studio 自动平滑追踪鼠标焦点并放大操作区域，自带专业级阴影与背景。
  - 你只需打开 MPE 窗口，按照下方表格操作一遍（约 1 分钟）。
  - 将下方的旁白文案直接复制进剪映的「文本朗读（TTS）」一键生成专业配音，甚至不需要真人开麦。

### 方案 B：无声动图（GitHub / 官网嵌入）
- **工具**：**LICEcap** 或 **Gifox** / **CleanShot X**
- **特点**：只需录制“点击运行 -> 节点流转流水灯 -> 展开查看 MCP Tool 结果 -> 终端 CLI 运行”这 15~20 秒的高光片段导出为 GIF。

---

## 视频分镜与操作脚本（精确到秒）

| 时间轴 | 画面核心动作 (屏幕操作) | 鼠标与视角焦点 | 视觉高亮点 | 旁白 / 中文解说文案 (可直接用于配音) | English Voiceover (Optional) |
|---|---|---|---|---|---|
| **0:00 - 0:10** (痛点引入) | 打开 MPE 画布，展示已经就绪的 `MCP Server Visual Debug & Eval` 流程拓扑。 | 画布居中，平移展示 `mcp:connect` $\to$ `mcp:call_tool` $\to$ `assertion` 连线。 | 现代暗色调画布，流畅的节点拓扑连线。 | “还在用终端硬敲 JSON-RPC 或者写 Python 脚本调试 MCP Server 吗？来试试 MPE 吧。” | "Still debugging MCP Servers with tedious CLI commands or custom scripts? Meet MPE." |
| **0:10 - 0:25** (服务连接与发现) | 点击 `Connect MCP Server` 节点，在右侧面板展示 Stdio 配置（`npx @modelcontextprotocol/server-everything`），点击「测试连接/发现」。 | 鼠标点击右侧抽屉中的服务发现区域，关注发现的 Tools 列表。 | 节点上方动态跳出 Tools、Resources 数量徽章。 | “填入 Stdio 或 SSE 命令，一键连接并自动内省发现所有 Tools、Resources 和 Prompts。” | "Connect via Stdio or SSE. Automatically discover all tools, resources, and prompts in one click." |
| **0:25 - 0:40** (多工具编排与参数传递) | 依次点击展示两个并行/串行的 `mcp:call_tool` 节点（调用 `echo` 和 `add`），以及 `Variable Extractor` 变量提取。 | 鼠标划过 Tool Name 下拉选择与 JSON 参数插值 `{{test_query}}`。 | 参数支持模板变量智能补全。 | “可视化配置工具入参，支持上下文变量无缝传递，轻松编排复杂工具调用链。” | "Visually configure tool parameters with dynamic variable chaining across the entire flow." |
| **0:40 - 0:55** (一键运行与实时断言评测) | 点击右上角「执行 (Run Flow)」按钮。 | 视角聚焦画布上的执行光效：节点依次高亮变绿，断言节点打勾。 | 节点执行状态呼吸灯特效，执行耗时微秒级展示。 | “点击运行，执行流实时可视化流转。搭配断言节点，秒级完成对 Agent 工具准确率与输出结构的 Eval 评测。” | "Execute in real-time. Combine with assertion nodes to evaluate tool accuracy and response schemas effortlessly." |
| **0:55 - 1:10** (无头 CLI 与 CI/CD) | 切换到终端命令行，输入 `mpe run tests/fixtures/flows/mcp_server_debug_eval.json` 回车，瞬间输出彩色终端报告。 | 终端窗口中的 ASCII 流程图与各节点通过绿标。 | 极致轻量：Rust 单二进制，极速冷启动。 | “不仅有 GUI，同一个流程用单二进制 CLI 直接跑，无缝接入 GitHub Actions 做回归测试。” | "From GUI to Headless CLI: run the same flow in your CI/CD pipeline with a single Rust binary." |
| **1:10 - 1:15** (结尾号召) | 画面切回 GitHub 仓库主页 / 官网展示，展示 Star 按钮与 Download 链接。 | 鼠标悬停在 GitHub Star 按钮。 | Slogan: *The Next-Gen Multi-Protocol Flow Executor*. | “多协议流程编排执行器 MPE，开源免费，欢迎体验！” | "MPE — The Next-Gen Multi-Protocol Flow Engine. Open source & free on GitHub." |

---

## 录制前准备检查清单

1. **测试用例文件位置**：
   - 流程文件已为你生成在：`tests/fixtures/flows/mcp_server_debug_eval.json`
2. **本地环境准备**：
   - 确保本机装有 Node.js (`npx`)，可快速拉起 `@modelcontextprotocol/server-everything` 测试服务。
3. **CLI 演示命令**：
   ```bash
   # 直接在项目根目录运行
   cargo run --bin mpe-cli -- run tests/fixtures/flows/mcp_server_debug_eval.json
   ```
