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

### 1. MPE Core Architecture & Concepts:
- **DAG Execution Flow**: Nodes execute as a Directed Acyclic Graph. The execution starts at an 'entry' node.
- **Node Structure**:
  - Each node has a unique 'uuid', a 'name', a 'type', 'position' ({x, y}), and type-specific 'data'.
  - Built-in node types:
    * Flow Control: 'entry', 'end', 'conditional', 'proxy'
    * HTTP & REST: 'http' (supports GET, POST, PUT, DELETE, PATCH, headers, body, params)
    * Scripts & Logic: 'script' (JavaScript sandbox runtime)
    * Assertions: 'assertion' (status code, JSONPath, response time, regex, etc.)
    * Data: 'variable_extractor'
    * WebSockets: 'ws_connect', 'ws_send_collect', 'ws_close'
    * TCP/UDP: 'tcp_connect', 'tcp_send', 'tcp_receive', 'tcp_close', 'udp_bind', 'udp_send_to', 'udp_recv_from', 'udp_close'
    * SSE: 'sse_connect', 'sse_listen', 'sse_disconnect'
    * GraphQL: 'graphql_connect', 'graphql_query', 'graphql_subscribe', 'graphql_introspect', 'graphql_disconnect'
    * Plugins: '<proto>:<action>' (e.g. grpc, redis, mysql, kafka, mqtt, etc.)
- **Ports & Routing**:
  - Connections link a source node's port ('source_port_id') to a target node's port ('target_port_id').
  - Default ports are 'out' / 'in' or 'default'.
  - Conditional nodes have two outgoing branch ports: 'true' and 'false'.
  - Nodes support error handling ('on_error' policy): 'RouteToFalse' (default), 'Ignore', or 'AbortFlow'.
- **Variable Interpolation**:
  - Use Jinja2-style syntax in URL, Headers, Body, or Query fields:
    * \`{{context.steps.<node_uuid_or_name>.response.body.<json_path>}}\`
    * \`{{env.<VARIABLE_NAME>}}\`
    * \`{{variables.<key>}}\`

### 2. JavaScript Script Node Sandbox API:
When generating or updating JavaScript for 'script' nodes, write modern ES6+ code utilizing the sandboxed globals:
- \`context.steps.<step_id>.response.body\`: Access preceding node response body (parsed JSON object or string).
- \`context.steps.<step_id>.response.status\`: HTTP status code (number).
- \`context.variables.get(key)\`: Retrieve variable from flow variable pool.
- \`context.variables.set(key, value)\`: Set variable into flow variable pool for downstream steps.
- \`assert(condition, message)\`: Fail step if condition is falsy.
- \`skip(reason)\`: Skip subsequent branches.
- \`abort(reason)\`: Immediately terminate flow execution with error.
- \`console.log(...)\`: Log to MPE execution console.

### 3. Guidelines for Action & Tool Calls:
- You have direct access to tools for modifying the canvas, updating node configurations, generating scripts, managing flow files, inspecting datasources, and running flow executions.
- When the user asks you to create or modify flows, directly invoke the corresponding tools.
- When creating a series of nodes, make sure to connect them logically and call \`auto_layout\` to organize the canvas nicely.
- For high-risk destructive actions (such as clearing the whole canvas, deleting multiple nodes, or installing marketplace plugins), the system will prompt the user for confirmation before executing.
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
        lines.push(
          `    - ${conn.source_node_uuid}:${conn.source_port_id} -> ${conn.target_node_uuid}:${conn.target_port_id}`
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
