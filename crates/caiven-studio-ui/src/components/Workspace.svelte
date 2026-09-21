<script lang="ts">
  import { flushSync, onDestroy } from 'svelte';
  import {
    Image, Layers, Pipette, Volume2, Music, FileCode2, FileImage,
    Plus, Pencil, PaintBucket, Minus, Square, SquareDashed, Undo2, Redo2, Eraser, ShieldCheck,
    FlipHorizontal, FlipVertical, RotateCw, RotateCcw, Trash2, Search, FolderOpen, Play,
    ExternalLink, Sparkles, ArrowRight, ArrowUp, ArrowDown, ArrowLeft, CircleCheck, ChevronRight, X,
    UserRound, Globe, Gamepad2, Grid3X3, BoxSelect, Puzzle,
  } from '@lucide/svelte';
  import { Button } from '@caiven/ui/button';
  import { Input } from '@caiven/ui/input';
  import { Textarea } from '@caiven/ui/textarea';
  import * as Dialog from '@caiven/ui/dialog';
  import type {
    ApiEntry, AssetIndex, AssetRef, AudioAction, AudioState, Breakpoint, CartMeta, CartSize, CollisionShape, CollisionType, Diagnostic, EditorInsertRequest,
    EditorRevealRequest, ExampleSummary, LocalCart, PortCart, PortSession, PreludeModule, Screen, SourceBuffer,
  } from '../types';
  import {
    autotileEdits, composeGroup, decomposeGroup, dragPanScroll, flipHorizontal, flipVertical, MAP_ZOOM_LEVELS,
    moveRegion, nextMapZoom, pasteRegion, regionValues, rotateClockwise, rotateCounterClockwise,
    type CollisionBrush, type CollisionEdit, type PixelRegion,
  } from '../lib/editorMath';
  import { emptyHistory, pushEntry, undoEntry, redoEntry, type HistoryState } from '../lib/history';
  import type LuaEditorModule from './LuaEditor.svelte';
  import MapCanvas from './MapCanvas.svelte';
  import SpriteCanvas, { type Pixel, type SpriteTool } from './SpriteCanvas.svelte';
  import {
    MAP_H, MAP_PX_H, MAP_PX_W, MAP_W, MUSIC_CHANNEL_COUNT, MUSIC_ORDER_OFFSET, MUSIC_ORDER_STEPS,
    MUSIC_PATTERN_COUNT, MUSIC_PATTERN_LEN, MUSIC_PATTERN_ROWS, SCREEN_HEIGHT, SCREEN_RGBA_LEN, SCREEN_WIDTH,
    SPRITE_SHEET_COLS,
  } from '../lib/ipc';

  /** Music channel timbres, fixed by column. Mirrors `audio::MUSIC_VOICE_KINDS`. */
  const MUSIC_CHANNEL_LABELS = ['Pulse 1', 'Pulse 2', 'Triangle', 'Noise'];

  type MapTool = 'pencil' | 'fill' | 'rect' | 'rect-outline' | 'pick' | 'erase' | 'line' | 'select' | 'autotile';
  type MapRegion = { x0: number; y0: number; w: number; h: number };

  interface Props {
    screen: Screen;
    sources: SourceBuffer[];
    activeSource: number;
    palette: string[];
    spriteSheet: number[];
    map: number[];
    spriteBanks: string[];
    mapBanks: string[];
    activeSpriteBank: string;
    activeMapBank: string;
    collision: number[];
    collisionTypes: CollisionType[];
    sfx: number[];
    music: number[];
    paletteBanks: string[];
    sfxBanks: string[];
    musicBanks: string[];
    activePaletteBank: string;
    activeSfxBank: string;
    activeMusicBank: string;
    cartSize: CartSize;
    audio: AudioState;
    assetIndex: AssetIndex;
    diagnostics: Diagnostic[];
    breakpoints: Breakpoint[];
    title: string;
    author: string;
    path: string;
    meta: CartMeta;
    dirty: boolean;
    tourDone: boolean;
    recent: string[];
    examples: ExampleSummary[];
    api: ApiEntry[];
    preludeModules: PreludeModule[];
    frameData: Uint8Array | null;
    insertRequest: EditorInsertRequest | null;
    revealRequest: EditorRevealRequest | null;
    onInsertHandled: (id: number) => void;
    onRevealHandled: (id: number) => void;
    /** Shared with App so space-to-preview knows which slot is selected. */
    soundSelection: { sfx: number; pattern: number };
    onNavigate: (screen: Screen) => void;
    onSource: (index: number) => void;
    onCode: (text: string) => void;
    onSprite: (sprite: number, pixels: number[]) => void;
    onCollision: (edits: CollisionEdit[]) => void;
    onCollisionTypes: (types: CollisionType[]) => void;
    onMap: (cells: { offset: number; tile: number }[]) => void;
    onAssetBank: (kind: 'sprites' | 'map' | 'palette' | 'sfx' | 'music', action: 'select' | 'create' | 'delete', name?: string) => void | Promise<boolean | void>;
    onSfx: (slot: number, bytes: number[]) => void;
    onMusic: (pattern: number, bytes: number[]) => void;
    /** Writes into the bank's song section at `offset` bytes past
     *  MUSIC_ORDER_OFFSET — the order table, then the loop-point byte. */
    onSong: (offset: number, bytes: number[]) => void;
    onAudio: (kind: 'sfx' | 'music', id: number, action: AudioAction) => void;
    onBreakpoint: (source: string, line: number) => void;
    onMeta: (title: string, author: string, meta: CartMeta) => void;
    onSetStdlibModule: (module: string, enabled: boolean) => void;
    onCreateModule: () => void;
    onPalette: (slot: number, hex: string) => void;
    onTour: () => void;
    onOpen: () => void;
    onNew: () => void;
    onRemix: (exampleId: string) => void;
    localCarts: LocalCart[];
    portCarts: PortCart[];
    portAccount: PortSession;
    portBusy: boolean;
    portError: string;
    portLinkPending: boolean;
    portLinkExpiresAt: string;
    portLinkUserCode: string;
    onScanLibrary: () => void;
    onSearchPort: (query: string) => void;
    onOpenLocal: (path: string) => void;
    onRemoveRecent: (path: string) => void;
    onDownloadPort: (cart: PortCart) => void;
    onOpenPortAccount: () => void;
    onPortLink: () => void;
    onPortLinkCancel: () => void;
    onPortLogout: () => void;
    onSetServerUrl: (url: string) => void;
    onInsertBuiltin: (name: string) => void;
    onOpenSource: (path: string, line: number | null, column?: number | null) => void;
    /** Lets App mirror the active editor's undo/redo availability into the ⌘K command palette. */
    onHistoryStatus?: (status: { canUndo: boolean; canRedo: boolean }) => void;
  }

  let {
    screen, sources, activeSource, palette, spriteSheet, map, spriteBanks, mapBanks, activeSpriteBank, activeMapBank, collision, collisionTypes, sfx, music,
    paletteBanks, sfxBanks, musicBanks, activePaletteBank, activeSfxBank, activeMusicBank, cartSize,
    audio, assetIndex, diagnostics, breakpoints, title, author, path, meta, dirty, tourDone, recent, examples, api, preludeModules, frameData, insertRequest, revealRequest, onInsertHandled, onRevealHandled,
    // Written through by the SFX/pattern list clicks below, so it must be
    // bindable — mutating it as a plain prop is a Svelte ownership violation.
    soundSelection = $bindable(),
    onNavigate, onSource, onCode, onSprite, onCollision, onCollisionTypes, onMap, onAssetBank, onSfx, onMusic, onSong, onAudio,
    onBreakpoint, onMeta, onSetStdlibModule, onCreateModule, onPalette, onTour, onOpen, onNew, onRemix,
    localCarts, portCarts, portAccount, portBusy, portError, portLinkPending, portLinkExpiresAt, portLinkUserCode, onScanLibrary,
    onSearchPort, onOpenLocal, onRemoveRecent, onDownloadPort, onOpenPortAccount, onPortLink, onPortLinkCancel, onPortLogout,
    onInsertBuiltin, onOpenSource, onHistoryStatus, onSetServerUrl,
  }: Props = $props();

  let serverUrlDraft = $state('');
  $effect(() => { serverUrlDraft = portAccount.portUrl; });

  // CodeMirror (LuaEditor's dependency) is the single largest chunk in the
  // Studio bundle — load it only once the code screen is actually opened
  // instead of paying for it on every Studio launch.
  let LuaEditorComponent: typeof LuaEditorModule | null = $state(null);
  $effect(() => {
    if (screen === 'code' && !LuaEditorComponent) {
      import('./LuaEditor.svelte').then((m) => { LuaEditorComponent = m.default; });
    }
  });

  let selectedColor = $state(8);
  let selectedSlot = $state(9);
  let selectedSprite = $state(0);
  // Read-only views of the shared selection; clicks write through soundSelection.
  const selectedSfx = $derived(soundSelection.sfx);
  const selectedPattern = $derived(soundSelection.pattern);
  let selectedTile = $state(0);
  // A multi-tile brush picked by marquee-dragging across the tile picker below.
  // null means "just paint selectedTile" — the default, unchanged 1x1 behavior.
  let mapStamp = $state<{ w: number; h: number; tiles: number[] } | null>(null);
  const effectiveStamp = $derived(mapStamp ?? { w: 1, h: 1, tiles: [selectedTile] });
  let pickerEl: HTMLDivElement | undefined = $state();
  let pickerDrag = $state<{ anchor: number; current: number } | null>(null);
  // The 'select' tool's current marquee (from MapCanvas) and the last thing
  // copied/cut from it — Ctrl+V turns the clipboard into a mapStamp and drops
  // into the pencil tool, reusing the same stamp-painting path as 3a.
  let mapSelection = $state<MapRegion | null>(null);
  let mapClipboard = $state<{ w: number; h: number; tiles: number[] } | null>(null);
  // Sprite editor's group canvas: {w,h} adjacent 8x8 slots edited as one
  // canvas, anchored at selectedSprite's row/col. {1,1} is the plain
  // single-sprite case, unchanged from before the group canvas existed.
  let spriteGroup = $state<{ w: number; h: number }>({ w: 1, h: 1 });
  let spriteSheetDrag = $state<{ anchor: number; current: number } | null>(null);
  let spriteSheetEl: HTMLDivElement | undefined = $state();
  // The sprite 'select' tool's marquee (in group-pixel coordinates) and the
  // last thing copied/cut from it — mirrors mapSelection/mapClipboard above.
  let spriteSelection = $state<PixelRegion | null>(null);
  let spriteClipboard = $state<{ w: number; h: number; pixels: number[] } | null>(null);
  let spriteZoom = $state(1);
  let mapTool = $state<MapTool>('pencil');
  let mapLayer = $state<'tiles' | 'collision'>('tiles');
  let collisionBrush = $state<CollisionBrush>(1);
  let collisionTypesPanel = $state(false);
  let mapZoom = $state(1);
  let mapPanning = $state(false);
  let mapPan: {
    pointerId: number;
    viewport: HTMLDivElement;
    lastX: number;
    lastY: number;
    pendingX: number;
    pendingY: number;
  } | null = null;
  let mapPanFrame: number | undefined;
  let mapHistory = $state<HistoryState>(emptyHistory());
  let spriteHistory = $state<HistoryState>(emptyHistory());
  let paletteHistory = $state<HistoryState>(emptyHistory());
  let sfxHistory = $state<HistoryState>(emptyHistory());
  let musicHistory = $state<HistoryState>(emptyHistory());
  // The tracker's row-range marquee (rows only — a step selection always spans
  // all four channels) and the last row range / whole pattern copied from it.
  let musicSelection = $state<{ from: number; to: number } | null>(null);
  let musicRowClipboard = $state<{ rows: number; cells: number[] } | null>(null);
  let musicPatternClipboard = $state<number[] | null>(null);
  let musicRowDrag = $state(false);
  let tileSelectionReady = $state(false);
  let collisionOverlay = $state(true);
  let mapHover = $state<{ x: number; y: number; tile: number } | null>(null);
  let tool = $state<SpriteTool>('pencil');
  // Hold Space to pan the map with a plain left-drag, so navigating never risks
  // an accidental paint — the universal pan convention outside this app too.
  let spacePan = $state(false);
  let mapWorkEl: HTMLDivElement | undefined = $state();
  let minimapCanvas: HTMLCanvasElement | undefined = $state();
  // Fraction (0..1) of the tile map currently visible in .map-work's scroll viewport.
  let mapViewport = $state({ x: 0, y: 0, w: 1, h: 1 });
  let docQuery = $state('');
  let docCategory = $state<string | null>(null);
  let libraryTab = $state<'local' | 'port'>('local');
  let libraryQuery = $state('');
  let loginName = $state('');
  let loginPassword = $state('');
  let coverCanvas = $state<HTMLCanvasElement>();
  let treeWidth = $state(230);
  let treeResizing = $state(false);
  let projectStatePath = $state('');
  let sourceCursor = $state<Record<string, number>>({});

  function startTreeResize(event: PointerEvent) {
    event.preventDefault();
    treeResizing = true;
    const startX = event.clientX;
    const startWidth = treeWidth;
    const onMove = (moveEvent: PointerEvent) => {
      treeWidth = Math.min(480, Math.max(160, startWidth + (moveEvent.clientX - startX)));
    };
    const onUp = () => {
      treeResizing = false;
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
  }

  $effect(() => {
    if (!coverCanvas || frameData?.length !== SCREEN_RGBA_LEN) return;
    const ctx = coverCanvas.getContext('2d');
    ctx?.putImageData(new ImageData(new Uint8ClampedArray(frameData), SCREEN_WIDTH, SCREEN_HEIGHT), 0, 0);
  });
  // Shared by the sprite rail and map toolbar so both editors present the same
  // order, icons, and keyboard shortcuts for their parity toolset.
  const editorTools: { id: SpriteTool | MapTool; icon: typeof Pencil; shortcut: string; label: string }[] = [
    { id: 'pencil', icon: Pencil, shortcut: 'p', label: 'Pencil' },
    { id: 'line', icon: Minus, shortcut: 'l', label: 'Line' },
    { id: 'rect', icon: Square, shortcut: 'r', label: 'Rectangle' },
    { id: 'fill', icon: PaintBucket, shortcut: 'f', label: 'Fill' },
    { id: 'pick', icon: Pipette, shortcut: 'i', label: 'Pick' },
    { id: 'erase', icon: Eraser, shortcut: 'e', label: 'Erase' },
  ];

  const active = $derived(sources[activeSource]);
  // selectedSprite is always the group's top-left slot; clamp the group so it
  // never reaches past the sheet's right/bottom edge from wherever it's anchored.
  const groupOriginSlot = $derived(selectedSprite);
  const groupCols = $derived(Math.min(spriteGroup.w, SPRITE_SHEET_COLS - (selectedSprite % SPRITE_SHEET_COLS)));
  const groupRows = $derived(Math.min(spriteGroup.h, Math.ceil(256 / SPRITE_SHEET_COLS) - Math.floor(selectedSprite / SPRITE_SHEET_COLS)));
  const groupSprite = $derived(composeGroup(spriteSheet, groupOriginSlot, groupCols, groupRows, SPRITE_SHEET_COLS));
  // Empty slots render as palette[0] on a black sheet, which makes them invisible.
  // Track which ones hold data so the sheet can grey them out instead.
  const spriteUsed = $derived.by(() => {
    const used = new Array<boolean>(256).fill(false);
    for (let index = 0; index < spriteSheet.length; index += 1) {
      if (spriteSheet[index]) used[index >> 6] = true;
    }
    return used;
  });
  $effect(() => {
    if (path === projectStatePath) return;
    projectStatePath = path;
    selectedSprite = 0;
    selectedTile = 0;
    tileSelectionReady = false;
    spriteHistory = emptyHistory();
    mapHistory = emptyHistory();
    paletteHistory = emptyHistory();
    sfxHistory = emptyHistory();
    musicHistory = emptyHistory();
    mapLayer = 'tiles';
    mapTool = 'pencil';
    collisionBrush = 1;
    mapStamp = null;
    mapStampLibrary = [];
    mapSelection = null;
    mapClipboard = null;
    spriteGroup = { w: 1, h: 1 };
    spriteSelection = null;
    spriteClipboard = null;
    musicSelection = null;
    musicRowClipboard = null;
    musicPatternClipboard = null;
    sourceCursor = {};
  });
  $effect(() => {
    if (tileSelectionReady || !spriteSheet.length) return;
    const firstUsed = spriteUsed.findIndex((used, index) => used && index > 0);
    selectedTile = firstUsed >= 0 ? firstUsed : 1;
    tileSelectionReady = true;
  });
  $effect(() => {
    activeSpriteBank;
    spriteHistory = emptyHistory();
  });
  $effect(() => {
    activeMapBank;
    mapHistory = emptyHistory();
    mapHover = null;
  });
  $effect(() => {
    activePaletteBank;
    paletteHistory = emptyHistory();
  });
  $effect(() => {
    activeSfxBank;
    sfxHistory = emptyHistory();
  });
  $effect(() => {
    activeMusicBank;
    musicHistory = emptyHistory();
    musicSelection = null;
  });
  const docCategories = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const entry of api) counts.set(entry.category, (counts.get(entry.category) ?? 0) + 1);
    return [...counts.entries()];
  });
  const activeDocCategory = $derived(docCategory ?? docCategories[0]?.[0] ?? '');
  const filteredApi = $derived(api.filter((entry) =>
    entry.category === activeDocCategory
    && `${entry.name} ${entry.doc}`.toLowerCase().includes(docQuery.toLowerCase()),
  ));

  const mapEmpty = $derived(map.length > 0 && map.every((tile) => tile === 0));

  // How many map cells use each tile, for the "used by" column on Assets.
  const mapTileCounts = $derived.by(() => {
    const counts = new Map<number, number>();
    for (const tile of map) if (tile) counts.set(tile, (counts.get(tile) ?? 0) + 1);
    return counts;
  });

  let assetFilter = $state('');

  function focusAssetFilter() {
    document.querySelector<HTMLInputElement>('#asset-filter')?.focus();
  }

  function assetLabel(entry: { kind: string; id: number }) {
    const id = entry.id.toString().padStart(entry.kind === 'sprite' ? 3 : 2, '0');
    if (entry.kind === 'color') return `${palette[entry.id] ?? 'Colour'} · slot ${id}`;
    return `${entry.kind[0].toUpperCase()}${entry.kind.slice(1)} ${id}`;
  }

  function assetUsage(entry: { kind: string; id: number }) {
    const usage: string[] = [];
    if (entry.kind === 'sprite') {
      const tiles = mapTileCounts.get(entry.id) ?? 0;
      if (tiles) usage.push(`map · ${tiles} ${tiles === 1 ? 'tile' : 'tiles'}`);
    }
    // Colour pixel counts already arrive from the index as a "sprite sheet" ref,
    // so nothing extra to add here.
    return usage;
  }

  /** Collapse repeated references to one pill with a count. */
  function groupRefs(refs: AssetRef[]) {
    const groups = new Map<string, { reference: AssetRef; count: number }>();
    for (const reference of refs) {
      const existing = groups.get(reference.label);
      if (existing) existing.count += 1;
      else groups.set(reference.label, { reference, count: 1 });
    }
    return [...groups.values()];
  }

  function assetScreen(kind: string): Screen {
    if (kind === 'sfx') return 'sfx';
    if (kind === 'music') return 'music';
    if (kind === 'color') return 'palette';
    return 'sprites';
  }

  function openAsset(entry: { kind: string; id: number }) {
    if (entry.kind === 'sprite') selectSprite(entry.id);
    else if (entry.kind === 'sfx') soundSelection.sfx = entry.id;
    else if (entry.kind === 'music') soundSelection.pattern = entry.id;
    else if (entry.kind === 'color') selectedSlot = entry.id;
    onNavigate(assetScreen(entry.kind));
  }

  const noteNames = ['---', ...Array.from({ length: 96 }, (_, i) => `${['C','C#','D','D#','E','F','F#','G','G#','A','A#','B'][(i + 9) % 12]}${Math.floor((i + 9) / 12)}`)];
  const assetStats = $derived(['sprite','sfx','music','color'].map((kind) => {
    const entries = assetIndex.entries.filter((entry) => entry.kind === kind);
    return { kind, used: entries.filter((entry) => entry.used || entry.nonzero).length, count: entries.length, bytes: entries.reduce((sum, entry) => sum + entry.bytes, 0), refs: entries.reduce((sum, entry) => sum + entry.refs.length, 0) };
  }));
  const codeBytes = $derived(sources.reduce((sum, source) => sum + new TextEncoder().encode(source.text).length, 0));
  const artBytes = $derived(spriteSheet.length + map.length + collision.length);
  const soundBytes = $derived(sfx.length + music.length);
  const cartPercent = $derived(Math.min(100, Math.round(cartSize.packedBytes / cartSize.maxBytes * 100)));

  const assetSummary = $derived([
    { label: 'Sprites', icon: Image, value: `${assetStats[0]?.used ?? 0}`, pct: ((assetStats[0]?.used ?? 0) / 256) * 100, detail: 'of 256 slots' },
    { label: 'Map tiles', icon: Layers, value: `${[...mapTileCounts.values()].reduce((sum, n) => sum + n, 0)}`, pct: (([...mapTileCounts.values()].reduce((sum, n) => sum + n, 0)) / (MAP_W * MAP_H)) * 100, detail: `of ${(MAP_W * MAP_H).toLocaleString()} cells` },
    { label: 'Sound effects', icon: Volume2, value: `${assetStats[1]?.used ?? 0}`, pct: ((assetStats[1]?.used ?? 0) / 16) * 100, detail: 'of 16 slots' },
    { label: 'Cart size', icon: Sparkles, value: `${(cartSize.packedBytes / 1024).toFixed(1)} KiB`, pct: cartPercent, detail: `of ${cartSize.maxBytes / 1024} KiB budget` },
  ]);

  const assetRows = $derived.by(() => {
    const needle = assetFilter.trim().toLowerCase();
    return assetIndex.entries
      .filter((entry) => entry.nonzero || entry.used)
      .filter((entry) => !needle
        || assetLabel(entry).toLowerCase().includes(needle)
        || entry.kind.includes(needle)
        || entry.refs.some((reference) => reference.label.toLowerCase().includes(needle)));
  });

  // Splits an edited group canvas back into per-slot writes and records one
  // history entry covering every slot the group touched — undo/redo restore
  // the whole group atomically, not slot by slot.
  function commitGroup(next: number[]) {
    const parts = decomposeGroup(next, groupOriginSlot, groupCols, groupRows, SPRITE_SHEET_COLS);
    const changes = parts
      .map((part) => ({ slot: part.slot, before: spriteSheet.slice(part.slot * 64, part.slot * 64 + 64), after: part.pixels }))
      .filter((change) => change.after.some((value, index) => value !== change.before[index]));
    if (!changes.length) return;
    spriteHistory = pushEntry(spriteHistory, {
      label: changes.length > 1 ? `Sprite group (${changes.length} slots)` : `Sprite ${changes[0].slot.toString().padStart(3, '0')}`,
      undo: () => changes.forEach((change) => onSprite(change.slot, change.before)),
      redo: () => changes.forEach((change) => onSprite(change.slot, change.after)),
    });
    changes.forEach((change) => onSprite(change.slot, change.after));
  }

  function strokeSprite(pixels: Pixel[]) {
    const next = [...groupSprite];
    for (const pixel of pixels) next[pixel.index] = pixel.color;
    commitGroup(next);
  }

  function undoSprite() { spriteHistory = undoEntry(spriteHistory); }
  function redoSpriteEdit() { spriteHistory = redoEntry(spriteHistory); }

  function copySpriteSelection() {
    if (!spriteSelection) return;
    spriteClipboard = { w: spriteSelection.w, h: spriteSelection.h, pixels: regionValues(groupSprite, spriteSelection, groupCols * 8) };
  }

  function cutSpriteSelection() {
    if (!spriteSelection) return;
    copySpriteSelection();
    const gw = groupCols * 8, gh = groupRows * 8;
    const next = [...groupSprite];
    for (let dy = 0; dy < spriteSelection.h; dy += 1) for (let dx = 0; dx < spriteSelection.w; dx += 1) {
      const x = spriteSelection.x0 + dx, y = spriteSelection.y0 + dy;
      if (x < gw && y < gh) next[y * gw + x] = 0;
    }
    commitGroup(next);
  }

  function pasteSpriteClipboard() {
    if (!spriteClipboard) return;
    const gw = groupCols * 8, gh = groupRows * 8;
    const x0 = spriteSelection?.x0 ?? 0, y0 = spriteSelection?.y0 ?? 0;
    const edits = pasteRegion(x0, y0, spriteClipboard.w, spriteClipboard.h, spriteClipboard.pixels, gw, gh);
    const next = [...groupSprite];
    for (const edit of edits) next[edit.index] = edit.value;
    commitGroup(next);
  }

  function commitMap(cells: { offset: number; tile: number }[]) {
    const latest = new globalThis.Map<number, number>();
    for (const cell of cells) latest.set(cell.offset, cell.tile);
    if (mapTool === 'autotile') {
      // The stroke already wrote the picked tile at each seed cell; recompute
      // that cell and its neighbors' edge variants against the post-stroke map.
      const projected = [...map];
      for (const [offset, tile] of latest) projected[offset] = tile;
      for (const auto of autotileEdits(projected, [...latest.keys()], MAP_W, MAP_H)) latest.set(auto.offset, auto.tile);
    }
    const edit = [...latest].map(([offset, after]) => ({ offset, before: map[offset] ?? 0, after }))
      .filter((cell) => cell.before !== cell.after);
    if (!edit.length) return;
    mapHistory = pushEntry(mapHistory, {
      label: 'Map edit',
      undo: () => onMap(edit.map(({ offset, before }) => ({ offset, tile: before }))),
      redo: () => onMap(edit.map(({ offset, after }) => ({ offset, tile: after }))),
    });
    onMap(edit.map(({ offset, after }) => ({ offset, tile: after })));
  }

  function commitCollision(edits: CollisionEdit[]) {
    const latest = new globalThis.Map<number, number>();
    for (const edit of edits) latest.set(edit.offset, edit.value);
    const changes = [...latest]
      .map(([offset, after]) => ({ offset, before: collision[offset] ?? 0, after }))
      .filter((edit) => edit.before !== edit.after);
    if (!changes.length) return;
    mapHistory = pushEntry(mapHistory, {
      label: 'Collision edit',
      undo: () => onCollision(changes.map(({ offset, before }) => ({ offset, value: before }))),
      redo: () => onCollision(changes.map(({ offset, after }) => ({ offset, value: after }))),
    });
    onCollision(changes.map(({ offset, after }) => ({ offset, value: after })));
  }

  // Tile picker: 16-wide, same layout as the sprite sheet, so picking a tile is
  // the same spatial muscle memory as picking a sprite. Marquee-dragging across
  // it (instead of a plain click) selects a rectangular multi-tile stamp.
  const PICKER_COLS = 16;
  const PICKER_ROWS = 16;

  function pickerRect(anchor: number, current: number) {
    const ax = anchor % PICKER_COLS, ay = Math.floor(anchor / PICKER_COLS);
    const cx = current % PICKER_COLS, cy = Math.floor(current / PICKER_COLS);
    const x0 = Math.min(ax, cx), x1 = Math.max(ax, cx);
    const y0 = Math.min(ay, cy), y1 = Math.max(ay, cy);
    return { x0, y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
  }

  function pickerIndexFromEvent(event: PointerEvent): number {
    const rect = pickerEl!.getBoundingClientRect();
    const col = Math.max(0, Math.min(PICKER_COLS - 1, Math.floor(((event.clientX - rect.left) / rect.width) * PICKER_COLS)));
    const row = Math.max(0, Math.min(PICKER_ROWS - 1, Math.floor(((event.clientY - rect.top) / rect.height) * PICKER_ROWS)));
    return row * PICKER_COLS + col;
  }

  function beginPickerDrag(event: PointerEvent) {
    if (event.button !== 0) return;
    const index = pickerIndexFromEvent(event);
    pickerDrag = { anchor: index, current: index };
    pickerEl!.setPointerCapture(event.pointerId);
  }

  function movePickerDrag(event: PointerEvent) {
    if (!pickerDrag) return;
    pickerDrag = { ...pickerDrag, current: pickerIndexFromEvent(event) };
  }

  function finishPickerDrag() {
    if (!pickerDrag) return;
    const { x0, y0, w, h } = pickerRect(pickerDrag.anchor, pickerDrag.current);
    if (w === 1 && h === 1) {
      selectedTile = y0 * PICKER_COLS + x0;
      mapStamp = null;
    } else {
      const tiles: number[] = [];
      for (let dy = 0; dy < h; dy += 1) for (let dx = 0; dx < w; dx += 1) tiles.push((y0 + dy) * PICKER_COLS + (x0 + dx));
      mapStamp = { w, h, tiles };
      selectedTile = tiles[0];
    }
    pickerDrag = null;
  }

  function inStamp(index: number): boolean {
    return mapStamp ? mapStamp.tiles.includes(index) : index === selectedTile;
  }

  function inPickerPreview(index: number): boolean {
    if (!pickerDrag) return false;
    const { x0, y0, w, h } = pickerRect(pickerDrag.anchor, pickerDrag.current);
    const x = index % PICKER_COLS, y = Math.floor(index / PICKER_COLS);
    return x >= x0 && x < x0 + w && y >= y0 && y < y0 + h;
  }

  const collisionTypeById = $derived(new globalThis.Map(collisionTypes.map((t) => [t.id, t])));
  const isBuiltinCollisionType = (id: number) => id === 0 || id === 1 || id === 2;

  function nextCollisionTypeId(): number {
    const used = new Set(collisionTypes.map((t) => t.id));
    for (let id = 3; id <= 255; id += 1) if (!used.has(id)) return id;
    return 255;
  }

  function rgbToHex([r, g, b]: [number, number, number]): string {
    return `#${[r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('')}`;
  }

  function hexToRgb(hex: string): [number, number, number] {
    const n = parseInt(hex.slice(1), 16) || 0;
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }

  function addCollisionType() {
    const id = nextCollisionTypeId();
    onCollisionTypes([...collisionTypes, { id, name: `type_${id}`, color: [128, 128, 128], shape: 'none' }]);
  }

  function updateCollisionType(id: number, patch: Partial<CollisionType>) {
    onCollisionTypes(collisionTypes.map((t) => (t.id === id ? { ...t, ...patch } : t)));
  }

  function removeCollisionType(id: number) {
    if (isBuiltinCollisionType(id)) return;
    onCollisionTypes(collisionTypes.filter((t) => t.id !== id));
    if (collisionBrush === id) collisionBrush = 0;
  }

  function undoMap() { mapHistory = undoEntry(mapHistory); }
  function redoMapEdit() { mapHistory = redoEntry(mapHistory); }

  function regionTiles(region: MapRegion): number[] {
    const tiles: number[] = [];
    for (let dy = 0; dy < region.h; dy += 1) for (let dx = 0; dx < region.w; dx += 1) {
      tiles.push(map[(region.y0 + dy) * MAP_W + (region.x0 + dx)] ?? 0);
    }
    return tiles;
  }

  function copySelection() {
    if (!mapSelection) return;
    mapClipboard = { w: mapSelection.w, h: mapSelection.h, tiles: regionTiles(mapSelection) };
  }

  function cutSelection() {
    if (!mapSelection) return;
    copySelection();
    const cells: { offset: number; tile: number }[] = [];
    for (let dy = 0; dy < mapSelection.h; dy += 1) for (let dx = 0; dx < mapSelection.w; dx += 1) {
      cells.push({ offset: (mapSelection.y0 + dy) * MAP_W + (mapSelection.x0 + dx), tile: 0 });
    }
    // One commitMap call is one history entry, so undo restores the whole cut region.
    commitMap(cells);
  }

  function pasteClipboard() {
    if (!mapClipboard) return;
    // Reuses the stamp-painting path from the tile picker (3a): dropping into
    // pencil with a multi-tile mapStamp means the next click/drag places it.
    mapStamp = mapClipboard;
    mapTool = 'pencil';
  }

  // Drops the clipboard straight into the current selection's top-left corner,
  // one commit, no click needed — the alternative to pasteClipboard's
  // click-to-place stamp when the destination is the active marquee itself.
  function pasteInPlace() {
    if (!mapClipboard || !mapSelection) return;
    const edits = pasteRegion(mapSelection.x0, mapSelection.y0, mapClipboard.w, mapClipboard.h, mapClipboard.tiles, MAP_W, MAP_H);
    commitMap(edits.map(({ index, value }) => ({ offset: index, tile: value })));
  }

  function transformMapSelection(kind: 'flip' | 'vflip' | 'rotate' | 'rotate-ccw') {
    if (!mapSelection) return;
    const { x0, y0, w, h } = mapSelection;
    const values = regionValues(map, mapSelection, MAP_W);
    let next: number[];
    if (kind === 'flip') next = flipHorizontal(values, w, h);
    else if (kind === 'vflip') next = flipVertical(values, w, h);
    else if (w !== h) return; // rotate needs a square selection — see SpriteCanvas's same rule
    else next = kind === 'rotate' ? rotateClockwise(values, w, h) : rotateCounterClockwise(values, w, h);
    const edits = pasteRegion(x0, y0, w, h, next, MAP_W, MAP_H);
    commitMap(edits.map(({ index, value }) => ({ offset: index, tile: value })));
  }

  // Nudges the selection's content by one tile; the selection itself follows,
  // so repeated clicks walk it across the map. No-ops rather than clipping at
  // the map edge — a partial move would silently drop tiles off the far side.
  function moveMapSelection(dx: number, dy: number) {
    if (!mapSelection) return;
    const { x0, y0, w, h } = mapSelection;
    const nx0 = x0 + dx, ny0 = y0 + dy;
    if (nx0 < 0 || ny0 < 0 || nx0 + w > MAP_W || ny0 + h > MAP_H) return;
    const edits = moveRegion(map, x0, y0, w, h, nx0, ny0, MAP_W, MAP_H);
    commitMap(edits.map(({ index, value }) => ({ offset: index, tile: value })));
    mapSelection = { x0: nx0, y0: ny0, w, h };
  }

  // Named, reusable multi-tile stamps — saved from either the tile picker's
  // current pick or the active map selection, kept for the session (not part
  // of the cart; see the plan doc for why this stays session-local for now).
  let mapStampLibrary = $state<{ name: string; w: number; h: number; tiles: number[] }[]>([]);

  function saveStampAs() {
    const source = mapStamp ?? (mapSelection ? { w: mapSelection.w, h: mapSelection.h, tiles: regionTiles(mapSelection) } : null);
    if (!source) return;
    openTextPrompt('Name this stamp', `stamp_${mapStampLibrary.length + 1}`, (name) => {
      mapStampLibrary = [...mapStampLibrary.filter((entry) => entry.name !== name), { name, ...source }];
      return null;
    });
  }

  function loadStamp(name: string) {
    const found = mapStampLibrary.find((entry) => entry.name === name);
    if (!found) return;
    mapStamp = { w: found.w, h: found.h, tiles: found.tiles };
    mapTool = 'pencil';
  }

  function deleteStamp(name: string) {
    mapStampLibrary = mapStampLibrary.filter((entry) => entry.name !== name);
  }

  function handleMapWheel(event: WheelEvent) {
    if (event.deltaY === 0) return;
    event.preventDefault();
    const viewport = event.currentTarget as HTMLDivElement;
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? viewport.clientHeight : 1;
    const delta = Math.max(-240, Math.min(240, event.deltaY * unit));
    const nextZoom = nextMapZoom(mapZoom, delta);
    if (nextZoom === mapZoom) return;

    const canvas = viewport.querySelector<HTMLElement>('[data-map-canvas]');
    if (!canvas) {
      flushSync(() => mapZoom = nextZoom);
      return;
    }

    const before = canvas.getBoundingClientRect();
    const anchorX = Math.max(0, Math.min(1, (event.clientX - before.left) / before.width));
    const anchorY = Math.max(0, Math.min(1, (event.clientY - before.top) / before.height));
    flushSync(() => mapZoom = nextZoom);
    const after = canvas.getBoundingClientRect();
    viewport.scrollLeft += after.left + anchorX * after.width - event.clientX;
    viewport.scrollTop += after.top + anchorY * after.height - event.clientY;
  }

  function applyMapPan() {
    mapPanFrame = undefined;
    if (!mapPan) return;
    mapPan.viewport.scrollLeft = dragPanScroll(mapPan.viewport.scrollLeft, 0, mapPan.pendingX);
    mapPan.viewport.scrollTop = dragPanScroll(mapPan.viewport.scrollTop, 0, mapPan.pendingY);
    mapPan.pendingX = 0;
    mapPan.pendingY = 0;
    updateMapViewport();
  }

  // Mirrors .map-work's scroll position into the minimap's viewport rectangle.
  // Called on every scroll and whenever zoom/bank/screen change the content size
  // without necessarily moving scrollLeft/scrollTop (so no native 'scroll' fires).
  function updateMapViewport() {
    if (!mapWorkEl) return;
    const totalX = MAP_PX_W * mapZoom;
    const totalY = MAP_PX_H * mapZoom;
    mapViewport = {
      x: mapWorkEl.scrollLeft / totalX,
      y: mapWorkEl.scrollTop / totalY,
      w: Math.min(1, mapWorkEl.clientWidth / totalX),
      h: Math.min(1, mapWorkEl.clientHeight / totalY),
    };
  }

  $effect(() => {
    mapZoom; activeMapBank; screen;
    queueMicrotask(updateMapViewport);
  });

  function renderMinimap() {
    if (!minimapCanvas) return;
    const context = minimapCanvas.getContext('2d');
    if (!context) return;
    const image = context.createImageData(MAP_W, MAP_H);
    const colors = palette.map((hex) => {
      const value = hex || '#000000';
      return [parseInt(value.slice(1, 3), 16), parseInt(value.slice(3, 5), 16), parseInt(value.slice(5, 7), 16), 255];
    });
    for (let y = 0; y < MAP_H; y += 1) for (let x = 0; x < MAP_W; x += 1) {
      const tile = map[y * MAP_W + x] ?? 0;
      if (tile === 0) continue;
      // Top-left pixel stands in for the whole tile — enough to read shapes at
      // this scale, and far cheaper than averaging all 64 pixels per tile.
      const paletteIndex = spriteSheet[tile * 64] ?? 0;
      if (paletteIndex === 0) continue;
      const rgba = colors[paletteIndex] ?? colors[0] ?? [0, 0, 0, 255];
      image.data.set(rgba, (y * MAP_W + x) * 4);
    }
    context.putImageData(image, 0, 0);
  }

  $effect(() => {
    map; spriteSheet; palette; minimapCanvas;
    renderMinimap();
  });

  function recenterMapAt(fx: number, fy: number) {
    if (!mapWorkEl) return;
    const totalX = MAP_PX_W * mapZoom;
    const totalY = MAP_PX_H * mapZoom;
    mapWorkEl.scrollLeft = Math.max(0, fx * totalX - mapWorkEl.clientWidth / 2);
    mapWorkEl.scrollTop = Math.max(0, fy * totalY - mapWorkEl.clientHeight / 2);
    updateMapViewport();
  }

  function recenterFromMinimap(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    recenterMapAt(
      Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)),
      Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height)),
    );
  }

  function recenterFromMinimapKey(event: KeyboardEvent) {
    // No pointer position to derive a target cell from a keypress — jump to the
    // map's center, which is at least a useful, predictable destination.
    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); recenterMapAt(0.5, 0.5); }
  }

  function beginMapPan(event: PointerEvent) {
    const panGesture = event.button === 2 || event.button === 1
      || (event.button === 0 && (event.ctrlKey || spacePan));
    if (!panGesture) return;
    event.preventDefault();
    event.stopPropagation();
    const viewport = event.currentTarget as HTMLDivElement;
    mapPan = {
      pointerId: event.pointerId,
      viewport,
      lastX: event.clientX,
      lastY: event.clientY,
      pendingX: 0,
      pendingY: 0,
    };
    mapPanning = true;
    viewport.setPointerCapture(event.pointerId);
  }

  function moveMapPan(event: PointerEvent) {
    if (!mapPan || mapPan.pointerId !== event.pointerId) return;
    event.preventDefault();
    mapPan.pendingX += event.clientX - mapPan.lastX;
    mapPan.pendingY += event.clientY - mapPan.lastY;
    mapPan.lastX = event.clientX;
    mapPan.lastY = event.clientY;
    if (mapPanFrame === undefined) mapPanFrame = requestAnimationFrame(applyMapPan);
  }

  function finishMapPan(event: PointerEvent) {
    if (!mapPan || mapPan.pointerId !== event.pointerId) return;
    if (mapPanFrame !== undefined) cancelAnimationFrame(mapPanFrame);
    applyMapPan();
    const viewport = mapPan.viewport;
    mapPan = null;
    mapPanning = false;
    if (viewport.hasPointerCapture(event.pointerId)) viewport.releasePointerCapture(event.pointerId);
  }

  function loseMapPan(event: PointerEvent) {
    if (!mapPan || mapPan.pointerId !== event.pointerId) return;
    if (mapPanFrame !== undefined) cancelAnimationFrame(mapPanFrame);
    applyMapPan();
    mapPan = null;
    mapPanning = false;
  }

  onDestroy(() => {
    if (mapPanFrame !== undefined) cancelAnimationFrame(mapPanFrame);
  });

  function transformSprite(kind: 'flip' | 'vflip' | 'rotate' | 'rotate-ccw' | 'clear') {
    const gw = groupCols * 8, gh = groupRows * 8;
    if (kind === 'clear') { commitGroup(new Array(gw * gh).fill(0)); return; }
    if (kind === 'flip') { commitGroup(flipHorizontal(groupSprite, gw, gh)); return; }
    if (kind === 'vflip') { commitGroup(flipVertical(groupSprite, gw, gh)); return; }
    // Rotate only makes sense on a square group — a non-square block would need
    // a differently-shaped canvas after rotating, which the fixed slot grid can't hold.
    if (groupCols !== groupRows) return;
    if (kind === 'rotate') commitGroup(rotateClockwise(groupSprite, gw, gh));
    if (kind === 'rotate-ccw') commitGroup(rotateCounterClockwise(groupSprite, gw, gh));
  }

  function selectSprite(index: number) {
    // No history reset: entries capture their own sprite id, so undo/redo stay
    // valid even after switching which sprite is on screen.
    selectedSprite = index;
    spriteGroup = { w: 1, h: 1 };
    spriteSelection = null;
  }

  // Sprite sheet: shift-drag across slots picks an N x M group canvas, the
  // same spatial gesture as the map's tile-picker marquee above.
  function spriteSheetIndexFromEvent(event: PointerEvent): number {
    const rect = spriteSheetEl!.getBoundingClientRect();
    const col = Math.max(0, Math.min(SPRITE_SHEET_COLS - 1, Math.floor(((event.clientX - rect.left) / rect.width) * SPRITE_SHEET_COLS)));
    const row = Math.max(0, Math.min(15, Math.floor(((event.clientY - rect.top) / rect.height) * 16)));
    return row * SPRITE_SHEET_COLS + col;
  }

  function beginSpriteSheetDrag(event: PointerEvent) {
    if (event.button !== 0) return;
    const index = spriteSheetIndexFromEvent(event);
    spriteSheetDrag = { anchor: index, current: index };
    spriteSheetEl!.setPointerCapture(event.pointerId);
  }

  function moveSpriteSheetDrag(event: PointerEvent) {
    if (!spriteSheetDrag) return;
    spriteSheetDrag = { ...spriteSheetDrag, current: spriteSheetIndexFromEvent(event) };
  }

  function finishSpriteSheetDrag() {
    if (!spriteSheetDrag) return;
    const ax = spriteSheetDrag.anchor % SPRITE_SHEET_COLS, ay = Math.floor(spriteSheetDrag.anchor / SPRITE_SHEET_COLS);
    const cx = spriteSheetDrag.current % SPRITE_SHEET_COLS, cy = Math.floor(spriteSheetDrag.current / SPRITE_SHEET_COLS);
    const x0 = Math.min(ax, cx), y0 = Math.min(ay, cy);
    const w = Math.abs(ax - cx) + 1, h = Math.abs(ay - cy) + 1;
    selectedSprite = y0 * SPRITE_SHEET_COLS + x0;
    spriteGroup = { w, h };
    spriteSelection = null;
    spriteSheetDrag = null;
  }

  function inSpriteSheetGroup(index: number): boolean {
    const x = index % SPRITE_SHEET_COLS, y = Math.floor(index / SPRITE_SHEET_COLS);
    const ox = selectedSprite % SPRITE_SHEET_COLS, oy = Math.floor(selectedSprite / SPRITE_SHEET_COLS);
    return x >= ox && x < ox + groupCols && y >= oy && y < oy + groupRows;
  }

  function inSpriteSheetDragPreview(index: number): boolean {
    if (!spriteSheetDrag) return false;
    const ax = spriteSheetDrag.anchor % SPRITE_SHEET_COLS, ay = Math.floor(spriteSheetDrag.anchor / SPRITE_SHEET_COLS);
    const cx = spriteSheetDrag.current % SPRITE_SHEET_COLS, cy = Math.floor(spriteSheetDrag.current / SPRITE_SHEET_COLS);
    const x0 = Math.min(ax, cx), x1 = Math.max(ax, cx), y0 = Math.min(ay, cy), y1 = Math.max(ay, cy);
    const x = index % SPRITE_SHEET_COLS, y = Math.floor(index / SPRITE_SHEET_COLS);
    return x >= x0 && x <= x1 && y >= y0 && y <= y1;
  }

  // Each sfx slot is 16 steps x 4 bytes: note, volume, wave (0 square / 1 noise),
  // byte3 (pan + attack/release envelope, see PAN_LABELS/ENV_LABELS below).
  // Note 0 is a rest; notes run 1..96 (C0..B7) via note_to_freq in the VM.
  const SFX_NOTE_MAX = 96;
  const SFX_VOLUME_MAX = 15;
  // Octave marks up the pitch axis, positioned by note value.
  const pitchAxis = Array.from({ length: 6 }, (_, i) => {
    const note = (i + 2) * 12 + 1;
    return { name: `C${i + 2}`, at: (note / SFX_NOTE_MAX) * 100 };
  });

  const sfxSlotFilled = $derived(Array.from({ length: 16 }, (_, slot) =>
    sfx.slice(slot * 64, slot * 64 + 64).some(Boolean)));
  const sfxPlaying = $derived(audio.sfxActive && audio.sfxId === selectedSfx);
  const musicPlaying = $derived(audio.musicActive && audio.musicPattern === selectedPattern);

  function selectEmptySfx() {
    const empty = sfxSlotFilled.findIndex((filled) => !filled);
    soundSelection.sfx = empty >= 0 ? empty : (selectedSfx + 1) % 16;
  }

  function selectEmptyPattern() {
    const empty = Array.from({ length: MUSIC_PATTERN_COUNT }, (_, pattern) => patternBytes(pattern).some(Boolean)).findIndex((filled) => !filled);
    soundSelection.pattern = empty >= 0 ? empty : (selectedPattern + 1) % MUSIC_PATTERN_COUNT;
  }

  /** One pattern's cells: MUSIC_PATTERN_ROWS rows of MUSIC_CHANNEL_COUNT bytes. */
  const patternBytes = (pattern: number) => music.slice(pattern * MUSIC_PATTERN_LEN, (pattern + 1) * MUSIC_PATTERN_LEN);

  const sfxByte = (step: number, field: number) => sfx[selectedSfx * 64 + step * 4 + field] ?? 0;
  const sfxStepActive = (step: number) => sfxPlaying && audio.sfxStep === step;

  // byte3 packs pan (bits 0-3, 0=center) and attack/release envelope levels
  // (bits 4-5 / 6-7, each 0-3). Mirrors crates/caiven-vm/src/vm/sfx.rs::decode_byte3.
  const PAN_LABELS = ['C', 'L1', 'R1', 'L2', 'R2', 'L3', 'R3', 'L4', 'R4', 'L5', 'R5', 'L6', 'R6', 'L7', 'R7', 'HL'];
  const ENV_LABELS = ['—', 'fast', 'med', 'slow'];

  const sfxPan = (step: number) => sfxByte(step, 3) & 0x0f;
  const sfxAttack = (step: number) => (sfxByte(step, 3) >> 4) & 0x03;
  const sfxRelease = (step: number) => (sfxByte(step, 3) >> 6) & 0x03;

  function packByte3(pan: number, attack: number, release: number) {
    return (pan & 0x0f) | ((attack & 0x03) << 4) | ((release & 0x03) << 6);
  }

  function setSfxPan(step: number, pan: number) {
    const current = sfxByte(step, 3);
    setSfxCells([{ step, field: 3, value: packByte3(pan, (current >> 4) & 0x03, (current >> 6) & 0x03) }]);
  }

  function setSfxAttack(step: number, attack: number) {
    const current = sfxByte(step, 3);
    setSfxCells([{ step, field: 3, value: packByte3(current & 0x0f, attack, (current >> 6) & 0x03) }]);
  }

  function setSfxRelease(step: number, release: number) {
    const current = sfxByte(step, 3);
    setSfxCells([{ step, field: 3, value: packByte3(current & 0x0f, (current >> 4) & 0x03, release) }]);
  }

  function setSfxCells(cells: { step: number; field: number; value: number }[]) {
    const bytes = sfx.slice(selectedSfx * 64, selectedSfx * 64 + 64);
    let changed = false;
    for (const { step, field, value } of cells) {
      const at = step * 4 + field;
      if ((bytes[at] ?? 0) === value) continue;
      bytes[at] = value;
      changed = true;
      // A note with no volume is silent, which reads as "drawing did nothing".
      if (field === 0 && value > 0 && (bytes[step * 4 + 1] ?? 0) === 0) bytes[step * 4 + 1] = SFX_VOLUME_MAX;
    }
    if (!changed) return;
    // Discrete edits (wave/fx button clicks) aren't part of a drag stroke, so they
    // record their own history entry immediately. Drag strokes record one entry
    // for the whole gesture in endSfxDraw instead — see sfxStrokeBefore below.
    if (!sfxDrawing) {
      const slot = selectedSfx;
      const before = sfx.slice(slot * 64, slot * 64 + 64);
      const after = [...bytes];
      sfxHistory = pushEntry(sfxHistory, {
        label: `SFX ${slot.toString().padStart(2, '0')}`,
        undo: () => onSfx(slot, before),
        redo: () => onSfx(slot, after),
      });
    }
    onSfx(selectedSfx, bytes);
  }

  function undoSfx() { sfxHistory = undoEntry(sfxHistory); }
  function redoSfx() { sfxHistory = redoEntry(sfxHistory); }

  let sfxDrawing = $state<{ field: number; erase: boolean } | null>(null);
  let sfxStrokeBefore: number[] | null = null;
  let sfxStrokeSlot = 0;

  function sfxCellFromEvent(event: PointerEvent, field: number) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const step = Math.max(0, Math.min(15, Math.floor(((event.clientX - rect.left) / rect.width) * 16)));
    const ratio = 1 - (event.clientY - rect.top) / rect.height;
    const max = field === 0 ? SFX_NOTE_MAX : SFX_VOLUME_MAX;
    const floor = field === 0 ? 1 : 0;
    const value = Math.max(floor, Math.min(max, Math.round(ratio * max)));
    return { step, value };
  }

  function beginSfxDraw(event: PointerEvent, field: number) {
    // Secondary button (or ctrl-click on macOS) erases.
    const erase = event.button === 2 || event.ctrlKey;
    sfxStrokeSlot = selectedSfx;
    sfxStrokeBefore = sfx.slice(sfxStrokeSlot * 64, sfxStrokeSlot * 64 + 64);
    sfxDrawing = { field, erase };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    applySfxDraw(event, field);
  }

  function continueSfxDraw(event: PointerEvent, field: number) {
    if (!sfxDrawing || sfxDrawing.field !== field) return;
    applySfxDraw(event, field);
  }

  function applySfxDraw(event: PointerEvent, field: number) {
    const { step, value } = sfxCellFromEvent(event, field);
    if (sfxDrawing?.erase) {
      setSfxCells(field === 0
        ? [{ step, field: 0, value: 0 }, { step, field: 1, value: 0 }]
        : [{ step, field: 1, value: 0 }]);
      return;
    }
    setSfxCells([{ step, field, value }]);
  }

  function endSfxDraw() {
    if (sfxStrokeBefore) {
      const before = sfxStrokeBefore;
      const slot = sfxStrokeSlot;
      const after = sfx.slice(slot * 64, slot * 64 + 64);
      if (!after.every((value, index) => value === before[index])) {
        sfxHistory = pushEntry(sfxHistory, {
          label: `SFX ${slot.toString().padStart(2, '0')}`,
          undo: () => onSfx(slot, before),
          redo: () => onSfx(slot, after),
        });
      }
    }
    sfxDrawing = null;
    sfxStrokeBefore = null;
  }

  function changeMusic(row: number, channel: number) {
    const pattern = selectedPattern;
    const before = patternBytes(pattern);
    const after = [...before];
    const at = row * MUSIC_CHANNEL_COUNT + channel;
    after[at] = ((after[at] ?? 0) + 1) % 17;
    musicHistory = pushEntry(musicHistory, {
      label: `Pattern ${pattern.toString().padStart(2, '0')}`,
      undo: () => onMusic(pattern, before),
      redo: () => onMusic(pattern, after),
    });
    onMusic(pattern, after);
  }

  /** Commits a whole pattern as one undo entry — the shared tail of every
   *  music edit that isn't a single cell cycle. */
  function commitPattern(pattern: number, before: number[], after: number[]) {
    if (after.every((value, index) => value === before[index])) return;
    musicHistory = pushEntry(musicHistory, {
      label: `Pattern ${pattern.toString().padStart(2, '0')}`,
      undo: () => onMusic(pattern, before),
      redo: () => onMusic(pattern, after),
    });
    onMusic(pattern, after);
  }

  /** Normalized row range of the tracker marquee, low row first. */
  const musicRows = $derived(musicSelection
    ? { from: Math.min(musicSelection.from, musicSelection.to), to: Math.max(musicSelection.from, musicSelection.to) }
    : null);
  const musicRowSelected = (row: number) => !!musicRows && row >= musicRows.from && row <= musicRows.to;

  function startRowSelect(row: number, extend: boolean) {
    musicSelection = extend && musicSelection ? { from: musicSelection.from, to: row } : { from: row, to: row };
    musicRowDrag = true;
  }

  function extendRowSelect(row: number) {
    if (musicRowDrag && musicSelection) musicSelection = { from: musicSelection.from, to: row };
  }

  function copyMusicRows() {
    if (!musicRows) return;
    const rows = musicRows.to - musicRows.from + 1;
    musicRowClipboard = {
      rows,
      cells: regionValues(patternBytes(selectedPattern), { x0: 0, y0: musicRows.from, w: MUSIC_CHANNEL_COUNT, h: rows }, MUSIC_CHANNEL_COUNT),
    };
  }

  function cutMusicRows() {
    if (!musicRows) return;
    copyMusicRows();
    const before = patternBytes(selectedPattern);
    const after = [...before];
    for (let row = musicRows.from; row <= musicRows.to; row += 1) {
      for (let channel = 0; channel < MUSIC_CHANNEL_COUNT; channel += 1) after[row * MUSIC_CHANNEL_COUNT + channel] = 0;
    }
    commitPattern(selectedPattern, before, after);
  }

  /** Pastes the copied rows starting at the selection's first row (or row 0
   *  with nothing selected); rows past the end of the pattern are clipped. */
  function pasteMusicRows() {
    if (!musicRowClipboard) return;
    const before = patternBytes(selectedPattern);
    const after = [...before];
    const edits = pasteRegion(0, musicRows?.from ?? 0, MUSIC_CHANNEL_COUNT, musicRowClipboard.rows, musicRowClipboard.cells, MUSIC_CHANNEL_COUNT, MUSIC_PATTERN_ROWS);
    for (const edit of edits) after[edit.index] = edit.value;
    commitPattern(selectedPattern, before, after);
  }

  function copyPattern() { musicPatternClipboard = patternBytes(selectedPattern); }

  function pastePattern() {
    if (!musicPatternClipboard) return;
    commitPattern(selectedPattern, patternBytes(selectedPattern), [...musicPatternClipboard]);
  }

  /** One song-order slot: 0 is empty, otherwise `pattern id + 1`. */
  const songStep = (step: number) => music[MUSIC_ORDER_OFFSET + step] ?? 0;
  const songLoopStep = $derived((music[MUSIC_ORDER_OFFSET + MUSIC_ORDER_STEPS] ?? 0) - 1);

  /** Click cycles a slot through every pattern then back to empty — the same
   *  interaction the tracker's own cells use. */
  function changeSongStep(step: number) {
    onSong(step, [(songStep(step) + 1) % (MUSIC_PATTERN_COUNT + 1)]);
  }

  /** Only one step can be the loop point, so setting a new one replaces it;
   *  clicking the current loop point clears it. The byte is `step + 1`. */
  function toggleSongLoop(step: number) {
    onSong(MUSIC_ORDER_STEPS, [songLoopStep === step ? 0 : step + 1]);
  }

  function undoMusic() { musicHistory = undoEntry(musicHistory); }
  function redoMusic() { musicHistory = redoEntry(musicHistory); }

  function jumpToRef(reference: { path: string; line?: number; col?: number }) {
    onOpenSource(reference.path, reference.line ?? null, reference.col ?? null);
  }

  let paletteStrokeBefore: string | null = null;

  function beginPaletteEdit() {
    paletteStrokeBefore = palette[selectedSlot];
  }

  function commitPaletteEdit() {
    const before = paletteStrokeBefore;
    paletteStrokeBefore = null;
    if (before === null) return;
    const slot = selectedSlot;
    const after = palette[slot];
    if (before === after) return;
    paletteHistory = pushEntry(paletteHistory, {
      label: `Palette ${slot.toString().padStart(2, '0')}`,
      undo: () => onPalette(slot, before),
      redo: () => onPalette(slot, after),
    });
  }

  function undoPalette() { paletteHistory = undoEntry(paletteHistory); }
  function redoPalette() { paletteHistory = redoEntry(paletteHistory); }

  function updatePalette(hex: string) {
    if (/^#[0-9a-f]{6}$/i.test(hex)) onPalette(selectedSlot, hex.toUpperCase());
  }

  function updateChannel(channel: number, value: number) {
    const channels = [0, 1, 2].map((index) => parseInt(palette[selectedSlot].slice(1 + index * 2, 3 + index * 2), 16));
    channels[channel] = value;
    updatePalette(`#${channels.map((part) => part.toString(16).padStart(2, '0')).join('')}`);
  }

  const BANK_NAME_PATTERN = /^[A-Za-z0-9_-]{1,31}$/;

  /** `window.prompt` is unimplemented in the Tauri webview (WKWebView never
   *  shows a JS text-input panel unless the host app wires one up, so the
   *  call just returns null with no dialog at all) — a self-drawn dialog is
   *  the only way to ask for typed text inside Studio. */
  interface TextPrompt { title: string; value: string; error: string; submit: (value: string) => string | null; }
  let textPrompt = $state<TextPrompt | null>(null);

  function openTextPrompt(title: string, initial: string, submit: (value: string) => string | null) {
    textPrompt = { title, value: initial, error: '', submit };
  }

  function submitTextPrompt() {
    if (!textPrompt) return;
    const value = textPrompt.value.trim();
    if (!value) { textPrompt.error = 'Required.'; return; }
    const error = textPrompt.submit(value);
    if (error) { textPrompt.error = error; return; }
    textPrompt = null;
  }

  /// Opens the rename dialog for a new bank; the submit callback re-checks
  /// the same charset the Rust decoder enforces (letters, digits, `_`, `-`,
  /// 1-31 chars) so a bad name never round-trips through the backend just to
  /// bounce back as an error toast.
  function promptNewBankName(kind: 'sprites' | 'map' | 'palette' | 'sfx' | 'music') {
    openTextPrompt(`Name the new ${kind} bank`, '', (name) => {
      if (!BANK_NAME_PATTERN.test(name)) return '1-31 letters, digits, _, or -.';
      onAssetBank(kind, 'create', name);
      return null;
    });
  }

  function signature(entry: ApiEntry) {
    return `${entry.name}(${entry.params.map((p) => `${p.name}: ${p.ty}`).join(', ')})`;
  }

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;
  }

  // The code screen owns its own undo via CodeMirror's historyKeymap — it has no
  // case here, so Mod-Z falls through untouched for it.
  const activeHistory = $derived.by((): HistoryState | null => {
    switch (screen) {
      case 'sprites': return spriteHistory;
      case 'map': return mapHistory;
      case 'palette': return paletteHistory;
      case 'sfx': return sfxHistory;
      case 'music': return musicHistory;
      default: return null;
    }
  });

  /** Undo/redo for whichever of the five asset editors is currently on screen.
   *  Exported so App can wire it into the ⌘K command palette, in addition to the
   *  per-editor toolbar buttons and the Ctrl+Z/Ctrl+Shift+Z shortcut below. */
  export function undoActive() {
    if (screen === 'sprites') undoSprite();
    else if (screen === 'map') undoMap();
    else if (screen === 'palette') undoPalette();
    else if (screen === 'sfx') undoSfx();
    else if (screen === 'music') undoMusic();
  }

  export function redoActive() {
    if (screen === 'sprites') redoSpriteEdit();
    else if (screen === 'map') redoMapEdit();
    else if (screen === 'palette') redoPalette();
    else if (screen === 'sfx') redoSfx();
    else if (screen === 'music') redoMusic();
  }

  $effect(() => {
    onHistoryStatus?.({ canUndo: !!activeHistory?.undo.length, canRedo: !!activeHistory?.redo.length });
  });

  function handleWorkspaceKeys(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z' && activeHistory) {
      event.preventDefault();
      if (event.shiftKey) redoActive(); else undoActive();
      return;
    }
    if ((event.metaKey || event.ctrlKey) && screen === 'map' && !isTypingTarget(event.target)) {
      const key = event.key.toLowerCase();
      if (key === 'c' && mapSelection) { event.preventDefault(); copySelection(); return; }
      if (key === 'x' && mapSelection) { event.preventDefault(); cutSelection(); return; }
      if (key === 'v' && event.shiftKey && mapClipboard && mapSelection) { event.preventDefault(); pasteInPlace(); return; }
      if (key === 'v' && mapClipboard) { event.preventDefault(); pasteClipboard(); return; }
    }
    if ((event.metaKey || event.ctrlKey) && screen === 'sprites' && !isTypingTarget(event.target)) {
      const key = event.key.toLowerCase();
      if (key === 'c' && spriteSelection) { event.preventDefault(); copySpriteSelection(); return; }
      if (key === 'x' && spriteSelection) { event.preventDefault(); cutSpriteSelection(); return; }
      if (key === 'v' && spriteClipboard) { event.preventDefault(); pasteSpriteClipboard(); return; }
    }
    if ((event.metaKey || event.ctrlKey) && screen === 'music' && !isTypingTarget(event.target)) {
      const key = event.key.toLowerCase();
      if (key === 'c' && musicRows) { event.preventDefault(); copyMusicRows(); return; }
      if (key === 'x' && musicRows) { event.preventDefault(); cutMusicRows(); return; }
      if (key === 'v' && musicRowClipboard) { event.preventDefault(); pasteMusicRows(); return; }
    }
    if (event.key === ' ' && screen === 'map' && !isTypingTarget(event.target)) {
      event.preventDefault();
      spacePan = true;
      return;
    }
    if (event.metaKey || event.ctrlKey || event.altKey || isTypingTarget(event.target)) return;
    if (screen !== 'sprites' && screen !== 'map') return;
    const match = editorTools.find((item) => item.shortcut === event.key.toLowerCase());
    if (!match) return;
    event.preventDefault();
    if (screen === 'sprites') tool = match.id as SpriteTool;
    else mapTool = match.id as MapTool;
  }

  function handleWorkspaceKeyUp(event: KeyboardEvent) {
    if (event.key === ' ') spacePan = false;
  }

  $effect(() => {
    if (screen !== 'map') spacePan = false;
  });
