#!/usr/bin/env python3
"""Asset generator for the platformer showcase cart.

Writes sprites.png (a real 128x128 indexed PNG sprite sheet, matching the
Studio-native format every other showcase cart uses — see
crates/caiven-cart/src/asset_png.rs::sprites_to_png for the exact layout
this must match), sfx.hex, and music.hex (project-dir hex-text format,
crates/caiven-cart/src/text.rs) directly, no image-library dependency (pure
stdlib zlib/struct PNG encoder below).

Re-run after editing SPRITES/SFX_STEPS/MUSIC_* below; outputs are committed,
this script is not part of the shipped cart.
"""
import pathlib
import struct
import zlib

OUT = pathlib.Path(__file__).resolve().parents[2] / "projects" / "showcase" / "platformer"

SPRITE_PX = 8
SHEET_COLS = 16
SHEET_PX = SHEET_COLS * SPRITE_PX  # 128


# ---------------------------------------------------------------------------
# Palette (16 colors): black-ink outline + white + 4 hue ramps (3 shades
# each) + 2 accents (sky background, gold), matching the design charter's
# "4 hue ramps x 3 shades + black + white + 2 accents" structure (§4).
# Index 0 doubles as the sprite-transparent value (crates/caiven-vm's
# sprite() builtin skips raw pixel byte 0 unconditionally — see
# lua_exec.rs's sprite_fn, "if pixel == 0 { continue }") so it is never used
# as an opaque sprite color; it is instead the world's sky-blue backdrop,
# painted each frame with fill_screen(0) before draw_map (main.lua).
# ---------------------------------------------------------------------------
INK = 1
WHITE = 2
STONE_DARK, STONE_MID, STONE_LIGHT = 3, 4, 5
FOLIAGE_DARK, FOLIAGE_MID, FOLIAGE_LIGHT = 6, 7, 8
WOOD_DARK, WOOD_MID, WOOD_LIGHT = 9, 10, 11
EMBER_DARK, EMBER_MID, EMBER_LIGHT = 12, 13, 14
GOLD = 15

PALETTE = [
    (168, 186, 214),  # 0 sky (accent) — sprite-transparent, world backdrop
    (24, 20, 28),      # 1 ink
    (240, 236, 224),   # 2 white
    (58, 56, 72),       # 3 stone dark
    (104, 100, 122),    # 4 stone mid
    (158, 154, 170),    # 5 stone light
    (32, 68, 46),        # 6 foliage dark
    (60, 120, 74),        # 7 foliage mid
    (132, 190, 108),       # 8 foliage light
    (92, 54, 34),            # 9 wood dark
    (168, 96, 48),             # 10 wood mid
    (230, 162, 90),              # 11 wood light
    (96, 24, 26),                  # 12 ember dark
    (188, 46, 40),                   # 13 ember mid
    (236, 108, 52),                    # 14 ember light
    (255, 214, 88),                      # 15 gold (accent)
]
assert len(PALETTE) == 16


def sprite_from_rows(rows, legend):
    px = []
    for row in rows:
        assert len(row) == SPRITE_PX, f"row must be {SPRITE_PX} chars, got {len(row)!r}"
        for ch in row:
            px.append(legend.get(ch, 0))
    assert len(px) == SPRITE_PX * SPRITE_PX
    return px


BLANK = [0] * (SPRITE_PX * SPRITE_PX)

# --- player -----------------------------------------------------------
# A hooded adventurer: ink outline, wood-amber tunic (mid/light), white
# eye. Facing right by default; main.lua flips flip_x for facing == -1.
PLAYER_IDLE = sprite_from_rows([
    ".OOOO...",
    "OHHHHHO.",
    "OHHHWHO.",
    ".OMMMMO.",
    ".OMMMMO.",
    ".ODMMDO.",
    "..OOOO..",
    "..ODDO..",
], {"O": INK, "H": WOOD_LIGHT, "W": WHITE, "M": WOOD_MID, "D": WOOD_DARK})

