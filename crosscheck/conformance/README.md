# Crosscheck conformance / inventory oracle

The doc-vs-implementation gate for Crosscheck *as a whole*. Same idea as the
invariant↔test coverage gate, lifted to the meta level: **docs ↔ artifacts.**
It exists because Crosscheck's own `plausible ≠ correct` failure mode bit its
maintainer — the docs described a methodology that only partly ships, and
nothing mechanical said which parts. This is that mechanical check.

It is a self-contained Go module (stdlib only, no third-party deps).

## Run

    go run ./crosscheck/conformance              # from repo root, scans crosscheck/
    go run ./crosscheck/conformance crosscheck   # explicit root
    go run ./crosscheck/conformance /path/to/crosscheck

Build a standalone binary:

    go build -o conformance ./crosscheck/conformance
    ./conformance crosscheck

Exit 0 = PASS, 1 = FAIL (any AUTO error, or any `unreviewed` ledger claim or
claim with an unknown status, or a `present_artifact` ledger check that
disagrees with the filesystem, or a `claims.json` that cannot be read or
parsed, or a plugin root that is not a Crosscheck plugin tree).

> Run commands assume the repo-root Go workspace (`go.work`), which lets the
> nested module resolve when invoked from the repo root. From inside this
> directory you can also run `go run . ..` and `go test ./...`.

## Two layers

