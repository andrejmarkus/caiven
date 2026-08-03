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
same libc and SDL the device has. For Miyoo, that is the
[miyoomini-toolchain](https://github.com/MiyooMini) Docker image; other
devices ship their own.

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