PLAYER_RUN1 = sprite_from_rows([
    ".OOOO...",
    "OHHHHHO.",
    "OHHHWHO.",
    ".OMMMMO.",
    ".OMMMMO.",
    "OODMMO..",
    "OD...OO.",
    "D.....O.",
], {"O": INK, "H": WOOD_LIGHT, "W": WHITE, "M": WOOD_MID, "D": WOOD_DARK})

PLAYER_RUN2 = sprite_from_rows([
    ".OOOO...",
    "OHHHHHO.",
    "OHHHWHO.",
    ".OMMMMO.",
    ".OMMMMO.",
    "..OMMDOO",
    ".OO...DO",
    ".O.....D",
], {"O": INK, "H": WOOD_LIGHT, "W": WHITE, "M": WOOD_MID, "D": WOOD_DARK})

PLAYER_RUN3 = sprite_from_rows([
    ".OOOO...",
    "OHHHHHO.",
    "OHHHWHO.",
    ".OMMMMO.",
    ".OMMMMO.",
    ".OMMMMO.",
    "..OOOO..",
    ".OD..DO.",
], {"O": INK, "H": WOOD_LIGHT, "W": WHITE, "M": WOOD_MID, "D": WOOD_DARK})

PLAYER_JUMP = sprite_from_rows([
    ".OOOO...",
    "OHHHHHO.",
    "OHHHWHO.",
    ".OMMMMO.",
    "OOM..MOO",
    "O.OMMO.O",
    "..OMMO..",
    "..ODDO..",
], {"O": INK, "H": WOOD_LIGHT, "W": WHITE, "M": WOOD_MID, "D": WOOD_DARK})

PLAYER_FALL = sprite_from_rows([
    "O..OO..O",
    ".OHHHHO.",
    "OHHHWHO.",
    ".OMMMMO.",
    "..OMMO..",
    ".OO..OO.",
    "OO....OO",
    "O......O",
], {"O": INK, "H": WOOD_LIGHT, "W": WHITE, "M": WOOD_MID, "D": WOOD_DARK})

PLAYER_WALL_SLIDE = sprite_from_rows([
    "..OOOO..",
    "OOHHHHHO",
    ".OHHWHO.",
    ".OMMMMO.",
    ".OMMMO..",
    ".OMMMO..",
    "..OMO...",
    "..OMO...",
], {"O": INK, "H": WOOD_LIGHT, "W": WHITE, "M": WOOD_MID})

PLAYER_DASH = sprite_from_rows([
    "........",
    "..MMMM..",
    ".HHHHHHM",
    "OHHHHHHM",
    ".HHHHHHM",
    "..MMMM..",
    "........",
    "........",
], {"H": WOOD_LIGHT, "M": WOOD_MID, "O": WHITE})

PLAYER_DEATH = sprite_from_rows([
    "D.....D.",
    ".M...M..",
    "..O.O...",
    "...W....",
    "..O.O...",
    ".M...M..",
    "D.....D.",
    "........",
], {"D": EMBER_DARK, "M": WOOD_MID, "O": INK, "W": WHITE})

# --- terrain ------------------------------------------------------------
# Seamless on left/right/bottom (no ink there) so tiling reads as one
# continuous surface; only the exposed top edge is decorated.
GROUND_TOP = sprite_from_rows([
    "FFFfFFff",
    "ffFFffFF",
    "mmMmmMmm",
    "MmmMmmMm",
    "mMmmMmmM",
    "mmMmmMmm",
    "MmmMmmMm",
    "mMmmsmmM",
], {"F": FOLIAGE_LIGHT, "f": FOLIAGE_MID, "m": STONE_MID, "M": STONE_LIGHT, "s": STONE_DARK})

GROUND_FILL = sprite_from_rows([
    "mmMmmMmm",
    "MmmsMmmM",
    "mMmmMmmM",
    "mmMmmsmm",
    "MmmMmmMm",
    "mMmmMmmM",
    "mmsMmmMm",
    "MmmMmmMm",
], {"m": STONE_MID, "M": STONE_LIGHT, "s": STONE_DARK})

