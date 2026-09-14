# Operating Caiven Port

The bundled Compose file is a local development setup with example database
credentials. Production infrastructure must provide its own secrets, TLS proxy,
backups, and monitoring. Pin the deployed image by version and preferably digest;
record the previous image before upgrading.

## Process and readiness checks

| Endpoint | Success | Failure | Purpose |
| --- | --- | --- | --- |
| `GET /healthz` | `200 {"status":"ok"}` | Process unavailable | Supervisor liveness |
| `GET /readyz` | `200 {"status":"ok"}` | `503 {"status":"unavailable"}` | Database reachability |

Both endpoints are unauthenticated and return `Cache-Control: no-store`.
Readiness waits at most two seconds for the database. It does not verify SMTP,
OAuth providers, free disk space, schema correctness, or backup integrity.
Database outages should remove the instance from traffic; they should not
trigger endless restarts of an otherwise live process.

The Docker healthcheck calls `/readyz` every 30 seconds after a startup grace
period. Docker reports health status; automatic remediation depends on the
orchestrator. Startup applies migrations before serving requests.

## Deployment configuration

1. Restrict direct access to Port. Terminate TLS at the reverse proxy and use
   HTTP/1.1 between the proxy and Port while the documented `h2` advisory remains.
2. Set `CAIVEN_BASE_URL` to the public HTTPS origin and enable
   `CAIVEN_SECURE_COOKIES=true`. Verify session cookies carry `Secure` through
   the deployed proxy.
3. Provide real database credentials and the complete SMTP configuration.
   Incomplete SMTP configuration logs verification/reset links as a development
   fallback; do not expose those logs or use that fallback in production.
4. Configure OAuth and Turnstile pairs completely when enabled. Verify a real
   sign-in, email verification, reset, upload, download, and browser Play flow.
5. Preserve authentication headers and cookies at the proxy. Apply request body,
   connection, and timeout limits there. Rate limits in Port are process-local;
   multiple replicas need an edge-level policy.

The image runs as UID/GID `10001:10001`, with `/app/data` writable for fallback
SQLite storage. Mount persistent storage there for SQLite; existing bind mounts
must grant that UID write access. PostgreSQL deployments use `DATABASE_URL`.
The application and SPA files remain root-owned and read-only to the process.

## Backup and recovery

For PostgreSQL, use database-native consistent backups and test restoration into
an isolated database. Cartridges and screenshots are stored in the database;
retain legacy data directories if upgrading an older path-backed installation.
For SQLite, use the SQLite backup API or stop the service before copying the
complete data directory. Copying only a live `port.db` can omit WAL data.

Before an upgrade, take a restorable backup and record the image digest and
migration state. Restore-test it, then upgrade a staging instance and exercise
the critical workflows. Startup migrations may make a binary-only rollback
unsafe. If rollback needs database restoration, stop writes first and restore
the matching database backup and application image together.

Choose recovery-time and recovery-point targets for the actual deployment;
measure restore duration and acceptable data loss. This repository does not
claim an availability SLA or a tested production load limit.

## Incident triage

- Liveness fails: inspect process exit, startup migration failures, configuration,
  and container logs.
- Liveness passes but readiness fails: inspect database availability, credentials,
  connection saturation, and network access before restarting the application.
- Both pass but users fail: inspect affected route/status, proxy behavior, SMTP
  or OAuth dependency, and browser errors. Readiness is deliberately narrower
  than a complete user journey.

Keep logs access-controlled. Never attach session tokens, passwords, database
URLs, email links, or real user content to public issues. Record incident cause,
recovery steps, and a regression test or operational check that catches recurrence.
