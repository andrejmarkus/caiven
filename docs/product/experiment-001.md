# Experiment 001 — run record

The protocol is [first-user-experiment.md](first-user-experiment.md). This
page freezes what participants used and holds the checks run before the
first session. Fill in the record; don't edit it after the main phase starts.

## Freeze record

| | |
| --- | --- |
| Git SHA (full) | |
| CI run on that SHA (green, URL) | |
| Image tag | `caiven-port:exp001-<short sha>` |
| Image id (`docker image inspect --format '{{.Id}}'`) | |
| Deployed at (UTC) | |
| Port URL | |
| Database | fresh, created by this image's baseline migration |
| Config checked | `CAIVEN_BASE_URL` · `CAIVEN_SECURE_COOKIES=true` · `CAIVEN_IP_HEADER` · SMTP complete · OAuth pairs · Turnstile |
| Smoke start (RFC 3339) | |
| Smoke set ids (not for participants) | juggle · meteor · hop · chain |
| Experiment set ids | juggle · meteor · hop · chain |
| `since` (RFC 3339, taken after the experiment set is published) | |
| Pilot: dates, n, SHA | |
| Main: start date, n, SHA | |

"The build" always means this SHA + this image id + these starter ids, never
"current master". A fix after the pilot means a new SHA, a new CI run, a new
image, a new experiment set, a new `since` and a new row here.

**Never publish a new version of a starter in the record while its phase
runs.** The server stores a remix's `parent_version` as the parent's latest
version at publish time, so a new starter version would mislabel every remix
made from the old one. Tagging the SHA (`git tag exp001-main <sha>`) is fine
locally; pushing a tag is a remote action and needs an explicit OK.

## Gates

All must hold before the first participant:

1. `CI and Release` green on the frozen SHA, including the Port mocked
   (3×) and live e2e gates.
2. The server runs that SHA: build the image from a clean checkout of it
   (`git status` clean, `git rev-parse HEAD` = the SHA) and record the image
   id.
3. Smoke test below passed, on the smoke set.
4. The readout for the smoke window matches the table below.
5. Email checked in real inboxes (below).
6. Experiment set published after the smoke test, `since` taken after that.

## Deploy (fresh v1 database)

There is no pre-v1 data to keep: start from an empty database. The image
applies the baseline migration on first start.

```sh
git clone https://github.com/andrejmarkus/caiven.git caiven-exp001 && cd caiven-exp001
git checkout <full sha>
git status --short                      # must print nothing
docker build -f crates/caiven-port/Dockerfile -t caiven-port:exp001-<short sha> .
docker image inspect --format '{{.Id}}' caiven-port:exp001-<short sha>   # record

docker network create caiven-exp001
docker volume create caiven-exp001-pg
docker run -d --name caiven-exp001-db --network caiven-exp001 --restart unless-stopped \
  -e POSTGRES_USER=caiven -e POSTGRES_PASSWORD=<secret> -e POSTGRES_DB=caiven \
  -v caiven-exp001-pg:/var/lib/postgresql/data postgres:16-alpine
docker run -d --name caiven-exp001-port --network caiven-exp001 --restart unless-stopped \
  -p 127.0.0.1:8080:8080 --env-file port.env caiven-port:exp001-<short sha>
curl -fsS http://127.0.0.1:8080/readyz
```

`port.env` (never commit it):

```sh
DATABASE_URL=postgres://caiven:<secret>@caiven-exp001-db:5432/caiven
CAIVEN_BASE_URL=https://<port host>
CAIVEN_SECURE_COOKIES=true
CAIVEN_IP_HEADER=X-Real-IP
SMTP_HOST=… SMTP_PORT=… SMTP_USERNAME=… SMTP_PASSWORD=… SMTP_FROM=…
# Only if enabled, each as a complete pair:
# GOOGLE_CLIENT_ID/SECRET, GITHUB_CLIENT_ID/SECRET, DISCORD_CLIENT_ID/SECRET
# TURNSTILE_SITE_KEY/TURNSTILE_SECRET_KEY
```

The TLS proxy forwards to `127.0.0.1:8080` over HTTP/1.1 and overwrites
`X-Real-IP` (nginx `proxy_set_header X-Real-IP $remote_addr;`, Caddy
`header_up X-Real-IP {remote_host}`). Details:
[port-operations.md](../development/port-operations.md).

Then, before anyone else can reach the site: **register the operator account
first** (the first account on a fresh database becomes admin), confirm its
email, and create a full-scope API token on the Profile page. That token runs
the seed and readout scripts. Back up after each phase:
`docker exec caiven-exp001-db pg_dump -U caiven caiven > exp001-<phase>.sql`.

## Starter sets

Run the seed script from the frozen checkout, so the starters are that SHA's
`projects/remix/`.

```sh
# Smoke set: kept out of Home's Start here row.
SKIP_START_HERE=1 CAIVEN_PORT_URL=… CAIVEN_PORT_API_KEY=<admin token> scripts/remix-seeds/publish.sh
# Experiment set, after the smoke test passes.
CAIVEN_PORT_URL=… CAIVEN_PORT_API_KEY=<admin token> scripts/remix-seeds/publish.sh
```

Each prints `name id` lines for the record. Publishing the same starters
again is allowed (same owner), and a participant's remix may be byte-identical
to a smoke remix or another participant's: remixes don't block each other.

## Smoke test

