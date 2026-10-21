# Spec: Name CGV as Layer 3 reach for Python in `/assurance-layer-audit`

Intent: `intent/2026-10-21-cgv-layer-3-python.md`. Task: VA-1.19.

## Requirements

- **CL-1.** Step 4, Layer 3, of `crosscheck/skills/assurance-layer-audit/SKILL.md` has a Python bullet that names CGV (`cgv/` in the Crosscheck repository) as Layer 3 reach for data paths into the declared fields of Django models, data classes, attrs, pydantic models, `NamedTuple` and `TypedDict`. It tells the auditor to report that reach only when the repository declares such fields, and to name `contracts check` from `cgv/` as the command that checks them. It does not name the `cgv_check` MCP tool.
- **CL-2.** The same bullet states CGV's strength and trusted base in the words of `crosscheck/docs/assurance-hierarchy.md` (Layer 3 row) and `cgv/README.md`: the Lean checker is deterministic and proved sound over the graph it is given; the trusted base is `Translation.lean` (no theorems yet) and `BehaviorModel.lean` (trusted, not proved); the Rust extractor is untrusted but auditable. It carries the exit-0 qualifiers: only paths the extractor discovered, relative to the translated constraints and not to Django or pydantic, for data class, attrs, `NamedTuple` and `TypedDict` fields only for a type-correct program, and a warning is an unknown path, not a consistent one. It points at `cgv/README.md` "What exit 0 promises" for the rest.
- **CL-3.** The same bullet says that Layer 3 for any other Python code is not yet reached. Blocking property: CGV's data path (`IsDataPath`) must end at a model node, and nothing else checks composition. Open question: how a callee's contract should be modelled when the callee has no model node. Runtime contract libraries (`icontract`, `dry-python`) are described as dynamic checks, tested evidence on the inputs that run.
- **CL-4.** A TypeScript bullet says Layer 3 is not yet reached. Runtime contract libraries such as `zod` give dynamic checks. Blocking property: no checker reads TypeScript declarations into a contract graph. Open question: how to check composition end to end, not pair by pair.
- **CL-5.** The words "not formal" no longer describe Python's Layer 3. `grep -n "not formal" crosscheck/skills/assurance-layer-audit/SKILL.md` prints nothing.
- **CL-6.** The Instructions paragraph no longer says "a Python repo loses most of Layer 1". It says that Layer 1 reaches only pure, sequential code written in Dafny and extracted to Python or Go, and that almost nobody has end-to-end Layer 3 subgraph verification outside CGV's Python data paths.
- **CL-7.** The verification checklist asks Ruby, Java and C# repositories, not Python ones, to state that Layer 1 is unreachable without a verifier bridge. It adds one item: a Python repository that declares such fields names CGV's Layer 3 reach with its trusted base and exit-0 qualifiers.
- **CL-8.** No step, output section, JSON field or threshold is added or removed. The Go bullet, the "honest reading" line, the example reach table and the Layer 5 text are unchanged (VA-1.8).

## Concerns flagged

- No check enforces CL-1 to CL-7. A wording lint for skills is not yet reached; VA-1.5's spec records why.
- The skill points at `cgv/` in the Crosscheck repository. A user of the installed plugin has no `cgv/` checkout, and CGV ships no package yet. The audit can name the reach, but running it needs the Crosscheck repository. ER-1.20's MCP tool may close that; this change does not depend on it.
- The skill's Step 3 detects no CGV signal. The audit decides CGV's reach from the field declarations it finds in Python source, which it reads under Step 4.
