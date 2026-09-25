-- StateSearch.lean
--
-- State-based checking: every composed hop is checked once, however many
-- paths reach it.
--
-- `checkPath` checks the hops `u₀ = e₀`, `uᵢ₊₁ = stepEdge uᵢ eᵢ₊₁` of a path.
-- A hop's results (`checkHop uᵢ`) and its successors (`stepEdge uᵢ n`)
-- depend only on the composed hop `uᵢ` itself, not on how it was reached, so
-- the checker explores the set of composed hops (the *states*) reachable from
-- the first edges of paths, instead of the paths. The number of states is
-- about the number of edges times the number of distinct composed
-- postconditions per edge, which differ only past nodes with dependent
-- (`input_*`) postconditions; the number of paths is exponential in the
-- depth of the graph.
--
-- States are normalised (`normHop`: the source's preconditions dropped;
-- neither `checkHop` nor `stepEdge` reads them), so hops into the same node
-- over different incoming edges (per-parameter edges) share their states.
--
-- The exploration (`explore`) is a breadth-first worklist over walks, not
-- simple paths: it may go round a cycle, which over-approximates the data
-- paths (every simple path is a walk). It is not verified. Instead its result
-- is checked (`closedStates`): every first hop is a state, and the successors
-- of every state are states. From that check alone, every hop of every data
-- path is a state (`closedStates_checkPath`), so a run without errors on the
-- states is a run without errors on every data path
-- (`runChecker_sound_all`, Main.lean).
--
-- A cycle through a dependent postcondition (e.g. `add(input_precision, 1)`)
-- can produce new states forever; the exploration stops past
-- `maxPerEdge` states on one edge or `maxStates` states in total, and the run
-- is reported incomplete.

import Std.Data.HashMap
import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition
import ContractGraph.Search

namespace ContractGraph

open Std

/-! ## Equality and hashing of hops (used by the state table) -/

deriving instance DecidableEq for DepExpr
deriving instance DecidableEq for VerificationLevel
deriving instance DecidableEq for Constraint
deriving instance DecidableEq for Node
deriving instance DecidableEq for Relationship
deriving instance DecidableEq for Edge
deriving instance Hashable for DepExpr
deriving instance Hashable for Constraint
deriving instance Hashable for Node
deriving instance Hashable for Relationship
deriving instance Hashable for Edge

/-! ## States -/

/-- A hop with its source's preconditions dropped: neither `checkHop` nor
    `stepEdge` reads them (`checkHop_normHop`, `stepEdge_normHop`). -/
def normHop (u : Edge) : Edge := { u with source := { u.source with preconditions := [] } }

theorem checkHop_normHop (u : Edge) : checkHop (normHop u) = checkHop u := rfl

theorem stepEdge_normHop (u n : Edge) : stepEdge (normHop u) n = stepEdge u n := rfl

theorem normHop_target (u : Edge) : (normHop u).target = u.target := rfl

/-- One explored state: the normalised composed hop, its predecessor (index
    + 1 in the state array; 0 for a first hop) and the raw graph edge of the
    hop (for the witness path). -/
structure StateRec where
  hop : Edge
  pred : Nat
  raw : Edge

/-- The result of a finished exploration. -/
structure Explored where
  recs : Array StateRec
  /-- Normalised hop ↦ its index in `recs`. -/
  index : HashMap Edge Nat

/-- Outcome of the exploration. -/
inductive ExploreOutcome where
  | done (ex : Explored)
  /-- More than `maxStates` states in total. -/
  | tooMany (ex : Explored)
  /-- More than `maxPerEdge` states on one edge; the state that exceeded
      it (not added) and its predecessor's index + 1. -/
  | capped (ex : Explored) (hop : Edge) (pred : Nat) (raw : Edge)

/-- Outgoing checked edges into kept nodes, by source id, with each edge's
    position in `checkedEdges` (unverified; the closure check uses
    `SearchSetup.out`). -/
