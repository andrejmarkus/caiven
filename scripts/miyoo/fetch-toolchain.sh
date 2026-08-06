#!/usr/bin/env bash
# Downloads and extracts the Miyoo Mini cross toolchain (steward-fu's
# mini_toolchain, an arm-buildroot-linux-gnueabihf gcc 8.2.1 build) into
# $MIYOO_TOOLCHAIN_DIR. Idempotent: skips the download if already extracted.
#
# The tarball ships two sibling directories that must both be present and
# both available at build time: "mini" (sysroot + gcc) and "prebuilt" (the
# actual gcc/binutils binaries mini's gcc wrapper delegates to). Neither
# works without the other.
#
# MIYOO_TOOLCHAIN_DIR must be /opt: the SDL2 fork's own Makefile (not ours —
# see build-sdl2.sh) hardcodes `export CROSS=/opt/mini/bin/...`, and a plain
# Makefile assignment overrides an inherited environment variable of the
# same name, so pointing CROSS/CC/etc. at a different path via the
# environment before calling `make` has no effect. Extracting anywhere else
# would silently build against whatever (or nothing) happens to already be
# at /opt/mini.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
source scripts/miyoo/versions.env

: "${MIYOO_TOOLCHAIN_DIR:?set MIYOO_TOOLCHAIN_DIR to /opt — see the comment above}"
if [[ "$MIYOO_TOOLCHAIN_DIR" != "/opt" ]]; then
  echo "MIYOO_TOOLCHAIN_DIR must be /opt (got: $MIYOO_TOOLCHAIN_DIR) — see the comment at the top of this script." >&2
  exit 1
fi

if [[ -x "$MIYOO_TOOLCHAIN_DIR/mini/bin/arm-linux-gnueabihf-gcc" && -d "$MIYOO_TOOLCHAIN_DIR/prebuilt" ]]; then
  echo "Toolchain already present at $MIYOO_TOOLCHAIN_DIR, skipping download."
  exit 0
fi

# Plain `mkdir`/`tar` if we already own the directory (true inside a
# container, which runs as root by default); fall back to sudo on a bare
# runner where /opt is root-owned but sudo is passwordless.
as_root() {
  if [[ -w "$MIYOO_TOOLCHAIN_DIR" || -w "$(dirname "$MIYOO_TOOLCHAIN_DIR")" ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

as_root mkdir -p "$MIYOO_TOOLCHAIN_DIR"
as_root chown "$(id -u):$(id -g)" "$MIYOO_TOOLCHAIN_DIR"

tmp_tar="$(mktemp -t miyoo-toolchain.XXXXXX.tar.gz)"
trap 'rm -f "$tmp_tar"' EXIT

echo "Downloading toolchain from $MIYOO_TOOLCHAIN_URL ..."
curl -sL --fail -o "$tmp_tar" "$MIYOO_TOOLCHAIN_URL"

echo "Extracting to $MIYOO_TOOLCHAIN_DIR ..."
# The tarball's terminfo tree has entries that differ only by case
# (P/P5 vs p/p5); on a case-insensitive filesystem (macOS default) tar exits
# non-zero on those collisions even though everything the toolchain itself
# needs extracts fine. Don't treat that as fatal — verify the binaries
# afterward instead.
tar xzf "$tmp_tar" -C "$MIYOO_TOOLCHAIN_DIR" || true

test -x "$MIYOO_TOOLCHAIN_DIR/mini/bin/arm-linux-gnueabihf-gcc"
test -d "$MIYOO_TOOLCHAIN_DIR/prebuilt"
echo "Toolchain ready at $MIYOO_TOOLCHAIN_DIR"
