#!/usr/bin/env node
/**
 * MPE plugin template (Node.js) — JSON-RPC 2.0 over binary stdio.
 *
 * Copy-able plugin speaking the MPE wire protocol (docs/plugin-protocol.md);
 * provides one node type, `nodedemo:echo`, which echoes the host-resolved
 * config back as `output_data`.
 *
 * Protocol surface:
 *   * `describe`  -> manifest + node descriptions (object shape)
 *   * `execute`   -> {success, next_ports, output_data, errors,
 *                     report_data, variable_updates, provider_updates}
 *   * `validate`  -> {valid, message}
 *   * `cancel`    -> notification, no-op (P0)
 *   * `shutdown`  -> request (answer then exit) or notification (exit)
 *   * unknown method -> JSON-RPC -32601 error
 *
 * Design notes:
 *   * **Binary-safe.** `process.stdin` is consumed as raw Buffers with a
 *     manual LF splitter — no string decoding before framing, so UTF-8
 *     payloads and CRLF terminators both survive. `\r\n` is tolerated.
 *   * **Pipelined.** The single-threaded event loop keeps reading requests
 *     while `execute` handlers run asynchronously; responses carry the
 *     matching id and may arrive out of order. `process.stdout.write` frames
 *     are serialized by the stream itself, so they never interleave.
 *   * **Malformed lines are skipped, never fatal.**
 *   * **Flat `variable_updates` keys only** (locked A10 contract).
 */

'use strict';

const MAX_LINE_BYTES = 2 * 1024 * 1024; // mirrors mpe_plugin_sdk::protocol::MAX_LINE_BYTES
const PLUGIN_NAME = 'nodedemo';
const PLUGIN_VERSION = '0.1.0';

/** The `describe` response: manifest echo + one node type. */
function describeResult() {
  return {
    plugin: {
      name: PLUGIN_NAME,
      version: PLUGIN_VERSION,
      entry: { command: 'node', args: ['plugin.js'] },
      capabilities: { streaming: false },
    },
    nodes: [
      {
        type_id: 'nodedemo:echo',
        display_name: 'NodeDemo Echo',
        category: 'demo',
        ports: [
          { id: 'in', label: 'In', kind: 'in' },
          { id: 'out', label: 'Out', kind: 'out' },
        ],
        default_config: { message: 'hello' },
      },
    ],
  };
}

/** Runs one `execute`: echo the resolved config back to the flow. */
async function handleExecute(params) {
  const config = (params && params.config) || {};
  return {
    success: true,
    next_ports: ['out'],
    output_data: config,
    // Flat keys only (A10): a dotted key would be skipped by the host.
    variable_updates: { nodedemo_echoed: JSON.stringify(config) },
  };
}

/** Validates an `execute` config: `message` must be a string when present. */
function handleValidate(params) {
  const config = (params && params.config) || {};
  if (typeof config !== 'object' || config === null || Array.isArray(config)) {
    return { valid: false, message: 'config must be an object' };
  }
  if (config.message !== undefined && typeof config.message !== 'string') {
    return { valid: false, message: 'config.message must be a string' };
  }
  return { valid: true };
}

/** Parses one line into a JSON value; `null` on malformed input. */
function parseFrame(line) {
  try {
    return JSON.parse(line.toString('utf8'));
  } catch (err) {
    process.stderr.write(`[nodedemo] skipping malformed frame: ${err.message}\n`);
    return null;
  }
}

/** Handles one parsed message. Returns `true` when the loop must exit. */
function dispatch(message) {
  if (message === null || typeof message !== 'object') {
    return false;
  }
  const method = message.method;
  const params = message.params && typeof message.params === 'object' ? message.params : {};
  const hasId = typeof message.id === 'number';

  if (!hasId) {
    // Notification: no response expected.
    if (method === 'cancel') {
      return false; // P0: cancellation is a no-op.
    }
    if (method === 'shutdown') {
      return true;
    }
    process.stderr.write(`[nodedemo] ignoring unknown notification '${method}'\n`);
    return false;
  }

  const respond = (result) =>
    process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id: message.id, result }) + '\n');
  const respondError = (code, text) =>
    process.stdout.write(
      JSON.stringify({ jsonrpc: '2.0', id: message.id, error: { code, message: text } }) + '\n'
    );

  if (method === 'describe') {
    respond(describeResult());
  } else if (method === 'execute') {
    // Pipelined: the handler runs async; the loop keeps reading meanwhile.
    handleExecute(params).then(respond, (err) =>
      respond({ success: false, errors: [String(err && err.message || err)] })
    );
  } else if (method === 'validate') {
    respond(handleValidate(params));
  } else if (method === 'shutdown') {
    respond(null);
    return true;
  } else {
    respondError(-32601, 'Method not found');
  }
  return false;
}

// Binary-safe line reader: split incoming chunks on `\n`, keep the
// remainder as the buffer. Node's readline interface would decode to
// strings first; this path stays byte-exact until the JSON parse.
let buffer = Buffer.alloc(0);
process.stdin.on('data', (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  let nl;
  while ((nl = buffer.indexOf(0x0a)) !== -1) {
    let line = buffer.subarray(0, nl);
    buffer = buffer.subarray(nl + 1);
    if (line.length > 0 && line[line.length - 1] === 0x0d) {
      line = line.subarray(0, line.length - 1); // CRLF tolerance
    }
    if (line.length > MAX_LINE_BYTES) {
      process.stderr.write(`[nodedemo] frame exceeds ${MAX_LINE_BYTES} bytes; skipping\n`);
      continue;
    }
    if (dispatch(parseFrame(line))) {
      // Shutdown: drain pending microtasks (async execute responses) and
      // queued stdout writes first — a synchronous exit would lose them.
      setImmediate(() => process.exit(0));
      return;
    }
  }
});
process.stdin.on('end', () => process.exit(0));
process.stdin.resume();
