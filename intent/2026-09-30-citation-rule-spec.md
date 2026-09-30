# Spec: The tier gate accepts any citation line, and only a regular file in the repository

Intent: `intent/2026-09-30-citation-rule.md`. Governing roadmap item: PB-1. Issue: #49.

This spec adds TG-12 and TG-13, and revises the citation rule that TG-2, TG-3 and TG-4 of `intent/2026-09-29-deterministic-evidence-spec.md` share. The other TG requirements do not change.

- **TG-12. Citations.** TG-2 accepts an `Intent:` citation, TG-3 a `Spec:` citation, and TG-4 a `Plan:` citation.
  - A citation line starts with the keyword, case-insensitive, optionally after spaces or tabs, and optionally after one list or quote marker (`-`, `*`, `+`, `>`) and at least one space or tab. The keyword in the middle of a line does not count. This format is unchanged.
  - The cited path is the first run of non-space characters after the keyword, with one leading and one trailing backtick removed.
  - Every citation line for the keyword counts. The requirement is met when at least one of them cites a valid path. Order does not matter, so `Plan: TBD` then `Plan: intent/p.md` passes.
  - A path is valid when it resolves, against the repository root and through any symlinks, to a regular file whose real path lies inside the real path of the repository root. A directory, a path that does not exist, a path outside the repository such as `../x`, and a symlink in the repository that points outside it are not valid.
  - Known gaps, not rules. The gate checks that the cited file exists, not what it says, so any regular file in the repository satisfies a citation. A file under `.git/` is inside the root and counts. Neither lets a change skip a check that another rule makes.
- **TG-13. Tests.** `scripts/ci/tier-gate.test.mjs` covers:
  - `+ Plan: intent/p.md` is a citation (TG-12);
  - `-Plan: intent/p.md`, `>> Plan: intent/p.md`, `1. Plan: intent/p.md` and `- [ ] Plan: intent/p.md` are not citations, and the gate asks for a plan (TG-12);
  - `Plan: TBD` then `Plan: intent/p.md` passes, and so does the reverse order (TG-12);
  - `Plan: intent/missing.md` then `Plan: intent` fails (TG-12);
  - `Plan: intent`, a directory, fails (TG-12);
  - `Plan: ../outside.md` and the absolute path of a file outside the repository fail, although the file exists (TG-12);
  - a symlink in the repository that points at a file outside it fails, and one that points at a file inside it passes (TG-12);
  - `Intent: TBD` then `Intent: intent/old.md` passes Tier 1, and `Spec: intent` fails Tier 2, so the rule holds for every keyword (TG-12).
- **DOC-7.** `docs/assurance/TIER-LAYER-MAP.md` and `docs/gates/tier-layer-gate.md` say that a citation names a file in the repository and that any citation line counts. `intent/2026-09-29-deterministic-evidence-spec.md` points TG-2 at this spec.
