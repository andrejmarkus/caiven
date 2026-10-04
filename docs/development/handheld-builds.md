# Building Caiven Machine for handhelds

`caiven-machine` is one binary for every target: desktop, small Linux
handhelds, and later Android and iOS. It uses SDL2 for the window, renderer,
audio and gamepad, because handheld firmwares ship SDL2. PICO-8 runs on
these devices for the same reason.

## Two ways to link SDL

| Mode | Cargo features | SDL comes from |
| :-- | :-- | :-- |
| Desktop and CI (default) | `sdl2-bundled` | Built from source and linked statically |
| Handheld | `sdl2-dynamic` | The device's own `libSDL2.so` |

### Desktop and CI

```bash
cargo build -p caiven-machine --release
```

Cargo compiles SDL from source and links it in, so the binary has no SDL
dependency and CI runners don't need `libsdl2-dev`. You need a C compiler
and `cmake`.

**CMake 4.** SDL2's bundled `CMakeLists.txt` asks for
`cmake_minimum_required(VERSION 3.0)`, and CMake 4 dropped support for
anything below 3.5. `.cargo/config.toml` sets
`CMAKE_POLICY_VERSION_MINIMUM=3.5` for the workspace, which is the workaround
CMake itself suggests. It only affects the vendored SDL sources. Without it,
configure fails with *"Compatibility with CMake < 3.5 has been removed from
CMake."*

### Handhelds

```bash
cargo build -p caiven-machine --release \
  --target armv7-unknown-linux-gnueabihf \
  --no-default-features --features sdl2-dynamic
```

Don't bundle SDL for these devices. Handheld firmwares patch SDL2 with
device-specific display and input code. The Miyoo Mini has no GPU, only a
SigmaStar 2D blitter, and only its own SDL port knows how to drive it. An
upstream SDL would lose that.

Build against the vendor toolchain's sysroot, so the binary links against
the libc and SDL on the device. `scripts/miyoo/` does the whole job for the
Miyoo Mini and Mini Plus. Other devices have their own toolchains and need
their own script along the same lines.

#### Miyoo Mini (Plus): `scripts/miyoo/`

```bash
scripts/miyoo/build-all.sh
```

This writes an OnionOS app folder to `dist/miyoo/Caiven/`. Copy it into
`/mnt/SDCARD/App/` on the SD card and "Caiven" appears in the Apps list. Use
`App/`, not `Roms/PORTS`: in our tests current OnionOS only found it as
an app with `config.json` and `launch.sh` at the folder root.

`build-all.sh` runs three scripts in order. You can rerun any of them on
its own, and running one twice is safe.

1. `fetch-toolchain.sh` downloads steward-fu's `mini_toolchain`
   (arm-buildroot-linux-gnueabihf, gcc 8.2.1) into `MIYOO_TOOLCHAIN_DIR`.
   The archive has two folders, `mini` (sysroot and gcc wrapper) and
   `prebuilt` (the gcc and binutils binaries the wrapper calls by hard-coded
   path). You need both.
