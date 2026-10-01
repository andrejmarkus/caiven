<script lang="ts">
  import VolumeIcon from '@lucide/svelte/icons/volume-2';
  import VolumeOffIcon from '@lucide/svelte/icons/volume-x';
  import { audio, saveAudio } from '../lib/audio-prefs.svelte';

  let { onchange }: { onchange: () => void } = $props();

  function update(next: Partial<typeof audio>) {
    Object.assign(audio, next);
    saveAudio();
    onchange();
  }
</script>

<div class="flex items-center gap-2">
  <button onclick={() => update({ muted: !audio.muted })} aria-label={audio.muted ? 'Unmute' : 'Mute'} class="flex size-9 items-center justify-center rounded-md border border-void-700 text-muted-foreground hover:bg-void-800">
    {#if audio.muted || audio.volume === 0}<VolumeOffIcon class="size-4" />{:else}<VolumeIcon class="size-4" />{/if}
  </button>
  <input
    type="range" min="0" max="1" step="0.05" aria-label="Volume"
    value={audio.volume}
    disabled={audio.muted}
    oninput={(e) => update({ volume: Number(e.currentTarget.value) })}
    class="hidden w-24 accent-[var(--color-ember)] disabled:opacity-40 sm:block"
  />
</div>
