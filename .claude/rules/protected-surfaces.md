# Protected surfaces

The checker's promise — exit code 0 means every data path is consistent —
rests on two things the Lean kernel does not check for you:

1. **The behaviour model.** `BehaviorModel.lean` states how Django and the
   plain-Python data class libraries behave. It is trusted, not proved: if a
   rule there is wrong, the checker can be wrong while every proof still
   passes.
2. **What the soundness theorems say.** Lean proves each theorem, but only
   the statement it is given. Narrowing a statement (a new hypothesis, a
   weaker conclusion, a changed definition it mentions) keeps the build green
   and silently shrinks the guarantee.

Changes to either are allowed, but never silently.

## What is protected

**Class A — trusted semantics (the whole file):**
- `cgv/prover/ContractGraph/BehaviorModel.lean`

**Class B — statements of the soundness theorems and the definitions they
mention** (the statement, not the proof; rewriting a proof freely is fine):

| Name | File |
| --- | --- |
| `constraintImplies` | `cgv/prover/ContractGraph/Checker.lean` |
| `checkEdge_sound`, `checkEdgeAll_sound`, `checkEdgeAll_sound_noErrors` | `cgv/prover/ContractGraph/Checker.lean` |
| `IsDataPath`, `stepwiseSound` | `cgv/prover/ContractGraph/Composition.lean` |
| `checkPath_sound`, `checkPath_sound_noErrors` | `cgv/prover/ContractGraph/Composition.lean` |
| `enumeratePaths_complete` | `cgv/prover/ContractGraph/Search.lean` |
| `closedStates_checkPath` | `cgv/prover/ContractGraph/StateSearch.lean` |
| `runChecker_sound`, `runChecker_sound_all` | `cgv/prover/ContractGraph/Main.lean` |
| `runChecker_exitCode_eq_zero_iff`, `runCheckerPaths_exitCode_eq_zero_iff`, `incompleteWith_exitCode` | `cgv/prover/ContractGraph/Main.lean` |

Renaming or deleting any of these counts as changing it.

## What a change needs

A PR that touches a protected surface includes, in its description, a
**Protected-surface change** section with:

1. **What changed** — the file and name, and the old and new wording (or a
   one-line diff of the statement).
2. **Why** — the bug, new capability or correction that requires it.
3. **Effect on the guarantee** — whether exit 0 now promises more, less or
   the same, and for which inputs. "Less" is allowed; unstated "less" is not.
4. **Evidence** — for Class A, the library documentation or a runtime check
   behind the rule (version-pinned); for Class B, which callers and docs
   (the trust model in `cgv/README.md`, the key theorems in `cgv/CLAUDE.md`) were updated to match.

A reviewer, human or automated, treats a protected-surface change without
this section as a blocking finding, whatever the code's correctness.

When a new soundness theorem becomes part of the end-to-end guarantee, add it
to the table above in the same PR.
