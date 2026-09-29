# TypeScript frontend: assertion-boundary checking

Status: implemented for the patterns in drivers-web PR #876. Nothing under `prover/` changes.

## Why only assertion boundaries

Under `strict`, tsc proves every flow whose source has a TypeScript type, for the two kinds a TS type expresses:
nullability and literal-union membership. Emitting those flows into the graph would repeat tsc's proof with weaker
machinery, spend the state budget, and add false positives wherever the syntactic frontend knows less than tsc
(a provider name `string | null` narrowed by `|| undefined` and a `!= null` filter before a required form field).

The checker adds value where a value enters the typed world by assertion instead of proof: a runtime read whose
result tsc types as `any` or `string | null`, asserted with `as T`. The TS frontend emits exactly those sites.

Trust split, stated in the README trust model: exit 0 on a TS project means every *extracted assertion site* is
consistent with the type it asserts. Typed flows are delegated to `tsc --strict`, which the checker does not run.

## Graph shape

- **Source.** The nearest named enclosing function (`function f`, `const f = () => ..`, or a named function passed to
  `useCallback` / `useMemo` / `memo` / `forwardRef`), as a `function` node. A top-level site uses `<module FILE>`.
  Function nodes carry no contract rows.
- **Runtime source table** (`RuntimeSource`, the whole policy of where tsc's guarantee is void):

  | Call | tsc type | Guarantee on the edge |
  |---|---|---|
  | `JSON.parse(..)` | `any` | nothing |
  | `localStorage.getItem(..)`, `sessionStorage.getItem(..)` | `string \| null` | nullable |
  | `<x>.searchParams.get(..)`, `new URLSearchParams(..).get(..)`, `searchParams.get(..)` | `string \| null` | nullable |

  The operand of `as T` (after unwrapping parentheses, and `as unknown` in `e as unknown as T`) must be one of these
  calls, or a `const` identifier in the same function whose initializer is one. `as const`, `satisfies`, `!`, and
  casts on any other operand are not sites: tsc checks overlap on those, or the operand's own type is the proof.
- **Target.** The asserted type `T`, resolved syntactically (see Types). Each target becomes `model` nodes (slots):
  - a named alias or enum of scalars: one node named after it (`Theme`);
  - an inline scalar type: one node named after its source text (`'a' | 'b'`);
  - an interface or object type: one node per property, `Iface.prop` (`SsoStash.origin`), like Python data class fields;
  - `T[]`: the element's slots, once.

  A slot's precondition rows are `nullability` (0 unless the property is `?:` or the type has a `null` / `undefined`
  member) and `choices` (only when every union member resolves to a string or number literal). No `type` rows: tsc
  owns types. A slot that can reject nothing (optional and no choices) is not written.
- **Edge.** One `writes_to` edge per (site, slot), `discovery = ast_pattern`, `source_override = 1`, `site_file` and
  `site_line` at the cast, with the guarantee as override rows via `value_analysis::facts_rows`. An empty guarantee is
  "no postconditions", so the checker warns where the slot could reject and errors where the guarantee contradicts it
  (`getItem(k) as Theme` without `| null` is an error).
- **Guard evidence.** For `const v = <site>` followed by `return v.p === x ? v : null` (either operand order, `!==` with
  the branches swapped, or `if (v.p !== x) return null; return v;`), property `p` takes the guarantee of `x` when `x` is
  a parameter or `const` with an annotation, or a literal. That discharges `SsoStash.clientName` in `peekSsoStash`.
  A guard that does not dominate the return gives nothing.

## Types

`TypeIndex` indexes every `type` alias, `interface` (with `extends` of indexed interfaces) and `enum` in the project,
keyed by (file, name). `ImportMap` follows named imports, `import type`, `import { A as B }` and `export { A } from`,
resolving specifiers through relative paths and `compilerOptions.paths` / `baseUrl` of the root `tsconfig.json`
(comments and trailing commas allowed), trying `.ts .tsx .d.ts .mts .cts` and `/index.*`.

`Shape` is `Primitive | Literals | Nullable | Object | Unknown`. Generics, conditional, mapped, indexed and
`keyof` types, intersections, `export *`, cycles and unresolved imports are `Unknown`, never guessed. In a union an
`Unknown` member removes `choices` but keeps an explicit `null` visible. A cast whose target is `Unknown` writes
nothing and is counted on stderr (`TS: N assertion sites skipped: target type unresolved`), so an unresolved type can
only remove findings.

## Files and CLI

- `src/ts/` holds the frontend: `mod.rs` (`extract`, `TsGraph::write`), `sites.rs` (runtime-source table, visitor,
  guard evidence), `types.rs` (`TypeIndex`, `ImportMap`, `Shape`, tsconfig).
- `extractor.rs` walks `.py` and `.ts .tsx .mts .cts .js .jsx .mjs .cjs` in one pass (`.d.ts` files feed the type
  index only), runs both frontends into one database, and names TS nodes by `<relative path>.<name>` so they cannot
  collide with Python module names. JavaScript files parse but yield no sites: they have no `as`.
- A parse error in a TS file is handled like a Python one: fatal unless `--allow-parse-errors`.
- `report.rs` text output shortens a choices list past 12 members; JSON output is unchanged.
- `scripts/diff-findings.sh BASE.json HEAD.json` prints findings present in HEAD but not BASE, keyed by path, guarantee
  and requirement (not by line), for reviewing a pull request.

## Not done (candidates for later)

- Taint from a cast to its consumers (`SSO_PROVIDER_BRANDING[stash.strategy]`): the defect is reported at the cast.
- `!` on nullable operands, and narrowing casts on typed operands (`x as 'a'` where `x: 'a' | 'b'`).
- Joi / zod runtime schemas as slots; `Response#json()` as a runtime source.
- Exact derived types (`keyof typeof`, generics), which would need tsc itself.

## Alternatives considered

- **tsc sidecar (Node + the TypeScript compiler API).** Exact types, including `TranslationKey` and guard narrowing.
  Rejected for this scope: a reviewer needs Node and an installed TypeScript (drivers-web's snapshot has no
  `node_modules`), fixtures stop being offline, and the TS 7 native port changes the API. On PR #876 it finds the same
  defect.
- **Emit every typed flow, Python-style.** Thousands of consistent edges and a false positive on the provider-name path
  unless the frontend re-implements tsc narrowing.
