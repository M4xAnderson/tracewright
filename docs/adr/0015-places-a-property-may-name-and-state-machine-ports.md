# A combined-scope property names place ports and generated places; a state machine's events are transition ports and its states are place ports

Amends ADR 0002 and ADR 0003. In a combined scope a property, a template parameter or a final predicate names only **place ports** of the children and **generated places** of the scope (net-semantics L9); in an own scope it may name any place of the element. A place port is never bound by a relationship in v1 (there is no place fusion); it exists so that the parent scope can name it, and promoting a place to a port is one attribute in the editor. A template expansion that lowering computes from the model (the "its initial marking" preset) may name every place, because it is not stored. The "weighted sum of places" of ADR 0003 has integer weights, negative allowed, so two sums can be compared. For a state machine, its input and output events are its transition ports (each event stands for every transition on it), every state is a place port that means "this state is active" and is named `element/state` in the one namespace that states, events and transitions of an element share, and its internal events are not ports and fire on their own in every scope.

## Considered Options

- **Any place or state of a child** (template catalog): no port step for the modeler, but parent files depend on child internals, and a renamed or deleted internal place breaks the parent's properties.
- **Own-scope properties only** (research F1 c): sound, but cannot sum across two children, which the slice 4 acceptance example needs.
- **A distinct `element/state:<name>` shape** for state atoms: a second ID shape in both explorers and a character outside the slug alphabet, for no gain over one namespace.

## Consequences

- E5 case 1 is ported with `control_plane/in_a`, `wait_a`, `ret_a` and `threads_free` as place ports; the slice 4 acceptance example reads `never billing/inflight + shipping/inflight > 2` with both places declared as ports.
- `docs/specs/net-semantics.md` revision 3 L9, `docs/specs/state-machines.md` §2.1 and `docs/specs/decisions.md` D-01 carry the rule.
