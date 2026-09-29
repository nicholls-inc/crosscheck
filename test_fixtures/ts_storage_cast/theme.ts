export type Theme = 'light' | 'dark';

// BUG: getItem returns string | null, asserted as a non-null Theme.
export const readTheme = (): Theme => localStorage.getItem('theme') as Theme;

// null is admitted by the asserted type: only the choices warning remains.
export const readThemeOrNull = (k: string): Theme | null => {
  return localStorage.getItem(k) as Theme | null;
};
