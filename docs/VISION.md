# Vision

This document states what the contract graph verifier (CGV) and the Crosscheck plugin are for. The two projects share one vision, agreed on 2026-09-29. It is written for anyone who decides whether to trust AI-generated code, whether they are an engineer or an executive.

## The problem

AI agents write code faster than people can review it. The common answer is to have another AI review the code. That moves the question instead of answering it, because you now have to trust the reviewer. We want a way to know that AI-generated code is correct without trusting an AI to say so.

## The vision

Every change an AI makes ships with a record of evidence. The record lists each claim about the change and how strongly the claim is supported. Where a claim can be proved, it is proved, and a small checker that anyone can rerun confirms the proof. Where a claim cannot be proved, the record says so and shows the evidence it has instead. Every claim traces up to a requirement that a person approved. Anyone can read the record at the level of detail they need, and anyone can rerun the checks behind it.

## What "correct" means

Correctness is always relative to something. This vision uses one chain from the world down to the code:

```
goal in the world  →  requirement  →  spec  →  code
```

- A **goal** is what the organisation wants to be true in the world, for example "customers pay the right amount".
- A **requirement** states what the system must achieve, in terms of the world.
- A **spec** states what the code must do at its boundary.
- The **code** is what runs.

The guarantee is that the code satisfies its formal spec, relative to a named trusted base. The trusted base is the short list of components the proof depends on, such as the proof checker, the compiler, and any model of a framework or the environment.

A proof can also climb one step higher. A spec, together with stated assumptions about the world, can be proved to achieve a requirement once that requirement is written formally. So completeness of a spec is provable relative to a formal requirement.

Two things can never be proved, because they are facts about the world rather than about formal objects:

- that the assumptions about the world are true, and
- that the requirements achieve the goal.

The system answers these two with labelled evidence from observation and with the recorded judgment of people who know the world.

| Link in the chain | What the system provides | Strength |
| --- | --- | --- |
| The code satisfies the spec | A machine-checked proof | Proved, relative to the trusted base |
| The spec and the assumptions give the requirement | A machine-checked proof, once the requirement is formal | Proved, relative to the assumptions |
| The assumptions are true of the world | Observation, such as acceptance tests and production monitoring | Observed |
| The requirements achieve the goal | Judgment by domain experts, informed by generated scenarios | Judged |

## Design rules

### Trust

1. **LLMs propose. Deterministic checks and humans decide.** An LLM may draft code, specs, proofs, and glossary entries. No guarantee rests on the judgment of an LLM. Tools that use an LLM to find likely problems, such as Crosscheck's `/intent-check` and LLM code review, are search tools. Their output points people at problems. It is never evidence.
2. **Trust a few small things, and measure them.** Every claim names its trusted base. Each proof is rechecked by more than one independently written checker. Each checker has a published track record of seeded errors that it rejected.
3. **Everything reruns.** A deterministic result gives the same answer when anyone reruns it from pinned inputs, without help from the original authors. A sampled result, such as a property-based test, records its seed so that it reruns the same way.

### Readability

4. **The plain-language spec is the spec.** Requirements and specs are written in a controlled English whose grammar maps one-to-one onto formal logic. The reader reads the spec itself, not a paraphrase of it.
5. **The organisation's words are defined once.** A glossary gives each domain term, such as "customer", "tariff", or "settlement", a formal definition that the domain experts approve. Specs use those terms.
6. **Examples make a spec judgeable.** A solver generates concrete scenarios that the spec allows and forbids, and it looks for surprising ones. A domain expert judges each scenario, for example: "The spec allows a refund larger than the payment. Is that right?"

### Honesty

7. **Every claim names its strength.** A claim is proved, tested, observed, or judged. The table above shows where proved, observed, and judged apply. Tested means checked on sampled inputs, such as a property-based test, and the claim states how many. "Verified" on its own is never a claim.

## Scope: every class of code

