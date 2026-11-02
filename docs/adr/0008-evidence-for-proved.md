# A Proved verdict needs evidence checked by independent code

The glossary requires that the tool has re-checked the evidence for every Proved. For a structural proof the evidence is a certificate (for example a P-invariant) that a small, separate checker verifies. For a proof by exhaustive exploration there is no small certificate, so a second, deliberately simple explorer (plain breadth-first search, no reductions, no shared code with the main explorer beyond the net types) must reach the same verdict; if the two disagree, the verdict is Unknown with the reason "internal disagreement" and the case is logged. Every Disproved is confirmed by replaying its counterexample on the original, unflattened model. In addition, a corpus of nets is checked in CI against TAPAAL as a development-only tool that is never shipped (it is GPL-3).

## Consequences

An exhaustive Proved costs roughly twice the exploration time, so the second explorer shares the check's budget; when the budget runs out during re-checking, the verdict is Unknown, never Proved.

Amended by ADR 0010, ADR 0017.