PLATFORM = sprite_from_rows([
    "LLoLLoLL",
    "dddddddd",
    "........",
    "........",
    "........",
    "........",
    "........",
    "........",
], {"L": WOOD_LIGHT, "o": INK, "d": WOOD_DARK})

SLOPE_RIGHT = sprite_from_rows([
    "0000000g",
    "000000gg",
    "00000ggm",
    "0000ggmm",
    "000ggmmm",
    "00ggmmmM",
    "0ggmmmMM",
    "ggmmmMMs",
], {"g": FOLIAGE_LIGHT, "m": STONE_MID, "M": STONE_LIGHT, "s": STONE_DARK})

SLOPE_LEFT = sprite_from_rows([
    "g0000000",
    "gg000000",
    "mgg00000",
    "mmgg0000",
    "mmmgg000",
    "Mmmmgg00",
    "MMmmmgg0",
    "sMMmmmgg",
], {"g": FOLIAGE_LIGHT, "m": STONE_MID, "M": STONE_LIGHT, "s": STONE_DARK})

# --- biome variants: cavern (underground, no grass cap; jagged dark rock) --
CAVE_TOP = sprite_from_rows([
    "MsMsMsMs",
    "smMsmMsm",
    "mmsmmsmm",
    "smmsmmsm",
    "mmsmmsmm",
    "smmsmmsm",
    "mmsmmsmm",
    "smmsmmsm",
], {"M": STONE_LIGHT, "s": STONE_DARK, "m": STONE_MID})

CAVE_FILL = sprite_from_rows([
    "smmsmmsm",
    "mmsmmsmm",
    "smmsmmsm",
    "mmsmmsmm",
    "ssmssmss",
    "mmsmmsmm",
    "smmsmmsm",
    "mmsmmsmm",
], {"s": STONE_DARK, "m": STONE_MID})

# --- biome variants: ruins (ancient stonework, gold inlay trim + cracks) --
RUIN_TOP = sprite_from_rows([
    "GGGGGGGG",
    "MMMMMMMM",
    "mmoMmmoM",
    "MmmMmmMm",
    "mMmmMmmM",
    "mmMmmMmm",
    "MmmMmmMm",
    "mMmmomMm",
], {"G": GOLD, "M": STONE_LIGHT, "m": STONE_MID, "o": INK})

RUIN_FILL = sprite_from_rows([
    "mmoMmmoM",
    "MmmMmmMm",
    "mMmmMmmM",
    "mmMmmMmm",
    "MmmoMmmM",
    "mMmmMmmM",
    "mmMmmMmm",
    "MmmMmmMm",
], {"M": STONE_LIGHT, "m": STONE_MID, "o": INK})

# --- biome variant: sky (a one-way platform styled as a fluffy cloud) ----
CLOUD = sprite_from_rows([
    ".WWWWWW.",
    "WWWWWWWW",
    "wwwwwwww",
    "........",
    "........",
    "........",
    "........",
    "........",
], {"W": WHITE, "w": STONE_LIGHT})

# --- hazard / collectible / goal ----------------------------------------
SPIKE = sprite_from_rows([
    "...tw...",
    "...ee...",
    "..eeee..",
    "..eeee..",
    ".eeeeee.",
    ".eeeeee.",
    "dddddddd",
    "iiiiiiii",
], {"t": EMBER_LIGHT, "w": WHITE, "e": EMBER_MID, "d": EMBER_DARK, "i": INK})

BERRY = sprite_from_rows([
    "..fo....",
    ".owo....",
    "ogggggo.",
    "oggggggo",
    "oggggggo",
    ".oggggo.",
    "..oggo..",
    "...oo...",
], {"f": FOLIAGE_DARK, "o": INK, "w": WHITE, "g": GOLD})

FLAG_A = sprite_from_rows([
    "P.......",
    "PGGGG...",
    "PGeGGG..",
    "PGGGG...",
    "P.......",
    "P.......",
    "P.......",
    "P.......",
], {"P": INK, "G": GOLD, "e": EMBER_MID})

