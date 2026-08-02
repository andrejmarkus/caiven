import type {
  AssetBankState, AssetIndex, AudioState, Breakpoint, CartMeta, CartSize, CartTemplateSummary, CollisionType, ExampleSummary, GlobalValue, LocalCart, PortCartList, PortSession,
  PublishResult, SourceBuffer, StudioBootstrap, TickSnapshot,
} from '../types';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';

const fallbackCode = `local SPEED = 2

local player = { x = 60, y = 60, score = 0 }

function _init()
  set_palette_color(0, 10, 10, 30)
end

function _update()
  clear_screen()

  if button_down(2) then player.x = player.x - SPEED end
  if button_down(3) then player.x = player.x + SPEED end
  if button_down(0) then player.y = player.y - SPEED end
  if button_down(1) then player.y = player.y + SPEED end

  if button_pressed(4) then
    player.score = player.score + 1
    play_sfx(0)
  end

  sprite(0, player.x, player.y)
  draw_text("score", 2, 2, 7)
  draw_number(player.score, 26, 2, 7)
end`;

export const defaultPalette = [
  '#000000', '#1D2B53', '#7E2553', '#008751',
  '#AB5236', '#5F574F', '#C2C3C7', '#FFF1E8',
  '#FF004D', '#FFA300', '#FFEC27', '#00E436',
  '#29ADFF', '#83769C', '#FF77A8', '#FFCCAA',
];

export const fallbackTemplates: CartTemplateSummary[] = [
  { id: 'top-down-mover', name: 'Top-down mover', description: 'Move a sprite around with arrow keys' },
  { id: 'tap-to-score', name: 'Tap to score', description: 'Bouncing ball with score and high-score HUD' },
  { id: 'tile-world', name: 'Tile world', description: 'Map drawing and per-cell collision' },
  { id: 'blank', name: 'Blank', description: 'Empty _init and _update starting point' },
];

export const fallbackExamples: ExampleSummary[] = [
  { id: 'movement', name: 'Movement', description: 'Smallest possible playable cart: one sprite, arrow keys' },
  { id: 'catch', name: 'Catch', description: 'A minigame with sound effects and a music bank' },
  { id: 'tiles', name: 'Tiles', description: 'A tilemap-driven scene built from the map editor' },
  { id: 'stdlib-demo', name: 'Stdlib demo', description: 'Tour of tweens, particles, and animation from the gameplay stdlib' },
];

export const defaultSprite = [
  0, 0, 7, 7, 7, 7, 0, 0,
  0, 7, 15, 15, 15, 15, 7, 0,
  7, 15, 1, 15, 15, 1, 15, 7,
  7, 15, 15, 15, 15, 15, 15, 7,
  7, 15, 8, 15, 15, 8, 15, 7,
  0, 7, 15, 15, 15, 15, 7, 0,
  0, 7, 9, 7, 7, 9, 7, 0,
  0, 0, 9, 0, 0, 9, 0, 0,
];

export const MEMORY = {
  sprites: 0x4000, map: 0x8000,
  palette: 0x9000, sfx: 0x9100, music: 0x9500, collision: 0x9603,
} as const;

export const COLLISION_LEN = 64 * 64;

/** Mirrors `caiven_core::builtin_collision_types()` for the browser fallback. */
export const defaultCollisionTypes: CollisionType[] = [
  { id: 0, name: 'walkable', color: [0, 0, 0], solid: false },
  { id: 1, name: 'solid', color: [255, 176, 0], solid: true },
  { id: 2, name: 'hazard', color: [224, 32, 32], solid: false },
];

/** Flattens `#RRGGBB` palette slots into the raw RGB byte layout a palette bank stores. */
function paletteToBytes(colors: string[]): number[] {
  return colors.flatMap((hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) || 0));
}

const emptyAudio: AudioState = {
  sfxActive: false, sfxId: 0, sfxStep: 0,
  musicActive: false, musicPattern: 0, musicRow: 0, musicLoop: true,
};

