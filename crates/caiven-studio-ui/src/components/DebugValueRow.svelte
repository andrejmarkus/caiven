<script lang="ts">
  import { untrack } from 'svelte';
  import { ChevronRight, ChevronDown, X } from '@lucide/svelte';
  import { Button } from '@caiven/ui/button';
  import type { DebugChild } from '../types';
  import Self from './DebugValueRow.svelte';

  interface Props {
    label: string;
    value: string;
    nodeId: string | null | undefined;
    depth?: number;
    /** Changes whenever Lua ran; an expanded row then refetches its children. */
    revision?: number;
    onExpand: (nodeId: string) => Promise<DebugChild[]>;
    onRemove?: (key: string) => void;
  }

  let { label, value, nodeId, depth = 0, revision = 0, onExpand, onRemove }: Props = $props();

  let expanded = $state(false);
  let children = $state<DebugChild[] | null>(null);
  let error = $state('');
  let fetching = false;
  let stale = false;

  // One request at a time; a revision that lands mid-request fetches once more after it.
  async function load() {
    if (fetching) {
      stale = true;
      return;
    }
    fetching = true;
    try {
      do {
        stale = false;
        if (!nodeId) break;
        try {
          children = await onExpand(nodeId);
          error = '';
        } catch {
          children = null;
          error = 'unavailable';
        }
      } while (stale && expanded);
    } finally {
      fetching = false;
    }
  }

  function toggle() {
    if (!nodeId) return;
    expanded = !expanded;
    if (expanded) void load();
  }

  $effect(() => {
    revision;
    untrack(() => {
      if (expanded) void load();
    });
  });

  // A value that is no longer a table or function has nothing to show.
  $effect(() => {
    if (!nodeId) untrack(() => (expanded = false));
  });
</script>

<div class="watch-row" style={`padding-left:${14 + depth * 14}px`}>
  {#if nodeId}
    <button class="expand-toggle" onclick={toggle} aria-label={expanded ? `Collapse ${label}` : `Expand ${label}`}>
      {#if expanded}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
    </button>
  {:else}
    <span class="expand-spacer"></span>
  {/if}
  <code>{label}</code><i>=</i><strong>{value}</strong>
  {#if onRemove}
    <Button variant="ghost" size="icon-xs" title={`Remove ${label}`} onclick={() => onRemove(label)}><X size={12} /></Button>
  {/if}
</div>
{#if expanded}
  {#if error}
    <div class="watch-row" style={`padding-left:${14 + (depth + 1) * 14}px`}><span class="watch-empty-inline">{error}</span></div>
  {:else if children === null}
    <div class="watch-row" style={`padding-left:${14 + (depth + 1) * 14}px`}><span class="watch-empty-inline">Loading…</span></div>
  {:else if children.length}
    {#each children as child (child.key)}
      <Self label={child.key} value={child.value} nodeId={child.nodeId} depth={depth + 1} {revision} {onExpand} />
    {/each}
  {:else}
    <div class="watch-row" style={`padding-left:${14 + (depth + 1) * 14}px`}><span class="watch-empty-inline">empty</span></div>
  {/if}
{/if}