2. `build-sdl2.sh` clones a pinned commit of
   [steward-fu/sdl2](https://github.com/steward-fu/sdl2), an SDL2 patched for
   the Miyoo's SigmaStar MI_GFX and MI_AO hardware. It builds that and the
   swiftshader EGL/GLESv2 shim, then copies `libSDL2-2.0.so.0.*`,
   `libEGL.so`, `libGLESv2.so` and the fork's SigmaStar SDK stub libraries
   (`libmi_ao.so`, `libmi_gfx.so` and others) into `MIYOO_SDL2_OUT` with
   SONAME symlinks.
3. `build-machine.sh` cross-compiles `caiven-machine` against that SDL2 and
   packs the binary, libraries, `config.json`, `icon.png`, a `launch.sh`
   entry point and `catch.cav` (a test cart) into `MIYOO_DIST_DIR/Caiven`.

Details in that last step:

- `config.json` needs `label`, `icon` and `description`. If one is missing,
  OnionOS drops the app from its list without any error.
- `launch.sh` adds `/config/lib` and `/customer/lib` to `LD_LIBRARY_PATH`.
  Those firmware folders hold the real MI SDK libraries that the stubs stand
  in for at build time.
- `launch.sh` also writes stdout and stderr to `caiven.log` next to the
  binary. A crash on the device drops you back to the menu with no message,
  so the log is the only clue.
- The package includes `libjson-c.so.5`, which `libSDL2.so` links against,
  because the firmware doesn't always have it.

The toolchain is a Linux x86_64 binary, so all three scripts must run on
Linux x86_64. On macOS, use a container:

```bash
docker run --rm --platform linux/amd64 -v "$PWD":/work -w /work \
  rust:slim-bookworm bash -c '
    apt-get update && apt-get install -y --no-install-recommends \
      build-essential autoconf automake libtool cmake git curl ca-certificates
    scripts/miyoo/build-all.sh'
```

##### Two bugs in the SDL2 fork

`build-sdl2.sh` works around two upstream bugs. You only need these details
if you build the fork by hand.

- **`autogen.sh` runs `autoconf` but not `autoheader`.** The checked-in
  `include/SDL_config.h.in` falls out of date with `configure.ac`.
  `./configure` runs and its feature checks pass, but `config.status` finds
  no template lines to fill in, so `SDL_config.h` keeps its `#undef`
  defaults without any error. That includes `SDL_VIDEO_DRIVER_MINI` and
  `SDL_AUDIO_DRIVER_MINI`, so you get a binary that builds but has no video
  or audio driver. Fix: run `autoheader` along with `autoconf` before
  configuring.
- **`SDL_internal.h` doesn't include `SDL_platform.h`.** `__LINUX__` stays
  undefined at the top of each file until some other include happens to
  pull `SDL_platform.h` in. `src/core/linux/SDL_threadprio.c` wraps its whole
  body in `#ifdef __LINUX__` before that happens, so the file compiles to
  nothing. You only find out at link time, from
  `undefined reference to SDL_LinuxSetThreadPriorityAndPolicy_REAL`, which
  doesn't point at the cause. Fix: add `#include "SDL_platform.h"` to
  `SDL_internal.h`.

The script applies both fixes with a `sed` edit and an extra `autoheader`
call at checkout time. None of this is our code, so we keep no patch
file.

The final link needs `-Wl,-rpath-link,<sdl2-out-dir>` as well as `-L`.
`libSDL2.so` has undefined references into the MI SDK (`libmi_gfx.so`,
`libmi_ao.so` and so on). `-L` only resolves explicit `-l` flags; `ld` uses
`-rpath-link` to find a shared library's own `NEEDED` entries.
`build-machine.sh` sets it.

## Checking SDL2 on a device

Whether SDL2 is present depends on the firmware, not the device model.
Check first:

```bash
# on the device, or in its rootfs
find / -name 'libSDL2*' 2>/dev/null
```

SDL2 ports that work on the Miyoo Mini family:

- <https://github.com/steward-fu/sdl2>
- <https://github.com/OOPay/sdl2>
- <https://github.com/XK9274/sdl2_miyoo>

## Rendering without a GPU

`Display::new` first asks for an accelerated renderer with vsync. SDL picks
only a driver that supports all requested flags, so on a device with
neither, the request fails instead of degrading. Machine then falls back to
whatever SDL offers and logs the choice at startup:

```
INFO caiven_machine::platform::window] render driver: software
```

Without vsync to pace it, the frame loop sleeps 1 ms whenever the fixed
timestep has no frame to run, so it doesn't spin a core.

Scaling uses nearest-neighbour only (`SDL_RENDER_SCALE_QUALITY=0`). On a
640×480 panel the default `--scale fit --aspect square` draws the 192×128
screen at 639×426 with black bars. `fit` shrinks to the panel width when
filling the height would push the wide screen off the edges.

## What the binary contains

The Miyoo has no GPU, so the console shell draws on the CPU with
`tiny-skia` and `fontdue`. Everything it needs is compiled in:

- Eight subset fonts, about 121 KB in total. See
  `crates/caiven-machine/assets/fonts/README.md` for the list and how to
  regenerate them.
- Six Lucide icons, stored as their upstream path data in
  `src/shell/icon.rs` and turned into geometry at the size needed.

Machine fetches nothing at runtime and never falls back to a system font,
so a device with no network and no fonts shows the same shell as a desktop.

## Running

```bash
caiven-machine --fullscreen game.cav
caiven-machine --scale 3x --aspect square game.cav
```

| Flag | Values | Default |
| :-- | :-- | :-- |
| `--fullscreen` | | off (turn it on for handhelds) |
| `--scale` | `fit`, `2x`, `3x` | `fit` |
| `--aspect` | `square`, `stretch` | `square` |

Controls come from `controls.toml` (see [controls.md](../controls.md)). The
`[gamepad]` table is optional and defaults to the standard SDL mapping:
`DPadUp`, `DPadDown`, `DPadLeft` and `DPadRight` for the D-pad, `South` for
A and `East` for B. Handhelds report their buttons as a game controller, so
the gamepad mapping is the one that counts on a device. Keyboard bindings
are for desktop.

## Headless

For CI or a smoke test without a display:

```bash
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy cargo run -p caiven-machine -- carts/dev/smoke.cav
```

The dummy video driver has no accelerated renderer, so this also tests the
software fallback.
