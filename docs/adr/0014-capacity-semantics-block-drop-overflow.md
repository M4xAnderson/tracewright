# Every place declares what happens when it is full: block, drop or overflow

Every place has a capacity (glossary) and a `when_full` choice. `block`: a transition that would put the place over its capacity is not enabled, so the sender waits (back-pressure). `drop`: the transition fires and the tokens that would exceed the capacity are discarded (load shedding). `overflow`: the transition fires, the count exceeds the capacity, and the resulting marking is an overflow marking, which violates the built-in property "no capacity exceeded". A generated place (the place an asynchronous binding adds, ADR 0002) defaults to `overflow`, so the user must state back-pressure explicitly and an unbounded producer is Disproved rather than silently throttled; a place inside a behavior model has no default and must choose, which the net editor enforces and the loader reports as "model incomplete" for hand-written files. An overflow marking is a violation state: it is not expanded further, it is not a deadlock and it is never final. For "no capacity exceeded" it is the Disproved marking. For every other property, a violation reached without passing through an overflow marking is Disproved, Proved requires that no overflow marking is reachable, and when an overflow marking is reachable but no violation was found the verdict is Unknown "capacity exceeded first", naming the place and suggesting `block`, `drop` or a larger capacity. Both explorers implement these rules independently from this text (ADR 0009, ADR 0010); neither may compile `block` into complement places shared with the other.

## Considered Options

- **Capacity always blocks (complement places, research R1):** keeps every net bounded by construction, but makes "no capacity exceeded" vacuous and hides the slice 0 overflow case and E5's retry amplification.
- **Capacity always an assertion that overflow violates:** matches E5's explorer, but then back-pressure needs hand-built complement places that the editor cannot see.
- **Default `block` for generated places:** friendlier to deadlock-free, but a user who never thought about overflow would get a Proved about a system that in reality drops or crashes.

## Consequences

- Deadlock-free on a model with an `overflow` place is decided only for the behavior before the first overflow; the verdict panel says so and the fix is one attribute away.
- The pattern library offers "bounded buffer, blocking", "bounded buffer, shed when full" and "retry with budget" on top of these three choices.

Amended by ADR 0017.
