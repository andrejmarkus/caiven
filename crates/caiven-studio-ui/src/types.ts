export type Screen =
  | 'welcome'
  | 'code'
  | 'sprites'
  | 'map'
  | 'palette'
  | 'sfx'
  | 'music'
  | 'assets'
  | 'cart'
  | 'library'
  | 'account'
  | 'docs';

export type RunState = 'running' | 'paused' | 'stopped';

export interface Breakpoint { source: string; line: number; }
export interface PauseReason {
  kind: 'manual' | 'breakpoint' | 'error';
  source: string | null;
  line: number | null;
  message: string | null;
}

export interface SourceBuffer {
  path: string;
  name: string;
  text: string;
  dirty: boolean;
}

export interface EditorInsertRequest { id: number; source: string; text: string; }
export interface EditorRevealRequest { id: number; source: string; line: number; column: number; }

export interface Diagnostic {
  severity: 'error' | 'info' | 'success';
  title: string;
  detail: string;
  path: string;
  line: number | null;
}

export interface GlobalValue { name: string; value: string; nodeId?: string | null; }
export interface DebugChild { key: string; value: string; nodeId?: string | null; }
export interface CallFrame { label: string; location: string; }
export interface CartMeta { description: string; tags: string[]; }
export interface CartSize { packedBytes: number; maxBytes: number; }
export interface AssetBankState { kind: 'sprites' | 'map' | 'palette' | 'sfx' | 'music'; ids: number[]; active: number; data: number[]; }
export interface AudioState {
  sfxActive: boolean; sfxId: number; sfxStep: number;
  musicActive: boolean; musicPattern: number; musicRow: number; musicLoop: boolean;
}
export interface AssetRef { path: string; line: number; col: number; label: string; }
export interface AssetEntry {
  kind: 'sprite' | 'sfx' | 'music' | 'color';
  id: number; used: boolean; nonzero: boolean; bytes: number; refs: AssetRef[];
}
export interface AssetIndex { entries: AssetEntry[]; computedRefs: number; }
export interface PortSession { authenticated: boolean; username: string; portUrl: string; }
export interface PortCart {
  id: string; title: string; author: string; description: string; tags: string[];
  downloads: number; owner: string | null; ratingAvg: number; ratingCount: number;
  latestVersion: number; cartSize: number; hasScreenshot: boolean; screenshotUrl: string;
}
export interface PortCartList { carts: PortCart[]; total: number; page: number; perPage: number; portUrl: string; }
export interface LocalCart { path: string; name: string; title: string; author: string; modified: number; project: boolean; }
export interface PublishProgress { step: 'pack' | 'cover' | 'upload' | 'notify'; pct: number; note: string; }
export interface PublishResult { cartId: string; version: number | null; }

export interface CartTemplateSummary {
  id: string;
  name: string;
  description: string;
}

export interface ExampleSummary {
  id: string;
  name: string;
  description: string;
}

export interface ApiEntry {
  name: string;
  params: { name: string; ty: string }[];
  returns: string;
  doc: string;
  category: string;
}

export interface PreludeModule {
  name: string;
  globals: string[];
  enabled: boolean;
}

export type CollisionShape = 'none' | 'solid' | 'one_way' | 'slope_left' | 'slope_right';

export interface CollisionType {
  id: number;
  name: string;
  color: [number, number, number];
  shape: CollisionShape;
}

export interface StudioBootstrap {
  connected: boolean;
  title: string;
  path: string;
  author: string;
  runState: RunState;
  frame: number;
  fps: number;
  cartSize: CartSize;
  sources: SourceBuffer[];
  palette: string[];
  spriteSheet: number[];
  map: number[];
  spriteBanks: number[];
  mapBanks: number[];
  activeSpriteBank: number;
  activeMapBank: number;
  collision: number[];
  collisionTypes: CollisionType[];
  sfx: number[];
  music: number[];
  paletteBanks: number[];
  activePaletteBank: number;
  sfxBanks: number[];
  activeSfxBank: number;
  musicBanks: number[];
  activeMusicBank: number;
  ram: number[];
  globals: GlobalValue[];
  watches: GlobalValue[];
  callStack: CallFrame[];
  locals: GlobalValue[];
  breakpoints: Breakpoint[];
  pauseReason: PauseReason | null;
  diagnostics: Diagnostic[];
  output: string[];
  meta: CartMeta;
  assetIndex: AssetIndex;
  audio: AudioState;
  recent: string[];
  api: ApiEntry[];
  preludeModules: PreludeModule[];
}

export interface TickSnapshot {
  runState: RunState;
  frame: number;
  fps: number;
  frameTimeMs: number;
  globals: GlobalValue[];
  watches: GlobalValue[];
  callStack: CallFrame[];
  locals: GlobalValue[];
  pauseReason: PauseReason | null;
  audio: AudioState;
  diagnostics: Diagnostic[];
  output: string[];
  activeSpriteBank: number;
  activeMapBank: number;
  activePaletteBank: number;
  activeSfxBank: number;
  activeMusicBank: number;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
    };
  }
}
