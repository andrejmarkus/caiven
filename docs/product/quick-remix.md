# Quick Remix — first vertical slice

Recorded 2026-09-23. Product direction set by the project owner: Caiven is a
network of tiny playable programs, and the shortest path through it is
**play → remix → change → publish → get remixed → return**. Studio stays the
advanced creation environment. Quick Remix is the small browser surface that
tests whether that loop is compelling.

The question it answers: after playing a small Caiven game, can someone go
from "I played this" to "I changed this and now it is mine" with almost no
friction?

## What ships

| Step | Where | How |
| --- | --- | --- |
| Play | `/play/:id` | Unchanged: WASM runtime, no account. **Remix this** shows when the cart is remixable. |
| Remix | `/remix/:id` | GAME \| CODE. The cart's real `LuaSource` section, pulled from the same `.cav` the player downloads. |
| Change | same page | A textarea editor with line numbers. **Change one thing** chips list top-level numeric constants (`local SPEED = 2`), and editing a chip rewrites that Lua line. |
| See result | same page | Run / Ctrl+Enter rebuilds the `.cav` in the browser (`src/lib/cav.js`) and restarts it in the same WASM module (`CartPlayer.reload`). |
| Errors | same page | The real Lua message, the line number, a highlighted line, a one-line plain-language hint for common errors (`src/lib/remix.js`), and the edit is kept. On a load error the last working build keeps running. |
| Publish | same page | Enabled only after a changed version has run cleanly. Creates a **new cart** with a structured parent link, uploads the current frame as its screenshot, and shows a share URL. |
| Lineage | `/cart/:id` | "Remixed from *X* by @y", "N remixes", and a list of recent remixes. |

No WASM rebuild, no cart-format change and no hardware change: the reload
path uses the existing `caiven_new` + `caiven_load_cart` exports.

## Decisions

### Source availability: opt-in per cart

`CREATOR_RIGHTS.md` promises creators need not publish source. So
`carts.remixable` defaults to **false**, and every existing cart stays closed.
Owners opt in with a checkbox on Upload or on their cart page. The server
rejects a remix whose parent is not remixable (403), so the rule doesn't
depend on the UI.

Caveat: the `.cav` download already contains the Lua (minified by default),
so "not remixable" means *no remix affordance and no attributed remix*. It
does not mean the source is secret. That matches the policy before this
change.

A remix publish defaults to remixable (**Let others remix my version**,
checked). The remixer can uncheck it.

### Minified source

Studio's `publish` and `build` minify Lua by default (comments and
indentation stripped, line numbers kept). A remixable cart published that
way is real Lua but unpleasant to read. `caiven-studio publish --remixable`
marks the cart remixable and skips minification. The Upload page tells
creators who upload a `.cav` to build it with `--no-minify`. Follow-up: the
same choice in Studio's publish dialog.

### Starter carts

Every existing cart is closed by default, so the loop needs carts that are
open from day one. `projects/remix/` holds four: Juggle, Meteor, Hop and
Chain. They're built to hook in the first second: instant action, screen
shake, particles, sound and music, a speed ramp and a one-button restart.
Each is one file with no sprites. Its top six lines are the constants that
show up as **Change one thing** chips, and each carries a `-- try N` hint
with a wild value, because an absurd first change is the most shareable.

A test plays each starter with scripted input: 15 s as shipped, then 10 s
with every `try` value applied at once. The audio (`sfx.hex`, `music.hex`)
is shared: six effects and two looping tracks. To publish, run
`scripts/remix-seeds/publish.sh` with `CAIVEN_PORT_URL` and
`CAIVEN_PORT_API_KEY` set. Each starter is tagged `remix-starter`.

### Multiple source files

A `.cav` holds exactly one `LuaSource` section. Multi-file projects are
bundled into it (`caiven_cart::bundle_lua`). Quick Remix shows and edits that
single chunk, with no file tree. For a bundled cart the modules appear as
long-string `package.preload` entries. That's ugly but correct and
round-trips. Seed carts should be single-file.

### Lineage model

Columns on `carts`, written once at create time and never updated
(`m20260923_000018_remix_lineage`):

- `parent_cart_id`: the cart that was remixed.
- `parent_version`: the parent version remixed from (its latest at publish).
- `root_cart_id`: the original of the chain. It's stored rather than walked
  because it never changes, and it makes "from this cartridge" and challenge
  queries one indexed lookup on both SQLite and Postgres.
- Depth and child count are derived, not stored.

`CartPatch` cannot change lineage. New versions of a child leave it alone.
Deleting a parent leaves `parent_cart_id` set, and the detail page then says
"Remixed from a cart that was removed". There's no FK cascade, on purpose.

A remix byte-identical to its parent's latest version is rejected with a
plain message. The existing cross-owner content-hash guard still applies.

### Auth transition

Play, remix, edit and run need no account. The account wall appears only
when the person presses Publish, after a changed version has run. The draft
(source, title, description, remixable) autosaves to `localStorage` under
`caiven:remix-draft:<cart id>`. Login and Register carry `?next=`
(same-origin only, see `safeNext`) back to `/remix/:id?publish=1`, which
restores the draft, reruns it and opens the publish form.

Known gaps: OAuth sign-in drops `next` (the draft still restores when the
person comes back to the remix URL). Accounts with an unverified email can't
publish (`VerifiedUser`), and the page says so while keeping the draft.

### Measurement

No SDK, no third party, nothing beyond counts. Table `funnel_events`, one row
per **(cart, step, viewer)**, deduped by a unique index:

| Step | Recorded by | Cart |
| --- | --- | --- |
| `qualified_play` | Play page after 20 s of play without a fault | played cart |
| `remix_opened` | Remix page loaded | parent |
| `remix_ran` | First successful run of a changed source | parent |
| `publish_started` | Publish pressed (before any auth wall) | parent |

Derived, never client-claimed: play started (`play_events`), publish
succeeded (`carts.parent_cart_id`), played by another viewer (a
`qualified_play` whose viewer key isn't the owner's), and remixed (child
rows).

`viewer_key` is the same unsalted SHA-256 used by `play_events`, of
`user:<id>` or `ip:<addr>`. No raw IPs, timestamps finer than the event row,
paths or durations are stored. Anonymous viewers behind one IP collapse into
one key, and an owner playing their own cart while logged out counts as
external. Both are acceptable at this scale.

Admin readout: `GET /api/v2/admin/metrics/remix-funnel?days=7`. It reports
every step plus **`social_creations`**, the North Star: carts published in
the window with at least one qualified play from someone other than the
owner. `remixes_with_external_play` is the stricter variant.

Retention: rows grow with at most carts × viewers × 4 and hold no personal
data beyond the hash. Nothing expires them yet; add a periodic delete of rows
older than the analysis window before the table matters.

To remove: delete `handlers/funnel.rs` and its two routes, drop
`funnel_events` (migration down), and remove the `recordFunnel` calls in
`Play.svelte` and `Remix.svelte`.

## Future compatibility

- **Remix as a reply:** lineage is structured, so a "replies" feed is a
  query on `parent_cart_id`.
- **Challenges** (one button, 32 lines, "change this seed"): a jam can name
  a seed cart. Entries are remixes whose `root_cart_id` matches, so no
  constraint engine is needed to find them.
- **Discovery rows** (most remixed, new remixes, from this cartridge,
  remixed from this creator): a `GROUP BY parent_cart_id` / `root_cart_id`
  over indexed columns.