FLAG_B = sprite_from_rows([
    "P.......",
    "P.......",
    "PGGGG...",
    "PGeGGG..",
    "PGGGG...",
    "P.......",
    "P.......",
    "P.......",
], {"P": INK, "G": GOLD, "e": EMBER_MID})

# Sprite ids, in sheet order (id = index into this list).
SPRITES = [
    BLANK,              # 0
    PLAYER_IDLE,        # 1
    PLAYER_RUN1,        # 2
    PLAYER_RUN2,        # 3
    PLAYER_RUN3,        # 4
    PLAYER_JUMP,        # 5
    PLAYER_FALL,        # 6
    PLAYER_WALL_SLIDE,  # 7
    PLAYER_DASH,        # 8
    PLAYER_DEATH,       # 9
    GROUND_TOP,         # 10
    GROUND_FILL,        # 11
    PLATFORM,           # 12
    SLOPE_RIGHT,        # 13
    SLOPE_LEFT,         # 14
    SPIKE,              # 15
    BERRY,              # 16
    FLAG_A,             # 17
    FLAG_B,             # 18
    CAVE_TOP,           # 19
    CAVE_FILL,          # 20
    RUIN_TOP,           # 21
    RUIN_FILL,          # 22
    CLOUD,              # 23
]
assert len(SPRITES) <= 256


# ---------------------------------------------------------------------------
# Minimal indexed-PNG encoder (stdlib zlib/struct only — no image library).
# Must match crates/caiven-cart/src/asset_png.rs::png_to_sprites exactly:
# a SHEET_PXxSHEET_PX 8-bit indexed PNG, sprite id = tile_y*16 + tile_x.
# ---------------------------------------------------------------------------
def _png_chunk(tag, data):
    out = struct.pack(">I", len(data)) + tag + data
    out += struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    return out


def write_indexed_png(path, width, height, indices, palette):
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 3, 0, 0, 0)
    plte = b"".join(struct.pack("BBB", *rgb) for rgb in palette)
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # filter type 0 (none) per scanline
        raw.extend(indices[y * width:(y + 1) * width])
    idat = zlib.compress(bytes(raw), 9)
    png = b"\x89PNG\r\n\x1a\n"
    png += _png_chunk(b"IHDR", ihdr)
    png += _png_chunk(b"PLTE", plte)
    png += _png_chunk(b"IDAT", idat)
    png += _png_chunk(b"IEND", b"")
    path.write_bytes(png)


def write_sprites_png():
    indices = bytearray(SHEET_PX * SHEET_PX)
    for sprite_id, sprite in enumerate(SPRITES):
        tile_x = sprite_id % SHEET_COLS
        tile_y = sprite_id // SHEET_COLS
        for sy in range(SPRITE_PX):
            for sx in range(SPRITE_PX):
                px = tile_x * SPRITE_PX + sx
                py = tile_y * SPRITE_PX + sy
                indices[py * SHEET_PX + px] = sprite[sy * SPRITE_PX + sx]
    write_indexed_png(OUT / "sprites.png", SHEET_PX, SHEET_PX, indices, PALETTE)


# ---------------------------------------------------------------------------
# SFX bank: 16 slots x 16 steps x 4 bytes (note, volume, wave, byte3).
# Slots 0-3 are gameplay one-shots; slots 4-15 double as the music tracker's
# "instruments" (see write_music_hex below) — a music pattern row doesn't
# carry its own note, it triggers one of these pre-composed SFX phrases on
# a typed channel (crates/caiven-vm/src/vm/sfx.rs::MusicPlayer, pattern_row_base).
# wave: 0 = pulse, 1 = triangle, 2 = noise (crates/caiven-vm/src/vm/audio.rs).
# ---------------------------------------------------------------------------
SFX_STEP_COUNT = 16  # crates/caiven-vm/src/vm/sfx.rs::SFX_STEPS — fixed per slot