def buildOutIndexed (keep : Nat → Bool) (edges : List Edge) : HashMap Nat (Array (Nat × Edge)) :=
  (edges.foldl (fun (acc : Nat × HashMap Nat (Array (Nat × Edge))) e =>
    let m := if keep e.target.id then
        acc.2.insert e.source.id ((acc.2.getD e.source.id #[]).push (acc.1, e))
      else acc.2
    (acc.1 + 1, m)) (0, ∅)).2

/-- Everything the exploration needs, computed once per graph. -/
structure StateSetup where
  setup : SearchSetup
  outI : HashMap Nat (Array (Nat × Edge))
  numEdges : Nat
  /-- Shortest distance (in edges) from a node id to a model node over the
      followed edges (for witness paths). -/
  dist : HashMap Nat Nat

/-- Backward breadth-first distances to the model nodes over `edges`. -/
def modelDistances (modelIds : List Nat) (edges : List Edge) : HashMap Nat Nat := Id.run do
  let rev := reverseIndex edges
  let mut dist : HashMap Nat Nat := ∅
  let mut queue : Array Nat := #[]
  for m in modelIds do
    if !dist.contains m then
      dist := dist.insert m 0
      queue := queue.push m
  let mut head := 0
  while h : head < queue.size do
    let v := queue[head]
    let d := dist.getD v 0
    for u in rev.getD v [] do
      if !dist.contains u then
        dist := dist.insert u (d + 1)
        queue := queue.push u
    head := head + 1
  return dist

def stateSetup (g : ContractGraph) : StateSetup :=
  let edges := checkedEdges g.edges
  let modelIds := modelNodeIds g
  let keep := keepOf (mkKeepSet modelIds edges)
  let kept := edges.filter (fun e => keep e.target.id)
  { setup := searchSetup g
    outI := buildOutIndexed keep edges
    numEdges := edges.length
    dist := modelDistances modelIds kept }

/-- Breadth-first exploration of the states reachable from the first hops
    (every followed edge out of a function node), stopping past `maxStates`
    states or past `maxPerEdge` states on one edge. Unverified: its result is
    checked by `closedStates`. -/
def explore (st : StateSetup) (g : ContractGraph) (maxStates maxPerEdge : Nat) :
    ExploreOutcome := Id.run do
  let mut recs : Array StateRec := #[]
  let mut index : HashMap Edge Nat := ∅
  let mut perEdge : Array Nat := Array.replicate st.numEdges 0
  for src in g.nodes do
    if src.kind == "function" then
      for (k, e) in st.outI.getD src.id #[] do
        let x := normHop e
        if !index.contains x then
          if perEdge[k]! + 1 > maxPerEdge then
            return .capped ⟨recs, index⟩ x 0 e
          if recs.size + 1 > maxStates then
            return .tooMany ⟨recs, index⟩
          perEdge := perEdge.modify k (· + 1)
          index := index.insert x recs.size
          recs := recs.push ⟨x, 0, e⟩
  let mut head := 0
  while h : head < recs.size do
    let w := recs[head].hop
    for (k, n) in st.outI.getD w.target.id #[] do
      let x := normHop (stepEdge w n)
      if !index.contains x then
        if perEdge[k]! + 1 > maxPerEdge then
          return .capped ⟨recs, index⟩ x (head + 1) n
        if recs.size + 1 > maxStates then
          return .tooMany ⟨recs, index⟩
        perEdge := perEdge.modify k (· + 1)
        index := index.insert x recs.size
        recs := recs.push ⟨x, head + 1, n⟩
    head := head + 1
  return .done ⟨recs, index⟩

/-! ## The closure check -/

/-- Whether `x` is a state of `ex` (looked up in the index, confirmed in the
    array). -/
def memStates (ex : Explored) (x : Edge) : Bool :=
  match ex.index[x]? with
  | some i =>
    match ex.recs[i]? with
    | some r => decide (r.hop = x)
    | none => false
  | none => false

theorem memStates_sound (ex : Explored) (x : Edge) (h : memStates ex x = true) :
    ∃ r ∈ ex.recs.toList, r.hop = x := by
  unfold memStates at h
  split at h
  · split at h
    · rename_i r hr
      refine ⟨r, ?_, of_decide_eq_true h⟩
      exact List.mem_of_getElem? (by simpa using hr)
    · cases h
  · cases h

/-- The function nodes of `g`: the sources of paths. -/
def functionNodes (g : ContractGraph) : List Node := g.nodes.filter (·.kind == "function")

/-- The explored states are closed: every first hop (a followed edge out of a
    function node) is a state, and every successor of a state is one. -/
def closedStates (s : SearchSetup) (g : ContractGraph) (ex : Explored) : Bool :=
  (functionNodes g).all (fun src => (s.out src.id).all fun e => memStates ex (normHop e)) &&
  ex.recs.toList.all (fun r => (s.out r.hop.target.id).all fun n =>
    memStates ex (normHop (stepEdge r.hop n)))

/-! ## Every hop of every data path is a state -/

/-- Along a chain of followed edges from a state, every `checkPath` result
    is a result of some state's `checkHop`. -/
theorem checkPath_mem_states (s : SearchSetup) (g : ContractGraph) (ex : Explored)
    (hc : closedStates s g ex = true) :
    ∀ (rest : List Edge) (u : Edge), (∃ r ∈ ex.recs.toList, r.hop = normHop u) →
      ChainFrom u.target.id rest → (∀ e ∈ rest, e ∈ s.out e.source.id) →
      ∀ x ∈ checkPath (u :: rest), ∃ r ∈ ex.recs.toList, x ∈ checkHop r.hop := by
  simp only [closedStates, Bool.and_eq_true, List.all_eq_true] at hc
  intro rest
  induction rest with
  | nil =>
    intro u ⟨r, hr, hru⟩ _ _ x hx
    rw [checkPath_single] at hx
    exact ⟨r, hr, by rw [hru, checkHop_normHop]; exact hx⟩
  | cons n rest ih =>
    intro u ⟨r, hr, hru⟩ hchain hout x hx
    simp only [ChainFrom] at hchain
    rw [checkPath_cons_cons] at hx
    rcases List.mem_append.mp hx with hx | hx
    · exact ⟨r, hr, by rw [hru, checkHop_normHop]; exact hx⟩
    · have hn : n ∈ s.out r.hop.target.id := by
        rw [hru, normHop_target, ← hchain.1]
        exact hout n List.mem_cons_self
      have hmem := hc.2 r hr n hn
      rw [hru, stepEdge_normHop] at hmem
      exact ih (stepEdge u n) (memStates_sound ex _ hmem) hchain.2
        (fun e he => hout e (List.mem_cons_of_mem _ he)) x hx

/-- The edges of a data path are followed by the search, and its first
    edge leaves a function node. -/
theorem IsDataPath.followed {g : ContractGraph} {p : List Edge} (h : IsDataPath g p) :
    (∀ e ∈ p, e ∈ (searchSetup g).out e.source.id) ∧
    ∃ src ∈ functionNodes g, ChainFrom src.id p := by
  obtain ⟨_, hedges, src, hsrc, tgt, htgt, hsk, htk, hchain, hend, _⟩ := h
  have hmodel : endId src.id p ∈ modelNodeIds g := by
    rw [hend]
    exact List.mem_map.mpr ⟨tgt, List.mem_filter.mpr ⟨htgt, by simp [htk]⟩, rfl⟩
  have hkeep := Closed.keep_nodeIds (mkKeep_closed (modelNodeIds g) (checkedEdges g.edges)) p src.id
    hedges hchain ((mkKeep_closed _ _).1 _ hmodel)
  refine ⟨fun e he => ?_, src, List.mem_filter.mpr ⟨hsrc, by simp [hsk]⟩, hchain⟩
  rw [searchSetup_out]
  refine List.mem_filter.mpr ⟨hedges e he, ?_⟩
  simp only [beq_self_eq_true, Bool.true_and]
  exact hkeep _ (mem_nodeIds_target _ _ hchain e he)

/-- KEY LEMMA. If the explored states pass the closure check, every result
    of `checkPath` on every data path is a result of `checkHop` on some
    explored state. -/
theorem closedStates_checkPath (g : ContractGraph) (ex : Explored)
    (hc : closedStates (searchSetup g) g ex = true) (p : List Edge) (hp : IsDataPath g p) :
    ∀ x ∈ checkPath p, ∃ r ∈ ex.recs.toList, x ∈ checkHop r.hop := by
  obtain ⟨hout, src, hsrc, hchain⟩ := hp.followed
  match p, hp.1, hchain with
  | e :: rest, _, hchain =>
    simp only [ChainFrom] at hchain
    have hc' := hc
    simp only [closedStates, Bool.and_eq_true, List.all_eq_true] at hc'
    have he : e ∈ (searchSetup g).out src.id := hchain.1 ▸ hout e List.mem_cons_self
    exact checkPath_mem_states _ g ex hc rest e (memStates_sound ex _ (hc'.1 src hsrc e he))
      hchain.2 (fun e' he' => hout e' (List.mem_cons_of_mem _ he'))

/-! ## Witness paths -/

/-- The raw edges from a first hop to the state `r` (following predecessors). -/
def prefixOf (recs : Array StateRec) (r : StateRec) : List Edge :=
  go r.pred [r.raw] recs.size
where
  go : Nat → List Edge → Nat → List Edge
    | 0, acc, _ => acc
    | _, acc, 0 => acc
    | p + 1, acc, fuel + 1 =>
      match recs[p]? with
      | some q => go q.pred (q.raw :: acc) fuel
      | none => acc

/-- A shortest continuation from node id `v` to a model node (the first
    followed edge, in edge order, that gets closer). -/
def continuationFrom (st : StateSetup) : Nat → Nat → List Edge
  | 0, _ => []
  | fuel + 1, v =>
    match st.dist[v]? with
    | some (d + 1) =>
      match (st.outI.getD v #[]).find? (fun (_, e) => st.dist[e.target.id]? == some d) with
      | some (_, e) => e :: continuationFrom st fuel e.target.id
      | none => []
    | _ => []

/-- The witness data path reported for a state: the shortest walk found to
    it, then a shortest continuation to a model node. -/
def witnessOf (st : StateSetup) (ex : Explored) (r : StateRec) : List Edge :=
  prefixOf ex.recs r ++ continuationFrom st (st.numEdges + 1) r.hop.target.id

end ContractGraph
