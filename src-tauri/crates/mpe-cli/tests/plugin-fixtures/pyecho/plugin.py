#!/usr/bin/env python3
"""MPE plugin template (Python) — JSON-RPC 2.0 over binary stdio.

This is a complete, copy-able plugin: it speaks the MPE wire protocol
(see `docs/plugin-protocol.md`) and provides one node type, ``pyecho:add``,
which adds two numbers from its config and returns the sum.

Protocol surface implemented:
  * ``describe``  -> manifest + node descriptions (object shape)
  * ``execute``   -> ``{success, next_ports, output_data, errors,
                      report_data, variable_updates, provider_updates}``
  * ``validate``  -> ``{valid, message}``
  * ``cancel``    -> notification, no-op (P0)
  * ``shutdown``  -> request (answer then exit) or notification (exit)
  * unknown method -> JSON-RPC -32601 error

Design notes:
  * **Binary stdio.** ``sys.stdin.buffer`` / ``sys.stdout.buffer``: Python
    text stdio translates line endings and encoding on Windows. The host also
    injects ``PYTHONUTF8=1`` / ``PYTHONIOENCODING=utf-8`` at spawn; we force
    it here too so the template works when run by hand.
  * **LF framing, CRLF tolerated.** A trailing ``\\r`` before ``\\n`` is
    stripped (Windows ``print()`` style).
  * **Pipelined.** Requests are processed as they arrive; each ``execute``
    runs in its own thread so a slow handler never blocks reading the next
    request. Responses carry the matching id and may arrive out of order.
    A global write lock keeps frames from interleaving.
  * **Malformed lines are skipped, never fatal.**
  * **Flat ``variable_updates`` keys only** (locked A10 contract): the host
    warns + skips any key containing a dot.
"""

import json
import os
import sys
import threading

# Line cap mirrored from the host framing (mpe_protocol::MAX_LINE_BYTES).
MAX_LINE_BYTES = 2 * 1024 * 1024

PLUGIN_NAME = "pyecho"
PLUGIN_VERSION = "0.1.0"


def describe_result():
    """The ``describe`` response: manifest echo + one node type."""
    return {
        "plugin": {
            "name": PLUGIN_NAME,
            "version": PLUGIN_VERSION,
            "entry": {"command": "python", "args": ["plugin.py"]},
            "capabilities": {"streaming": False},
        },
        "nodes": [
            {
                "type_id": "pyecho:add",
                "display_name": "PyEcho Add",
                "category": "demo",
                "ports": [
                    {"id": "in", "label": "In", "kind": "in"},
                    {"id": "out", "label": "Out", "kind": "out"},
                ],
                "default_config": {"a": 0, "b": 0},
                "config_schema": {
                    "type": "object",
                    "properties": {
                        "a": {"type": "number", "default": 0},
                        "b": {"type": "number", "default": 0},
                    },
                },
            }
        ],
    }


def number_or_zero(config, key):
    """Reads a number from ``config``, tolerating strings and missing keys."""
    value = config.get(key)
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return 0
    return value


def handle_execute(params):
    """Computes the node output for one ``execute`` request."""
    config = params.get("config") or {}
    a = number_or_zero(config, "a")
    b = number_or_zero(config, "b")
    total = a + b
    return {
        "success": True,
        "next_ports": ["out"],
        "output_data": {"sum": total, "a": a, "b": b},
        # Flat keys only (A10): a dotted key would be skipped by the host.
        "variable_updates": {"pyecho_last_sum": total},
    }


def handle_validate(params):
    """Validates an ``execute`` config: both operands must be numbers."""
    config = params.get("config") or {}
    if not isinstance(config, dict):
        return {"valid": False, "message": "config must be an object"}
    for key in ("a", "b"):
        value = config.get(key)
        if value is None:
            continue  # missing operand defaults to 0
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            return {
                "valid": False,
                "message": "config.%s must be a number" % key,
            }
    return {"valid": True}


class PluginServer:
    """Reads LF-delimited requests from stdin and dispatches them."""

    def __init__(self):
        # os.device_encoding can be None on redirected pipes; force UTF-8.
        os.environ.setdefault("PYTHONUTF8", "1")
        os.environ.setdefault("PYTHONIOENCODING", "utf-8")
        self.stdin = sys.stdin.buffer
        self.stdout = sys.stdout.buffer
        self.write_lock = threading.Lock()

    # -- framing ---------------------------------------------------------

    def read_line(self):
        """Reads one LF-terminated line (CRLF tolerated); ``None`` on EOF."""
        line = self.stdin.readline(MAX_LINE_BYTES + 1)
        if not line:
            return None
        if line.endswith(b"\r\n"):
            line = line[:-2]
        elif line.endswith(b"\n"):
            line = line[:-1]
        if len(line) > MAX_LINE_BYTES:
            self.log("frame exceeds %d bytes; skipping" % MAX_LINE_BYTES)
            return b""
        return line

    def write_frame(self, payload):
        """Writes one JSON-RPC frame atomically (payload + bare LF)."""
        data = json.dumps(payload, separators=(",", ":")).encode("utf-8") + b"\n"
        with self.write_lock:
            self.stdout.write(data)
            self.stdout.flush()

    def respond(self, request_id, result):
        self.write_frame({"jsonrpc": "2.0", "id": request_id, "result": result})

    def respond_error(self, request_id, code, message):
        self.write_frame(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": code, "message": message},
            }
        )

    def log(self, message):
        """Plugin-side stderr log (drained by the host at spawn)."""
        try:
            sys.stderr.write("[pyecho] %s\n" % message)
            sys.stderr.flush()
        except Exception:  # pragma: no cover - stderr must never kill us
            pass

    # -- dispatch --------------------------------------------------------

    def dispatch(self, message):
        """Handles one parsed request. Returns True when the loop must exit."""
        request_id = message.get("id")
        method = message.get("method")
        params = message.get("params") if isinstance(message.get("params"), dict) else {}

        if "id" not in message or request_id is None:
            # Notification: no response expected.
            if method == "cancel":
                return False  # P0: cancellation is a no-op.
            if method == "shutdown":
                return True
            self.log("ignoring unknown notification '%s'" % method)
            return False

        if method == "describe":
            self.respond(request_id, describe_result())
        elif method == "execute":
            # Pipelined: each execute runs on its own thread; the response
            # carries the matching id, so ordering does not matter.
            threading.Thread(
                target=self.respond, args=(request_id, handle_execute(params)), daemon=True
            ).start()
        elif method == "validate":
            self.respond(request_id, handle_validate(params))
        elif method == "shutdown":
            self.respond(request_id, None)
            return True
        else:
            self.respond_error(request_id, -32601, "Method not found")
        return False

    def run(self):
        """The protocol loop; returns the process exit code."""
        while True:
            line = self.read_line()
            if line is None:
                return 0  # clean EOF (host closed stdin after shutdown)
            if not line:
                continue  # over-long line already logged
            try:
                message = json.loads(line.decode("utf-8"))
            except (ValueError, UnicodeDecodeError) as err:
                self.log("skipping malformed frame: %s" % err)
                continue
            if self.dispatch(message):
                return 0


def main():
    try:
        server = PluginServer()
        sys.exit(server.run())
    except KeyboardInterrupt:  # pragma: no cover
        sys.exit(0)


if __name__ == "__main__":
    main()
