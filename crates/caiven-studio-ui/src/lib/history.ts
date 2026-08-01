// Shared undo/redo for the asset editors (sprite/map/palette/sfx/music). Command
// pattern: each entry carries its own undo/redo thunks, so this module doesn't
// need to know about RAM offsets or asset shapes — the caller decides what
// "undo" means for a sprite, a map-cell diff, a palette slot, whatever.
//
// Pure functions over a plain `{undo, redo}` state, in the style of editorMath.ts.
// Each editor screen holds its own `$state` instance (declared where it's used,
// since runes only work inside .svelte / .svelte.ts files) and scopes Ctrl+Z to
// whichever screen is active — pixel-art tools (Aseprite, PICO-8) undo the
// document you're looking at, not everything at once.

export interface HistoryEntry {
  label: string;
  undo: () => void;
  redo: () => void;
}

export interface HistoryState {
  undo: HistoryEntry[];
  redo: HistoryEntry[];
}

const CAP = 64;

export function emptyHistory(): HistoryState {
  return { undo: [], redo: [] };
}

export function pushEntry(state: HistoryState, entry: HistoryEntry): HistoryState {
  return { undo: [...state.undo.slice(-(CAP - 1)), entry], redo: [] };
}

export function undoEntry(state: HistoryState): HistoryState {
  const entry = state.undo.at(-1);
  if (!entry) return state;
  entry.undo();
  return { undo: state.undo.slice(0, -1), redo: [...state.redo.slice(-(CAP - 1)), entry] };
}

export function redoEntry(state: HistoryState): HistoryState {
  const entry = state.redo.at(-1);
  if (!entry) return state;
  entry.redo();
  return { undo: [...state.undo.slice(-(CAP - 1)), entry], redo: state.redo.slice(0, -1) };
}
