export interface Profile {
  id: string;
}

// Correct pattern: the early return dominates, so id equals the annotated parameter.
export const loadProfile = (raw: string, expected: string): Profile | null => {
  const v = JSON.parse(raw) as Profile;
  if (v.id !== expected) return null;
  return v;
};
