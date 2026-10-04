# Bundled shell faces

The console shell draws its text with these fonts. `include_bytes!` compiles
them into the `caiven-machine` binary, because a handheld has no network and
no usable system fonts. Machine loads nothing from disk at runtime, so a
device can't be missing a font.

| File | Family | Weight | Role |
| :-- | :-- | :-- | :-- |
| `space-grotesk-600.ttf` | Space Grotesk | 600 | Display: eyebrows, lockups |
| `space-grotesk-700.ttf` | Space Grotesk | 700 | Display: wordmark, titles |
| `inter-400.ttf` | Inter | 400 | Body: blurbs, rows |
| `inter-500.ttf` | Inter | 500 | Body: labels |
| `inter-600.ttf` | Inter | 600 | Body: badges, caps labels |
| `jetbrains-mono-400.ttf` | JetBrains Mono | 400 | Mono: spec lines, stage text |
| `jetbrains-mono-500.ttf` | JetBrains Mono | 500 | Mono: the clock |
| `jetbrains-mono-700.ttf` | JetBrains Mono | 700 | Mono: legend chip glyphs |

## Why static instances

All three families come upstream as variable fonts. `fontdue` renders the
instance a file describes and can't set a weight axis, so a variable file
would only ever render at its default weight. For Space Grotesk that's 300,
a weight the design doesn't use. Each weight the design needs gets its own
file instead.

## Regenerating

`build_fonts.py` doesn't download anything. Put the upstream variable fonts
in `src/` and it writes the pinned, subset files to `out/`:

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

The subset is printable ASCII plus `·`, `×`, `…`, `’`, `◄`, `►`, `←` and `→`,
which covers all the shell's text. If you add text with another character,
regenerate the fonts. A test in `font.rs` fails when a face can't render a
character the shell uses.

Space Grotesk has no `◄` or `►` glyph, and the script warns about it. The
legend's direction chips use the mono face for that reason.

The eight files total about 121 KB.

## Licenses

All three families use the SIL Open Font License 1.1. The OFL requires the
license text to travel with a redistributed subset, so it sits next to the
fonts as `OFL-Inter.txt`, `OFL-SpaceGrotesk.txt` and `OFL-JetBrainsMono.txt`.
