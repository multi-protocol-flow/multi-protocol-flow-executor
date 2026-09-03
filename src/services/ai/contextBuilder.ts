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
export function buildSystemPrompt(customSystemPrompt?: string): string {
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
- **Variable Interpolation**:
  * Use Jinja2-style syntax in URL, Headers, Body, or Query fields:
    - \`{{context.steps.<node_uuid_or_name>.response.body.<json_path>}}\`
    - \`{{env.<VARIABLE_NAME>}}\`
    - \`{{variables.<key>}}\`

### 2. Node Configuration Schemas (config object structure):
When calling \`batch_create_pipeline\`, \`add_node\`, or \`update_node_config\`, ensure \`config\` matches these exact structures:
- **'http'**:
  \`\`\`json
  {
    "method": "get" | "post" | "put" | "delete" | "patch", // MUST be lowercase
    "url": "https://api.example.com/endpoint",
    "headers": { "Content-Type": "application/json", "Authorization": "Bearer {{variables.token}}" },
    "query_params": { "page": "1" },
    "body_type": "none" | "body" | "form-data" | "x-www-form-urlencoded" | "binary",
    "body": "{\\"key\\": \\"value\\"}",
    "timeout_ms": 30000
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
        "source": "body.data.token", // JSONPath or field in upstream response
        "target": "auth_token",       // variable name to save into workflow variable pool
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
- **Plugins ('<proto>:<action>')**: \`{ ...specific_plugin_params }\`

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
- **CREATING MULTI-STEP FLOWS**: ALWAYS prefer calling \`batch_create_pipeline\` when creating 2 or more sequential steps. It automatically creates all nodes, applies accurate configurations, wires ports sequentially ('out/true' -> 'in'), and triggers \`auto_layout\` in a single call, preventing multi-turn latency and connection mistakes.
- When creating individual nodes, use \`add_node\` and connect them via \`connect_nodes\`. Remember: source port is 'out' (entry/extractor) or 'true' (http/script/assertion/plugins), and target port is 'in'.
- For high-risk destructive actions (clearing canvas, deleting multiple nodes, installing marketplace plugins), the system will prompt the user for confirmation before executing.
- Be concise, professional, clear, and proactive. Explain what actions you took after calling tools.`
  );

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

    // 列出节点摘要 (uuid, name, type, position)
    if (currentFlow.nodes.length > 0) {
      lines.push(`  * Nodes:`);
      for (const node of currentFlow.nodes) {
        lines.push(
          `    - [${node.type}] "${node.name}" (uuid: ${node.uuid}, pos: {x: ${Math.round(node.position.x)}, y: ${Math.round(node.position.y)}})`
        );
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
    lines.push(`- **Last Execution Report**:`);
    lines.push(`  * Status: ${executionState.report.status}`);
    lines.push(`  * Total Duration: ${executionState.report.duration_ms}ms`);
    const failedNodes = executionState.report.node_reports
      .filter((nr) => nr.status === 'failed')
      .map((nr) => nr.node_name);
    if (failedNodes.length > 0) {
      lines.push(`  * Failed Nodes: ${failedNodes.join(', ')}`);
    }
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
