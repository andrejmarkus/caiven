import { api, type UserInfo } from './api';

let user = $state<UserInfo | null>(null);
let loaded = $state(false);

export const currentUser = {
  get value() {
    return user;
  },
  get loaded() {
    return loaded;
  },
};

export async function hydrateUser(): Promise<void> {
  try {
    user = await api.me();
  } catch {
    user = null;
  } finally {
    loaded = true;
  }
}

export function setUser(u: UserInfo | null): void {
  user = u;
}
