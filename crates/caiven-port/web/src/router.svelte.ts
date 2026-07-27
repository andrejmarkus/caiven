let path = $state(window.location.pathname);
let search = $state(window.location.search);

window.addEventListener('popstate', () => {
  path = window.location.pathname;
  search = window.location.search;
});

export const route = {
  get path() {
    return path;
  },
  get search() {
    return new URLSearchParams(search);
  },
};

export function navigate(to: string): void {
  const url = new URL(to, window.location.origin);
  if (url.pathname !== window.location.pathname || url.search !== window.location.search) {
    window.history.pushState({}, '', to);
  }
  path = url.pathname;
  search = url.search;
}

export function link(node: HTMLAnchorElement): { destroy(): void } {
  function onClick(e: MouseEvent) {
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    const href = node.getAttribute('href');
    if (!href || href.startsWith('http') || href.startsWith('//')) return;
    e.preventDefault();
    navigate(href);
  }
  node.addEventListener('click', onClick);
  return {
    destroy() {
      node.removeEventListener('click', onClick);
    },
  };
}

export interface RouteMatch {
  name: string;
  params: Record<string, string>;
}

export function matchRoute(p: string): RouteMatch {
  const segs = p.split('/').filter(Boolean);
  if (segs.length === 0) return { name: 'home', params: {} };
  if (segs[0] === 'browse') return { name: 'browse', params: {} };
  if (segs[0] === 'tags') return { name: 'tags', params: {} };
  if (segs[0] === 'collections' && segs[1]) return { name: 'collection', params: { slug: segs[1] } };
  if (segs[0] === 'collections') return { name: 'collections', params: {} };
  if (segs[0] === 'jams' && segs[1]) return { name: 'jam', params: { slug: segs[1] } };
  if (segs[0] === 'jams') return { name: 'jams', params: {} };
  if (segs[0] === 'activity') return { name: 'activity', params: {} };
  if (segs[0] === 'library') return { name: 'library', params: {} };
  if (segs[0] === 'dashboard') return { name: 'dashboard', params: {} };
  if (segs[0] === 'settings' || segs[0] === 'profile') return { name: 'settings', params: {} };
  if (segs[0] === 'login') return { name: 'login', params: {} };
  if (segs[0] === 'register') return { name: 'register', params: {} };
  if (segs[0] === 'verify-email') return { name: 'verify-email', params: {} };
  if (segs[0] === 'forgot-password') return { name: 'forgot-password', params: {} };
  if (segs[0] === 'reset-password') return { name: 'reset-password', params: {} };
  if (segs[0] === 'upload') return { name: 'upload', params: {} };
  if (segs[0] === 'link-studio') return { name: 'link-studio', params: {} };
  if (segs[0] === 'cart' && segs[1]) return { name: 'cart', params: { id: segs[1] } };
  if (segs[0] === 'play' && segs[1]) return { name: 'play', params: { id: segs[1] } };
  if (segs[0] === 'author' && segs[1]) return { name: 'author', params: { username: segs[1] } };
  return { name: 'notfound', params: {} };
}
