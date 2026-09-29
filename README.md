# Crosscheck

Tools for verifying AI-generated code. The goal is to know that AI-generated code is correct without trusting an AI to say so. [`docs/VISION.md`](docs/VISION.md) states the vision.

## Tools

- [`cgv/`](cgv/README.md) is the contract graph verifier. It extracts contracts from Python code (Django models and plain-Python data classes) and checks them across component boundaries. Its checker has machine-checked soundness proofs in Lean.
