package verifier

import (
	"os"
	"strings"
	"testing"
)

// crosscheckGraph builds the 3-node 2-hop chain: mcp_client(0) → dafny_verify(1) → run_dafny(2).
// intermediateType controls the inputType on the second edge (hop 1 → 2),
// allowing callers to inject a mismatch without duplicating the graph setup.
func crosscheckGraph(intermediateType string) *GraphFile {
	return &GraphFile{
		Nodes: []Node{
			{ID: 0, Name: "mcp_client"},
			{ID: 1, Name: "dafny_verify"},
			{ID: 2, Name: "run_dafny"},
		},
		Edges: []Edge{
			{
				From:     0,
				To:       1,
				Contract: Contract{InputType: "MCPToolCall", OutputType: "VerifyInput"},
			},
			{
				From:     1,
				To:       2,
				Contract: Contract{InputType: intermediateType, OutputType: "DockerResult"},
			},
		},
		Paths: [][]int{{0, 1, 2}},
	}
}

// TestVerifyPath_CrosscheckChain_Compatible is the primary acceptance test (AC-2, AC-5).
// It MUST fail if VerifyPath is removed or its logic is silenced — if this test passes
// on a no-op VerifyPath, the verify logic has been stripped.
func TestVerifyPath_CrosscheckChain_Compatible(t *testing.T) {
	g := crosscheckGraph("VerifyInput") // matches edge(0→1).OutputType
	result := VerifyPath(g, []int{0, 1, 2})

	if result.Kind != ContractCompatible {
		t.Fatalf("expected ContractCompatible, got: %s", result.Message)
	}
}

// TestVerifyPath_HopTypeMismatch_HopIndex1 covers AC-3: corrupted YAML produces
// HopTypeMismatch(hopIndex: 1).
func TestVerifyPath_HopTypeMismatch_HopIndex1(t *testing.T) {
	g := crosscheckGraph("WrongInput") // deliberate mismatch — does not match "VerifyInput"
	result := VerifyPath(g, []int{0, 1, 2})

	if result.Kind != HopTypeMismatch {
		t.Fatalf("expected HopTypeMismatch, got: %s", result.Message)
	}
	if result.HopIndex != 1 {
		t.Fatalf("expected hopIndex 1, got %d", result.HopIndex)
	}
}

// TestVerifyPath_InvalidPath_TooShort checks that a 1-node path is rejected.
func TestVerifyPath_InvalidPath_TooShort(t *testing.T) {
	g := crosscheckGraph("VerifyInput")
	result := VerifyPath(g, []int{0})

	if result.Kind != InvalidPath {
		t.Fatalf("expected InvalidPath for 1-node path, got: %s", result.Message)
	}
}

// TestVerifyPath_InvalidPath_MissingEdge checks that a path with no connecting edge yields InvalidPath.
func TestVerifyPath_InvalidPath_MissingEdge(t *testing.T) {
	g := crosscheckGraph("VerifyInput")
	result := VerifyPath(g, []int{0, 2}) // no direct edge from 0 to 2

	if result.Kind != InvalidPath {
		t.Fatalf("expected InvalidPath for missing edge, got: %s", result.Message)
	}
}

// TestVerifyPath_InvalidPath_UnknownNode checks that referencing a nonexistent node yields InvalidPath.
func TestVerifyPath_InvalidPath_UnknownNode(t *testing.T) {
	g := crosscheckGraph("VerifyInput")
	result := VerifyPath(g, []int{0, 99})

	if result.Kind != InvalidPath {
		t.Fatalf("expected InvalidPath for unknown node 99, got: %s", result.Message)
	}
	if !strings.Contains(result.Message, "99") {
		t.Errorf("expected message to mention node 99, got: %s", result.Message)
	}
}

