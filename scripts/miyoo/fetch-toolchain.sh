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

# The SDL2 fork hardcodes /opt/mini/bin/cmake and passes a non-standard
# --host=<toolchain-file> option. The bundled binary currently requires
# GLIBC_2.36, so it cannot start on Ubuntu 22.04. When that happens, retain
# the original binary for inspection and install a wrapper that translates
# the custom option to standard CMake syntax before invoking the host CMake.
ensure_host_cmake() {
  local cmake_bin="$MIYOO_TOOLCHAIN_DIR/mini/bin/cmake"
  local original="$cmake_bin.toolchain"

  [[ -x "$cmake_bin" ]] || return 0
  if [[ -x "$original" ]] && head -n 1 "$cmake_bin" | grep -q '^#!/usr/bin/env bash$'; then
    return 0
  fi
  if "$cmake_bin" --version >/dev/null 2>&1; then
    return 0
  fi
  if [[ ! -x /usr/bin/cmake ]]; then
    echo "Bundled CMake cannot run and /usr/bin/cmake is unavailable." >&2
    exit 1
  fi

  mv "$cmake_bin" "$original"
  cat > "$cmake_bin" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

args=()
for arg in "$@"; do
  case "$arg" in
    --host=*) args+=("-DCMAKE_TOOLCHAIN_FILE=${arg#--host=}") ;;
    *) args+=("$arg") ;;
  esac
done
exec /usr/bin/cmake "${args[@]}"
EOF
  chmod +x "$cmake_bin"
  echo "Bundled CMake is incompatible with this host; using /usr/bin/cmake through a compatibility wrapper."
}

if [[ -x "$MIYOO_TOOLCHAIN_DIR/mini/bin/arm-linux-gnueabihf-gcc" && -d "$MIYOO_TOOLCHAIN_DIR/prebuilt" ]]; then
  ensure_host_cmake
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
# Write permission can come from an ACL even when the runner does not own
# /opt. Ownership changes still require root, so do not use the writability
# heuristic for chown.
if [[ ! -O "$MIYOO_TOOLCHAIN_DIR" ]]; then
  if [[ "$(id -u)" -eq 0 ]]; then
    chown "$(id -u):$(id -g)" "$MIYOO_TOOLCHAIN_DIR"
  else
    sudo chown "$(id -u):$(id -g)" "$MIYOO_TOOLCHAIN_DIR"
  fi
fi

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
ensure_host_cmake
echo "Toolchain ready at $MIYOO_TOOLCHAIN_DIR"
