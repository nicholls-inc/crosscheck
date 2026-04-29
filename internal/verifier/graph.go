// Package verifier implements the Contract Graph Verifier engine.
// Data types mirror the Dafny formal model in specs/CGVCore.dfy.
package verifier

import (
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

// Contract is the typed interface boundary between two nodes.
// String-typed for Phase 1; Phase 2 will introduce structural subtyping.
type Contract struct {
	InputType  string `yaml:"inputType"`
	OutputType string `yaml:"outputType"`
}

// Edge is a directed, labelled arc between two node IDs.
type Edge struct {
	From     int      `yaml:"from"`
	To       int      `yaml:"to"`
	Contract Contract `yaml:"contract"`
}

// Node is a named vertex in the contract graph.
type Node struct {
	ID   int    `yaml:"id"`
	Name string `yaml:"name"`
}

// GraphFile is the top-level YAML document.
// Nodes are referenced by integer ID in edges and paths.
// Paths lists the sequences of node IDs to verify end-to-end.
type GraphFile struct {
	Nodes []Node  `yaml:"nodes"`
	Edges []Edge  `yaml:"edges"`
	Paths [][]int `yaml:"paths"`
}

// nodeCount returns the number of distinct node IDs in the graph.
// Node IDs must be 0-based and contiguous for the ValidGraph check.
func (g *GraphFile) nodeCount() int {
	max := -1
	for _, n := range g.Nodes {
		if n.ID > max {
			max = n.ID
		}
	}
	return max + 1
}

// hopContract returns the contract on arc (from→to), or an error if no such edge exists.
// When multiple edges share the same endpoints, the first declared edge wins (Phase 1).
func (g *GraphFile) hopContract(from, to int) (Contract, bool) {
	for _, e := range g.Edges {
		if e.From == from && e.To == to {
			return e.Contract, true
		}
	}
	return Contract{}, false
}

// validateStructure checks that all edge endpoints are valid node IDs and there are no self-loops.
func (g *GraphFile) validateStructure() error {
	n := g.nodeCount()
	nodeSet := make(map[int]bool, n)
	for _, nd := range g.Nodes {
		nodeSet[nd.ID] = true
	}
	for i, e := range g.Edges {
		if !nodeSet[e.From] {
			return fmt.Errorf("edge[%d]: unknown from-node id %d", i, e.From)
		}
		if !nodeSet[e.To] {
			return fmt.Errorf("edge[%d]: unknown to-node id %d", i, e.To)
		}
		if e.From == e.To {
			return fmt.Errorf("edge[%d]: self-loop on node %d", i, e.From)
		}
	}
	return nil
}

// ParseGraphFile reads and validates a YAML contract graph file.
func ParseGraphFile(path string) (*GraphFile, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("reading graph file: %w", err)
	}
	var g GraphFile
	if err := yaml.Unmarshal(data, &g); err != nil {
		return nil, fmt.Errorf("parsing YAML: %w", err)
	}
	if len(g.Nodes) == 0 {
		return nil, fmt.Errorf("graph must declare at least one node")
	}
	if len(g.Paths) == 0 {
		return nil, fmt.Errorf("graph must declare at least one path to verify")
	}
	if err := g.validateStructure(); err != nil {
		return nil, fmt.Errorf("invalid graph structure: %w", err)
	}
	return &g, nil
}
