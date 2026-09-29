export interface Session {
  id: string;
}

declare function log(message: string): void;

// The comparison does not dominate the return: v is returned on every path.
export const loadSession = (raw: string, expected: string): Session => {
  const v = JSON.parse(raw) as Session;
  if (v.id === expected) log('match');
  return v;
};
