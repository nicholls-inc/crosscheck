# Plain-English explanations for Crosscheck's site

Draft definitions for specialist verification and charging terms. Prefer rewriting jargon first. Ubiquitous software-engineering and technology terms need no definition, dotted underline or tooltip; the user explicitly exempts "AI" and "plugin". Code, tests, test cases, inputs, outputs, installation, marketplace and similar familiar terms remain ordinary text. The entries below explain specialist meanings rather than every technical word. The mockups are not a complete term inventory for a later site.

| Term | Plain-English explanation |
| --- | --- |
| proof; proved; machine-checked proof | A computer-checked mathematical argument that a rule always holds under stated assumptions. It does not tell us whether the rule is the right one. |
| proof assumptions | Conditions taken as true for a mathematical result. They must be checked against how the program will actually be used. The ordinary word "assumptions" alone does not require a tooltip. |
| Dafny | A language and tool for checking that code follows written rules. It can turn checked code into Python or Go. |
| Lean | A language for precisely describing how code should behave, and a tool for checking mathematical arguments about those descriptions. |
| model; reference program | A program describing the behavior we expect. We compare its answers with the code being checked. It covers only the behavior it describes. |
| formal verification | Using mathematics to check that code follows stated rules, under stated assumptions. |
| formal specification | Written rules precise enough for a computer to check mathematical claims about them. The familiar software-engineering word "specification" alone does not require a tooltip. |
| invariant | A rule that must hold whenever the conditions in the written rules are met. |
| differential testing; differential random testing; DRT | Run the same chosen inputs through two programs and compare their answers. A difference needs investigation; matching answers cover only the inputs tried. |
| correspondence review | A review of whether the program describing expected behavior follows the written rules. The review is separate from a mathematical proof. |
| anchor; recurring anchor | The starting date and time from which a repeating schedule is counted. |
| leap second | An extra second sometimes added to clock time to keep it in line with Earth's rotation. |
| OCPP | Open Charge Point Protocol: shared rules for communication between electric-vehicle charging stations and the software managing them. This study checks charging schedules under version 1.6. |
| trusted base | The parts assumed to work correctly for a proof result to hold, such as the program checking the proof and the program turning checked code into runnable software. |

Plain-language substitutions must preserve the source report's meaning. In particular, 1,140 differing cases means 67 warnings plus 1,073 failures in the 2,000-case older-starting-date configuration; it is not a count of distinct bugs. Use the source report's definition of warning and failure in the detailed study.
