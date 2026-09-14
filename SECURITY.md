# Security

Caiven processes untrusted cartridges and community content. Security-sensitive
boundaries include cartridge parsing, the Lua sandbox, native filesystem access,
Tauri IPC, account authentication, uploads, and release tooling.

## Reporting a vulnerability

Use GitHub's **Report a vulnerability** action in this repository's Security tab
when available. If private reporting is unavailable, ask the maintainer for a
private channel before sharing exploit details. Do not post credentials, user
data, or a working exploit in a public issue.

Include the affected version or commit, platform, reproduction steps, expected
and actual behavior, and a minimal synthetic sample. Avoid testing against
other people's accounts or public servers without permission.

There is no published response-time SLA or supported-version window yet.
Report the exact version; do not assume an older release receives backports.

## Dependency review

Run `cargo audit` and `npm audit --audit-level=high` in **both** frontend
directories. Do not omit development dependencies: Svelte, Vite, CodeMirror,
and their transforms participate in shipped bundles or build execution.

As of the 2026-09-14 review, the full Rust audit still fails for two existing
exceptions in CI:

| Advisory | Dependency | Existing mitigation and limitation |
| --- | --- | --- |
| RUSTSEC-2026-0235 | rkyv 0.7.46 | Not present in the current host's active dependency graph; retained in the lockfile. Recheck all supported targets/features before relying on this exclusion. |
| RUSTSEC-2026-0258 | h2 0.3.27 through Rocket/hyper | Remains in the active Port graph. Restrict backend network access and configure the TLS proxy to use HTTP/1.1 upstream. This is a deployment mitigation, not a patched dependency. |

CI ignores these two IDs, not all advisories. Maintainership and unsoundness
warnings also remain. Review each exception before every release and whenever
features, dependency versions, or deployment topology change. A passing CI
audit with exceptions is not a clean full audit.

The runtime image excludes local `.env` files and runs as UID/GID 10001.
Production deployment still needs the controls in
[Port operations](docs/development/port-operations.md).
