# TLS test certificates

These are **self-signed test fixtures**, not production credentials.

They exist so the TLS suite (the `tests/cli/tls_*.json` flows, driven by
`tests/run-tls-tests.mjs`, against `tests/https-test-server.mjs`) can exercise
CA verification, verification skipping, mTLS client auth, and untrusted-issuer
failure paths without network access or a real CA.

| File | Purpose |
|---|---|
| `ca.pem` / `ca.key` | Local CA used to sign the server and client certificates |
| `server.pem` / `server.key` | HTTPS server identity, hostname `localhost` |
| `client.pem` / `client.key` | Client identity for mTLS tests |
| `ca-wrong.pem` / `ca-wrong.key` | A second, unrelated CA for negative tests (untrusted issuer) |
| `openssl.cnf` | OpenSSL config used to generate the above |
| `generate-certs.ps1` | Regenerates the whole set on Windows |

The private keys are committed intentionally and are worthless outside this
test fixture: they are self-signed, valid only from 2026-04-29 to 2027-04-29,
issued for `CN=localhost`, and never used by the application at runtime.
Security scanners that flag committed `.key` files will report these — that is
a false positive.

Do not reuse these certificates for anything else, and do not add real
certificates or keys to this directory.
