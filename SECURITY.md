# Security

Caiven runs carts and community content from people you don't know. The
sensitive areas are cart parsing, the Lua sandbox, native file access, Tauri
IPC, account login, uploads and the release tooling.

## Reporting a vulnerability

Use **Report a vulnerability** on this repository's Security tab. If private
reporting isn't available, ask the maintainer for a private channel before
you share exploit details. Don't post credentials, user data or a working
exploit in a public issue.

Include the version or commit, your platform, steps to reproduce, what you
expected and what happened, and a small made-up sample file. Don't test
against other people's accounts or public servers without permission.

The project has no published response time and no list of supported
versions yet. Tell us the exact version you tested, and don't count on fixes
being backported to older releases.

## Dependency review

Run `cargo audit`, and `npm audit --audit-level=high` in both frontend
folders. Include dev dependencies: Svelte, Vite, CodeMirror and their
transforms end up in the shipped bundles or run during the build.

At the 2026-09-14 review, the full Rust audit failed on two known advisories,
which CI ignores:

| Advisory | Dependency | Mitigation and limits |
| --- | --- | --- |
| RUSTSEC-2026-0235 | rkyv 0.7.46 | Absent from the active dependency graph on the current host, but still in the lockfile. Check every supported target and feature set before relying on this. |
| RUSTSEC-2026-0258 | h2 0.3.27, via Rocket and hyper | Present in Port's active graph. Restrict the backend's network access and have the TLS proxy talk HTTP/1.1 to Port. The dependency itself is still unpatched. |

CI ignores only these two IDs. Maintenance and unsoundness warnings still
show up. Recheck both exceptions before each release and whenever features,
dependency versions or the deployment setup change. A CI audit that passes
with exceptions is not a clean audit.

The Port runtime image leaves out local `.env` files and runs as UID/GID
10001. A production deployment still needs the controls listed in
[port-operations.md](docs/development/port-operations.md).
