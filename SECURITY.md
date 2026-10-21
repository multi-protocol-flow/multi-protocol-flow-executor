# Security Policy

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Use GitHub's private vulnerability reporting on this repository:
**Security** → **Report a vulnerability**.

If you cannot use that channel, email the maintainers at
`security@mpe.run` and include:

- affected version (`mpe --version`, or the installer filename)
- platform and OS version
- a description of the issue and its impact
- reproduction steps or a proof of concept
- any suggested fix

You can expect an acknowledgement within a few days. Please give us a
reasonable window to ship a fix before publishing details.

## Supported versions

Only the latest published beta receives fixes. The project is pre-1.0 and does
not maintain long-lived release branches.

## Scope

In scope:

- the execution engine (`flow-engine-*` crates)
- the CLI (`mpe`, `mpe-cli`)
- the desktop application and its IPC surface
- the plugin host, including sidecar process isolation and the sandboxed
  plugin frontend bridge
- the update mechanism and release artifacts

Out of scope:

- vulnerabilities in third-party plugins (report them to the plugin's own
  repository; the plugin list is in [`CONTRIBUTING.md`](./CONTRIBUTING.md))
- issues that require an already-compromised host or a malicious flow file
  that the user was explicitly warned about
- the self-signed certificates under `tests/certs/` — these are test fixtures
  and are intentionally public

## Known limitations

These are accepted risks, not undisclosed vulnerabilities. Please do not report
them as new findings:

1. **Plugin trust model.** Plugins run as separate OS processes with the same
   privileges as the host. `plugin.json` declares permissions, but there is no
   OS-level sandbox — installing a plugin is equivalent to running its binary.
   Only install plugins you trust. The *frontend* half of a plugin (config
   panel, report viewer) does run in a sandboxed iframe with an opaque origin
   and a whitelisted `requestAction` bridge.

2. **Update artifacts are not signature-verified.** Release downloads are
   fetched over HTTPS and checked for a well-formed installer, but the
   application does not verify a cryptographic signature before launching the
   installer. A compromised download host could substitute a payload. If this
   matters for your threat model, build from source.

3. **Plugin registry assets are checksum-only.** The marketplace registry
   publishes `sha256` per platform asset and the installer refuses a mismatch,
   but the registry itself is not signed.

4. **Flow files are executable.** A `.mpf` flow can run scripts, shell-adjacent
   actions through the terminal tool exposed to the AI Copilot, and arbitrary
   network requests. Treat an untrusted flow file like untrusted code.

5. **AI Copilot tool execution.** The Copilot can mutate flows, run them, and
   invoke a whitelisted terminal tool. High-risk actions prompt for
   confirmation by default; "Full Trust" mode disables those prompts and should
   only be enabled deliberately.
