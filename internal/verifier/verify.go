package verifier

import "fmt"

// Result mirrors the VerifyResult datatype from specs/CGVCore.dfy.
type ResultKind int

const (
	ContractCompatible ResultKind = iota
	HopTypeMismatch
	InvalidPath
)

// VerifyResult is the outcome of verifying a single path through a contract graph.
type VerifyResult struct {
	Kind     ResultKind
	HopIndex int    // valid only when Kind == HopTypeMismatch
	Message  string // human-readable description
}

func (r VerifyResult) String() string {
	return r.Message
}

// Exit returns the process exit code: 0 for ContractCompatible, 1 otherwise.
func (r VerifyResult) Exit() int {
	if r.Kind == ContractCompatible {
		return 0
	}
	return 1
}

// PathResult pairs a path description with its verification outcome.
type PathResult struct {
	PathIndex int
	Path      []int
	Result    VerifyResult
}

// VerifyPath checks end-to-end contract compatibility for a single path through g.
//
// Mirrors the postconditions of VerifyPath in specs/CGVCore.dfy:
//   - an invalid path (missing edges or out-of-bounds nodes) yields InvalidPath
//   - a fully compatible path yields ContractCompatible
//   - the first incompatible hop yields HopTypeMismatch with hopIndex = k+1
//     (the 1-based index of the downstream hop in the failing pair)
//
// hopIndex definition: for path [n0, n1, n2, …], hop k goes from n[k] to n[k+1].
// When edge(n[k]→n[k+1]).outputType ≠ edge(n[k+1]→n[k+2]).inputType, we report
// HopTypeMismatch(hopIndex: k+1) — the index of the downstream hop in the failing pair.
func VerifyPath(g *GraphFile, path []int) VerifyResult {
	nodeSet := make(map[int]bool, len(g.Nodes))
	for _, n := range g.Nodes {
		nodeSet[n.ID] = true
	}

	if len(path) < 2 {
		return VerifyResult{
			Kind:    InvalidPath,
			Message: "InvalidPath: path must contain at least 2 nodes",
		}
	}

	for _, id := range path {
		if !nodeSet[id] {
			return VerifyResult{
				Kind:    InvalidPath,
				Message: fmt.Sprintf("InvalidPath: node %d is not in the graph", id),
			}
		}
	}

	for k := 0; k < len(path)-1; k++ {
		if _, ok := g.hopContract(path[k], path[k+1]); !ok {
			return VerifyResult{
				Kind:    InvalidPath,
				Message: fmt.Sprintf("InvalidPath: no edge from node %d to node %d", path[k], path[k+1]),
			}
		}
	}

	// Check pairwise compatibility at each intermediate boundary.
	// For each pair of consecutive hops (k, k+1), the upstream edge's outputType
	// must equal the downstream edge's inputType.
	for k := 0; k < len(path)-2; k++ {
		upstream, _ := g.hopContract(path[k], path[k+1])
		downstream, _ := g.hopContract(path[k+1], path[k+2])
		if upstream.OutputType != downstream.InputType {
			hopIndex := k + 1
			return VerifyResult{
				Kind:     HopTypeMismatch,
				HopIndex: hopIndex,
				Message: fmt.Sprintf(
					"HopTypeMismatch(hopIndex: %d): hop %d output %q ≠ hop %d input %q",
					hopIndex,
					k, upstream.OutputType,
					hopIndex, downstream.InputType,
				),
			}
		}
	}

	return VerifyResult{
		Kind:    ContractCompatible,
		Message: "ContractCompatible",
	}
}

// VerifyGraph verifies all paths declared in the GraphFile and returns one result per path.
func VerifyGraph(g *GraphFile) []PathResult {
	results := make([]PathResult, 0, len(g.Paths))
	for i, path := range g.Paths {
		results = append(results, PathResult{
			PathIndex: i,
			Path:      path,
			Result:    VerifyPath(g, path),
		})
	}
	return results
}
