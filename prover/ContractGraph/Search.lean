-- Search.lean
--
-- Path enumeration and checking with shared prefixes.
--
-- One depth-first search per function node collects the paths to every model
-- node. The search carries the composed last hop (`stepEdge`) and the results
-- so far, so each prefix is composed and checked once, however many paths
-- extend it. It follows only edges into nodes that can reach a model node
-- (`SearchSetup.keep`), looked up in an index of outgoing edges built once.
--
-- Proved here: every emitted path's results are `checkPath` of that path
-- (`checkAllPaths_spec`), and every data path is emitted
-- (`checkAllPaths_complete`, `enumeratePaths_complete`). The reachability set
-- is computed by an unverified search and then checked to be closed under
-- the edges (`closedCheck`); if the check fails, pruning is switched off.

import Std.Data.HashMap
import Std.Data.HashSet
import ContractGraph.Types
import ContractGraph.Checker
import ContractGraph.Composition

namespace ContractGraph

open Std

/-! ## Composing a path one edge at a time -/

/-- The composed hop reached after `first` and then `mid`: `checkPath`'s
    `updatedEdge` for the last edge of `first :: mid`. -/
def lastHop (u : Edge) : List Edge → Edge
  | [] => u
  | n :: rest => lastHop (stepEdge u n) rest

theorem lastHop_append (u e : Edge) (mid : List Edge) :
    lastHop u (mid ++ [e]) = stepEdge (lastHop u mid) e := by
  induction mid generalizing u with
  | nil => rfl
  | cons n rest ih => exact ih (stepEdge u n)

theorem checkPath_single (e : Edge) : checkPath [e] = checkHop e := by
  rw [checkPath]

theorem checkPath_cons_cons (e n : Edge) (rest : List Edge) :
    checkPath (e :: n :: rest) = checkHop e ++ checkPath (stepEdge e n :: rest) := by
  rw [checkPath]

/-- `checkPath` of a path extended by one edge: the old results, then the
    new last hop's (composed through the whole prefix). -/
theorem checkPath_snoc (first e : Edge) (mid : List Edge) :
    checkPath (first :: (mid ++ [e])) =
      checkPath (first :: mid) ++ checkHop (stepEdge (lastHop first mid) e) := by
  induction mid generalizing first with
  | nil =>
    simp only [List.nil_append, checkPath_cons_cons, checkPath_single, lastHop]
  | cons m rest ih =>
    simp only [List.cons_append, checkPath_cons_cons, ih, lastHop, List.append_assoc]

/-! ## The search -/

/-- Search state after at least one edge: the composed last hop, the path so
    far (reversed) and its results so far (reversed). Reversed lists let
    every extension share its prefix. -/
abbrev PathState := Edge × List Edge × List CheckResult

/-- Extend the search state by edge `e`. -/
def advance (st : Option PathState) (e : Edge) : PathState :=
  match st with
  | none => (e, [e], (checkHop e).reverse)
  | some (u, rp, rr) =>
    let u' := stepEdge u e
    (u', e :: rp, (checkHop u').reverse ++ rr)

/-- The reversed path of a search state (`[]` before the first edge). -/
def revPathOf : Option PathState → List Edge
  | none => []
  | some (_, rp, _) => rp

theorem advance_revPath (st : Option PathState) (e : Edge) :
    (advance st e).2.1 = e :: revPathOf st := by
  cases st with
  | none => rfl
  | some s => obtain ⟨u, rp, rr⟩ := s; rfl

/-- Depth-first search from node id `v`, never entering an id in `v ::
    visited`. Emits `(reversed path, reversed results)` for every path that
    ends at a node with `isModel`, and continues through it. `out v` lists
    the edges followed from `v`. Structural recursion on `fuel`, the maximum
    number of edges on a path. -/
