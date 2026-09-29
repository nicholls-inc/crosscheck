# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working in this repository.

## What this is

A suite of tools for verifying AI-generated code. `docs/VISION.md` states what the suite is for.

| Path | What it holds | Guidance |
| --- | --- | --- |
| `cgv/` | The contract graph verifier: a Rust extractor and a Lean checker with soundness proofs | `cgv/CLAUDE.md` |
| `docs/VISION.md` | The vision the whole suite shares | |

Run each tool's commands from its own directory. For example, run `cargo test` from `cgv/`.

## Vision

Two rules from `docs/VISION.md` apply to every session:

- No class of code is out of scope. When a tool does not reach a class of code, say "not yet reached" and name the property that blocks it and the open research question. Never call a class "excluded" or "out of scope".
- No guarantee rests on the judgment of an LLM. An LLM may draft code, specs, and proofs, and LLM-based checks may point at likely problems, but only deterministic checks and human judgment count as evidence.

## Protected surfaces

`.claude/rules/protected-surfaces.md` lists the files and theorem statements that need a stated rationale in any PR that changes them.
