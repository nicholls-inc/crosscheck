# Correspondence: Ocpp16Composite

Pipeline step: 4 of 5 — classified by /correspondence-review.

Reviewed 2026-10-24. The sources are OCPP 1.6 edition 2 FINAL (2017-09-28), as extracted in `/tmp/claude-502/fhwa-dl/raw.txt`, and the signed `formal-verification/specs/ocpp16_composite_informal.md` (2026-10-24). Below, `raw.txt:L-M` always means that exact extraction, and `informal:L-M` means the signed document. EVerest is the later system under test, not the correspondence source; no EVerest definition is classified here.

## Verdict summary

This is the reviewer's original table, kept as written. The audit below revised it: exact 20, abstraction 4, approximation 0, mismatch 0, DRT readiness READY. Read the revised table, not this one, for the current state.

| Class (reviewer's original) | Count | DRT effect |
|---|---|---|
| exact | 18 | runs, subject to dependency gates and stated preconditions |
| abstraction | 2 | runs with omitted context held constant |
| approximation | 2 | skipped, flagged |
| mismatch | 2 | blocks DRT until resolved |

Reviewer's original verdict: overall DRT readiness **BLOCKED**. Current verdict after the audit and the fixes: **READY**.

Counts include all 24 `def` declarations, including `periodsWellFormed`, `jan1`, `hour`, and `ex1` to `ex5`; types and theorems are assessed separately. Classifications describe each definition's translation of its cited source expression. A locally exact composition does not repair a mismatched or approximate dependency: `scheduleStart → covers → limitAt/candidates → prevailing → purposeLimit/cpMaxResult/txResult → composite` propagates the oracle restriction. No composite DRT is authorized by the exact rows while the mismatch gate is open.

Two root mismatch issues are recorded locally below, as CR-OC16-01 and CR-OC16-02. Both were fixed before DRT ran (see the audit). No remote issue was created: this review was authorized to edit only this document.

## Pipeline owner's audit of this review (2026-10-24)

The review above was run by an independent model (Codex) and is kept as written. The pipeline owner audited each non-exact verdict and acted as follows. The verdict summary table above is superseded by the revised table here. Sections below this audit are the reviewer's text as written. Where the audit changed a class, the section heading and a Status line say so. Wording inside those sections that calls a mismatch open, or says it blocks DRT, is the reviewer's original finding and is historical.

| Lean ident | Review verdict | Action | Revised class |
| --- | --- | --- | --- |
| `scheduleStart` | mismatch (CR-OC16-01) | Agreed. The model now gives an Absolute profile its startSchedule unconditionally, as D3 says, and `covers` requires s ≤ t. The witness exposed that the signed D4 had lost "s ≤ t" when A5 and A6 were applied. D4 is restored with a correction note, which the operator confirmed on 2026-10-27. | exact |
| `WellFormed` | mismatch (CR-OC16-02) | Agreed. P6 is now stated over list positions with a local `pairwise`, which means the same as `List.Pairwise` and builds on the Crosscheck image's Lean v4.10.0, so duplicate entries violate it. `WellFormed` is not used by any proved theorem, so no proof changes. | exact |
| `candidates` | approximation | Not adopted. The model follows signed resolution A1, which the operator chose knowing the official sentence reads otherwise (OCA note N1). Divergences here are triaged first as spec-reading gaps, not EVerest bugs. Downgraded from exact rather than skipped, because skipping would remove the composite from DRT. | abstraction |
| `txResult` | approximation | Not adopted, for the same reason under resolution A2. | abstraction |

Revised counts: exact 20, abstraction 4, approximation 0, mismatch 0. Overall DRT readiness: **READY**, with divergences in `candidates` and `txResult` triaged as spec-reading gaps first. The rebuilt model passes all five worked-example guards, and `check_ocpp16_composite.sh` passes for q1 to q3, i1 to i3 and the lemma.

The reclassification of `candidates` and `txResult` is a judgment by the pipeline owner against the reviewer's verdict. The operator approved it on 2026-10-27: DRT tests `candidates` and `txResult` against signed resolutions A1 and A2, and any divergence there is reported as a spec-reading gap, never as an EVerest bug.

## Prerequisite checks and source pairing

- The Lean file exists, its header says “Pipeline step: 3 of 5”, and every definition has an implementation. The only `sorry` is the proof of `i4` (Lean line 297).
- The original correspondence stub existed and had Modelling decisions, Definitions to classify, Open questions, and Source files sections.
- The user-authorized replacement for Step 0.4 was run from the repository root: `~/.elan/bin/elan run leanprover/lean4:v4.28.0 lean formal-verification/lean/CrosscheckModel/Ocpp16Composite.lean`. Exit 0; only line 293's expected “declaration uses sorry” warning. This checks the module, not a whole Lake build. Docker was not used.
- Both specification sources exist and were read, including §§3.13.1–3.13.3, 3.13.7, 7.8–7.14 and 7.37. The later SUT copy also exists at `formal-verification/tests/ocpp16_composite/everest_driver/.deps/EVerest-dd696165d407d461a724232a4d77fe476a964ef1/lib/everest/ocpp/lib/ocpp/v16/profile.cpp`.
- Traceability finding: `candidates`, `pickHighest`, `purposeLimit`, `minOpt`, `periodsWellFormed`, `WellFormed`, `jan1`, `hour`, and `ex1`–`ex5` lack immediate `-- src:` comments. They were candidate mismatches at Step 1; explicit semantic counterparts are paired below, so missing comments alone are not asserted to be behavioral mismatches. Existing comments name sections rather than extraction line ranges; the ranges below supply that missing precision.
- The complete in-scope operation surface in informal:18–20 and D1–D6/Q1–Q4 has counterparts. Installation, replacement/deletion, connector-0 aggregation, Relative profiles, transaction matching, unit conversion, phases and offline behavior are boundary restrictions in informal:22–26 and P8, not silently claimed as verified behavior.

## Definitions

### applicable — class: exact

- **Source.** §3.13.1, `raw.txt:1186-1209`; informal:38,47–48 (D1/P1/P2).
- **Behaviour.** Lean:75–79 implements D1: station maximum only on connector 0, defaults on 0 or c, transaction profiles on c. This is a one-for-one test on the signed input domain c ≥ 1.
- **Rubric step that drove the class.** Step 2.1, same predicates.
- **Divergences.** For c = 0 a TxProfile on 0 passes the equality test, but §3.13.1 (`raw.txt:1209`) and P1/P2 disallow that input. No in-domain disagreement. Aggregate station allocation (`raw.txt:1245-1247`) is not represented by this applicability test.
- **Theorem impact.** Transitive dependency of q1–q3, i1–i4 and `candidates_eq_of_same_nonDefault`; no local blocker.
- **DRT note.** Require c ≥ 1 and valid purpose/connector combinations; applicability is independently testable after the global gate closes.

### validAt — class: abstraction

- **Source.** §7.8, `raw.txt:5353-5370`; informal:39,110 (D2/A4).
- **Behaviour.** Lean:83–89 implements inclusive validFrom and exclusive validTo, with missing bounds returning true. It agrees with the signed interval test. Receipt, replacement and transaction termination are context supplied by the installed-profile snapshot.
- **Rubric step that drove the class.** Step 2.2, lifecycle context omitted; uncertain equivalence to unrestricted timestamp queries is downgraded from exact.
- **Divergences.** Missing validFrom means valid since receipt, not for all historical times (`raw.txt:5359-5361`); missing validTo means until replacement (`raw.txt:5369-5370`). TxProfile also expires at transaction termination (§7.10, `raw.txt:5426-5430`). The model has no receipt/replacement/termination times. Its result is faithful only for the current installed snapshot during the active transaction in P1.
- **Theorem impact.** All selection-based properties depend on this abstraction; none proves lifecycle validity.
- **DRT note.** Keep receipt ≤ evaluation time, retain only installed profiles, and hold the active transaction constant. Do not query historical lifetimes using this helper alone.

### recurrence — class: abstraction

- **Source.** §§7.9 and 7.37, `raw.txt:5389-5395,6305-6317`; informal:40,111–112,125 (D3/A5/A6/N4).
- **Behaviour.** Lean:93–96 returns no recurrence for Absolute, 86400 for Daily and 604800 for Weekly. These constants are exact under the signed calendar-day clock; the source's separate kind/recurrencyKind fields are fused into `Kind`.
- **Rubric step that drove the class.** Step 2.2, representation abstraction and explicitly selected interpretation of ambiguous clock wording.
- **Divergences.** The official text combines elapsed-hour language and same-clock-time language (`raw.txt:6307,6317`). The signed spec chooses UTC calendar days without leap seconds. This review does not establish that all OCPP elapsed-time representations share that interpretation. Relative and missing recurrence are unrepresentable under P3.
- **Theorem impact.** Schedule-dependent properties are about the signed clock, not a proof resolving N4 for all implementations.
- **DRT note.** Normalize calendar timestamps consistently; do not silently compare leap-second-counting elapsed seconds with these integers. No rounding of dates or limits is authorized.

### scheduleStart — reviewer's original class: mismatch (revised: exact)

- **Status.** CR-OC16-01 was fixed before DRT ran: the model gives an Absolute profile its startSchedule unconditionally, and the signed D4 was restored. Revised class exact, see the audit table. The text below is the reviewer's original finding.

- **Source.** §§7.9, 7.13 and 7.37, `raw.txt:5389-5395,5501-5508,6305-6317`; informal:40,111–112 (D3/A5/A6). Lean:101–106.
- **Behaviour.** For Recurring schedules, guarded subtraction and division compute exactly the latest nonnegative recurrence index, including restart instants. However the pre-start guard is applied to Absolute profiles too. D3 gives an Absolute profile its fixed startSchedule without that exception; A5 only adds the exception for Recurring profiles.
- **Rubric step that drove the class.** Step 2.3, model/source behavioral disagreement on an admitted helper input.
- **Divergences.** An Absolute profile with startSchedule = 1000 queried at t = 999 yields `none`; signed D3 yields 1000. P1–P8 do not forbid querying before an Absolute schedule's start. The official “starting point” wording (`raw.txt:5507-5508`) makes Lean's interpretation plausible, but does not erase the discrepancy with signed D3. See CR-OC16-01; this may be a prose omission rather than an intended model bug.
- **Theorem impact.** All schedule-dependent properties inherit a correspondence blocker. Existing proofs can remain true internally; they cannot establish agreement with the signed D3 as written.
- **DRT note.** Block pending triage. Recurring arithmetic alone passed the concrete checks below.

### covers — class: exact

- **Source.** §7.13, `raw.txt:5491-5508`; informal:41,85,88 (D4/A6). Lean:110–116.
- **Behaviour.** Direct case analysis of signed D4: no schedule start means false; a start with no duration means true; otherwise t < s + duration. No off-by-one duration error was found.
- **Rubric step that drove the class.** Step 2.1, exact translation of D4 in terms of D3.
- **Divergences.** No additional local difference. Its actual pre-Absolute-start result differs because `scheduleStart` differs: for s = 1000, d = 10, t = 999, Lean returns false whereas literal D3/D4 yield true. This is inherited CR-OC16-01, not independent evidence that the signed prose intended coverage before start. Zero duration is representable but disallowed by P7.
- **Theorem impact.** All selection properties inherit CR-OC16-01 and the signed latest-occurrence choice.
- **DRT note.** Dependency-blocked. At recurring restart, compare against the new occurrence's end, not the preceding occurrence's end.

### limitAt — class: exact

- **Source.** §§7.13–7.14, `raw.txt:5497-5499,5524-5526,5553-5570`; informal:41–42,52,84 (D4/D5/P4). Lean:121–126.
- **Behaviour.** Within a covering schedule with P4 periods, filtering starts ≤ offset and taking the final list element selects the unique latest chronological period. Limits are tenths, so no float approximation is present.
- **Rubric step that drove the class.** Steps 2.1 and 2.2, same result for strictly increasing periods, with the partial D5 operation made optional outside coverage.
- **Divergences.** On unordered periods the algorithm selects the last qualifying list element, not the qualifying period with greatest startPeriod. Before a nonzero first startPeriod it returns none. Both violate P4 (`informal:52`; corrected field interpretation of `raw.txt:5525-5526`). The official text names a nonexistent first-period startSchedule field; A8 explicitly resolves it. The pre-start D3 discrepancy remains a dependency restriction.
- **Theorem impact.** q1/q2 speak about this model lookup; i1/i3 remain algebraic even on malformed periods. I2 intentionally handles a prevailing TxProfile returning none. No property verifies period sorting at runtime.
- **DRT note.** P4 is mandatory. Do not sort or repair malformed SUT inputs and then claim this review classified their behavior.

### candidates — reviewer's original class: approximation (revised: abstraction)

- **Status.** The operator approved the reclassification on 2026-10-27. DRT tests `candidates` against signed resolution A1, and a divergence there is reported as a spec-reading gap, never as an EVerest bug. The DRT note below (flag and skip) is the reviewer's original instruction and was not followed. The text below is the reviewer's original finding.

- **Source.** §3.13.2, `raw.txt:1217-1219,1233-1234`; informal:43,107,122 (D6/A1/N1). Lean:129–131.
- **Behaviour.** Exact filter for the signed D6: purpose, connector applicability, validity and coverage. The normative prevailing sentence names validity via validFrom/validTo; the model adds coverage, following signed A1 and the duration/fallback note.
- **Rubric step that drove the class.** Step 2.2, uncertain source semantics downgraded: exact to signed A1, but not established equivalent to both readings of the official text. This is a flagged approximation of the unresolved normative surface, not a floating-point approximation.
- **Divergences.** A higher-stack valid profile after its duration disappears here; a literal reading of `raw.txt:1217-1219` still selects it. `raw.txt:1233-1234` supports fallback, so the official text is internally problematic. Hand witness C6 below. Also inherits CR-OC16-01. No immediate src comment; correspondence recovered from D6.
- **Theorem impact.** q1/q2 and i2/i4 concern signed A1 selection; their proofs cannot resolve N1 or validate the alternate OCPP reading.
- **DRT note.** Flag and skip disputed coverage-versus-validity regions until source interpretation is settled. Never call a divergence there an unconditional OCPP violation.

### pickHighest — class: exact

- **Source.** §3.13.2, `raw.txt:1217-1222`; informal:43,56 (D6/P6). Lean:135–141.
- **Behaviour.** A left fold keeps the maximum stackLevel; a strict comparison keeps the first equal maximum. P6 prevents distinct applicable same-purpose same-connector ties. A3 also excludes default ties across connector 0 and c.
- **Rubric step that drove the class.** Steps 2.1 and 2.2, finite fold computes the specified maximum on the admitted domain.
- **Divergences.** First-list tie breaking has no normative source: official text prevents equal-stack conflicts (`raw.txt:1220-1222`). It is an extension outside the unique-maximum domain. The comment that P6 prevents all ties is too strong for duplicate entries accepted by `WellFormed`; see CR-OC16-02. Exact duplicate winners have equal outputs, so that gap is not itself a differing-limit witness.
- **Theorem impact.** i4 uses strict inequality, so equal-stack behavior is irrelevant to its statement. i2 preserves the order of all non-default profiles.
- **DRT note.** Test ties as malformed-domain diagnostics only. Enforce installed-profile uniqueness outside the current `WellFormed` predicate.

### prevailing — class: exact

- **Source.** §3.13.2, `raw.txt:1217-1234`; informal:43,56,107 (D6/P6/A1). Lean:146–148.
- **Behaviour.** Directly composes D6's candidate restriction with its highest-stack selection.
- **Rubric step that drove the class.** Step 2.1, exact signed composition.
- **Divergences.** No new local divergence. It inherits the candidates approximation, scheduleStart mismatch, lifecycle abstraction and input uniqueness requirement. The official validity-only reading remains unsettled.
- **Theorem impact.** All q/i properties involving selected profiles inherit those restrictions; i4's remaining sorry does not settle any of them.
- **DRT note.** Dependency-blocked; locally exact composition is not independent certification of the resulting selected profile.

### purposeLimit — class: exact

- **Source.** §3.13.3, `raw.txt:1236-1244`; informal:42–43,62–63 (D5/D6/Q1/Q2). Lean:151–153.
- **Behaviour.** Bind the prevailing profile to its limit, returning none if either lookup fails. This is the signed notion of a prevailing purpose's limit.
- **Rubric step that drove the class.** Step 2.1, exact composition of D5 and D6; its source counterpart is semantic rather than a named code function.
- **Divergences.** None locally; dependency restrictions apply. No immediate src comment.
- **Theorem impact.** q1/q2 and i1–i3 depend on it; their equalities are internal model facts.
- **DRT note.** Block through prevailing; do not equate “profile absent” with “profile present but no period” in a harness that tests malformed inputs.

### cpMaxResult — class: exact

- **Source.** §§3.13.1 and 3.13.3, `raw.txt:1186-1190,1207-1209,1236-1247`; informal:62 (Q1). Lean:157–158.
- **Behaviour.** Select the prevailing ChargePointMaxProfile limit, exactly Q1 for one connector.
- **Rubric step that drove the class.** Step 2.1, literal purpose specialization.
- **Divergences.** None locally. A per-connector upper bound is not proof of the sum constraint on all connectors (`raw.txt:1245-1247`); aggregation is outside the signed boundary.
- **Theorem impact.** q1 is reflexive; i1 bounds composite against this selected result, not against every station profile or total energy flow.
- **DRT note.** Inherits prevailing's restrictions. Hold other-connector allocation outside this pointwise comparison.

### txResult — reviewer's original class: approximation (revised: abstraction)

- **Status.** The operator approved the reclassification on 2026-10-27. DRT tests `txResult` against signed resolution A2, and a divergence there is reported as a spec-reading gap, never as an EVerest bug. The DRT note below (flag and skip) is the reviewer's original instruction and was not followed. The text below is the reviewer's original finding.

- **Source.** §3.13.1, `raw.txt:1200-1209`; §7.10, `raw.txt:5426-5430`; informal:63,108 (Q2/A2). Lean:162–165.
- **Behaviour.** Exact signed Q2: branch on the existence of a prevailing TxProfile, return its limit even if none, and use TxDefault only if no TxProfile prevails. Signed A2 limits the override to covered instants.
- **Rubric step that drove the class.** Step 2.2, uncertainty about equivalence to the authoritative OCPP wording; downgraded rather than declaring the signed interpretation an exact universal oracle.
- **Divergences.** The official text says override for the current transaction and use TxDefault when no TxProfile is provided (`raw.txt:1200-1209`). That does not explicitly give the signed outside-coverage fallback. An installed TxProfile whose duration has ended is a concrete differing branch under those readings (C6). The model faithfully implements A2, but that human resolution does not rewrite the official wording. A prevailing profile with no eligible period returns none rather than a default limit; this agrees with signed Q2 and violates P4, not an in-domain fallback bug.
- **Theorem impact.** q2 and i2 encode A2. i2's proved independence is intentional even when `limitAt` is none. Proof of i2 is not proof of whole-transaction OCPP override semantics.
- **DRT note.** Flag/skip the disputed outside-coverage override region. Inherits scheduleStart and prevailing gates.

### minOpt — class: exact

- **Source.** §3.13.3, `raw.txt:1239-1244`; informal:64–65,113 (Q3/A7). Lean:168–172.
- **Behaviour.** Four cases give minimum of two present limits, the sole present limit, or no bound. This is the signed minimum with none interpreted as no constraint.
- **Rubric step that drove the class.** Step 2.1, exact case split over optional bounds.
- **Divergences.** None on nonnegative tenths. The spec does not prescribe unconstrained charger behavior; no bound is not a required output of zero. No immediate src comment.
- **Theorem impact.** q3, i1, i3 and composite-based i2/i4 use it; no local blocker.
- **DRT note.** Can test this arithmetic independently after the global gate closes; preserve none distinctly from 0.

### composite — class: exact

- **Source.** §3.13.3, `raw.txt:1236-1247`; informal:64–65,116 (Q3/Q4/A10). Lean:176–177.
- **Behaviour.** Exact signed Q3 composition: minimum of station-maximum and transaction/default results. It computes the largest permitted bound, while Q4 permits a lower implementation result.
- **Rubric step that drove the class.** Step 2.1, exact minimum composition relative to the signed purpose results.
- **Divergences.** No new local divergence. It inherits both approximate regions and CR-OC16-01. It does not compute a time-interval list or enforce station-wide sums (`raw.txt:1240-1247`). The signed boundary deliberately specifies one time and connector.
- **Theorem impact.** q3 and i1–i4 are model-level equalities/bounds. No proven theorem bridges the disputed source interpretations.
- **DRT note.** Block. Later compare conformance with Q4, not equality alone; smaller implementation limits are permitted.

### conforms — class: exact

- **Source.** §3.13.3, `raw.txt:1242-1244`; informal:65,113,116 (Q4/A7/A10). Lean:182–185.
- **Behaviour.** Accept impl ≤ model when a bound exists and accept any impl when none exists, exactly Q4.
- **Rubric step that drove the class.** Step 2.1, literal inequality and optional-bound branch.
- **Divergences.** None on encoded limits. The signature cannot compare missing implementation outputs, unit conversions or an implementation's interval coverage; those require harness checks. No theorem named q4 is declared; Q4 is implemented by this definition.
- **Theorem impact.** No theorem directly states conformance; proved equalities alone do not establish implementation conformance.
- **DRT note.** If the model says 160 and the SUT says 120, this predicate accepts; 170 fails. A model none accepts either. Retain that asymmetry in reporting.

### periodsWellFormed — class: exact

- **Source.** §§7.13–7.14, `raw.txt:5522-5526,5559-5560`; informal:52,114 (P4/A8). Lean:188–190.
- **Behaviour.** Reject the empty list; require first startPeriod = 0 and pairwise strict increase. Pairwise comparison is equivalent to strictly increasing adjacent starts on this total order.
- **Rubric step that drove the class.** Step 2.1, exact P4 predicate, using signed A8 to interpret the official field-name typo.
- **Divergences.** No behavioral divergence from signed P4. The raw field spelling is erroneous, not a separate implemented startSchedule field. No immediate src comment.
- **Theorem impact.** No q/i theorem assumes this predicate; therefore those proofs do not establish it for their inputs.
- **DRT note.** Check every generated profile, including profiles not applicable to c.

### WellFormed — reviewer's original class: mismatch (revised: exact)

- **Status.** CR-OC16-02 was fixed before DRT ran: P6 is stated over list positions with a local `pairwise`, so duplicate entries violate it. Revised class exact, see the audit table. The text below is the reviewer's original finding.

- **Source.** §§3.13.1–3.13.2 and 7.8–7.14, `raw.txt:1186-1222,5298-5306,5318-5326,5491-5526,5562-5570`; informal:47–58 (P1–P8). Lean:193–203.
- **Behaviour.** Checks positive target connector, purpose connector restrictions, P4, positive duration, same-purpose/same-connector unequal-record stack uniqueness, and absence of default profiles on both 0 and c. It is not a complete encoding of P1–P8 over installed-profile lists.
- **Rubric step that drove the class.** Steps 2.2 and 2.3: several intentional external abstractions plus an unrecorded multiplicity gap in P6. Uncertain set/list equivalence is not promoted to exact.
- **Divergences.** P6's pair quantification exempts `p = q`. Thus two list occurrences of the same record pass. Two installed profiles with distinct omitted chargingProfileIds but identical modeled fields also collapse to that equality; official identity exists (§7.8, `raw.txt:5290-5296`) and conflict replacement is specified in `raw.txt:1220-1222`. No list-Nodup invariant is supplied. If the intended input is a mathematical set already deduplicated before encoding, that must be an explicit callee/harness contract; the signed spec says it has no assumed callee contracts (informal:28). See CR-OC16-02. Active transaction (P1), shared unit (P5), and no transactionId (P8) are not tested; see the audit below. These omitted fields are not automatically validation of raw inputs.
- **Theorem impact.** None of q1–q3/i1–i4 assumes `WellFormed`; these omissions do not invalidate their Lean proofs. They do invalidate a claim that any `WellFormed` sample is necessarily a signed-domain sample.
- **DRT note.** Block use as the sole generator acceptance predicate. No default overlap is enforced regardless of time, which correctly matches signed P6/A3's installation restriction.

### jan1 — class: exact

- **Source.** §7.13 dateTime start, `raw.txt:5501-5508`; informal:95 (worked-example clock). Lean:302.
- **Behaviour.** 1767225600 is 2026-01-01T00:00:00Z on the selected POSIX/calendar-second scale.
- **Rubric step that drove the class.** Step 2.1, exact fixture encoding.
- **Divergences.** No local difference; this timestamp is a fixture from the signed examples, not an extra normative rule. No immediate src comment.
- **Theorem impact.** None; used by compile-time examples only.
- **DRT note.** Fixture, not a separate production oracle entry point.

### hour — class: exact

- **Source.** §7.13 duration in seconds, `raw.txt:5491-5499`; informal:99–101 (worked-example hours). Lean:303.
- **Behaviour.** 3600 seconds per hour is the signed examples' clock conversion.
- **Rubric step that drove the class.** Step 2.1, exact constant.
- **Divergences.** None on that clock. No immediate src comment.
- **Theorem impact.** None; fixture only.
- **DRT note.** No standalone DRT function required.

### ex1 — class: exact

- **Source.** §3.13.7, `raw.txt:1608-1698`; informal:97. Lean:306–310.
- **Behaviour.** Daily default from 2013-01-01 with duration 86400 and periods 0/28800/72000 at 11000/6000/11000 W, exactly scaled by ten. The raw example does not supply connector 1; the signed fixture does.
- **Rubric step that drove the class.** Step 2.1, exact signed fixture encoding.
- **Divergences.** Raw example's id and phase count are omitted under the signed module boundary; they do not affect this selected single-unit limit computation. No immediate src comment.
- **Theorem impact.** No theorem quantified over ex1; its five build-time guards passed, without proving all Daily schedules.
- **DRT note.** Seed only; no Weekly coverage follows from this example.

### ex2 — class: exact

- **Source.** §§3.13.1 and 3.13.3, `raw.txt:1186-1190,1207-1209,1239-1244`; informal:98. Lean:313–319.
- **Behaviour.** Encodes station maximum 16 A and transaction 32 A as 160 and 320, fixed at jan1.
- **Rubric step that drove the class.** Step 2.1, exact signed fixture.
- **Divergences.** None locally; signed “every t” is understood during the described post-start evaluation, not evidence about CR-OC16-01.
- **Theorem impact.** Build guards passed at jan1 and +20 hours; not a quantified proof.
- **DRT note.** Seed subject to inherited gates.

### ex3 — class: exact

- **Source.** §3.13.2, `raw.txt:1217-1219,1233-1234`; informal:99. Lean:322–328.
- **Behaviour.** Encodes default 10 A at stack 0 plus default 20 A at stack 1 starting +10 hours for 3600 seconds.
- **Rubric step that drove the class.** Step 2.1, exact signed fixture.
- **Divergences.** No fixture error; expected fallbacks rely on A1 and the Absolute pre-start interpretation discussed in CR-OC16-01. Its faithful encoding does not make its expected behavior independent of those assumptions.
- **Theorem impact.** Guards at 9/10/11 hours passed; they do not discharge i4.
- **DRT note.** Treat outside-coverage selection as an interpretation-sensitive seed.

### ex4 — class: exact

- **Source.** §3.13.1, `raw.txt:1200-1209`; informal:100. Lean:331–337.
- **Behaviour.** Encodes 6 A Tx for twelve hours over a 32 A station-default profile, as 60 and 320.
- **Rubric step that drove the class.** Step 2.1, exact signed fixture encoding.
- **Divergences.** No fixture error. The expected noon fallback is signed A2, whose official-text correspondence is approximate in txResult.
- **Theorem impact.** Guards at 11/12 hours passed; not a proof of the official whole-transaction reading.
- **DRT note.** Flag the noon comparison as interpretation-sensitive.

### ex5 — class: exact

- **Source.** §§7.8 and 3.13.3, `raw.txt:5363-5370,1239-1244`; informal:101. Lean:340–346.
- **Behaviour.** Encodes 8 A station maximum expiring at noon, plus 16 A default.
- **Rubric step that drove the class.** Step 2.1, exact signed fixture.
- **Divergences.** None locally; validTo is explicitly half-open under A4. No immediate src comment.
- **Theorem impact.** Guards at 11/12 hours passed; no additional proof obligation.
- **DRT note.** Useful exact-boundary seed after global issues are resolved.

## Concrete adversarial calculations

All limits below are encoded tenths. Unless changed, a profile has connector 1, purpose Tx, startSchedule s = 1000, no validity bounds/duration, and periods [(0,100)]. These are hand calculations from the definitions, not differential testing. Selected results were additionally checked by sending the unchanged Lean source plus queries through `lean --stdin`; no extra file was written. The duplicate-list proposition was checked by `simp [WellFormed, periodsWellFormed, applicable]`. A first attempt using `decide` lacked a synthesized Decidable instance; this was a query elaboration failure, not a failure of the module or of the proposition.

### C1. Daily and Weekly arithmetic

- Daily r = 86400: t = 999 returns none (guard before Nat subtraction); 1000 gives k = 0 and s = 1000; 87399 gives floor(86399/86400) = 0 and s = 1000; exactly 87400 gives k = 1 and s = 87400. At 174800, floor(173800/86400) = 2, hence start = 173800, offset = 1000. Exact second restart is t = 173800, also start = 173800, offset = 0.
- Weekly r = 604800: 999 gives none; 1000 gives 1000; 605799 gives k = 0 and 1000; exactly 605800 gives k = 1 and 605800; exactly 1210600 gives k = 2 and 1210600.
- Both therefore restart at offset zero exactly on a boundary and never wrap a pre-start Nat subtraction into an apparent occurrence. This matches signed D3/A5/A6 and §7.37 (`raw.txt:6307,6317`) on the chosen clock. Leap-second conversion remains a harness boundary, not an arithmetic defect.

### C2. Duration and validity boundaries

- Absolute s = 1000, d = 10: at 1000 and 1009 coverage is true; exactly 1010 it is false. No duration keeps coverage after start indefinitely. d = 0 covers no time, and fails P7.
- Daily d = 10: at 87399 latest start is 1000, so false; at 87400 latest start is 87400, so true. With d = 86400 there is no gap: at 87400 the new occurrence starts. With d = 90000 the latest occurrence still wins at 87400; no attempt is made to keep an overlapping older occurrence. No duration likewise restarts period offsets rather than accumulating them forever. Weekly at 605800 behaves identically with r = 604800.
- validFrom = 1000, validTo = 1010: validAt 999/1000/1009/1010 gives false/true/true/false. Missing bounds give true; equal endpoints give an empty interval. These agree with signed D2/A4; official fields are `raw.txt:5353-5370`.
- Absolute s = 1000 at t = 999 gives scheduleStart = none and coverage false in Lean. D3 literally gives s = 1000 and D4 would accept t < s + d. That is CR-OC16-01, not an error in recurring division. Signed D5's negative offset before an Absolute start is also underspecified relative to Lean Nat subtraction; this review does not invent a limit there.

### C3. Period lookup and malformed lists

- Sorted [(0,100),(10,200),(20,300)] at offsets 9/10/19/20 gives 100/200/200/300. At exactly a period start the new limit applies (§7.14, `raw.txt:5559-5560`).
- Unsorted [(0,100),(20,200),(10,300)] at offset 25 filters to all three and returns 300, although the greatest chronological start is 20 with limit 200. `periodsWellFormed` rejects the list. This is not an in-P4 model bug.
- First start 10, periods [(10,60)]: offset 5 returns none, offset 10 returns 60. Empty periods always return none. Both violate P4 (`informal:52`, §7.13 `raw.txt:5522-5526`).

### C4. Connector applicability and ties

At c = 2, for profile connectorIds 0/1/2/3 respectively:

| Purpose | Applicability results |
|---|---|
| ChargePointMaxProfile | true / false / false / false |
| TxDefaultProfile | true / false / true / false |
| TxProfile | false / false / true / false |

Nonzero station-maximum IDs and zero Tx IDs also fail P2 in `WellFormed`. Non-applicable profiles otherwise do not enter candidates. These are the signed D1 cases (§3.13.1, `raw.txt:1186-1209`).

For two covering Tx profiles a and b, both connector 1, stack 7, limits 60 and 320: `pickHighest [a,b] = some a`, while reversing the list gives b. Distinct records fail P6 in `WellFormed`, so no in-domain tie convention is specified. For `[p,p]`, both occurrences tie and the first wins, yet `WellFormed [p,p] 1` is provable because every unequal-record premise is false. This is a domain-encoding gap, not a differing-limit counterexample. Under signed A3, defaults at connector 0 and c are forbidden even when their stacks differ; the Lean final conjunct correctly rejects that mix.

### C5. Prevailing TxProfile with no period

Use Tx p with periods [(10,60)], s = 1000, no duration; use a covering default with [(0,320)]. At t = 1005, p is applicable, valid, and covering, so it prevails. Its offset is 5 and no period qualifies: limitAt p = none. `txResult [p,default] 1 1005 = none`, not 320; with no station maximum, composite is none. q2 matches this result and i2 says changing the default still has no effect. This is consistent with the signed branch-on-profile Q2, and fails P4. Under P4 a covering schedule has offset ≥ 0 and a period at zero, so this hole cannot occur in Lean's admitted coverage domain.

### C6. Official wording versus signed A1/A2

Let low default have stack 0, start 1000, no duration, limit 100. Let high default have stack 1, start 1000, duration 10, limit 200. Both have no validity bounds. At 1010 the signed/model candidates exclude high and return 100; the literal validity-only prevailing sentence still selects high (`raw.txt:1217-1219`). The duration note (`raw.txt:1233-1234`) motivates signed A1, so this is an unresolved wording issue rather than a proven implementation fault.

For override, instead install a Tx at 60 with duration 10 plus a default at 320, same start 1000. At 1010, Lean returns default 320. Signed A2 explicitly wants this. The official transaction-long override and “in case no profile ... is provided” wording (`raw.txt:1200-1209`) admits a reading where the installed Tx still suppresses the default; it does not settle the output in this gap. Flag the disputed branch; do not claim the official text unambiguously requires either 320 or none.

## WellFormed audit against P1–P8

| Precondition | Encoding | Assessment |
|---|---|---|
| P1 | `1 ≤ c` | Connector bound exact; active transaction absent. External snapshot contract required (§3.13.1 `raw.txt:1200-1209`; §7.10 `raw.txt:5426-5430`). |
| P2 | Nat stack/connector; Purpose enum; station maximum on 0; Tx on positive ID | Faithful on represented fields (`raw.txt:1186-1209,5318-5326`). Non-applicable profiles permitted. |
| P3 | Kind = Absolute/Daily/Weekly; mandatory startSchedule | Faithful by representation to signed P3, though official optional startSchedule/Relative behaviors are not reached (`raw.txt:5389-5400,5501-5508`). |
| P4 | periodsWellFormed for every profile | Faithful; nonempty, first zero, pairwise strict increase. A8 repairs the official typo (`raw.txt:5522-5526,5559-5560`). |
| P5 | Nat tenths, no unit field | Nonnegative one-decimal limits exact by encoding (`raw.txt:5562-5570`); shared chargingRateUnit external (`raw.txt:5510-5516`). Mixed A/W can erase to identical Lean inputs, so raw-input validation is mandatory. |
| P6 | Unequal-record pair condition plus no mixed 0/c defaults | Distinct modeled records checked; duplicate occurrences/collapsed identities escape. CR-OC16-02. Default installation restriction agrees with signed A3, stronger than merely excluding simultaneously valid overlap. Official replacement text uses same purpose/stack without connector qualification (`raw.txt:1220-1222`); the signed connector qualification is a narrower contract, not a proof of global installed-registry validity. |
| P7 | Every present duration > 0 | Exact signed positivity (`informal:58`); official duration has integer type (`raw.txt:5491-5499`). |
| P8 | No transactionId field | Faithful only if erased input was first checked to have no transactionId. A nonmatching Tx transactionId is not representable/testable; official matching is `raw.txt:5298-5306`. |

Time is additionally restricted to nonnegative integral calendar seconds by `Time := Nat` (Lean:31–33). Fractional DateTime instants and pre-epoch dates cannot be rounded into this domain and still claim exactness. The signed text fixes calendar-day/leap-second interpretation but does not explicitly state the epoch/integral restriction. Treat those unrepresentable inputs as not yet reached, not as approved numeric approximations. Limits, by contrast, are exactly encoded in tenths; no floating-point arithmetic is modeled.

## Theorem impact and statement audit

Only i4 still bears sorry, but every named statement was checked as requested. A mismatch is a correspondence blocker, not evidence that an already kernel-checked model theorem is false. In particular the skill's phrase “vacuously unprovable” must not be interpreted as a mathematical conclusion about these existing proofs.

| Declaration | Signed property and definitions reached | Statement assessment and impact |
|---|---|---|
| q1 (Lean:209–211) | Q1; cpMaxResult/purposeLimit/prevailing/limitAt | Matches signed Q1 by definitional equality. It does not independently characterize highest-stack selection or prove source correspondence. Inherits CR-OC16-01 and A1 approximation. |
| q2 (215–220) | Q2; txResult/prevailing/limitAt/purposeLimit | Matches signed Q2 exactly, including no fallback if a prevailing Tx has no period. Encodes A2; not a proof of the official transaction-long reading. |
| q3 (223–231) | Q3; composite/minOpt and both purpose results | Correct optional minimum cases. Tautological composition over model results; inherited blockers remain. |
| i1 (234–241) | I1; cpMaxResult/composite | Matches signed I1: existing Q1 result implies an existing composite ≤ it. Does not bound the aggregate of connectors or prove that cpMaxResult is the correct official bound. |
| candidates_eq_of_same_nonDefault (245–262) | Supporting lemma; candidates | Correct order-preserving non-default-list equality premise. No separately named informal invariant is claimed. Inherits candidates interpretation for source-level application. |
| i2 (266–279) | I2; prevailing/composite | Matches signed I2 on the modeled list representation: same ordered non-default list, arbitrary default modifications, and a prevailing Tx. Reordering unrelated non-default entries is not included by hSame; P6 makes equal-stack order irrelevant on intended inputs. No WellFormed assumption, so the model result also covers C5. |
| i3 (282–289) | I3; txResult/composite | Matches signed I3: existing Q2 result implies composite exists and is bounded by it. Inherits A1/A2 interpretations. |
| i4 (293–297) | I4; prevailing/composite | No evident misstatement. It omits the prose's “which does not itself prevail” hypothesis, but this is implied by a currently prevailing p and strict q.stackLevel < p.stackLevel: filtering in a lower candidate cannot displace p. If q is not a candidate it has no effect either. Its purpose is explicitly the purpose queried in hp. This is a stronger-looking but equivalent sufficient condition, not a counterexample. Still unproved; inherited source correspondence blocker must be resolved before treating any future proof as source evidence. |

No q1–q3/i1–i4 statement was found to assert a different signed property. None quantifies over `WellFormed`, and none proves it enforces P1–P8. The misleading claims are the completeness of the WellFormed description and the unconditional interpretation of scheduleStart's “D3” label. Q4 has a definition but no dedicated theorem; the signed boundary does not require a separately named q4 theorem.

## Modelling decisions (carried forward from /lean-impl)

- **Time is Nat seconds of UTC in calendar days.** Signed D3/N4 choose 86400-second calendar days without leap-second accumulation. Daily/Weekly arithmetic is correct under this choice. The original suggestion that every elapsed-time disagreement must be a charger bug is too strong: the official same-clock-time/elapsed-hours wording remains ambiguous (`raw.txt:6307,6317`). Epoch/integral representability is an additional restriction to expose in the harness.
- **Limits are Nat tenths of one unit.** Exactly represents nonnegative one-decimal values (§7.14, `raw.txt:5568-5570`, P5). No A/W conversion is modeled; a shared unit must be checked before erasure. This is not a real-versus-float approximation of arithmetic.
- **Kind has no Relative constructor.** Signed P3 is enforced by type shape, including a startSchedule for every profile. Relative, missing-start and missing-recurrency raw inputs must be rejected before translation.
- **Equal-stack ties keep the first candidate.** Deliberate extension outside the intended unique-maximum domain. Distinct conflicting candidates fail P6; identical list duplicates escape WellFormed. The stub's blanket claim that ties cannot occur needs that qualification.
- **limitAt returns none without coverage or an eligible period.** This safely totalizes a partial signed notion. Outside-P4 holes cannot be interpreted as a default fallback. The stub's “no risk” description was too broad because neither operations nor theorem statements enforce WellFormed.
- **txResult branches on a prevailing TxProfile, not a successful limit lookup.** Faithful to signed Q2/I2, including C5. It relies on signed A1/A2 when there is no covering TxProfile; exact official-text equivalence is not established in those regions.
- **Single connector and one timestamp.** This model specifies the pointwise upper envelope for a single active transaction. Aggregate station allocation, interval construction and implementation conformance require additional harness contracts; an arbitrary smaller implementation result is accepted by signed Q4.

## Mismatch issues opened

These are the reviewer's local tracking issues, kept as written. Both are now fixed, as the status line under each heading says. The reviewer proposed and performed no repair.

### CR-OC16-01 — Absolute pre-start scheduleStart differs from signed D3

- **Status.** Fixed before DRT ran. The model applies the pre-start guard only to Recurring profiles, and the signed D4 regained its "s ≤ t" condition, confirmed by the operator on 2026-10-27. Class of `scheduleStart` is now exact.

- **Lean ident.** `scheduleStart`, Lean:101–106; transitive consumers `covers`, `limitAt`, `candidates`, `prevailing`, `purposeLimit`, `cpMaxResult`, `txResult`, `composite`.
- **Source.** §§7.9/7.13, `raw.txt:5389-5395,5501-5508`; signed D3/D4/A5, informal:40–41,111.
- **Disagreement.** The model guards all kinds before startSchedule; signed D3 only gives Recurring schedules no start before startSchedule and otherwise specifies the Absolute fixed start. At least the helper output and literal D4 coverage disagree on a P1–P8-admitted query. Signed D5 before that fixed start also needs a clear offset interpretation; this review does not assume Nat truncation is the prose's intended subtraction.
- **Reproducible witness.** Absolute profile, connector 1, stack 0, startSchedule 1000, duration 10, no validity bounds, periods [(0,100)], c = 1, t = 999, active transaction. Lean scheduleStart = none; literal signed D3 = 1000. Lean covers = false; literal signed D4 = true. The profile satisfies represented WellFormed requirements.
- **Owner.** Needs human triage between `/lean-impl` and signed-spec revision. The official starting-point wording makes a signed-prose omission plausible; no silent amendment is made.
- **Impact.** Blocks source-level oracle claims along the dependency chain. It does not falsify q/i proofs about the existing model.

### CR-OC16-02 — WellFormed does not enforce installed-list P6 multiplicity

- **Status.** Fixed before DRT ran. P6 is stated over list positions with a local `pairwise`, so equal duplicate entries violate it. Class of `WellFormed` is now exact.

- **Lean ident.** `WellFormed`, Lean:199–201; misleading no-ties comment at 133–134.
- **Source.** §3.13.2 replacement/conflict rule, `raw.txt:1220-1222`; §7.8 identifier, `raw.txt:5290-5296`; informal:28,56 (P6).
- **Disagreement.** The predicate only constrains unequal Profile values. As a predicate over `List Profile`, it accepts equal duplicate occurrences with the same purpose/stack/connector. Distinct raw installed IDs can be erased into equal values. Signed P6 says no two applicable installed profiles share that key. If lists merely represent deduplicated mathematical sets, that interpretation must be stated and enforced; it is not a theorem supplied by WellFormed. Equal duplicates have no observable limit difference, so this is a precondition/traceability mismatch rather than a witnessed composite-limit bug.
- **Reproducible witness.** Let p be Absolute Tx, connector 1, stack 0, start 1000, no duration or validity bounds, periods [(0,100)]. `WellFormed [p,p] 1` is provable with `simp [WellFormed, periodsWellFormed, applicable]`; the list has two applicable occurrences with the same P6 key. This exact proposition was checked via Lean stdin.
- **Owner.** Needs human triage for set-versus-list interpretation; `/lean-impl` owns a model-domain correction if P6 means installed occurrences. No identity/multiplicity policy is invented here.
- **Impact.** Blocks treating WellFormed as the complete signed input-domain gate. P1/P5/P8 also require the explicitly documented external checks; those omissions alone are deliberate abstractions, not independent behavioral counterexamples.

## Open questions for /drt-oracle

- Resolve both mismatch issues before invoking DRT. Done: both were fixed before DRT ran. This review did not run differential tests or change the Lean model/spec/SUT.
- A1 candidates and A2 txResult are flagged approximation regions relative to unsettled official wording. The signed resolutions are explicit and faithfully encoded, but exact official conformance is not established. The reviewer asked for flagged skips. The operator approved a different handling on 2026-10-27: these regions are tested against A1 and A2 and any divergence is reported as a spec-reading gap, not an EVerest bug.
- Recurrence clock and missing validity bounds require fixed context: calendar-second encoding, installed snapshots, active transaction, same units, no transactionId. Fractional or pre-epoch dates must be rejected rather than rounded. Precision/tenths conversion must preserve decimals exactly.
- Period validity and uniqueness are cross-definition contracts. Test generation must validate all P1–P8 before field erasure; `WellFormed` alone is insufficient. Preserve list order only as encoding, not an authorized tie-resolution rule.
- A future EVerest divergence around duration fallback, whole-transaction override, connector-0 defaults, or leap-second conversion may reflect the already recorded wording questions (informal N1/N2/N4), not a demonstrated EVerest defect. No EVerest behavior was classified or newly established by this review.
- Use Q4's inequality rather than exact equality for implementation conformance, including no model bound. Independently validate the SUT's returned time coverage/unit representation. A conforming smaller value does not test equality with the maximal oracle envelope.
- The direct helper source pair missing from D3 for Absolute pre-start queries must be resolved before using any generated witness in that region as evidence against the SUT. Resolved by the CR-OC16-01 fix.

## Verification Checklist

- [x] Step 0 prerequisites passed, with the user-authorized host Lean build substituted for lean_check; only expected i4 warning.
- [x] Every def has a source pair recorded; missing immediate src comments are explicitly documented.
- [x] Every in-scope signed operation has a counterpart; boundary restrictions and external preconditions are identified.
- [x] Every classification cites its rubric step and source section/extraction lines.
- [x] Concrete recurrence, duration, validity, malformed-period, connector, tie and missing-Tx-period cases were worked by hand; selected Lean results were checked without editing the model.
- [x] Theorem impact assessed for remaining sorry theorem i4 and all other requested named properties.
- [x] Verdict counts and overall BLOCKED readiness emitted (the reviewer's original verdict; revised to READY by the audit).
- [x] Both mismatch issues opened as local records in this document, with witnesses and owners; no external issue creation claimed.
- [x] Pipeline stopped; no /drt-oracle invocation or handoff.
- [x] All mismatches resolved. The reviewer left CR-OC16-01 and CR-OC16-02 open. Both were fixed before DRT ran (see the audit), and the revised counts are exact 20, abstraction 4, approximation 0, mismatch 0.
- [x] The reclassification of `candidates` and `txResult` to abstraction was approved by the operator on 2026-10-27.
