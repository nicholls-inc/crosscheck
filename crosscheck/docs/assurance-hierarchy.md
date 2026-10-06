# Assurance hierarchy — onboarding guide

## TL;DR

Crosscheck organises correctness into six layers of assurance. Layer 1 proves the implementation is correct against a specification — deterministic, machine-checkable. Layers 2 and 3 are not yet reached (see the table), apart from CGV's check of Python data paths. Layers 4–6 prove the specification is the right specification — deterministic at Layer 4, probabilistic at Layer 5 (~96% accuracy on round-trip checks). A proof at Layer 6 that a spec is complete is not yet reached. The property that blocks it is that no formal requirement is tied to the spec, so there is nothing to prove the spec complete against. The open question is how to write requirements formally and prove that a spec achieves them ([roadmap item RQ-1](https://github.com/nicholls-inc/crosscheck/blob/main/docs/assurance/ROADMAP.md)). Until then, `/spec-adversary` probes for missing properties, and what it finds points at gaps but is not evidence that none remain. For the full treatment — motivation, semantics, and trade-offs — see [`./research/assurance-hierarchy.md`](./research/assurance-hierarchy.md).

## Skill → layer mapping

| Layer | Concern | Crosscheck skill(s) | Confidence | Owner |
|-------|---------|---------------------|------------|-------|
| 1 | Formally verified pure code | [`/spec-iterate`](../skills/spec-iterate/SKILL.md), [`/generate-verified`](../skills/generate-verified/SKILL.md), [`/extract-code`](../skills/extract-code/SKILL.md), [`/lightweight-verify`](../skills/lightweight-verify/SKILL.md) | Deterministic | byfuglien |
| 2 | Compilation correctness | Not yet reached. The property that blocks it is that the Dafny backends and your toolchain are not verified, so they sit in the trusted base. The open question is whether translation validation of each extracted file can replace that trust. | Trusted, not checked | n/a |
| 3 | Contract graph verification | Not yet reached across a whole service. The property that blocks it is that Dafny-verified units and their callers share no contract format. The open question is how to check composition end to end, not pair by pair. CGV (`cgv/`) is a separate mechanism: a Rust extractor reads Python source for edges and field constraints, `Translation.lean` turns them into a graph, and a Lean checker checks the data paths over it. See the [research doc](./research/assurance-hierarchy.md). | CGV: the Lean checker is deterministic and proved sound over the graph it is given. A kernel replay of the built environment is not yet reached ([TB-1](https://github.com/nicholls-inc/crosscheck/blob/main/docs/assurance/ROADMAP.md)). The property that blocks it is that nothing rechecks the built environment, so a declaration that skipped the kernel can pass CI's axiom check. The open question is whether a replay (`lean4checker`, `Environment.replay`) fits CGV CI's time budget. The trusted base is `Translation.lean` (no theorems yet) and `BehaviorModel.lean` (trusted, not proved; documentation only, no theorem references it yet). The Rust extractor is untrusted but auditable: it tags its output `[EXTRACTED]` with source locations. A reader must also check what the soundness theorems state; CI tracks their statements in `cgv/prover/protected-statements.txt`. See the trust table in [`cgv/README.md`](https://github.com/nicholls-inc/crosscheck/blob/main/cgv/README.md). Elsewhere: not checked | n/a |
| 4 | Implementation–spec alignment + semi-formal rationales | [`/invariant-coverage-scaffold`](../skills/invariant-coverage-scaffold/SKILL.md), [`/protected-surface-amend`](../skills/protected-surface-amend/SKILL.md), [`/rationale`](../skills/rationale/SKILL.md) (see [snapshot](./specs/rationale-2026-05-11.md)), [`/check-regressions`](../skills/check-regressions/SKILL.md), [`/assurance-probe`](../skills/assurance-probe/SKILL.md) (Phase 1 – experimental; gates on SNR ≥ 1:3 over 20 runs) | Deterministic (4); semi-formal (rationales) | hellebuyck (4 + rationales) / byfuglien (regressions, probe) |
| 5 | Specification–intent alignment | [`/intent-check`](../skills/intent-check/SKILL.md), [`/acceptance-oracle-draft`](../skills/acceptance-oracle-draft/SKILL.md) | Probabilistic (~96%) | hellebuyck |
| 6 | Specification completeness | [`/spec-adversary`](../skills/spec-adversary/SKILL.md) | Search only. A proof of completeness is not yet reached, because no formal requirement is tied to the spec ([RQ-1](https://github.com/nicholls-inc/crosscheck/blob/main/docs/assurance/ROADMAP.md)) | hellebuyck |

## Getting started — onboarding flow

1. Run [`/assurance-layer-audit`](../skills/assurance-layer-audit/SKILL.md) to scope which layers are reachable in your repo (language, tooling, ecosystem maturity).
2. Run [`/assurance-init`](../skills/assurance-init/SKILL.md) to scaffold the governance skeletons (ROADMAP, protected surfaces, skeleton invariant docs for 1–3 modules).
3. Run [`/invariant-coverage-scaffold`](../skills/invariant-coverage-scaffold/SKILL.md) once per supported language to install the pre-commit + CI gate that ties invariant docs to property tests.
4. On every protected-surface change: run [`/protected-surface-amend`](../skills/protected-surface-amend/SKILL.md) to generate the governance-note amendment block; on every invariant-related change: run [`/intent-check`](../skills/intent-check/SKILL.md) to verify the spec→test alignment survived the diff.
5. Run [`/assurance-status`](../skills/assurance-status/SKILL.md) weekly to surface drift, FP rate, and kill-criterion triggers.
6. Run [`/assurance-probe`](../skills/assurance-probe/SKILL.md) every 2-4 weeks on active modules (rotation-based) to measure test strength via mutation probes.
7. Run [`/spec-adversary`](../skills/spec-adversary/SKILL.md) on stable modules to probe for invariants the spec is missing — Layer 6 is iterative, not deterministic.

## When to use what

- Writing new business logic that should be provably correct? → [`/spec-iterate`](../skills/spec-iterate/SKILL.md) → [`/generate-verified`](../skills/generate-verified/SKILL.md) → [`/extract-code`](../skills/extract-code/SKILL.md) (byfuglien).
- Code is correct but you suspect the spec might be wrong? → [`/intent-check`](../skills/intent-check/SKILL.md) (hellebuyck).
- Code mixes provable, testable, readable, and judgement claims and you want one structured argument? → [`/rationale`](../skills/rationale/SKILL.md) (hellebuyck — Layer 4 semi-formal rationales).
- Adding a new feature with user-observable behaviour? → [`/acceptance-oracle-draft`](../skills/acceptance-oracle-draft/SKILL.md) to lock down the scenarios upfront (hellebuyck).
- Changing a file that's already governed (e.g. an invariant doc, an agent, a workflow)? → [`/protected-surface-amend`](../skills/protected-surface-amend/SKILL.md) (hellebuyck).
- Module has been stable for a while — what might its spec be missing? → [`/spec-adversary`](../skills/spec-adversary/SKILL.md) (hellebuyck).
- Test passes but you suspect it's too weak to catch real failures? → [`/assurance-probe`](../skills/assurance-probe/SKILL.md) (byfuglien).
- Forgot whether your repo's onboarded? → [`/assurance-status`](../skills/assurance-status/SKILL.md) (hellebuyck).

## Closing pointers

- [`./agents.md`](./agents.md) — which agent owns what.
- [`./skills.md`](./skills.md) — full skill catalogue with trigger phrases.
- [`./research/assurance-hierarchy.md`](./research/assurance-hierarchy.md) — full research treatment.
- [`./research/literature-review.md`](./research/literature-review.md) — academic prior art.