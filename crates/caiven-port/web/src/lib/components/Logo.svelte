<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <polygon fill="#FEB05D" points="100,0 180,54.5 200,126 100,200 0,126 20,54.5" />
  <polygon fill="#FFD29F" points="100,0 100,24.6 39.8,65.6 20,54.5" />
  <polygon fill="#FFC685" points="20,54.5 39.8,65.6 24.8,119.6 0,126" />
  <polygon fill="#F7A854" points="180,54.5 200,126 153.6,114 143.2,75.4" />
  <polygon fill="#F29B3F" points="153.6,114 200,126 100,200 100,153.6" />
  <polygon fill="#FFC685" points="100,46.1 143.2,75.4 153.6,114 100,153.6 46.4,114 56.8,75.4" />
  <polygon fill="#F29B3F" points="100,70 124.2,86.2 130,108 100,130 70,108 75.8,86.2" />
</svg>
