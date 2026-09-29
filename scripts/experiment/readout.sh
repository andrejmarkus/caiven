#!/usr/bin/env bash
# Prints the Quick Remix funnel for an experiment window, starters first.
# Usage: CAIVEN_PORT_URL=... CAIVEN_PORT_API_KEY=<admin full-scope token> \
#   scripts/experiment/readout.sh <since RFC 3339> [starter-id ...]
# Counts are unique viewer keys (see docs/product/first-user-experiment.md),
# not people: read them next to the session notes.
set -euo pipefail

: "${CAIVEN_PORT_URL:?set CAIVEN_PORT_URL}"
: "${CAIVEN_PORT_API_KEY:?set CAIVEN_PORT_API_KEY to an admin full-scope token}"
since=${1:?usage: readout.sh <since> [starter-id ...]}
shift

curl -fsS -G -H "X-Api-Key: $CAIVEN_PORT_API_KEY" \
  --data-urlencode "since=$since" \
  "$CAIVEN_PORT_URL/api/v1/admin/metrics/remix-funnel" |
  STARTERS="$*" node -e '
const f = JSON.parse(require("fs").readFileSync(0, "utf8"));
const starters = (process.env.STARTERS || "").split(" ").filter(Boolean);
const cols = [
  ["plays", "play started"], ["qualified_plays", "qualified play"],
  ["remix_opened", "remix opened"], ["remix_ran", "changed run ok"],
  ["publish_started", "publish pressed"], ["remixes_published", "remixes published"],
  ["remixes_with_external_play", "…played by another"], ["remixes_remixed", "…remixed again"],
];
const pad = (v, n) => String(v).padEnd(n);
console.log(`window since ${f.since} (staff ${f.include_staff ? "included" : "excluded"})`);
console.log("unit: unique viewer keys per cart; the remixes columns count carts\n");
console.log(pad("cart", 26) + cols.map(([, l]) => pad(l, 19)).join(""));
const rows = [...f.by_cart].sort((a, b) =>
  (starters.includes(b.cart_id) - starters.includes(a.cart_id)) || (b.plays - a.plays));
for (const r of rows) {
  const tag = starters.includes(r.cart_id) ? "* " : r.parent_cart_id ? "  ↳ " : "  ";
  console.log(pad(tag + (r.title || r.cart_id).slice(0, 22), 26) + cols.map(([k]) => pad(r[k], 19)).join(""));
}
console.log("\n" + pad("total", 26) + cols.map(([k]) => pad(f[k], 19)).join(""));
console.log(`\ncarts published ${f.carts_published} · social creations ${f.social_creations}`);
console.log("Stage ratios are not cohort conversions: a person can open Remix before a");
console.log("qualified play, and signing up makes them a second key. * = starter.");
'
