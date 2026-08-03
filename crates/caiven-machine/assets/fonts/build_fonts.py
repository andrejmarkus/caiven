"""Instance + subset the three variable fonts into static shell faces."""

import pathlib
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

HERE = pathlib.Path(__file__).parent
SRC = HERE / "src"
OUT = HERE / "out"

# Everything the shell can draw: printable ASCII plus the handful of symbols
# the design copy uses.
CHARS = "".join(chr(c) for c in range(0x20, 0x7F))
CHARS += "·"  # · separator in spec lines
CHARS += "×"  # × in "128×128", "integer 2×"
CHARS += "…"  # … in "press a key…"
CHARS += "’"  # ’ typographic apostrophe
CHARS += "◄►"  # ◄ ► legend chip glyphs
CHARS += "←→"  # ← → fallbacks if ◄ ► are unmapped

FACES = [
    ("Inter-var.ttf", "inter", {"wght": 400, "opsz": 14}),
    ("Inter-var.ttf", "inter", {"wght": 500, "opsz": 14}),
    ("Inter-var.ttf", "inter", {"wght": 600, "opsz": 14}),
    ("SpaceGrotesk-var.ttf", "space-grotesk", {"wght": 600}),
    ("SpaceGrotesk-var.ttf", "space-grotesk", {"wght": 700}),
    ("JetBrainsMono-var.ttf", "jetbrains-mono", {"wght": 400}),
    ("JetBrainsMono-var.ttf", "jetbrains-mono", {"wght": 500}),
    ("JetBrainsMono-var.ttf", "jetbrains-mono", {"wght": 700}),
]


def main() -> int:
    OUT.mkdir(exist_ok=True)
    missing_report = {}

    for filename, slug, axes in FACES:
        font = TTFont(SRC / filename)

        cmap = font.getBestCmap()
        missing = sorted(c for c in set(CHARS) if ord(c) not in cmap)
        if missing:
            missing_report.setdefault(slug, set()).update(missing)

        static = instancer.instantiateVariableFont(font, axes, inplace=True)

        options = subset.Options()
        options.layout_features = ["kern", "liga"]
        options.name_IDs = ["*"]
        options.notdef_outline = True
        options.recalc_bounds = True
        options.drop_tables += ["DSIG"]
        subsetter = subset.Subsetter(options=options)
        subsetter.populate(text=CHARS)
        subsetter.subset(static)

        out = OUT / f"{slug}-{axes['wght']}.ttf"
        static.save(out)
        print(f"{out.name}: {out.stat().st_size / 1024:.1f} KB")

    for slug, chars in missing_report.items():
        codes = " ".join(f"U+{ord(c):04X} {c}" for c in sorted(chars))
        print(f"MISSING in {slug}: {codes}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
