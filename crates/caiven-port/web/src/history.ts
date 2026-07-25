import type { Cart } from './api';

const KEY = 'caiven-play-history-v1';

export interface HistoryEntry {
  cart: Cart;
  played_at: string;
}

export function readHistory(): HistoryEntry[] {
  try {
    const value = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(value) ? value : [];
  } catch {
    return [];
  }
}

export function rememberCart(cart: Cart): void {
  const next = [{ cart, played_at: new Date().toISOString() }, ...readHistory().filter((entry) => entry.cart.id !== cart.id)].slice(0, 100);
  localStorage.setItem(KEY, JSON.stringify(next));
}

export function playSessionId(): string {
  const key = 'caiven-play-session-v1';
  let value = sessionStorage.getItem(key);
  if (!value) {
    value = crypto.randomUUID();
    sessionStorage.setItem(key, value);
  }
  return value;
}
