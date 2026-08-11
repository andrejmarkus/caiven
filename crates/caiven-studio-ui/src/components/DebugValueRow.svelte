<script lang="ts">
  import { ChevronRight, ChevronDown, X } from '@lucide/svelte';
  import { Button } from '@caiven/ui/button';
  import type { DebugChild } from '../types';
  import Self from './DebugValueRow.svelte';

  interface Props {
    label: string;
    value: string;
    nodeId: string | null | undefined;
    depth?: number;
    onExpand: (nodeId: string) => Promise<DebugChild[]>;
    onRemove?: (key: string) => void;
  }

  let { label, value, nodeId, depth = 0, onExpand, onRemove }: Props = $props();

  let expanded = $state(false);
  let children = $state<DebugChild[] | null>(null);
  let loading = $state(false);
  let error = $state('');

  async function toggle() {
    if (!nodeId) return;
    if (!expanded && children === null) {
      loading = true;
      error = '';
      try {
        children = await onExpand(nodeId);
      } catch {
        error = 'expired';
      } finally {
        loading = false;
      }
    }
    expanded = !expanded;
  }
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
  {#if loading}
    <div class="watch-row" style={`padding-left:${14 + (depth + 1) * 14}px`}><span class="watch-empty-inline">Loading…</span></div>
  {:else if error}
    <div class="watch-row" style={`padding-left:${14 + (depth + 1) * 14}px`}><span class="watch-empty-inline">{error}</span></div>
  {:else if children && children.length}
    {#each children as child (child.key)}
      <Self label={child.key} value={child.value} nodeId={child.nodeId} depth={depth + 1} {onExpand} />
    {/each}
  {:else}
    <div class="watch-row" style={`padding-left:${14 + (depth + 1) * 14}px`}><span class="watch-empty-inline">empty</span></div>
  {/if}
{/if}
