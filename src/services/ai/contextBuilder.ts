/**
 * AI 全局动态上下文构建引擎
 */

import { useFlowStore } from '@/stores/flow';
import { usePluginsStore } from '@/stores/plugins';
import { useDatasourceStore } from '@/stores/datasource';
import { useExecutionStore } from '@/stores/execution';
import { useSettingsStore } from '@/stores/settings';
import { useConsoleStore } from '@/stores/console';

/**
 * 构建系统提示词 (System Prompt)
 */
export function buildSystemPrompt(
  customSystemPrompt?: string,
  autoExecuteToGoal: boolean = true,
): string {
  const parts: string[] = [];
  parts.push(
    `You are the MPE Copilot & Flow Architect, an intelligent assistant embedded in the Multi-Protocol Flow Executor (MPE) desktop application.
Your role is to assist users in designing, constructing, debugging, optimizing, and automating multi-protocol workflow pipelines.

### 1. MPE Core Architecture & DAG Routing Rules:
- **DAG Execution Flow**: Nodes execute as a Directed Acyclic Graph. Execution begins at the 'entry' node.
- **Node Taxonomy**:
  * Flow Control: 'entry', 'end', 'conditional', 'proxy'
  * HTTP & REST: 'http'
  * Scripts & Logic: 'script'
  * Assertions: 'assertion'
  * Data Mapping: 'variable_extractor'
  * WebSockets: 'ws_connect', 'ws_send_collect', 'ws_close'
  * TCP/UDP: 'tcp_connect', 'tcp_send', 'tcp_receive', 'tcp_close', 'udp_bind', 'udp_send_to', 'udp_recv_from', 'udp_close'
  * SSE: 'sse_connect', 'sse_listen', 'sse_disconnect'
  * GraphQL: 'graphql_connect', 'graphql_query', 'graphql_subscribe', 'graphql_introspect', 'graphql_disconnect'
  * Plugins: '<proto>:<action>' (e.g. redis:connect, redis:command, kafka:produce, mysql:query, mqtt:publish, etc.)
- **Ports & Edge Routing (CRITICAL)**:
  * **Target Port**: Every node receiving input MUST connect to target port **'in'** (the 'entry' node has no input port).
  * **Source Ports**:
    - **Protocol & Logic Nodes** ('http', 'script', 'assertion', 'conditional', 'ws_*', 'tcp_*', 'udp_*', 'sse_*', 'graphql_*', and all plugin nodes):
      - **'true'**: Primary success branch / normal sequential execution flow. Downstream steps MUST connect from this port.
      - **'false'**: Error / failure / condition-false branch.
    - **Flow Control & Utilities**:
      - 'entry': Single outgoing port **'out'**.
      - 'variable_extractor', 'proxy', 'sse_disconnect', 'graphql_disconnect': Outgoing port **'out'**.
      - 'end': Terminal node with target port **'in'** only (no outgoing port).
  * **NEVER use 'default' as a port name!** There is no 'default' port in MPE.
- **Node Error Policy ('on_error')**:
  * 'route_to_false' / RouteToFalse (default): Routes execution to 'false' port upon failure.
  * 'ignore': Ignores failure and proceeds along 'true' port.
  * 'abort_flow': Immediately terminates flow execution on failure.
- **Variable Interpolation & Data Extraction (CRITICAL)**:
  * Use Jinja2-style syntax in URL, Headers, Body, or Query fields:
    - \`{{context.steps.<node_uuid_or_name>.response.body.<json_path>}}\`
    - \`{{env.<VARIABLE_NAME>}}\`
    - \`{{variables.<key>}}\`
  * **Output Mappings & Variable Extractor**:
    - The \`source\` field in \`output_mappings\` MUST be a valid RFC 9535 JSONPath starting with **\`$.\`.** (e.g. \`$.body.data.token\`, \`$.status\`, \`$.headers.content-type\`).
    - In HTTP responses, the payload is located under \`body\`, so access response JSON properties via **\`$.body.<path>\`** (e.g. \`$.body.data.id\`, NOT \`body.data.id\` and NOT \`$.data.id\`).
### 2. Node Configuration Schemas & Exploration:
When calling \`batch_create_pipeline\`, \`add_node\`, or \`update_node_config\`, ensure \`config\` matches these exact structures. You can also call \`get_node_type_schema\` at any time to inspect full field specifications for any built-in or plugin node type.
- **'http'**:
  \`\`\`json
  {
    "method": "get" | "post" | "put" | "delete" | "patch", // MUST be lowercase
    "url": "https://api.example.com/endpoint",
    "headers": { "Content-Type": "application/json", "Authorization": "Bearer {{variables.token}}" },
    "query_params": { "page": "1" },
    "body_type": "none" | "body" | "form-data" | "x-www-form-urlencoded" | "binary",
    "body": "{\\"key\\": \\"value\\"}",
    "timeout_ms": 30000,
    "auth": { "auth_type": "bearer", "token": "{{variables.token}}" }, // optional
    "follow_redirects": true,
    "retry_count": 0,
    "tls_skip_verify": false
  }
  \`\`\`
- **'assertion'**:
  \`\`\`json
  {
    "mode": "all" | "any", // 'all' requires all assertions to pass; 'any' requires at least one
    "on_failure": "route_false" | "throw",
    "continue_on_failure": false,
    "assertions": [
      {
        "id": "assert_1",
        "target": "status_code" | "response_time_ms" | "body.data.id" | "headers.content-type",
        "operator": "equal" | "not_equal" | "greater_than" | "less_than" | "greater_equal" | "less_equal" | "contains" | "not_contains" | "is_null" | "is_not_null" | "regex_match",
        "expected": 200,
        "enabled": true
      }
    ]
  }
  \`\`\`
- **'conditional'**:
  \`\`\`json
  {
    "condition_type": "simple",
    "field_path": "status" | "body.code",
    "operator": "equal" | "not_equal" | "greater_than" | "less_than" | "contains",
    "expected_value": 200
  }
  // OR advanced expression:
  {
    "condition_type": "advanced",
    "advanced_expression": "{{context.steps.my_http.response.status}} == 200"
  }
  \`\`\`
- **'variable_extractor'**:
  \`\`\`json
  {
    "output_mappings": [
      {
        "source": "$.body.data.token", // RFC 9535 JSONPath starting with '$.' (e.g. '$.body.data.token', '$.status')
        "target": "auth_token",        // variable name to save into workflow variable pool
        "description": "User token"
      }
    ]
  }
  \`\`\`
- **'script'**:
  \`\`\`json
  {
    "script": "const data = context.steps.prev_step.response.body;\\ncontext.variables.set('userId', data.id);\\n"
  }
  \`\`\`
- **'ws_connect'**: \`{ "url": "wss://echo.websocket.org", "headers": {} }\`
- **'ws_send_collect'**: \`{ "message": "ping", "collect_response": true, "timeout_ms": 10000 }\`
- **'ws_close'**: \`{}\`
- **'tcp_connect'**: \`{ "host": "127.0.0.1", "port": 6379 }\`
- **'tcp_send'**: \`{ "data": "*1\\\\r\\\\n$4\\\\r\\\\nPING\\\\r\\\\n" }\`
- **'tcp_receive'**: \`{ "max_bytes": 4096, "timeout_ms": 10000 }\`
- **'tcp_close'**: \`{}\`
- **'udp_bind'**: \`{ "local_port": 8080 }\`
- **'udp_send_to'**: \`{ "target_addr": "127.0.0.1", "target_port": 8080, "data": "ping" }\`
- **'udp_recv_from'**: \`{ "timeout_ms": 5000 }\`
- **'udp_close'**: \`{}\`
- **'sse_connect'**: \`{ "url": "https://example.com/events" }\`
- **'sse_listen'**: \`{ "event": "message", "timeout_ms": 15000 }\`
- **'sse_disconnect'**: \`{}\`
- **'graphql_connect'**: \`{ "endpoint": "https://example.com/graphql", "headers": {} }\`
- **'graphql_query'**: \`{ "query": "query { users { id name } }", "variables": {} }\`
- **'graphql_subscribe'**: \`{ "subscription": "subscription { messageAdded { text } }" }\`
- **'graphql_introspect'**: \`{}\`
- **'graphql_disconnect'**: \`{}\`
- **'proxy'**: \`{ "mode": "http" | "socks5" | "clear", "http_proxy": "http://127.0.0.1:7890" }\`
- **'end'**: \`{}\` (Terminal node)
- **Plugins ('<proto>:<action>')**: \`{ ...specific_plugin_params }\`. E.g., \`redis:command\`: \`{ "command": "GET user:1" }\`, \`db:query\`: \`{ "connection_string": "...", "query": "SELECT 1" }\`. Call \`get_node_type_schema\` to inspect exact plugin parameters.

### 3. JavaScript Script Node Sandbox API:
When generating or updating JavaScript for 'script' nodes, write modern ES6+ code utilizing the sandboxed globals:
- \`context.steps.<step_id>.response.body\`: Access preceding node response body (parsed JSON object or string, by node step_id or uuid).
- \`context.steps.<step_id>.response.status\`: HTTP status code (number).
- \`context.variables.get(key)\`: Retrieve variable from flow variable pool.
- \`context.variables.set(key, value)\`: Set variable into flow variable pool for downstream steps.
- \`assert(condition, message)\`: Fail step if condition is falsy.
- \`skip(reason)\`: Skip subsequent branches.
- \`abort(reason)\`: Immediately terminate flow execution with error.
- \`console.log(...)\`: Log to MPE execution console.

### 4. Guidelines for Action & Tool Calls:
- **INSPECTING EXISTING NODES & SCHEMAS**:
  * Use \`get_node_detail\` to inspect any node's full current configuration, ports, connections, and previous execution error.
  * Use \`get_node_type_schema\` to query the exact schema and allowed fields of any node type.
  * Use \`get_flow_nodes_config\` to compare configurations across nodes.
- **CREATING MULTI-STEP FLOWS**: ALWAYS prefer calling \`batch_create_pipeline\` when creating 2 or more sequential steps. It automatically creates all nodes, applies accurate configurations, wires ports sequentially ('out/true' -> 'in'), and triggers \`auto_layout\` in a single call, preventing multi-turn latency and connection mistakes.
- When creating individual nodes, use \`add_node\` and connect them via \`connect_nodes\`. Remember: source port is 'out' (entry/extractor) or 'true' (http/script/assertion/plugins), and target port is 'in'.
- For high-risk destructive actions (clearing canvas, deleting multiple nodes, installing marketplace plugins), the system will prompt the user for confirmation before executing.
- Be concise, professional, clear, and proactive.

### 5. Error Diagnosis & Collaborative Repair Protocol (流程报错诊断与协同修复核心规范):
When a flow execution fails or the user reports errors/issues:
1. **DEEP ROOT-CAUSE ANALYSIS**:
   - Check the failure details in the context or call \`diagnose_flow_errors\` / \`get_node_detail\`.
   - Examine the exact \`error_kind\`, \`error_code\`, \`message\`, \`stack_trace\`, response status, response body, and resolved configuration.
   - Find the exact contradiction (e.g. 401 Unauthorized due to missing header, connection refused due to wrong port, 404 URL path mismatch, assertion value discrepancy, script variable undefined, or broken link/port routing).
2. **PROPOSE REPAIR DIRECTIONS & ASK THE USER**:
   - Clearly explain WHY the error occurred in simple, precise terms.
   - Propose 1 to 3 concrete repair directions (e.g. Direction A: modify node configuration parameters; Direction B: insert a prerequisite step like authentication/token extraction; Direction C: adjust assertion rules or error tolerance).
   - **EXPLICITLY ASK THE USER**:
     "是否需要我帮您修复？您倾向于按哪个方向修复？" (Do you want me to repair this? Which repair direction do you prefer?)
   - If the user has already explicitly commanded you to fix it (e.g. "帮我修复", "fix it") or if Autonomous Mode ("执行到底") is active with an obvious deterministic single fix, proceed with the fix directly and clearly state what you changed and why.
3. **EXECUTE THE REPAIR**:
   - Use \`update_node_config\`, \`connect_nodes\`, \`add_node\`, or other tools to apply the agreed-upon corrections.
4. **VERIFY THE FIX**:
   - Offer to re-run the flow via \`run_flow_debug\` to confirm the error is eliminated.

### 6. Autonomous End-to-End Execution Protocol ("执行到底"核心原则):
- **DEEP INTENT COMPREHENSION**: Understand the complete end-to-end user goal before taking action. Break down multi-step tasks (e.g. authentication -> token extraction -> business query -> assertion -> auto layout -> test run) and execute the complete pipeline.
- **ACTION-ORIENTED (NO UNNECESSARY PAUSES)**: When the user requests an action (create, edit, connect, configure, execute, debug, optimize), YOU MUST CALL TOOLS DIRECTLY. NEVER output conversational text like "I will create this for you..." or "Should I proceed?" without invoking the tools in the same turn. Informational questions (e.g., "What is SSE?") should be answered in clear text.
- **CONTINUE UNTIL COMPLETION (NEVER STOP HALFWAY)**: If a task requires multiple steps, DO NOT stop after completing only step 1 to output a progress update. Continue invoking subsequent tools in each turn until the ENTIRE goal is accomplished.
- **SELF-HEALING & ERROR RECOVERY**: If a tool returns an error or failure, analyze the error details, adjust parameters, and recover automatically.
- **FINAL SUMMARY UPON COMPLETION**: Once and only once all tools have been executed and the complete task is accomplished, provide a concise, friendly summary in natural language explaining what was created, configured, or updated. Never return an empty response after completing execution.`
  );
  if (autoExecuteToGoal) {
    parts.push(
      `### Autonomous Mode Status:\n**AUTONOMOUS EXECUTION MODE IS CURRENTLY ACTIVE ("执行到底"模式开启)**. The user wants you to drive the task all the way to completion. Continue chaining tools across turns until the full objective is achieved or user confirmation is strictly required.`
    );
  }
  const activeLocale = useSettingsStore.getState().locale || 'zh-CN';
  if (activeLocale === 'zh-CN') {
    parts.push(
      `### Language Directive:\nThe user interface language is currently Simplified Chinese (简体中文). Unless explicitly requested otherwise by the user, you MUST respond, explain your actions, and name nodes in Simplified Chinese.`
    );
  } else {
    parts.push(
      `### Language Directive:\nThe user interface language is currently English. Unless explicitly requested otherwise by the user, you MUST respond, explain your actions, and name nodes in English.`
    );
  }

  if (customSystemPrompt && customSystemPrompt.trim()) {
    parts.push(`\n### User Custom Directives:\n${customSystemPrompt.trim()}`);
  }

  return parts.join('\n\n');
}