- **AUTO (deterministic, fails CI):**
  1. *Structural* — every `skills/<dir>/` has a `SKILL.md` with `name`/`description`
     frontmatter; `name` matches the dir; agents likewise. Non-empty.
  2. *Phantom* — docs reference a `/<skill>` or `/crosscheck:<skill>` that doesn't exist.
  3. *Orphan* — an artifact ships but is referenced in no user-facing doc (WARN).
  4. *MCP* — README-claimed `dafny_*` tools exist in `mcp-server/` source (WARN).
  5. *Routing* — every skill/agent an agent's **body** routes to via a backtick
     `` `/<x>` `` or `/crosscheck:<x>` token resolves to a real artifact (frontmatter
     is stripped first, so a `/<skill>` token in a `description:` is not mistaken
     for a routing edge). Extends reference integrity to the *trunk*: the phantom
     check only scans the user-facing doc set, so an orchestrator routing to a
     non-existent skill would otherwise slip through. The second trunk-level
     self-check after this oracle itself (CLAIM-SELF-COVERAGE).

     **What AUTO 5 does NOT catch** (disclosed reach; tracked in
     [#221](https://github.com/nicholls-inc/claude-code-marketplace/issues/221)):
     - *Bare-slash routing* — `/<skill>` written without backticks is invisible to
       the routing scanner (it matches only backtick `` `/<x>` `` and `/crosscheck:<x>`),
       so a skill reachable only via bare-slash prose is unchecked (false negative).
     - *Cross-plugin / example tokens* — a backtick `/<token>` in an agent body is
       flagged even when it names a sibling-plugin skill (e.g. the `field-report` plugin's skill), an
       example, or a flag; there is no cross-plugin allowlist (false positive).
     - *Plain-prose agent→agent edges* — the dominant routing form (e.g. naming
       `byfuglien`/`hellebuyck` in prose) is not modelled as a routing edge at all.
- **LEDGER (`claims.json`, reviewed not auto-proved):** narrative claims that
  can't be checked by reference integrity — layer/phase/mode counts, terminal
  states, self-coverage. Each entry records the claim, the observed reality, a
  review `status`, and where possible an auto-check that **re-fires if reality
  changes** (e.g. `agents/lowry.md expect_present:false` flips to FAIL the day
  Phase 4 ships, forcing the ledger to be updated). `status:"unreviewed"` fails
  CI to force triage of any new claim. A `status:"known-gap"` entry must carry a
  `tracked_in` link to its tracking issue (the ADD epic
  [#217](https://github.com/nicholls-inc/claude-code-marketplace/issues/217) and
  its children); a known-gap with no link also fails CI, so a "known" gap can
  never be tracked nowhere. `status` must be exactly one of `unreviewed`,
  `known-gap`, `reviewed-disclosed` or `reviewed-accurate`. Any other value,
  including an empty or missing one, fails CI, so a typo such as
  `reviewed-disclsed` cannot pass as a reviewed claim.

  A missing `claims.json` is an empty ledger. A `claims.json` that cannot be
  read, or is not valid JSON, fails CI, so a syntax error cannot pass as a
  ledger with no claims. A `claims.json` or `conformance` directory that is a
  symlink to a missing target cannot be read, so it fails CI too. So does a
  plugin root that does not resolve, because the path is wrong or a symlink on
  it dangles: a run that scans nothing cannot pass. A plugin root is a directory
  whose `.claude-plugin/plugin.json` has a `name` key, spelt exactly as Claude
  Code reads it, whose value is `crosscheck`, and that holds at least one skill
  (`skills/<name>/SKILL.md`) and one agent (`agents/<name>.md`). Any other
  directory, such as the repository root or a directory that holds only a copy
  of the manifest, fails CI. Not yet reached: a copied manifest next to one
  skill and one agent passes, whatever else is missing. The property that
  blocks it is a check that ties the tree to a released Crosscheck inventory,
  and the open question is whether one can be written without pinning a count
  that changes with every release. JSON that parses but breaks the ledger
  schema fails CI as well:

  | Place | Required keys | Optional keys |
  |---|---|---|
  | top level | `version` (the number `1`), `narrative_claims` (an array) | `description` (a string) |
  | claim | `id`, `source`, `claim`, `reality` (non-blank strings), `status` (a string), `check` (an object) | `tracked_in` (a string) |
  | `check` of type `manual` | `type` | none |
  | `check` of type `present_artifact` | `type`, `path` (a non-blank string) | `expect_present` (`true` or `false`, default `true`) |

  Any other `check.type`, such as `present_artfact`, fails, so a misspelt type
  cannot pass as a check that never runs. Keys match exactly, including case, so
  a misspelt `tracked-in` or `expect-present` fails instead of being dropped. A
  key that appears twice in one object fails, so no copy of a key can hide from
  the check. No value may be `null`. Two claims whose `id`s match once
  surrounding white space is trimmed and case is folded fail, so `C1`, `c1` and
  `C1 ` cannot name three different claims.

  The file must be UTF-8 and hold one JSON value with nothing after it but
  whitespace. An empty file, a truncated one, a byte-order mark and data after
  the value each fail with a message written by the checker rather than by
  `encoding/json`, so a Go upgrade cannot change it. The byte-order mark and
  the data after the value name the byte; the empty and truncated messages do
  not. A `check.type` that is not a string fails as `must be a string`. A required
  text field of only white space and format characters, such as a zero-width
  space, is blank. No string may hold U+FFFD or an unpaired surrogate escape,
  which `encoding/json` would turn into U+FFFD without saying so.

  Not yet reached: what the other text fields say. `source` need not name a
  real file, `tracked_in` need not name a real issue, and `check.path` may point
  outside the plugin root. The property that blocks them is a check of each
  field against the tree and the tracker, and the open question is which of
  them can be checked without a network call. Two `id`s that differ only by a
  look-alike letter from another script are distinct. What blocks it is a rule
  for which characters an `id` may hold, and the open question is whether that
  is an allowlist or a Unicode confusables check (PB-1.45). Two `id`s that differ
  only by a format character, such as `C1` and `C1` plus U+200B, are distinct
  (PB-1.49). Text that no reader
  sees but that is neither white space nor a format character, such as U+3164
  HANGUL FILLER, is not blank. The property that blocks it is a definition of
  visible text, and the open question is whether Unicode's
  Default_Ignorable_Code_Point property is it.

## First-run findings (2026-05-30, plugin v2.5.1)

- **ERROR** `assurance-probe` — `SKILL.md` had no frontmatter; couldn't load as a
  skill, yet `byfuglien` routes to it. **Fixed in the PR that introduced this
  oracle** (frontmatter added; this is why the gate is now GREEN).
- **WARN** `journal-context` — was undocumented in the user-facing doc set; now
  documented in the README skills overview and the top-level `CLAUDE.md`, so the
  orphan warning clears.
- **LEDGER known-gaps** — Phase 4 agent, operating modes, committed methodology,
  Phase 5 auditor, self-coverage. See `claims.json`; each is tracked in the ADD
  epic [#217](https://github.com/nicholls-inc/claude-code-marketplace/issues/217)
  and its child issues (#218–#221), which replaced the former `docs/add/roadmap.md`.

## Wire to CI (suggested)

```yaml
# .github/workflows/conformance.yml  (sketch)
jobs:
  conformance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-go@v5
        with: { go-version: '1.25' }
      - run: go run ./crosscheck/conformance crosscheck
      - run: go test ./crosscheck/conformance/...
```

## Extend

Add a claim to `claims.json` whenever the docs assert something about the plugin
that the filesystem doesn't already prove. New claims start `status:"unreviewed"`
(fails CI) until a human triages them to `reviewed-disclosed`,
`reviewed-accurate` or `known-gap`.
