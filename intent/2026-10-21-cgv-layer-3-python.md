# Intent: Name CGV as Layer 3 reach for Python in `/assurance-layer-audit`

Task: VA-1.19. Governing roadmap item: VA-1.

## Problem statement
`/assurance-layer-audit` tells a Python repository that Layer 3 has only runtime contract libraries (`icontract`, `dry-python`), which give dynamic checks and "not formal" subgraph verification (`crosscheck/skills/assurance-layer-audit/SKILL.md`, Step 4, Layer 3). That is no longer true. CGV, in `cgv/` of the same repository, checks the data paths of a Python project into the declared fields of Django models, data classes, attrs, pydantic, `NamedTuple` and `TypedDict`, and its Lean checker is proved sound over the graph it is given. `crosscheck/docs/assurance-hierarchy.md` (Layer 3 row) already says so. The skill is what an agent reads and repeats to a user, so a Python user is told that Layer 3 reach does not exist for them.

The same skill says in its Instructions that "a Python repo loses most of Layer 1", and its verification checklist asks that Python repos "explicitly state Layer 1 is unreachable without a verifier bridge". Both contradict Step 4 of the same skill, which says Python reaches Layer 1 through Dafny's Python backend, and `/extract-code`, which compiles verified Dafny to Python. Layer 1 reaches the same class of code in Python as in Go: pure, sequential functions written in Dafny and extracted.

## Proposed outcome
`/assurance-layer-audit` names CGV as Layer 3 reach for a Python repository that declares such fields. It states CGV's strength and trusted base as `crosscheck/docs/assurance-hierarchy.md` and `cgv/README.md` state them: the Lean checker is deterministic and proved sound over the graph it is given; the trusted base is `Translation.lean` (no theorems yet) and `BehaviorModel.lean` (trusted, not proved); the Rust extractor is untrusted but auditable; and exit 0 covers only the paths the extractor found, relative to the translated constraints, and for annotation-only contracts only for a type-correct program. A warning means an unknown path, not a consistent one. Everything else at Layer 3 in a Python repository stays "not yet reached", with the blocking property and the open question. TypeScript, which shared the old bullet, gets its own "not yet reached" bullet with the same two parts.

The Layer 1 statements about Python say that Python reaches Layer 1 for the same class of code as Go, and the checklist keeps "unreachable without a verifier bridge" for Ruby, Java and C#.

## Affected users and systems
- Anyone who runs `/assurance-layer-audit` on a Python or TypeScript repository and reads its projection, and `/assurance-init`, which fills its roadmap projection from that output.
- One Class A protected-surface file, `crosscheck/skills/assurance-layer-audit/SKILL.md`, plus `docs/TASKS.md`, `crosscheck/JOURNAL.md` and a governance note. The change is Tier 3.

## Constraints
- Do not overclaim. CGV reaches data paths into the fields listed above and nothing else. Its exit-0 qualifiers go with the claim.
- No skill gains or loses a step, an output section, a JSON field or a threshold.
- The Layer 3 "aspirational" wording (the Go bullet, the "honest reading" line and the example table row) and the Layer 5 strength belong to VA-1.8. This change leaves those words as they are.
- ER-1.20 is adding a `cgv_check` MCP tool in another lane. The skill refers to running `contracts check` from `cgv/` and does not name that tool yet. ER-1.21 routes `byfuglien` to it later.
- Only the VA-1.19 row of `docs/TASKS.md` changes.

## Decided without the maintainer
The maintainer granted full autonomy for this run in chat on 2026-10-21 and is away. Each open question was settled with the most conservative option.

1. **`/assurance-init` and `/assurance-status` are not edited.** The row says all three skills tell a Python repository that Layer 3 has only runtime contract libraries. A grep of `crosscheck/skills` and `crosscheck/agents` finds that claim, and any Layer 3 claim at all, only in `/assurance-layer-audit`. `/assurance-init` leaves each reach cell of its projection as `TODO: fill from /assurance-layer-audit output`, so it takes the corrected wording from the audit. `/assurance-status` states no layer reach. Editing a protected surface that holds no wrong claim adds risk and no correction. The maintainer can ask for an explicit CGV pointer in either skill as a new row.
2. **The checklist line on Layer 1 is corrected with the Instructions line.** The brief named only the Instructions line. The checklist line makes the same claim in the same file, and leaving it would have the skill contradict itself.
3. **The TypeScript half of the old bullet becomes "not yet reached".** Splitting the bullet leaves TypeScript on its own. Its blocking property and open question are the ones the Layer 3 row of `crosscheck/docs/assurance-hierarchy.md` gives for a whole service, so no new position is proposed.
4. **The blocking property and open question for Python code outside CGV's reach come from `cgv/README.md`.** "Only paths that end at a model node": the theorem's `IsDataPath` requires a path to end at a model node; the open question is how a callee's contract should be modelled when the callee has no model node.

## Open questions
None left open. The decisions above are recorded for the maintainer, who ratifies them by merging.
