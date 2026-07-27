<script lang="ts">
  import {
    ChevronsLeftRight, Image, Volume2, Layers, Package, Library,
    BookOpen, CircleHelp, House,
  } from '@lucide/svelte';
  import { Button } from '@caiven/ui/button';
  import type { Screen } from '../types';

  interface Props {
    screen: Screen;
    onNavigate: (screen: Screen) => void;
    onTour: () => void;
  }

  let { screen, onNavigate, onTour }: Props = $props();

  const art = $derived(['sprites', 'map', 'palette'].includes(screen));
  const sound = $derived(['sfx', 'music'].includes(screen));
  const modes = $derived([
    { id: 'code' as Screen, label: 'Code', key: 'F1', icon: ChevronsLeftRight, active: screen === 'code' },
    { id: 'sprites' as Screen, label: 'Art', key: 'F2', icon: Image, active: art },
    { id: 'sfx' as Screen, label: 'Sound', key: 'F4', icon: Volume2, active: sound },
    { id: 'assets' as Screen, label: 'Assets', key: '', icon: Layers, active: screen === 'assets' },
    { id: 'cart' as Screen, label: 'Cart', key: 'F7', icon: Package, active: screen === 'cart' },
    { id: 'library' as Screen, label: 'Library', key: 'F8', icon: Library, active: screen === 'library' },
    { id: 'docs' as Screen, label: 'Docs', key: 'F9', icon: BookOpen, active: screen === 'docs' },
  ]);
</script>

<nav class="mode-rail" aria-label="Studio modes">
  {#each modes as mode}
    {@const Icon = mode.icon}
    <Button variant="ghost" class={mode.active ? 'active' : undefined} title={`${mode.label}${mode.key ? ` — ${mode.key}` : ''}`} onclick={() => onNavigate(mode.id)}>
      <Icon size={20} />
      <span>{mode.label}</span>
    </Button>
  {/each}
  <div class="rail-spacer"></div>
  <Button variant="ghost" title="Take guided tour" onclick={onTour}>
    <CircleHelp size={20} />
    <span>Learn</span>
  </Button>
  <Button variant="ghost" class={screen === 'welcome' ? 'active' : undefined} title="Start screen" onclick={() => onNavigate('welcome')}>
    <House size={20} />
    <span>Start</span>
  </Button>
</nav>
