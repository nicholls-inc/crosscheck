// Correct pattern: the value is parsed as unknown and validated, with no assertion.
export interface Handoff {
  target: 'login' | 'signup';
}

export const readHandoff = (raw: string): Handoff | null => {
  const value: unknown = JSON.parse(raw);
  if (typeof value !== 'object' || value === null) return null;
  const target = (value as Record<string, unknown>).target;
  return target === 'login' || target === 'signup' ? { target } : null;
};
