import type { ExternalThing } from 'some-lib';

interface Options {
  retries: number;
  mode: 'a' | 'b';
}

// Neither target resolves syntactically: nothing is emitted, so nothing is reported.
export const loadThing = (raw: string) => JSON.parse(raw) as ExternalThing;

export const loadPartial = (raw: string) => JSON.parse(raw) as Partial<Options>;
