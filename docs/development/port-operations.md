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
6. Behind a proxy, set `CAIVEN_IP_HEADER=X-Real-IP` and have the proxy
   overwrite that header with the client address (nginx
   `proxy_set_header X-Real-IP $remote_addr;`, Caddy
   `header_up X-Real-IP {remote_host}`). Without it every visitor has the
   proxy's IP: registration's 5-per-hour limit becomes site-wide and
   anonymous plays and funnel steps collapse into one viewer. Set it only when
   Port is unreachable except through that proxy, or clients choose their IP.
7. Set `CAIVEN_OPERATOR_NAME` and `CAIVEN_CONTACT_EMAIL`. They fill `/terms`
   and `/privacy`, and content reports (`/report`) are mailed to the contact
   address. Reports fail with 500 until it and SMTP are set.
   `CAIVEN_OPERATOR_ADDRESS` is optional; left unset, the pages omit it.

## Legal launch checklist

Port ships Terms, Privacy and a report form (GDPR, EU DSA Art. 11–17, US DMCA,
COPPA). These steps happen outside the code:

| Step | Why |
| --- | --- |
| Sign the DPA of your hosting and SMTP provider; prefer EU hosting | GDPR Art. 28; the privacy page says "run from the EU" |
| Optional: register a DMCA designated agent at copyright.gov/dmca-directory (USD 6, renew every 3 years) | Only US safe harbour depends on it |
| Rotate proxy access logs within 14 days and backups within 30 days | Periods promised on `/privacy` |
| Keep a short record of processing activities (the `/privacy` table is the source) | GDPR Art. 30 |
| On a data breach, notify the Slovak authority (ÚOOÚ) within 72 hours; email affected users if the risk is high | GDPR Art. 33–34 |
| Answer report and appeal emails, and write each decision down | DSA Art. 16–17; the `moderation_actions` table keeps 3 years |
| Report threats to life or safety you learn of to the police | DSA Art. 18 |
| Changing a retention period: change `src/retention.rs` and `Privacy.svelte` together, and email users 30 days ahead for material Terms changes | The texts promise it |

Get the texts reviewed by a lawyer before they carry real traffic; they are a
grounded starting point, not legal advice.

The image runs as UID/GID `10001:10001`, with `/app/data` writable for fallback
SQLite storage. Mount persistent storage there for SQLite; existing bind mounts
must grant that UID write access. PostgreSQL deployments use `DATABASE_URL`.
The application and SPA files remain root-owned and read-only to the process.

## Backup and recovery

For PostgreSQL, use database-native consistent backups and test restoration into
an isolated database. Cartridges and screenshots are stored in the database.
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
