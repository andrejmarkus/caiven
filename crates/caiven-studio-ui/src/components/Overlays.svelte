<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import {
    Search, Play, Upload, Package, Image, Layers, ChevronsLeftRight,
    Check, LoaderCircle, X, Pause, Minimize2, Sparkles, FilePlus2,
    FileCode2, FolderPlus, Gamepad2, Grid3X3, Trophy,
  } from '@lucide/svelte';
  import type { ApiEntry, CartMeta, CartTemplateSummary, PortSession, PublishProgress, Screen } from '../types';
  import { TOUR_STEPS, moveTourStep } from '../lib/tour';

  interface Props {
    overlay: 'palette' | 'publish' | 'tour' | 'focus' | 'module' | 'new-cart' | null;
    running: boolean;
    palette: string[];
    onClose: () => void;
    onNavigate: (screen: Screen) => void;
    onRun: () => void;
    onExport: () => void;
    onPublish: () => void;
    title: string;
    author: string;
    meta: CartMeta;
    portAccount: PortSession;
    publishProgress: PublishProgress | null;
    publishError: string;
    publishDone: string;
    onStartPublish: (changelog: string) => void;
    onTourDone: () => void;
    onOpenProject: () => void;
    onNewProject: () => void;
    onCloseProject: () => void;
    templates: CartTemplateSummary[];
    onCreateProject: (templateId: string) => Promise<boolean>;
    frameData: Uint8Array | null;
    api: ApiEntry[];
    onInsertBuiltin: (name: string) => void;
    onCreateModule: (name: string) => Promise<string | null>;
  }

  let { overlay, running, palette, onClose, onNavigate, onRun, onExport, onPublish,
    title, author, meta, portAccount, publishProgress, publishError, publishDone,
    onStartPublish, onTourDone, onOpenProject, onNewProject, onCloseProject,
    templates, onCreateProject, frameData, api, onInsertBuiltin, onCreateModule }: Props = $props();
  let query = $state('');
  let changelog = $state('');
  let tourStep = $state(0);
  let tourWasOpen = $state(false);
  let tourLayout = $state('');
  let focusCanvas = $state<HTMLCanvasElement>();
  let commandInput = $state<HTMLInputElement>();
  let moduleInput = $state<HTMLInputElement>();
  let selectedCommand = $state(0);
  let moduleName = $state('module.lua');
  let moduleError = $state('');
  let moduleBusy = $state(false);
  let moduleWasOpen = $state(false);
  let paletteWasOpen = $state(false);
  let selectedTemplate = $state('top-down-mover');
  let newCartError = $state('');
  let newCartBusy = $state(false);
  let newCartWasOpen = $state(false);
  const activeTemplate = $derived(templates.find((template) => template.id === selectedTemplate));
  const tilePreview = [
    1,1,1,1,1,1,1,1,
    1,0,0,0,1,0,0,1,
    1,0,1,0,1,0,1,1,
    1,0,1,0,0,0,0,1,
    1,0,1,1,1,1,0,1,
    1,0,0,0,0,1,0,1,
    1,1,1,1,0,0,0,1,
    1,1,1,1,1,1,1,1,
  ];
  const publishSteps = ['pack', 'cover', 'upload', 'notify'] as const;
  const currentStep = $derived(publishProgress ? publishSteps.indexOf(publishProgress.step) : -1);

  const commands = $derived([
    { group: 'Suggested', name: running ? 'Pause cart' : 'Run cart', detail: 'compile and start', keys: '⌘R', icon: Play, action: onRun },
    { group: 'Suggested', name: 'Publish to port', detail: 'new version', keys: '⇧⌘P', icon: Upload, action: onPublish },
    { group: 'Suggested', name: 'Pack cartridge (.cav)', detail: 'distribution build', keys: '', icon: Package, action: onExport },
    { group: 'Suggested', name: 'Open project', detail: 'folder or cart', keys: '', icon: Package, action: onOpenProject },
    { group: 'Suggested', name: 'New cart', detail: 'choose a starting template', keys: '', icon: Sparkles, action: onNewProject },
    { group: 'Suggested', name: 'Close cart', detail: title, keys: '', icon: X, action: onCloseProject },
    { group: 'Go to', name: 'Sprites', detail: '', keys: 'F2', icon: Image, screen: 'sprites' as Screen },
    { group: 'Go to', name: 'Map', detail: '', keys: 'F3', icon: Layers, screen: 'map' as Screen },
    { group: 'Go to', name: 'main.lua', detail: title, keys: '', icon: ChevronsLeftRight, screen: 'code' as Screen },
    ...api.slice(0, 12).map((entry) => ({ group: 'Insert a builtin', name: entry.name, detail: entry.params.map((param) => param.name).join(', '), keys: '', icon: ChevronsLeftRight, action: () => onInsertBuiltin(entry.name) })),
  ].filter((command) => `${command.name} ${command.detail}`.toLowerCase().includes(query.toLowerCase())));

  function activate(command: typeof commands[number]) {
    onClose();
    if (command.screen) onNavigate(command.screen);
    command.action?.();
  }

  function commandKey(event: KeyboardEvent) {
    if (!commands.length) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const direction = event.key === 'ArrowDown' ? 1 : -1;
      selectedCommand = (selectedCommand + direction + commands.length) % commands.length;
      requestAnimationFrame(() => document.querySelector('.command-results button.selected')?.scrollIntoView({ block: 'nearest' }));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      activate(commands[selectedCommand]);
    }
  }

  async function submitModule() {
    const name = moduleName.trim();
    if (!name || moduleBusy) return;
    moduleBusy = true;
    moduleError = await onCreateModule(name) ?? '';
    moduleBusy = false;
    if (moduleError) moduleInput?.focus();
  }

  async function submitNewProject() {
    if (!selectedTemplate || newCartBusy) return;
    newCartBusy = true;
    newCartError = '';
    try {
      if (await onCreateProject(selectedTemplate)) onClose();
    } catch (error) {
      newCartError = error instanceof Error ? error.message : String(error);
    } finally {
      newCartBusy = false;
    }
  }

  function updateTourLayout() {
    if (overlay !== 'tour') return;
    const target = document.querySelector<HTMLElement>(`[data-tour-target="${TOUR_STEPS[tourStep].id}"]`);
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const pad = 8;
    const fallback = {
      left: Math.max(12, viewportWidth * 0.12),
      top: 76,
      right: Math.min(viewportWidth - 12, viewportWidth * 0.82),
      bottom: Math.max(220, viewportHeight - 54),
    };
    const rect = target?.getBoundingClientRect() ?? fallback;
    const left = Math.max(8, rect.left - pad);
    const top = Math.max(8, rect.top - pad);
    const right = Math.min(viewportWidth - 8, rect.right + pad);
    const bottom = Math.min(viewportHeight - 8, rect.bottom + pad);
    const popoverWidth = Math.min(390, viewportWidth - 24);
    const popoverHeight = 350;
    const gap = 18;
    let popoverLeft: number;
    if (viewportWidth - right >= popoverWidth + gap) popoverLeft = right + gap;
    else if (left >= popoverWidth + gap) popoverLeft = left - popoverWidth - gap;
    else popoverLeft = Math.min(viewportWidth - popoverWidth - 12, Math.max(12, left + (right - left - popoverWidth) / 2));
    const popoverTop = Math.min(viewportHeight - popoverHeight - 12, Math.max(12, top));
    tourLayout = [
      `--tour-left:${left}px`, `--tour-top:${top}px`,
      `--tour-width:${Math.max(1, right - left)}px`, `--tour-height:${Math.max(1, bottom - top)}px`,
      `--tour-popover-left:${popoverLeft}px`, `--tour-popover-top:${Math.max(12, popoverTop)}px`,
    ].join(';');
  }

  function scheduleTourLayout() {
    requestAnimationFrame(() => requestAnimationFrame(updateTourLayout));
  }

  function showTourStep(index: number) {
    const next = moveTourStep(index, 0);
    tourStep = next.index;
    onNavigate(next.screen);
    scheduleTourLayout();
  }

  function changeTourStep(delta: -1 | 1) {
    const next = moveTourStep(tourStep, delta);
    tourStep = next.index;
    onNavigate(next.screen);
    scheduleTourLayout();
  }

  onMount(() => {
    window.addEventListener('resize', updateTourLayout);
    return () => window.removeEventListener('resize', updateTourLayout);
  });

  $effect(() => {
    if (overlay === 'tour' && !tourWasOpen) {
      tourStep = 0;
      onNavigate(TOUR_STEPS[0].screen);
      scheduleTourLayout();
    }
    if (overlay === 'tour') {
      tourStep;
      scheduleTourLayout();
    }
    tourWasOpen = overlay === 'tour';
  });

  $effect(() => {
    overlay; query;
    if (overlay === 'palette') {
      if (!paletteWasOpen) {
        query = '';
        selectedCommand = 0;
      }
      selectedCommand = Math.min(selectedCommand, Math.max(0, commands.length - 1));
      requestAnimationFrame(() => commandInput?.focus());
    }
    paletteWasOpen = overlay === 'palette';
  });

  $effect(() => {
    if (overlay === 'module' && !moduleWasOpen) {
      moduleName = 'module.lua';
      moduleError = '';
      moduleBusy = false;
      requestAnimationFrame(() => { moduleInput?.focus(); moduleInput?.select(); });
    }
    moduleWasOpen = overlay === 'module';
  });

  $effect(() => {
    if (overlay === 'new-cart' && (!newCartWasOpen || !templates.some((template) => template.id === selectedTemplate))) {
      selectedTemplate = templates.find((template) => template.id === 'top-down-mover')?.id ?? templates[0]?.id ?? '';
      newCartError = '';
      newCartBusy = false;
    }
    newCartWasOpen = overlay === 'new-cart';
  });

  $effect(() => {
    frameData;
    if (!focusCanvas) return;
    const context = focusCanvas.getContext('2d'); if (!context) return;
    if (frameData?.length === 128 * 128 * 4) context.putImageData(new ImageData(new Uint8ClampedArray(frameData), 128, 128), 0, 0);
    else { context.fillStyle = '#000'; context.fillRect(0, 0, 128, 128); }
  });
