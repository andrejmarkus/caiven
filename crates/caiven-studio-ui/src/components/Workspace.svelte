<script lang="ts">
  import { flushSync, onDestroy } from 'svelte';
  import {
    Image, Layers, Pipette, Volume2, Music, FileCode2, FileImage,
    Plus, Pencil, PaintBucket, Minus, Square, Undo2, Redo2, Eraser, ShieldCheck,
    FlipHorizontal, RotateCw, Trash2, Search, FolderOpen, Play,
    ExternalLink, Sparkles, ArrowRight, CircleCheck, ChevronRight, X,
    UserRound, Globe,
  } from '@lucide/svelte';
  import { Button } from '@caiven/ui/button';
  import { Input } from '@caiven/ui/input';
  import { Textarea } from '@caiven/ui/textarea';
  import type {
    ApiEntry, AssetIndex, AssetRef, AudioState, Breakpoint, CartMeta, CartSize, Diagnostic, EditorInsertRequest,
    EditorRevealRequest, LocalCart, PortCart, PortSession, Screen, SourceBuffer,
  } from '../types';
  import {
    dragPanScroll, MAP_ZOOM_LEVELS, nextMapZoom,
    type CollisionBrush, type SpriteFlagEdit,
  } from '../lib/editorMath';
  import LuaEditor from './LuaEditor.svelte';
  import MapCanvas from './MapCanvas.svelte';
  import SpriteCanvas, { type Pixel, type SpriteTool } from './SpriteCanvas.svelte';

  type MapTool = 'pencil' | 'fill' | 'rect' | 'pick' | 'erase' | 'line';
  type MapHistoryEntry =
    | { kind: 'tiles'; changes: { offset: number; before: number; after: number }[] }
    | { kind: 'flags'; changes: { tile: number; before: number; after: number }[] };

  interface Props {
    screen: Screen;
    sources: SourceBuffer[];
    activeSource: number;
    palette: string[];
    spriteSheet: number[];
    map: number[];
    spriteBanks: number[];
    mapBanks: number[];
    activeSpriteBank: number;
    activeMapBank: number;
    spriteFlags: number[];
    sfx: number[];
    music: number[];
    paletteBanks: number[];
    sfxBanks: number[];
    musicBanks: number[];
    activePaletteBank: number;
    activeSfxBank: number;
    activeMusicBank: number;
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
    api: ApiEntry[];
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
    onFlags: (sprite: number, flags: number) => void;
    onFlagsBatch: (edits: SpriteFlagEdit[]) => void;
    onMap: (cells: { offset: number; tile: number }[]) => void;
    onAssetBank: (kind: 'sprites' | 'map' | 'palette' | 'sfx' | 'music', action: 'select' | 'create' | 'delete', id?: number) => void | Promise<boolean | void>;
    onSfx: (slot: number, bytes: number[]) => void;
    onMusic: (pattern: number, bytes: number[]) => void;
    onAudio: (kind: 'sfx' | 'music', id: number, action: 'play' | 'stop') => void;
    onBreakpoint: (source: string, line: number) => void;
    onMeta: (title: string, author: string, meta: CartMeta) => void;
    onCreateModule: () => void;
    onPalette: (slot: number, hex: string) => void;
    onTour: () => void;
    onOpen: () => void;
    onNew: () => void;
    localCarts: LocalCart[];
    portCarts: PortCart[];
    portAccount: PortSession;
    portBusy: boolean;
    portError: string;
    portLinkPending: boolean;
    portLinkExpiresAt: string;
    onScanLibrary: () => void;
    onSearchPort: (query: string) => void;
    onOpenLocal: (path: string) => void;
    onRemoveRecent: (path: string) => void;
    onDownloadPort: (cart: PortCart) => void;
    onOpenPortAccount: () => void;
    onPortLink: () => void;
    onPortLinkCancel: () => void;
    onPortLogout: () => void;
    onInsertBuiltin: (name: string) => void;
    onOpenSource: (path: string, line: number | null, column?: number | null) => void;
  }

  let {
    screen, sources, activeSource, palette, spriteSheet, map, spriteBanks, mapBanks, activeSpriteBank, activeMapBank, spriteFlags, sfx, music,
    paletteBanks, sfxBanks, musicBanks, activePaletteBank, activeSfxBank, activeMusicBank, cartSize,
    audio, assetIndex, diagnostics, breakpoints, title, author, path, meta, dirty, tourDone, recent, api, frameData, insertRequest, revealRequest, onInsertHandled, onRevealHandled,
    soundSelection,
    onNavigate, onSource, onCode, onSprite, onFlags, onFlagsBatch, onMap, onAssetBank, onSfx, onMusic, onAudio,
    onBreakpoint, onMeta, onCreateModule, onPalette, onTour, onOpen, onNew,
    localCarts, portCarts, portAccount, portBusy, portError, portLinkPending, portLinkExpiresAt, onScanLibrary,
    onSearchPort, onOpenLocal, onRemoveRecent, onDownloadPort, onOpenPortAccount, onPortLink, onPortLinkCancel, onPortLogout,
    onInsertBuiltin, onOpenSource,
  }: Props = $props();

  let selectedColor = $state(8);
  let selectedSlot = $state(9);
  let selectedSprite = $state(0);
  // Read-only views of the shared selection; clicks write through soundSelection.
  const selectedSfx = $derived(soundSelection.sfx);
  const selectedPattern = $derived(soundSelection.pattern);
  let selectedTile = $state(0);
  let mapTool = $state<MapTool>('pencil');
  let mapLayer = $state<'tiles' | 'collision'>('tiles');
  let collisionBrush = $state<CollisionBrush>(1);
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
  let mapUndo = $state<MapHistoryEntry[]>([]);
  let mapRedo = $state<MapHistoryEntry[]>([]);
  let tileSelectionReady = $state(false);
  let collisionOverlay = $state(true);
  let mapHover = $state<{ x: number; y: number; tile: number } | null>(null);
  let spriteUndo = $state<number[][]>([]);
  let spriteRedo = $state<number[][]>([]);
  let tool = $state<SpriteTool>('pencil');
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
    if (!coverCanvas || frameData?.length !== 128 * 128 * 4) return;
    const ctx = coverCanvas.getContext('2d');
    ctx?.putImageData(new ImageData(new Uint8ClampedArray(frameData), 128, 128), 0, 0);
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
  const sprite = $derived(spriteSheet.slice(selectedSprite * 64, selectedSprite * 64 + 64));
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
    spriteUndo = [];
    spriteRedo = [];
    mapUndo = [];
    mapRedo = [];
    mapLayer = 'tiles';
    mapTool = 'pencil';
    collisionBrush = 1;
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
    spriteUndo = [];
    spriteRedo = [];
  });
  $effect(() => {
    activeMapBank;
    mapUndo = [];
    mapRedo = [];
    mapHover = null;
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

  // All eight bits the VM exposes. Naming and explaining them is the point of
  // the redesign — a raw bitmask told nobody what a flag actually did.
  const spriteFlagNames = $derived([
    { name: 'Solid', hint: 'Blocks movement', dot: palette[3] },
    { name: 'Hazard', hint: 'Damages the player', dot: palette[8] },
    { name: 'Pickup', hint: 'Collectible', dot: palette[10] },
    { name: 'Water', hint: 'Slows movement', dot: palette[12] },
    { name: 'Ladder', hint: 'Climbable', dot: palette[11] },
    { name: 'Custom 5', hint: 'Yours to define', dot: palette[13] },
    { name: 'Custom 6', hint: 'Yours to define', dot: palette[14] },
    { name: 'Custom 7', hint: 'Yours to define', dot: palette[15] },
  ]);

  const noteNames = ['---', ...Array.from({ length: 96 }, (_, i) => `${['C','C#','D','D#','E','F','F#','G','G#','A','A#','B'][i % 12]}${Math.floor(i / 12)}`)];
  const assetStats = $derived(['sprite','sfx','music','color'].map((kind) => {
    const entries = assetIndex.entries.filter((entry) => entry.kind === kind);
    return { kind, used: entries.filter((entry) => entry.used || entry.nonzero).length, count: entries.length, bytes: entries.reduce((sum, entry) => sum + entry.bytes, 0), refs: entries.reduce((sum, entry) => sum + entry.refs.length, 0) };
  }));
  const codeBytes = $derived(sources.reduce((sum, source) => sum + new TextEncoder().encode(source.text).length, 0));
  const artBytes = $derived(spriteSheet.length + map.length + spriteFlags.length);
  const soundBytes = $derived(sfx.length + music.length);
  const cartPercent = $derived(Math.min(100, Math.round(cartSize.packedBytes / cartSize.maxBytes * 100)));

  const assetSummary = $derived([
    { label: 'Sprites', icon: Image, value: `${assetStats[0]?.used ?? 0}`, pct: ((assetStats[0]?.used ?? 0) / 256) * 100, detail: 'of 256 slots' },
    { label: 'Map tiles', icon: Layers, value: `${[...mapTileCounts.values()].reduce((sum, n) => sum + n, 0)}`, pct: (([...mapTileCounts.values()].reduce((sum, n) => sum + n, 0)) / 4096) * 100, detail: 'of 4 096 cells' },
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

  function commitSprite(next: number[]) {
    if (next.every((value, index) => value === sprite[index])) return;
    spriteUndo = [...spriteUndo.slice(-49), [...sprite]];
    spriteRedo = [];
    onSprite(selectedSprite, next);
  }

  function strokeSprite(pixels: Pixel[]) {
    const next = [...sprite];
    for (const pixel of pixels) next[pixel.index] = pixel.color;
    commitSprite(next);
  }

  function undoSprite() {
    const previous = spriteUndo.at(-1); if (!previous) return;
    spriteUndo = spriteUndo.slice(0, -1); spriteRedo = [...spriteRedo, [...sprite]];
    onSprite(selectedSprite, previous);
  }

  function redoSpriteEdit() {
    const next = spriteRedo.at(-1); if (!next) return;
    spriteRedo = spriteRedo.slice(0, -1); spriteUndo = [...spriteUndo, [...sprite]];
    onSprite(selectedSprite, next);
  }

  function commitMap(cells: { offset: number; tile: number }[]) {
    const latest = new globalThis.Map<number, number>();
    for (const cell of cells) latest.set(cell.offset, cell.tile);
    const edit = [...latest].map(([offset, after]) => ({ offset, before: map[offset] ?? 0, after }))
      .filter((cell) => cell.before !== cell.after);
    if (!edit.length) return;
    mapUndo = [...mapUndo.slice(-49), { kind: 'tiles', changes: edit }];
    mapRedo = [];
    onMap(edit.map(({ offset, after }) => ({ offset, tile: after })));
  }

  function commitCollision(edits: SpriteFlagEdit[]) {
    const latest = new globalThis.Map<number, number>();
    for (const edit of edits) latest.set(edit.tile, edit.flags);
    const changes = [...latest]
      .map(([tile, after]) => ({ tile, before: spriteFlags[tile] ?? 0, after }))
      .filter((edit) => edit.before !== edit.after);
    if (!changes.length) return;
    mapUndo = [...mapUndo.slice(-49), { kind: 'flags', changes }];
    mapRedo = [];
    onFlagsBatch(changes.map(({ tile, after }) => ({ tile, flags: after })));
  }

  function applyMapHistory(entry: MapHistoryEntry, side: 'before' | 'after') {
    if (entry.kind === 'tiles') {
      onMap(entry.changes.map((edit) => ({ offset: edit.offset, tile: edit[side] })));
    } else {
      onFlagsBatch(entry.changes.map((edit) => ({ tile: edit.tile, flags: edit[side] })));
    }
  }

  function undoMap() {
    const entry = mapUndo.at(-1);
    if (!entry) return;
    mapUndo = mapUndo.slice(0, -1);
    mapRedo = [...mapRedo.slice(-49), entry];
    applyMapHistory(entry, 'before');
  }

  function redoMapEdit() {
    const entry = mapRedo.at(-1);
    if (!entry) return;
    mapRedo = mapRedo.slice(0, -1);
    mapUndo = [...mapUndo.slice(-49), entry];
    applyMapHistory(entry, 'after');
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
  }

  function beginMapPan(event: PointerEvent) {
    const panGesture = event.button === 2 || event.button === 1 || (event.button === 0 && event.ctrlKey);
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

  function transformSprite(kind: 'flip' | 'rotate' | 'clear') {
    const next = Array(64).fill(0);
    for (let y = 0; y < 8; y += 1) for (let x = 0; x < 8; x += 1) {
      if (kind === 'flip') next[y * 8 + (7 - x)] = sprite[y * 8 + x];
      if (kind === 'rotate') next[x * 8 + (7 - y)] = sprite[y * 8 + x];
    }
    commitSprite(kind === 'clear' ? next : next);
  }

  function selectSprite(index: number) {
    selectedSprite = index; spriteUndo = []; spriteRedo = [];
  }

  // Each sfx slot is 16 steps x 4 bytes: note, volume, wave (0 square / 1 noise),
  // effect. Note 0 is a rest; notes run 1..96 (C0..B7) via note_to_freq in the VM.
  const SFX_NOTE_MAX = 96;
  const SFX_VOLUME_MAX = 15;
  const sfxEffects = [
    { label: '—', hint: 'No effect' },
    { label: 'SL', hint: 'Slide to next note' },
    { label: 'VB', hint: 'Vibrato' },
    { label: 'DR', hint: 'Drop pitch' },
  ];
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
    const empty = Array.from({ length: 8 }, (_, pattern) => music.slice(pattern * 32, pattern * 32 + 32).some(Boolean)).findIndex((filled) => !filled);
    soundSelection.pattern = empty >= 0 ? empty : (selectedPattern + 1) % 8;
  }

  const sfxByte = (step: number, field: number) => sfx[selectedSfx * 64 + step * 4 + field] ?? 0;
  const sfxStepActive = (step: number) => sfxPlaying && audio.sfxStep === step;

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
    if (changed) onSfx(selectedSfx, bytes);
  }

  let sfxDrawing = $state<{ field: number; erase: boolean } | null>(null);

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
    sfxDrawing = null;
  }

  function changeMusic(row: number, channel: number) {
    const bytes = music.slice(selectedPattern * 32, selectedPattern * 32 + 32);
    const at = row * 2 + channel;
    bytes[at] = ((bytes[at] ?? 0) + 1) % 17;
    onMusic(selectedPattern, bytes);
  }

  function jumpToRef(reference: { path: string; line?: number; col?: number }) {
    onOpenSource(reference.path, reference.line ?? null, reference.col ?? null);
  }

  function updatePalette(hex: string) {
    if (/^#[0-9a-f]{6}$/i.test(hex)) onPalette(selectedSlot, hex.toUpperCase());
  }

  function updateChannel(channel: number, value: number) {
    const channels = [0, 1, 2].map((index) => parseInt(palette[selectedSlot].slice(1 + index * 2, 3 + index * 2), 16));
    channels[channel] = value;
    updatePalette(`#${channels.map((part) => part.toString(16).padStart(2, '0')).join('')}`);
  }

  function signature(entry: ApiEntry) {
    return `${entry.name}(${entry.params.map((p) => `${p.name}: ${p.ty}`).join(', ')})`;
  }

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;
  }

  function handleWorkspaceKeys(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z') {
      if (screen === 'sprites') {
        event.preventDefault();
        if (event.shiftKey) redoSpriteEdit(); else undoSprite();
      } else if (screen === 'map') {
        event.preventDefault();
        if (event.shiftKey) redoMapEdit(); else undoMap();
      }
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
</script>

<svelte:window onkeydown={handleWorkspaceKeys} />

<main class="workspace">
  {#if ['sprites', 'map', 'palette'].includes(screen)}
    <nav class="subnav">
      <Button variant="ghost" class={screen === 'sprites' ? 'active' : undefined} onclick={() => onNavigate('sprites')}><Image size={15} />Sprites</Button>
      <Button variant="ghost" class={screen === 'map' ? 'active' : undefined} onclick={() => onNavigate('map')}><Layers size={15} />Map</Button>
      <Button variant="ghost" class={screen === 'palette' ? 'active' : undefined} onclick={() => onNavigate('palette')}><Pipette size={15} />Palette</Button>
      {#if screen === 'sprites' || screen === 'map' || screen === 'palette'}
        {@const bankKind = screen === 'sprites' ? 'sprites' : screen === 'map' ? 'map' : 'palette'}
        {@const bankIds = screen === 'sprites' ? spriteBanks : screen === 'map' ? mapBanks : paletteBanks}
        {@const activeBank = screen === 'sprites' ? activeSpriteBank : screen === 'map' ? activeMapBank : activePaletteBank}
        <div class="bank-picker">
          <span>Bank</span>
          <select value={activeBank} onchange={async (event) => { const select = event.currentTarget; if (await onAssetBank(bankKind, 'select', Number(select.value)) === false) select.value = String(activeBank); }}>
            {#each bankIds as id}<option value={id}>{id}</option>{/each}
          </select>
          <button title={`Create ${bankKind} bank`} onclick={() => onAssetBank(bankKind, 'create')}><Plus size={14} /></button>
          <button class="danger" disabled={activeBank === 0} title={`Delete ${bankKind} bank ${activeBank}`} onclick={() => onAssetBank(bankKind, 'delete', activeBank)}><Trash2 size={14} /></button>
        </div>
      {/if}
      <code>{screen === 'sprites' ? `${assetStats[0]?.used ?? 0} of 256 used` : screen === 'map' ? '64 × 64 tiles' : '16 colors'}</code>
    </nav>
  {:else if ['sfx', 'music'].includes(screen)}
    <nav class="subnav">
      <Button variant="ghost" class={screen === 'sfx' ? 'active' : undefined} onclick={() => onNavigate('sfx')}><Volume2 size={15} />Sound effects</Button>
      <Button variant="ghost" class={screen === 'music' ? 'active' : undefined} onclick={() => onNavigate('music')}><Music size={15} />Music</Button>
      {#if screen === 'sfx' || screen === 'music'}
        {@const bankKind = screen === 'sfx' ? 'sfx' : 'music'}
        {@const bankIds = screen === 'sfx' ? sfxBanks : musicBanks}
        {@const activeBank = screen === 'sfx' ? activeSfxBank : activeMusicBank}
        <div class="bank-picker">
          <span>Bank</span>
          <select value={activeBank} onchange={async (event) => { const select = event.currentTarget; if (await onAssetBank(bankKind, 'select', Number(select.value)) === false) select.value = String(activeBank); }}>
            {#each bankIds as id}<option value={id}>{id}</option>{/each}
          </select>
          <button title={`Create ${bankKind} bank`} onclick={() => onAssetBank(bankKind, 'create')}><Plus size={14} /></button>
          <button class="danger" disabled={activeBank === 0} title={`Delete ${bankKind} bank ${activeBank}`} onclick={() => onAssetBank(bankKind, 'delete', activeBank)}><Trash2 size={14} /></button>
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
              <div class:done={i < 2}><i>{i < 2 ? '✓' : step[0]}</i><span><strong>{step[1]}</strong><small>{['Real Lua, familiar tools.','See every change instantly.','Paint sprites and maps.','Pack or publish.'][i]}</small></span></div>
            {/each}
          </div>
          <Button variant="ghost" onclick={onTour}>Take 4-step tour<ArrowRight size={15} /></Button>
        </aside>
      {/if}
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
          {#key active?.name ?? ''}
            <LuaEditor
              value={active?.text ?? ''}
              path={active?.name ?? ''}
              initialCursor={sourceCursor[active?.name ?? ''] ?? 0}
              {api}
              {diagnostics}
              {breakpoints}
              {insertRequest}
              {revealRequest}
              {onInsertHandled}
              {onRevealHandled}
              onChange={onCode}
              onCursor={(source, offset) => sourceCursor[source] = offset}
              onToggleBreakpoint={onBreakpoint}
            />
          {/key}
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
        <span></span>
        <button title="Undo sprite edit" disabled={!spriteUndo.length} onclick={undoSprite}><Undo2 size={18} /></button><button title="Redo sprite edit" disabled={!spriteRedo.length} onclick={redoSpriteEdit}><Redo2 size={18} /></button>
        <button title="Flip horizontally" onclick={() => transformSprite('flip')}><FlipHorizontal size={18} /></button><button title="Rotate clockwise" onclick={() => transformSprite('rotate')}><RotateCw size={18} /></button>
        <button class="danger" title="Clear sprite" onclick={() => transformSprite('clear')}><Trash2 size={18} /></button>
      </aside>
      <div class="asset-canvas-wrap" data-tour-target="draw">
        <div class="asset-heading"><span><span class="eyebrow">Sprite</span><strong>{selectedSprite.toString().padStart(3,'0')}</strong></span><code>8 × 8 px · 64 bytes</code></div>
        <SpriteCanvas
          {sprite}
          {palette}
          {selectedColor}
          {tool}
          onStroke={strokeSprite}
          onPick={(color) => selectedColor = color}
        />
        <div class="palette-strip">
          {#each palette as color, index}<button aria-label={`Color ${index}`} class:active={selectedColor === index} style={`--swatch:${color}`} onclick={() => selectedColor = index}></button>{/each}
        </div>
        <div class="used-by"><span class="eyebrow">Used by</span>{#each assetIndex.entries.find((entry) => entry.kind === 'sprite' && entry.id === selectedSprite)?.refs ?? [] as reference}<button onclick={() => jumpToRef(reference)}>{reference.label}</button>{/each}{#if !(assetIndex.entries.find((entry) => entry.kind === 'sprite' && entry.id === selectedSprite)?.refs.length)}<small>No indexed references</small>{/if}</div>
      </div>
      <aside class="sheet-panel">
        <div class="panel-cap"><span class="eyebrow">Sprite sheet</span><code>256 slots</code></div>
        <div class="sprite-sheet">
          {#each Array(256) as _, index}
            <button
              class:active={selectedSprite === index}
              class:empty={!spriteUsed[index]}
              title={`Sprite ${index.toString().padStart(3, '0')}${spriteUsed[index] ? '' : ' — empty'}`}
              onclick={() => selectSprite(index)}
            >
              {#if spriteUsed[index]}
                {#each Array(64) as _, p}<i style={`background:${palette[spriteSheet[index * 64 + p] ?? 0]}`}></i>{/each}
              {/if}
            </button>
          {/each}
        </div>
        <section class="flags">
          <span class="eyebrow">Behaviour flags</span>
          {#each spriteFlagNames as flag, i}
            <label title={`bit ${i}`}>
              <span>
                <i style={`background:${flag.dot}`}></i>
                <b class="flag-name">{flag.name}</b>
                <small>{flag.hint}</small>
              </span>
              <code>bit {i}</code>
              <input
                type="checkbox"
                checked={Boolean((spriteFlags[selectedSprite] ?? 0) & (1 << i))}
                onchange={(event) => onFlags(selectedSprite, event.currentTarget.checked ? (spriteFlags[selectedSprite] ?? 0) | (1 << i) : (spriteFlags[selectedSprite] ?? 0) & ~(1 << i))}
              />
              <b class="flag-switch"></b>
            </label>
          {/each}
          <p class="flag-raw">flags = 0x{(spriteFlags[selectedSprite] ?? 0).toString(16).padStart(2, '0').toUpperCase()}</p>
        </section>
      </aside>
    </section>

  {:else if screen === 'map'}
    <section class="map-screen">
      <div class="map-toolbar">
        <div class="map-layer-switch" aria-label="Map edit layer">
          <button class:active={mapLayer === 'tiles'} onclick={() => mapLayer = 'tiles'}><Layers size={15} />Tiles</button>
          <button class:active={mapLayer === 'collision'} onclick={() => { mapLayer = 'collision'; collisionOverlay = true; if (mapTool === 'pick') mapTool = 'pencil'; }}><ShieldCheck size={15} />Collision</button>
        </div>
        <i class="map-toolbar-divider"></i>
        {#each editorTools as item}
          {@const Icon = item.icon}
          <button class:active={mapTool === item.id} title={`${item.label} (${item.shortcut})`} onclick={() => mapTool = item.id as MapTool}><Icon size={16} />{item.label}</button>
        {/each}
        <button title="Undo map edit" disabled={!mapUndo.length} onclick={undoMap}><Undo2 size={16} /></button>
        <button title="Redo map edit" disabled={!mapRedo.length} onclick={redoMapEdit}><Redo2 size={16} /></button>
        <span class="map-toolbar-spacer"></span>
        {#if mapLayer === 'collision'}
          <div class="collision-brush" aria-label="Collision brush">
            <button class:active={collisionBrush === 0 && mapTool !== 'erase'} onclick={() => { collisionBrush = 0; if (mapTool === 'erase') mapTool = 'pencil'; }}><i class="brush-dot walkable"></i>Walkable</button>
            <button class:active={collisionBrush === 1 && mapTool !== 'erase'} onclick={() => { collisionBrush = 1; if (mapTool === 'erase') mapTool = 'pencil'; }}><i class="brush-dot solid"></i>Solid</button>
            <button class:active={collisionBrush === 2 && mapTool !== 'erase'} onclick={() => { collisionBrush = 2; if (mapTool === 'erase') mapTool = 'pencil'; }}><i class="brush-dot hazard"></i>Hazard</button>
          </div>
        {/if}
        {#if mapLayer === 'tiles'}
          <label><input type="checkbox" bind:checked={collisionOverlay} />Collision overlay</label>
        {/if}
        <div class="map-zoom" aria-label="Map zoom">{#each MAP_ZOOM_LEVELS as value}<button class:active={Math.abs(mapZoom - value) < 0.02} onclick={() => mapZoom = value}>{value * 100}%</button>{/each}</div>
        <code class="map-zoom-readout">{Math.round(mapZoom * 100)}%</code>
      </div>
      <div
        class="map-work"
        class:panning={mapPanning}
        role="region"
        aria-label="Map canvas"
        title="Mouse wheel zoom · right or middle drag pan"
        onwheel={handleMapWheel}
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
          {spriteFlags}
          {selectedTile}
          collision={collisionOverlay || mapLayer === 'collision'}
          layer={mapLayer}
          {collisionBrush}
          tool={mapTool}
          zoom={mapZoom}
          onStroke={commitMap}
          onCollisionStroke={commitCollision}
          onPick={(tile) => selectedTile = tile}
          onCollisionPick={(brush) => { collisionBrush = brush; mapTool = 'pencil'; }}
          onHover={(cell) => mapHover = cell}
        />
        {/key}
      </div>
      <aside class="map-inspector">
        {#if mapLayer === 'collision'}
          <div class="collision-edit-note">
            <span class="eyebrow"><ShieldCheck size={13} />Collision painting</span>
            <strong>{mapTool === 'erase' || collisionBrush === 0 ? 'Walkable' : collisionBrush === 1 ? 'Solid' : 'Hazard'} brush</strong>
            <p>Per tile type, not per cell — painting one cell recolors every cell using that tile everywhere on the map. Other flags (2–7) stay untouched.</p>
          </div>
        {/if}
        <span class="eyebrow">Tile picker</span>
        <div class="tile-picker">
          {#each Array(256) as _, i}
            <button
              aria-label={`Tile ${i.toString().padStart(3, '0')}${spriteUsed[i] ? '' : ' — empty'}`}
              title={`Tile ${i.toString().padStart(3, '0')}${spriteUsed[i] ? '' : ' — empty'}`}
              onclick={() => selectedTile = i}
              class:active={i === selectedTile}
              class:empty={!spriteUsed[i]}
            >
              {#if spriteUsed[i]}
                {#each Array(64) as _, p}<i style={`background:${palette[spriteSheet[i * 64 + p] ?? 0]}`}></i>{/each}
              {/if}
            </button>
          {/each}
        </div>
        <div class="inspector-row"><span>Cell</span><code>{mapHover ? `${mapHover.x}, ${mapHover.y}` : '—'}</code></div>
        <div class="inspector-row"><span>Hovered tile</span><code>{mapHover ? `${mapHover.tile.toString().padStart(3,'0')} · ${((spriteFlags[mapHover.tile] ?? 0) & 2) !== 0 ? 'hazard' : ((spriteFlags[mapHover.tile] ?? 0) & 1) !== 0 ? 'solid' : 'walkable'}` : '—'}</code></div>
        <div class="inspector-row"><span>Selected</span><code>{selectedTile.toString().padStart(3,'0')} · 0x{selectedTile.toString(16).padStart(2,'0')}</code></div>
        <div class="collision-key"><span class="eyebrow">Collision</span><p class="solid"><i></i>Solid · {spriteFlags.filter((flags) => (flags & 1) !== 0).length} tile types</p><p class="hazard"><i></i>Hazard · {spriteFlags.filter((flags) => (flags & 2) !== 0).length} tile types</p></div>
        {#if mapEmpty}
          <p class="map-note">
            This map is empty. Pick a tile and paint to start it.
          </p>
        {/if}
        <p class="map-note subtle">Wheel zoom · right/middle drag pan · Pick tool samples. Tile 000 stays collision-free.</p>
      </aside>
    </section>

  {:else if screen === 'palette'}
    <section class="palette-screen">
      <header><span><span class="eyebrow">Cart palette</span><h1>Palette</h1></span><p>Sixteen colors shared by every sprite, tile, and draw call.</p></header>
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
          <label>Hex<input value={palette[selectedSlot]} onblur={(e) => updatePalette(e.currentTarget.value)} /></label>
          {#each ['Red','Green','Blue'] as channel, i}
            <label>{channel}<input type="range" min="0" max="255" value={parseInt(palette[selectedSlot].slice(1 + i * 2, 3 + i * 2), 16)} oninput={(event) => updateChannel(i, Number(event.currentTarget.value))} /><code>{parseInt(palette[selectedSlot].slice(1 + i * 2, 3 + i * 2),16)}</code></label>
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
        </header>

        <div class="sfx-tracker">
          <div class="sfx-labels">
            <span class="sfx-label-step">step</span>
            <div class="sfx-label-pitch">
              {#each pitchAxis as mark}<span style={`bottom:${mark.at}%`}>{mark.name}</span>{/each}
            </div>
            <div class="sfx-label-volume"><span>15</span><span>8</span><span>0</span></div>
            <span class="sfx-label-row">wave</span>
            <span class="sfx-label-row">fx</span>
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

            <div class="sfx-fx">
              {#each Array(16) as _, step}
                {@const fx = sfxByte(step, 3)}
                <button
                  class:active={fx > 0}
                  title={sfxEffects[fx]?.hint ?? 'No effect'}
                  onclick={() => setSfxCells([{ step, field: 3, value: (fx + 1) % sfxEffects.length }])}
                >{sfxEffects[fx]?.label ?? '—'}</button>
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
        <p class="sfx-hints subtle">The effect column is stored in the cart, but the VM does not apply it yet.</p>
      </div>
    </section>

  {:else if screen === 'music'}
    <section class="music-screen">
      <aside class="pattern-list">
        <div class="panel-cap"><span class="eyebrow">Patterns</span><button title="Select first empty pattern" onclick={selectEmptyPattern}><Plus size={14} /></button></div>
        {#each Array(8) as _, index}
          <button class:active={selectedPattern === index} onclick={() => soundSelection.pattern = index}>
            <code>{index.toString().padStart(2,'0')}</code><span>{music.slice(index * 32, index * 32 + 32).some(Boolean) ? `Pattern ${index.toString().padStart(2,'0')}` : 'Empty pattern'}</span>
          </button>
        {/each}
        <div class="song-order"><span class="eyebrow">Playback</span><button class:active={audio.musicActive} onclick={() => onAudio('music', audio.musicPattern, audio.musicActive ? 'stop' : 'play')}><code>{audio.musicPattern.toString().padStart(2,'0')}</code>{audio.musicActive ? `Row ${audio.musicRow.toString(16).toUpperCase()}` : 'Stopped'}<small>{audio.musicLoop ? 'loop' : 'once'}</small></button></div>
      </aside>
      <div class="music-grid-wrap">
        <header><span><span class="eyebrow">Pattern {selectedPattern.toString().padStart(2,'0')}</span><h2>Pattern {selectedPattern.toString().padStart(2,'0')}</h2></span><button class="btn secondary" onclick={() => onAudio('music', selectedPattern, musicPlaying ? 'stop' : 'play')}>{#if musicPlaying}<Square size={14} />Stop{:else}<Play size={14} />Play pattern{/if}</button></header>
        <div class="music-grid">
          <div class="music-head"><span>Row</span><span>Channel 1</span><span>Channel 2</span></div>
          {#each Array(16) as _, row}
            <div class:playhead={audio.musicActive && audio.musicPattern === selectedPattern && audio.musicRow === row}><code>{row.toString(16).toUpperCase().padStart(2,'0')}</code>{#each Array(2) as _, channel}{@const cell = music[selectedPattern * 32 + row * 2 + channel] ?? 0}<button onclick={() => changeMusic(row, channel)}>{cell ? `SFX ${(cell - 1).toString().padStart(2,'0')}` : '—'}</button>{/each}</div>
          {/each}
        </div>
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
          {#if !portAccount.authenticated}<Button variant="outline" onclick={onOpenPortAccount}>Open Port account</Button>{/if}
        </div>
        <aside class="cart-preview">
          <span class="eyebrow">Port preview</span>
          <div class="cover-art">
            {#if frameData?.length === 128 * 128 * 4}
              <canvas bind:this={coverCanvas} width="128" height="128"></canvas>
            {:else}
              {#each Array(256) as _,p}<i style={`background:${palette[(p * 7 + 5) % 16]}`}></i>{/each}
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
          <p>Sign in or register in the browser tab, then approve Caiven Studio there — Studio picks it up automatically.</p>
          <p class="account-expiry">Link expires {portLinkExpiresAt ? new Date(portLinkExpiresAt).toLocaleTimeString() : 'soon'}.</p>
          <Button variant="outline" disabled={portBusy} onclick={onPortLinkCancel}>Cancel</Button>
        {:else}
          <div class="account-avatar"><UserRound size={28} /></div>
          <span class="account-status">Not linked</span>
          <h2>Link Port account</h2>
          <p>Required before publishing. The browser handles sign-in — Studio never sees your password.</p>
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
</main>
