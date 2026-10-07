# Intent: The ledger checker owns its parse messages and closes the gaps PB-1.40 left

Task: PB-1.43. Governing roadmap item: PB-1. Found in the PB-1.40 review (`intent/2026-10-07-ledger-required-fields.md`).

## Problem statement
PB-1.40 made `checkLedgerSchema` in `crosscheck/conformance/main.go` reject a `claims.json` with missing fields, unknown check types or duplicate keys. Its review left five gaps. Each was measured on `origin/main` at 60e1a43 by calling `checkLedgerSchema` and `json.Unmarshal` on the input.

- **`check.type` that is not a string.** `5` gives `narrative_claims[0].check.type is 5, want one of manual|present_artifact`, and `[]` gives `... is [], want one of ...`. Every other key reports a wrong kind as `<path> must be <kind>`. No test covers `5`, `[]` or `""`.
- **Faults in the decoder's words.** An empty file gives `the ledger: EOF`. A file of only whitespace gives the same. A truncated file gives `the ledger: unexpected EOF`. A non-array `narrative_claims` gives `narrative_claims: json: cannot unmarshal object into Go value of type []json.RawMessage`, which names a Go type. Data after the top-level object passes `checkLedgerSchema`, and the later `json.Unmarshal` reports `invalid character '{' after top-level value`, which names no ledger path. A Go upgrade can change any of these, and the tests pin them.
- **`version` spellings.** `1e0` and `-1` give `version must be 1, got 1e0` and `... got -1`. `01` is not JSON and gives `the ledger: invalid character '1' after object key:value pair`. None has a test.
- **Invisible text.** An `id` of `"​"` (a zero-width space) or `"﻿"` passes the non-blank check, so a claim the report prints with no visible name loads.
- **Text that is not Unicode.** A `source` with the raw bytes `ff fe` passes. So does an `id` of `"\udc00"`, an unpaired surrogate escape. `encoding/json` replaces both with U+FFFD without saying so, so the report prints a character the file does not hold.

## Proposed outcome
The checker writes every load fault in its own words, and each decision below has a test.

- **The file comes first.** Before any key is read, the file must be valid UTF-8, must hold a JSON value, and must hold nothing after it but JSON whitespace (space, tab, line feed, carriage return). The faults are:
  - `the ledger is not valid UTF-8 at byte <n>`, with `<n>` the offset of the first bad byte.
  - `the ledger is empty`, for an empty file or one of only whitespace.
  - `the ledger ends before its top-level value is complete`, for a file cut short.
  - `the ledger is not valid JSON at byte <n>`, with `<n>` the offset of the byte where `encoding/json` stopped. A byte-order mark is in this case, because RFC 8259 forbids one in JSON text.
  - `the ledger has data after its top-level value at byte <n>`, with `<n>` the offset of the first byte that is not whitespace.
- **`narrative_claims` that is not an array** is `narrative_claims must be an array`, in the same form as every other kind fault.
- **`check.type` that is not a string** (`5`, `[]`, `{}`, `true`) is `narrative_claims[<i>].check.type must be a string`. A `null` type is `... is null`, as for every other key. A string that names no type, `""` included, keeps `... is "<t>", want one of manual|present_artifact`.
- **`version`** stays the JSON number written `1`. `1e0` and `-1` are wrong values. `01` is not JSON. Each gets a test.
- **Blank text.** A required text field is blank when it holds only white space and Unicode format characters (general category Cf, such as U+200B, U+2060 and U+FEFF). The message stays `must be a non-blank string`. An `id` with a format character between visible ones, such as `C​1`, is not blank and loads.
- **U+FFFD.** No string value may hold U+FFFD, written raw or as an escape, or an unpaired surrogate escape, which `encoding/json` turns into U+FFFD. The fault is `<path> must not contain U+FFFD or an unpaired surrogate`. A raw invalid byte is caught first, by the UTF-8 check.
- The final `json.Unmarshal` stays as a decode. After these checks it cannot fail on a file that passes them, so no message of its own reaches a user.
- The spec, `crosscheck/conformance/README.md` and the header comment of `main.go` say what changed and what is still not yet reached.

## Affected users and systems
- Anyone who edits `claims.json`. A file with an invisible `id`, a U+FFFD, a byte-order mark or a non-string `check.type` now fails the `conformance` job or fails it with a new message. The real ledger still loads its claims.
- `crosscheck/conformance/main.go`, `main_test.go` and `README.md`. The spec `intent/2026-10-07-ledger-load-fails-closed-spec.md`. `docs/TASKS.md`.

## Constraints
- No protected surface changes. `crosscheck/conformance/` is not in `.claude/rules/protected-surfaces.md`.
- The LL-2 and LL-3 prefixes stay. PB-1.40's schema messages keep their wording, except the `check.type` message for a non-string or `null` type, which this task changes on purpose.
- No new rule number. LL-9 gains a file-level bullet, so this change takes no number an open pull request might also take.
- Byte offsets are data the checker reports, not wording. The syntax-error offset comes from `encoding/json`, and a test pins it, so a Go upgrade that moves it fails a test rather than changing a message silently.

## Open questions
None. The task row asks which gaps change the reported fault and whether the checker should own its messages. Both are settled above. Owning the text is the only choice that keeps the messages stable across Go upgrades and lets every fault name a ledger path.

Not yet reached: text that is visible to no reader but is not white space or a format character, such as U+3164 HANGUL FILLER, U+2800 BRAILLE PATTERN BLANK or a lone combining mark. The property that blocks it is a definition of visible text. The open question is whether Unicode's Default_Ignorable_Code_Point property plus a list of fillers is that definition. Two `id`s that differ only by a format character, `C1` and `C​1`, are PB-1.42's question of how ids compare.
