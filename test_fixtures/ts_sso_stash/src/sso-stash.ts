import type { SsoStrategy } from '@/app/session';

export interface SsoStash {
  clientName: string;
  origin: 'login' | 'signup';
  search?: string;
  strategy?: SsoStrategy;
}

export const peekSsoStash = (clientName: string): SsoStash | null => {
  try {
    const raw = sessionStorage.getItem('clerkSsoStash');
    if (!raw) return null;
    const stash = JSON.parse(raw) as SsoStash;
    return stash.clientName === clientName ? stash : null;
  } catch {
    return null;
  }
};