def searchFrom (out : Nat → List Edge) (isModel : Nat → Bool) :
    (fuel : Nat) → (v : Nat) → (visited : List Nat) → Option PathState →
    List (List Edge × List CheckResult)
  | 0, _, _, _ => []
  | fuel + 1, v, visited, st =>
    let visited' := v :: visited
    (out v).flatMap fun e =>
      if visited'.contains e.target.id then [] else
        let st' := advance st e
        let here := if isModel e.target.id then [(st'.2.1, st'.2.2)] else []
        here ++ searchFrom out isModel fuel e.target.id visited' (some st')

/-! ### Emitted results are `checkPath` of the emitted path -/

/-- The search-state invariant: the reversed path is `first :: mid`, the last
    hop is its composition, and the reversed results are its `checkPath`. -/
def StateInv : Option PathState → Prop
  | none => True
  | some (u, rp, rr) =>
    ∃ first mid, rp.reverse = first :: mid ∧ u = lastHop first mid ∧
      rr.reverse = checkPath (first :: mid)

theorem advance_inv (st : Option PathState) (e : Edge) (h : StateInv st) :
    StateInv (some (advance st e)) := by
  cases st with
  | none =>
    refine ⟨e, [], rfl, rfl, ?_⟩
    simp [checkPath_single]
  | some s =>
    obtain ⟨u, rp, rr⟩ := s
    obtain ⟨first, mid, hrp, hu, hrr⟩ := h
    refine ⟨first, mid ++ [e], ?_, ?_, ?_⟩
    · simp [hrp]
    · simp [hu, lastHop_append]
    · simp [hrr, checkPath_snoc, hu]

theorem StateInv.results {u : Edge} {rp : List Edge} {rr : List CheckResult}
    (h : StateInv (some (u, rp, rr))) : rr.reverse = checkPath rp.reverse := by
  obtain ⟨first, mid, hrp, _, hrr⟩ := h
  rw [hrp, hrr]

theorem searchFrom_spec (out : Nat → List Edge) (isModel : Nat → Bool) :
    ∀ (fuel v : Nat) (visited : List Nat) (st : Option PathState), StateInv st →
      ∀ x ∈ searchFrom out isModel fuel v visited st, x.2.reverse = checkPath x.1.reverse := by
  intro fuel
  induction fuel with
  | zero => intro v visited st _ x hx; simp [searchFrom] at hx
  | succ f ih =>
    intro v visited st hst x hx
    simp only [searchFrom] at hx
    obtain ⟨e, _, hx⟩ := List.mem_flatMap.mp hx
    split at hx
    · cases hx
    · have hinv := advance_inv st e hst
      rcases List.mem_append.mp hx with hh | hr
      · split at hh
        · rw [List.mem_singleton.mp hh]
          generalize advance st e = s at hinv ⊢
          obtain ⟨u, rp, rr⟩ := s
          exact StateInv.results hinv
        · cases hh
      · exact ih _ _ _ hinv x hr

/-! ### Completeness of the search -/

