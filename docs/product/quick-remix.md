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
| Change | same page | A textarea editor with line numbers. **Change one thing** chips list top-level numeric constants (`local SPEED = 2`), and editing a chip rewrites that Lua line. A constant whose comment says `-- try N` gets a **try N** button that writes that value. |
| See result | same page | A 700 ms pause in typing reruns it (a broken edit isn't retried until it changes); Run / Ctrl+Enter reruns right away. Each run rebuilds the `.cav` in the browser (`src/lib/cav.js`) and restarts it in the same WASM module (`CartPlayer.reload`). |
| Errors | same page | The real Lua message, the line number, a highlighted line, a one-line plain-language hint for common errors (`src/lib/remix.js`), and the edit is kept. On a load error the last working build keeps running. |
| Publish | same page | Enabled only after a changed version has run cleanly. Creates a **new cart** with a structured parent link, uploads the current frame as its screenshot, and shows a share URL. |
| Lineage | `/cart/:id` | "Remixed from *X* by @y", "N remixes", and a list of recent remixes. |
| Share | `/play/:id` | A remix shows "@you remixed *X* by @y" plus what changed, linking to the original. Every remixable cart shows a **Your turn** card under the game. The server puts `og:`/`twitter:` preview tags (title, lineage line, cover) into `/play/:id` and `/cart/:id`, so a pasted link unfurls as the game. All user text in them is HTML-escaped. |

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
marks the cart remixable and skips minification. Studio's publish dialog has
the same choice, **Allow remix**. It defaults to the published cart's current
setting, so a new version never closes an open cart by accident. Studio's
token can only publish, so the setting travels in the upload's `meta`
(`remixable` on a new cart or a new version) instead of a cart PATCH. The
Upload page tells creators who upload a `.cav` to build it with
`--no-minify`.

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

Home shows a **Start here** row: the editorial collection with slug
`start-here`, remixable carts only, each with a Remix button straight into
`/remix/:id`. It's a curated collection, not the tag, so nobody can put a
cart on Home by tagging it. The seed script creates the collection and adds
the starters when its token belongs to an admin; otherwise it skips that
step.

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
`caiven:remix-draft:<cart id>`. The wall opens on **Register** (most people
reaching it are new), which says the remix is saved and links to Log in.
Register, Log in and OAuth all carry `?next=` back to `/remix/:id?publish=1`,
which restores the draft, reruns it and opens the publish form. OAuth keeps
`next` in its state cookie, and the server accepts only a plain same-origin
path (`safe_oauth_next`).

Publishing still needs a confirmed email (`VerifiedUser`). A new account
sees that in the publish form before pressing anything: the address the link
went to, **Send it again**, and a note that the remix is saved. Publish also
records the remix as pending, so the confirmation link, opened in the same
browser, goes straight back to the publish form. Coming back to the original
tab refreshes the account, so Publish works there too. A link opened on
another device confirms the email, and the page tells the person to go back
to the remix's tab.

### Measurement

No SDK, no third party, nothing beyond counts. Table `funnel_events`, one row
per **(cart, step, viewer)**, deduped by a unique index:

| Step | Recorded by | Cart |
| --- | --- | --- |
| `qualified_play` | Play page after 20 s of running game, counted from the first button press, without a fault | played cart |
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
external. Both are acceptable at this scale. Behind a reverse proxy the IP
is only real with `CAIVEN_IP_HEADER` set (see `port-operations.md`).
Otherwise every anonymous visitor is one viewer. A person who signs up
halfway is two keys, so the page returning from the account wall
(`?publish=1`) doesn't report their steps again.

Admin readout: `GET /api/v2/admin/metrics/remix-funnel?days=7`, or
`?since=<RFC 3339>` to start at an experiment's first session. It reports
totals, step-to-step `conversion` ratios, and `by_cart` rows: each cart's
plays and steps, plus how many of its direct remixes were published, got an
external qualified play, or were remixed again. **`social_creations`** is the
North Star: carts published in the window with at least one qualified play
from someone other than the owner. `remixes_with_external_play` is the
stricter variant. Admin accounts' events, carts and plays are left out
unless `include_staff=true`, so whoever runs a test doesn't count. That works
only while they're logged in.

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
- **Discovery rows:** "New remixes" (`sort=remixes`, remixes only, newest
  first) and "Most remixed" (`sort=remixed`, carts with at least one remix,
  by direct remix count) ship on Home and Browse. The count is a correlated
  subquery on the indexed `parent_cart_id`. "From this cartridge" and
  "remixed from this creator" are the same kind of query on
  `root_cart_id` / the parent's owner.