// TestVerifyPath_TwoNodePath_VacuouslyCompatible verifies the single-hop case.
// A 2-node path has no intermediate boundary — always ContractCompatible.
func TestVerifyPath_TwoNodePath_VacuouslyCompatible(t *testing.T) {
	g := &GraphFile{
		Nodes: []Node{{ID: 0, Name: "a"}, {ID: 1, Name: "b"}},
		Edges: []Edge{
			{From: 0, To: 1, Contract: Contract{InputType: "X", OutputType: "Y"}},
		},
		Paths: [][]int{{0, 1}},
	}
	result := VerifyPath(g, []int{0, 1})
	if result.Kind != ContractCompatible {
		t.Fatalf("2-node path should be vacuously compatible, got: %s", result.Message)
	}
}

// TestVerifyPath_ThreeHop_MismatchAtSecondBoundary checks hopIndex for 4-node paths.
func TestVerifyPath_ThreeHop_MismatchAtSecondBoundary(t *testing.T) {
	g := &GraphFile{
		Nodes: []Node{
			{ID: 0, Name: "a"},
			{ID: 1, Name: "b"},
			{ID: 2, Name: "c"},
			{ID: 3, Name: "d"},
		},
		Edges: []Edge{
			{From: 0, To: 1, Contract: Contract{InputType: "T0", OutputType: "T1"}},
			{From: 1, To: 2, Contract: Contract{InputType: "T1", OutputType: "T2"}},    // boundary 1: OK
			{From: 2, To: 3, Contract: Contract{InputType: "WRONG", OutputType: "T3"}}, // boundary 2: mismatch
		},
		Paths: [][]int{{0, 1, 2, 3}},
	}

	result := VerifyPath(g, []int{0, 1, 2, 3})
	if result.Kind != HopTypeMismatch {
		t.Fatalf("expected HopTypeMismatch, got: %s", result.Message)
	}
	if result.HopIndex != 2 {
		t.Fatalf("expected hopIndex 2, got %d", result.HopIndex)
	}
}

// TestVerifyGraph_MultiplePaths verifies VerifyGraph processes all declared paths.
func TestVerifyGraph_MultiplePaths(t *testing.T) {
	g := &GraphFile{
		Nodes: []Node{
			{ID: 0, Name: "a"},
			{ID: 1, Name: "b"},
			{ID: 2, Name: "c"},
		},
		Edges: []Edge{
			{From: 0, To: 1, Contract: Contract{InputType: "T1", OutputType: "T2"}},
			{From: 1, To: 2, Contract: Contract{InputType: "T2", OutputType: "T3"}},
		},
		Paths: [][]int{
			{0, 1, 2}, // compatible
			{0, 1},    // vacuously compatible
		},
	}

	results := VerifyGraph(g)
	if len(results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(results))
	}
	for _, r := range results {
		if r.Result.Kind != ContractCompatible {
			t.Errorf("path %v: expected ContractCompatible, got: %s", r.Path, r.Result.Message)
		}
	}
}

// --- ParseGraphFile integration tests ---

func TestParseGraphFile_Valid(t *testing.T) {
	f := writeTempYAML(t, `
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
`)

	g, err := ParseGraphFile(f)
	if err != nil {
		t.Fatalf("unexpected parse error: %v", err)
	}
	if len(g.Nodes) != 3 {
		t.Errorf("expected 3 nodes, got %d", len(g.Nodes))
	}
	if len(g.Edges) != 2 {
		t.Errorf("expected 2 edges, got %d", len(g.Edges))
	}
}

// TestParseGraphFile_SelfLoop checks that graphs with self-loops are rejected at parse time.
func TestParseGraphFile_SelfLoop(t *testing.T) {
	f := writeTempYAML(t, `
nodes:
  - id: 0
    name: a
edges:
  - from: 0
    to: 0
    contract:
      inputType: X
      outputType: Y
paths:
  - [0, 0]
`)

	_, err := ParseGraphFile(f)
	if err == nil {
		t.Fatal("expected error for self-loop, got nil")
	}
}

func writeTempYAML(t *testing.T, content string) string {
	t.Helper()
	dir := t.TempDir()
	path := dir + "/graph.yaml"
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("writeTempYAML: %v", err)
	}
	return path
}