No class of code is outside this vision. The current tools reach some classes and not others. When a tool does not reach a class, say "not yet reached", and name the property that blocks it and the open research question. Do not call a class "excluded" or "out of scope".

To make sure that every piece of code has a place, classify code by the properties that make it hard to verify, not by a list of kinds:

| Property | Values |
| --- | --- |
| Determinism | Deterministic, nondeterministic, or probabilistic |
| Effects | Pure, local state, or external systems |
| Time | Sequential, concurrent, or distributed |
| Environment | Hardware, network, or third-party services |
| Informal quality | Appearance, prose, or quality of predictions |

A partial list of classes, each with its main open question:

| Class | Open question |
| --- | --- |
| Framework-heavy code, such as Django or React | How to build a model of the framework that is small enough to trust |
| Concurrent and distributed systems | How to keep the code and its model in step as the code changes |
| Failure and network partition | How to state the fault model, which is an assumption about the world |
| Performance | How to state bounds that depend on the hardware and the load |
| Security | How to state the attacker model completely |
| User interfaces | How to specify "looks right" |
| Machine-learning models and code that calls LLMs | What a statistical guarantee means to a reader, and how to state one |
| Infrastructure as code, SQL migrations, and data pipelines | How to model the systems they change |
| Firmware and floating-point numerics | How to model the hardware and rounding |
| Tests | How to prove that a test checks what it claims to check |

Each class has prior work to build on, for example TLA+ for distributed designs, Iris for effectful code, and seL4 and CompCert for proofs about whole systems.

The limit is not decidability. Rice's theorem says that no tool can decide correctness for every program automatically. It does not stop a small checker from checking a proof that comes with the code. So in this vision the AI writes the proof along with the code. The hard limits that remain are the boundary between formal and informal statements, the size of the trusted base, and the cost of writing proofs and models. AI lowers the last of these.

## Who reads what

- **An executive** reads the requirements in controlled English, the glossary, the generated scenarios, the assumptions about the world, and the list of the trusted base. They judge the assumptions and the requirements, which is where their knowledge of the industry, the organisation, and the market applies. They do not read proofs.
- **An engineer** reads the specs, the status of each proof, and the trusted base. They rerun any check.
- **An auditor** reruns every check from pinned inputs and compares the results with the record.

## Where the tools stand on 2026-09-29

**Crosscheck** covers the chain from the spec down to the code with Dafny (verify and extract) and with Lean models used as oracles for differential random testing. It also has tools that use LLMs, such as `/intent-check`, `/spec-adversary`, and the semi-formal reasoning skills. Rule 1 makes these search tools. Crosscheck's six-layer assurance hierarchy treats spec completeness as best effort and lists some classes of code as not addressed. This vision replaces both positions: completeness is provable relative to a formal requirement, and no class of code is out of scope.

**CGV** checks that data flowing through Python code satisfies the constraints declared on Django model fields, data classes, and type annotations. It needs no hand-written spec, so it is the entry point for existing code that has no specs. Its checker is proved in Lean (`runChecker_sound_all` in `prover/ContractGraph/Main.lean`). Its extraction from Python is not proved yet, so its trusted base includes the Rust extractor, `Translation.lean`, and `BehaviorModel.lean`. Issue #16 tracks proving extraction. CGV shows that code agrees with its own declared contracts. Those contracts do not yet trace to requirements. On one real codebase, 3 of 67 triaged errors were reachable bugs, as reported in `docs/evaluation/real-codebase-evaluation-2026-09.md`.

## Open questions

These came up while the vision was agreed and are not settled:

- Which controlled English to use, and how to trade expressiveness against readability.
- Who owns the glossary, and how a change to a definition reaches the specs that use it.
- How evidence of different strengths combines into one verdict on a pull request.
- How to keep the AI that writes the code from also writing the checks that grade it.
- How to measure the whole suite, for example with a shared corpus of replayed and seeded bugs.
- Where intent is recorded. The prompts, issues, and transcripts behind AI-generated code are candidate inputs to requirements.
