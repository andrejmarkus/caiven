# Bundled shell faces

These are the type faces the console shell draws with. They are compiled into
the `caiven-machine` binary with `include_bytes!` — a handheld has no network
and no system fonts worth depending on, so nothing here is loaded at runtime
and nothing can be missing on the device.

| File | Family | Weight | Role |
| :-- | :-- | :-- | :-- |
| `space-grotesk-600.ttf` | Space Grotesk | 600 | Display — eyebrows, lockups |
| `space-grotesk-700.ttf` | Space Grotesk | 700 | Display — wordmark, titles |
| `inter-400.ttf` | Inter | 400 | Body — blurbs, rows |
| `inter-500.ttf` | Inter | 500 | Body — labels |
| `inter-600.ttf` | Inter | 600 | Body — badges, caps labels |
| `jetbrains-mono-400.ttf` | JetBrains Mono | 400 | Mono — spec lines, stage text |
| `jetbrains-mono-500.ttf` | JetBrains Mono | 500 | Mono — the clock |
| `jetbrains-mono-700.ttf` | JetBrains Mono | 700 | Mono — legend chip glyphs |

## Why static instances

Upstream ships all three as variable fonts. `fontdue` rasterizes whatever
instance a file describes and exposes no way to set a weight axis, so a
variable file would render at its default weight and nothing else — Space
Grotesk's default is 300, and the design never uses it. Each weight the
design calls for is therefore pinned into its own file.

## Regenerating

`build_fonts.py` downloads nothing; point it at a `src/` directory holding the
upstream variable fonts, and it instances and subsets them into `out/`:

```bash
mkdir -p src out
curl -sL -o src/Inter-var.ttf \
  'https://raw.githubusercontent.com/google/fonts/main/ofl/inter/Inter%5Bopsz%2Cwght%5D.ttf'
curl -sL -o src/SpaceGrotesk-var.ttf \
  'https://raw.githubusercontent.com/google/fonts/main/ofl/spacegrotesk/SpaceGrotesk%5Bwght%5D.ttf'
curl -sL -o src/JetBrainsMono-var.ttf \
  'https://raw.githubusercontent.com/google/fonts/main/ofl/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf'

python3 -m venv .venv && .venv/bin/pip install fonttools brotli
.venv/bin/python build_fonts.py
```

The subset is printable ASCII plus `·`, `×`, `…`, `’`, `◄`, `►`, `←` and `→` —
that is every character the shell's copy uses. Adding copy that needs a
character outside this set means regenerating, and `font.rs` has a test that
fails when a face cannot render one of them.

Space Grotesk has no glyph for `◄`/`►`; the script reports it. That is why the
legend's direction chips are set in the mono face rather than the display one.

The whole set is about 121 KB.

## Licenses

All three families are SIL Open Font License 1.1. The license text ships
alongside them as `OFL-Inter.txt`, `OFL-SpaceGrotesk.txt` and
`OFL-JetBrainsMono.txt`, which is what the OFL requires of a redistributed
subset.