const fallback: StudioBootstrap = {
  connected: false,
  title: 'catch',
  path: '~/carts/catch',
  author: 'andrej',
  runState: 'running',
  frame: 1284,
  fps: 60,
  cartSize: { packedBytes: 22 * 1024, maxBytes: 128 * 1024 },
  sources: [
    { path: '~/carts/catch/main.lua', name: 'main.lua', text: fallbackCode, dirty: false },
    { path: '~/carts/catch/enemy.lua', name: 'enemy.lua', text: '-- Enemy movement\nreturn {}', dirty: true },
    { path: '~/carts/catch/ui/hud.lua', name: 'ui/hud.lua', text: '-- HUD helpers\nreturn {}', dirty: false },
  ],
  palette: defaultPalette,
  spriteSheet: [...defaultSprite, ...Array(255 * 64).fill(0)],
  map: Array(4096).fill(0),
  spriteBanks: [0],
  mapBanks: [0],
  activeSpriteBank: 0,
  activeMapBank: 0,
  collision: Array(COLLISION_LEN).fill(0),
  collisionTypes: structuredClone(defaultCollisionTypes),
  sfx: Array(1024).fill(0),
  music: Array(256).fill(0),
  paletteBanks: [0],
  activePaletteBank: 0,
  sfxBanks: [0],
  activeSfxBank: 0,
  musicBanks: [0],
  activeMusicBank: 0,
  ram: Array(65536).fill(0),
  globals: [{ name: 'player', value: '{x=60, y=60, score=0}' }],
  watches: [],
  callStack: [],
  breakpoints: [],
  pauseReason: null,
  diagnostics: [],
  output: ['Browser preview · VM disconnected'],
  meta: { description: 'A tiny cave platformer.', tags: ['platformer', 'jam'] },
  assetIndex: { entries: [], computedRefs: 0 },
  audio: emptyAudio,
  recent: ['~/carts/catch', '~/carts/lantern', '~/downloads/tide-pool', '~/carts/ghost-line'],
  api: [
    { name: 'clear_screen', params: [], returns: 'nil', doc: 'Clear world and UI layers to transparent.', category: 'Console builtins' },
    { name: 'fill_screen', params: [{ name: 'color', ty: 'int' }], returns: 'nil', doc: 'Fill whole screen with one palette color.', category: 'Console builtins' },
    { name: 'draw_rect', params: [{ name: 'x', ty: 'number' }, { name: 'y', ty: 'number' }, { name: 'w', ty: 'number' }, { name: 'h', ty: 'number' }, { name: 'color', ty: 'int' }], returns: 'nil', doc: 'Draw rectangle outline, camera-aware.', category: 'Console builtins' },
    { name: 'fill_rect', params: [{ name: 'x', ty: 'number' }, { name: 'y', ty: 'number' }, { name: 'w', ty: 'number' }, { name: 'h', ty: 'number' }, { name: 'color', ty: 'int' }], returns: 'nil', doc: 'Draw filled rectangle.', category: 'Console builtins' },
    { name: 'draw_text', params: [{ name: 'text', ty: 'string' }, { name: 'x', ty: 'number' }, { name: 'y', ty: 'number' }, { name: 'color', ty: 'int' }], returns: 'nil', doc: 'Draw string on UI layer.', category: 'Console builtins' },
    { name: 'set_palette_color', params: [{ name: 'index', ty: 'int' }, { name: 'r', ty: 'u8' }, { name: 'g', ty: 'u8' }, { name: 'b', ty: 'u8' }], returns: 'nil', doc: 'Replace one palette entry at runtime.', category: 'Console builtins' },
  ],
};

export const isTauri = () => Boolean(window.__TAURI_INTERNALS__);

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!window.__TAURI_INTERNALS__) throw new Error('Tauri IPC unavailable');
  return window.__TAURI_INTERNALS__.invoke<T>(command, args);
}

export async function bootstrap(): Promise<StudioBootstrap> {
  if (!isTauri()) return structuredClone(fallback);
  return invoke<StudioBootstrap>('studio_bootstrap');
}

export async function readCartSize(): Promise<CartSize> {
  return isTauri() ? invoke<CartSize>('studio_cart_size') : structuredClone(fallback.cartSize);
}

export async function listTemplates(): Promise<CartTemplateSummary[]> {
  if (!isTauri()) return structuredClone(fallbackTemplates);
  return invoke<CartTemplateSummary[]>('studio_list_templates');
}

export async function chooseProject(title = 'Open cart project'): Promise<string | null> {
  if (!isTauri()) throw new Error('Folder chooser needs native Caiven Studio. Launch with `npm --prefix ../caiven-studio-ui exec tauri dev` from crates/caiven-studio.');
  const selected = await openDialog({ directory: true, multiple: false, title });
  return typeof selected === 'string' ? selected : null;
}

export async function openProject(path: string): Promise<StudioBootstrap> {
  return invoke<StudioBootstrap>('studio_open_project', { path });
}

export async function newProject(path: string, templateId: string): Promise<StudioBootstrap> {
  return invoke<StudioBootstrap>('studio_new_project', { path, templateId });
}

export async function listExamples(): Promise<ExampleSummary[]> {
  if (!isTauri()) return structuredClone(fallbackExamples);
  return invoke<ExampleSummary[]>('studio_list_examples');
}

export async function remixExample(path: string, exampleId: string): Promise<StudioBootstrap> {
  return invoke<StudioBootstrap>('studio_remix_example', { path, exampleId });
}

export async function chooseExportPath(title: string): Promise<string | null> {
  if (!isTauri()) return null;
  return saveDialog({
    title: 'Pack cartridge',
    defaultPath: `${title || 'cart'}.cav`,
    filters: [{ name: 'Caiven cartridge', extensions: ['cav'] }],
  });
}

