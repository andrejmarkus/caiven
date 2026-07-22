const BASE = '/api/v2';

export interface Cart {
  id: string;
  title: string;
  author: string;
  description: string;
  tags: string[];
  uploaded_at: string;
  downloads: number;
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

export interface CommentInfo {
  id: string;
  author: string;
  body: string;
  created_at: string;
}

export type Sort = 'new' | 'popular' | 'top';

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
};