def sfx_line(steps):
    # The project-dir .hex loader (crates/caiven-cart/src/text.rs::decode_hex_block,
    # called on the *whole file*) ignores line breaks and just concatenates
    # every line's hex digits into one flat byte stream — it does not treat
    # each line as an independently-sized slot. But SfxPlayer::sfx_bytes_base
    # computes a slot's address as `SFX_BANK_BASE + sfx_id*64 + step*4`, i.e.
    # it assumes every slot occupies exactly 64 bytes in that stream. A line
    # shorter than 16 steps (as the old hand-typed gameplay one-shots were)
    # shifts every later slot's real data out from under where the engine
    # looks for it — so every slot must be zero-padded to the full 16 steps
    # here, not just given as many steps as the phrase actually uses.
    step_bytes = []
    for note, vol, wave, byte3 in steps:
        step_bytes += [note, vol, wave, byte3]
    assert len(steps) <= SFX_STEP_COUNT, f"SFX slot has {len(steps)} steps, max {SFX_STEP_COUNT}"
    step_bytes += [0] * ((SFX_STEP_COUNT - len(steps)) * 4)
    return "".join(f"{b:02x}" for b in step_bytes)


def _sustained_note(note, wave, vol=11):
    # A music-tracker row doesn't hold its own note — it triggers one of
    # these 16-step SFX phrases on a channel, and a phrase that only fills
    # its first 2-3 steps then falls silent reads as a "pluck" followed by
    # ~0.85s of dead air per ~1.07s row (MUSIC_PATTERN_ROWS ticks_per_row is
    # fixed at 64 by the engine, not cart-adjustable). That is what made the
    # earlier composition sound like sparse random blips instead of music.
    # Filling every one of the 16 steps with the *same* note instead makes
    # the tone ring for (almost) the row's entire duration: tick_sfx_channel
    # re-reads the row on every 4-tick step and retriggers the voice
    # (crates/caiven-vm/src/vm/execution.rs::tick_sfx_channel bumps
    # `voice.epoch` each step even on an unchanged note), but with an
    # instant envelope (byte3 0) that retrigger is a sub-sample discontinuity,
    # not an audible gap — the note reads as one continuous held tone. Only
    # the first step gets a soft attack (byte3 0x20, 50ms) so the onset
    # isn't a hard click, and the last step gets a soft release (byte3 0xC0,
    # 150ms release ramp) so it doesn't cut hard into the next row's retrigger.
    steps = [(note, vol, wave, 0x20)]
    steps += [(note, vol, wave, 0) for _ in range(SFX_STEP_COUNT - 2)]
    steps += [(note, vol, wave, 0xC0)]
    return steps


# Pitches (crates/caiven-vm/src/vm/sfx.rs::note_to_freq: note 49 = A4 = 440Hz,
# +/-1 per semitone). A-minor tonality, matching the existing gameplay SFX.
A3, C4, D4, E4, F3, G3 = 37, 40, 42, 44, 33, 35  # bass register
A4, C5, D5, E5, G5, A5 = 49, 52, 54, 57, 59, 61  # melody/harmony register

SFX_STEPS = [
    # 0: jump - short rising blip
    [(48, 10, 0, 0), (55, 10, 0, 0), (60, 9, 0, 0)],
    # 1: dash - quick noise burst
    [(40, 12, 2, 0), (40, 8, 2, 0)],
    # 2: death - short descending tone
    [(52, 12, 1, 0), (46, 10, 1, 0), (40, 8, 1, 0), (34, 6, 1, 0)],
    # 3: collect - bright two-note chime
    [(64, 11, 0, 0x10), (71, 11, 0, 0x10)],
    # 4-7: sustained triangle bass roots for a i-VI-III-VII progression in
    # A minor (Am - F - C - G), the harmonic backbone of the theme below.
    _sustained_note(A3, 1, vol=13),  # 4  Am root
    _sustained_note(F3, 1, vol=13),  # 5  F root
    _sustained_note(C4, 1, vol=13),  # 6  C root
    _sustained_note(G3, 1, vol=13),  # 7  G root
    # 8-12: sustained pulse pitches — the melody line and, doubled on the
    # second pulse channel, its harmony. A-minor pentatonic plus scale tones
    # so the same five pitches phrase sensibly over all four chords.
    _sustained_note(A4, 0, vol=10),  # 8
    _sustained_note(C5, 0, vol=10),  # 9
    _sustained_note(D5, 0, vol=10),  # 10
    _sustained_note(E5, 0, vol=10),  # 11
    _sustained_note(G5, 0, vol=10),  # 12
    # 13-14: soft noise percussion — a downbeat kick and a lighter off-beat
    # tick, short one-shots (not sustained; percussion should stay punchy).
    [(30, 11, 2, 0), (20, 5, 2, 0)],
    [(58, 7, 2, 0), (48, 3, 2, 0)],
    # 15: melodic accent — an octave-up climax note for the variation section.
    _sustained_note(A5, 0, vol=10),
]
assert len(SFX_STEPS) == 16


