#!/usr/bin/env bash
# Cross-builds SDL2 (steward-fu's Miyoo Mini fork) plus the GPU (EGL/GLESv2
# swiftshader) shim, and stages everything a caiven-machine link needs —
# libSDL2, the swiftshader shim, and the SigmaStar MI SDK stub libs the fork
# bundles — into $MIYOO_SDL2_OUT with proper SONAME symlinks.
#
# Must run on Linux x86_64 (the toolchain in fetch-toolchain.sh is a Linux
# ELF binary; on macOS run this script inside a Linux container — see
# docs/development/handheld-builds.md).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
source scripts/miyoo/versions.env

: "${MIYOO_TOOLCHAIN_DIR:?run fetch-toolchain.sh first and export MIYOO_TOOLCHAIN_DIR}"
: "${MIYOO_WORK_DIR:?set MIYOO_WORK_DIR to a scratch directory}"
: "${MIYOO_SDL2_OUT:?set MIYOO_SDL2_OUT to the directory that should receive the built libraries}"

MINI="$MIYOO_TOOLCHAIN_DIR/mini"
PREBUILT="$MIYOO_TOOLCHAIN_DIR/prebuilt"
SDL2_CHECKOUT="$MIYOO_WORK_DIR/sdl2"

if [[ -f "$MIYOO_SDL2_OUT/.build-commit" ]] && [[ "$(cat "$MIYOO_SDL2_OUT/.build-commit")" == "$MIYOO_SDL2_COMMIT" ]]; then
  echo "SDL2 already built at $MIYOO_SDL2_OUT for commit $MIYOO_SDL2_COMMIT, skipping."
  exit 0
fi

mkdir -p "$MIYOO_WORK_DIR" "$MIYOO_SDL2_OUT"
# Resolve to absolute paths before the script cd's into the SDL2 checkout
# below and never cd's back — a relative MIYOO_WORK_DIR/MIYOO_SDL2_OUT
# (the default build-all.sh uses) would otherwise resolve against the
# wrong directory for every reference after that point.
MIYOO_WORK_DIR="$(cd "$MIYOO_WORK_DIR" && pwd)"
MIYOO_SDL2_OUT="$(cd "$MIYOO_SDL2_OUT" && pwd)"
SDL2_CHECKOUT="$MIYOO_WORK_DIR/sdl2"

if [[ ! -d "$SDL2_CHECKOUT/.git" ]]; then
  git clone "$MIYOO_SDL2_REPO" "$SDL2_CHECKOUT"
fi
git -C "$SDL2_CHECKOUT" fetch --depth 1 origin "$MIYOO_SDL2_COMMIT"
git -C "$SDL2_CHECKOUT" checkout --force "$MIYOO_SDL2_COMMIT"
git -C "$SDL2_CHECKOUT" clean -xfd

# Defensive: only matters on a checkout done with core.autocrlf=true, but
# harmless and idempotent everywhere else. Every shebang script in this repo
# needs a real LF shebang line to run at all.
grep -rlZ '^#!.*sh' "$SDL2_CHECKOUT" 2>/dev/null | tr '\0' '\n' | while IFS= read -r f; do
  if file "$f" | grep -q CRLF; then
    sed -i 's/\r$//' "$f"
  fi
done

# Upstream bug (present in the vanilla SDL2 release this fork is based on,
# not something steward-fu introduced): SDL_internal.h never includes
# SDL_platform.h, so __LINUX__ is undefined at the top of every translation
# unit and is only picked up if some later include in that file happens to
# pull in SDL_platform.h transitively first. That's inconsistent within a
# single file (see SDL_systhread.c, which has two textually-close
# __LINUX__ checks that evaluate differently) and breaks
# src/core/linux/SDL_threadprio.c entirely, which guards its whole body in
# `#ifdef __LINUX__` before anything transitively includes SDL_platform.h —
# that file silently compiles to nothing, and the final caiven-machine link
# fails with "undefined reference to SDL_LinuxSetThreadPriorityAndPolicy_REAL".
# Fix it once at the include site instead of patching every call site.
internal_h="$SDL2_CHECKOUT/sdl2/src/SDL_internal.h"
if ! grep -q '#include "SDL_platform.h"' "$internal_h"; then
  sed -i '/^#define SDL_internal_h_$/a\#include "SDL_platform.h"' "$internal_h"
fi

# On-device evidence (caiven.log, 3 attempts): MI_AO_SetPubAttr fails with
# MI error 0xa0052009 (module=AO, level=ERROR, id=E_MI_ERR_NOT_PERM) on the
# very first audio open of the process, even after a defensive per-process
# MI_AO_DisableChn/Disable (attempt #2, which itself reports "Dev0 has not
# been enabled" — this process never touched AO) and even after suspending
# OnionOS's MainUI shell for the whole run (attempt #3, identical error).
# MINI_OpenDevice calls SetPubAttr exactly once with no retry; the leading
# theory left is that Dev0 is still being torn down asynchronously by
# whatever played the boot/menu sound (a prior owner exiting doesn't mean
# the kernel driver has released the device yet) and a single immediate
# call loses that race. Retry with a short backoff instead of failing dead
# on the first attempt, and log every MI_AO return code so a repeat
# failure still shows exactly which call rejected it and after how many
# tries.
audio_mini_c="$SDL2_CHECKOUT/sdl2/src/audio/mini/SDL_audio_mini.c"
if ! grep -q 'retry SetPubAttr' "$audio_mini_c"; then
  sed -i \
    -e 's/MI_S32 miret = 0;/MI_S32 miret = 0;\n    int ao_retry = 0;/' \
    -e '/miret = MI_AO_SetPubAttr(AoDevId, &stSetAttr);/i\
    /* retry SetPubAttr: see build-sdl2.sh */\
    for (ao_retry = 0; ao_retry < 10; ao_retry++) {\
        MI_AO_DisableChn(AoDevId, AoChn);\
        MI_AO_Disable(AoDevId);' \
    -e 's/miret = MI_AO_SetPubAttr(AoDevId, \&stSetAttr);/&\n        if (miret == MI_SUCCESS) {\n            break;\n        }\n        usleep(20000);\n    }\n    printf("caiven-audio: MI_AO_SetPubAttr -> %d (after %d retries)\\n", miret, ao_retry);/' \
    -e 's/miret = MI_AO_GetPubAttr(AoDevId, \&stGetAttr);/&\n    printf("caiven-audio: MI_AO_GetPubAttr -> %d\\n", miret);/' \
    -e 's/miret = MI_AO_Enable(AoDevId);/&\n    printf("caiven-audio: MI_AO_Enable -> %d\\n", miret);/' \
    -e 's/miret = MI_AO_EnableChn(AoDevId, AoChn);/&\n    printf("caiven-audio: MI_AO_EnableChn -> %d\\n", miret);/' \
    -e 's/miret = MI_AO_SetVolume(AoDevId, s32SetVolumeDb);/&\n    printf("caiven-audio: MI_AO_SetVolume -> %d\\n", miret);/' \
    "$audio_mini_c"