</script>

{#if overlay}
  <div
    class="overlay-backdrop"
    class:tour-overlay={overlay === 'tour'}
    role="presentation"
    transition:fade={{ duration: 120 }}
    onclick={(event) => { if (event.currentTarget === event.target) onClose(); }}
    onkeydown={(event) => { if (event.key === 'Escape') onClose(); }}
  >
    {#if overlay === 'palette'}
      <div class="command-palette" role="dialog" aria-label="Command palette" tabindex="-1" transition:fly={{ y: -8, duration: 180 }} onkeydown={commandKey}>
        <div class="command-input"><Search size={18} /><input bind:this={commandInput} bind:value={query} placeholder="Search or run a command" /><kbd>esc</kbd></div>
        <div class="command-results">
          {#each ['Suggested','Go to','Insert a builtin'] as group}
            {@const items = commands.filter((command) => command.group === group)}
            {#if items.length}
              <span class="eyebrow">{group}</span>
              {#each items as command}
                {@const Icon = command.icon}
                <button class:selected={commands[selectedCommand] === command} onmouseenter={() => selectedCommand = commands.indexOf(command)} onclick={() => activate(command)}>
                  <i><Icon size={15} /></i><span><strong>{command.name}</strong><small>{command.detail}</small></span>{#if command.keys}<kbd>{command.keys}</kbd>{/if}
                </button>
              {/each}
            {/if}
          {/each}
        </div>
        <footer><span><kbd>↑↓</kbd> navigate</span><span><kbd>↵</kbd> select</span><span><kbd>esc</kbd> close</span></footer>
      </div>
    {:else if overlay === 'new-cart'}
      <form class="new-cart-dialog" aria-label="New cartridge from template" transition:fly={{ y: 8, duration: 180 }} onsubmit={(event) => { event.preventDefault(); void submitNewProject(); }}>
        <button type="button" class="dialog-close" aria-label="Close" onclick={onClose}><X size={17} /></button>
        <header>
          <span class="eyebrow">New cartridge</span>
          <h2>Pick a world to start from.</h2>
          <p>Studio writes selected Lua starter into <code>main.lua</code>. Everything stays editable.</p>
        </header>
        <div class="template-grid">
          {#each templates as template}
            <button
              type="button"
              class="template-card"
              class:selected={template.id === selectedTemplate}
              aria-pressed={template.id === selectedTemplate}
              onclick={() => { selectedTemplate = template.id; newCartError = ''; }}
              ondblclick={() => { selectedTemplate = template.id; void submitNewProject(); }}
            >
              <span class={`template-preview preview-${template.id}`} aria-hidden="true">
                {#if template.id === 'top-down-mover'}
                  <i class="preview-grid-lines"></i><i class="mover-trail"></i><i class="mover-player"><b></b><b></b></i>
                {:else if template.id === 'tap-to-score'}
                  <strong class="score-readout">07</strong><i class="score-ball"></i><i class="score-shadow"></i>
                {:else if template.id === 'tile-world'}
                  <span class="tile-maze">{#each tilePreview as wall}<i class:wall={wall === 1}></i>{/each}</span><i class="tile-player"></i>
                {:else}
                  <code><b>function</b> _init()<br />&nbsp;&nbsp;-- yours<br /><b>end</b><br /><br /><b>function</b> _update()</code>
                {/if}
                <span class="preview-badge-icon">
                  {#if template.id === 'top-down-mover'}<Gamepad2 size={13} />{:else if template.id === 'tap-to-score'}<Trophy size={13} />{:else if template.id === 'tile-world'}<Grid3X3 size={13} />{:else}<FileCode2 size={13} />{/if}
                </span>
              </span>
              <span class="template-copy"><strong>{template.name}</strong><small>{template.description}</small></span>
              <i class="template-check">{#if template.id === selectedTemplate}<Check size={12} />{/if}</i>
            </button>
          {/each}
        </div>
        {#if newCartError}<div class="new-cart-error" role="alert">{newCartError}</div>{/if}
        <footer>
          <span>{#if activeTemplate}<strong>{activeTemplate.name}</strong><small>Folder picker opens next.</small>{/if}</span>
          <div><button type="button" class="btn secondary" onclick={onClose}>Cancel</button><button class="btn primary" disabled={newCartBusy || !selectedTemplate}>{#if newCartBusy}<LoaderCircle class="spin" size={15} />Creating…{:else}<FolderPlus size={15} />Choose folder{/if}</button></div>
        </footer>
      </form>
    {:else if overlay === 'module'}
      <form class="module-dialog" transition:fly={{ y: 8, duration: 180 }} onsubmit={(event) => { event.preventDefault(); void submitModule(); }}>
        <button type="button" class="dialog-close" onclick={onClose}><X size={17} /></button>
        <span class="eyebrow">Project source</span>
        <h2>New Lua module</h2>
        <p>Use nested paths for folders, for example <code>ui/hud.lua</code>.</p>
        <label>
          Module path
          <span><FilePlus2 size={15} /><input bind:this={moduleInput} bind:value={moduleName} aria-invalid={Boolean(moduleError)} autocomplete="off" spellcheck="false" /></span>
        </label>
        {#if moduleError}<div class="form-error" role="alert">{moduleError}</div>{/if}
        <footer><button type="button" class="btn secondary" onclick={onClose}>Cancel</button><button class="btn primary" disabled={moduleBusy || !moduleName.trim()}>{moduleBusy ? 'Creating…' : 'Create module'}</button></footer>
      </form>
    {:else if overlay === 'publish'}
      <section class="publish-dialog" transition:fly={{ y: 8, duration: 180 }}>
        <button class="dialog-close" onclick={onClose}><X size={17} /></button>
        <span class="eyebrow">Publish {title || 'cart'}</span>
        <h2>{publishDone ? 'Cart shipped' : publishProgress ? 'Publishing to port' : 'Ship a new release'}</h2>
        <p>{publishError || publishDone || publishProgress?.note || (portAccount.authenticated ? `Signed in as ${portAccount.username}` : 'Log in from Library → Port before publishing.')}</p>
        <div class="publish-cover">
          <div>{#each Array(64) as _,p}<i style={`background:${palette[(p * 7 + 3) % 16]}`}></i>{/each}</div>
          <span><strong>{title}</strong><small>by {author}</small><code>{meta.tags.join(' · ') || 'untagged'}</code></span>
        </div>
        {#if !publishProgress && !publishDone}<label class="publish-changelog">Changelog<input bind:value={changelog} placeholder="What changed?" /></label>{/if}
        <div class="publish-progress">
          <span style={`width:${publishProgress?.pct ?? (publishDone ? 100 : 0)}%`}></span>
        </div>
        <div class="publish-steps">
          {#each [['Pack cartridge','live buffers'],['Capture cover','30 frames'],['Upload to port','cartridge + PNG'],['Notify followers','server-side']] as row, index}
            <div class:done={Boolean(publishDone) || currentStep > index} class:busy={!publishDone && currentStep === index}>
              <i>{#if publishDone || currentStep > index}<Check size={13} />{:else if currentStep === index}<LoaderCircle size={13} />{:else}{index + 1}{/if}</i><strong>{row[0]}</strong><code>{row[1]}</code>
            </div>
          {/each}
        </div>
        <footer><button class="btn secondary" onclick={onClose}>{publishProgress && !publishDone ? 'Keep working' : 'Close'}</button>{#if !publishProgress && !publishDone}<button class="btn primary" disabled={!portAccount.authenticated} onclick={() => onStartPublish(changelog)}>Publish</button>{/if}</footer>
      </section>
    {:else if overlay === 'tour'}
      <div class="tour-layer" style={tourLayout}>
        <div class="tour-spotlight"></div>
        <div class="tour-popover" role="dialog" aria-live="polite" aria-label={`Tutorial step ${tourStep + 1}`} tabindex="-1" transition:fly={{ y: 8, duration: 180 }}>
          <nav class="tour-progress" aria-label="Tutorial steps">
            {#each TOUR_STEPS as step, index}
              <button class:active={index === tourStep} class:done={index < tourStep} aria-current={index === tourStep ? 'step' : undefined} onclick={() => showTourStep(index)}><i>{index < tourStep ? '✓' : index + 1}</i><span>{step.eyebrow}</span></button>
            {/each}
          </nav>
          {#key tourStep}
            <div class="tour-step-content" transition:fade={{ duration: 100 }}>
              <span class="eyebrow">Step {tourStep + 1} of {TOUR_STEPS.length} · {TOUR_STEPS[tourStep].eyebrow}</span>
              <h2>{TOUR_STEPS[tourStep].title}</h2>
              <p>{TOUR_STEPS[tourStep].copy}</p>
              {#if TOUR_STEPS[tourStep].visual === 'code'}
                <div class="tour-visual tour-code"><code><b>function</b> _update()<br />&nbsp;&nbsp;<em>clear_screen</em>()<br />&nbsp;&nbsp;sprite(0, player.x, player.y)<br /><b>end</b></code></div>
              {:else if TOUR_STEPS[tourStep].visual === 'transport'}
                <div class="tour-visual tour-transport"><button><Play size={14} fill="currentColor" />Run</button><button><Pause size={14} />Pause</button><span><i></i><strong>Running</strong><code>60 fps</code></span></div>
              {:else if TOUR_STEPS[tourStep].visual === 'sprite'}
                <div class="tour-visual tour-sprite"><div>{#each tilePreview as pixel}<i style={`background:${palette[pixel ? 8 : 0] ?? (pixel ? '#FF004D' : '#000')}`}></i>{/each}</div><span><Grid3X3 size={15} /><strong>Sprite 000</strong><code>8 × 8 · 64 bytes</code></span></div>
              {:else}
                <div class="tour-visual tour-publish"><span><Package size={15} /><strong>Pack .cav</strong><small>Portable cartridge</small></span><span><Upload size={15} /><strong>Publish to port</strong><small>Versioned release</small></span><span><Check size={15} /><strong>Ready</strong><small>Live buffers included</small></span></div>
              {/if}
            </div>
          {/key}
          <footer><button onclick={() => { onTourDone(); onClose(); }}>Skip tour</button><span>{#if tourStep > 0}<button class="btn secondary" onclick={() => changeTourStep(-1)}>Back</button>{/if}<button class="btn primary" onclick={() => { if (tourStep === TOUR_STEPS.length - 1) { onTourDone(); onClose(); } else changeTourStep(1); }}>{tourStep === TOUR_STEPS.length - 1 ? 'Start building' : `Next: ${TOUR_STEPS[tourStep + 1].eyebrow.toLowerCase()}`}</button></span></footer>
        </div>
      </div>
    {:else}
      <section class="focus-mode" transition:fade={{ duration: 180 }}>
        <button class="focus-exit" onclick={onClose}><Minimize2 size={16} />Exit focus <kbd>esc</kbd></button>
        <div class="focus-screen">
          <canvas class="focus-pixels" bind:this={focusCanvas} width="128" height="128" aria-label="Cart framebuffer"></canvas>
          <div class="scanline-overlay"></div><div class="crt-vignette"></div>
        </div>
        <div class="focus-controls"><button class="btn primary" onclick={onRun}>{#if running}<Pause size={15} />Pause{:else}<Play size={15} />Run{/if}</button><span>WASD move · J/K buttons</span></div>
      </section>
    {/if}
  </div>
{/if}
