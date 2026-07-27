import assert from 'node:assert/strict';
import test from 'node:test';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { createServer } from 'vite';

test('Assets screen renders both no-cart and open-cart states', async (context) => {
  const vite = await createServer({
    appType: 'custom',
    configFile: false,
    plugins: [svelte()],
    resolve: { preserveSymlinks: true },
    server: { hmr: false, middlewareMode: true, ws: false },
  });
  context.after(() => vite.close());
  const { render } = await vite.ssrLoadModule('svelte/server');
  const { default: Workspace } = await vite.ssrLoadModule('/src/components/Workspace.svelte');
  const noop = () => {};
  const props = {
    screen: 'assets',
    path: '',
    sources: [],
    activeSource: 0,
    palette: Array(16).fill('#000000'),
    spriteSheet: Array(256 * 64).fill(0),
    map: Array(64 * 64).fill(0),
    spriteFlags: Array(256).fill(0),
    sfx: Array(16 * 64).fill(0),
    music: Array(8 * 32).fill(0),
    api: [],
    assetIndex: {
      entries: [
        { kind: 'sprite', id: 1, used: true, nonzero: true, bytes: 64, refs: [{ path: 'main.lua', line: 2, col: 1, label: 'main.lua:2' }] },
      ],
      computedRefs: 0,
    },
    soundSelection: { sfx: 0, pattern: 0 },
    frameData: null,
    onNavigate: noop,
  };
  const html = render(Workspace, { props });

  assert.match(html.body, /<h1>Assets<\/h1>/);
  assert.match(html.body, /No cart open/);
  assert.doesNotMatch(html.body, /Sound effects/);

  const openCartHtml = render(Workspace, { props: { ...props, path: '/tmp/cart' } });
  assert.match(openCartHtml.body, /Cart size/);
  assert.match(openCartHtml.body, /Sprite 001/);
  assert.doesNotMatch(openCartHtml.body, /Sound effects<\/h1>/);
});
