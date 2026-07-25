const BASE = '/api/v2';

export interface Cart {
  id: string;
  title: string;
  author: string;
  description: string;
  tags: string[];
  uploaded_at: string;
  downloads: number;
  plays: number;
  owner: string | null;
  rating_avg: number;
  rating_count: number;
  latest_version: number;
  cart_size: number;
  has_screenshot: boolean;
}

export interface CartVersionInfo {
  version: number;
  cart_size: number;
  changelog: string;
  has_screenshot: boolean;
  created_at: string;
}

export interface CartDetail extends Cart {
  versions: CartVersionInfo[];
  own_rating: number | null;
}

export interface CartList {
  carts: Cart[];
  total: number;
  page: number;
  per_page: number;
}

export interface TagCount {
  tag: string;
  count: number;
}

export interface UserProfile {
  username: string;
  is_admin: boolean;
  created_at: string;
  carts: Cart[];
  total: number;
  total_plays: number;
  follower_count: number;
  following_count: number;
  followed_by_me: boolean;
}

export interface UserInfo {
  id: string;
  username: string;
  is_admin: boolean;
}

export interface TokenInfo {
  id: string;
  name: string;
  created_at: string;
  last_used_at: string | null;
}

export interface TokenCreated extends TokenInfo {
  token: string;
}

export interface SessionInfo {
  id: string;
  created_at: string;
  expires_at: string;
  current: boolean;
}

export interface CommentInfo {
  id: string;
  author: string;
  body: string;
  created_at: string;
}

export type Sort = 'new' | 'popular' | 'trending' | 'top';

export interface CollectionInfo {
  slug: string;
  title: string;
  description: string;
  kind: 'editorial' | 'player';
  featured_rank: number | null;
  owner: string;
  cart_count: number;
  follower_count: number;
  followed_by_me: boolean;
  carts: Cart[];
  created_at: string;
  updated_at: string;
}

export interface JamInfo {
  slug: string;
  title: string;
  description: string;
  rules: string;
  starts_at: string;
  submissions_close_at: string;
  ends_at: string;
  status: 'upcoming' | 'open' | 'closed';
  entry_count: number;
  creator_count: number;
  carts: Cart[];
}

export interface FeedEvent {
  kind: 'cart_published' | 'version_published' | 'collection_addition' | 'jam_entry';
  actor: string;
  occurred_at: string;
  cart: Cart;
  version: number | null;
  collection_slug: string | null;
  collection_title: string | null;
  jam_slug: string | null;
  jam_title: string | null;
}

export interface FeedPage {
  events: FeedEvent[];
  page: number;
  per_page: number;
  total: number;
}

export interface MetricWindow {
  current: number;
  previous: number;
}

export interface DailyMetric {
  date: string;
  plays: number;
  unique_players: number;
}

export interface DashboardInfo {
  plays: MetricWindow;
  unique_players: MetricWindow;
  rating_avg: number;
  followers: number;
  new_followers: number;
  daily: DailyMetric[];
  carts: Cart[];
}

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    credentials: 'include',
    headers: init?.body instanceof FormData ? undefined : { 'Content-Type': 'application/json' },
    ...init,
  });
  if (!res.ok) {
    let message = res.statusText;
    try {
      const body = await res.json();
      message = body.error ?? message;
    } catch {
      // no JSON body
    }
    throw new ApiError(res.status, message);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

function qs(params: Record<string, string | number | undefined>): string {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== '') p.set(k, String(v));
  }
  const s = p.toString();
  return s ? `?${s}` : '';
}