export async function exportCartridge(path: string): Promise<void> {
  await invoke('studio_export', { path });
}

export async function transport(action: 'run' | 'pause' | 'reset' | 'step'): Promise<TickSnapshot> {
  if (!isTauri()) {
    fallback.runState = action === 'pause' || action === 'step' ? 'paused' : 'running';
    if (action === 'step') fallback.frame += 1;
    fallback.pauseReason = action === 'pause' || action === 'step'
      ? { kind: 'manual', source: null, line: null, message: null }
      : null;
    return { runState: fallback.runState, frame: fallback.frame, fps: fallback.fps, frameTimeMs: 5.2, globals: fallback.globals, watches: fallback.watches, callStack: fallback.callStack, pauseReason: fallback.pauseReason, audio: fallback.audio, diagnostics: fallback.diagnostics, output: fallback.output, activeSpriteBank: fallback.activeSpriteBank, activeMapBank: fallback.activeMapBank, activePaletteBank: fallback.activePaletteBank, activeSfxBank: fallback.activeSfxBank, activeMusicBank: fallback.activeMusicBank };
  }
  return invoke<TickSnapshot>('studio_transport', { action });
}

export async function saveProject(): Promise<string[]> {
  return isTauri() ? invoke<string[]>('studio_save') : ['main.lua', 'enemy.lua', 'ui/hud.lua'];
}

export async function writeBuffer(path: string, text: string): Promise<void> {
  if (isTauri()) await invoke('studio_write_buffer', { path, text });
}

export async function readFrame(): Promise<Uint8Array | null> {
  if (!isTauri()) return null;
  const buffer = await invoke<ArrayBuffer>('studio_frame');
  return new Uint8Array(buffer);
}

export async function readTick(): Promise<TickSnapshot> {
  if (isTauri()) return invoke<TickSnapshot>('studio_tick');
  return { runState: fallback.runState, frame: fallback.frame++, fps: 60, frameTimeMs: 5.2, globals: fallback.globals, watches: fallback.watches, callStack: fallback.callStack, pauseReason: fallback.pauseReason, audio: fallback.audio, diagnostics: fallback.diagnostics, output: fallback.output, activeSpriteBank: fallback.activeSpriteBank, activeMapBank: fallback.activeMapBank, activePaletteBank: fallback.activePaletteBank, activeSfxBank: fallback.activeSfxBank, activeMusicBank: fallback.activeMusicBank };
}

export async function setInput(button: number, pressed: boolean): Promise<void> {
  if (isTauri()) await invoke('studio_set_input', { button, pressed });
}

export async function writeSprite(sprite: number, pixels: number[]): Promise<void> {
  if (isTauri()) await invoke('studio_write_sprite', { sprite, pixels });
}

export async function writePalette(slot: number, hex: string): Promise<void> {
  if (isTauri()) await invoke('studio_write_palette', { slot, hex });
}

export async function toggleBreakpoint(source: string, line: number): Promise<Breakpoint[]> {
  if (isTauri()) return invoke<Breakpoint[]>('studio_toggle_breakpoint', { source, line });
  const match = fallback.breakpoints.findIndex((item) => item.source === source && item.line === line);
  if (match >= 0) fallback.breakpoints.splice(match, 1);
  else fallback.breakpoints.push({ source, line });
  return structuredClone(fallback.breakpoints);
}

export async function addWatch(expression: string): Promise<GlobalValue[]> {
  if (isTauri()) return invoke<GlobalValue[]>('studio_add_watch', { expression });
  if (!fallback.watches.some((item) => item.name === expression)) fallback.watches.push({ name: expression, value: 'nil' });
  return structuredClone(fallback.watches);
}

export async function removeWatch(expression: string): Promise<GlobalValue[]> {
  if (isTauri()) return invoke<GlobalValue[]>('studio_remove_watch', { expression });
  fallback.watches = fallback.watches.filter((item) => item.name !== expression);
  return structuredClone(fallback.watches);
}

export async function clearOutput(): Promise<void> {
  if (isTauri()) await invoke('studio_clear_output');
  else fallback.output = [];
}

export async function removeRecent(path: string): Promise<string[]> {
  if (isTauri()) return invoke<string[]>('studio_remove_recent', { path });
  fallback.recent = fallback.recent.filter((candidate) => candidate !== path);
  return structuredClone(fallback.recent);
}

export async function readMemory(address: number, len: number): Promise<number[]> {
  return isTauri() ? invoke<number[]>('studio_read_memory', { address, len }) : fallback.ram.slice(address, address + len);
}

export async function writeMemory(address: number, bytes: number[]): Promise<void> {
  if (isTauri()) await invoke('studio_write_memory', { address, bytes });
  else fallback.ram.splice(address, bytes.length, ...bytes);
}