def write_sfx_hex():
    lines = [sfx_line(steps) for steps in SFX_STEPS]
    (OUT / "sfx.hex").write_text("\n".join(lines) + "\n")


# ---------------------------------------------------------------------------
# Music bank: MUSIC_PATTERN_DATA_LEN (8 patterns x 16 rows x 4 typed
# channels, one SFX-slot reference byte per cell) + a 32-step song order
# table + one loop-point byte (crates/caiven-core/src/memory.rs). A row
# byte is (sfx_id + 1), 0 = silent for that channel this row
# (crates/caiven-vm/src/vm/sfx.rs::resolve_song_step / MusicPlayer).
# Channels, in column order, are typed pulse/pulse/triangle/noise
# (crates/caiven-vm/src/vm/audio.rs::MUSIC_VOICE_KINDS) — so channels 0/1
# reference the pulse melody/harmony slots (8-12, 15), channel 2 the
# triangle bass-root slots (4-7), channel 3 the noise percussion slots
# (13-14).
# ---------------------------------------------------------------------------
MUSIC_PATTERN_ROWS = 16
MUSIC_CHANNEL_COUNT = 4
MUSIC_PATTERN_COUNT = 8
MUSIC_ORDER_STEPS = 32


def _pattern(ch0, ch1, ch2, ch3):
    channels = [ch0, ch1, ch2, ch3]
    for ch in channels:
        assert len(ch) == MUSIC_PATTERN_ROWS
    rows = []
    for row in range(MUSIC_PATTERN_ROWS):
        for ch in channels:
            slot = ch[row]
            rows.append(0 if slot is None else slot + 1)
    return bytes(rows)


# Sfx-slot ids for readability below (see SFX_STEPS: 4-7 bass roots,
# 8-12 melody/harmony pitches, 13-14 percussion, 15 accent).
BASS_AM, BASS_F, BASS_C, BASS_G = 4, 5, 6, 7
NOTE_A4, NOTE_C5, NOTE_D5, NOTE_E5, NOTE_G5 = 8, 9, 10, 11, 12
PERC_KICK, PERC_TICK, NOTE_A5 = 13, 14, 15

N = None

# The main theme: one 16-row phrase, a full i-VI-III-VII progression in A
# minor (Am-F-C-G, four rows each — the classic "adventure" progression).
# Every row carries a real sustained note on both bass and lead (no dead
# rows), the pulse-2 channel holds each chord's third/fifth as a pad
# underneath, and the melody traces a rise-and-return contour per chord
# that resolves back to A4 at the loop point for a seamless repeat.
PATTERN_MAIN = _pattern(
    ch0=[  # lead melody
        NOTE_A4, NOTE_C5, NOTE_D5, NOTE_C5,   # Am: rise and settle
        NOTE_C5, NOTE_D5, NOTE_E5, NOTE_D5,   # F: rise further
        NOTE_E5, NOTE_G5, NOTE_A5, NOTE_G5,   # C: climax
        NOTE_D5, NOTE_E5, NOTE_D5, NOTE_A4,   # G: descend home
    ],
    ch1=[  # harmony pad: one held chord tone per section
        NOTE_E5, N, N, N,
        NOTE_A4, N, N, N,
        NOTE_E5, N, N, N,
        NOTE_D5, N, N, N,
    ],
    ch2=[  # bass: one sustained root per chord section
        BASS_AM, N, N, N,
        BASS_F, N, N, N,
        BASS_C, N, N, N,
        BASS_G, N, N, N,
    ],
    ch3=[  # gentle downbeat/off-beat pulse, never overwhelming the melody
        PERC_KICK, N, PERC_TICK, N,
        PERC_KICK, N, PERC_TICK, N,
        PERC_KICK, N, PERC_TICK, N,
        PERC_KICK, N, PERC_TICK, N,
    ],
)

