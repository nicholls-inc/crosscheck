# Crosscheck publication and application material

These are drafts for Harry to edit and use. They are not sent messages or a
claim that the website has launched. Replace the marked URL and recipient
fields only after checking the public release and the recipient's actual work.

## The story

I built Crosscheck to make claims about AI-written code inspectable. The work
combines specification, AI-assisted implementation, formal models and repeatable
tests, with the assumptions and missing guarantees kept beside the result.

I applied its Lean workflow to EVerest's OCPP 1.6 charging-schedule code. The
study reports a low-severity recurrence drift caused by leap seconds. A model,
a small witness and pinned replay instructions make the finding inspectable.
The result is a bounded comparison with real charging software, not a proof of
the whole system or an upstream fix.

The published [vision essay](https://brainflow.substack.com/p/review-evidence-not-code)
explains why I think review should focus on evidence. The case study shows what
that looks like in practice and where it stops. My contribution was directing
the study, approving interpretations, reviewing model correspondence and deciding
what the evidence supports; agents drafted artifacts and deterministic tools
checked the completed proofs and sampled comparisons.

## Application summary

> I build tools that help engineers inspect claims about AI-written code. With
> Crosscheck, I directed an AI-assisted verification study of EVerest's EV charging
> scheduler: a Lean reference model, checked model properties and differential
> tests against production C++. The study reports a reproducible leap-second
> recurrence drift. My case study covers the design choices, human decisions,
> AI assistance and limits: [public study URL].

For an AI-platform or developer-tool team, emphasize the engineer-facing
workflow and how a claim can be reproduced. For an applied-AI or climate/deep-tech
team, emphasize deriving a useful check from domain rules and preserving the
boundary between a formal model and a system that acts in the world. This study
does not establish aerospace, biotech or robotics production experience.

Possible résumé bullets, subject to your own wording and résumé context:

- Built Crosscheck, an AI-assisted developer-tool workflow combining specifications,
  formal models, checked proofs and reproducible tests.
- Directed an OCPP 1.6 study comparing EVerest's existing C++ scheduling code with
  a Lean reference model; the reported 2,000-case older-anchor run identified a
  low-severity leap-second recurrence drift with a concrete replayable witness.
- Recorded human interpretation decisions, trusted dependencies and unfinished
  work so reviewers could distinguish model proofs from sampled production tests.

## A specific side-door message

Subject: [Their concrete engineering problem] and a result from charging software

Hi [name],

[One sentence about a specific public artifact, engineering decision or problem
of theirs, with enough detail to show you actually read it.]

I built Crosscheck to make claims about AI-written code inspectable. I recently
used its AI-assisted workflow to model an EV charging scheduler and test an
existing C++ implementation against it. The study found a small leap-second
timing drift; the case study includes my decisions, the witness and the limits:
[public study URL].

[Explain the narrow connection to their problem. Offer one useful observation,
question or small artifact; do not imply you audited their system.]

Would it be useful to compare notes on [one focused question]?

Harry

This draft follows the emphasis on visible work and specific problems in
[Maja's side-door essay](https://velvetnoise.substack.com/p/how-to-enter-side-doors),
and the specificity, generosity, honesty and relevance discussed in
[Sublime's email essay](https://sublimeinternet.substack.com/p/can-one-email-change-your-life).
Those are prompts for a personal message, not evidence that outreach will work.

## Before choosing a recipient

Fill this in for one person or team before drafting. There is no researched
recipient or employer list in this document.

| Question | Your notes |
| --- | --- |
| What specific problem or public artifact caught your attention? | |
| What is the primary-source link, and when did you read it? | |
| Which part of your work is useful to that problem? | |
| What can you offer without asking them to do the research? | |
| Which study limit matters for this comparison? | |
| What one question would make a useful conversation? | |
| Have you checked role, location and compensation fit separately? | |

## Optional essay follow-up

> I've now written up a worked example using Crosscheck on EVerest's OCPP 1.6
> charging-schedule code. It follows one claim through a model, tests and a
> concrete timing difference, with the assumptions and missing guarantees shown:
> [public study URL].

Use this only after the public site check passes. Keep the study's reported/WIP
status clear; describe EVerest issue #3323 as a filed, open report, not a confirmed
fix, and do not present a diagnostic input change as a production fix.
