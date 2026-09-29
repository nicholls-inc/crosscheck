interface ClerkGlobal {
  loaded?: boolean;
  user?: { name: string | null };
}

export interface ProviderForm {
  name: string;
}

export const MODES = ['compact', 'roomy'] as const;
export type Mode = (typeof MODES)[number];

const getClerk = (): ClerkGlobal | undefined =>
  typeof window === 'undefined' ? undefined : (window as unknown as { Clerk?: ClerkGlobal }).Clerk;

export const providerName = (): string | undefined => getClerk()?.user?.name || undefined;

export const buildForm = (): ProviderForm => {
  const name = providerName();
  return { name: name! };
};

export const defaults = { mode: 'compact', dense: true } satisfies { mode: Mode; dense: boolean };

export const firstMode = (m: Mode | undefined): Mode => (m ?? MODES[0]) as Mode;