# Variation ("lift"): same harmonic backbone (bass unchanged, so the two
# patterns stay in the same key/progression), melody jumps an octave with
# the accent note and drops the harmony pad for contrast, percussion is
# busier — a brighter second phrase before returning to the main theme.
PATTERN_LIFT = _pattern(
    ch0=[
        NOTE_G5, NOTE_A5, NOTE_G5, NOTE_E5,
        NOTE_A5, NOTE_G5, NOTE_E5, NOTE_D5,
        NOTE_G5, NOTE_A5, NOTE_G5, NOTE_E5,
        NOTE_E5, NOTE_D5, NOTE_A4, NOTE_A4,
    ],
    ch1=[N] * 16,
    ch2=[
        BASS_AM, N, N, N,
        BASS_F, N, N, N,
        BASS_C, N, N, N,
        BASS_G, N, N, N,
    ],
    ch3=[
        PERC_KICK, PERC_TICK, N, PERC_TICK,
        PERC_KICK, PERC_TICK, N, PERC_TICK,
        PERC_KICK, PERC_TICK, N, PERC_TICK,
        PERC_KICK, PERC_TICK, N, PERC_TICK,
    ],
)
PATTERN_SILENT = _pattern(ch0=[N] * 16, ch1=[N] * 16, ch2=[N] * 16, ch3=[N] * 16)

MUSIC_PATTERNS = [
    PATTERN_MAIN,    # 0
    PATTERN_LIFT,    # 1
    PATTERN_SILENT,  # 2
    PATTERN_SILENT,  # 3
    PATTERN_SILENT,  # 4
    PATTERN_SILENT,  # 5
    PATTERN_SILENT,  # 6
    PATTERN_SILENT,  # 7
]
assert len(MUSIC_PATTERNS) == MUSIC_PATTERN_COUNT

# Song order: main theme twice, then the lift once, repeating — an actual
# AAB song structure instead of a single flat loop. Order-table bytes are
# pattern-id+1 (0 = empty); the table is longer than one AAB cycle so it
# doesn't need to rely on the loop point until it genuinely runs out.
_CYCLE = [1, 1, 2]  # PATTERN_MAIN, PATTERN_MAIN, PATTERN_LIFT
MUSIC_ORDER = (_CYCLE * (MUSIC_ORDER_STEPS // len(_CYCLE) + 1))[:MUSIC_ORDER_STEPS]
MUSIC_LOOP_POINT = 1  # order-table step 0 (start of the AAB cycle)


def write_music_hex():
    data = bytearray()
    for pattern in MUSIC_PATTERNS:
        data += pattern
    assert len(data) == MUSIC_PATTERN_COUNT * MUSIC_PATTERN_ROWS * MUSIC_CHANNEL_COUNT
    assert len(MUSIC_ORDER) == MUSIC_ORDER_STEPS
    data += bytes(MUSIC_ORDER)
    data += bytes([MUSIC_LOOP_POINT])
    (OUT / "music.hex").write_text("\n".join(
        "".join(f"{b:02x}" for b in data[i:i + 64])
        for i in range(0, len(data), 64)
    ) + "\n")


if __name__ == "__main__":
    write_sprites_png()
    write_sfx_hex()
    write_music_hex()
    print(f"wrote sprites.png, sfx.hex, music.hex to {OUT}")
