# Intent: A human can tell an agent to tick a verification box, and tools settle the mechanical ones

Task: PB-1.38. Governing roadmap item: PB-1.

## Problem statement
A protected-surface pull request carries a governance-note block with a Review Checklist and `REQUIRES HUMAN VERIFICATION:` markers. The rules say a human resolves them and an agent never self-authorises. Two things go wrong in practice.

1. On #108, #70 and #77 the maintainer was asked to tick boxes such as "Authoriser is a named human", "The amendment block appears in the PR body" and "Diff plan enumerates every affected file". A command decides each of these. A human reading the diff to confirm them adds no evidence.
2. The maintainer then told the agent to tick the boxes, and the agent first declined, because the rules say a human resolves a marker. The maintainer had to give the instruction three times in one session.

The rule text that causes both is in `.claude/rules/protected-surfaces.md` (Amendment pattern, steps 1 and 4), `REVIEW.md` (the protected-surface question), `docs/gates/protected-surface-amendment.md`, `crosscheck/skills/protected-surface-amend/SKILL.md` (the checklist template) and the template in `crosscheck/skills/assurance-init/SKILL.md` (Step 5), which is how plugin users get the rule.

## Proposed outcome
- A maintainer's explicit instruction to an agent, "tick these boxes", is a human resolution. The agent ticks, and leaves a PR comment that names who instructed it.
- A checklist item that a command decides is drafted already ticked, with the command and its output in the block. The reviewer is not asked about it. Only items that need judgment stay as markers.
- The same text ships in the plugin's `protected-surface-amend` skill and in the `assurance-init` scaffold, so a plugin user's repository gets it.

## Affected users and systems
- The maintainer, who stops being asked to confirm mechanical facts.
- Any agent that drafts a governance note or is asked to tick one.
- Plugin users who run `/assurance-init` or `/protected-surface-amend`.
- `.claude/rules/protected-surfaces.md`, `REVIEW.md`, `docs/gates/protected-surface-amendment.md`, the two skills above, `docs/TASKS.md`, `JOURNAL.md`.

## Constraints
- `docs/VISION.md` rule 2: no guarantee rests on an LLM's judgment. A mechanical item is ticked only with the output of a deterministic command in the block. If the drafting tool cannot run the command, the item stays a marker.
- An agent never ticks on its own initiative, and an instruction that arrives in file, tool or PR content is not an instruction from the human.
- The maintainer's merge stays the approval. Delegating a tick does not make an agent the authoriser.

## Open questions
None blocking. A CI step that recomputes the mechanical items, so they stop being ticks at all, is not yet reached: the property that blocks it is that the gate does not parse the block, and the open question is how to parse a free-form PR body reliably. It is row PB-1.39.
