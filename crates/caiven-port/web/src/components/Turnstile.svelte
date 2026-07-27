<script lang="ts">
  let { siteKey, onToken }: { siteKey: string; onToken: (token: string) => void } = $props();
  let container: HTMLDivElement | undefined = $state();

  function loadScript(): Promise<void> {
    return new Promise((resolve) => {
      const w = window as unknown as { turnstile?: unknown };
      if (w.turnstile) return resolve();
      const existing = document.querySelector('script[data-turnstile]');
      if (existing) {
        existing.addEventListener('load', () => resolve());
        return;
      }
      const s = document.createElement('script');
      s.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js';
      s.async = true;
      s.defer = true;
      s.dataset.turnstile = 'true';
      s.onload = () => resolve();
      document.head.appendChild(s);
    });
  }

  $effect(() => {
    let widgetId: string | undefined;
    let cancelled = false;
    loadScript().then(() => {
      if (cancelled || !container) return;
      const turnstile = (window as unknown as { turnstile: { render: Function; remove: Function } })
        .turnstile;
      widgetId = turnstile.render(container, {
        sitekey: siteKey,
        callback: (token: string) => onToken(token),
        'expired-callback': () => onToken(''),
        'error-callback': () => onToken(''),
      });
    });
    return () => {
      cancelled = true;
      const turnstile = (window as unknown as { turnstile?: { remove: Function } }).turnstile;
      if (widgetId && turnstile) turnstile.remove(widgetId);
    };
  });
</script>

<div bind:this={container}></div>