theorem mem_nodeIds_target (v : Nat) (p : List Edge) (h : ChainFrom v p) :
    ∀ e ∈ p, e.target.id ∈ nodeIds v p := by
  induction p generalizing v with
  | nil => intro e he; cases he
  | cons e rest ih =>
    intro e' he'
    simp only [ChainFrom] at h
    simp only [nodeIds]
    rcases List.mem_cons.mp he' with rfl | hr
    · apply List.mem_cons_of_mem
      cases rest with
      | nil => simp [nodeIds]
      | cons _ _ => simp [nodeIds]
    · exact List.mem_cons_of_mem _ (ih _ h.2 e' hr)

/-- Every simple chain from `v` that ends at an `isModel` node, uses only
    edges listed by `out`, avoids `visited` and has at most `fuel` edges, is
    emitted (extending the state's reversed path). -/
theorem mem_searchFrom (out : Nat → List Edge) (isModel : Nat → Bool) :
    ∀ (p : List Edge) (v : Nat) (visited : List Nat) (st : Option PathState) (fuel : Nat),
      p ≠ [] → (∀ e ∈ p, e ∈ out e.source.id) → ChainFrom v p →
      isModel (endId v p) = true → (nodeIds v p).Nodup →
      (∀ x ∈ nodeIds v p, x ∉ visited) → p.length ≤ fuel →
      ∃ rr, (p.reverse ++ revPathOf st, rr) ∈ searchFrom out isModel fuel v visited st := by
  intro p
  induction p with
  | nil => intro _ _ _ _ hne; exact absurd rfl hne
  | cons e tl ih =>
    intro v visited st fuel _ hout hchain hmodel hnodup hvis hlen
    obtain ⟨f, rfl⟩ : ∃ f, fuel = f + 1 := ⟨fuel - 1, by simp at hlen; omega⟩
    simp only [ChainFrom] at hchain
    have he : e ∈ out v := hchain.1 ▸ hout e List.mem_cons_self
    simp only [nodeIds] at hnodup hvis
    have ⟨hvnot, hnodup'⟩ := List.nodup_cons.mp hnodup
    have htgt_mem : e.target.id ∈ nodeIds e.target.id tl := by
      cases tl with
      | nil => simp [nodeIds]
      | cons _ _ => simp [nodeIds]
    have hne : e.target.id ≠ v := fun h => hvnot (h ▸ htgt_mem)
    have hnv : e.target.id ∉ visited := hvis _ (List.mem_cons_of_mem _ htgt_mem)
    have hcont : (v :: visited).contains e.target.id = false := by
      simp [hne, hnv]
    simp only [searchFrom]
    cases tl with
    | nil =>
      refine ⟨(advance st e).2.2, List.mem_flatMap.mpr ⟨e, he, ?_⟩⟩
      simp only [endId] at hmodel
      simp only [hcont, Bool.false_eq_true, if_false, hmodel, if_true]
      apply List.mem_append_left
      simp [advance_revPath]
    | cons e2 tl2 =>
      obtain ⟨rr, hrr⟩ := ih e.target.id (v :: visited) (some (advance st e)) f
        (List.cons_ne_nil _ _) (fun x hx => hout x (List.mem_cons_of_mem _ hx)) hchain.2
        hmodel hnodup'
        (fun x hx hmem => by
          rcases List.mem_cons.mp hmem with rfl | hv
          · exact hvnot hx
          · exact hvis x (List.mem_cons_of_mem _ hx) hv)
        (by simp only [List.length_cons] at hlen ⊢; omega)
      refine ⟨rr, List.mem_flatMap.mpr ⟨e, he, ?_⟩⟩
      simp only [hcont, Bool.false_eq_true, if_false]
      apply List.mem_append_right
      have : (e :: e2 :: tl2).reverse ++ revPathOf st =
          (e2 :: tl2).reverse ++ revPathOf (some (advance st e)) := by
        simp [revPathOf, advance_revPath]
      rw [this]; exact hrr

/-! ## Search setup: index, model set, reachability pruning -/

/-- Outgoing edges by source id, keeping only edges into `keep`, in the
    order of `edges`. Built once. -/
def buildOutIndex (keep : Nat → Bool) (edges : List Edge) : HashMap Nat (List Edge) :=
  edges.foldr (fun e m =>
    if keep e.target.id then m.insert e.source.id (e :: m.getD e.source.id []) else m) ∅

theorem buildOutIndex_getD (keep : Nat → Bool) (edges : List Edge) (v : Nat) :
    (buildOutIndex keep edges).getD v [] =
      edges.filter (fun e => e.source.id == v && keep e.target.id) := by
  induction edges with
  | nil => simp [buildOutIndex]
  | cons e es ih =>
    simp only [buildOutIndex, List.foldr_cons] at ih ⊢
    by_cases hk : keep e.target.id = true
    · rw [if_pos hk, HashMap.getD_insert]
      by_cases hv : e.source.id = v
      · subst hv; simp [ih, hk]
      · have : (e.source.id == v) = false := by simp [hv]
        rw [if_neg (by simp [this]), ih]
        simp [hv]
    · rw [if_neg hk, ih]
      simp [hk]

/-- A set of node ids. -/
def idSet (ids : List Nat) : HashSet Nat := ids.foldl (fun s i => s.insert i) ∅

theorem idSet_contains_aux (ids : List Nat) (s : HashSet Nat) (x : Nat) :
    (ids.foldl (fun s i => s.insert i) s).contains x = (s.contains x || ids.contains x) := by
  induction ids generalizing s with
  | nil => simp
  | cons i is ih =>
    rw [List.foldl_cons, ih, HashSet.contains_insert, List.contains_cons]
    by_cases hx : x = i
    · subst hx; simp
    · have h1 : (i == x) = false := by simp [Ne.symm hx]
      have h2 : (x == i) = false := by simp [hx]
      rw [h1, h2]; simp

theorem idSet_contains (ids : List Nat) (x : Nat) : (idSet ids).contains x = ids.contains x := by
  simp [idSet, idSet_contains_aux]

/-- Reverse adjacency: target id ↦ source ids. -/
def reverseIndex (edges : List Edge) : HashMap Nat (List Nat) :=
  edges.foldl (fun m e => m.insert e.target.id (e.source.id :: m.getD e.target.id [])) ∅

/-- Backward search from the worklist over `rev` (unverified; its result is
    checked by `closedCheck`). -/
def reachLoop (rev : HashMap Nat (List Nat)) :
    (fuel : Nat) → (work : List Nat) → (seen : HashSet Nat) → HashSet Nat
  | 0, _, seen => seen
  | _, [], seen => seen
  | fuel + 1, v :: work, seen =>
    let (work, seen) := (rev.getD v []).foldl (fun (acc : List Nat × HashSet Nat) u =>
      if acc.2.contains u then acc else (u :: acc.1, acc.2.insert u)) (work, seen)
    reachLoop rev fuel work seen

/-- Node ids that can reach one of `modelIds` over `edges` (every node is
    popped at most once, so the fuel suffices). -/
def reachSet (modelIds : List Nat) (edges : List Edge) : HashSet Nat :=
  reachLoop (reverseIndex edges) (modelIds.length + edges.length + 1) modelIds (idSet modelIds)

/-- `keep` contains every model id and is closed backwards under `edges`. -/
def Closed (keep : Nat → Bool) (modelIds : List Nat) (edges : List Edge) : Prop :=
  (∀ x ∈ modelIds, keep x = true) ∧ (∀ e ∈ edges, keep e.target.id = true → keep e.source.id = true)

/-- Decide `Closed`. -/
def closedCheck (keep : Nat → Bool) (modelIds : List Nat) (edges : List Edge) : Bool :=
  modelIds.all keep && edges.all (fun e => !keep e.target.id || keep e.source.id)

/-- Membership in a pruning set; `none` keeps every node. -/
def keepOf : Option (HashSet Nat) → Nat → Bool
  | some r, v => r.contains v
  | none, _ => true

/-- The pruning set: `reachSet` if it passes `closedCheck`, else `none` (no
    pruning). A set, not a function, so that it is computed once. -/
def mkKeepSet (modelIds : List Nat) (edges : List Edge) : Option (HashSet Nat) :=
  let r := reachSet modelIds edges
  if closedCheck (keepOf (some r)) modelIds edges then some r else none

/-- The pruning predicate. -/
def mkKeep (modelIds : List Nat) (edges : List Edge) : Nat → Bool :=
  keepOf (mkKeepSet modelIds edges)

theorem mkKeep_closed (modelIds : List Nat) (edges : List Edge) :
    Closed (mkKeep modelIds edges) modelIds edges := by
  unfold mkKeep mkKeepSet
  simp only
  split
  · rename_i h
    simp only [closedCheck, Bool.and_eq_true, List.all_eq_true] at h
    refine ⟨h.1, fun e he hk => ?_⟩
    have := h.2 e he
    simp only [Bool.or_eq_true, Bool.not_eq_true'] at this
    rcases this with h' | h'
    · rw [h'] at hk; cases hk
    · exact h'
  · exact ⟨fun _ _ => rfl, fun _ _ _ => rfl⟩

/-- Along a chain into a kept node, every node is kept. -/
theorem Closed.keep_nodeIds {keep : Nat → Bool} {modelIds : List Nat} {edges : List Edge}
    (hc : Closed keep modelIds edges) :
    ∀ (p : List Edge) (v : Nat), (∀ e ∈ p, e ∈ edges) → ChainFrom v p →
      keep (endId v p) = true → ∀ x ∈ nodeIds v p, keep x = true := by
  intro p
  induction p with
  | nil => intro v _ _ hend x hx; simp only [ContractGraph.nodeIds, List.mem_singleton] at hx; simp only [endId] at hend; rw [hx]; exact hend
  | cons e rest ih =>
    intro v hedges hchain hend x hx
    simp only [ChainFrom] at hchain
    simp only [endId] at hend
    have hrest := ih e.target.id (fun e' he' => hedges e' (List.mem_cons_of_mem _ he'))
      hchain.2 hend
    simp only [ContractGraph.nodeIds] at hx
    rcases List.mem_cons.mp hx with rfl | hx
    · rw [← hchain.1]
      apply hc.2 e (hedges e List.mem_cons_self)
      apply hrest
      cases rest with
      | nil => simp [ContractGraph.nodeIds]
      | cons _ _ => simp [ContractGraph.nodeIds]
    · exact hrest x hx

/-- Ids of the model nodes of `g`. -/
def modelNodeIds (g : ContractGraph) : List Nat :=
  (g.nodes.filter (·.kind == "model")).map (·.id)

/-- Everything the searches need, computed once per graph. -/
structure SearchSetup where
  /-- Outgoing checked edges into kept nodes, by source id. -/
  idx : HashMap Nat (List Edge)
  /-- Model node ids. -/
  models : HashSet Nat
  /-- Maximum path length searched: checked edges + 1. -/
  fuel : Nat

/-- Edges followed from a node id. -/
def SearchSetup.out (s : SearchSetup) (v : Nat) : List Edge := s.idx.getD v []

/-- Whether a node id is a model node's. -/
def SearchSetup.isModel (s : SearchSetup) (v : Nat) : Bool := s.models.contains v

def searchSetup (g : ContractGraph) : SearchSetup :=
  let edges := checkedEdges g.edges
  let modelIds := modelNodeIds g
  let keepSet := mkKeepSet modelIds edges
  { idx := buildOutIndex (keepOf keepSet) edges
    models := idSet modelIds
    fuel := edges.length + 1 }

/-- The search results of every function node of `g`, raw (reversed paths
    and results). -/
def searchRaw (s : SearchSetup) (g : ContractGraph) : List (List Edge × List CheckResult) :=
  (g.nodes.filter (·.kind == "function")).flatMap fun src =>
    searchFrom s.out s.isModel s.fuel src.id [] none

/-- A raw search result as `(path, results)`. -/
def viewPath (x : List Edge × List CheckResult) : List Edge × List CheckResult :=
  (x.1.reverse, x.2.reverse)

/-- Every data path of `g` with its `checkPath` results. -/
def checkAllPaths (g : ContractGraph) : List (List Edge × List CheckResult) :=
  (searchRaw (searchSetup g) g).map viewPath

/-- Every data path of `g` (pruned, indexed, one search per function node). -/
def enumeratePaths (g : ContractGraph) : List (List Edge) :=
  (checkAllPaths g).map (·.1)

theorem searchRaw_spec (s : SearchSetup) (g : ContractGraph) :
    ∀ x ∈ searchRaw s g, x.2.reverse = checkPath x.1.reverse := by
  intro x hx
  obtain ⟨src, _, hx⟩ := List.mem_flatMap.mp hx
  exact searchFrom_spec _ _ _ _ _ none trivial x hx

/-- Every enumerated path comes with its `checkPath` results. -/
theorem checkAllPaths_spec (g : ContractGraph) :
    ∀ x ∈ checkAllPaths g, x.2 = checkPath x.1 := by
  intro x hx
  obtain ⟨y, hy, rfl⟩ := List.mem_map.mp hx
  exact searchRaw_spec _ g y hy

theorem searchSetup_out (g : ContractGraph) (v : Nat) :
    (searchSetup g).out v = (checkedEdges g.edges).filter
      (fun e => e.source.id == v && mkKeep (modelNodeIds g) (checkedEdges g.edges) e.target.id) :=
  buildOutIndex_getD _ _ v

theorem searchSetup_isModel (g : ContractGraph) (v : Nat) :
    (searchSetup g).isModel v = (modelNodeIds g).contains v :=
  idSet_contains _ v

/-- Every data path of `g` is found by the search, in reversed form. -/
theorem searchRaw_complete (g : ContractGraph) (p : List Edge) (h : IsDataPath g p) :
    ∃ rr, (p.reverse, rr) ∈ searchRaw (searchSetup g) g := by
  have hlen := h.length_lt
  obtain ⟨hne, hedges, src, hsrc, tgt, htgt, hsk, htk, hchain, hend, hnodup⟩ := h
  have hmodel : endId src.id p ∈ modelNodeIds g := by
    rw [hend]
    exact List.mem_map.mpr ⟨tgt, List.mem_filter.mpr ⟨htgt, by simp [htk]⟩, rfl⟩
  have hkeep := Closed.keep_nodeIds (mkKeep_closed (modelNodeIds g) (checkedEdges g.edges)) p src.id
    hedges hchain ((mkKeep_closed _ _).1 _ hmodel)
  have hout : ∀ e ∈ p, e ∈ (searchSetup g).out e.source.id := by
    intro e he
    rw [searchSetup_out]
    refine List.mem_filter.mpr ⟨hedges e he, ?_⟩
    simp only [beq_self_eq_true, Bool.true_and]
    exact hkeep _ (mem_nodeIds_target _ _ hchain e he)
  obtain ⟨rr, hrr⟩ := mem_searchFrom (searchSetup g).out (searchSetup g).isModel p src.id []
    none (searchSetup g).fuel hne hout hchain
    (by rw [searchSetup_isModel]; exact List.contains_iff_mem.mpr hmodel)
    hnodup (fun _ _ h => by cases h) (by show p.length ≤ (checkedEdges g.edges).length + 1; omega)
  refine ⟨rr, List.mem_flatMap.mpr ⟨src, List.mem_filter.mpr ⟨hsrc, by simp [hsk]⟩, ?_⟩⟩
  simpa [revPathOf] using hrr

/-- COMPLETENESS: every checked data path of `g` is checked, with its
    `checkPath` results. -/
theorem checkAllPaths_complete (g : ContractGraph) (p : List Edge) (h : IsDataPath g p) :
    (p, checkPath p) ∈ checkAllPaths g := by
  obtain ⟨rr, hrr⟩ := searchRaw_complete g p h
  have hspec := searchRaw_spec _ g _ hrr
  simp only [List.reverse_reverse] at hspec
  exact List.mem_map.mpr ⟨_, hrr, by simp [viewPath, hspec]⟩

/-- COMPLETENESS: every checked data path of `g` is enumerated. -/
theorem enumeratePaths_complete (g : ContractGraph) (p : List Edge)
    (h : IsDataPath g p) : p ∈ enumeratePaths g :=
  List.mem_map.mpr ⟨_, checkAllPaths_complete g p h, rfl⟩

/-! ## Streaming: fold over the search results without building the list -/

/-- `searchFrom`, folding `F` over each emitted result as it is found instead
    of returning the list (`searchFold_eq`). Nothing but the current search
    stack is retained. -/
def searchFold {α : Type} (out : Nat → List Edge) (isModel : Nat → Bool)
    (F : α → List Edge × List CheckResult → α) :
    (fuel : Nat) → (v : Nat) → (visited : List Nat) → Option PathState → α → α
  | 0, _, _, _, acc => acc
  | fuel + 1, v, visited, st, acc =>
    let visited' := v :: visited
    (out v).foldl (fun acc e =>
      if visited'.contains e.target.id then acc else
        let st' := advance st e
        let acc := if isModel e.target.id then F acc (st'.2.1, st'.2.2) else acc
        searchFold out isModel F fuel e.target.id visited' (some st') acc) acc

theorem searchFold_eq {α : Type} (out : Nat → List Edge) (isModel : Nat → Bool)
    (F : α → List Edge × List CheckResult → α) :
    ∀ fuel v visited st acc,
      searchFold out isModel F fuel v visited st acc =
        (searchFrom out isModel fuel v visited st).foldl F acc := by
  intro fuel
  induction fuel with
  | zero => intro _ _ _ _; rfl
  | succ f ih =>
    intro v visited st acc
    simp only [searchFold, searchFrom, List.foldl_flatMap]
    congr 1
    funext acc e
    split
    · rfl
    · rw [List.foldl_append, ih]
      split <;> rfl

/-- `searchRaw` folded with `F`, streaming. -/
def foldRaw {α : Type} (s : SearchSetup) (g : ContractGraph)
    (F : α → List Edge × List CheckResult → α) (init : α) : α :=
  (g.nodes.filter (·.kind == "function")).foldl (fun acc src =>
    searchFold s.out s.isModel F s.fuel src.id [] none acc) init

theorem foldRaw_eq {α : Type} (s : SearchSetup) (g : ContractGraph)
    (F : α → List Edge × List CheckResult → α) (init : α) :
    foldRaw s g F init = (searchRaw s g).foldl F init := by
  simp only [foldRaw, searchRaw, List.foldl_flatMap, searchFold_eq]

/-! ## Counting paths under a budget (performance only)

`countPaths` runs the same search without composing or checking and stops
once more than `maxPaths` paths (or more than `maxSteps` edge traversals)
have been seen. It only decides whether the checker runs the full search; no
theorem depends on its value. -/

def countFrom (out : Nat → List Edge) (isModel : Nat → Bool) (maxPaths maxSteps : Nat) :
    (fuel : Nat) → (v : Nat) → (visited : List Nat) → Nat × Nat → Nat × Nat
  | 0, _, _, acc => acc
  | fuel + 1, v, visited, acc =>
    let visited' := v :: visited
    (out v).foldl (fun (acc : Nat × Nat) e =>
      if acc.1 > maxPaths || acc.2 > maxSteps then acc
      else if visited'.contains e.target.id then (acc.1, acc.2 + 1)
      else
        let acc := (acc.1 + (if isModel e.target.id then 1 else 0), acc.2 + 1)
        countFrom out isModel maxPaths maxSteps fuel e.target.id visited' acc) acc

/-- `(paths, steps)` seen by the search of every function node, stopping
    early past either budget. -/
def countPaths (s : SearchSetup) (g : ContractGraph) (maxPaths maxSteps : Nat) : Nat × Nat :=
  (g.nodes.filter (·.kind == "function")).foldl (fun acc src =>
    if acc.1 > maxPaths || acc.2 > maxSteps then acc
    else countFrom s.out s.isModel maxPaths maxSteps s.fuel src.id [] acc) (0, 0)

end ContractGraph
