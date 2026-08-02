<script lang="ts">
  import { onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { PanelRightOpen, WifiOff } from '@lucide/svelte';
  import { Button } from '@caiven/ui/button';
  import * as Tooltip from '@caiven/ui/tooltip';
  import { Toaster, toast } from '@caiven/ui/sonner';
  import Header from './components/Header.svelte';
  import ModeRail from './components/ModeRail.svelte';
  import Workspace from './components/Workspace.svelte';
  import ConsolePane from './components/ConsolePane.svelte';
  import Drawer from './components/Drawer.svelte';
  import Overlays from './components/Overlays.svelte';
  import type {
    CartTemplateSummary, Diagnostic, EditorInsertRequest, EditorRevealRequest, ExampleSummary, LocalCart, PortCart, PortSession,
    PublishProgress, Screen, StudioBootstrap, TickSnapshot,
  } from './types';
  import {
    bootstrap, chooseExportPath, chooseProject, exportCartridge, fallbackExamples, fallbackTemplates, isTauri, listExamples, listTemplates, newProject,
    openProject, readAssetIndex, readCartSize, readFrame, readMemory, readTick, remixExample, saveProject, setInput, transport,
    addWatch, assetBank, audioTransport, clearOutput, closeProject, COLLISION_LEN, createModule, MEMORY, portDownload, portLinkCancel, portLinkPoll, portLinkStart, portListCarts,
    portLogout, portPublish, portSession, portSetUrl, scanLibrary, toggleBreakpoint, writeBuffer,
    removeRecent, removeWatch, writeCollisionCells, writeCollisionTypes, writeMapCells, writeMemory, writeMeta, writePalette, writeSprite,
  } from './lib/ipc';
  import { plural, tidyPath } from './lib/format';
  import { createGamepadInput } from './lib/gamepad';

  let studio = $state<StudioBootstrap>({
    connected: false, title: '', path: '', author: '', runState: 'stopped',
    frame: 0, fps: 0, cartSize: { packedBytes: 0, maxBytes: 128 * 1024 }, sources: [], palette: [], spriteSheet: [], map: [], spriteBanks: [0], mapBanks: [0], activeSpriteBank: 0, activeMapBank: 0, collision: [], collisionTypes: [],
    sfx: [], music: [], paletteBanks: [0], activePaletteBank: 0, sfxBanks: [0], activeSfxBank: 0, musicBanks: [0], activeMusicBank: 0, ram: [], globals: [], watches: [], callStack: [], breakpoints: [], pauseReason: null, diagnostics: [], output: [],
    meta: { description: '', tags: [] }, assetIndex: { entries: [], computedRefs: 0 },
    audio: { sfxActive: false, sfxId: 0, sfxStep: 0, musicActive: false, musicPattern: 0, musicRow: 0, musicLoop: true },
    recent: [], api: [],
  });
  let screen = $state<Screen>('code');
  let activeSource = $state(0);
  let drawerOpen = $state(false);
  let drawerTab = $state<'problems' | 'output' | 'memory'>('problems');
  let consoleOpen = $state(true);
  let consoleWidth = $state(604);
  let resizing = $state(false);
  let overlay = $state<'palette' | 'publish' | 'tour' | 'focus' | 'module' | 'new-cart' | 'controls' | null>(null);
  let status = $state('Starting Studio…');
  let frameData = $state<Uint8Array | null>(null);
  let frameTime = $state(5.2);
  let metaDirty = $state(false);
  let writeTimer: ReturnType<typeof setTimeout> | undefined;
  let localCarts = $state<LocalCart[]>([]);
  let portCarts = $state<PortCart[]>([]);
  let portAccount = $state<PortSession>({ authenticated: false, username: '', portUrl: '' });
  let portLink = $state<{ requestId: string; pollSecret: string; expiresAt: string } | null>(null);
  let portBusy = $state(false);
  let portError = $state('');
  let publishProgress = $state<PublishProgress | null>(null);
  let publishError = $state('');
  let publishDone = $state('');
  let pendingWrites = $state(0);
  let handledPause = $state('');
  let handledDiagnostic = $state('');
  let insertRequest = $state<EditorInsertRequest | null>(null);
  let revealRequest = $state<EditorRevealRequest | null>(null);
  let insertSerial = 0;
  let revealSerial = 0;
  type BankKind = 'sprites' | 'map' | 'palette' | 'sfx' | 'music';
  const bankRefreshes = new Set<BankKind>();
  let templates = $state<CartTemplateSummary[]>(fallbackTemplates);
  let examples = $state<ExampleSummary[]>(fallbackExamples);

  // Button indices match the VM's Button enum (0 Up, 1 Down, 2 Left, 3 Right, 4 A, 5 B).
  const BUTTON_LABELS = ['Up', 'Down', 'Left', 'Right', 'A', 'B'];
  const DEFAULT_KEYMAP: Record<number, string[]> = {
    0: ['ArrowUp', 'w'], 1: ['ArrowDown', 's'], 2: ['ArrowLeft', 'a'],
    3: ['ArrowRight', 'd'], 4: ['j'], 5: ['k'],
  };
  const INPUT_STORAGE_KEY = 'caiven-studio-input';

  function loadKeymap(): Record<number, string[]> {
    try {
      const raw = localStorage.getItem(INPUT_STORAGE_KEY);
      if (!raw) return structuredClone(DEFAULT_KEYMAP);
      const parsed = JSON.parse(raw) as Record<string, string[]>;
      const keymap = structuredClone(DEFAULT_KEYMAP);
      for (const button of Object.keys(DEFAULT_KEYMAP)) {
        const keys = parsed[button];
        if (Array.isArray(keys) && keys.every((key) => typeof key === 'string') && keys.length) {
          keymap[Number(button)] = keys;
        }
      }
      return keymap;
    } catch {
      return structuredClone(DEFAULT_KEYMAP);
    }
  }

  let keymap = $state<Record<number, string[]>>(loadKeymap());

  $effect(() => {
    localStorage.setItem(INPUT_STORAGE_KEY, JSON.stringify(keymap));
  });

  function gameButton(key: string): number | undefined {
    const lower = key.length === 1 ? key.toLowerCase() : key;
    for (const button of Object.keys(keymap)) {
      const keys = keymap[Number(button)];
      if (keys.includes(key) || keys.includes(lower)) return Number(button);
    }
    return undefined;
  }

  // Rebinding a button replaces its whole alias list with just the new key —
  // once customized, a button is exactly what the player set it to; only the
  // untouched defaults keep their arrows+WASD dual binding.
  function rebindButton(button: number, key: string) {
    keymap = { ...keymap, [button]: [key] };
  }

  function resetKeymap() {
    keymap = structuredClone(DEFAULT_KEYMAP);
  }
  // Which sound slot the editors have selected, shared with Workspace so the
  // space-to-preview shortcut acts on the same thing the user is looking at.
  let soundSelection = $state({ sfx: 0, pattern: 0 });

  // Mirrors what the VM believes is held, so the on-screen input map lights up
  // for keyboard play and not only for clicks on the chips themselves.
  let heldButtons = $state<number[]>([]);

  // Imperative handle onto Workspace so the ⌘K command palette can trigger
  // undo/redo for whichever asset editor is on screen; historyStatus mirrors its
  // availability so the palette can show it (and no-op harmlessly when empty).
  let workspaceRef: Workspace | undefined;
  let historyStatus = $state({ canUndo: false, canRedo: false });

  function pressButton(button: number, pressed: boolean) {
    if (pressed) {
      if (!heldButtons.includes(button)) heldButtons = [...heldButtons, button];
    } else {
      heldButtons = heldButtons.filter((value) => value !== button);
    }
    void setInput(button, pressed);
  }

  const gamepad = createGamepadInput({
    onButton: pressButton,
    onConnect: (label) => showToast(`Gamepad connected: ${label}`),
    onDisconnect: () => showToast('Gamepad disconnected'),
  });

  const consoleScreens: Screen[] = ['code', 'sprites', 'map', 'palette', 'sfx', 'music'];
  const consoleRelevant = $derived(consoleScreens.includes(screen));
  let tourDone = $state(false);

  const dirty = $derived(metaDirty || studio.sources.some((source) => source.dirty));
  const running = $derived(studio.runState === 'running');
  const allDiagnostics = $derived<Diagnostic[]>([
    ...studio.diagnostics,
    ...studio.assetIndex.entries
      // Only assets that cost cart space are worth flagging. The palette is a
      // fixed 16 slots whether or not a cart draws with them, so an unused
      // colour is normal and reporting it drowns out real problems.
      .filter((entry) => entry.kind !== 'color' && entry.nonzero && !entry.used)
      .slice(0, 30)
      .map((entry): Diagnostic => ({
        severity: 'info',
        title: `${entry.kind[0].toUpperCase()}${entry.kind.slice(1)} ${entry.id.toString().padStart(entry.kind === 'sprite' ? 3 : 2, '0')} is unused`,
        detail: `It occupies ${entry.bytes} bytes but has no indexed references.`,
        path: entry.kind === 'sprite' ? 'sprites.png' : entry.kind,
        line: null,
      })),
  ]);

  $effect(() => {
    localStorage.setItem('caiven-studio-layout', JSON.stringify({ screen, drawerOpen, drawerTab, consoleOpen, consoleWidth }));
  });

  function startResize(event: PointerEvent) {
    event.preventDefault();
    resizing = true;
    const startX = event.clientX;
    const startWidth = consoleWidth;
    const onMove = (moveEvent: PointerEvent) => {
      const next = startWidth - (moveEvent.clientX - startX);
      const maxWidth = Math.min(900, Math.max(320, window.innerWidth - 560));
      consoleWidth = Math.min(maxWidth, Math.max(320, next));
    };
    const onUp = () => {
      resizing = false;
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
  }

  function showToast(message: string) {
    toast(message);
  }

  function confirmDiscard(action: string) {
    return !dirty || window.confirm(`${action} and discard unsaved changes?`);
  }

  function errorText(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  async function refreshCartSize() {
    try { studio.cartSize = await readCartSize(); }
    catch { /* Size display must not turn a successful edit into a failed edit. */ }
  }

  async function commitMutation(label: string, write: () => Promise<void>, rollback: () => void) {
    pendingWrites += 1;
    try {
      await write();
      await refreshCartSize();
    } catch (error) {
      rollback();
      showToast(`${label} failed: ${errorText(error)}`);
    } finally {
      pendingWrites -= 1;
    }
  }

  function applyTick(tick: TickSnapshot) {
    const wasRunning = studio.runState === 'running';
    studio.runState = tick.runState;
    studio.frame = tick.frame;
    studio.fps = tick.fps;
    frameTime = tick.frameTimeMs;
    studio.globals = tick.globals;
    studio.watches = tick.watches;
    studio.callStack = tick.callStack;
    studio.pauseReason = tick.pauseReason;
    studio.audio = tick.audio;
    studio.diagnostics = tick.diagnostics;
    studio.output = tick.output;
    if (tick.activeSpriteBank !== studio.activeSpriteBank) void refreshAssetBank('sprites');
    if (tick.activeMapBank !== studio.activeMapBank) void refreshAssetBank('map');
    if (tick.activePaletteBank !== studio.activePaletteBank) void refreshAssetBank('palette');
    if (tick.activeSfxBank !== studio.activeSfxBank) void refreshAssetBank('sfx');
    if (tick.activeMusicBank !== studio.activeMusicBank) void refreshAssetBank('music');

    const firstError = tick.diagnostics.find((diagnostic) => diagnostic.severity === 'error');
    const diagnosticKey = firstError
      ? `${firstError.title}:${firstError.path}:${firstError.line ?? ''}:${firstError.detail}`
      : '';
    if (firstError && diagnosticKey !== handledDiagnostic) {
      drawerTab = 'problems';
      drawerOpen = true;
      status = `${firstError.title} · ${firstError.path}${firstError.line ? `:${firstError.line}` : ''}`;
    }
    handledDiagnostic = diagnosticKey;

    const reason = tick.pauseReason;
    const pauseKey = reason ? `${reason.kind}:${reason.source ?? ''}:${reason.line ?? ''}:${reason.message ?? ''}` : '';
    if (reason?.kind === 'breakpoint' && pauseKey !== handledPause) {
      const index = studio.sources.findIndex((source) => source.name === reason.source || source.path === reason.source);
      if (index >= 0) activeSource = index;
      screen = 'code';
      if (index >= 0 && reason.line) {
        revealRequest = { id: ++revealSerial, source: studio.sources[index].name, line: reason.line, column: 1 };
      }
      status = `Paused at ${reason.source ?? 'source'}:${reason.line ?? '?'}`;
    }
    handledPause = pauseKey;
    if (wasRunning && tick.runState !== 'running') releaseInputs();
  }

  const bankLabels: Record<BankKind, string> = {
    sprites: 'Sprite', map: 'Map', palette: 'Palette', sfx: 'SFX', music: 'Music',
  };

  /** `#RRGGBB` byte layout <-> raw RGB triples, matching a palette bank's on-disk shape. */
  function bytesToHexColors(bytes: number[]): string[] {
    const colors: string[] = [];
    for (let i = 0; i < bytes.length; i += 3) {
      const rgb = [bytes[i] ?? 0, bytes[i + 1] ?? 0, bytes[i + 2] ?? 0];
      colors.push(`#${rgb.map((c) => c.toString(16).padStart(2, '0')).join('')}`.toUpperCase());
    }
    return colors;
  }

  function applyAssetBank(bank: Awaited<ReturnType<typeof assetBank>>) {
    studio.ram.splice(MEMORY[bank.kind], bank.data.length, ...bank.data);
    if (bank.kind === 'sprites') {
      studio.spriteBanks = bank.ids; studio.activeSpriteBank = bank.active; studio.spriteSheet = bank.data;
    } else if (bank.kind === 'map') {
      studio.mapBanks = bank.ids; studio.activeMapBank = bank.active; studio.map = bank.data;
    } else if (bank.kind === 'palette') {
      studio.paletteBanks = bank.ids; studio.activePaletteBank = bank.active; studio.palette = bytesToHexColors(bank.data);
    } else if (bank.kind === 'sfx') {
      studio.sfxBanks = bank.ids; studio.activeSfxBank = bank.active; studio.sfx = bank.data;
    } else {
      studio.musicBanks = bank.ids; studio.activeMusicBank = bank.active; studio.music = bank.data;
    }
  }

  const bankRequestSerial: Record<BankKind, number> = { sprites: 0, map: 0, palette: 0, sfx: 0, music: 0 };

  async function refreshAssetBank(kind: BankKind) {
    if (bankRefreshes.has(kind)) return;
    bankRefreshes.add(kind);
    const request = bankRequestSerial[kind];
    try {
      const next = await assetBank(kind, 'read');
      if (request === bankRequestSerial[kind]) applyAssetBank(next);
    }
    catch (error) { showToast(`Bank refresh failed: ${errorText(error)}`); }
    finally { bankRefreshes.delete(kind); }
  }

  const activeBankOf: Record<BankKind, () => number> = {
    sprites: () => studio.activeSpriteBank, map: () => studio.activeMapBank,
    palette: () => studio.activePaletteBank, sfx: () => studio.activeSfxBank, music: () => studio.activeMusicBank,
  };

  async function changeAssetBank(kind: BankKind, action: 'select' | 'create' | 'delete', id?: number) {
    if (action === 'delete' && !window.confirm(`Delete ${kind} bank ${id}?`)) return;
    const request = ++bankRequestSerial[kind];
    try {
      const next = await assetBank(kind, action, id);
      if (request !== bankRequestSerial[kind]) return false;
      applyAssetBank(next);
      studio.assetIndex = await readAssetIndex();
      if (request !== bankRequestSerial[kind]) return false;
      await refreshCartSize();
      if (request !== bankRequestSerial[kind]) return false;
      status = `${bankLabels[kind]} bank ${activeBankOf[kind]()}`;
      return true;
    } catch (error) {
      if (request !== bankRequestSerial[kind]) return false;
      showToast(`Bank ${action} failed: ${errorText(error)}`);
      return false;
    }
  }

  async function doTransport(action: 'run' | 'pause' | 'reset' | 'step') {
    try {
      if (action !== 'pause') {
        clearTimeout(writeTimer);
        await Promise.all(studio.sources.filter((source) => source.dirty).map((source) => writeBuffer(source.path, source.text)));
      }
      const tick = await transport(action);
      applyTick(tick);
      if (tick.pauseReason?.kind !== 'breakpoint') {
        status = action === 'step'
          ? `Stepped to frame ${tick.frame}`
          : `${tick.runState === 'running' ? 'Running' : tick.runState === 'paused' ? 'Paused' : 'Stopped'} · ${studio.title}`;
      }
    } catch (error) {
      showToast(error instanceof Error ? error.message : String(error));
    }
  }

  async function doSave() {
    try {
      clearTimeout(writeTimer);
      await Promise.all(studio.sources.filter((source) => source.dirty).map((source) => writeBuffer(source.path, source.text)));
      const files = await saveProject();
      for (const source of studio.sources) source.dirty = false;
      metaDirty = false;
      status = `Saved ${plural(files.length, 'file')} · ${tidyPath(studio.path)}`;
      showToast(`Saved ${plural(files.length, 'file')} to ${tidyPath(studio.path)}`);
    } catch (error) {
      showToast(`Save failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  function updateCode(text: string) {
    const source = studio.sources[activeSource];
    if (!source) return;
    source.text = text;
    source.dirty = true;
    status = `Editing ${source.name}`;
    clearTimeout(writeTimer);
    writeTimer = setTimeout(() => {
      pendingWrites += 1;
      void writeBuffer(source.path, text)
        .then(() => refreshCartSize())
        .catch((error) => showToast(`Write ${source.name} failed: ${errorText(error)}`))
        .finally(() => pendingWrites -= 1);
    }, 180);
  }

  function insertBuiltin(name: string) {
    const source = studio.sources[activeSource];
    if (!source) return;
    screen = 'code';
    insertRequest = { id: ++insertSerial, source: source.name, text: `${name}()` };
  }

  function updateSprite(sprite: number, pixels: number[]) {
    const previous = studio.spriteSheet.slice(sprite * 64, sprite * 64 + 64);
    studio.spriteSheet.splice(sprite * 64, 64, ...pixels);
    studio.ram.splice(MEMORY.sprites + sprite * 64, 64, ...pixels);
    status = `Sprite ${sprite.toString().padStart(3, '0')} changed`;
    void commitMutation(`Sprite ${sprite.toString().padStart(3, '0')}`, () => writeSprite(sprite, pixels), () => {
      studio.spriteSheet.splice(sprite * 64, 64, ...previous);
      studio.ram.splice(MEMORY.sprites + sprite * 64, 64, ...previous);
    });
  }

  function updateCollision(edits: { offset: number; value: number }[]) {
    const previous = edits.map((edit) => ({ offset: edit.offset, value: studio.collision[edit.offset] ?? 0 }));
    for (const edit of edits) {
      studio.collision[edit.offset] = edit.value;
      studio.ram[MEMORY.collision + edit.offset] = edit.value;
    }
    void commitMutation('Collision edit', () => writeCollisionCells(edits), () => {
      for (const edit of previous) {
        studio.collision[edit.offset] = edit.value;
        studio.ram[MEMORY.collision + edit.offset] = edit.value;
      }
    });
  }

  function updateCollisionTypes(types: StudioBootstrap['collisionTypes']) {
    const previous = studio.collisionTypes;
    studio.collisionTypes = types;
    void commitMutation('Collision types edit', () => writeCollisionTypes(types), () => {
      studio.collisionTypes = previous;
    });
  }

  function updateMap(cells: { offset: number; tile: number }[]) {
    const previous = cells.map((cell) => ({ offset: cell.offset, tile: studio.map[cell.offset] ?? 0 }));
    for (const cell of cells) {
      studio.map[cell.offset] = cell.tile;
      studio.ram[MEMORY.map + cell.offset] = cell.tile;
    }
    void commitMutation('Map edit', () => writeMapCells(cells), () => {
      for (const cell of previous) {
        studio.map[cell.offset] = cell.tile;
        studio.ram[MEMORY.map + cell.offset] = cell.tile;
      }
    });
  }

  function updateSfx(slot: number, bytes: number[]) {
    const previous = studio.sfx.slice(slot * 64, slot * 64 + 64);
    studio.sfx.splice(slot * 64, 64, ...bytes);
    studio.ram.splice(MEMORY.sfx + slot * 64, 64, ...bytes);
    void commitMutation(`SFX ${slot.toString().padStart(2, '0')}`, () => writeMemory(MEMORY.sfx + slot * 64, bytes), () => {
      studio.sfx.splice(slot * 64, 64, ...previous);
      studio.ram.splice(MEMORY.sfx + slot * 64, 64, ...previous);
    });
  }

  function updateMusic(pattern: number, bytes: number[]) {
    const previous = studio.music.slice(pattern * 32, pattern * 32 + 32);
    studio.music.splice(pattern * 32, 32, ...bytes);
    studio.ram.splice(MEMORY.music + pattern * 32, 32, ...bytes);
    void commitMutation(`Pattern ${pattern.toString().padStart(2, '0')}`, () => writeMemory(MEMORY.music + pattern * 32, bytes), () => {
      studio.music.splice(pattern * 32, 32, ...previous);
      studio.ram.splice(MEMORY.music + pattern * 32, 32, ...previous);
    });
  }

  async function doAudio(kind: 'sfx' | 'music', id: number, action: 'play' | 'stop') {
    try { studio.audio = await audioTransport(kind, id, action); }
    catch (error) { showToast(String(error)); }
  }

  function previewSound() {
    if (screen === 'sfx') {
      void doAudio('sfx', soundSelection.sfx, studio.audio.sfxActive ? 'stop' : 'play');
    } else {
      void doAudio('music', soundSelection.pattern, studio.audio.musicActive ? 'stop' : 'play');
    }
  }

  async function doBreakpoint(source: string, line: number) {
    try { studio.breakpoints = await toggleBreakpoint(source, line); }
    catch (error) { showToast(String(error)); }
  }

  async function doAddWatch(expression: string): Promise<string | null> {
    try {
      studio.watches = await addWatch(expression);
      return null;
    } catch (error) {
      const message = errorText(error);
      showToast(message);
      return message;
    }
  }

  async function doRemoveWatch(expression: string) {
    try { studio.watches = await removeWatch(expression); }
    catch (error) { showToast(String(error)); }
  }

  async function doMeta(title: string, author: string, meta: StudioBootstrap['meta']) {
    const previous = {
      title: studio.title,
      author: studio.author,
      meta: { description: studio.meta.description, tags: [...studio.meta.tags] },
      dirty: metaDirty,
    };
    studio.title = title;
    studio.author = author;
    studio.meta = meta;
    metaDirty = true;
    try { await writeMeta(title, author, meta); status = 'Cart metadata changed'; }
    catch (error) {
      studio.title = previous.title;
      studio.author = previous.author;
      studio.meta = previous.meta;
      metaDirty = previous.dirty;
      showToast(`Metadata failed: ${errorText(error)}`);
    }
  }

  async function doCreateModule(name: string): Promise<string | null> {
    try {
      const source = await createModule(name);
      studio.sources.push(source);
      activeSource = studio.sources.length - 1;
      screen = 'code';
      overlay = null;
      status = `Created ${source.name}`;
      await refreshCartSize();
      return null;
    } catch (error) {
      return errorText(error);
    }
  }

  function updatePalette(slot: number, hex: string) {
    const previous = studio.palette[slot];
    studio.palette[slot] = hex;
    status = `Palette slot ${slot.toString().padStart(2, '0')} changed`;
    void commitMutation(`Palette slot ${slot.toString().padStart(2, '0')}`, () => writePalette(slot, hex), () => {
      studio.palette[slot] = previous;
    });
  }

  function navigate(next: Screen) {
    screen = next;
    if (next === 'code') status = studio.sources[activeSource]?.name ?? 'Code';
    // The library opens on its Local tab, so don't reach for the port until the
    // Port tab is actually selected — otherwise a port outage surfaces as an
    // error on a screen that never needed the network.
  }

  function jumpToDiagnostic(diagnostic: Diagnostic) {
    const index = studio.sources.findIndex((source) => source.name === diagnostic.path || source.path === diagnostic.path);
    if (index >= 0) {
      activeSource = index;
      screen = 'code';
      if (diagnostic.line) {
        revealRequest = { id: ++revealSerial, source: studio.sources[index].name, line: diagnostic.line, column: 1 };
      }
      return;
    }
    const target: Record<string, Screen> = {
      'sprites.png': 'sprites', 'map.png': 'map', 'palette.png': 'palette',
      sprite: 'sprites', sfx: 'sfx', music: 'music', color: 'palette',
    };
    if (target[diagnostic.path]) screen = target[diagnostic.path];
  }

  function jumpToSource(source: string, line: number | null = null, column: number | null = null) {
    const index = studio.sources.findIndex((candidate) => candidate.name === source || candidate.path === source);
    if (index >= 0) {
      activeSource = index;
      screen = 'code';
      if (line && line > 0) {
        revealRequest = {
          id: ++revealSerial,
          source: studio.sources[index].name,
          line,
          column: column && column > 0 ? column : 1,
        };
      }
      return;
    }
    const target: Record<string, Screen> = {
      'sprites.png': 'sprites', 'map.png': 'map', 'palette.png': 'palette',
      'sfx.hex': 'sfx', 'music.hex': 'music',
    };
    if (target[source]) screen = target[source];
  }

  async function searchPort(query: string) {
    portBusy = true;
    portError = '';
    try { portCarts = (await portListCarts(query)).carts; }
    catch (error) { portError = describePortError(error); }
    finally { portBusy = false; }
  }

  // Transport failures arrive as `<url>: Connection Failed: Connect error: …
  // (os error 61)`. Users get the plain meaning; the raw text goes to the console.
  function describePortError(error: unknown): string {
    const raw = error instanceof Error ? error.message : String(error);
    console.error('port request failed:', raw);
    if (/connection refused|connect error|connection failed|dns|timed out/i.test(raw)) {
      return `Can’t reach ${portAccount.portUrl || 'the port'}. Check that it is running and that you are online.`;
    }
    if (/401|unauthor/i.test(raw)) return 'Your port session has expired. Log in again.';
    return raw.replace(/https?:\/\/\S+?:\s*/, '');
  }

  async function scanLocal() {
    const path = await chooseProject('Choose library folder');
    if (!path) return;
    try { localCarts = await scanLibrary(path); }
    catch (error) { showToast(String(error)); }
  }

  async function openPath(path: string) {
    if (!confirmDiscard('Open another cart')) return;
    clearTimeout(writeTimer);
    try { studio = await openProject(path); metaDirty = false; activeSource = 0; handledPause = ''; handledDiagnostic = ''; screen = 'code'; status = `Loaded ${tidyPath(studio.path)}`; }
    catch (error) { showToast(String(error)); }
  }

  async function doRemoveRecent(path: string) {
    try {
      studio.recent = await removeRecent(path);
      status = `Removed ${tidyPath(path)} from recent carts`;
    } catch (error) {
      showToast(`Could not remove recent cart: ${errorText(error)}`);
    }
  }

  async function doClearOutput() {
    try {
      await clearOutput();
      studio.output = [];
    } catch (error) {
      showToast(`Could not clear output: ${errorText(error)}`);
    }
  }

  async function downloadPort(cart: PortCart) {
    portBusy = true;
    try { await openPath(await portDownload(cart.id, cart.title)); }
    catch (error) { showToast(String(error)); }
    finally { portBusy = false; }
  }

  async function linkPort() {
    portBusy = true;
    try { portLink = await portLinkStart(); portError = 'Browser opened. Finish linking, then return.'; }
    catch (error) { portError = String(error); }
    finally { portBusy = false; }
  }

  async function pollPortLink() {
    if (!portLink) return;
    try {
      const session = await portLinkPoll(portLink.requestId, portLink.pollSecret);
      if (session) { portAccount = session; portLink = null; portError = ''; }
    } catch (error) { portLink = null; portError = String(error); }
  }

  async function cancelPortLink() {
    if (!portLink) return;
    portBusy = true;
    try { await portLinkCancel(portLink.requestId, portLink.pollSecret); portLink = null; portError = ''; }
    catch (error) { portError = String(error); }
    finally { portBusy = false; }
  }

  function openPortAccount() { screen = 'account'; }

  async function logoutPort() {
    try { portAccount = await portLogout(); } catch (error) { showToast(String(error)); }
  }

  async function setServerUrl(url: string) {
    try { portAccount = await portSetUrl(url); portError = ''; }
    catch (error) { portError = error instanceof Error ? error.message : String(error); }
  }

  async function doPublish(changelog: string) {
    publishError = '';
    publishDone = '';
    publishProgress = { step: 'pack', pct: 0, note: 'Starting' };
    try {
      clearTimeout(writeTimer);
      await Promise.all(studio.sources.filter((source) => source.dirty).map((source) => writeBuffer(source.path, source.text)));
      const result = await portPublish({
        title: studio.title, description: studio.meta.description,
        tags: studio.meta.tags, changelog,
      });
      publishDone = `${result.cartId}${result.version ? ` · v${result.version}` : ''}`;
    } catch (error) { publishError = error instanceof Error ? error.message : String(error); }
  }

  function showPublish() {
    publishProgress = null;
    publishError = '';
    publishDone = '';
    overlay = 'publish';
  }

  async function doOpen() {
    if (!confirmDiscard('Open another cart')) return;
    try {
      const path = await chooseProject();
      if (!path) return;
      clearTimeout(writeTimer);
      studio = await openProject(path);
      metaDirty = false;
      activeSource = 0;
      handledPause = '';
      handledDiagnostic = '';
      screen = 'code';
      status = `Loaded ${tidyPath(studio.path)}`;
    } catch (error) {
      showToast(`Open failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  function showNew() {
    if (!confirmDiscard('Create a new cart')) return;
    overlay = 'new-cart';
  }

  async function createNew(templateId: string): Promise<boolean> {
    const path = await chooseProject('Choose an empty folder for new cart');
    if (!path) return false;
    clearTimeout(writeTimer);
    studio = await newProject(path, templateId);
    metaDirty = false;
    activeSource = 0;
    handledPause = '';
    handledDiagnostic = '';
    screen = 'code';
    status = `Created ${tidyPath(studio.path)}`;
    showToast(`Created ${studio.title}`);
    return true;
  }

  async function doRemix(exampleId: string) {
    if (!confirmDiscard('Remix this example')) return;
    const path = await chooseProject('Choose an empty folder for the remix');
    if (!path) return;
    clearTimeout(writeTimer);
    try {
      studio = await remixExample(path, exampleId);
      metaDirty = false;
      activeSource = 0;
      handledPause = '';
      handledDiagnostic = '';
      screen = 'code';
      status = `Created ${tidyPath(studio.path)}`;
      showToast(`Remixed ${studio.title}`);
    } catch (error) {
      showToast(`Remix failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async function doClose() {
    if (dirty && !window.confirm('Close cart with unsaved changes?')) return;
    clearTimeout(writeTimer);
    try { studio = await closeProject(); metaDirty = false; activeSource = 0; handledPause = ''; handledDiagnostic = ''; screen = 'welcome'; status = 'No cart open'; }
    catch (error) { showToast(String(error)); }
  }

  async function doExport() {
    try {
      const path = await chooseExportPath(studio.title);
      if (!path) return;
      clearTimeout(writeTimer);
      await Promise.all(studio.sources.filter((source) => source.dirty).map((source) => writeBuffer(source.path, source.text)));
      await exportCartridge(path);
      status = `Packed ${tidyPath(path)}`;
      showToast(`Packed ${path}`);
    } catch (error) {
      showToast(`Pack failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  function handleKeys(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const editing = target?.matches('input, textarea, [contenteditable="true"]');
    const cmd = event.metaKey || event.ctrlKey;
    const button = gameButton(event.key);

    if (!editing && !cmd && running && (overlay === null || overlay === 'focus') && button !== undefined) {
      event.preventDefault();
      if (!event.repeat) pressButton(button, true);
      return;
    }

    if (event.key === 'Escape') {
      const closingFocus = overlay === 'focus';
      overlay = null;
      releaseInputs();
      if (!closingFocus && studio.audio.sfxActive) void doAudio('sfx', studio.audio.sfxId, 'stop');
      if (!closingFocus && studio.audio.musicActive) void doAudio('music', studio.audio.musicPattern, 'stop');
      return;
    }
    if (cmd && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      overlay = overlay === 'palette' ? null : 'palette';
      return;
    }
    if (cmd && event.key.toLowerCase() === 'r') {
      event.preventDefault();
      void doTransport(running ? 'pause' : 'run');
      return;
    }
    if (cmd && event.key.toLowerCase() === 's') {
      event.preventDefault();
      void doSave();
      return;
    }
    if (cmd && event.shiftKey && event.key.toLowerCase() === 'p') {
      event.preventDefault();
      showPublish();
      return;
    }
    if (editing) return;
    // Space previews whatever the sound editors have selected.
    if (event.key === ' ' && (screen === 'sfx' || screen === 'music')) {
      event.preventDefault();
      previewSound();
      return;
    }
    const map: Record<string, Screen> = {
      F1: 'code', F2: 'sprites', F3: 'map', F4: 'sfx', F5: 'music',
      F6: 'palette', F7: 'cart', F8: 'library', F9: 'docs',
    };
    if (map[event.key]) {
      event.preventDefault();
      navigate(map[event.key]);
    }
  }

  function handleKeyUp(event: KeyboardEvent) {
    const button = gameButton(event.key);
    if (button !== undefined) pressButton(button, false);
  }

  function releaseInputs() {
    for (let button = 0; button < 6; button += 1) void setInput(button, false);
    heldButtons = [];
  }

  onMount(() => {
    let alive = true;
    let animation = 0;
    let tickTimer: ReturnType<typeof setInterval>;
    let stateTimer: ReturnType<typeof setInterval>;
    let unlistenPublish: UnlistenFn | undefined;
    let unlistenMenu: UnlistenFn | undefined;

    window.addEventListener('keydown', handleKeys);
    window.addEventListener('keyup', handleKeyUp);
    window.addEventListener('blur', releaseInputs);
    gamepad.attach();
    if (isTauri()) {
      void listen<PublishProgress>('publish:progress', (event) => { publishProgress = event.payload; }).then((fn) => { unlistenPublish = fn; });
      void listen<string>('menu-action', (event) => {
        switch (event.payload) {
          case 'new': showNew(); break;
          case 'open': void doOpen(); break;
          case 'save': void doSave(); break;
          case 'export': void doExport(); break;
          case 'close': void doClose(); break;
          case 'run_toggle': void doTransport(running ? 'pause' : 'run'); break;
          case 'palette': overlay = overlay === 'palette' ? null : 'palette'; break;
        }
      }).then((fn) => { unlistenMenu = fn; });
    }
    void listTemplates().then((items) => { if (alive && items.length) templates = items; })
      .catch((error) => { if (alive) showToast(`Templates unavailable: ${errorText(error)}`); });
    void listExamples().then((items) => { if (alive && items.length) examples = items; })
      .catch((error) => { if (alive) showToast(`Examples unavailable: ${errorText(error)}`); });
    void portSession().then((session) => { portAccount = session; });
    const linkPoll = window.setInterval(() => void pollPortLink(), 2000);
    tourDone = localStorage.getItem('caiven-studio-tour-complete') === '1';
    void bootstrap().then((initial) => {
      if (!alive) return;
      studio = initial;
      const saved = localStorage.getItem('caiven-studio-layout');
      if (saved) {
        try {
          const layout = JSON.parse(saved) as { screen?: Screen; drawerOpen?: boolean; drawerTab?: typeof drawerTab; consoleOpen?: boolean; consoleWidth?: number };
          if (layout.screen) screen = layout.screen;
          drawerOpen = Boolean(layout.drawerOpen);
          if (layout.drawerTab) drawerTab = layout.drawerTab;
          consoleOpen = layout.consoleOpen ?? true;
          if (layout.consoleWidth) consoleWidth = Math.min(900, Math.max(320, window.innerWidth - 560), Math.max(320, layout.consoleWidth));
        } catch { /* keep defaults */ }
      }
      if (initial.sources.length === 0) screen = 'welcome';
      else if (!localStorage.getItem('caiven-studio-tour-complete')) overlay = 'tour';
      status = initial.connected ? `Loaded ${tidyPath(initial.path)}` : 'Browser preview · IPC disconnected';

      let tickInFlight = false;
      tickTimer = setInterval(() => {
        if (tickInFlight) return;
        tickInFlight = true;
        void readTick().then((tick) => {
          if (!alive) return;
          applyTick(tick);
        }).catch(() => {}).finally(() => { tickInFlight = false; });
      }, 120);

      let stateInFlight = false;
      stateTimer = setInterval(() => {
        if (pendingWrites > 0 || stateInFlight) return;
        stateInFlight = true;
        void Promise.all([readMemory(0, 65536), readAssetIndex()]).then(([ram, index]) => {
          if (!alive) return;
          studio.ram = ram;
          studio.spriteSheet = ram.slice(MEMORY.sprites, MEMORY.map);
          studio.map = ram.slice(MEMORY.map, MEMORY.palette);
          studio.collision = ram.slice(MEMORY.collision, MEMORY.collision + COLLISION_LEN);
          studio.sfx = ram.slice(MEMORY.sfx, MEMORY.music);
          studio.music = ram.slice(MEMORY.music, MEMORY.music + 256);
          studio.assetIndex = index;
        }).catch(() => {}).finally(() => { stateInFlight = false; });
      }, 1000);

      const pullFrame = async () => {
        if (!alive) return;
        // frameData feeds the live console preview (code/sprites/map/palette/sfx/music
        // screens) and the cart screen's cover-art preview — polling it off those screens
        // wastes a 128x128 RGBA IPC round-trip every animation frame and, in the real Tauri
        // app, competes with MapCanvas/SpriteCanvas's own rAF-driven redraw for the main
        // thread, causing visible lag while drawing.
        if (consoleRelevant || screen === 'cart') {
          try {
            const next = await readFrame();
            if (next) frameData = next;
          } catch { /* transient IPC hiccup — keep polling */ }
        }
        // Same gate as the keyboard handler: only steer the cart while it's actually
        // running and no blocking overlay (other than focus mode) is up.
        if (running && (overlay === null || overlay === 'focus')) gamepad.poll();
        animation = requestAnimationFrame(pullFrame);
      };
      animation = requestAnimationFrame(pullFrame);
    }).catch((error) => {
      status = `Startup failed: ${error instanceof Error ? error.message : String(error)}`;
    });

    return () => {
      clearInterval(linkPoll);
      alive = false;
      clearInterval(tickTimer);
      clearInterval(stateTimer);
      cancelAnimationFrame(animation);
      clearTimeout(writeTimer);
      window.removeEventListener('keydown', handleKeys);
      window.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('blur', releaseInputs);
      gamepad.detach();
      unlistenPublish?.();
      unlistenMenu?.();
    };
  });
</script>

<Tooltip.Provider delayDuration={300}>
<div class="studio-app" class:drawer-open={drawerOpen}>
  <Header
    title={studio.title}
    path={studio.path}
    {dirty}
    runState={studio.runState}
    frame={studio.frame}
    fps={studio.fps}
    onTransport={doTransport}
    onPalette={() => overlay = 'palette'}
    onSave={doSave}
    onPublish={showPublish}
    onHome={() => navigate('welcome')}
  />
  <div class="studio-body">
    <ModeRail {screen} onNavigate={navigate} onTour={() => overlay = 'tour'} />
    <div class="studio-right">
      <div class="studio-main" style={consoleOpen && consoleRelevant ? `--studio-console:${consoleWidth}px` : undefined}>
        <Workspace
          bind:this={workspaceRef}
          {screen}
          sources={studio.sources}
          {activeSource}
          palette={studio.palette}
          spriteSheet={studio.spriteSheet}
          map={studio.map}
          spriteBanks={studio.spriteBanks}
          mapBanks={studio.mapBanks}
          activeSpriteBank={studio.activeSpriteBank}
          activeMapBank={studio.activeMapBank}
          collision={studio.collision}
          collisionTypes={studio.collisionTypes}
          sfx={studio.sfx}
          music={studio.music}
          paletteBanks={studio.paletteBanks}
          sfxBanks={studio.sfxBanks}
          musicBanks={studio.musicBanks}
          activePaletteBank={studio.activePaletteBank}
          activeSfxBank={studio.activeSfxBank}
          activeMusicBank={studio.activeMusicBank}
          cartSize={studio.cartSize}
          audio={studio.audio}
          assetIndex={studio.assetIndex}
          diagnostics={studio.diagnostics}
          breakpoints={studio.breakpoints}
          title={studio.title}
          author={studio.author}
          path={studio.path}
          meta={studio.meta}
          {dirty}
          {tourDone}
          recent={studio.recent}
          {examples}
          api={studio.api}
          {frameData}
          {insertRequest}
          {revealRequest}
          onInsertHandled={(id) => { if (insertRequest?.id === id) insertRequest = null; }}
          onRevealHandled={(id) => { if (revealRequest?.id === id) revealRequest = null; }}
          onNavigate={navigate}
          onSource={(index) => activeSource = index}
          onCode={updateCode}
          onSprite={updateSprite}
          onCollision={updateCollision}
          onCollisionTypes={updateCollisionTypes}
          onMap={updateMap}
          onAssetBank={changeAssetBank}
          onSfx={updateSfx}
          onMusic={updateMusic}
          {soundSelection}
          onAudio={(kind, id, action) => void doAudio(kind, id, action)}
          onBreakpoint={(source, line) => void doBreakpoint(source, line)}
          onMeta={(title, author, meta) => void doMeta(title, author, meta)}
          onCreateModule={() => overlay = 'module'}
          onPalette={updatePalette}
          onTour={() => overlay = 'tour'}
          onOpen={doOpen}
          onNew={showNew}
          onRemix={(exampleId) => void doRemix(exampleId)}
          {localCarts}
          {portCarts}
          {portAccount}
          {portBusy}
          {portError}
          portLinkPending={portLink !== null}
          portLinkExpiresAt={portLink?.expiresAt ?? ''}
          onScanLibrary={() => void scanLocal()}
          onSearchPort={(query) => void searchPort(query)}
          onOpenLocal={(path) => void openPath(path)}
          onRemoveRecent={(path) => void doRemoveRecent(path)}
          onDownloadPort={(cart) => void downloadPort(cart)}
          onOpenPortAccount={openPortAccount}
          onPortLink={() => void linkPort()}
          onPortLinkCancel={() => void cancelPortLink()}
          onPortLogout={() => void logoutPort()}
          onSetServerUrl={(url) => void setServerUrl(url)}
          onInsertBuiltin={insertBuiltin}
          onOpenSource={jumpToSource}
          onHistoryStatus={(status) => historyStatus = status}
        />
        {#if consoleRelevant && consoleOpen}
          <div
            class="pane-resizer"
            class:dragging={resizing}
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize console"
            onpointerdown={startResize}
          ></div>
          <ConsolePane
            runState={studio.runState}
            frame={studio.frame}
            fps={studio.fps}
            {frameTime}
            {frameData}
            onFocus={() => overlay = 'focus'}
            held={heldButtons}
            onInput={pressButton}
            globals={studio.globals}
            watches={studio.watches}
            callStack={studio.callStack}
            breakpointCount={studio.breakpoints.length}
            diagnostics={studio.diagnostics}
            pauseReason={studio.pauseReason}
            onJumpToError={jumpToDiagnostic}
            onJumpToLocation={jumpToSource}
            onAddWatch={doAddWatch}
            onRemoveWatch={(expression) => void doRemoveWatch(expression)}
            onClose={() => consoleOpen = false}
          />
        {:else if consoleRelevant}
          <Button variant="ghost" class="console-reopen" title="Show console" onclick={() => consoleOpen = true}>
            <PanelRightOpen size={14} /><span>Console</span>
          </Button>
        {/if}
      </div>
      <Drawer
        open={drawerOpen}
        tab={drawerTab}
        {status}
        diagnostics={allDiagnostics}
        output={studio.output}
        ram={studio.ram}
        onJump={jumpToDiagnostic}
        onClearOutput={() => void doClearOutput()}
        onToggle={() => drawerOpen = !drawerOpen}
        onTab={(tab) => { drawerTab = tab; drawerOpen = true; }}
      />
    </div>
  </div>

  <Overlays
    {overlay}
    {running}
    palette={studio.palette}
    onClose={() => { const closingFocus = overlay === 'focus'; overlay = null; if (closingFocus) releaseInputs(); }}
    onNavigate={navigate}
    onRun={() => void doTransport(running ? 'pause' : 'run')}
    onExport={doExport}
    onPublish={showPublish}
    title={studio.title}
    author={studio.author}
    meta={studio.meta}
    portAccount={portAccount}
    {publishProgress}
    {publishError}
    {publishDone}
    onStartPublish={(changelog) => void doPublish(changelog)}
    onLinkPort={openPortAccount}
    onTourDone={() => { localStorage.setItem('caiven-studio-tour-complete', '1'); tourDone = true; }}
    onOpenProject={() => void doOpen()}
    onNewProject={showNew}
    onCloseProject={() => void doClose()}
    {templates}
    onCreateProject={createNew}
    {frameData}
    api={studio.api}
    onInsertBuiltin={insertBuiltin}
    onCreateModule={doCreateModule}
    canUndo={historyStatus.canUndo}
    canRedo={historyStatus.canRedo}
    onUndo={() => workspaceRef?.undoActive()}
    onRedo={() => workspaceRef?.redoActive()}
    {keymap}
    buttonLabels={BUTTON_LABELS}
    onRebindButton={rebindButton}
    onResetKeymap={resetKeymap}
    onOpenControls={() => overlay = 'controls'}
  />

  <Toaster position="bottom-right" richColors />
  {#if !studio.connected}
    <div class="preview-badge" title="Open through Tauri for live VM and filesystem access"><WifiOff size={12} />Preview</div>
  {/if}
</div>
</Tooltip.Provider>
