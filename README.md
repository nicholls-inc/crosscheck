# Contract Graph Verifier (CGV)

Layer 3 of the Six-Layer Assurance Hierarchy. Verifies interface contracts at
integration boundaries across module subgraphs.

## What it does

Given a YAML contract graph, CGV checks that every declared path through the
graph is end-to-end contract-compatible: the output type of each hop matches
the input type of the next hop at every intermediate boundary.

This catches composition failures that pairwise unit contracts cannot detect —
two modules can each satisfy their own contracts while producing emergent type
mismatches when assembled.

## Usage

```
cgv verify --graph <file.yaml>
```

Exit 0 if all declared paths are `ContractCompatible`. Exit 1 on the first
`HopTypeMismatch` or `InvalidPath`. Exit 2 on usage/file errors.

## Graph file format

```yaml
nodes:
  - id: 0
    name: mcp_client
  - id: 1
    name: dafny_verify
  - id: 2
    name: run_dafny

edges:
  - from: 0
    to: 1
    contract:
      inputType: MCPToolCall
      outputType: VerifyInput
  - from: 1
    to: 2
    contract:
      inputType: VerifyInput
      outputType: DockerResult

paths:
  - [0, 1, 2]
```

## Assurance hierarchy compliance

- **Layer 1:** Formal spec in `specs/CGVCore.dfy` travels with the binary.
  `CompatibilityIsSequential` lemma is fully proved.
  `VerifyPath` postconditions are specified; body admits `assume false` (Phase 1).
- **Layer 3:** This binary is the Layer 3 verifier.
- **Layer 4:** Go implementation mirrors the Dafny spec types and predicates directly.

## CI

`go test ./...` + `go run ./cmd/cgv verify --graph crosscheck-cgv.yaml` run on every push.
Dafny syntax check runs in a separate job.