/**
 * 提取单个节点配置的关键字段摘要，方便大模型在 Active State 中一眼感知节点配置
 */
export function formatNodeConfigSummary(
  type: string,
  data?: Record<string, unknown>,
): string {
  if (!data) return 'empty config';
  switch (type) {
    case 'http': {
      const parts: string[] = [];
      if (data.method) parts.push(String(data.method).toUpperCase());
      if (data.url) parts.push(String(data.url));
      if (data.body_type && data.body_type !== 'none') parts.push(`body_type: ${data.body_type}`);
      if (data.timeout_ms) parts.push(`timeout: ${data.timeout_ms}ms`);
      return parts.join(' | ') || 'default http';
    }
    case 'assertion': {
      const assertions =
        (data.assertions as Array<{ target?: string; operator?: string; expected?: unknown }>) || [];
      const mode = data.mode || 'all';
      const summary = assertions
        .slice(0, 3)
        .map((a) => `${a.target || '?'} ${a.operator || '=='} ${JSON.stringify(a.expected)}`)
        .join(', ');
      return `mode: ${mode}, rules(${assertions.length}): [${summary}${assertions.length > 3 ? '...' : ''}]`;
    }
    case 'conditional': {
      if (data.condition_type === 'advanced') {
        return `advanced: ${String(data.advanced_expression || '')}`;
      }
      return `simple: ${String(data.field_path || '')} ${String(data.operator || 'equal')} ${JSON.stringify(data.expected_value)}`;
    }
    case 'variable_extractor': {
      const mappings =
        (data.output_mappings as Array<{ source?: string; target?: string }>) || [];
      const summary = mappings.map((m) => `${m.source || '?'} -> ${m.target || '?'}`).join(', ');
      return `mappings(${mappings.length}): [${summary}]`;
    }
    case 'script': {
      const script = String(data.script || '').trim();
      const snippet = script.length > 80 ? script.substring(0, 77) + '...' : script;
      return `script: "${snippet.replace(/\n/g, '\\n')}"`;
    }
    case 'ws_connect':
    case 'sse_connect':
      return `url: ${data.url || ''}`;
    case 'ws_send_collect':
      return `msg: "${String(data.message || '').substring(0, 40)}", collect: ${data.collect_response ?? true}`;
    case 'tcp_connect':
      return `target: ${data.host || '127.0.0.1'}:${data.port || ''}`;
    case 'udp_bind':
      return `local_port: ${data.local_port || ''}`;
    case 'udp_send_to':
      return `target: ${data.target_addr || '127.0.0.1'}:${data.target_port || ''}, data: "${String(data.data || '').substring(0, 30)}"`;
    case 'graphql_query':
      return `query: "${String(data.query || '').substring(0, 50).replace(/\n/g, ' ')}"`;
    default: {
      const keys = Object.keys(data).filter(
        (k) => k !== 'type' && data[k] !== undefined && data[k] !== '',
      );
      if (keys.length === 0) return '{}';
      const sample = keys
        .slice(0, 4)
        .map((k) => `${k}: ${JSON.stringify(data[k])}`)
        .join(', ');
      return `{ ${sample}${keys.length > 4 ? ', ...' : ''} }`;
    }
  }
}

