# Building Caiven Machine for handhelds

`caiven-machine` is one binary for every target: desktop, small Linux
handhelds, and later Android/iOS. The platform layer is SDL2 — window,
renderer, audio and gamepad — because SDL2 is what handheld firmwares
actually ship. It is the same reason PICO-8 runs on these devices.

## The two linking modes

| Mode | Cargo features | SDL comes from |
| :-- | :-- | :-- |
| Desktop / CI (default) | `sdl2-bundled` | Built from source and statically linked |
| Handheld | `sdl2-dynamic` | The device's own `libSDL2.so` |

### Desktop and CI

```bash
cargo build -p caiven-machine --release
```

SDL is compiled from source and linked in, so the artifact is self-contained
and no CI runner needs a `libsdl2-dev` step. This requires a C compiler and
`cmake` on the build machine.

**CMake 4 note.** SDL2's bundled `CMakeLists.txt` declares
`cmake_minimum_required(VERSION 3.0)`, and CMake 4 removed compatibility with
anything below 3.5. `.cargo/config.toml` sets
`CMAKE_POLICY_VERSION_MINIMUM=3.5` for the whole workspace to work around
this — it is the escape hatch CMake itself suggests, and it applies only to
the vendored SDL sources. Without it the build fails at configure time with
*"Compatibility with CMake < 3.5 has been removed from CMake."*

### Handhelds

```bash
cargo build -p caiven-machine --release \
  --target armv7-unknown-linux-gnueabihf \
  --no-default-features --features sdl2-dynamic
```

Do **not** bundle SDL for these devices. The SDL2 builds shipped by handheld
firmwares are patched with device-specific display and input code — the
Miyoo Mini has no GPU at all, only a SigmaStar 2D blitter, and its SDL port
is what knows how to drive it. A bundled upstream SDL would lose that.

Build against the vendor toolchain's sysroot so the binary links against the
same libc and SDL the device has. `scripts/miyoo/` does this end to end for
the Miyoo Mini (Plus); see below. Other devices ship their own toolchain and
need their own script, following the same pattern.

#### Miyoo Mini (Plus): `scripts/miyoo/`

```bash
scripts/miyoo/build-all.sh
```

produces an OnionOS Apps-ready folder at `dist/miyoo/Caiven/` — drag it
straight into `/mnt/SDCARD/App/` on the card; it'll show up as "Caiven" in
the Apps list. (Not `Roms/PORTS`: that's the older convention and OnionOS
didn't pick Caiven up from there in testing — Apps, with a `config.json` +
`launch.sh` at the folder root, is what current OnionOS actually scans.)
It runs three scripts in order, each idempotent and individually
re-runnable:

1. `fetch-toolchain.sh` — downloads and extracts steward-fu's
   `mini_toolchain` (arm-buildroot-linux-gnueabihf gcc 8.2.1) to
   `MIYOO_TOOLCHAIN_DIR`. The tarball has two sibling directories, `mini`
   (sysroot + gcc) and `prebuilt` (the actual gcc/binutils binaries mini's
   gcc wrapper delegates to by hardcoded path) — both must be present.
