# Spec: Count only a whole `Fixes-Incident:` line as an incident reference

Intent: `intent/2026-10-06-incident-line.md`. Governing roadmap item: PB-1. Task: PB-1.16.

This spec revises IE-5 and IE-6 of `intent/2026-09-30-incident-eval-range-spec.md` and adds IE-9. ID IE-8 is skipped because PR #66 claims it.

- **IE-9.** A line of the PR body or of a commit message is an incident reference when it matches `^[ \t]*Fixes-Incident:[ \t]*(\S+)[ \t]*$`, ignoring case. The id is the captured word, with one trailing `.`, `,` or `;` dropped. Every other line is not a reference. In particular:
  - the trigger in the middle of a line, including inside backticks, is not a reference;
  - a list or quote marker (`- `, `* `, `+ `, `> `) before the trigger makes the line not a reference;
  - a line that starts with the trigger and has two or more words after it is not a reference;
  - a trigger line with no value is not a reference, and never takes the next line as its id.
- **IE-5 (revised).** The PR body is split into lines on `\n` or `\r\n` and matched one line at a time, as commit messages already are. The body's lines are read before the commits' lines, and the first reference wins. The previous IE-5 clause "`PR_BODY` is matched as one text, so an incident line with no value can still take the next line's first token" is withdrawn. The `incident` label, the eval and invariant lookups and the exit codes are unchanged.
- **IE-6 (revised).** `scripts/ci/incident-eval-check.test.mjs` replaces the case "the body is matched as one text, so an incident line with no value takes the next line" with its opposite (exit 0, skip line), and adds:
  - the body line and the commit line of #62, verbatim: exit 0 and the skip line, each on its own;
  - `- `, `* `, `+ ` and `> ` before a trigger with an id, in the body and in a commit: exit 0 and the skip line;
  - a commit line that starts with the trigger and has more words after the id: exit 0 and the skip line;
  - an indented trigger line with trailing spaces in the body, and a body with `\r\n` line ends: exit 1, and the output names the id;
  - a quoted trigger line earlier in the body and a whole trigger line later: the later line's id is read.

## Not yet reached
- **A trigger line inside a fenced code block still counts.** The property that blocks it is a Markdown parse of the PR body. The open question is whether a parser that the scripts can carry with no dependencies agrees with GitHub's renderer on where a fence ends. The tier gate's `Tier:` line has the same gap.
- **A placeholder id counts.** A line that reads exactly `Fixes-Incident: <id>` names the incident `<id>`. The property that blocks it is a grammar for incident ids. The open question is which systems issue incident ids and what form they take; the repository has no incident record yet to settle it.
- **A malformed reference is silent.** `Fixes-Incident: INC-7 (the outage)` is not a reference, so a pull request that writes it and sets no label is skipped. The property that blocks a failure on it is telling a malformed reference from wrapped prose, which #62 shows can start a line with the trigger. The open question is whether the commit trailer block, as `git interpret-trailers --parse` reads it, can be required for commits while the PR body, which has no trailer block, keeps the whole-line rule.
