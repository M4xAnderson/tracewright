# The final predicate is a parameter of the deadlock-free property and arrives in slice 1

Amends ADR 0010 and ADR 0013. The **final predicate** (UI label "done when") stays model data of the scope element, one for its own behavior and one for its combined behavior (ADR 0003), but it is **not part of the net**: lowering copies it into `NetProperty::DeadlockFree { final }` of that behavior, the canonical property line is `property deadlock-free final none | final <pred>`, and the flattened net's encoding has no `final` line. Editing "done when" therefore changes only the deadlock-free key of that behavior and never stales "no capacity exceeded" or a user property, which keeps ADR 0013's rule that staleness is never spurious; the second explorer needs nothing beyond the parts and the bindings to compose. A property may not override the scope's final predicate. Final predicates arrive in **slice 1** in a minimal form (the editor offers a conjunction of `place = n` and, for a state machine with several final states, the single sum atom `Σ element/final-state = 1`; hand-written text may be any predicate of the one grammar), which replaces ADR 0010's "(slice 4; before that there are none)". Slice 0 stores every predicate and `[final]` string opaque and lowers `final none`; a `[final]` table present in slice 0 is reported as "not checked until slice 1". The Disproved panel for a deadlock with no final predicate says "no transition is enabled and no final predicate is defined", points to "done when", and never advises a transition back to the initial marking, because a reset transition hides real deadlocks.

Amended by ADR 0022.

## Considered Options

- **Final predicate in the net** (`Net.finals`, `OpenNets.final`): every property of the scope goes stale when "done when" changes, and the second explorer must carry the predicate to rebuild the encoding.
- **Slice 0** for the minimal form: slice 0 stays the smallest skeleton; its gate needs a Disproved, not a Proved fix. **Slice 4**: both E5 fixes are Disproved at their intended end until then, so slice 1 is where a fix is first shown to work.

## Consequences

- Predicate evaluation lands in both explorers in slice 1, used only by the final predicate; user properties reuse it in slice 4.
- The encoding header stays `petrinet-net 1`: no cached verdict exists before slice 0 code.
- `docs/specs/net-semantics.md` revision 3 §3.3, §8.1, §11 and `docs/specs/decisions.md` D-04, D-05, D-23, D-25, D-41, D-42 carry the rules.
