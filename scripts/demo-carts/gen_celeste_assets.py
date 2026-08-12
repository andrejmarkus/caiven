#!/usr/bin/env python3
"""One-off asset generator for the celeste_clone showcase cart.

Writes sprites.hex (sprite-major RAM order, id*64 + sy*8 + sx, one hex
digit pair per byte = one palette index 0-15 per pixel, 0 = transparent),
sfx.hex (one line per SFX slot, 16 steps x 4 bytes = note/volume/wave/byte3),
and music.hex (one line per music slot, 8 steps x 4 bytes) directly as the
project-dir .hex text format (crates/caiven-cart/src/text.rs), so no image
library is needed. Re-run after editing SPRITES/SFX_STEPS/MUSIC_STEPS below;
outputs are committed, this script is not part of the shipped cart.
"""
import pathlib

OUT = pathlib.Path(__file__).resolve().parents[2] / "projects" / "showcase" / "celeste_clone"

SPRITE_BYTES = 64  # 8x8 pixels, 1 byte/pixel


def sprite_from_rows(rows, legend):
    px = []
    for row in rows:
        for ch in row:
            px.append(legend.get(ch, 0))
    assert len(px) == SPRITE_BYTES, f"sprite must be 8x8, got {len(px)} pixels"
    return px


BLANK = [0] * SPRITE_BYTES

PLAYER_IDLE = sprite_from_rows([
    "..1111..",
    ".111111.",
    ".122221.",
    ".111111.",
    "..1111..",
    "..1111..",
    ".11..11.",
    "11....11",
], {"1": 4, "2": 5})

PLAYER_RUN1 = sprite_from_rows([
    "..1111..",
    ".111111.",
    ".122221.",
    ".111111.",
    "..1111..",
    ".11111..",
    "11...11.",
    "1.....1.",
], {"1": 4, "2": 5})

PLAYER_RUN2 = sprite_from_rows([
    "..1111..",
    ".111111.",
    ".122221.",
    ".111111.",
    "..1111..",
    "..11111.",
    ".11...11",
    ".1.....1",
], {"1": 4, "2": 5})

GROUND = sprite_from_rows([
    "33333333",
    "11211112",
    "11111111",
    "12111121",
    "11111111",
    "11112111",
    "11111112",
    "12111111",
], {"1": 1, "2": 2, "3": 3})

PLATFORM = sprite_from_rows([
    "33333333",
    "22222222",
    "........",
    "........",
    "........",
    "........",
    "........",
    "........",
], {"2": 2, "3": 3})

SPIKE = sprite_from_rows([
    "...11...",
    "...11...",
    "..1111..",
    "..1111..",
    ".111111.",
    ".111111.",
    "11111111",
    "22222222",
], {"1": 8, "2": 9})

BERRY = sprite_from_rows([
    "..77....",
    ".766....",
    "76666667",
    "66666666",
    "66666666",
    ".666666.",
    "..6666..",
    "...66...",
], {"6": 6, "7": 7})

FLAG = sprite_from_rows([
    "9.......",
    "9AAAA...",
    "9ABAAA..",
    "9AAAA...",
    "9.......",
    "9.......",
    "9.......",
    "9.......",
], {"9": 12, "A": 10, "B": 11})

SLOPE_RIGHT = sprite_from_rows([
    "0000000G",
    "000000GG",
    "00000GG1",
    "0000GG11",
    "000GG111",
    "00GG1111",
    "0GG11111",
    "GG111111",
], {"G": 3, "1": 1})

SLOPE_LEFT = sprite_from_rows([
    "G0000000",
    "GG000000",
    "1GG00000",
    "11GG0000",
    "111GG000",
    "1111GG00",
    "11111GG0",
    "111111GG",
], {"G": 3, "1": 1})

# Order matches the sprite ids documented in the plan/main.lua constants.
SPRITES = [
    BLANK,        # 0
    PLAYER_IDLE,  # 1
    PLAYER_RUN1,  # 2
    PLAYER_RUN2,  # 3
    GROUND,       # 4
    PLATFORM,     # 5
    SPIKE,        # 6
    BERRY,        # 7
    FLAG,         # 8
    SLOPE_RIGHT,  # 9
    SLOPE_LEFT,   # 10
]


def write_sprites_hex():
    lines = []
    for sprite in SPRITES:
        lines.append("".join(f"{b:02x}" for b in sprite))
    (OUT / "sprites.hex").write_text("\n".join(lines) + "\n")


# SFX: note (MIDI-ish 0-127), volume (0-15), wave (0-3), byte3 (pan/envelope,
# 0 = center pan, instant envelope). One line per slot, 16 steps max;
# trailing all-zero steps may be omitted, decoder treats missing tail as 0.
def sfx_line(steps):
    step_bytes = []
    for note, vol, wave, byte3 in steps:
        step_bytes += [note, vol, wave, byte3]
    return "".join(f"{b:02x}" for b in step_bytes)


SFX_STEPS = [
    # 0: jump - short rising blip
    [(48, 10, 0, 0), (55, 10, 0, 0), (60, 9, 0, 0)],
    # 1: dash - quick noise burst
    [(40, 12, 2, 0), (40, 8, 2, 0)],
    # 2: death - short descending tone
    [(52, 12, 1, 0), (46, 10, 1, 0), (40, 8, 1, 0), (34, 6, 1, 0)],
    # 3: collect - bright two-note chime
    [(64, 11, 0, 0), (71, 11, 0, 0)],
]


def write_sfx_hex():
    lines = [sfx_line(steps) for steps in SFX_STEPS]
    (OUT / "sfx.hex").write_text("\n".join(lines) + "\n")


# Music: 8 steps x 4 bytes per slot. One short looping melody.
MUSIC_STEPS = [
    [
        (48, 8, 0, 0), (52, 8, 0, 0), (55, 8, 0, 0), (52, 8, 0, 0),
        (48, 8, 0, 0), (55, 8, 0, 0), (52, 8, 0, 0), (48, 8, 0, 0),
    ],
]


def write_music_hex():
    lines = [sfx_line(steps) for steps in MUSIC_STEPS]
    (OUT / "music.hex").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    write_sprites_hex()
    write_sfx_hex()
    write_music_hex()
    print(f"wrote sprites.hex, sfx.hex, music.hex to {OUT}")
