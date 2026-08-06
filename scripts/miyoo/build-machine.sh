#!/usr/bin/env bash
# Cross-compiles caiven-machine for the Miyoo Mini (Plus) and assembles an
# OnionOS Apps-ready folder in $MIYOO_DIST_DIR/Caiven — drag it straight
# into /mnt/SDCARD/App/ on the SD card.
#
# Requires fetch-toolchain.sh and build-sdl2.sh to have already populated
# MIYOO_TOOLCHAIN_DIR and MIYOO_SDL2_OUT. Must run on Linux x86_64 for the
# same reason as build-sdl2.sh — the toolchain is a Linux ELF binary.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

: "${MIYOO_TOOLCHAIN_DIR:?run fetch-toolchain.sh first and export MIYOO_TOOLCHAIN_DIR}"
: "${MIYOO_SDL2_OUT:?run build-sdl2.sh first and export MIYOO_SDL2_OUT}"
: "${MIYOO_DIST_DIR:=dist/miyoo}"

TARGET=armv7-unknown-linux-gnueabihf
GCC="$MIYOO_TOOLCHAIN_DIR/mini/bin/arm-linux-gnueabihf-gcc"
AR="$MIYOO_TOOLCHAIN_DIR/mini/bin/arm-linux-gnueabihf-ar"

rustup target add "$TARGET" >/dev/null

# cc-rs/rustc's per-target env var convention keeps the dash-separated
# target triple verbatim (CC_armv7-unknown-linux-gnueabihf), which isn't a
# legal `export NAME=val` identifier in any POSIX shell — set it through
# `env` instead of `export`.
#
# -rpath-link (not just -L) is required: SDL2.so itself needs libmi_ao.so /
# libmi_gfx.so / etc (the SigmaStar MI SDK stubs build-sdl2.sh staged
# alongside it) to satisfy ITS OWN undefined symbols, and -L alone only
# resolves explicit -l flags, not a shared library's transitive NEEDED
# entries — ld only follows -rpath-link for those.
env \
  "CC_${TARGET}=$GCC" \
  "AR_${TARGET}=$AR" \
  "CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=$GCC" \
  "CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_RUSTFLAGS=-L $MIYOO_SDL2_OUT -C link-args=-Wl,-rpath-link,$MIYOO_SDL2_OUT" \
  cargo build --locked --release -p caiven-machine \
    --target "$TARGET" \
    --no-default-features --features sdl2-dynamic

out="$MIYOO_DIST_DIR/Caiven"
rm -rf "$out"
mkdir -p "$out"

cp "target/$TARGET/release/caiven-machine" "$out/"
cp "$MIYOO_SDL2_OUT"/libSDL2-2.0.so.0.*.* "$out/"
ln -sf "$(basename "$(ls "$MIYOO_SDL2_OUT"/libSDL2-2.0.so.0.*.* | head -1)")" "$out/libSDL2-2.0.so.0"
cp "$MIYOO_SDL2_OUT"/{libEGL.so,libGLESv2.so,libmi_ao.so,libmi_common.so,libmi_gfx.so,libmi_sys.so,libshmvar.so,libssgfx.so} "$out/"
cp "$MIYOO_SDL2_OUT"/libjson-c.so.5.*.* "$out/"
ln -sf "$(basename "$(ls "$MIYOO_SDL2_OUT"/libjson-c.so.5.*.* | head -1)")" "$out/libjson-c.so.5"

cp crates/caiven-studio/icons/icon.png "$out/icon.png"

cat > "$out/config.json" <<'EOF'
{
  "label": "Caiven",
  "launch": "launch.sh",
  "icon": "icon.png",
  "description": "Fantasy console player"
}
EOF

cat > "$out/launch.sh" <<'EOF'
#!/bin/sh
# OnionOS Apps launcher for Caiven Machine.
# Drag this whole folder into /mnt/SDCARD/App/ on the SD card.
cd "$(dirname "$0")"
# /config/lib and /customer/lib are where the Miyoo's own firmware keeps
# libraries (json-c, the real MI SDK implementations, etc.) that the
# bundled libmi_*.so stubs and libSDL2.so link against but don't provide
# themselves — see docs/development/handheld-builds.md.
export LD_LIBRARY_PATH="$(pwd):/config/lib:/customer/lib:${LD_LIBRARY_PATH}"

# Tried SIGSTOP/SIGCONT on MainUI around the run (theory: it holds AO Dev0
# enabled, blocking our MI_AO_SetPubAttr with NOT_PERM). Reverted: it didn't
# fix audio (same 0xa0052009 with MainUI stopped) AND broke input — MainUI
# is apparently in the path that forwards GPIO button events to uinput on
# this device, so freezing it drops key-up events and buttons read as stuck
# held. Do not reintroduce without solving the input side too.
#
# On-device `ps aux` found the real owner: /mnt/SDCARD/miyoo/app/audioserver
# runs the whole session (not MainUI) and is almost certainly the process
# that keeps AO Dev0 enabled — matches every prior failure (a retry loop in
# MI_AO_SetPubAttr itself still hit NOT_PERM every time, because the holder
# never goes away on its own; it isn't a boot-time race). Unlike MainUI,
# audioserver is a leaf audio daemon with no known role in input, so killing
# it should be safe for buttons. It does own OnionOS's own system sounds
# (menu clicks, low-battery beep) while we run — kill it, run, and always
# try to respawn it on exit, even if caiven-machine itself crashes.
AUDIOSERVER_CMD="/mnt/SDCARD/miyoo/app/audioserver -37"
AUDIOSERVER_PID="$(pidof audioserver 2>/dev/null || true)"
if [ -n "$AUDIOSERVER_PID" ]; then
  kill "$AUDIOSERVER_PID" 2>/dev/null
fi

{
  echo "=== $(date) ==="
  # No cart path here on purpose: passing one (as this launcher used to)
  # makes caiven-machine skip straight into that cart and never draw the
  # library screen at all, so any other .cav dropped into carts/ next to
  # this binary was invisible. Starting bare boots into the library, which
  # scans carts/ itself.
  ./caiven-machine --fullscreen --scale fit --aspect square
  echo "--- exit code: $? ---"
} > ./caiven.log 2>&1

if [ -n "$AUDIOSERVER_PID" ]; then
  $AUDIOSERVER_CMD &
fi
EOF
chmod +x "$out/launch.sh" "$out/caiven-machine"
# `cart_library::default_dir()` scans exe-relative `carts/`, not the exe's
# own directory — the demo cart has to live there for the library to find it.
mkdir -p "$out/carts"
cp carts/fixtures/catch.cav "$out/carts/"

echo "Packaged Miyoo Mini build in $out:"
ls -la "$out"
file "$out/caiven-machine"