fi

export CROSS="$MINI/bin/arm-linux-gnueabihf-"
export CC="${CROSS}gcc"
export AR="${CROSS}ar"
export AS="${CROSS}as"
export LD="${CROSS}ld"
export CXX="${CROSS}g++"
export HOST=arm-linux
# SDL's autoconf pthread/dlfcn checks pass fine on this toolchain; what they
# need is _GNU_SOURCE, which SDL_internal.h only self-defines if it isn't
# already set — set it globally so it also covers files where that guard is
# evaluated before SDL_internal.h has run (same ordering class of bug as
# above; dlfcn.h gates RTLD_DEFAULT behind __USE_GNU, which glibc only sets
# from _GNU_SOURCE).
export CFLAGS=-D_GNU_SOURCE

# The mini toolchain's gcc wrapper shells out to sibling binaries under
# $PREBUILT (ld.bfd etc.) by absolute path baked in at package time — both
# directories from the tarball must be present, and ld needs these symlinks.
mkdir -p "$PREBUILT/arm-linux-gnueabihf/bin" "$PREBUILT/bin"
ln -sf "$PREBUILT/arm-linux-gnueabihf/bin/ld.bfd" "$PREBUILT/arm-linux-gnueabihf/bin/ld"
ln -sf "$PREBUILT/arm-linux-gnueabihf/bin/ld.bfd" "$PREBUILT/bin/arm-linux-gnueabihf-ld"
ln -sf "$PREBUILT/arm-linux-gnueabihf/bin/ld.bfd" "$PREBUILT/bin/arm-linux-gnueabihf-ld.bfd"
export PATH="$PREBUILT/arm-linux-gnueabihf/bin:$PREBUILT/bin:$MINI/bin:$PATH"

cd "$SDL2_CHECKOUT"
rm -rf sdl2/build

# autogen.sh only runs autoconf, not autoheader, so the checked-in
# include/SDL_config.h.in is stale relative to configure.ac — configure
# still runs and every AC_DEFINE check still passes, but config.status has
# nothing to substitute them into, and silently leaves the whole header at
# its unconfigured (#undef everything) defaults. That's not just a thread
# macro: SDL_VIDEO_DRIVER_MINI/SDL_AUDIO_DRIVER_MINI come back #undef too,
# which would silently produce a non-functional build rather than a build
# failure. Regenerating the header template with autoheader alongside
# autoconf (which autogen.sh should have done) fixes it.
(cd sdl2 && cat acinclude/* > aclocal.m4 && autoconf && autoheader && rm -f aclocal.m4 && rm -rf autom4te.cache)

make cfg
make gpu
make sdl2

sdl2_lib="$(ls sdl2/build/.libs/libSDL2-2.0.so.0.*.* | head -1)"
sdl2_lib_name="$(basename "$sdl2_lib")"

cp "$sdl2_lib" "$MIYOO_SDL2_OUT/$sdl2_lib_name"
ln -sf "$sdl2_lib_name" "$MIYOO_SDL2_OUT/libSDL2-2.0.so.0"
ln -sf libSDL2-2.0.so.0 "$MIYOO_SDL2_OUT/libSDL2.so"
cp sdl2/libEGL.so sdl2/libGLESv2.so "$MIYOO_SDL2_OUT/"
cp mini/lib/*.so "$MIYOO_SDL2_OUT/"

# libjson-c isn't part of the fork's bundled mini/lib stubs, but the SDL2
# audio-mini driver links against it (EXTRA_LDFLAGS in sdl2/Makefile has
# -ljson-c) and it is NOT reliably present on-device — without shipping it
# ourselves, caiven-machine fails to even start on OnionOS with "error
# while loading shared libraries: libjson-c.so.5", no other symptom. The
# toolchain's own sysroot has the exact version SDL2 was linked against.
jsonc_lib="$(readlink -f "$MINI/arm-buildroot-linux-gnueabihf/sysroot/usr/lib/libjson-c.so.5")"
jsonc_lib_name="$(basename "$jsonc_lib")"
cp "$jsonc_lib" "$MIYOO_SDL2_OUT/$jsonc_lib_name"
ln -sf "$jsonc_lib_name" "$MIYOO_SDL2_OUT/libjson-c.so.5"

echo "$MIYOO_SDL2_COMMIT" > "$MIYOO_SDL2_OUT/.build-commit"
echo "SDL2 and friends staged in $MIYOO_SDL2_OUT:"
ls -la "$MIYOO_SDL2_OUT"
