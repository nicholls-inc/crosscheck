export type Density = 'compact' | 'roomy';

// BUG: getItem returns string | null, asserted as a non-null Density.
export const readDensity = (): Density => localStorage.getItem('density') as Density;