export async function writeMapCells(cells: { offset: number; tile: number }[]): Promise<void> {
  if (isTauri()) await invoke('studio_write_map_cells', { cells });
}

export async function writeCollisionCells(cells: { offset: number; value: number }[]): Promise<void> {
  if (isTauri()) await invoke('studio_write_collision_cells', { cells });
}

export async function readCollisionTypes(): Promise<CollisionType[]> {
  if (isTauri()) return invoke<CollisionType[]>('studio_read_collision_types');
  return structuredClone(fallback.collisionTypes);
}

export async function writeCollisionTypes(types: CollisionType[]): Promise<void> {
  if (isTauri()) await invoke('studio_write_collision_types', { types });
  else fallback.collisionTypes = structuredClone(types);
}

export async function assetBank(
  kind: 'sprites' | 'map' | 'palette' | 'sfx' | 'music', action: 'read' | 'select' | 'create' | 'delete', id?: number,
): Promise<AssetBankState> {
  if (isTauri()) return invoke<AssetBankState>('studio_asset_bank', { kind, action, id: id ?? null });
  const byKind = {
    sprites: { ids: fallback.spriteBanks, active: fallback.activeSpriteBank, data: fallback.spriteSheet },
    map: { ids: fallback.mapBanks, active: fallback.activeMapBank, data: fallback.map },
    palette: { ids: fallback.paletteBanks, active: fallback.activePaletteBank, data: paletteToBytes(fallback.palette) },
    sfx: { ids: fallback.sfxBanks, active: fallback.activeSfxBank, data: fallback.sfx },
    music: { ids: fallback.musicBanks, active: fallback.activeMusicBank, data: fallback.music },
  } as const;
  return { kind, ...byKind[kind] };
}

export async function writeMeta(title: string, author: string, meta: CartMeta): Promise<void> {
  if (isTauri()) await invoke('studio_write_meta', { title, author, meta });
}

export async function createModule(name: string): Promise<SourceBuffer> {
  if (!isTauri()) throw new Error('Module creation requires desktop Studio');
  return invoke<SourceBuffer>('studio_create_module', { name });
}

export async function closeProject(): Promise<StudioBootstrap> {
  return isTauri() ? invoke<StudioBootstrap>('studio_close_project') : structuredClone(fallback);
}

export async function audioTransport(
  kind: 'sfx' | 'music', id: number, action: 'play' | 'stop', loopOn?: boolean,
): Promise<AudioState> {
  if (!isTauri()) return emptyAudio;
  return invoke<AudioState>('studio_audio_transport', { kind, id, action, loopOn });
}

export async function readAssetIndex(): Promise<AssetIndex> {
  return isTauri() ? invoke<AssetIndex>('studio_asset_index') : fallback.assetIndex;
}

export async function portSession(): Promise<PortSession> {
  return isTauri() ? invoke<PortSession>('port_session') : { authenticated: false, username: '', portUrl: 'http://localhost:8080' };
}

export interface PortLinkPending { requestId: string; pollSecret: string; expiresAt: string; }
export async function portLinkStart(): Promise<PortLinkPending> {
  if (!isTauri()) throw new Error('Port account linking needs native Caiven Studio. Browser preview cannot store Studio token.');
  return invoke<PortLinkPending>('port_link_start');
}
export async function portLinkPoll(requestId: string, pollSecret: string): Promise<PortSession | null> {
  return invoke<PortSession | null>('port_link_poll', { requestId, pollSecret });
}
export async function portLinkCancel(requestId: string, pollSecret: string): Promise<void> {
  return invoke<void>('port_link_cancel', { requestId, pollSecret });
}

export async function portLogout(): Promise<PortSession> {
  return invoke<PortSession>('port_logout');
}

export async function portSetUrl(url: string): Promise<PortSession> {
  if (!isTauri()) return { authenticated: false, username: '', portUrl: url || 'http://localhost:8080' };
  return invoke<PortSession>('port_set_url', { url });
}

export async function portListCarts(query = '', sort = 'new', page = 0): Promise<PortCartList> {
  return invoke<PortCartList>('port_list_carts', { query, sort, page });
}

export async function portDownload(id: string, title: string): Promise<string> {
  return invoke<string>('port_download', { id, title });
}

export async function scanLibrary(path: string): Promise<LocalCart[]> {
  return invoke<LocalCart[]>('studio_scan_library', { path });
}

export async function portPublish(input: {
  title: string; description: string; tags: string[];
  changelog: string; targetCartId?: string; frames?: number;
}): Promise<PublishResult> {
  return invoke<PublishResult>('studio_port_publish', {
    ...input,
    targetCartId: input.targetCartId ?? null,
    frames: input.frames ?? 30,
  });
}
