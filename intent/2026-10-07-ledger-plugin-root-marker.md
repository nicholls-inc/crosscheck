# Intent: A plugin root that is not a Crosscheck plugin tree fails the conformance run

Task: PB-1.41. Governing roadmap item: PB-1. Found in the PB-1.30 review (`intent/2026-10-07-ledger-missing-root.md`).

## Problem statement
PB-1.30 made the conformance oracle fail on a plugin root that does not resolve (LL-10). A root that resolves to a directory that is not a plugin tree still passes. Measured on `origin/main` at 683caf0, from `crosscheck/conformance`, with `go run . <root>`:

- `<root>` an empty directory: `skills discovered : 0`, `ERRORS   : 0`, `NARRATIVE LEDGER (0 claims):`, `RESULT: PASS`, exit 0.
- `<root>` = `../..`, the repository root, which is what `go run ./crosscheck/conformance .` scans from the repository root: the same output, exit 0.
- `<root>` = `../docs`: the same output, exit 0.

`TestLedgerLoadRoot/empty_root` pins the pass. A run that scans a directory with no plugin in it vouches for nothing, so it must not print `RESULT: PASS`.

The task row asked what marks a directory as a plugin root, a `skills/` directory or a `.claude-plugin/plugin.json`. The measurements settle it:

- Claude Code loads a plugin from the directory that holds `.claude-plugin/plugin.json`, and the manifest's `name` is the plugin's identity. `crosscheck/.claude-plugin/plugin.json` names `crosscheck`, and so does each installed copy under `~/.claude/plugins/cache/nicholls/crosscheck/<version>/`.
- A `skills/` directory does not mark a Crosscheck tree. `~/.claude` has one (84 skills discovered), and so does every other plugin in the cache, such as `cloudflare/cloudflare/1.0.0` (14 skills discovered). Those runs fail today only because their skills break Crosscheck's own rules, which is luck, not a check.
- A manifest alone does not mark a Crosscheck tree either. Every plugin in the cache has one. The oracle checks Crosscheck's own claims (the `dafny_*` tools, the `add-mode` tags, the ledger), so the manifest must name `crosscheck`.

## Proposed outcome
- New rule LL-11. When `os.Stat` on the plugin root reports a directory, and `<root>/.claude-plugin/plugin.json` cannot be read, does not decode as JSON, or does not name `crosscheck` in its `name` field, `analyze` appends one error, `[root] plugin root <root> is not a Crosscheck plugin tree: <reason>`. The run prints `RESULT: FAIL` and exits 1. The scan and the ledger checks still run, so the other errors are reported too.
- A root that does not resolve stays an LL-10 error. A root that is a regular file or a symlink loop stays an LL-2 error. LL-11 does not fire on them, because `os.Stat` reports no directory, so each still fails with one error.
- A mode-000 directory as the root fails with two errors, LL-11 (the manifest read is denied) and LL-2 (the ledger read is denied). Both are true.
- `TestLedgerLoadRoot` asserts LL-11 for an empty root, a directory with files but no manifest, a manifest that is not JSON, and a manifest that names another plugin. It asserts LL-2 and no LL-10 for a regular file, a mode-000 directory (skipped when the process runs as root) and a symlink loop. `baseTree` gains a manifest that names `crosscheck`, so every other test still builds a plugin tree.
- The spec, the header comment of `main.go` and `crosscheck/conformance/README.md` say what marks a plugin root.

## Affected users and systems
- Anyone who runs `go run ./crosscheck/conformance <root>` on a directory that is not the Crosscheck plugin tree, such as the repository root. The run now fails instead of passing with zero skills, agents and claims.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.
- The `conformance` CI job runs on the real `crosscheck/` tree, whose manifest names `crosscheck`, so it still passes with seven claims.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`, and `crosscheck/.claude-plugin/plugin.json` is read, not edited.
- LL-1 to LL-10 keep their rules and messages.
- PB-1.40 (required ledger fields) is a separate row and stays out of this change.

## Open questions
None. The measurements above answer the task row's question.

Not yet reached: a manifest is checked for its `name` only, matched by `json.Unmarshal`, which ignores the case of the key. A directory that holds a copied Crosscheck manifest and nothing else still passes with 0 skills. The property that blocks it is a minimum inventory for a Crosscheck tree, and the open question is whether the oracle should require one, such as at least one skill and one agent.
