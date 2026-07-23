# Caiven brand colors — Obsidian & Ember

One palette, three surfaces: the logo/wordmark, Caiven Port (web), Caiven Studio (egui).
Two hues carry the whole identity — everything else is neutral or semantic.

Source: [Color Hunt — Retro](https://colorhunt.co/palettes/retro), palette
`#2B2A2A · #5A7ACD · #FEB05D · #F5F2F2`.

## Why this palette, why now

The console's logo is moving to a crystal/mineral mark — and obsidian fits harder
than a colored gem does: it's a genuinely gray/black stone (volcanic glass), it
reads as *hardware* the way a real console's casing does, and it has a real-world
property worth using — a cold blue sheen where light catches the glass. That gives
the system its second color for free, instead of inventing one.

- **Obsidian** — the mineral. Black glass, faint blue-black undertone. This is the
  logo's body color and nothing else — it's not a UI surface color, Port already
  has its own neutral ramp.
- **Ember** — molten glow escaping the glass. The one warm, high-energy color in
  the system, and the *only* one used for anything interactive: buttons, links,
  focus rings, rating stars. If it's clickable or it's a highlight, it's ember.
- **Sheen** — the cold light catching the glass's facets. A scarce, cool accent:
  wash behind badges/pills, never a button, never the interactive color. Ember
  acts, sheen glints.

## Core tokens

| Name | Hex | Role |
| :-- | :-- | :-- |
| `ember` | `#FEB05D` | Primary brand + interactive color. Buttons, links, focus ring, rating stars. |
| `ember-ink` | `#3A2308` | Text/icon color on an `ember` fill. |
| `ember-bright` | `#FFC685` | Hover/lighter tint of ember. |
| `obsidian` | `#3B3E48` | The mineral. Logo body color only. |
| `sheen` | `#5A7ACD` | Cold glint accent — reserved for wash/badge use, never a button fill. |
| `sheen-wash` | `#343A4A` | Tinted background behind badges/pills (the "featured cart" pill, etc). |
| `sheen-bright` | `#93A8DE` | Text/icon sitting on a `sheen-wash` background. |
| `destructive` | `#E5555F` | Errors, delete actions, crash overlays. Semantic, not brand — unchanged across every palette revision so far. |

## Neutrals

True gray/black this time, warm-neutral (not blue- or violet-tinted like the last
revision) — a console's plastic and steel, not a glowing cave.

| Name | Hex | Role |
| :-- | :-- | :-- |
| `void-900` | `#2B2A2A` | Base app background. |
| `void-800` | `#3F3E3E` | Raised surface — cards, panels, popovers. |
| `void-700` | `#4F4E4E` | Secondary fill, muted surface, hover states. |
| `void-600` | `#605E5E` | Borders, dividers. |
| `ink` | `#F5F2F2` | Primary text. |
| `ink-dim` | `#9A9898` | Muted/secondary text. |
| `ink-faint` | `#727070` | Disabled/faintest text. |

## Where it lives

- **Logo / favicon** (`crates/caiven-port/web/src/lib/components/Logo.svelte`,
  `public/favicon.svg`) — body = `obsidian`, glow stripe = `ember`, glyph = white.
  When the mark becomes an actual crystal shape: obsidian body, ember light
  escaping from inside it, an optional thin `sheen` facet-highlight where it
  catches light. Same split as before, just re-mineraled.
- **Caiven Port** (`crates/caiven-port/web/src/app.css`) — `--primary` = `ember`,
  `--accent`/`--accent-foreground` = `sheen-wash`/`sheen-bright`, `--color-brand`
  (logo) = `obsidian`.
- **Caiven Studio** (`crates/caiven-studio/src/studio/theme.rs`) — `ACCENT` = the
  same `ember` hex directly (it's already light enough to read as a syntax-keyword
  color at small monospace sizes — no separate soft tint needed this time),
  `ERROR` = `destructive`. Studio's other syntax colors (`BUILTIN`, `STRING`,
  `NUMBER`, `COMMENT`) stay their own per-token-type scheme, unrelated to brand.

## Rules of thumb

1. Ember is the only color that fills a primary button or is the default link
   color, on any surface.
2. Sheen never fills a large surface or a button. It's a cold glint — wash behind
   a badge, nothing more.
3. Obsidian lives on the logo only. Don't reach for it as a Port/Studio surface
   color — that's what the void/gray ramp is for.
4. Don't introduce a third brand hue. Reach for an ember or sheen tint before
   inventing something new.

## History

This is the third palette this brand has run under Caiven Port's dark theme —
phosphor-green/cart-gold, then amethyst/ember, now this. If a fourth revision
happens, keep the same shape: one warm interactive hue, one cool scarce accent,
a neutral ramp that isn't just desaturated black. The hues can move; that
structure is what's actually been "the brand" the whole time.
