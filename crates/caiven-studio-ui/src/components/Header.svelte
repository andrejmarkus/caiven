<script lang="ts">
  import {
    Play, Pause, RotateCcw, StepForward, Search, Save, Upload, Check,
  } from '@lucide/svelte';
  import type { RunState } from '../types';
  import { tidyPath } from '../lib/format';

  interface Props {
    title: string;
    path: string;
    dirty: boolean;
    runState: RunState;
    frame: number;
    fps: number;
    onTransport: (action: 'run' | 'pause' | 'reset' | 'step') => void;
    onPalette: () => void;
    onSave: () => void;
    onPublish: () => void;
    onHome: () => void;
  }

  let {
    title, path, dirty, runState, frame, fps,
    onTransport, onPalette, onSave, onPublish, onHome,
  }: Props = $props();

  const running = $derived(runState === 'running');
</script>

<header class="studio-header">
  <button class="brand-block" title="Caiven Studio home" onclick={onHome}>
    <span class="brand-mark"><span></span><i></i></span>
    <span class="brand-type">
      <strong>Caiven</strong>
      <small>Studio</small>
    </span>
  </button>

  <div class="cart-identity">
    <strong>{title || 'No cart open'}</strong>
    {#if dirty}<span class="dirty-dot" title="Unsaved changes"></span>{/if}
    <code title={path}>{path ? tidyPath(path, 3) : 'Start one from a template'}</code>
  </div>

  <div class="transport" data-tour-target="run">
    <button class="btn primary run-button" onclick={() => onTransport(running ? 'pause' : 'run')}>
      {#if running}<Pause size={15} />{:else}<Play size={15} fill="currentColor" />{/if}
      <span>{running ? 'Pause' : 'Run'}</span>
      <kbd>⌘R</kbd>
    </button>
    <button class="icon-btn" title="Reset" onclick={() => onTransport('reset')}>
      <RotateCcw size={16} />
    </button>
    <button class="icon-btn" title="Step one frame" disabled={running} onclick={() => onTransport('step')}>
      <StepForward size={16} />
    </button>
    <div class="state-pill">
      <i class:running></i>
      <span>{running ? 'Running' : runState === 'paused' ? 'Paused' : 'Stopped'}</span>
      <b>·</b>
      <code>{running ? `${Math.round(fps)} fps` : `frame ${frame.toLocaleString()}`}</code>
    </div>
  </div>

  <span class="header-divider"></span>

  <button class="command-field" onclick={onPalette}>
    <Search size={15} />
    <span>Search or run a command</span>
    <kbd>⌘K</kbd>
  </button>
  <button class="btn subtle" onclick={onSave}><Save size={15} />Save</button>
  <button class="btn secondary" onclick={onPublish}><Upload size={15} />Publish</button>
</header>
