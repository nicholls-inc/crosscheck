# Crosscheck

Tools for verifying AI-generated code. The goal is to know that AI-generated code is correct without trusting an AI to say so. [`docs/VISION.md`](docs/VISION.md) states the vision.

## Tools

- [`crosscheck/`](crosscheck/README.md) is the Crosscheck Claude Code plugin. It verifies code with Dafny and Lean, runs differential random testing against Lean models, and carries the specification and assurance skills.
- [`cgv/`](cgv/README.md) is the contract graph verifier. It extracts contracts from Python code (Django models and plain-Python data classes) and checks them across component boundaries. Its checker has machine-checked soundness proofs in Lean.

## Install the plugin

The plugin is published through the `nicholls` marketplace:

```bash
claude plugin marketplace add nicholls-inc/claude-code-marketplace
claude plugin install crosscheck@nicholls
```

See [`crosscheck/README.md`](crosscheck/README.md) for prerequisites. See [`cgv/README.md`](cgv/README.md) to build and run CGV.

## Repository layout

- `docs/VISION.md` is the vision the whole suite shares.
- `docs/assurance/`, `docs/gates/`, `docs/decisions/`, `intent/`, `.assurance/`, `evals/`, `REVIEW.md`, `JOURNAL.md`, and `plan.md` hold the development framework and its records.
- `formal-verification/` holds the specs and fixtures that the Lean and Dafny pipelines are tested against.
- `logic-distribution/` holds the research behind `crosscheck/docs/research/logic-distribution-analysis.md`.