Automated in CI (`crates/caiven-port/web/e2e/live/remix-loop.spec.ts`, real
backend): anonymous play → Remix → real Lua edit → changed picture → Publish
→ register → back to the draft → publish child → second browser plays the
child → remixes it → registers → publishes the grandchild → lineage
root → child → grandchild checked through the API → funnel readout equal to
the session. Mocked e2e (`e2e/mock/remix.spec.ts`) covers the error paths:
syntax, runtime, endless loop and top-level loop errors recovering in place,
no blank frame on rerun, dropped connection and expired session at Publish,
double-clicked Publish, stale and reused email links, unsafe `next`.

Manual, on the deployed Port, from a network no participant will use:

| # | Step | Expect |
| --- | --- | --- |
| 1 | Fresh private window, open `/play/<smoke juggle>` | Game running within a few seconds |
| 2 | Press ← → for 25 s | Game responds; nothing else |
| 3 | **Remix this** | Code editor with real Lua, six chips with **try N** |
| 4 | **try 190** on `PADDLE_WIDTH` | Paddle visibly wider within a second |
| 5 | Type `local BALLS = ` and stop | Red line + plain hint; old version keeps running |
| 6 | Finish it: `local BALLS = 5` | Error gone, five balls |
| 7 | Change `GRAVITY` by hand | Visible change |
| 7a | **Stop**, change `BALL_SIZE` (or any chip) | Game stays stopped, edit kept |
| 7b | **Run** | Game resumes with the new value |
| 8 | Publish | Register page, "Your remix is saved" |
| 9 | Register a new address you can read | Back on the remix, edit intact, Publish form with the email notice |
| 10 | Press **Send it again**, then open the *first* email's link in the same browser | Returns to the Publish form, notice gone |
| 11 | Publish | "Published", share link |
| 12 | Open the link on a phone (other account, logged out) | Child plays, lineage line names the starter |
| 13 | Remix the child there, change it, publish with a second new account | Grandchild published |
| 14 | `/cart/<grandchild>` | "Remixed from" the child; child credits the starter |
| 15 | `scripts/experiment/readout.sh <smoke start> <smoke ids>` | Matches the table below; compare each number to what you did |

Expected readout for the window of steps 1–14, by cart:

| | smoke Juggle | child |
| --- | --- | --- |
| play started | 1 | 1 |
| qualified play | 1 | 1 if played 20 s on `/play`, else 0 |
| remix opened | 1 | 1 |
| changed run ok | 1 | 1 |
| publish pressed | 1 | 1 |
| remixes published | 1 | 1 |
| …played by another | 1 if the phone played the child 20 s | 0 |
| …remixed again | 1 | 0 |

A difference means a measurement bug or a shared network. Resolve it before
publishing the experiment set.

## Email (only real inboxes prove this)

Covered by tests: tokens are single-use and expire; resend keeps earlier
links valid (reset links stay newest-only); a used or old link for a
confirmed account carries on to Publish; a dead link offers the way back to
the saved remix. Read in code, not tested: links use `CAIVEN_BASE_URL`,
tokens are stored hashed, resend is limited to 3 per hour per IP, and an
SMTP failure is logged while registration still succeeds. Registration waits
for the SMTP send, so a slow relay shows as a slow Create account button.

Manual, before the pilot (record date and result):

| Check | Gmail | Outlook / Hotmail | One more (iCloud, Proton, school) |
| --- | --- | --- | --- |
| Arrives, minutes | | | |
| Inbox or spam | | | |
| Sender and subject look legitimate | | | |
| Link opens the right Port origin over HTTPS | | | |
| Link on the phone mail app returns to Publish (or says to go back to the tab) | | | |
| "Send it again" → first email's link still works | | | |

If any provider lands in spam, fix SPF/DKIM/DMARC for the sending domain
before the pilot. Don't tell participants to check spam unless it happens.

## Still manual in production

- Email deliverability and spam placement (above).
- OAuth with the real providers, if enabled: new account and existing
  account, both returning to `/remix/<id>?publish=1`.
- The proxy really sets `X-Real-IP`: two phones on different networks must
  show as two viewers in the readout.
- Real phones (below). Playwright's Pixel 7 project checks the layout, the
  editor and the buttons; it doesn't prove a soft keyboard works.
- Public deployment behaviour: TLS, cookies marked `Secure`, link previews in
  one chat app.

## Real phones (manual)

On the deployed Port, one smoke starter, logged out. Record pass / fail and
the device + browser version.

| Check | Android Chrome | iPhone Safari |
| --- | --- | --- |
| Game plays with touch controls | | |
| Remix: code visible, page scrolls, editor scrolls inside itself | | |
| Tap a **try N**: game changes | | |
| Tap into a number, soft keyboard opens, change it | | |
| **Run** reruns with the edit | | |
| Switch to the mail app / another tab and back: edit and game still there | | |
| Publish → register → email link on the phone → back to Publish | | |

## Later — requires evidence

Considered during the readiness audit and deliberately not built. Revisit only
if the sessions point at them.

- A recorded "second edit" event. The notes capture it with its context.
- Joining the anonymous and signed-up keys of one person. Needs a first-party
  cookie id and a privacy decision.
- Per-window dedup instead of forever, if fresh starter sets become a burden.
- Showing the running build SHA on `/healthz`.
- `parent_version` from the version the draft was opened from, not the
  parent's latest. Post-experiment correctness; frozen starters cover it now.