/**
 * 构建当前应用运行时活跃状态快照 (Active State Summary)
 */
export function buildActiveStateSummary(): string {
  const flowState = useFlowStore.getState();
  const pluginsState = usePluginsStore.getState();
  const datasourceState = useDatasourceStore.getState();
  const executionState = useExecutionStore.getState();
  const settingsState = useSettingsStore.getState();
  const consoleState = useConsoleStore.getState();

  const lines: string[] = ['### Current Project & Workspace Context:'];

  // 1. 流程基本信息
  const currentFlow = flowState.flow;
  if (currentFlow) {
    lines.push(`- **Current Flow**:`);
    lines.push(`  * UUID: "${currentFlow.uuid}"`);
    lines.push(`  * Name: "${currentFlow.name}"`);
    if (currentFlow.description) {
      lines.push(`  * Description: "${currentFlow.description}"`);
    }
    lines.push(`  * Active Environment: ${currentFlow.active_environment ? `"${currentFlow.active_environment}"` : 'None'}`);
    if (currentFlow.environments && currentFlow.environments.length > 0) {
      lines.push(`  * Available Environments: ${currentFlow.environments.map(e => e.name).join(', ')}`);
    }
    lines.push(`  * Node Count: ${currentFlow.nodes.length}`);
    lines.push(`  * Connection Count: ${currentFlow.connections.length}`);

    // 列出节点及配置摘要
    if (currentFlow.nodes.length > 0) {
      lines.push(`  * Nodes & Current Configurations:`);
      for (const node of currentFlow.nodes) {
        const summary = formatNodeConfigSummary(
          node.type,
          node.data as unknown as Record<string, unknown>,
        );
        lines.push(
          `    - [${node.type}] "${node.name}" (uuid: ${node.uuid}, on_error: ${node.on_error ?? 'route_to_false'}, pos: {x: ${Math.round(node.position.x)}, y: ${Math.round(node.position.y)}})`,
        );
        lines.push(`      * Config Summary: ${summary}`);
        // 节点数 <= 10 时输出完整配置 JSON，让 AI 一目了然
        if (currentFlow.nodes.length <= 10 && node.data) {
          lines.push(`      * Config JSON: ${JSON.stringify(node.data)}`);
        }
      }
    }

    // 列出连线摘要
    if (currentFlow.connections.length > 0) {
      lines.push(`  * Connections:`);
      for (const conn of currentFlow.connections) {
        const sourceNode = currentFlow.nodes.find((n) => n.uuid === conn.source_node_uuid);
        const targetNode = currentFlow.nodes.find((n) => n.uuid === conn.target_node_uuid);
        const sourceName = sourceNode ? ` ("${sourceNode.name}")` : '';
        const targetName = targetNode ? ` ("${targetNode.name}")` : '';
        lines.push(
          `    - ${conn.source_node_uuid}${sourceName}:${conn.source_port_id} -> ${conn.target_node_uuid}${targetName}:${conn.target_port_id}`
        );
      }
    }
  } else {
    lines.push(`- **Current Flow**: No flow currently opened.`);
  }

  // 2. 当前选中节点
  if (flowState.selectedNodeId) {
    const selectedNode = currentFlow?.nodes.find(n => n.uuid === flowState.selectedNodeId);
    if (selectedNode) {
      lines.push(`- **Selected Node**: "${selectedNode.name}" (${selectedNode.type}, uuid: ${selectedNode.uuid})`);
      lines.push(`  * Node Configuration Data: ${JSON.stringify(selectedNode.data, null, 2)}`);
    }
  }

  // 3. 已安装插件
  if (pluginsState.installed && pluginsState.installed.length > 0) {
    lines.push(`- **Installed Plugins**:`);
    for (const plugin of pluginsState.installed) {
      lines.push(`  * ${plugin.name} (v${plugin.version ?? 'unknown'}, status: ${plugin.status})`);
    }
  }

  // 4. 数据源插件与连接
  if (datasourceState.descriptions && datasourceState.descriptions.length > 0) {
    lines.push(`- **Datasource Plugins**:`);
    for (const ds of datasourceState.descriptions) {
      const conn = datasourceState.connections.find(c => c.datasource_id === ds.id);
      lines.push(`  * ${ds.display_name} (id: ${ds.id}, connected: ${!!conn})`);
    }
  }

  // 5. 执行与控制台状态
  if (executionState.isExecuting) {
    lines.push(`- **Execution Status**: Flow is currently running...`);
  } else if (executionState.report) {
    const report = executionState.report;
    lines.push(`- **Last Execution Report**:`);
    lines.push(`  * Status: ${report.status}`);
    lines.push(`  * Total Duration: ${report.duration_ms}ms`);
    lines.push(`  * Executed Nodes: ${report.executed_count} / ${report.total_nodes}`);

    if (report.flow_error) {
      lines.push(
        `  * Flow-Level Error: [${report.flow_error.error_kind}] (code: ${report.flow_error.error_code || 'none'}) ${report.flow_error.message}`,
      );
    }

    const failedReports = (report.node_reports || []).filter((nr) => nr.status === 'failed');
    if (failedReports.length > 0) {
      lines.push(`  * **Failed Nodes Details (CRITICAL - USE TO DIAGNOSE & PROPOSE REPAIRS)**:`);
      for (const nr of failedReports) {
        lines.push(`    - Failed Node: [${nr.node_type}] "${nr.node_name}" (uuid: ${nr.node_uuid}, duration: ${nr.duration_ms}ms):`);
        if (nr.error) {
          lines.push(`      * Error Kind: ${nr.error.error_kind}`);
          if (nr.error.error_code) {
            lines.push(`      * Error Code: ${nr.error.error_code}`);
          }
          lines.push(`      * Error Message: "${nr.error.message}"`);
          if (nr.error.stack_trace) {
            const shortStack = nr.error.stack_trace.split('\n').slice(0, 3).join(' -> ');
            lines.push(`      * Stack Trace: ${shortStack}`);
          }
        } else {
          lines.push(`      * Error: Node failed without specific error message.`);
        }
        if (nr.resolved_config) {
          lines.push(`      * Resolved Config: ${JSON.stringify(nr.resolved_config)}`);
        }
        if (nr.output_data) {
          const outStr = JSON.stringify(nr.output_data);
          lines.push(`      * Output / Response: ${outStr.length > 250 ? outStr.substring(0, 247) + '...' : outStr}`);
        }
      }
    }
  }

  if (executionState.error) {
    lines.push(`- **Execution Engine Error**: ${executionState.error}`);
  }

  // 控制台最近错误
  const errorLogs = consoleState.logs.filter(l => l.level === 'ERROR').slice(-3);
  if (errorLogs.length > 0) {
    lines.push(`- **Recent Console Errors**:`);
    for (const log of errorLogs) {
      lines.push(`  * [${new Date(log.timestamp).toLocaleTimeString()}] ${log.message}`);
    }
  }

  // 6. 系统外观与语言
  lines.push(`- **System Settings**: Theme = ${settingsState.theme}, Locale = ${settingsState.locale ?? 'en-US'}`);

  return lines.join('\n');
}
