// CGVCore.dfy
// Layer 1 formal specification — Contract Graph Verifier core traversal logic.
// Status: spec skeleton. VerifyPath body uses `assume false` (Phase 1 impl target).
// Mandate 1 compliance: syntactically valid Dafny; no full proofs required at this stage.
//
// This spec travels with the binary per the proof-carrying code principle.
// The Go implementation in internal/verifier/ mirrors these types and predicates.

module CGVCore {

  // Option datatype (Dafny standard library not assumed in all target configs).
  datatype Option<T> = None | Some(value: T)

  // A typed interface boundary — the atomic unit the CGV reasons about.
  // Contracts are intentionally minimal: a string-encoded input type and output type.
  // Phase 2 will replace strings with a richer type algebra.
  datatype Contract = Contract(inputType: string, outputType: string)

  // A directed edge between two nodes (by index), labelled with its interface contract.
  datatype Edge = Edge(from: nat, to: nat, contract: Contract)

  // A contract graph: a node count and a sequence of directed, labelled edges.
  // Nodes are referenced by index; names live outside the formal model.
  datatype Graph = Graph(nodeCount: nat, edges: seq<Edge>)

  // Well-formedness: all edge endpoints are valid node indices; no self-loops.
  predicate ValidGraph(g: Graph) {
    forall i :: 0 <= i < |g.edges| ==>
      g.edges[i].from < g.nodeCount &&
      g.edges[i].to   < g.nodeCount &&
      g.edges[i].from != g.edges[i].to
  }

  // Two adjacent contracts are compatible when the upstream output type
  // equals the downstream input type exactly.
  predicate ContractsCompatible(upstream: Contract, downstream: Contract) {
    upstream.outputType == downstream.inputType
  }

  // Retrieve the first matching edge contract on the arc (from, to), if any.
  // When multiple edges exist between the same pair, only the first is considered.
  // Phase 2 will handle multi-edge graphs (overloaded interfaces).
  function HopContract(g: Graph, from: nat, to: nat): Option<Contract> {
    if (exists i :: 0 <= i < |g.edges| &&
        g.edges[i].from == from &&
        g.edges[i].to   == to)
    then
      var i :| 0 <= i < |g.edges| &&
               g.edges[i].from == from &&
               g.edges[i].to   == to;
      Some(g.edges[i].contract)
    else
      None
  }

  // A path is valid when:
  //   1. It has at least two nodes (otherwise there are no boundaries to check).
  //   2. All node indices are within bounds.
  //   3. Every consecutive pair has at least one connecting edge.
  predicate ValidPath(g: Graph, path: seq<nat>)
    requires ValidGraph(g)
  {
    |path| >= 2 &&
    (forall k :: 0 <= k < |path|     ==> path[k] < g.nodeCount) &&
    (forall k :: 0 <= k < |path| - 1 ==> HopContract(g, path[k], path[k+1]).Some?)
  }

  // A path is end-to-end contract-compatible when every pair of consecutive hops
  // has matching output/input types at the intermediate node.
  // A 2-node path (single edge) is vacuously compatible.
  predicate PathContractCompatible(g: Graph, path: seq<nat>)
    requires ValidGraph(g)
    requires ValidPath(g, path)
  {
    forall k :: 0 <= k < |path| - 2 ==>
      HopContract(g, path[k],     path[k+1]).Some? &&
      HopContract(g, path[k+1],   path[k+2]).Some? &&
      ContractsCompatible(
        HopContract(g, path[k],   path[k+1]).value,
        HopContract(g, path[k+1], path[k+2]).value
      )
  }

  datatype VerifyResult =
    | ContractCompatible
    | HopTypeMismatch(hopIndex: nat)
    | InvalidPath

  // Core verifier: traverses a path through a contract graph and checks
  // end-to-end contract compatibility at every intermediate boundary.
  //
  // Postconditions establish the determinism guarantee:
  //   - an invalid path always yields InvalidPath
  //   - a compatible path always yields ContractCompatible
  // The HopTypeMismatch branch is deliberately unconstrained at spec level;
  // the implementation must determine hopIndex from the first failing hop.
  method VerifyPath(g: Graph, path: seq<nat>) returns (result: VerifyResult)
    requires ValidGraph(g)
    ensures !ValidPath(g, path) ==> result == InvalidPath
    ensures ValidPath(g, path) && PathContractCompatible(g, path) ==>
              result == ContractCompatible
  {
    assume false; // Phase 1 implementation target — body proved in Phase 2
    result := InvalidPath;
  }

  // Lemma: compatibility is sequential, not transitive.
  //
  // If a→b is compatible and b→c is compatible, the types satisfy:
  //   a.outputType == b.inputType  AND  b.outputType == c.inputType
  // but a.outputType need not equal c.inputType (b.inputType ≠ b.outputType in general).
  //
  // This is the formal justification for why end-to-end subgraph verification
  // cannot be reduced to pairwise transitive closure of compatibility:
  // each hop consumes the output of the previous hop via the intermediate node's type.
  lemma CompatibilityIsSequential(a: Contract, b: Contract, c: Contract)
    requires ContractsCompatible(a, b)
    requires ContractsCompatible(b, c)
    ensures a.outputType == b.inputType
    ensures b.outputType == c.inputType
  {
    // Follows directly from the definition of ContractsCompatible.
  }

}
