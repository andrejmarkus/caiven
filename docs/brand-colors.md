# Brand colors

The logo, Caiven Port and Caiven Studio share one palette. Two hues, ember
and sheen, carry the brand. Everything else is a neutral gray or a status
color.

The starting point was the [Color Hunt Retro palette](https://colorhunt.co/palettes/retro):
`#2B2A2A · #5A7ACD · #FEB05D · #F5F2F2`.

## The idea

The logo is a piece of obsidian, a black volcanic glass. It looks like
hardware, like a console case, and when light hits it you see a cold blue
sheen. That sheen gave us the second color.

- **Obsidian** is the stone itself: black with a faint blue undertone. Use it
  for the logo body only. Port and Studio have their own gray ramp for
  surfaces.
- **Ember** is the glow coming out of the glass. It is the one warm color,
  and the only color for interactive things: buttons, links, focus rings and
  rating stars.
- **Sheen** is the cold light on the facets. Use it sparingly, as a tint
  behind badges and pills. Never put it on a button.

## Core tokens

| Name | Hex | Role |
| :-- | :-- | :-- |
| `ember` | `#FEB05D` | Brand and interactive color: buttons, links, focus ring, rating stars |
| `ember-ink` | `#3A2308` | Text and icons on an `ember` fill |
| `ember-bright` | `#FFC685` | Hover state and lighter tint |
| `obsidian` | `#3B3E48` | Logo body only |
| `sheen` | `#5A7ACD` | Cool accent for tints and badges, never a button fill |
| `sheen-wash` | `#343A4A` | Background behind badges and pills, such as the "featured cart" pill |
| `sheen-bright` | `#93A8DE` | Text and icons on `sheen-wash` |
| `destructive` | `#E5555F` | Errors, delete actions, crash overlays. A status color that has stayed the same through every palette change. |

## Neutrals

Warm-neutral grays, without the blue or violet tint of the previous palette.

| Name | Hex | Role |
| :-- | :-- | :-- |
| `void-900` | `#2B2A2A` | App background |
| `void-800` | `#3F3E3E` | Raised surfaces: cards, panels, popovers |
| `void-700` | `#4F4E4E` | Secondary fills, muted surfaces, hover states |
| `void-600` | `#605E5E` | Borders and dividers |
| `ink` | `#F5F2F2` | Main text |
| `ink-dim` | `#9A9898` | Secondary text |
| `ink-faint` | `#727070` | Disabled text |

## Where the colors are defined

- **Logo and favicon**: `crates/caiven-port/web/src/lib/components/Logo.svelte`,
  `public/favicon.svg` in the Port and Studio UIs, and
  `crates/caiven-studio/icons/`. The crystal uses `ember` for the body,
  `ember-bright` for the ring and left facet, `#FFD29F` for the top highlight,
  and `#F7A854` and `#F29B3F` for the shaded facets and core. To regenerate
  the Studio app icons, run `npx tauri icon <logo.png> -o
  ../caiven-studio/icons` from `crates/caiven-studio-ui`.
- **Port and Studio UI**: both use the shared theme in
  `crates/caiven-ui/src/theme.css`. `--primary` is `ember`, and
  `--accent` / `--accent-foreground` are `sheen-wash` / `sheen-bright`.
- **Port emails** (`crates/caiven-port/src/mailer.rs`): a white card on an
  `ink` page with `void-900` and `void-800` text, so the copy stays readable
  in any mail client. Buttons are `ember` with `ember-ink` labels. Text links
  use a darker ember, `#8A4A0B`, because `ember` on white fails contrast. The
  logo is `assets/email-logo.png`, embedded as an inline CID image.

## Rules

1. Only ember fills primary buttons or colors default links, on every
   surface.
2. Sheen never fills a large area or a button. Keep it to tints behind
   badges.
3. Obsidian stays on the logo. For Port and Studio surfaces, use the void
   grays.
4. Don't add a third brand hue. Try a tint of ember or sheen first.

## History

This is the third palette for Port's dark theme, after phosphor green with
cart gold and then amethyst with ember. If you replace it, keep the same
shape: one warm interactive hue, one cool accent used sparingly, and a
neutral ramp with some warmth in it.
