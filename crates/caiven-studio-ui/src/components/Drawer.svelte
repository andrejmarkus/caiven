<script lang="ts">
  import {
    Check, ChevronDown, ChevronLeft, ChevronRight, ChevronUp, Cpu, Eraser,
    Info, SquareTerminal, TriangleAlert,
  } from '@lucide/svelte';
  import type { Diagnostic } from '../types';
  import {
    MEMORY_PAGE_SIZE, MEMORY_REGIONS, clampMemoryBase, formatMemoryAddress,
    formatMemoryRows, parseMemoryAddress,
  } from '../lib/drawerMath';

  interface Props {
    open: boolean;
    tab: 'problems' | 'output' | 'memory';
    status: string;
    diagnostics: Diagnostic[];
    output: string[];
    ram: number[];
    onToggle: () => void;
    onTab: (tab: 'problems' | 'output' | 'memory') => void;
    onJump: (diagnostic: Diagnostic) => void;
    onClearOutput: () => void;
  }

  let {
    open, tab, status, diagnostics, output, ram,
    onToggle, onTab, onJump, onClearOutput,
  }: Props = $props();
  let memoryBase = $state(0);
  let memoryAddress = $state('0x0000');
  let memoryRegion = $state('');
  let outputViewport = $state<HTMLDivElement>();
  let followOutput = $state(true);

  const errorCount = $derived(diagnostics.filter((entry) => entry.severity === 'error').length);
  const resolvedMemoryBase = $derived(clampMemoryBase(memoryBase, ram.length));
  const memoryRows = $derived(formatMemoryRows(ram, resolvedMemoryBase));
  const memoryEnd = $derived(Math.min(Math.max(0, ram.length - 1), resolvedMemoryBase + MEMORY_PAGE_SIZE - 1));

  function goMemory(address: number) {
    memoryBase = clampMemoryBase(address, ram.length);
    memoryAddress = formatMemoryAddress(memoryBase);
  }

  function shiftMemory(direction: -1 | 1) {
    goMemory(resolvedMemoryBase + direction * MEMORY_PAGE_SIZE);
  }

  function commitMemoryAddress() {
    const parsed = parseMemoryAddress(memoryAddress, ram.length);
    if (parsed === null) {
      memoryAddress = formatMemoryAddress(resolvedMemoryBase);
      return;
    }
    goMemory(parsed);
  }

  function jumpMemoryRegion() {
    if (memoryRegion) goMemory(Number(memoryRegion));
    memoryRegion = '';
  }

  function trackOutputScroll() {
    if (!outputViewport) return;
    followOutput = outputViewport.scrollHeight - outputViewport.scrollTop - outputViewport.clientHeight < 20;
  }

  $effect(() => {
    output.length;
    output[output.length - 1];
    if (!open || tab !== 'output' || !followOutput) return;
    requestAnimationFrame(() => {
      if (outputViewport) outputViewport.scrollTop = outputViewport.scrollHeight;
    });
  });
</script>

<section class="bottom-drawer" class:open>
  <div class="drawer-tabs" role="tablist" aria-label="Studio messages and memory">
    <button role="tab" class:active={open && tab === 'problems'} aria-selected={open && tab === 'problems'} onclick={() => onTab('problems')}>
      Problems <span class:danger-badge={errorCount > 0}>{diagnostics.length}</span>
    </button>
    <button role="tab" class:active={open && tab === 'output'} aria-selected={open && tab === 'output'} onclick={() => onTab('output')}>
      Output <span>{output.length}</span>
    </button>
    <button role="tab" class:active={open && tab === 'memory'} aria-selected={open && tab === 'memory'} onclick={() => onTab('memory')}>
      Memory <span>64K</span>
    </button>
    <code title={status}>{status}</code>
    <button class="drawer-toggle" aria-label={open ? 'Collapse drawer' : 'Expand drawer'} title={open ? 'Collapse drawer' : 'Expand drawer'} onclick={onToggle}>
      {#if open}<ChevronDown size={15} />{:else}<ChevronUp size={15} />{/if}
    </button>
  </div>

  {#if open}
    <div class="drawer-content">
      {#if tab === 'problems'}
        <div class="problems-list">
          {#each diagnostics as problem}
            {@const Icon = problem.severity === 'error' ? TriangleAlert : problem.severity === 'success' ? Check : Info}
            <button class="problem-row {problem.severity}" onclick={() => onJump(problem)}>
              <Icon size={15} />
              <div><strong>{problem.title}</strong><p>{problem.detail}</p></div>
              <code>{problem.path}{problem.line ? `:${problem.line}` : ''}</code>
            </button>
          {/each}
          {#if diagnostics.length === 0}
            <article class="problem-row success"><Check size={15} /><div><strong>No problems</strong><p>Latest compile and VM state are clean.</p></div></article>
          {/if}
        </div>
      {:else if tab === 'output'}
        <div class="output-pane">
          <div class="drawer-toolbar output-toolbar">
            <span><SquareTerminal size={14} /><strong>Cart print stream</strong><code>{output.length}/200 lines</code></span>
            <button disabled={!output.length} onclick={onClearOutput}><Eraser size={13} />Clear</button>
          </div>
          <div class="output-scroll" bind:this={outputViewport} onscroll={trackOutputScroll}>
            {#if output.length}
              {#each output as line, index}
                <div class="output-line"><code>{String(index + 1).padStart(3, '0')}</code><span>{line || ' '}</span></div>
              {/each}
            {:else}
              <div class="drawer-empty"><SquareTerminal size={18} /><strong>No output yet</strong><span>Cart <code>print()</code> calls appear here.</span></div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="memory-pane">
          <div class="drawer-toolbar memory-toolbar">
            <span class="memory-title"><Cpu size={14} /><strong>RAM</strong><code>{ram.length ? `${ram.length / 1024} KiB` : 'disconnected'}</code></span>
            <div class="memory-pager">
              <button aria-label="Previous memory page" title="Previous 96 bytes" disabled={resolvedMemoryBase === 0} onclick={() => shiftMemory(-1)}><ChevronLeft size={14} /></button>
              <label class="memory-address"><span>Address</span><input bind:value={memoryAddress} spellcheck="false" onblur={commitMemoryAddress} onkeydown={(event) => { if (event.key === 'Enter') { commitMemoryAddress(); event.currentTarget.blur(); } }} /></label>
              <button aria-label="Next memory page" title="Next 96 bytes" disabled={!ram.length || memoryEnd >= ram.length - 1} onclick={() => shiftMemory(1)}><ChevronRight size={14} /></button>
            </div>
            <select bind:value={memoryRegion} aria-label="Jump to memory region" onchange={jumpMemoryRegion}>
              <option value="">Jump to region…</option>
              {#each MEMORY_REGIONS as region}<option value={region.address}>{formatMemoryAddress(region.address)} {region.label}</option>{/each}
            </select>
            <code class="memory-range">{formatMemoryAddress(resolvedMemoryBase)}–{formatMemoryAddress(memoryEnd)}</code>
          </div>
          {#if memoryRows.length}
            <div class="memory-grid">
              <div class="memory-header"><span>ADDR</span><span>00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F</span><span>ASCII</span></div>
              {#each memoryRows as row}
                <div class="memory-row"><code>{row.address}</code><code>{row.hex}</code><code>{row.ascii}</code></div>
              {/each}
            </div>
          {:else}
            <div class="drawer-empty"><Cpu size={18} /><strong>Memory unavailable</strong><span>Open a cart to inspect VM RAM.</span></div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</section>
