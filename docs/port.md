# Caiven Port (Cart Sharing Server)

Self-hostable cart gallery server: Rocket + Svelte web UI. Accounts, cart
versioning, ratings & comments, and tag/author/sort discovery. Everything —
including cart files and screenshots — is stored in the database (`BYTEA`),
so a PostgreSQL instance is the only stateful thing to provision or back up.

```bash
cd crates/caiven-port
cargo run --release
# or, for a real PostgreSQL-backed deploy:
docker compose up
```

Without `--database-url`/`DATABASE_URL` set, `cargo run` falls back to an
on-disk SQLite database under `--data-dir` — zero-setup for local dev.
`docker compose up` runs the real deploy path: a `postgres` service plus the
server, wired together via `DATABASE_URL`.

Published images (built from tagged `port-v<version>` releases, see
[docs/releasing.md](releasing.md)) are at
`ghcr.io/andrejmarkus/caiven-port` — pull a pinned version or `:latest`
instead of building from source:

```bash
docker pull ghcr.io/andrejmarkus/caiven-port:latest
```

| Flag                                  | Default                       | Description                                                                         |
| :------------------------------------- | :----------------------------- | :-------------------------------------------------------------------------------------|
| `--address`                           | `0.0.0.0`                     | Listen address                                                                      |
| `--port`                              | `8080`                        | Listen port                                                                         |
| `--database-url` (env `DATABASE_URL`) | unset                         | PostgreSQL connection string. When set, carts/screenshots/all data live in Postgres |
| `--data-dir`                          | `data`                        | Fallback SQLite database directory, used only when `--database-url` is unset        |
| `--web-dir`                           | `crates/caiven-port/web/dist` | Built SPA directory (`npm run build` output in `crates/caiven-port/web/`)           |

Open the base URL in a browser to register an account, browse/search/filter
carts by tag, author or sort (new/popular/top), upload new carts or versions,
rate and comment, and view author profile pages. The web UI uses a session
cookie; the same account can also mint per-user API tokens (Profile page) for
`caiven-studio publish` or direct API calls — sent as an `X-Api-Key` header.

| Method                | Path                                           | Description                                                          |
| :---------------------| :------------------------------------------------| :------------------------------------------------------------------- |
| `POST`                | `/api/v2/auth/register` / `/login` / `/logout` | Account auth (session cookie)                                        |
| `GET`                 | `/api/v2/auth/me`                              | Current user                                                         |
| `GET`/`POST`/`DELETE` | `/api/v2/auth/tokens`                          | Manage per-user API tokens                                           |
| `GET`                 | `/api/v2/carts`                                | List/search carts (`page`, `per_page`, `q`, `tag`, `author`, `sort`) |
| `POST`                | `/api/v2/carts`                                | Upload new cart (multipart: `cart` + JSON `meta`)                    |
| `GET`/`DELETE`        | `/api/v2/carts/:id`                            | Cart detail / delete (owner or admin)                                |
| `POST`                | `/api/v2/carts/:id/versions`                   | Upload a new version of an owned cart                                |
| `GET`                 | `/api/v2/carts/:id/cart` \| `/screenshot`      | Download cart/screenshot (`?version=n`, defaults to latest)          |
| `PUT`/`DELETE`        | `/api/v2/carts/:id/rating`                     | Rate a cart (1-5)                                                    |
| `GET`/`POST`/`DELETE` | `/api/v2/carts/:id/comments[/:cid]`            | Comments                                                             |
| `GET`                 | `/api/v2/tags` \| `/api/v2/users/:username`    | Discovery                                                            |

Legacy `/api/carts*` routes (v1 shape, single cart file per cart) remain for
backward compatibility — `caiven-studio publish` still targets them internally.

## Web Play

Every cart on the hub has a **Play** button (gallery card and detail page) that
opens `/play/:id` — a zero-install browser build of the runtime, no download
required. Backed by `crates/caiven-web`, a WASM (`wasm32-unknown-emscripten`)
build of the VM that fetches the cart over the same REST API and renders to a
`<canvas>` at 60fps.

- **Controls:** arrows/WASD to move, `J`/`Z` = A, `K`/`X` = B, standard
  Gamepad API support, and an on-screen touch d-pad + A/B on coarse-pointer
  (mobile) viewports.
- **Audio:** the same square/noise synth used natively, driven by a
  `ScriptProcessorNode` instead of SDL2.
- **Crash handling:** a Lua runtime error stops the cart and shows the error
  and line number over the last frame, instead of hanging silently.
- Click the canvas or press a key once to start audio — browsers require a
  user gesture before playing sound.

Rebuilding `caiven-web` requires the Emscripten SDK (`emcc`/`emar` on `PATH`).
A throwaway Docker recipe (run from the repo root):

```bash
docker run --rm -v "$(pwd):/work" -w /work emscripten/emsdk:latest \
  bash crates/caiven-web/build-web.sh
```

Then copy `target/wasm32-unknown-emscripten/release/caiven_web.{js,wasm}`
into `crates/caiven-port/web/public/wasm/` and `npm run build` in
`crates/caiven-port/web/` — the built artifact ships with the repo since
there's no CI wasm pipeline yet.