2. `build-sdl2.sh` — clones the pinned commit of
   [steward-fu/sdl2](https://github.com/steward-fu/sdl2) (SDL2 patched for
   the Miyoo's SigmaStar MI_GFX/MI_AO hardware), builds it plus the
   swiftshader EGL/GLESv2 shim, and stages `libSDL2-2.0.so.0.*`,
   `libEGL.so`, `libGLESv2.so`, and the SigmaStar MI SDK stub libs the fork
   bundles (`libmi_ao.so`, `libmi_gfx.so`, etc.) into `MIYOO_SDL2_OUT` with
   proper SONAME symlinks.
3. `build-machine.sh` — cross-compiles `caiven-machine` against that SDL2
   and packages everything (binary + libs + `config.json` + `icon.png` + a
   `launch.sh` entry point + `catch.cav` as a smoke-test cart) into
   `MIYOO_DIST_DIR/Caiven`. `config.json` needs `label`, `icon`, and
   `description` — OnionOS silently drops an App from its list if any of
   those three are missing, with no error shown. `launch.sh` widens
   `LD_LIBRARY_PATH` to include
   `/config/lib` and `/customer/lib` — the Miyoo firmware's own library
   paths, which is where the *real*, non-stub MI SDK implementations live —
   and logs stdout/stderr to `caiven.log` next to the binary on every run,
   since a crash on-device otherwise gives no visible error at all (straight
   back to the menu, no message). `libjson-c.so.5` (that `libSDL2.so` itself
   links against) is bundled directly rather than relied on from firmware,
   since it isn't reliably present there.

All three must run on Linux x86_64 — the toolchain is a Linux ELF binary,
so on macOS run them inside a Linux container:

```bash
docker run --rm --platform linux/amd64 -v "$PWD":/work -w /work \
  rust:slim-bookworm bash -c '
    apt-get update && apt-get install -y --no-install-recommends \
      build-essential autoconf automake libtool cmake git curl ca-certificates
    scripts/miyoo/build-all.sh'
```

Two upstream bugs in the SDL2 fork make `build-sdl2.sh` more than a plain
`./configure && make`, and are worth knowing about if this ever needs
debugging by hand instead of through the script:

- **`autogen.sh` doesn't run `autoheader`**, only `autoconf`. That leaves
  the checked-in `include/SDL_config.h.in` stale relative to
  `configure.ac` — `./configure` still runs and every feature check still
  passes, but `config.status` has no matching template line to substitute
  most of them into, and silently leaves the generated `SDL_config.h` at
  its unconfigured (`#undef` everything) defaults instead of failing
  loudly. This isn't just cosmetic: `SDL_VIDEO_DRIVER_MINI` and
  `SDL_AUDIO_DRIVER_MINI` come back undefined too, which would produce a
  binary that *builds* but silently uses no video/audio driver at all.
  Fix: run `autoheader` alongside `autoconf` before configuring.
- **`SDL_internal.h` never includes `SDL_platform.h`**, so `__LINUX__` is
  undefined at the top of every translation unit and only becomes defined
  partway through a file if something *else* it includes happens to pull
  in `SDL_platform.h` first. `src/core/linux/SDL_threadprio.c` guards its
  entire body in `#ifdef __LINUX__` before any such include runs, so the
  whole file silently compiles to nothing — the failure only shows up much
  later, as a link error (`undefined reference to
  SDL_LinuxSetThreadPriorityAndPolicy_REAL`) that doesn't obviously point
  back at this. Fix: add `#include "SDL_platform.h"` directly to
  `SDL_internal.h`, once, rather than patching every call site that
  happens to need it early.

`build-sdl2.sh` applies both fixes with a `sed` edit and an extra
`autoheader` call; nothing here is caiven's own code, so there's no vendored
patch file, just the script doing it at checkout time.

The final link also needs `-Wl,-rpath-link,<sdl2-out-dir>`, not just `-L`:
`libSDL2.so` itself has undefined references into the MI SDK
(`libmi_gfx.so`, `libmi_ao.so`, ...), and `-L` alone only resolves explicit
`-l` flags — `ld` only follows `-rpath-link` to satisfy a shared library's
own transitive `NEEDED` entries. `build-machine.sh` sets this.

## Verifying SDL2 on a device

SDL2 availability is per-firmware, not per-device. Check before assuming:

```bash
# on the device, or in its rootfs
find / -name 'libSDL2*' 2>/dev/null
```

Known-good SDL2 ports for the Miyoo Mini family:

- <https://github.com/steward-fu/sdl2>
- <https://github.com/OOPay/sdl2>
- <https://github.com/XK9274/sdl2_miyoo>

## What the renderer does on a GPU-less device

`Display::new` asks for an accelerated, vsynced renderer first. SDL only
selects a render driver that supports *every* requested flag, so on a device
with neither, that request fails outright rather than degrading — which is
why there is an explicit fallback to whatever SDL can provide. The chosen
driver is logged at startup:

```
INFO caiven_machine::platform::window] render driver: software
```

When there is no vsync to pace the loop, the frame loop sleeps 1ms on
iterations where the fixed timestep has no frame to advance, rather than
spinning a core.

Scaling is nearest-neighbour only (`SDL_RENDER_SCALE_QUALITY=0`). On a
640×480 panel the default `--scale fit --aspect square` draws the 128×128
framebuffer at 480×480, pillarboxed on black.

## What ships inside the binary

The console shell rasterizes on the CPU (`tiny-skia` + `fontdue`) because the
Miyoo has no GPU, and everything it draws with is compiled in:

- Eight subset type faces, about 121 KB total — see
  `crates/caiven-machine/assets/fonts/README.md` for what they are and how to
  regenerate them.
- Six Lucide icons, carried as their upstream path data in
  `src/shell/icon.rs` and built into geometry at the requested size.

Nothing is fetched at runtime and nothing falls back to a system font, so a
device with no network and no fonts installed renders the same shell as a
desktop does.

## Running

```bash
caiven-machine --fullscreen game.cav
caiven-machine --scale 3x --aspect square game.cav
```

| Flag | Values | Default |
| :-- | :-- | :-- |
| `--fullscreen` | — | off (on is what handhelds want) |
| `--scale` | `fit`, `2x`, `3x` | `fit` |
| `--aspect` | `square`, `stretch` | `square` |

Controls come from `controls.toml` (see the README). The `[gamepad]` table
is optional and defaults to the standard SDL mapping — `DPadUp`/`DPadDown`/
`DPadLeft`/`DPadRight` for the D-pad, `South` for A, `East` for B. Handhelds
expose their built-in buttons as a game controller, so this is the path that
matters on device; the keyboard bindings are for desktop.

## Headless

For CI or a smoke test with no display:

```bash
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy cargo run -p caiven-machine -- carts/demo_smoke.cav
```

This also exercises the software-renderer fallback path, since the dummy
video driver has no accelerated renderer.
