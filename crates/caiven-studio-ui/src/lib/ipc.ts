import type {
  ApiEntry, AssetBankState, AssetIndex, AudioAction, AudioState, Breakpoint, CartMeta, CartSize, CartTemplateSummary, CollisionType, DebugChild, ExampleSummary, GlobalValue, LocalCart, PortCartList, PortSession,
  PreludeModule, PublishResult, SourceBuffer, StudioBootstrap, TickSnapshot,
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
  draw_text("score", 2, 2, 15)
  draw_number(player.score, 26, 2, 15)
end`;

/**
 * Console default palette. Mirrors `caiven_vm::vm::palette::DEFAULT_COLORS`, and a
 * drift test in `crates/caiven-vm/tests/palette_sync.rs` keeps the two in step.
 * Slots 1-12 are four hue ramps in dark → mid → light order; 0 and 15 are black and
 * white, 13 and 14 the accents.
 */
export const defaultPalette = [
  '#10101A', '#6E1F2E', '#C2372F', '#F2803C',
  '#1E3A2A', '#3E8A4A', '#86CF62', '#23345E',
  '#3D6DC4', '#74C0E8', '#3A3340', '#7A6E72',
  '#C3B5A8', '#F5C542', '#E060A0', '#F4F1E6',
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
  palette: 0xE000, sfx: 0xE100, music: 0xE500, collision: 0xE803,
} as const;

/** Sprite sheet shape. Mirrors `caiven_core::memory::SPRITE_COUNT`/`SPRITE_SHEET_COLS`. */
export const SPRITE_COUNT = 256;
export const SPRITE_SHEET_COLS = 16;

/** Total addressable memory. Mirrors `caiven_core::memory::RAM_SIZE`. */
export const RAM_SIZE = 114688;

/** Console framebuffer size. Mirrors `caiven_core::memory::SCREEN_WIDTH`/`SCREEN_HEIGHT`. */
export const SCREEN_WIDTH = 192;
export const SCREEN_HEIGHT = 128;
/** Framebuffer byte length in RGBA. */
export const SCREEN_RGBA_LEN = SCREEN_WIDTH * SCREEN_HEIGHT * 4;

/** Tile map size in tiles. Mirrors `caiven_core::memory::MAP_W`/`MAP_H`. */
export const MAP_W = 192;
export const MAP_H = 128;
/** Map/collision byte length — one byte per cell. */
export const MAP_LEN = MAP_W * MAP_H;
export const COLLISION_LEN = MAP_LEN;
/** SFX bank byte length. Mirrors `caiven_core::memory::SFX_BANK_LEN`. */
export const SFX_BANK_LEN = 16 * 64;
/** Music tracker shape. Mirrors `caiven_core::memory::MUSIC_PATTERN_COUNT`,
 * `MUSIC_PATTERN_ROWS` and `MUSIC_CHANNEL_COUNT`. The four channels are typed
 * by column: pulse 1, pulse 2, triangle, noise. */
export const MUSIC_PATTERN_COUNT = 8;
export const MUSIC_PATTERN_ROWS = 16;
export const MUSIC_CHANNEL_COUNT = 4;
/** Song order steps per bank. Mirrors `caiven_core::memory::MUSIC_ORDER_STEPS`. */
export const MUSIC_ORDER_STEPS = 32;
/** Song section byte length: order table plus loop-point byte. Mirrors
 * `caiven_core::memory::MUSIC_SONG_LEN`. */
export const MUSIC_SONG_LEN = MUSIC_ORDER_STEPS + 1;
/** Byte offset of the song order table within a music bank. */
export const MUSIC_ORDER_OFFSET = MUSIC_PATTERN_COUNT * MUSIC_PATTERN_ROWS * MUSIC_CHANNEL_COUNT;
/** Byte offset of the loop-point byte within a music bank. */
export const MUSIC_LOOP_POINT_OFFSET = MUSIC_ORDER_OFFSET + MUSIC_ORDER_STEPS;
/** Music bank byte length — one byte per channel per row, then the song section. */
export const MUSIC_BANK_LEN =
  MUSIC_PATTERN_COUNT * MUSIC_PATTERN_ROWS * MUSIC_CHANNEL_COUNT + MUSIC_SONG_LEN;
/** Bytes one pattern occupies — the tracker's stride between patterns. */
export const MUSIC_PATTERN_LEN = MUSIC_PATTERN_ROWS * MUSIC_CHANNEL_COUNT;

/** Tile edge in pixels; the map editor renders at 1:1 tile pixels. */
export const TILE_SIZE = 8;
export const MAP_PX_W = MAP_W * TILE_SIZE;
export const MAP_PX_H = MAP_H * TILE_SIZE;
/** Size of one console screen in tiles — the map editor's screen grid. */
export const SCREEN_TILES_W = SCREEN_WIDTH / TILE_SIZE;
export const SCREEN_TILES_H = SCREEN_HEIGHT / TILE_SIZE;

/** Mirrors `caiven_core::builtin_collision_types()` for the browser fallback. */
export const defaultCollisionTypes: CollisionType[] = [
  { id: 0, name: 'walkable', color: [0, 0, 0], shape: 'none' },
  { id: 1, name: 'solid', color: [255, 176, 0], shape: 'solid' },
  { id: 2, name: 'hazard', color: [224, 32, 32], shape: 'none' },
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
  map: Array(MAP_LEN).fill(0),
  spriteBanks: ['default'],
  mapBanks: ['default'],
  activeSpriteBank: 'default',
  activeMapBank: 'default',
  collision: Array(COLLISION_LEN).fill(0),
  collisionTypes: structuredClone(defaultCollisionTypes),
  sfx: Array(SFX_BANK_LEN).fill(0),
  music: Array(MUSIC_BANK_LEN).fill(0),
  paletteBanks: ['default'],
  activePaletteBank: 'default',
  sfxBanks: ['default'],
  activeSfxBank: 'default',
  musicBanks: ['default'],
  activeMusicBank: 'default',
  assetDirty: false,
  ram: Array(RAM_SIZE).fill(0),
  globals: [{ name: 'player', value: '{x=60, y=60, score=0}' }],
  watches: [],
  callStack: [],
  locals: [],
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
  preludeModules: [
    { name: 'vec2', globals: ['Vec2', 'Sprite'], enabled: false },
    { name: 'collision', globals: ['aabb_overlap', 'circle_overlap', 'point_in_rect', 'point_in_circle', 'tile_solid', 'box_touches_solid'], enabled: false },
    { name: 'movement', globals: ['move_and_collide'], enabled: false },
    { name: 'tween', globals: ['new_tween', 'tween_update', 'new_anim', 'anim_update', 'anim_sprite'], enabled: false },
    { name: 'particles', globals: ['Particles'], enabled: false },
    { name: 'scenes', globals: ['Scenes'], enabled: false },
    { name: 'entities', globals: ['Entities'], enabled: false },
    { name: 'camera', globals: ['Camera'], enabled: false },
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

export async function chooseExportWebPath(title: string): Promise<string | null> {
  if (!isTauri()) return null;
  return saveDialog({
    title: 'Export to web',
    defaultPath: `${title || 'cart'}.html`,
    filters: [{ name: 'Web page', extensions: ['html'] }],
  });
}

/// Self-contained, offline-playable `.html` (SPEC §I `export-web`) — inlines
/// the caiven-web WASM runtime, the packed cart, and the audio worklet.
export async function exportCartridgeWeb(path: string): Promise<void> {
  await invoke('studio_export_web', { path });
}

export async function chooseExportScreenshotPath(title: string): Promise<string | null> {
  if (!isTauri()) return null;
  return saveDialog({
    title: 'Export screenshot',
    defaultPath: `${title || 'cart'}.png`,
    filters: [{ name: 'PNG image', extensions: ['png'] }],
  });
}

export async function exportCartridgeScreenshot(path: string): Promise<void> {
  await invoke('studio_export_screenshot', { path });
}

export async function chooseExportSourceZipPath(title: string): Promise<string | null> {
  if (!isTauri()) return null;
  return saveDialog({
    title: 'Export source',
    defaultPath: `${title || 'cart'}.zip`,
    filters: [{ name: 'Zip archive', extensions: ['zip'] }],
  });
}

export async function exportCartridgeSourceZip(path: string): Promise<void> {
  await invoke('studio_export_source_zip', { path });
}

export async function transport(action: 'run' | 'pause' | 'reset' | 'step'): Promise<TickSnapshot> {
  if (!isTauri()) {
    fallback.runState = action === 'pause' || action === 'step' ? 'paused' : 'running';
    if (action === 'step') fallback.frame += 1;
    fallback.pauseReason = action === 'pause' || action === 'step'
      ? { kind: 'manual', source: null, line: null, message: null }
      : null;
    return { runState: fallback.runState, frame: fallback.frame, fps: fallback.fps, frameTimeMs: 5.2, globals: fallback.globals, watches: fallback.watches, callStack: fallback.callStack, locals: fallback.locals, pauseReason: fallback.pauseReason, audio: fallback.audio, diagnostics: fallback.diagnostics, output: fallback.output, activeSpriteBank: fallback.activeSpriteBank, activeMapBank: fallback.activeMapBank, activePaletteBank: fallback.activePaletteBank, activeSfxBank: fallback.activeSfxBank, activeMusicBank: fallback.activeMusicBank, assetDirty: false };
  }
  return invoke<TickSnapshot>('studio_transport', { action });
}

export async function saveProject(): Promise<{ output: string[]; unusedModules: string[] }> {
  if (isTauri()) return invoke('studio_save');
  return { output: ['main.lua', 'enemy.lua', 'ui/hud.lua'], unusedModules: [] };
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
  return { runState: fallback.runState, frame: fallback.frame++, fps: 60, frameTimeMs: 5.2, globals: fallback.globals, watches: fallback.watches, callStack: fallback.callStack, locals: fallback.locals, pauseReason: fallback.pauseReason, audio: fallback.audio, diagnostics: fallback.diagnostics, output: fallback.output, activeSpriteBank: fallback.activeSpriteBank, activeMapBank: fallback.activeMapBank, activePaletteBank: fallback.activePaletteBank, activeSfxBank: fallback.activeSfxBank, activeMusicBank: fallback.activeMusicBank, assetDirty: false };
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

export async function setStdlibModule(module: string, enabled: boolean): Promise<{ api: ApiEntry[]; preludeModules: PreludeModule[] }> {
  if (isTauri()) return invoke('studio_set_stdlib_module', { module, enabled });
  const entry = fallback.preludeModules.find((item) => item.name === module);
  if (entry) entry.enabled = enabled;
  return { api: structuredClone(fallback.api), preludeModules: structuredClone(fallback.preludeModules) };
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

export async function expandDebugValue(nodeId: string): Promise<DebugChild[]> {
  if (isTauri()) return invoke<DebugChild[]>('studio_expand_debug_value', { nodeId });
  return []; // browser/dev fallback — no live VM to expand against
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
  kind: 'sprites' | 'map' | 'palette' | 'sfx' | 'music', action: 'read' | 'select' | 'create' | 'delete', name?: string,
): Promise<AssetBankState> {
  if (isTauri()) return invoke<AssetBankState>('studio_asset_bank', { kind, action, name: name ?? null });
  const byKind = {
    sprites: { names: fallback.spriteBanks, active: fallback.activeSpriteBank, data: fallback.spriteSheet },
    map: { names: fallback.mapBanks, active: fallback.activeMapBank, data: fallback.map },
    palette: { names: fallback.paletteBanks, active: fallback.activePaletteBank, data: paletteToBytes(fallback.palette) },
    sfx: { names: fallback.sfxBanks, active: fallback.activeSfxBank, data: fallback.sfx },
    music: { names: fallback.musicBanks, active: fallback.activeMusicBank, data: fallback.music },
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

export async function forceClose(): Promise<void> {
  if (isTauri()) await invoke<void>('studio_force_close');
}

export async function closeProject(): Promise<StudioBootstrap> {
  return isTauri() ? invoke<StudioBootstrap>('studio_close_project') : structuredClone(fallback);
}

export async function audioTransport(
  kind: 'sfx' | 'music', id: number, action: AudioAction, loopOn?: boolean,
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

export interface PortLinkPending { requestId: string; pollSecret: string; expiresAt: string; userCode: string; }
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
