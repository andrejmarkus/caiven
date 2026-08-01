// Ported from crates/caiven-port/web/src/player.ts's CartPlayer gamepad handling —
// same standard-gamepad mapping and edge-detected poll loop, just framework-shaped
// as a plain factory instead of private class fields.

// Standard-gamepad button index -> Caiven button (0 Up, 1 Down, 2 Left, 3 Right, 4 A, 5 B).
const GAMEPAD_TO_BUTTON: Record<number, number> = {
  12: 0, // d-pad up
  13: 1, // d-pad down
  14: 2, // d-pad left
  15: 3, // d-pad right
  0: 4, // A / bottom face button
  1: 5, // B / right face button
};

export interface GamepadInputOptions {
  onButton: (button: number, pressed: boolean) => void;
  onConnect?: (label: string) => void;
  onDisconnect?: () => void;
}

export function createGamepadInput({ onButton, onConnect, onDisconnect }: GamepadInputOptions) {
  let index: number | null = null;
  let prevState = new Set<number>();

  function onGamepadConnected(event: GamepadEvent) {
    index ??= event.gamepad.index;
    onConnect?.(event.gamepad.id);
  }

  function onGamepadDisconnected(event: GamepadEvent) {
    if (index !== event.gamepad.index) return;
    index = null;
    for (const button of prevState) onButton(button, false);
    prevState = new Set();
    onDisconnect?.();
  }

  function poll() {
    if (index === null) return;
    const pad = navigator.getGamepads()[index];
    if (!pad) return;
    const pressed = new Set<number>();
    for (const [padButton, caivenButton] of Object.entries(GAMEPAD_TO_BUTTON)) {
      if (pad.buttons[Number(padButton)]?.pressed) pressed.add(caivenButton);
    }
    for (const button of pressed) if (!prevState.has(button)) onButton(button, true);
    for (const button of prevState) if (!pressed.has(button)) onButton(button, false);
    prevState = pressed;
  }

  function attach() {
    window.addEventListener('gamepadconnected', onGamepadConnected);
    window.addEventListener('gamepaddisconnected', onGamepadDisconnected);
  }

  function detach() {
    window.removeEventListener('gamepadconnected', onGamepadConnected);
    window.removeEventListener('gamepaddisconnected', onGamepadDisconnected);
  }

  return { attach, detach, poll };
}
