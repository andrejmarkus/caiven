#!/usr/bin/env bash
# Runs the full Miyoo Mini pipeline: toolchain, SDL2, caiven-machine.
# See docs/development/handheld-builds.md for what each stage needs and
# why. Must run on Linux x86_64 — on macOS, run this inside a Linux
# container (the toolchain is a Linux ELF binary; see the doc for the
# container invocation this was developed against).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

export MIYOO_TOOLCHAIN_DIR="${MIYOO_TOOLCHAIN_DIR:-/opt}"
export MIYOO_WORK_DIR="${MIYOO_WORK_DIR:-.cache/miyoo/work}"
export MIYOO_SDL2_OUT="${MIYOO_SDL2_OUT:-.cache/miyoo/sdl2-out}"
export MIYOO_DIST_DIR="${MIYOO_DIST_DIR:-dist/miyoo}"

scripts/miyoo/fetch-toolchain.sh
scripts/miyoo/build-sdl2.sh
scripts/miyoo/build-machine.sh