export const api = {
  register: (username: string, password: string) =>
    request<UserInfo>('/auth/register', { method: 'POST', body: JSON.stringify({ username, password }) }),
  login: (username: string, password: string) =>
    request<UserInfo>('/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) }),
  logout: () => request<void>('/auth/logout', { method: 'POST' }),
  me: () => request<UserInfo>('/auth/me'),
  changePassword: (currentPassword: string, newPassword: string) =>
    request<void>('/auth/password', {
      method: 'POST',
      body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
    }),
  listSessions: () => request<SessionInfo[]>('/auth/sessions'),
  revokeSession: (id: string) => request<void>(`/auth/sessions/${id}`, { method: 'DELETE' }),
  revokeAllSessions: () => request<void>('/auth/sessions', { method: 'DELETE' }),
  listTokens: () => request<TokenInfo[]>('/auth/tokens'),
  createToken: (name: string) =>
    request<TokenCreated>('/auth/tokens', { method: 'POST', body: JSON.stringify({ name }) }),
  revokeToken: (id: string) => request<void>(`/auth/tokens/${id}`, { method: 'DELETE' }),

  listCarts: (opts: { page?: number; per_page?: number; q?: string; tag?: string; author?: string; sort?: Sort } = {}) =>
    request<CartList>(`/carts${qs(opts)}`),
  getCart: (id: string) => request<CartDetail>(`/carts/${id}`),
  createCart: (cart: File, meta: { title: string; author: string; description: string; tags: string[] }) => {
    const form = new FormData();
    form.set('cart', cart);
    form.set('meta', JSON.stringify(meta));
    return request<Cart>('/carts', { method: 'POST', body: form });
  },
  updateCart: (id: string, patch: { title?: string; description?: string; tags?: string[] }) =>
    request<Cart>(`/carts/${id}`, { method: 'PATCH', body: JSON.stringify(patch) }),
  deleteCart: (id: string) => request<void>(`/carts/${id}`, { method: 'DELETE' }),
  createVersion: (id: string, cart: File, changelog: string) => {
    const form = new FormData();
    form.set('cart', cart);
    form.set('meta', JSON.stringify({ changelog }));
    return request<Cart>(`/carts/${id}/versions`, { method: 'POST', body: form });
  },
  cartUrl: (id: string, version?: number) => `${BASE}/carts/${id}/cart${qs({ version })}`,
  screenshotUrl: (id: string, version?: number) => `${BASE}/carts/${id}/screenshot${qs({ version })}`,

  rateCart: (id: string, score: number) =>
    request<void>(`/carts/${id}/rating`, { method: 'PUT', body: JSON.stringify({ score }) }),
  unrateCart: (id: string) => request<void>(`/carts/${id}/rating`, { method: 'DELETE' }),
  listComments: (id: string) => request<CommentInfo[]>(`/carts/${id}/comments`),
  addComment: (id: string, body: string) =>
    request<CommentInfo>(`/carts/${id}/comments`, { method: 'POST', body: JSON.stringify({ body }) }),
  deleteComment: (id: string, commentId: string) =>
    request<void>(`/carts/${id}/comments/${commentId}`, { method: 'DELETE' }),

  listTags: () => request<TagCount[]>('/tags'),
  userProfile: (username: string, page?: number, per_page?: number) =>
    request<UserProfile>(`/users/${username}${qs({ page, per_page })}`),

  recordPlay: (id: string, session_id: string) =>
    request<{ counted: boolean; plays: number }>(`/carts/${id}/play`, {
      method: 'POST',
      body: JSON.stringify({ session_id }),
    }),
  followUser: (username: string) => request<void>(`/users/${username}/follow`, { method: 'PUT' }),
  unfollowUser: (username: string) => request<void>(`/users/${username}/follow`, { method: 'DELETE' }),
  feed: (page = 0, per_page = 20) => request<FeedPage>(`/feed${qs({ page, per_page })}`),
  dashboard: () => request<DashboardInfo>('/dashboard'),

  listCollections: (opts: { kind?: string; owner?: string; page?: number; per_page?: number } = {}) =>
    request<CollectionInfo[]>(`/collections${qs(opts)}`),
  getCollection: (slug: string) => request<CollectionInfo>(`/collections/${slug}`),
  createCollection: (input: { title: string; description?: string }) =>
    request<CollectionInfo>('/collections', { method: 'POST', body: JSON.stringify(input) }),
  createEditorialCollection: (input: { title: string; description?: string; featured_rank?: number | null }) =>
    request<CollectionInfo>('/admin/collections', { method: 'POST', body: JSON.stringify(input) }),
  updateCollection: (slug: string, input: { title?: string; description?: string; featured_rank?: number | null }) =>
    request<CollectionInfo>(`/collections/${slug}`, { method: 'PATCH', body: JSON.stringify(input) }),
  deleteCollection: (slug: string) => request<void>(`/collections/${slug}`, { method: 'DELETE' }),
  addCollectionCart: (slug: string, cart_id: string) =>
    request<CollectionInfo>(`/collections/${slug}/carts`, { method: 'POST', body: JSON.stringify({ cart_id }) }),
  removeCollectionCart: (slug: string, cartId: string) =>
    request<CollectionInfo>(`/collections/${slug}/carts/${cartId}`, { method: 'DELETE' }),
  reorderCollection: (slug: string, cart_ids: string[]) =>
    request<CollectionInfo>(`/collections/${slug}/order`, { method: 'PUT', body: JSON.stringify({ cart_ids }) }),
  followCollection: (slug: string) => request<void>(`/collections/${slug}/follow`, { method: 'PUT' }),
  unfollowCollection: (slug: string) => request<void>(`/collections/${slug}/follow`, { method: 'DELETE' }),

  listJams: () => request<JamInfo[]>('/jams'),
  getJam: (slug: string) => request<JamInfo>(`/jams/${slug}`),
  createJam: (input: {
    title: string;
    slug?: string;
    description?: string;
    rules?: string;
    starts_at: string;
    submissions_close_at: string;
    ends_at: string;
  }) => request<JamInfo>('/admin/jams', { method: 'POST', body: JSON.stringify(input) }),
  enterJam: (slug: string, cart_id: string) =>
    request<JamInfo>(`/jams/${slug}/entries`, { method: 'POST', body: JSON.stringify({ cart_id }) }),
  withdrawJam: (slug: string, cartId: string) =>
    request<JamInfo>(`/jams/${slug}/entries/${cartId}`, { method: 'DELETE' }),
};
