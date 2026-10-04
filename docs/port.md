# Caiven Port

Port is a cart sharing server you can host yourself. It runs on Rocket with
a Svelte web UI and handles accounts, cart versions, ratings, comments and
search by tag, author or sort order. Port stores everything in the database,
cart files and screenshots included (`BYTEA` columns), so PostgreSQL is the
only thing you need to provision and back up.

```bash
cd crates/caiven-port
cargo run --release
# or, with PostgreSQL:
docker compose up
```

If you don't set `--database-url` or `DATABASE_URL`, `cargo run` uses a
SQLite file under `--data-dir`, which needs no setup for local work.
`docker compose up` starts a `postgres` service and the server, connected
through `DATABASE_URL`, the same way a real deployment runs.

CI checks `port-v<version>` tags but doesn't publish an image yet. Build it
from the workspace root:

```bash
docker build -f crates/caiven-port/Dockerfile -t caiven-port:<version> .
```

For production setup (TLS proxy, environment variables, backups), see
[development/port-operations.md](development/port-operations.md).

| Flag                                  | Default                       | Description |
| :------------------------------------- | :----------------------------- | :---------- |
| `--address`                           | `0.0.0.0`                     | Listen address |
| `--port`                              | `8080`                        | Listen port |
| `--database-url` (env `DATABASE_URL`) | unset                         | PostgreSQL connection string. When set, Port keeps all data in Postgres. |
| `--data-dir`                          | `data`                        | SQLite folder, used only when `--database-url` is unset |
| `--web-dir`                           | `crates/caiven-port/web/dist` | Built web UI (`npm run build` in `crates/caiven-port/web/`) |
| `--ip-header` (env `CAIVEN_IP_HEADER`) | unset                        | Client IP header that your proxy overwrites, such as `X-Real-IP`. Set it when Port runs behind a proxy. |
| `CAIVEN_OPERATOR_NAME` / `_ADDRESS` / `CAIVEN_CONTACT_EMAIL` | unset | Operator details shown on `/terms` and `/privacy`. Content reports go to the email address. |

Open the base URL in a browser to register. From there you can browse and
search carts by tag, author or sort (new, popular, top), upload carts and new
versions, rate and comment, and view author profiles. The web UI signs you in
with a session cookie. On the Profile page you can also create API tokens
for `caiven-studio publish` or your own scripts; send them in an `X-Api-Key`
header.

## REST API

| Method                | Path                                           | Description |
| :---------------------| :------------------------------------------------| :---------- |
| `POST`                | `/api/v1/auth/register` / `/login` / `/logout` | Account auth (session cookie) |
| `GET`                 | `/api/v1/auth/me`                              | Current user |
| `GET`/`POST`/`DELETE` | `/api/v1/auth/tokens`                          | Manage API tokens |
| `GET`                 | `/api/v1/carts`                                | List and search carts (`page`, `per_page`, `q`, `tag`, `author`, `sort`) |
| `POST`                | `/api/v1/carts`                                | Upload a cart (multipart: `cart` plus JSON `meta`; set `meta.remixable`, and `meta.parent_cart_id` for a remix) |
| `GET`/`PATCH`/`DELETE`| `/api/v1/carts/:id`                            | Cart detail (with `parent`, `remix_count`, `recent_remixes`), edit (including `remixable`), delete (owner or admin) |
| `POST`                | `/api/v1/carts/:id/funnel`                     | Record one remix step (`{event}`), counted once per viewer. |
| `GET`                 | `/api/v1/admin/metrics/remix-funnel?days=7`    | Admin only: remix funnel totals and per-cart rows (`since=`, `include_staff=`) |
| `POST`                | `/api/v1/carts/:id/versions`                   | Upload a new version of a cart you own |
| `GET`                 | `/api/v1/carts/:id/cart` \| `/screenshot`      | Download the cart or screenshot (`?version=n`, latest by default) |
| `PUT`/`DELETE`        | `/api/v1/carts/:id/rating`                     | Rate a cart (1 to 5) |
| `GET`/`POST`/`DELETE` | `/api/v1/carts/:id/comments[/:cid]`            | Comments |
| `GET`                 | `/api/v1/tags` \| `/api/v1/users/:username`    | Tags and user profiles |
| `GET` / `POST`        | `/api/v1/legal` / `/api/v1/reports`            | Operator identity, and content reports (DSA notices) mailed to the operator |
| `DELETE`              | `/api/v1/carts/:id?reason=` (and comments)    | Admin takedown. Port emails the owner a statement of reasons. |

## Web play

Each cart has a Play button on its gallery card and detail page. It opens
`/play/:id`, which runs the game in the browser. The player comes from
`crates/caiven-web`, a WASM build (`wasm32-unknown-emscripten`) of the VM. It
fetches the cart through the REST API and draws to a `<canvas>` at 60 fps.

- Controls: arrows or WASD to move, `J`, `Z` or `Space` for A, `K` or `X`
  for B, `Backspace` for Select. Gamepads work through the Gamepad API, and
  phones get an on-screen d-pad with A and B.
- Audio uses the same synth as the desktop player, driven by a
  `ScriptProcessorNode` instead of SDL2.
- If the cart throws a Lua error, the player stops and shows the message and
  line number over the last frame.
- Browsers block sound until the user interacts with the page, so click the
  canvas or press a key once to start audio.

## Quick Remix

If a cart's owner ticked *Let others remix*, the Play page and the cart
page show a *Remix this* button. `/remix/:id` puts the game next to its Lua
source. You can edit the code, rerun with Ctrl+Enter, and publish your
version as a new cart linked to the original. You need an account only when
you publish.

Remixing starts off for every cart. Turning it off hides the Remix button
only: the downloadable `.cav` still holds the Lua, minified unless the
creator built it with `--no-minify` or published it with `--remixable`.
Home's *Start here* row shows the collection with slug `start-here`.

Port counts 20-second plays and remix steps (page opened, changed version
ran, publish pressed) in its own `funnel_events` table: one row per cart,
step and hashed viewer, with no raw IPs or timings. Admins read the totals
at `/api/v1/admin/metrics/remix-funnel`.

## Rebuilding the WASM player

You need the Emscripten SDK (`emcc` and `emar` on `PATH`). Without a local
install, use Docker from the repository root:

```bash
docker run --rm -v "$(pwd):/work" -w /work emscripten/emsdk:latest \
  bash crates/caiven-web/build-web.sh
```

Then copy `target/wasm32-unknown-emscripten/release/caiven_web.{js,wasm}`
into `crates/caiven-port/web/public/wasm/` and run `npm run build` in
`crates/caiven-port/web/`. The repository includes the built files because
CI has no WASM build step yet.
