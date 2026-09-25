// Which remix was on its way to Publish when the account wall appeared, so
// the email-confirmation link can bring the person straight back to it.

const KEY = 'caiven:remix-pending-publish';

export const draftKey = (cartId: string) => `caiven:remix-draft:${cartId}`;

export function markPendingPublish(cartId: string): void {
  try { localStorage.setItem(KEY, cartId); } catch { /* storage blocked */ }
}

export function clearPendingPublish(): void {
  try { localStorage.removeItem(KEY); } catch { /* nothing to clear */ }
}

/** The pending remix's cart id, only while its unpublished draft still exists. */
export function pendingPublish(): string | null {
  try {
    const id = localStorage.getItem(KEY);
    return id && localStorage.getItem(draftKey(id)) ? id : null;
  } catch {
    return null;
  }
}