</script>

<svelte:window onkeydown={handleWorkspaceKeys} onkeyup={handleWorkspaceKeyUp} onblur={() => spacePan = false} onpointerup={() => musicRowDrag = false} />

<main class="workspace">
  {#if ['sprites', 'map', 'palette'].includes(screen)}
    <nav class="subnav">
      <Button variant="ghost" class={screen === 'sprites' ? 'active' : undefined} onclick={() => onNavigate('sprites')}><Image size={15} />Sprites</Button>
      <Button variant="ghost" class={screen === 'map' ? 'active' : undefined} onclick={() => onNavigate('map')}><Layers size={15} />Map</Button>
      <Button variant="ghost" class={screen === 'palette' ? 'active' : undefined} onclick={() => onNavigate('palette')}><Pipette size={15} />Palette</Button>
      {#if screen === 'sprites' || screen === 'map' || screen === 'palette'}
        {@const bankKind = screen === 'sprites' ? 'sprites' : screen === 'map' ? 'map' : 'palette'}
        {@const bankNames = screen === 'sprites' ? spriteBanks : screen === 'map' ? mapBanks : paletteBanks}
        {@const activeBank = screen === 'sprites' ? activeSpriteBank : screen === 'map' ? activeMapBank : activePaletteBank}
        <div class="bank-picker">
          <span>Bank</span>
          <select value={activeBank} onchange={async (event) => { const select = event.currentTarget; if (await onAssetBank(bankKind, 'select', select.value) === false) select.value = activeBank; }}>
            {#each bankNames as name}<option value={name}>{name}</option>{/each}
          </select>
          <button title={`Create ${bankKind} bank`} onclick={() => promptNewBankName(bankKind)}><Plus size={14} /></button>
          <button class="danger" disabled={activeBank === 'default'} title={`Delete ${bankKind} bank ${activeBank}`} onclick={() => onAssetBank(bankKind, 'delete', activeBank)}><Trash2 size={14} /></button>
        </div>
      {/if}
      <code>{screen === 'sprites' ? `${assetStats[0]?.used ?? 0} of 256 used` : screen === 'map' ? `${MAP_W} × ${MAP_H} tiles` : '16 colors'}</code>
    </nav>
  {:else if ['sfx', 'music'].includes(screen)}
    <nav class="subnav">
      <Button variant="ghost" class={screen === 'sfx' ? 'active' : undefined} onclick={() => onNavigate('sfx')}><Volume2 size={15} />Sound effects</Button>
      <Button variant="ghost" class={screen === 'music' ? 'active' : undefined} onclick={() => onNavigate('music')}><Music size={15} />Music</Button>
      {#if screen === 'sfx' || screen === 'music'}
        {@const bankKind = screen === 'sfx' ? 'sfx' : 'music'}
        {@const bankNames = screen === 'sfx' ? sfxBanks : musicBanks}
        {@const activeBank = screen === 'sfx' ? activeSfxBank : activeMusicBank}
        <div class="bank-picker">
          <span>Bank</span>
          <select value={activeBank} onchange={async (event) => { const select = event.currentTarget; if (await onAssetBank(bankKind, 'select', select.value) === false) select.value = activeBank; }}>
            {#each bankNames as name}<option value={name}>{name}</option>{/each}
          </select>
          <button title={`Create ${bankKind} bank`} onclick={() => promptNewBankName(bankKind)}><Plus size={14} /></button>
          <button class="danger" disabled={activeBank === 'default'} title={`Delete ${bankKind} bank ${activeBank}`} onclick={() => onAssetBank(bankKind, 'delete', activeBank)}><Trash2 size={14} /></button>
        </div>
      {/if}
      <code>{screen === 'sfx' ? `${assetStats[1]?.used ?? 0} of 16 slots used` : `${assetStats[2]?.used ?? 0} of 8 patterns`}</code>
    </nav>
  {/if}

  {#if screen === 'welcome'}
    <section class="welcome-screen">
      <div class="welcome-glow"></div>
      <div class="welcome-copy">
        <span class="eyebrow">Caiven Studio</span>
        <h1>Make small worlds.<br /><em>Keep every pixel.</em></h1>
        <p>Write real Lua, draw directly into cart memory, and publish something playable before idea cools.</p>
        <div class="welcome-actions">
          <Button onclick={onNew}><Plus size={16} />New cart</Button>
          <Button variant="outline" onclick={onOpen}><FolderOpen size={16} />Open project</Button>
        </div>
      </div>
      {#if !tourDone}
        <aside class="tour-card">
          <div class="tour-steps-mini">
            {#each [['1','Write'],['2','Run'],['3','Draw'],['4','Ship']] as step, i}
              <div><i>{step[0]}</i><span><strong>{step[1]}</strong><small>{['Real Lua, familiar tools.','See every change instantly.','Paint sprites and maps.','Pack or publish.'][i]}</small></span></div>
            {/each}
          </div>
          <Button variant="ghost" onclick={onTour}>Take 4-step tour<ArrowRight size={15} /></Button>
        </aside>
      {/if}
      <div class="examples-section">
        <div class="section-heading"><span><strong>Examples</strong><small>Real, playable carts. Click one to open an editable copy.</small></span></div>
        <div class="examples-grid">
          {#each examples as example (example.id)}
            <button type="button" class="example-card" onclick={() => onRemix(example.id)}>
              <span class="example-icon" aria-hidden="true">
                {#if example.id === 'catch'}<Volume2 size={18} />{:else if example.id === 'tiles'}<Grid3X3 size={18} />{:else if example.id === 'stdlib-demo'}<Sparkles size={18} />{:else}<Gamepad2 size={18} />{/if}
              </span>
              <span class="example-copy"><strong>{example.name}</strong><small>{example.description}</small></span>
              <i class="example-remix">Remix<ArrowRight size={13} /></i>
            </button>
          {/each}
        </div>
      </div>
      <div class="recent-section">
        <div class="section-heading"><span><strong>Recent carts</strong><small>Pick up where you left off.</small></span><Button variant="ghost" onclick={() => onNavigate('library')}>See library <ChevronRight size={14} /></Button></div>
        <div class="recent-grid">
          {#each recent.slice(0, 4) as item, i (item)}
            <article class="recent-card">
              <button class="recent-open" onclick={() => onOpenLocal(item)}>
                <span class="mini-cover" style={`--seed:${i}`}>
                  {#each Array(64) as _, p}<i style={`background:${palette[(p * 7 + i * 3) % 16]}`}></i>{/each}
                </span>
                <span><strong>{item.split('/')[item.split('/').length - 1]}</strong><code>{item}</code></span>
                <small>Recent</small>
              </button>
              <button class="recent-remove" aria-label={`Remove ${item} from recent carts`} title="Remove from recent carts" onclick={() => onRemoveRecent(item)}><X size={14} /></button>
            </article>
          {/each}
          {#if recent.length === 0}<div class="recent-empty"><strong>No recent carts</strong><span>Opened projects appear here.</span></div>{/if}
        </div>
      </div>
    </section>

  {:else if screen === 'code'}
    <section class="code-screen" style={`--tree-width:${treeWidth}px`}>
      <aside class="project-tree">
        <div class="panel-cap"><span class="eyebrow">Project</span><button title="New module" onclick={onCreateModule}><Plus size={14} /></button></div>
        <div class="tree-files">
          <div class="tree-root"><ChevronRight size={12} class="tree-open" /><strong>{title || 'cart'}</strong></div>
          {#each sources as source, index}
            <button class:active={index === activeSource} onclick={() => onSource(index)}>
              <FileCode2 size={14} />
              <span>{source.name}</span>
              {#if source.dirty}<i></i>{/if}
            </button>
          {/each}
          <button onclick={() => onNavigate('sprites')}><FileImage size={14} /><span>sprites.png</span></button>
          <button onclick={() => onNavigate('map')}><FileImage size={14} /><span>map.png</span></button>
          <button onclick={() => onNavigate('palette')}><Pipette size={14} /><span>palette.png</span></button>
        </div>
        <div class="budget-card">
          <span class="eyebrow">Cart budget</span>
          <div><i style={`width:${cartPercent}%`}></i></div><code>{(cartSize.packedBytes / 1024).toFixed(1)} / {cartSize.maxBytes / 1024} KiB</code>
          <small>Code {(codeBytes / 1024).toFixed(1)} KiB · Art {(artBytes / 1024).toFixed(1)} KiB · Sound {(soundBytes / 1024).toFixed(1)} KiB</small>
        </div>
      </aside>
      <div
        class="pane-resizer"
        class:dragging={treeResizing}
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize project tree"
        onpointerdown={startTreeResize}
      ></div>
      <div class="editor-shell" data-tour-target="write">
        <div class="editor-tabs">
          {#each sources as source, index}
            <button class:active={index === activeSource} onclick={() => onSource(index)}>
              {source.name}{#if source.dirty}<i></i>{/if}
            </button>
          {/each}
          <button class="new-tab" title="New Lua module" onclick={onCreateModule}>+</button>
        </div>
        <div class="breadcrumbs"><span>{title}</span><b>›</b><span>src</span><b>›</b><strong>{active?.name}</strong><code>Lua 5.4</code></div>
        <div class="code-editor">
          {#if LuaEditorComponent}
            {#key active?.path ?? active?.name ?? ''}
              <LuaEditorComponent
                value={active?.text ?? ''}
                path={active?.name ?? ''}
                initialCursor={sourceCursor[active?.name ?? ''] ?? 0}
                {api}
                {preludeModules}
                {diagnostics}
                {breakpoints}
                {insertRequest}
                {revealRequest}
                {onInsertHandled}
                {onRevealHandled}
                onChange={onCode}
                onCursor={(source, offset) => sourceCursor[source] = offset}
                onToggleBreakpoint={onBreakpoint}
                onEnableModule={(module) => onSetStdlibModule(module, true)}
              />
            {/key}
          {:else}
            <div class="editor-loading">Loading editor…</div>
          {/if}
        </div>
        {#if diagnostics[0]}
          <div class="inline-diagnostic"><span>{diagnostics[0].path}:{diagnostics[0].line ?? '?'}</span><strong>{diagnostics[0].title}</strong><p>{diagnostics[0].detail}</p></div>
        {/if}
      </div>
    </section>

  {:else if screen === 'sprites'}
    <section class="asset-editor sprite-editor">
      <aside class="tool-rail">
        {#each editorTools as item}
          {@const Icon = item.icon}
          <button class:active={tool === item.id} title={`${item.label} (${item.shortcut})`} onclick={() => tool = item.id as SpriteTool}><Icon size={18} /></button>
        {/each}
        <button
          class:active={tool === 'select'}
          title="Select — marquee a region, then Ctrl+C/X to copy/cut, Ctrl+V to paste"
          onclick={() => tool = 'select'}
        ><BoxSelect size={18} /></button>
        <span></span>
        <button title="Undo sprite edit" disabled={!spriteHistory.undo.length} onclick={undoSprite}><Undo2 size={18} /></button><button title="Redo sprite edit" disabled={!spriteHistory.redo.length} onclick={redoSpriteEdit}><Redo2 size={18} /></button>
        <button title="Flip horizontally" onclick={() => transformSprite('flip')}><FlipHorizontal size={18} /></button>
        <button title="Flip vertically" onclick={() => transformSprite('vflip')}><FlipVertical size={18} /></button>
        <button title={groupCols === groupRows ? 'Rotate clockwise' : 'Rotate needs a square group'} disabled={groupCols !== groupRows} onclick={() => transformSprite('rotate')}><RotateCw size={18} /></button>
        <button title={groupCols === groupRows ? 'Rotate counter-clockwise' : 'Rotate needs a square group'} disabled={groupCols !== groupRows} onclick={() => transformSprite('rotate-ccw')}><RotateCcw size={18} /></button>
        <button class="danger" title="Clear sprite" onclick={() => transformSprite('clear')}><Trash2 size={18} /></button>
      </aside>
      <div class="asset-canvas-wrap" data-tour-target="draw">
        <div class="asset-heading">
          <span><span class="eyebrow">Sprite</span><strong>{selectedSprite.toString().padStart(3,'0')}</strong>{#if groupCols > 1 || groupRows > 1}<em>{groupCols} × {groupRows} group</em>{/if}</span>
          <code>{groupCols * 8} × {groupRows * 8} px · {groupCols * groupRows * 64} bytes</code>
        </div>
        <SpriteCanvas
          sprite={groupSprite}
          cols={groupCols}
          rows={groupRows}
          zoom={spriteZoom}
          {palette}
          {selectedColor}
          {tool}
          onStroke={strokeSprite}
          onPick={(color) => selectedColor = color}
          onSelectionChange={(region) => spriteSelection = region}
        />
        <div class="map-zoom sprite-zoom" aria-label="Sprite zoom">{#each MAP_ZOOM_LEVELS as value}<button class:active={Math.abs(spriteZoom - value) < 0.02} onclick={() => spriteZoom = value}>{value * 100}%</button>{/each}</div>
        {#if tool === 'select'}
          <p class="map-note subtle">{spriteSelection ? `${spriteSelection.w} × ${spriteSelection.h} selected` : 'Drag to select a region'} — Ctrl+C copy · Ctrl+X cut · Ctrl+V paste.</p>
        {/if}
        <div class="palette-strip">
          {#each palette as color, index}<button aria-label={`Color ${index}`} class:active={selectedColor === index} style={`--swatch:${color}`} onclick={() => selectedColor = index}></button>{/each}
        </div>
        <div class="used-by"><span class="eyebrow">Used by</span>{#each assetIndex.entries.find((entry) => entry.kind === 'sprite' && entry.id === selectedSprite)?.refs ?? [] as reference}<button onclick={() => jumpToRef(reference)}>{reference.label}</button>{/each}{#if !(assetIndex.entries.find((entry) => entry.kind === 'sprite' && entry.id === selectedSprite)?.refs.length)}<small>No indexed references</small>{/if}</div>
      </div>
      <aside class="sheet-panel">
        <div class="panel-cap"><span class="eyebrow">Sprite sheet</span><code>256 slots</code></div>
        <p class="map-note subtle">Drag across the sheet to edit an N × M group as one canvas.</p>
        <div
          class="sprite-sheet"
          bind:this={spriteSheetEl}
          role="application"
          aria-label="Sprite sheet — drag to select an adjacent group of slots to edit as one canvas."
          onpointerdown={beginSpriteSheetDrag}
          onpointermove={moveSpriteSheetDrag}
          onpointerup={finishSpriteSheetDrag}
          onpointercancel={finishSpriteSheetDrag}
        >
          {#each Array(256) as _, index}
            <button
              tabindex="-1"
              class:active={inSpriteSheetGroup(index)}
              class:previewed={inSpriteSheetDragPreview(index)}
              class:empty={!spriteUsed[index]}
              title={`Sprite ${index.toString().padStart(3, '0')}${spriteUsed[index] ? '' : ' — empty'}`}
            >
              {#if spriteUsed[index]}
                {#each Array(64) as _, p}<i style={`background:${palette[spriteSheet[index * 64 + p] ?? 0]}`}></i>{/each}
              {/if}
            </button>
          {/each}
        </div>
      </aside>
    </section>

  {:else if screen === 'map'}
    <section class="asset-editor map-editor">
      <aside class="tool-rail">
        {#each editorTools as item}
          {@const Icon = item.icon}
          <button class:active={mapTool === item.id} title={`${item.label} (${item.shortcut})`} onclick={() => mapTool = item.id as MapTool}><Icon size={18} /></button>
        {/each}
        <button
          class:active={mapTool === 'rect-outline'}
          title="Rectangle outline — border only, no fill"
          onclick={() => mapTool = 'rect-outline'}
        ><SquareDashed size={18} /></button>
        <button
          class:active={mapTool === 'autotile'}
          title="Autotile — paints a terrain tile and matches its edges to its neighbors automatically"
          onclick={() => mapTool = 'autotile'}
        ><Puzzle size={18} /></button>
        <button
          class:active={mapTool === 'select'}
          title="Select — marquee a region, then Ctrl+C/X to copy/cut, Ctrl+V to place"
          onclick={() => mapTool = 'select'}
        ><BoxSelect size={18} /></button>
        <span></span>
        <button title="Undo map edit" disabled={!mapHistory.undo.length} onclick={undoMap}><Undo2 size={18} /></button>
        <button title="Redo map edit" disabled={!mapHistory.redo.length} onclick={redoMapEdit}><Redo2 size={18} /></button>
      </aside>
      <div class="map-canvas-col">
        <div class="map-toolbar">
          <div class="map-layer-switch" aria-label="Map edit layer">
            <button class:active={mapLayer === 'tiles'} onclick={() => mapLayer = 'tiles'}><Layers size={15} />Tiles</button>
            <button class:active={mapLayer === 'collision'} onclick={() => { mapLayer = 'collision'; collisionOverlay = true; if (mapTool === 'pick') mapTool = 'pencil'; }}><ShieldCheck size={15} />Collision</button>
          </div>
          <i class="map-toolbar-divider"></i>
          {#if mapLayer === 'collision'}
            <div class="bank-picker collision-type-picker" aria-label="Collision brush">
              <i class="brush-dot" style={`background:${rgbToHex(collisionTypeById.get(collisionBrush)?.color ?? [0, 0, 0])}`}></i>
              <select
                value={collisionBrush}
                onchange={(event) => { collisionBrush = Number((event.target as HTMLSelectElement).value); if (mapTool === 'erase') mapTool = 'pencil'; }}
              >
                {#each collisionTypes as ctype (ctype.id)}
                  <option value={ctype.id}>{ctype.name}</option>
                {/each}
              </select>
              <button title="Manage collision types" onclick={() => collisionTypesPanel = true}><Pencil size={13} /></button>
            </div>
          {/if}
          {#if mapLayer === 'tiles'}
            <label><input type="checkbox" bind:checked={collisionOverlay} />Collision overlay</label>
          {/if}
          <span class="map-toolbar-spacer"></span>
          <div class="map-zoom" aria-label="Map zoom">{#each MAP_ZOOM_LEVELS as value}<button class:active={Math.abs(mapZoom - value) < 0.02} onclick={() => mapZoom = value}>{value * 100}%</button>{/each}</div>
          <code class="map-zoom-readout">{Math.round(mapZoom * 100)}%</code>
        </div>
        <div
          class="map-work"
          class:panning={mapPanning}
          class:space-pan={spacePan}
          role="region"
          aria-label="Map canvas"
          title="Mouse wheel zoom · right or middle drag pan · hold Space to drag-pan"
          bind:this={mapWorkEl}
          onwheel={handleMapWheel}
          onscroll={updateMapViewport}
          onpointerdowncapture={beginMapPan}
          onpointermove={moveMapPan}
          onpointerup={finishMapPan}
          onpointercancel={finishMapPan}
          onlostpointercapture={loseMapPan}
          oncontextmenu={(event) => event.preventDefault()}
          onauxclick={(event) => event.preventDefault()}
        >
          {#key activeMapBank}
          <MapCanvas
            {map}
            {spriteSheet}
            {palette}
            {collision}
            {collisionTypes}
            stamp={effectiveStamp}
            showCollision={collisionOverlay || mapLayer === 'collision'}
            layer={mapLayer}
            {collisionBrush}
            tool={mapTool}
            zoom={mapZoom}
            onStroke={commitMap}
            onCollisionStroke={commitCollision}
            onPick={(tile) => { selectedTile = tile; mapStamp = null; }}
            onCollisionPick={(brush) => { collisionBrush = brush; mapTool = 'pencil'; }}
            onHover={(cell) => mapHover = cell}
            onSelectionChange={(region) => mapSelection = region}
          />
          {/key}
        </div>
      </div>
      <aside class="map-inspector">
        <span class="eyebrow">Minimap</span>
        <div class="minimap" onclick={recenterFromMinimap} onkeydown={recenterFromMinimapKey} role="button" tabindex="0" aria-label="Minimap — click to jump to a location">
          <canvas bind:this={minimapCanvas} width={MAP_W} height={MAP_H}></canvas>
          <div
            class="minimap-viewport"
            style={`left:${mapViewport.x * 100}%; top:${mapViewport.y * 100}%; width:${mapViewport.w * 100}%; height:${mapViewport.h * 100}%`}
          ></div>
        </div>
        {#if mapLayer === 'collision'}
          <div class="collision-edit-note">
            <span class="eyebrow"><ShieldCheck size={13} />Collision painting</span>
            <strong>{collisionTypeById.get(mapTool === 'erase' ? 0 : collisionBrush)?.name ?? 'walkable'} brush</strong>
            <p>Per cell — painting only changes the cells under the brush, independent of which sprite tile they show.</p>
          </div>
        {/if}
        {#if mapTool === 'autotile'}
          <div class="collision-edit-note">
            <span class="eyebrow"><Puzzle size={13} />Autotile</span>
            <strong>Paints tile {selectedTile.toString().padStart(3, '0')}'s terrain</strong>
            <p>Placing or removing a tile updates its edges and its neighbors' — the sheet's next 15 tiles after this terrain's first must be its edge/corner variants.</p>
          </div>
        {/if}
        {#if mapTool === 'select'}
          <div class="collision-edit-note">
            <span class="eyebrow"><BoxSelect size={13} />Region select</span>
            <strong>{mapSelection ? `${mapSelection.w} × ${mapSelection.h} selected` : 'Drag to select a region'}</strong>
            <p>Ctrl+C copy · Ctrl+X cut · Ctrl+V places the clipboard as a stamp — click or drag to drop it.</p>
            {#if mapSelection}
              <div class="selection-ops">
                <button title="Paste in place (Ctrl+Shift+V)" disabled={!mapClipboard} onclick={pasteInPlace}>Paste in place</button>
                <button title="Flip horizontally" onclick={() => transformMapSelection('flip')}><FlipHorizontal size={14} /></button>
                <button title="Flip vertically" onclick={() => transformMapSelection('vflip')}><FlipVertical size={14} /></button>
                <button
                  title={mapSelection.w === mapSelection.h ? 'Rotate clockwise' : 'Rotate needs a square selection'}
                  disabled={mapSelection.w !== mapSelection.h}
                  onclick={() => transformMapSelection('rotate')}
                ><RotateCw size={14} /></button>
                <button
                  title={mapSelection.w === mapSelection.h ? 'Rotate counter-clockwise' : 'Rotate needs a square selection'}
                  disabled={mapSelection.w !== mapSelection.h}
                  onclick={() => transformMapSelection('rotate-ccw')}
                ><RotateCcw size={14} /></button>
                <button title="Move up" onclick={() => moveMapSelection(0, -1)}><ArrowUp size={14} /></button>
                <button title="Move down" onclick={() => moveMapSelection(0, 1)}><ArrowDown size={14} /></button>
                <button title="Move left" onclick={() => moveMapSelection(-1, 0)}><ArrowLeft size={14} /></button>
                <button title="Move right" onclick={() => moveMapSelection(1, 0)}><ArrowRight size={14} /></button>
                <button title="Save this selection as a named stamp" onclick={saveStampAs}>Save as stamp…</button>
              </div>
            {/if}
          </div>
        {/if}
        <span class="eyebrow">Tile picker</span>
        <p class="map-note subtle">Drag across the sheet to pick a multi-tile stamp.</p>
        {#if mapStamp}
          <button class="stamp-save" onclick={saveStampAs}>Save current stamp…</button>
        {/if}
        {#if mapStampLibrary.length}
          <div class="stamp-library" aria-label="Saved stamps">
            {#each mapStampLibrary as entry (entry.name)}
              <span class="stamp-chip">
                <button title={`${entry.name} — ${entry.w} × ${entry.h}`} onclick={() => loadStamp(entry.name)}>{entry.name}</button>
                <button title={`Delete ${entry.name}`} aria-label={`Delete ${entry.name}`} onclick={() => deleteStamp(entry.name)}><X size={11} /></button>
              </span>
            {/each}
          </div>
        {/if}
        <div
          class="tile-picker"
          bind:this={pickerEl}
          role="application"
          aria-label="Tile picker — same layout as the sprite sheet. Drag to select a multi-tile stamp."
          onpointerdown={beginPickerDrag}
          onpointermove={movePickerDrag}
          onpointerup={finishPickerDrag}
          onpointercancel={finishPickerDrag}
        >
          {#each Array(256) as _, i}
            <button
              aria-label={`Tile ${i.toString().padStart(3, '0')}${spriteUsed[i] ? '' : ' — empty'}`}
              title={`Tile ${i.toString().padStart(3, '0')}${spriteUsed[i] ? '' : ' — empty'}`}
              tabindex="-1"
              class:active={inStamp(i)}
              class:previewed={inPickerPreview(i)}
              class:empty={!spriteUsed[i]}
            >
              {#if spriteUsed[i]}
                {#each Array(64) as _, p}<i style={`background:${palette[spriteSheet[i * 64 + p] ?? 0]}`}></i>{/each}
              {/if}
            </button>
          {/each}
        </div>
        <div class="inspector-row"><span>Cell</span><code>{mapHover ? `${mapHover.x}, ${mapHover.y}` : '—'}</code></div>
        <div class="inspector-row"><span>Hovered tile</span><code>{mapHover ? `${mapHover.tile.toString().padStart(3,'0')} · ${collisionTypeById.get(collision[mapHover.y * MAP_W + mapHover.x] ?? 0)?.name ?? 'unknown'}` : '—'}</code></div>
        <div class="inspector-row">
          <span>Selected</span>
          <code>{mapStamp ? `${mapStamp.w} × ${mapStamp.h} stamp` : `${selectedTile.toString().padStart(3,'0')} · 0x${selectedTile.toString(16).padStart(2,'0')}`}</code>
        </div>
        {#if mapEmpty}
          <p class="map-note">
            This map is empty. Pick a tile and paint to start it.
          </p>
        {/if}
        <p class="map-note subtle">Wheel zoom · right/middle drag pan · Pick tool samples.</p>
      </aside>
    </section>
    <Dialog.Root open={collisionTypesPanel} onOpenChange={(open) => collisionTypesPanel = open}>
      <Dialog.Content showCloseButton={false} class="dialog-frame">
        <div class="collision-types-dialog">
          <Button variant="ghost" size="icon-sm" class="dialog-close" aria-label="Close" onclick={() => collisionTypesPanel = false}><X size={17} /></Button>
          <span class="eyebrow">Map · Collision layer</span>
          <h2>Collision types</h2>
          <p>Custom types are cart-wide and readable from Lua via <code>get_collision</code>. Built-ins can't be renamed or removed.</p>
          <div class="collision-types-list">
            {#each collisionTypes as ctype (ctype.id)}
              <div class="collision-types-row">
                <span class="collision-types-swatch"><i style={`background:${rgbToHex(ctype.color)}`}></i><input
                  type="color"
                  aria-label={`${ctype.name} color`}
                  value={rgbToHex(ctype.color)}
                  oninput={(event) => updateCollisionType(ctype.id, { color: hexToRgb((event.target as HTMLInputElement).value) })}
                /></span>
                <input
                  class="collision-types-name"
                  value={ctype.name}
                  disabled={isBuiltinCollisionType(ctype.id)}
                  oninput={(event) => updateCollisionType(ctype.id, { name: (event.target as HTMLInputElement).value })}
                />
                <div class="collision-types-shape" role="radiogroup" aria-label={`${ctype.name} shape`}>
                  {#each [['none', 'None'], ['solid', 'Solid'], ['one_way', 'One-way'], ['slope_left', 'Slope L'], ['slope_right', 'Slope R']] as [value, label] (value)}
                    <label>
                      <input
                        type="radio"
                        name={`collision-shape-${ctype.id}`}
                        value={value}
                        checked={ctype.shape === value}
                        disabled={isBuiltinCollisionType(ctype.id)}
                        onchange={() => updateCollisionType(ctype.id, { shape: value as CollisionShape })}
                      />{label}
                    </label>
                  {/each}
                </div>
                <code>{ctype.id.toString().padStart(2,'0')}</code>
                <button
                  class="danger"
                  title={isBuiltinCollisionType(ctype.id) ? 'Built-in types cannot be removed' : 'Remove collision type'}
                  disabled={isBuiltinCollisionType(ctype.id)}
                  onclick={() => removeCollisionType(ctype.id)}
                ><Trash2 size={13} /></button>
              </div>
            {/each}
          </div>
          <button class="collision-types-add" onclick={addCollisionType}><Plus size={14} />Add collision type</button>
          <footer><Button variant="outline" onclick={() => collisionTypesPanel = false}>Done</Button></footer>
        </div>
      </Dialog.Content>
    </Dialog.Root>

  {:else if screen === 'palette'}
    <section class="palette-screen">
      <header>
        <span><span class="eyebrow">Cart palette</span><h1>Palette</h1></span>
        <p>Sixteen colors shared by every sprite, tile, and draw call.</p>
        <div class="history-controls">
          <button title="Undo palette edit" disabled={!paletteHistory.undo.length} onclick={undoPalette}><Undo2 size={16} /></button>
          <button title="Redo palette edit" disabled={!paletteHistory.redo.length} onclick={redoPalette}><Redo2 size={16} /></button>
        </div>
      </header>
      <div class="palette-layout">
        <div class="palette-grid">
          {#each palette as color, index}
            <button class:active={selectedSlot === index} onclick={() => selectedSlot = index}>
              <i style={`background:${color}`}></i><span><strong>{index.toString().padStart(2,'0')}</strong><code>{color}</code></span>
              <small>{spriteSheet.filter((slot) => slot === index).length} px</small>
            </button>
          {/each}
        </div>
        <aside class="color-inspector">
          <div class="color-preview" style={`background:${palette[selectedSlot]}`}></div>
          <div><span class="eyebrow">Slot {selectedSlot.toString().padStart(2,'0')}</span><h2>{palette[selectedSlot]}</h2></div>
          <label>Hex<input value={palette[selectedSlot]} onfocus={beginPaletteEdit} onblur={(e) => { updatePalette(e.currentTarget.value); commitPaletteEdit(); }} /></label>
          {#each ['Red','Green','Blue'] as channel, i}
            <label>{channel}<input type="range" min="0" max="255" value={parseInt(palette[selectedSlot].slice(1 + i * 2, 3 + i * 2), 16)} onpointerdown={beginPaletteEdit} oninput={(event) => updateChannel(i, Number(event.currentTarget.value))} onchange={commitPaletteEdit} /><code>{parseInt(palette[selectedSlot].slice(1 + i * 2, 3 + i * 2),16)}</code></label>
          {/each}
          <section><span class="eyebrow">Usage</span><p><strong>{spriteSheet.filter((color) => color === selectedSlot).length}</strong> sprite pixels</p><p><strong>{assetIndex.entries.find((entry) => entry.kind === 'color' && entry.id === selectedSlot)?.refs.length ?? 0}</strong> references in code</p><p><strong>{map.filter((tile) => spriteSheet[tile * 64] === selectedSlot).length}</strong> map tiles</p></section>
        </aside>
      </div>
    </section>

  {:else if screen === 'sfx'}
    <section class="sound-screen">
      <aside class="slot-list">
        <div class="panel-cap"><span class="eyebrow">Sound effects</span><button title="Select first empty SFX slot" onclick={selectEmptySfx}><Plus size={14} /></button></div>
        {#each Array(16) as _, index}
          <button class:active={selectedSfx === index} onclick={() => soundSelection.sfx = index}>
            <code>{index.toString().padStart(2,'0')}</code>
            <span>{sfxSlotFilled[index] ? `SFX ${index.toString().padStart(2,'0')}` : 'Empty slot'}</span>
            <em class="slot-wave" class:filled={sfxSlotFilled[index]} aria-hidden="true">
              {#each Array(6) as _, bar}
                {@const note = sfx[index * 64 + bar * 8] ?? 0}
                <i style={`height:${note ? Math.max(20, (note / 96) * 100) : 12}%`}></i>
              {/each}
            </em>
          </button>
        {/each}
      </aside>
      <div class="tracker">
        <header>
          <button class="btn primary" onclick={() => onAudio('sfx', selectedSfx, sfxPlaying ? 'stop' : 'play')}>
            {#if sfxPlaying}<Square size={13} />Stop{:else}<Play size={13} />Play{/if}
          </button>
          <span>
            <h2>{sfxSlotFilled[selectedSfx] ? `SFX ${selectedSfx.toString().padStart(2,'0')}` : 'Empty slot'}</h2>
            <code>sfx {selectedSfx} · 16 steps{sfxPlaying ? ` · step ${audio.sfxStep.toString().padStart(2,'0')}` : ''}</code>
          </span>
          <div class="history-controls">
            <button title="Undo SFX edit" disabled={!sfxHistory.undo.length} onclick={undoSfx}><Undo2 size={16} /></button>
            <button title="Redo SFX edit" disabled={!sfxHistory.redo.length} onclick={redoSfx}><Redo2 size={16} /></button>
          </div>
        </header>

        <div class="sfx-tracker">
          <div class="sfx-labels">
            <span class="sfx-label-step">step</span>
            <div class="sfx-label-pitch">
              {#each pitchAxis as mark}<span style={`bottom:${mark.at}%`}>{mark.name}</span>{/each}
            </div>
            <div class="sfx-label-volume"><span>15</span><span>8</span><span>0</span></div>
            <span class="sfx-label-row">wave</span>
            <span class="sfx-label-row">pan</span>
            <span class="sfx-label-row">atk</span>
            <span class="sfx-label-row">rel</span>
          </div>

          <div class="sfx-columns">
            <div class="sfx-steps">
              {#each Array(16) as _, step}
                <code class:playhead={sfxStepActive(step)}>{step.toString().padStart(2,'0')}</code>
              {/each}
            </div>

            <!-- Pitch: drag to draw notes, right-drag to erase. -->
            <div
              class="sfx-pitch"
              role="application"
              aria-label="Note pitch per step. Drag to draw, right-drag to erase."
              onpointerdown={(event) => beginSfxDraw(event, 0)}
              onpointermove={(event) => continueSfxDraw(event, 0)}
              onpointerup={endSfxDraw}
              onpointercancel={endSfxDraw}
              oncontextmenu={(event) => event.preventDefault()}
            >
              {#each Array(16) as _, step}
                {@const note = sfxByte(step, 0)}
                <div class="sfx-cell" class:beat={step % 4 === 0} class:playhead={sfxStepActive(step)}>
                  {#if note > 0}
                    <i
                      class:noise={sfxByte(step, 2) === 1}
                      style={`height:${Math.max(4, (note / 96) * 100)}%`}
                      title={`step ${step} · ${noteNames[note]}`}
                    ></i>
                  {/if}
                </div>
              {/each}
            </div>

            <div
              class="sfx-volume"
              role="application"
              aria-label="Volume per step. Drag to draw."
              onpointerdown={(event) => beginSfxDraw(event, 1)}
              onpointermove={(event) => continueSfxDraw(event, 1)}
              onpointerup={endSfxDraw}
              onpointercancel={endSfxDraw}
              oncontextmenu={(event) => event.preventDefault()}
            >
              {#each Array(16) as _, step}
                {@const volume = sfxByte(step, 1)}
                <div class="sfx-cell" class:beat={step % 4 === 0} class:playhead={sfxStepActive(step)}>
                  {#if sfxByte(step, 0) > 0}<i style={`height:${Math.max(4, (volume / 15) * 100)}%`} title={`volume ${volume}`}></i>{/if}
                </div>
              {/each}
            </div>

            <div class="sfx-wave">
              {#each Array(16) as _, step}
                {@const empty = sfxByte(step, 0) === 0}
                {@const noise = sfxByte(step, 2) === 1}
                <button
                  class:noise
                  class:empty
                  disabled={empty}
                  title={empty ? 'No note on this step' : noise ? 'Noise — click for square' : 'Square — click for noise'}
                  onclick={() => setSfxCells([{ step, field: 2, value: noise ? 0 : 1 }])}
                >
                  {#if empty}·{:else if noise}<svg viewBox="0 0 20 10" aria-hidden="true"><polyline points="0,5 2,2 4,8 6,3 8,7 10,1 12,9 14,4 16,6 18,2 20,5" /></svg>
                  {:else}<svg viewBox="0 0 20 10" aria-hidden="true"><polyline points="0,8 0,2 5,2 5,8 10,8 10,2 15,2 15,8 20,8" /></svg>{/if}
                </button>
              {/each}
            </div>

            <div class="sfx-pan">
              {#each Array(16) as _, step}
                {@const empty = sfxByte(step, 0) === 0}
                {@const pan = sfxPan(step)}
                <button
                  class:empty
                  disabled={empty}
                  title={empty ? 'No note on this step' : `Pan ${PAN_LABELS[pan]}`}
                  onclick={() => setSfxPan(step, (pan + 1) % 16)}
                >{empty ? '·' : PAN_LABELS[pan]}</button>
              {/each}
            </div>

            <div class="sfx-attack">
              {#each Array(16) as _, step}
                {@const empty = sfxByte(step, 0) === 0}
                {@const attack = sfxAttack(step)}
                <button
                  class:empty
                  disabled={empty}
                  title={empty ? 'No note on this step' : `Attack ${ENV_LABELS[attack]}`}
                  onclick={() => setSfxAttack(step, (attack + 1) % 4)}
                >{empty ? '·' : ENV_LABELS[attack]}</button>
              {/each}
            </div>

            <div class="sfx-release">
              {#each Array(16) as _, step}
                {@const empty = sfxByte(step, 0) === 0}
                {@const release = sfxRelease(step)}
                <button
                  class:empty
                  disabled={empty}
                  title={empty ? 'No note on this step' : `Release ${ENV_LABELS[release]}`}
                  onclick={() => setSfxRelease(step, (release + 1) % 4)}
                >{empty ? '·' : ENV_LABELS[release]}</button>
              {/each}
            </div>
          </div>
        </div>

        <p class="sfx-hints">
          <span>Drag in the pitch grid to draw notes</span>
          <span>Right-drag to erase</span>
          <span>Space to preview</span>
          <span><i class="swatch-square"></i>square <i class="swatch-noise"></i>noise</span>
        </p>
      </div>
    </section>

  {:else if screen === 'music'}
    <section class="music-screen">
      <aside class="pattern-list">
        <div class="panel-cap"><span class="eyebrow">Patterns</span><button title="Select first empty pattern" onclick={selectEmptyPattern}><Plus size={14} /></button></div>
        {#each Array(MUSIC_PATTERN_COUNT) as _, index}
          <button class:active={selectedPattern === index} onclick={() => soundSelection.pattern = index}>
            <code>{index.toString().padStart(2,'0')}</code><span>{patternBytes(index).some(Boolean) ? `Pattern ${index.toString().padStart(2,'0')}` : 'Empty pattern'}</span>
          </button>
        {/each}
        <div class="pattern-clipboard">
          <button class="pattern-copy" onclick={copyPattern}>Copy pattern</button>
          <button class="pattern-paste" disabled={!musicPatternClipboard} onclick={pastePattern}>Paste into {selectedPattern.toString().padStart(2,'0')}</button>
        </div>
        <div class="song-order"><span class="eyebrow">Playback</span><button class:active={audio.musicActive} onclick={() => onAudio('music', audio.musicPattern, audio.musicActive ? 'stop' : 'play')}><code>{audio.musicPattern.toString().padStart(2,'0')}</code>{audio.musicActive ? `Row ${audio.musicRow.toString(16).toUpperCase()}` : 'Stopped'}<small>{audio.musicLoop ? 'loop' : 'once'}</small></button></div>
      </aside>
      <aside class="song-editor">
        <div class="panel-cap"><span class="eyebrow">Song order</span><button class="song-play" title="Play the song from step 00" onclick={() => onAudio('music', 0, 'play_song')}><Play size={14} /></button></div>
        <p class="song-hint">Click a step to pick the pattern it plays. The loop point sends the song back there when it runs out; without one it stops.</p>
        <div class="song-steps">
          {#each Array(MUSIC_ORDER_STEPS) as _, step}
            {@const slot = songStep(step)}
            <div class="song-step" class:filled={slot > 0} class:loop={songLoopStep === step}>
              <code>{step.toString().padStart(2,'0')}</code>
              <button
                class="song-slot"
                aria-label={`Song step ${step}: ${slot ? `pattern ${slot - 1}` : 'empty'}`}
                onclick={() => changeSongStep(step)}
              >{slot ? (slot - 1).toString().padStart(2,'0') : '—'}</button>
              <button
                class="song-loop"
                aria-label={`${songLoopStep === step ? 'Clear' : 'Set'} loop point at step ${step}`}
                aria-pressed={songLoopStep === step}
                onclick={() => toggleSongLoop(step)}
              >⟲</button>
            </div>
          {/each}
        </div>
      </aside>
      <div class="music-grid-wrap">
        <header>
          <span><span class="eyebrow">Pattern {selectedPattern.toString().padStart(2,'0')}</span><h2>Pattern {selectedPattern.toString().padStart(2,'0')}</h2></span>
          <button class="btn secondary" onclick={() => onAudio('music', selectedPattern, musicPlaying ? 'stop' : 'play')}>{#if musicPlaying}<Square size={14} />Stop{:else}<Play size={14} />Play pattern{/if}</button>
          <div class="history-controls">
            <button title="Undo pattern edit" disabled={!musicHistory.undo.length} onclick={undoMusic}><Undo2 size={16} /></button>
            <button title="Redo pattern edit" disabled={!musicHistory.redo.length} onclick={redoMusic}><Redo2 size={16} /></button>
          </div>
        </header>
        <div class="music-grid">
          <div class="music-head"><span>Row</span>{#each MUSIC_CHANNEL_LABELS as label}<span>{label}</span>{/each}</div>
          {#each Array(MUSIC_PATTERN_ROWS) as _, row}
            <div class:playhead={audio.musicActive && audio.musicPattern === selectedPattern && audio.musicRow === row} class:selected={musicRowSelected(row)}><button
              class="row-handle"
              aria-label={`Select row ${row}`}
              aria-pressed={musicRowSelected(row)}
              onpointerdown={(event) => startRowSelect(row, event.shiftKey)}
              onpointerenter={() => extendRowSelect(row)}
            >{row.toString(16).toUpperCase().padStart(2,'0')}</button>{#each MUSIC_CHANNEL_LABELS as _, channel}{@const cell = music[selectedPattern * MUSIC_PATTERN_LEN + row * MUSIC_CHANNEL_COUNT + channel] ?? 0}<button class="music-cell" onclick={() => changeMusic(row, channel)}>{cell ? `SFX ${(cell - 1).toString().padStart(2,'0')}` : '—'}</button>{/each}</div>
          {/each}
        </div>
        <p class="music-hints">
          <span>Drag row numbers to select a step range</span>
          <span>Ctrl+C copy · Ctrl+X cut · Ctrl+V paste rows</span>
          <span>Space to preview</span>
        </p>
      </div>
    </section>

  {:else if screen === 'assets'}
    <section class="page-screen assets-screen">
      <header class="page-header"><span><span class="eyebrow">Cart inventory</span><h1>Assets</h1><p>Every byte of art and sound, with where it appears.</p></span>{#if path}<Button variant="outline" onclick={focusAssetFilter}><Search size={15} />Find reference</Button>{/if}</header>
      {#if !path}
        <div class="port-empty">
          <strong>No cart open</strong>
          <p>Open or create cart before browsing its assets.</p>
          <Button onclick={() => onNavigate('welcome')}>Open Start screen</Button>
        </div>
      {:else}
        <div class="asset-summary">
          {#each assetSummary as card}
            {@const Icon = card.icon}
            <div>
              <span class="eyebrow"><Icon size={14} />{card.label}</span>
              <strong>{card.value}</strong>
              <div class="meter"><i style={`width:${Math.min(100, card.pct)}%`}></i></div>
              <small>{card.detail}</small>
            </div>
          {/each}
        </div>

        <div class="asset-filter">
          <Search size={14} />
          <Input id="asset-filter" bind:value={assetFilter} placeholder="Filter assets and references" />
          <code>{assetRows.length} of {assetIndex.entries.filter((entry) => entry.nonzero || entry.used).length}</code>
        </div>

        <div class="xref-table">
          <div class="table-head"><span>Preview</span><span>Asset</span><span>Used by</span><span>Edit</span></div>
          {#each assetRows as row (row.kind + row.id)}
            <div class="xref-row">
            <span class="xref-preview">
              {#if row.kind === 'sprite'}
                <em class="xref-sprite">{#each Array(64) as _, p}<i style={`background:${palette[spriteSheet[row.id * 64 + p] ?? 0]}`}></i>{/each}</em>
              {:else if row.kind === 'color'}
                <em class="xref-swatch" style={`background:${palette[row.id]}`}></em>
              {:else}
                <em class="xref-bars">{#each Array(5) as _, bar}<i style={`height:${25 + ((row.id * 37 + bar * 19) % 70)}%`}></i>{/each}</em>
              {/if}
            </span>
            <span class="xref-name">
              <strong>{assetLabel(row)}</strong>
              <code>{row.kind} {row.id.toString().padStart(row.kind === 'sprite' ? 3 : 2, '0')} · {row.bytes} B</code>
            </span>
            <span class="xref-refs">
              {#each groupRefs(row.refs) as group}
                <button class="pill code" onclick={() => jumpToRef(group.reference)}>
                  {group.reference.label}{#if group.count > 1}<b>×{group.count}</b>{/if}
                </button>
              {/each}
              {#each assetUsage(row) as usage}
                <span class="pill asset">{usage}</span>
              {/each}
              {#if !row.refs.length && !assetUsage(row).length}<small>Not referenced</small>{/if}
            </span>
            <button class="xref-open" onclick={() => openAsset(row)}>Open <ArrowRight size={13} /></button>
            </div>
          {/each}
          {#if !assetRows.length}
            <div class="xref-empty">{assetFilter ? 'Nothing matches that filter.' : 'This cart has no assets yet.'}</div>
          {/if}
        </div>
      {/if}
    </section>

  {:else if screen === 'cart'}
    <section class="page-screen cart-screen">
      <header class="page-header"><span><span class="eyebrow">Project metadata</span><h1>Cart details</h1><p>What players see when cart reaches port.</p></span><span class="saved-note"><CircleCheck size={14} />{dirty ? 'Unsaved changes' : 'All changes saved'}</span></header>
      <div class="cart-layout" data-tour-target="ship">
        <div class="cart-form">
          <label>Title<Input maxlength={64} value={title} onblur={(event) => onMeta(event.currentTarget.value, author, meta)} /></label>
          <label>Local author <Input maxlength={64} value={author} onblur={(event) => onMeta(title, event.currentTarget.value, meta)} /><small>Stored in local cart metadata. Port uses linked account when publishing.</small></label>
          <label>Description<Textarea maxlength={240} value={meta.description} onblur={(event) => onMeta(title, author, { ...meta, description: event.currentTarget.value })}></Textarea><small>{meta.description.length} / 240</small></label>
          <label>Tags<div class="tag-input">{#each meta.tags as tag}<button onclick={() => onMeta(title, author, { ...meta, tags: meta.tags.filter((value) => value !== tag) })}>{tag} ×</button>{/each}<input placeholder="Add tag…" onkeydown={(event) => { if (event.key === 'Enter' && event.currentTarget.value.trim()) { event.preventDefault(); onMeta(title, author, { ...meta, tags: [...meta.tags, event.currentTarget.value.trim()] }); event.currentTarget.value = ''; } }} /></div></label>
          <div class="cart-facts">{#each [['Format',path.endsWith('.cav') ? '.cav' : 'project dir'],['Packed size',`${(cartSize.packedBytes / 1024).toFixed(1)} KiB`],['Sources',`${sources.length} module${sources.length === 1 ? '' : 's'}`],['Port',portAccount.authenticated ? portAccount.username : 'not signed in']] as fact}<span><small>{fact[0]}</small><code>{fact[1]}</code></span>{/each}</div>
          {#if !portAccount.authenticated}<Button class="cart-port-cta" onclick={onOpenPortAccount}>Open Port account</Button>{/if}
          <div class="stdlib-modules">
            <div class="stdlib-modules-heading">Stdlib modules<small>Enabling a module here makes its globals available to the cart's Lua source. The editor also offers a quick-fix to enable a module when you reference it in code.</small></div>
            <div class="stdlib-modules-list">
              {#each preludeModules as module (module.name)}
                <label class="stdlib-module-row">
                  <input type="checkbox" checked={module.enabled} onchange={(event) => onSetStdlibModule(module.name, event.currentTarget.checked)} />
                  <span class="stdlib-module-name">{module.name}</span>
                  <small class="stdlib-module-globals">{module.globals.join(', ')}</small>
                </label>
              {/each}
            </div>
          </div>
        </div>
        <aside class="cart-preview">
          <span class="eyebrow">Port preview</span>
          <div class="cover-art">
            {#if frameData?.length === SCREEN_RGBA_LEN}
              <canvas bind:this={coverCanvas} width={SCREEN_WIDTH} height={SCREEN_HEIGHT}></canvas>
            {:else}
              {#each Array(384) as _,p}<i style={`background:${palette[(p * 7 + 5) % 16]}`}></i>{/each}
            {/if}
            <div class="scanline-overlay"></div>
          </div>
          <h2>{title}</h2><p>by {author}</p><small>{meta.description}</small>
          <small class="cover-note">Cover is captured live from the console when you publish.</small>
        </aside>
      </div>
    </section>

  {:else if screen === 'account'}
    <section class="page-screen account-screen">
      <header class="page-header"><span><span class="eyebrow">Caiven Port</span><h1>Account</h1><p>Port identity owns published carts and version edits.</p></span></header>
      <div class="account-card">
        {#if portAccount.authenticated}
          <div class="account-avatar linked"><UserRound size={28} /></div>
          <span class="account-status linked">Linked</span>
          <h2>{portAccount.username}</h2>
          <p>Publishing uses this Port account. Local cart author stays local metadata.</p>
          <Button variant="outline" onclick={onPortLogout}>Log out</Button>
        {:else if portLinkPending}
          <div class="account-avatar pending"><Globe size={28} /></div>
          <span class="account-status pending">Browser opened</span>
          <h2>Finish linking in Port</h2>
          <p>Sign in or register in the browser tab, then enter this code to approve Caiven Studio — Studio picks it up automatically.</p>
          <p class="port-link-code">{portLinkUserCode || '········'}</p>
          <p class="account-expiry">Link expires {portLinkExpiresAt ? new Date(portLinkExpiresAt).toLocaleTimeString() : 'soon'}.</p>
          <Button variant="outline" disabled={portBusy} onclick={onPortLinkCancel}>Cancel</Button>
        {:else}
          <div class="account-avatar"><UserRound size={28} /></div>
          <span class="account-status">Not linked</span>
          <h2>Link Port account</h2>
          <p>Required before publishing. The browser handles sign-in — Studio never sees your password.</p>
          <label class="server-url-field">Port server<Input value={serverUrlDraft} placeholder="http://localhost:8080" onblur={(event) => { serverUrlDraft = event.currentTarget.value; onSetServerUrl(serverUrlDraft); }} onkeydown={(event) => { if (event.key === 'Enter') { event.currentTarget.blur(); } }} /><small>Self-hosting or joining a community instance? Point Studio at it here — leave blank for {portAccount.portUrl || 'the default'}.</small></label>
          <Button disabled={portBusy} onclick={onPortLink}>Link Port account</Button>
        {/if}
        {#if portError}
          <div class="port-empty account-issue">
            <strong>Account issue</strong>
            <p>{portError}</p>
            {#if !portLinkPending && !portAccount.authenticated}<button onclick={onPortLink}>Retry</button>{/if}
          </div>
        {/if}
      </div>
    </section>

  {:else if screen === 'library'}
    <section class="page-screen library-screen">
      <header class="page-header"><span><span class="eyebrow">Your carts</span><h1>Library</h1><p>Local projects and carts from port.</p></span><div class="segmented"><Button variant="ghost" class={libraryTab === 'local' ? 'active' : undefined} onclick={() => libraryTab = 'local'}>Local</Button><Button variant="ghost" class={libraryTab === 'port' ? 'active' : undefined} onclick={() => { libraryTab = 'port'; if (!portCarts.length) onSearchPort(''); }}>Port</Button></div></header>
      <div class="library-toolbar"><div><Search size={15} /><Input bind:value={libraryQuery} placeholder="Search carts" onkeydown={(event) => { if (event.key === 'Enter' && libraryTab === 'port') onSearchPort(libraryQuery); }} /></div>{#if libraryTab === 'local'}<Button variant="outline" onclick={onScanLibrary}><FolderOpen size={15} />Scan folder</Button>{:else if portAccount.authenticated}<span class="port-account">{portAccount.username}<Button variant="ghost" onclick={onPortLogout}>Log out</Button></span>{/if}</div>
      {#if libraryTab === 'port' && !portAccount.authenticated}<div class="port-login"><span><strong>Port account</strong><small>Link before publishing.</small></span><Button onclick={onOpenPortAccount}>Open Account</Button></div>{/if}
      {#if portError && libraryTab === 'port'}<div class="port-empty"><strong>Port unavailable</strong><p>{portError}</p><button onclick={() => onSearchPort(libraryQuery)}>Retry</button></div>{/if}
      <div class="cart-grid">
        {#if libraryTab === 'local'}
          {#each localCarts.filter((cart) => `${cart.title} ${cart.author}`.toLowerCase().includes(libraryQuery.toLowerCase())) as cart,i}
            <button class="library-card" onclick={() => onOpenLocal(cart.path)}>
              <div class="library-cover">{#each Array(64) as _,p}<i style={`background:${palette[(p * 5 + i * 3) % 16]}`}></i>{/each}<span class="scanline-overlay"></span></div>
              <span><strong>{cart.title || cart.name}</strong><small>by {cart.author || 'unknown'}</small></span>
              <footer><code>{cart.project ? 'project' : '.cav'}</code><small>{new Date(cart.modified * 1000).toLocaleDateString()}</small></footer>
            </button>
          {/each}
        {:else}
          {#each portCarts.filter((cart) => `${cart.title} ${cart.author} ${cart.tags.join(' ')}`.toLowerCase().includes(libraryQuery.toLowerCase())) as cart}
            <button class="library-card" disabled={portBusy} onclick={() => onDownloadPort(cart)}>
              <div class="library-cover">{#if cart.screenshotUrl}<img src={cart.screenshotUrl} alt="" />{:else}{#each Array(64) as _,p}<i style={`background:${palette[(p * 5 + cart.title.length) % 16]}`}></i>{/each}{/if}<span class="scanline-overlay"></span></div>
              <span><strong>{cart.title}</strong><small>by {cart.author}</small></span>
              <footer><code>v{cart.latestVersion || 1}</code><small>{cart.downloads} downloads</small></footer>
            </button>
          {/each}
        {/if}
      </div>
      {#if libraryTab === 'local' && !localCarts.length}<div class="port-empty"><strong>No folder scanned</strong><p>Choose a folder containing projects or .cav files.</p><button onclick={onScanLibrary}>Scan folder</button></div>{:else if libraryTab === 'port' && !portBusy && !portError && !portCarts.length}<div class="port-empty"><strong>No carts found</strong><p>Try another search.</p></div>{/if}
    </section>

  {:else if screen === 'docs'}
    <section class="docs-screen">
      <aside class="docs-nav">
        <div class="docs-search"><Search size={14} /><Input bind:value={docQuery} placeholder="Search API" /></div>
        {#each docCategories as [name, count]}
          <button class:active={name === activeDocCategory} onclick={() => docCategory = name}><span>{name}</span><code>{count}</code></button>
        {/each}
      </aside>
      <div class="docs-content">
        <header><span class="eyebrow">API reference</span><h1>{activeDocCategory}</h1><p>Every {activeDocCategory.toLowerCase()} entry a cart can call, sourced live from the API registry.</p></header>
        <div class="api-list">
          {#each filteredApi as entry}
            <article>
              <h3><code>{signature(entry)}</code><small>→ {entry.returns}</small></h3>
              <p>{entry.doc}</p>
              <Button variant="ghost" onclick={() => { onInsertBuiltin(entry.name); onNavigate('code'); }}>Insert into editor <ExternalLink size={12} /></Button>
            </article>
          {/each}
        </div>
      </div>
    </section>
  {/if}

  {#if textPrompt}
    <Dialog.Root open onOpenChange={(open) => { if (!open) textPrompt = null; }}>
    <Dialog.Content showCloseButton={false} class="dialog-frame">
    <form class="module-dialog" onsubmit={(event) => { event.preventDefault(); submitTextPrompt(); }}>
      <Button type="button" variant="ghost" size="icon-sm" class="dialog-close" aria-label="Close" onclick={() => textPrompt = null}><X size={17} /></Button>
      <h2>{textPrompt.title}</h2>
      <label>
        Name
        <span><Input bind:value={textPrompt.value} aria-invalid={Boolean(textPrompt.error)} autocomplete="off" spellcheck="false" autofocus /></span>
      </label>
      {#if textPrompt.error}<div class="form-error" role="alert">{textPrompt.error}</div>{/if}
      <footer><Button type="button" variant="outline" onclick={() => textPrompt = null}>Cancel</Button><Button type="submit" disabled={!textPrompt.value.trim()}>Create</Button></footer>
    </form>
    </Dialog.Content>
    </Dialog.Root>
  {/if}
</main>
