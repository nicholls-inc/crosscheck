# Evidence-led website specification

Derived from [intent](2026-10-28-evidence-led-site.md) and the selected Canvas
approach `evidence` v2.

1. The home page leads with inspectable evidence and an EVerest summary.
   Its primary action opens the local study article; installation remains easy
   to find. The published essay is linked as the argument behind the work.
2. The study connects the written recurrence rule, concrete one-second witness,
   three proved model properties, unfinished fourth proof, sampled production
   comparison, human interpretation decisions, trusted dependencies and limits.
   It never claims EVerest itself is proved correct.
3. All study artifact links use the same full immutable revision. Counts are
   from that revision's report and clearly labelled reported. The article
   acknowledges the report's conflicting legacy classification note rather
   than silently treating pass/warn totals as exact agreement.
4. A downloadable publication record lists sources, revision, reported results,
   seed, tools, prerequisites, exact replay commands, assumptions, coverage
   limits, witness and human decisions. It is an editorial snapshot, not an
   automatically generated or independently checked proof attestation.
5. The article, home summary and downloadable record agree. Links connect the
   essay, study, sources, replay instructions, glossary and installation.
6. Retain the existing warm paper, ruled tables, local fonts and responsive
   layout. All text is present without JavaScript. Help works by keyboard,
   mouse and touch; copy controls provide success and failure feedback.
7. Static serving needs no build or dependency installation. New publication
   tooling runs using Node's built-in modules. Checks catch broken local links,
   divergent publication data, missing glossary definitions and moving study
   references. No deployment or upstream issue submission is included.

## Known boundary

The cited study is WIP and has unfinished assurance work. Publishing its bounded
claims and gaps does not resolve them. Scientific runs are not repeated as part
of website validation. Pinned build dependencies can be inspected in the cited
driver script; the record must not claim a hermetically reproducible environment.
