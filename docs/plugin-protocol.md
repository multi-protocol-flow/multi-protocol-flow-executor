# MPE Plugin Wire Protocol — Specification (Scaffold)

> **Status:** scaffold — the normative details are filled in M1 (work item
> M1.7). This document is the source of truth for the JSON-RPC 2.0 stdio
> transport between the MPE host (`flow-engine-plugin`) and sidecar plugins
> (`mpe-plugin-sdk` and language templates). The wire types live in
> `mpe-protocol` so host and SDK cannot drift.
>
> Locked decisions this spec must honor: batch arrays forbidden by
> construction; `variable_updates` flat-top-level keys only (A10);
> event notifications carry `execution_id` + `node_instance_id`.

## 1. Framing

- TODO(M1): single JSON value per line, `LF` delimiter; reader tolerates
  trailing `CR` (`CRLF`, Python `print()`); byte-exact UTF-8 validation;
  partial-line buffering until delimiter or EOF half-close; max line
  `2 MiB` (`mpe-protocol::MAX_LINE_BYTES`).

## 2. Notification demux

- TODO(M1): id-less notifications (including `log` and event streams) are
  demuxed to registered handlers BEFORE pending-request lookup; a
  notification must never be mistaken for a response.

## 3. Error-object mapping

- TODO(M1): JSON-RPC error objects (`JsonRpcError`) map to typed
  host `PluginError` variants; standard codes (parse/invalid-request/
  method-not-found/invalid-params/internal/server) get canonical
  host-side translations.

## 4. `variable_updates` contract

- TODO(M1): keys MUST be flat top-level variable names (no dots) — the host
  `VariableStorage` is flat/exact-name-keyed; dotted keys are rejected with
  a host warn + skip (non-fatal) and recorded in the report (locked
  deviation A10).

## 5. Event correlation fields

- TODO(M1): every event/log notification carries `execution_id` +
  `node_instance_id` so streams correlate to a single node execution;
  frozen in the schema day one.

## 6. P1 frontend loading (CSP foundation)

- Locked: P0 ships NO iframe. P1 default (OD3): `mpe-plugin://` custom Tauri
  protocol with its own origin — no blob, no CSP intersection. Fallback:
  blob URL + `frame-src blob:; script-src blob:`.
- CSP staging note (M0.5): `src-tauri/tauri.conf.json` **cannot carry a
  staging comment** — the Tauri config parser rejects unknown keys
  (empirically tested: `$comment` fails with "unknown field"), and strict
  JSON forbids `//` comments. The P1 CSP change (`frame-src` /
  `connect-src` additions) must therefore land as a real, behavior-changing
  edit together with the P1 protocol registration — tracked here, not in
  tauri.conf.json.
