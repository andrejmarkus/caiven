<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <rect width="32" height="32" rx="8" fill="var(--color-brand)" />
  <rect x="9" y="24" width="14" height="4" rx="1.5" fill="var(--color-ember)" />
  <text
    x="16"
    y="19.5"
    text-anchor="middle"
    font-family="'Space Grotesk', ui-sans-serif, sans-serif"
    font-weight="700"
    font-size="15"
    fill="white">C</text
  >
</svg>
