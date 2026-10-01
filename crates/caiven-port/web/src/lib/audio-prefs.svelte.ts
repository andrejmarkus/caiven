import type { CartPlayer } from '../player';

const KEY = 'caiven:audio';

function read(): { muted: boolean; volume: number } {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? 'null');
    if (typeof saved?.muted === 'boolean' && typeof saved?.volume === 'number') {
      return { muted: saved.muted, volume: Math.min(1, Math.max(0, saved.volume)) };
    }
  } catch {
    // Storage blocked or garbled: start from defaults.
  }
  return { muted: false, volume: 1 };
}

/** This viewer's sound settings, shared by every game preview. */
export const audio = $state(read());

export function saveAudio() {
  try { localStorage.setItem(KEY, JSON.stringify(audio)); } catch { /* settings last this tab only */ }
}

export function applyAudio(player: CartPlayer | null) {
  player?.setMuted(audio.muted);
  player?.setVolume(audio.volume);
}
