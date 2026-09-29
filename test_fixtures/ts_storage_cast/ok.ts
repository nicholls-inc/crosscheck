// Correct pattern: validate the stored string instead of asserting it.
export type Mode = 'compact' | 'roomy';

export const readMode = (): Mode => {
  const raw = localStorage.getItem('mode');
  return raw === 'roomy' ? 'roomy' : 'compact';
};
